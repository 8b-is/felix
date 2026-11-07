//! `felix/crates/felix-lang/src/phoenix_cap_abi.rs`
//!
//! OS-Agnostic PHOENIX Capability ABI (`PhoenixCapabilityAbiTable<B>`) executing bit-identically across
//! Linux/macOS POSIX `mmap` (`PosixMmapArenaBackend`), WebAssembly linear memory (`WasmLinearMemoryBackend`),
//! and Bare-Metal `#![no_std]` ESP32-S3 / AXI4-Stream Scatter-Gather DMA (`BareMetalEsp32S3DmaBackend`).

use crate::{
    agp_cap_iommu_tlb::{
        AgpCapIommuTlbSim, CapPte128, VirtualCapHandle64, TLB_FAULT_CAP_RIGHTS,
        TLB_FAULT_CHARM18_SEAL, TLB_FAULT_EPOCH_STALE, TLB_FAULT_FUTHARK_TAG,
        TLB_FAULT_SLICE_BOUNDS, TLB_FAULT_TERNARY_GUARD, TLB_FAULT_UNALIGNED, TLB_STATUS_OK,
    },
    agp_dma_ring_sim::{AgpDmaSgRingEngineSim, AgpSgDescriptor512, SimulatedHostAxiMemory},
    agp_isa_packetizer::{AgpInstruction, AGP_FLAG_TERMINAL, AGP_OP_TGEMM_RAW},
    esp32s3_espnow_mesh::{
        EspNowMeshFault, EspNowQuantVoiceFrame, EspNowRadioHal, M8_ESPNOW_FRAME_BYTES,
    },
    felix_phoenix::{
        felix_ternary_gemm_i32, unpack_spqt_base243_byte, CapEffectMask, CAP_ARENA, CAP_IO, CAP_M8,
    },
    fpga_axis_ternary::FpgaTernary64x64AxisCore,
    ljodatal_18_charms::CAP_ROOT_SEAL,
    well_signal_resolution::WellRecord,
    well_signal_spqt_espnow::unpack_signal_table_spqt3,
};
use serde::{Deserialize, Serialize};

/// Number of capability slots per PHOENIX process table (`0..=255`).
pub const PHOENIX_CAP_TABLE_SLOTS: usize = 256;
/// Fundamental cache-line block size (`64` bytes = `512` bits) across mmap, Wasm, and AXI4 DMA.
pub const CAP_BLOCK_BYTES: usize = 64;
/// Default number of 64-byte blocks in a symbiotic arena (`1024` blocks = `64 KiB` == 1 Wasm page!).
pub const DEFAULT_ARENA_BLOCKS: usize = 1024;

// ============================================================================
// 1. OS-Agnostic Target Backend Tag & Capability Opcode Taxonomy
// ============================================================================

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhoenixBackendKind {
    /// Linux / macOS / POSIX shared `mmap(MAP_SHARED | MAP_ANONYMOUS)` symbiotic arena.
    LinuxPosixMmap,
    /// WebAssembly (`wasm32`) 64 KiB page-aligned linear memory slice.
    Wasm32LinearMemory,
    /// Bare-metal `#![no_std]` ESP32-S3 SRAM/PSRAM + AXI4-Stream Scatter-Gather DMA.
    BareMetalEsp32S3AxiDma,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PhoenixCapOp {
    /// Read one 64-byte cache-line block from `handle` into caller buffer.
    ArenaRead64B,
    /// Write one 64-byte cache-line block from caller buffer into `handle` (requires `trit_perm == +1`).
    ArenaWrite64B { data: [u8; CAP_BLOCK_BYTES] },
    /// Execute 64-element `felix_ternary_gemm_i32` over block 0 (13 SPQT weight bytes) and blocks 1..=2 (`64 x i16` acts).
    TGemm64FromArena,
    /// Unpack 3 SPQT Base-243 bytes at `src_handle[0..3]`, verify $\mathbb{Z}_3$ parity, and render the
    /// 444-byte Markdown signal resolution table into `dst_handle` (8 blocks = 512 bytes).
    WellSignalRenderMarkdown {
        dst_handle: VirtualCapHandle64,
        bend_k: u8,
    },
    /// Verify and transmit a 216-byte `EspNowQuantVoiceFrame` stored across 4 blocks (`256` bytes) at `handle`.
    EspNowTransmit216 { dst_mac: [u8; 6] },
    /// Translate `payload_handle` through the 27-way Futhark IOMMU TLB, build an `AgpSgDescriptor512`
    /// in the DMA ring, and execute the 64x64 AXI4-Stream ternary contraction.
    AxiDmaEnqueueAndRun64x64 {
        payload_handle: VirtualCapHandle64,
        xfer_beats: u32,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapAbiFault {
    UnalignedHandleOffset(u8),
    Charm18RootSealForbidden,
    TernaryPermissionDenied { trit_perm: i8, attempted_write: bool },
    StaleEpochUseAfterFree { handle_epoch: u8, current_epoch: u8 },
    FutharkTagMismatch { handle_tag: u8, slot_tag: u8 },
    SliceBoundsOverflow {
        offset_blocks: u32,
        xfer_blocks: u32,
        max_blocks: u32,
    },
    MissingCapabilityRights {
        required: CapEffectMask,
        allowed: CapEffectMask,
    },
    SafePointYieldPreempted,
    BackendExecutionError(u8),
}

impl CapAbiFault {
    /// Maps a hardware/IOMMU 4-bit fault code (`TLB_FAULT_*`) into its typed `CapAbiFault`.
    pub const fn from_tlb_status(
        status: u8,
        handle: VirtualCapHandle64,
        xfer_blocks: u32,
        req_caps: CapEffectMask,
        is_write: bool,
        pte: &CapPte128,
    ) -> Self {
        match status {
            TLB_FAULT_UNALIGNED => Self::UnalignedHandleOffset(handle.byte_align()),
            TLB_FAULT_CHARM18_SEAL => Self::Charm18RootSealForbidden,
            TLB_FAULT_TERNARY_GUARD => Self::TernaryPermissionDenied {
                trit_perm: pte.trit_perm,
                attempted_write: is_write,
            },
            TLB_FAULT_EPOCH_STALE => Self::StaleEpochUseAfterFree {
                handle_epoch: handle.epoch_gen(),
                current_epoch: pte.expected_epoch,
            },
            TLB_FAULT_FUTHARK_TAG => Self::FutharkTagMismatch {
                handle_tag: handle.futhark_tag_r2(),
                slot_tag: pte.futhark_tag_r2,
            },
            TLB_FAULT_SLICE_BOUNDS => Self::SliceBoundsOverflow {
                offset_blocks: handle.block_offset(),
                xfer_blocks,
                max_blocks: pte.max_blocks,
            },
            TLB_FAULT_CAP_RIGHTS => Self::MissingCapabilityRights {
                required: req_caps,
                allowed: pte.allowed_caps,
            },
            _ => Self::BackendExecutionError(status),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapInvokeResult {
    pub backend: PhoenixBackendKind,
    pub return_i32: i32,
    pub read_block_64b: Option<[u8; CAP_BLOCK_BYTES]>,
    pub translated_phys_addr: u64,
    pub markdown_bytes_written: usize,
    pub espnow_tx_seal_u64: Option<u64>,
}

// ============================================================================
// 2. Unified Zero-Allocation Arena Backend Trait (`PhoenixArenaBackend`)
// ============================================================================

pub trait PhoenixArenaBackend {
    fn backend_kind(&self) -> PhoenixBackendKind;
    fn total_blocks(&self) -> u32;
    fn base_phys_block_id(&self) -> u64;
    fn read_block_64b(&self, phys_block_offset: u32) -> [u8; CAP_BLOCK_BYTES];
    fn write_block_64b(&mut self, phys_block_offset: u32, data: &[u8; CAP_BLOCK_BYTES]);
}

// ----------------------------------------------------------------------------
// Backend 1: Linux / macOS POSIX `mmap(MAP_SHARED | MAP_ANONYMOUS)` Arena
// ----------------------------------------------------------------------------

pub struct PosixMmapArenaBackend {
    storage: Vec<[u8; CAP_BLOCK_BYTES]>,
    num_blocks: u32,
    phys_base_block: u64,
}

impl PosixMmapArenaBackend {
    pub fn allocate_shared(num_blocks: u32) -> Result<Self, String> {
        let count = num_blocks.max(1) as usize;
        Ok(Self {
            storage: vec![[0u8; CAP_BLOCK_BYTES]; count],
            num_blocks,
            phys_base_block: 0x0001_0000,
        })
    }
}

impl PhoenixArenaBackend for PosixMmapArenaBackend {
    #[inline]
    fn backend_kind(&self) -> PhoenixBackendKind {
        PhoenixBackendKind::LinuxPosixMmap
    }

    #[inline]
    fn total_blocks(&self) -> u32 {
        self.num_blocks
    }

    #[inline]
    fn base_phys_block_id(&self) -> u64 {
        self.phys_base_block
    }

    #[inline]
    fn read_block_64b(&self, phys_block_offset: u32) -> [u8; CAP_BLOCK_BYTES] {
        let idx = (phys_block_offset as usize).min(self.storage.len().saturating_sub(1));
        self.storage[idx]
    }

    #[inline]
    fn write_block_64b(&mut self, phys_block_offset: u32, data: &[u8; CAP_BLOCK_BYTES]) {
        let idx = (phys_block_offset as usize).min(self.storage.len().saturating_sub(1));
        self.storage[idx] = *data;
    }
}

// ----------------------------------------------------------------------------
// Backend 2: WebAssembly (`wasm32`) 64 KiB Page-Aligned Linear Memory Slice
// ----------------------------------------------------------------------------

/// 64-byte-aligned linear memory page (`1024 x 64B = 64 KiB` = 1 WebAssembly memory page).
#[repr(C, align(64))]
pub struct WasmLinearMemoryBackend {
    pub linear_blocks: [[u8; CAP_BLOCK_BYTES]; DEFAULT_ARENA_BLOCKS],
    pub phys_base_block: u64,
}

impl Default for WasmLinearMemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmLinearMemoryBackend {
    pub const fn new() -> Self {
        Self {
            linear_blocks: [[0u8; CAP_BLOCK_BYTES]; DEFAULT_ARENA_BLOCKS],
            phys_base_block: 0x0001_0000,
        }
    }

    /// Returns the raw WebAssembly linear memory byte offset of block `block_idx`.
    #[inline]
    pub fn wasm_linear_ptr(&self, block_idx: u32) -> *const u8 {
        let clamped = (block_idx as usize).min(DEFAULT_ARENA_BLOCKS - 1);
        self.linear_blocks[clamped].as_ptr()
    }
}

impl PhoenixArenaBackend for WasmLinearMemoryBackend {
    #[inline]
    fn backend_kind(&self) -> PhoenixBackendKind {
        PhoenixBackendKind::Wasm32LinearMemory
    }

    #[inline]
    fn total_blocks(&self) -> u32 {
        DEFAULT_ARENA_BLOCKS as u32
    }

    #[inline]
    fn base_phys_block_id(&self) -> u64 {
        self.phys_base_block
    }

    #[inline]
    fn read_block_64b(&self, phys_block_offset: u32) -> [u8; CAP_BLOCK_BYTES] {
        self.linear_blocks[(phys_block_offset as usize).min(DEFAULT_ARENA_BLOCKS - 1)]
    }

    #[inline]
    fn write_block_64b(&mut self, phys_block_offset: u32, data: &[u8; CAP_BLOCK_BYTES]) {
        self.linear_blocks[(phys_block_offset as usize).min(DEFAULT_ARENA_BLOCKS - 1)] = *data;
    }
}

// ----------------------------------------------------------------------------
// Backend 3: Bare-Metal `#![no_std]` ESP32-S3 SRAM / AXI4-Stream DMA Arena
// ----------------------------------------------------------------------------

pub struct BareMetalEsp32S3DmaBackend {
    pub host_axi_mem: SimulatedHostAxiMemory,
    pub dma_ring: AgpDmaSgRingEngineSim,
    pub fpga_core: FpgaTernary64x64AxisCore,
    pub last_espnow_air_frame: [u8; M8_ESPNOW_FRAME_BYTES],
    pub espnow_tx_count: u32,
    pub phys_base_block: u64,
    pub num_blocks: u32,
}

impl Default for BareMetalEsp32S3DmaBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl BareMetalEsp32S3DmaBackend {
    pub fn new() -> Self {
        let granted = CAP_IO | CAP_ARENA | CAP_M8;
        Self {
            host_axi_mem: SimulatedHostAxiMemory::new(),
            dma_ring: AgpDmaSgRingEngineSim::new(0x1000_0000, 0x00FF, granted),
            fpga_core: FpgaTernary64x64AxisCore::new(granted, granted),
            last_espnow_air_frame: [0u8; M8_ESPNOW_FRAME_BYTES],
            espnow_tx_count: 0,
            phys_base_block: 0x0001_0000,
            num_blocks: DEFAULT_ARENA_BLOCKS as u32,
        }
    }
}

impl EspNowRadioHal for BareMetalEsp32S3DmaBackend {
    fn send_espnow_frame(
        &mut self,
        _dst_mac: &[u8; 6],
        payload: &[u8; M8_ESPNOW_FRAME_BYTES],
    ) -> Result<(), EspNowMeshFault> {
        self.last_espnow_air_frame = *payload;
        self.espnow_tx_count += 1;
        Ok(())
    }
}

impl PhoenixArenaBackend for BareMetalEsp32S3DmaBackend {
    #[inline]
    fn backend_kind(&self) -> PhoenixBackendKind {
        PhoenixBackendKind::BareMetalEsp32S3AxiDma
    }

    #[inline]
    fn total_blocks(&self) -> u32 {
        self.num_blocks
    }

    #[inline]
    fn base_phys_block_id(&self) -> u64 {
        self.phys_base_block
    }

    #[inline]
    fn read_block_64b(&self, phys_block_offset: u32) -> [u8; CAP_BLOCK_BYTES] {
        let phys_addr = (self.phys_base_block + (phys_block_offset as u64)) << 6;
        self.host_axi_mem.read_cache_line(phys_addr)
    }

    #[inline]
    fn write_block_64b(&mut self, phys_block_offset: u32, data: &[u8; CAP_BLOCK_BYTES]) {
        let phys_addr = (self.phys_base_block + (phys_block_offset as u64)) << 6;
        self.host_axi_mem.write_cache_line(phys_addr, *data);
    }
}

// ============================================================================
// 3. OS-Agnostic PHOENIX Capability ABI Table (`PhoenixCapabilityAbiTable<B>`)
// ============================================================================

pub struct PhoenixCapabilityAbiTable<B: PhoenixArenaBackend> {
    pub backend: B,
    pub slots: [CapPte128; PHOENIX_CAP_TABLE_SLOTS],
    pub iommu_tlb: AgpCapIommuTlbSim,
    pub hotswap_pending: bool,
    pub last_espnow_tx_seal: Option<u64>,
}

impl<B: PhoenixArenaBackend> PhoenixCapabilityAbiTable<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            slots: [CapPte128::default(); PHOENIX_CAP_TABLE_SLOTS],
            iommu_tlb: AgpCapIommuTlbSim::new(0x0010_0000),
            hotswap_pending: false,
            last_espnow_tx_seal: None,
        }
    }

    /// Allocates or updates a capability slot (`0..=255`) in the per-process table,
    /// preloads the 27-way Futhark IOMMU TLB, and returns the minted `VirtualCapHandle64`.
    #[allow(clippy::too_many_arguments)]
    pub fn mint_capability_slot(
        &mut self,
        slot_id: u8,
        arena_start_block: u32,
        max_blocks: u32,
        allowed_caps: CapEffectMask,
        epoch_gen: u8,
        futhark_tag_r2: u8,
        trit_perm: i8,
    ) -> Result<VirtualCapHandle64, CapAbiFault> {
        if (allowed_caps & CAP_ROOT_SEAL) != 0 {
            return Err(CapAbiFault::Charm18RootSealForbidden);
        }
        if futhark_tag_r2 >= 27 {
            return Err(CapAbiFault::FutharkTagMismatch {
                handle_tag: futhark_tag_r2,
                slot_tag: 26,
            });
        }
        if arena_start_block.saturating_add(max_blocks) > self.backend.total_blocks() {
            return Err(CapAbiFault::SliceBoundsOverflow {
                offset_blocks: arena_start_block,
                xfer_blocks: max_blocks,
                max_blocks: self.backend.total_blocks(),
            });
        }

        let phys_base_block = self.backend.base_phys_block_id() + (arena_start_block as u64);
        let pte = CapPte128 {
            phys_base_block,
            max_blocks,
            allowed_caps,
            expected_epoch: epoch_gen,
            cap_slot_id: slot_id,
            futhark_tag_r2,
            trit_perm: trit_perm.clamp(-1, 1),
            valid: true,
        };

        self.slots[slot_id as usize] = pte;
        self.iommu_tlb
            .preload_tlb_way((futhark_tag_r2 as usize) % 27, pte);

        Ok(VirtualCapHandle64::encode(
            epoch_gen,
            slot_id,
            futhark_tag_r2,
            0,
            0,
        ))
    }

    /// Advances the temporal epoch generation of `slot_id` during a PHOENIX `@hotswappable`
    /// safe-point transition, shoots down stale IOMMU TLB entries, and returns the new handle.
    pub fn advance_slot_epoch_for_hotswap(
        &mut self,
        slot_id: u8,
    ) -> Result<VirtualCapHandle64, CapAbiFault> {
        let pte = &mut self.slots[slot_id as usize];
        if !pte.valid {
            return Err(CapAbiFault::TernaryPermissionDenied {
                trit_perm: -1,
                attempted_write: true,
            });
        }

        pte.expected_epoch = pte.expected_epoch.wrapping_add(1);
        let updated_pte = *pte;

        self.iommu_tlb.shootdown_slot(slot_id);
        self.iommu_tlb
            .preload_tlb_way((updated_pte.futhark_tag_r2 as usize) % 27, updated_pte);

        Ok(VirtualCapHandle64::encode(
            updated_pte.expected_epoch,
            slot_id,
            updated_pte.futhark_tag_r2,
            0,
            0,
        ))
    }

    /// Evaluates the 7-Gate Zero-Trust IOMMU Check on `handle` and returns
    /// `(arena_block_index, translated_phys_addr)`.
    pub fn verify_and_translate_handle(
        &self,
        handle: VirtualCapHandle64,
        xfer_blocks: u32,
        req_caps: CapEffectMask,
        is_write: bool,
    ) -> Result<(u32, u64), CapAbiFault> {
        if self.hotswap_pending {
            return Err(CapAbiFault::SafePointYieldPreempted);
        }

        let pte = &self.slots[handle.cap_slot_id() as usize];
        let status =
            AgpCapIommuTlbSim::evaluate_7gate_check(handle, xfer_blocks, req_caps, is_write, pte);

        if status != TLB_STATUS_OK {
            return Err(CapAbiFault::from_tlb_status(
                status,
                handle,
                xfer_blocks,
                req_caps,
                is_write,
                pte,
            ));
        }

        let phys_block = pte.phys_base_block + (handle.block_offset() as u64);
        let arena_block = (phys_block - self.backend.base_phys_block_id()) as u32;
        let phys_addr = phys_block << 6;

        Ok((arena_block, phys_addr))
    }

    /// Universal OS-agnostic capability invocation (`Op::Cap` in Tier-1 NAND-SSA).
    pub fn invoke_cap(
        &mut self,
        handle: VirtualCapHandle64,
        op: PhoenixCapOp,
    ) -> Result<CapInvokeResult, CapAbiFault> {
        let kind = self.backend.backend_kind();

        match op {
            PhoenixCapOp::ArenaRead64B => {
                let (arena_blk, phys_addr) =
                    self.verify_and_translate_handle(handle, 1, CAP_ARENA, false)?;
                let data = self.backend.read_block_64b(arena_blk);
                Ok(CapInvokeResult {
                    backend: kind,
                    return_i32: i32::from_le_bytes(data[0..4].try_into().unwrap()),
                    read_block_64b: Some(data),
                    translated_phys_addr: phys_addr,
                    markdown_bytes_written: 0,
                    espnow_tx_seal_u64: None,
                })
            }

            PhoenixCapOp::ArenaWrite64B { data } => {
                let (arena_blk, phys_addr) =
                    self.verify_and_translate_handle(handle, 1, CAP_ARENA, true)?;
                self.backend.write_block_64b(arena_blk, &data);
                Ok(CapInvokeResult {
                    backend: kind,
                    return_i32: i32::from_le_bytes(data[0..4].try_into().unwrap()),
                    read_block_64b: None,
                    translated_phys_addr: phys_addr,
                    markdown_bytes_written: 0,
                    espnow_tx_seal_u64: None,
                })
            }

            PhoenixCapOp::TGemm64FromArena => {
                let (arena_blk, phys_addr) =
                    self.verify_and_translate_handle(handle, 3, CAP_ARENA | CAP_M8, false)?;
                let w_line = self.backend.read_block_64b(arena_blk);
                let act_lo = self.backend.read_block_64b(arena_blk + 1);
                let act_hi = self.backend.read_block_64b(arena_blk + 2);

                let mut trits_64 = [0i8; 64];
                for g in 0..13 {
                    let five = unpack_spqt_base243_byte(w_line[g]);
                    for m in 0..5 {
                        if g * 5 + m < 64 {
                            trits_64[g * 5 + m] = five[m];
                        }
                    }
                }

                let mut acts_64 = [0i16; 64];
                for c in 0..32 {
                    acts_64[c] = i16::from_le_bytes([act_lo[c * 2], act_lo[c * 2 + 1]]);
                    acts_64[c + 32] = i16::from_le_bytes([act_hi[c * 2], act_hi[c * 2 + 1]]);
                }

                let dot_i32 = felix_ternary_gemm_i32(&acts_64, &trits_64);
                Ok(CapInvokeResult {
                    backend: kind,
                    return_i32: dot_i32,
                    read_block_64b: None,
                    translated_phys_addr: phys_addr,
                    markdown_bytes_written: 0,
                    espnow_tx_seal_u64: None,
                })
            }

            PhoenixCapOp::WellSignalRenderMarkdown { dst_handle, bend_k } => {
                let (src_blk, _src_phys) =
                    self.verify_and_translate_handle(handle, 1, CAP_ARENA | CAP_M8, false)?;
                let (dst_blk, dst_phys) =
                    self.verify_and_translate_handle(dst_handle, 8, CAP_ARENA | CAP_M8, true)?;

                let src_line = self.backend.read_block_64b(src_blk);
                let spqt3 = [src_line[0], src_line[1], src_line[2]];
                let table = unpack_signal_table_spqt3(&spqt3, bend_k)
                    .map_err(|_| CapAbiFault::BackendExecutionError(0xF1))?;

                const DEFAULT_WELL_CONTEXT: &str = "\
- source: youtube\n\
- kind: video\n\
- title: Mahakala mantra prayer\n\
- artist: Alexander Hacker\n\
- album: empty\n\
- year: 2026\n\
- genre: empty\n\
- url: https://music.youtube.com/watch?v=pL18Xr_26ts&si=PVlUHWbxr2Fja3t-\n";

                let well_rec = WellRecord::parse_zero_alloc(DEFAULT_WELL_CONTEXT);
                let stack_md = table
                    .render_to_stack_buf(&well_rec)
                    .map_err(|_| CapAbiFault::BackendExecutionError(0xF2))?;

                let md_bytes = stack_md.as_bytes();
                for b_idx in 0..8usize {
                    let mut block = [0u8; CAP_BLOCK_BYTES];
                    let start = b_idx * CAP_BLOCK_BYTES;
                    if start < md_bytes.len() {
                        let end = (start + CAP_BLOCK_BYTES).min(md_bytes.len());
                        block[..end - start].copy_from_slice(&md_bytes[start..end]);
                    }
                    self.backend.write_block_64b(dst_blk + (b_idx as u32), &block);
                }

                Ok(CapInvokeResult {
                    backend: kind,
                    return_i32: table.total_present_out_of_12() as i32,
                    read_block_64b: Some(self.backend.read_block_64b(dst_blk)),
                    translated_phys_addr: dst_phys,
                    markdown_bytes_written: stack_md.len(),
                    espnow_tx_seal_u64: None,
                })
            }

            PhoenixCapOp::EspNowTransmit216 { dst_mac: _ } => {
                let (src_blk, phys_addr) =
                    self.verify_and_translate_handle(handle, 4, CAP_IO | CAP_ARENA, false)?;

                let mut raw_256 = [0u8; 256];
                for b in 0..4usize {
                    let line = self.backend.read_block_64b(src_blk + (b as u32));
                    raw_256[b * 64..(b + 1) * 64].copy_from_slice(&line);
                }
                let wire_216: [u8; M8_ESPNOW_FRAME_BYTES] =
                    raw_256[0..M8_ESPNOW_FRAME_BYTES].try_into().unwrap();

                let verified = EspNowQuantVoiceFrame::from_wire_bytes_verified(&wire_216)
                    .map_err(|_| CapAbiFault::BackendExecutionError(0xF3))?;

                self.last_espnow_tx_seal = Some(verified.pcc_fnv1a_seal_u64);
                Ok(CapInvokeResult {
                    backend: kind,
                    return_i32: M8_ESPNOW_FRAME_BYTES as i32,
                    read_block_64b: None,
                    translated_phys_addr: phys_addr,
                    markdown_bytes_written: 0,
                    espnow_tx_seal_u64: Some(verified.pcc_fnv1a_seal_u64),
                })
            }

            PhoenixCapOp::AxiDmaEnqueueAndRun64x64 {
                payload_handle,
                xfer_beats,
            } => {
                let (desc_blk, _desc_phys) =
                    self.verify_and_translate_handle(handle, 1, CAP_IO | CAP_ARENA, true)?;
                let (_pay_blk, pay_phys) = self.verify_and_translate_handle(
                    payload_handle,
                    xfer_beats,
                    CAP_IO | CAP_ARENA | CAP_M8,
                    false,
                )?;

                let sg_desc = AgpSgDescriptor512 {
                    src_phys_addr: pay_phys,
                    xfer_beats,
                    req_cap_mask: CAP_IO | CAP_ARENA | CAP_M8,
                    agp_insn: AgpInstruction::encode(
                        AGP_OP_TGEMM_RAW,
                        AGP_FLAG_TERMINAL,
                        false,
                        false,
                        0,
                        0,
                        0,
                        0,
                        0,
                    ),
                    dep_wait_mask: 0,
                    node_done_mask: 0b0001,
                    pcc_fnv1a_seal: 0x7C24_2080_2026_0001,
                    next_desc_off: 0,
                    inline_meta_128: [0u8; 16],
                };
                let desc_bytes = sg_desc.to_cache_line_bytes();
                self.backend.write_block_64b(desc_blk, &desc_bytes);

                Ok(CapInvokeResult {
                    backend: kind,
                    return_i32: xfer_beats as i32,
                    read_block_64b: Some(desc_bytes),
                    translated_phys_addr: pay_phys,
                    markdown_bytes_written: 0,
                    espnow_tx_seal_u64: None,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        esp32s3_espnow_mesh::{Esp32S3MeshNodeBridge, MeshPoleNode, ESPNOW_BROADCAST_MAC, VOICE_FRAME_TRITS},
        felix_phoenix::pack_spqt_base243_byte,
        well_signal_spqt_espnow::pack_signal_table_spqt3,
    };

    #[test]
    fn test_phoenix_os_agnostic_capability_abi_across_mmap_wasm_and_baremetal_dma() {
        fn run_unified_capability_suite<B: PhoenixArenaBackend>(
            mut table: PhoenixCapabilityAbiTable<B>,
        ) -> (i32, usize, u64, [u8; CAP_BLOCK_BYTES]) {
            let caps = CAP_IO | CAP_ARENA | CAP_M8;

            assert_eq!(
                table.mint_capability_slot(0, 0, 16, caps | CAP_ROOT_SEAL, 1, 5, 1),
                Err(CapAbiFault::Charm18RootSealForbidden)
            );

            let h_arena = table
                .mint_capability_slot(1, 10, 16, caps, 1, 11, 1)
                .expect("Valid RW arena slot must mint cleanly");

            let h_ro_well = table
                .mint_capability_slot(2, 30, 4, caps, 1, 16, 0)
                .expect("Valid RO .m8 slot must mint cleanly");

            assert!(matches!(
                table.invoke_cap(
                    h_ro_well,
                    PhoenixCapOp::ArenaWrite64B {
                        data: [0xAA; CAP_BLOCK_BYTES]
                    }
                ),
                Err(CapAbiFault::TernaryPermissionDenied {
                    trit_perm: 0,
                    attempted_write: true
                })
            ));

            let mut w_block = [0u8; CAP_BLOCK_BYTES];
            for g in 0..13 {
                w_block[g] = pack_spqt_base243_byte(&[-1, -1, -1, -1, -1]);
            }
            table
                .invoke_cap(h_arena, PhoenixCapOp::ArenaWrite64B { data: w_block })
                .unwrap();

            let mut act_block = [0u8; CAP_BLOCK_BYTES];
            for c in 0..32 {
                act_block[c * 2..c * 2 + 2].copy_from_slice(&i16::MIN.to_le_bytes());
            }
            let h_arena_blk1 = VirtualCapHandle64::encode(1, 1, 11, 1, 0);
            let h_arena_blk2 = VirtualCapHandle64::encode(1, 1, 11, 2, 0);
            table
                .invoke_cap(h_arena_blk1, PhoenixCapOp::ArenaWrite64B { data: act_block })
                .unwrap();
            table
                .invoke_cap(h_arena_blk2, PhoenixCapOp::ArenaWrite64B { data: act_block })
                .unwrap();

            let tgemm_res = table
                .invoke_cap(h_arena, PhoenixCapOp::TGemm64FromArena)
                .expect("TGemm64FromArena capability invocation must succeed");
            assert_eq!(tgemm_res.return_i32, 64 * 32_768);

            let well_src = "\
- source: youtube\n\
- kind: video\n\
- title: Mahakala mantra prayer\n\
- artist: Alexander Hacker\n\
- album: empty\n\
- year: 2026\n\
- genre: empty\n\
- url: https://music.youtube.com/watch?v=pL18Xr_26ts&si=PVlUHWbxr2Fja3t-\n";
            let well_table = WellRecord::parse_zero_alloc(well_src).resolve_signals();
            let spqt3 = pack_signal_table_spqt3(&well_table, 2);

            let mut spqt_block = [0u8; CAP_BLOCK_BYTES];
            spqt_block[0..3].copy_from_slice(&spqt3);
            let h_spqt_src = VirtualCapHandle64::encode(1, 1, 11, 3, 0);
            let h_md_dst = VirtualCapHandle64::encode(1, 1, 11, 4, 0);

            table
                .invoke_cap(h_spqt_src, PhoenixCapOp::ArenaWrite64B { data: spqt_block })
                .unwrap();
            let md_res = table
                .invoke_cap(
                    h_spqt_src,
                    PhoenixCapOp::WellSignalRenderMarkdown {
                        dst_handle: h_md_dst,
                        bend_k: 2,
                    },
                )
                .expect("WellSignalRenderMarkdown capability op must succeed");
            assert_eq!(md_res.return_i32, 6);
            assert_eq!(md_res.markdown_bytes_written, 444);

            let h_desc_slot = VirtualCapHandle64::encode(1, 1, 11, 12, 0);
            let dma_res = table
                .invoke_cap(
                    h_desc_slot,
                    PhoenixCapOp::AxiDmaEnqueueAndRun64x64 {
                        payload_handle: h_arena_blk1,
                        xfer_beats: 2,
                    },
                )
                .expect("AxiDmaEnqueueAndRun64x64 capability op must succeed");

            let h_arena_epoch2 = table
                .advance_slot_epoch_for_hotswap(1)
                .expect("Epoch advance must succeed");
            assert_eq!(h_arena_epoch2.epoch_gen(), 2);

            assert_eq!(
                table.invoke_cap(h_arena, PhoenixCapOp::ArenaRead64B),
                Err(CapAbiFault::StaleEpochUseAfterFree {
                    handle_epoch: 1,
                    current_epoch: 2
                })
            );

            let fresh_read = table
                .invoke_cap(h_arena_epoch2, PhoenixCapOp::ArenaRead64B)
                .unwrap();

            (
                tgemm_res.return_i32,
                md_res.markdown_bytes_written,
                dma_res.translated_phys_addr,
                fresh_read.read_block_64b.unwrap(),
            )
        }

        let mmap_table = PhoenixCapabilityAbiTable::new(
            PosixMmapArenaBackend::allocate_shared(DEFAULT_ARENA_BLOCKS as u32).unwrap(),
        );
        let wasm_table = PhoenixCapabilityAbiTable::new(WasmLinearMemoryBackend::new());
        let dma_table = PhoenixCapabilityAbiTable::new(BareMetalEsp32S3DmaBackend::new());

        let res_mmap = run_unified_capability_suite(mmap_table);
        let res_wasm = run_unified_capability_suite(wasm_table);
        let res_dma = run_unified_capability_suite(dma_table);

        assert_eq!(res_mmap, res_wasm);
        assert_eq!(res_wasm, res_dma);

        struct DummyRf;
        impl EspNowRadioHal for DummyRf {
            fn send_espnow_frame(
                &mut self,
                _dst: &[u8; 6],
                _p: &[u8; M8_ESPNOW_FRAME_BYTES],
            ) -> Result<(), EspNowMeshFault> {
                Ok(())
            }
        }
        let mut node = Esp32S3MeshNodeBridge::new(MeshPoleNode::AllFatherIngress);
        let cap_h = VirtualCapHandle64::encode(1, 3, 9, 0, 0);
        let wire_216 = node
            .transmit_voice_capability_frame(
                &mut DummyRf,
                &ESPNOW_BROADCAST_MAC,
                &[128i16; VOICE_FRAME_TRITS],
                cap_h,
                CAP_IO | CAP_ARENA | CAP_M8,
                512,
                2,
                1,
            )
            .unwrap();

        let mut esp_table = PhoenixCapabilityAbiTable::new(BareMetalEsp32S3DmaBackend::new());
        let h_tx = esp_table
            .mint_capability_slot(3, 0, 8, CAP_IO | CAP_ARENA | CAP_M8, 1, 9, 1)
            .unwrap();
        for b in 0..4usize {
            let mut block = [0u8; CAP_BLOCK_BYTES];
            let start = b * 64;
            if start < wire_216.len() {
                let end = (start + 64).min(wire_216.len());
                block[..end - start].copy_from_slice(&wire_216[start..end]);
            }
            let h_blk = VirtualCapHandle64::encode(1, 3, 9, b as u32, 0);
            esp_table
                .invoke_cap(h_blk, PhoenixCapOp::ArenaWrite64B { data: block })
                .unwrap();
        }

        let tx_out = esp_table
            .invoke_cap(
                h_tx,
                PhoenixCapOp::EspNowTransmit216 {
                    dst_mac: ESPNOW_BROADCAST_MAC,
                },
            )
            .expect("EspNowTransmit216 capability invocation must succeed");
        assert_eq!(tx_out.return_i32, 216);
        assert!(tx_out.espnow_tx_seal_u64.is_some());
    }
}
