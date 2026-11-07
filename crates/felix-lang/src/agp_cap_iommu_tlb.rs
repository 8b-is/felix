//! `felix/crates/felix-lang/src/agp_cap_iommu_tlb.rs`
//!
//! 27-Way Futhark Cube IOMMU TLB & 7-Gate Zero-Trust Capability Handle Translator (`VirtualCapHandle64`).

use crate::{
    agp_dma_ring_sim::SimulatedHostAxiMemory, felix_phoenix::CapEffectMask,
    ljodatal_18_charms::CAP_ROOT_SEAL,
};
use serde::{Deserialize, Serialize};

pub const TLB_STATUS_OK: u8 = 0x00;
pub const TLB_FAULT_UNALIGNED: u8 = 0x01;
pub const TLB_FAULT_CHARM18_SEAL: u8 = 0x02;
pub const TLB_FAULT_TERNARY_GUARD: u8 = 0x03;
pub const TLB_FAULT_EPOCH_STALE: u8 = 0x04;
pub const TLB_FAULT_FUTHARK_TAG: u8 = 0x05;
pub const TLB_FAULT_SLICE_BOUNDS: u8 = 0x06;
pub const TLB_FAULT_CAP_RIGHTS: u8 = 0x07;

/// 64-bit Virtual Capability Handle (`VirtualCapHandle64`):
/// - `[63:56]`: `epoch_gen` (`u8`)
/// - `[55:48]`: `cap_slot_id` (`u8`)
/// - `[47:43]`: `futhark_tag_r2` (`0..=26`)
/// - `[37:6]`:  `block_offset` (`u32`)
/// - `[5:0]`:   `byte_align` (`0..=63`, must be `0` for 64B cache-line access)
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VirtualCapHandle64(pub u64);

impl VirtualCapHandle64 {
    #[inline(always)]
    pub const fn encode(
        epoch_gen: u8,
        cap_slot_id: u8,
        futhark_tag_r2: u8,
        block_offset: u32,
        byte_align: u8,
    ) -> Self {
        let ep = (epoch_gen as u64) << 56;
        let sl = (cap_slot_id as u64) << 48;
        let tg = ((futhark_tag_r2 as u64) & 0x1F) << 43;
        let bl = (block_offset as u64) << 6;
        let al = (byte_align as u64) & 0x3F;
        Self(ep | sl | tg | bl | al)
    }

    #[inline(always)]
    pub const fn epoch_gen(self) -> u8 {
        ((self.0 >> 56) & 0xFF) as u8
    }

    #[inline(always)]
    pub const fn cap_slot_id(self) -> u8 {
        ((self.0 >> 48) & 0xFF) as u8
    }

    #[inline(always)]
    pub const fn futhark_tag_r2(self) -> u8 {
        ((self.0 >> 43) & 0x1F) as u8
    }

    #[inline(always)]
    pub const fn block_offset(self) -> u32 {
        ((self.0 >> 6) & 0xFFFF_FFFF) as u32
    }

    #[inline(always)]
    pub const fn byte_align(self) -> u8 {
        (self.0 & 0x3F) as u8
    }
}

/// 128-bit Capability Page Table Entry (`CapPte128`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapPte128 {
    pub phys_base_block: u64,
    pub max_blocks: u32,
    pub allowed_caps: CapEffectMask,
    pub expected_epoch: u8,
    pub cap_slot_id: u8,
    pub futhark_tag_r2: u8,
    pub trit_perm: i8,
    pub valid: bool,
}

impl Default for CapPte128 {
    fn default() -> Self {
        Self {
            phys_base_block: 0,
            max_blocks: 0,
            allowed_caps: 0,
            expected_epoch: 0,
            cap_slot_id: 0,
            futhark_tag_r2: 0,
            trit_perm: -1,
            valid: false,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TlbTranslationOutput {
    pub status: u8,
    pub phys_addr: u64,
    pub tlb_hit: bool,
}

/// 27-Way (`3^3`) Futhark Cube Associative IOMMU TLB Simulator.
pub struct AgpCapIommuTlbSim {
    pub page_table_phys_base: u64,
    pub ways_27: [CapPte128; 27],
    pub slots_cache: [CapPte128; 256],
}

impl AgpCapIommuTlbSim {
    pub fn new(page_table_phys_base: u64) -> Self {
        Self {
            page_table_phys_base,
            ways_27: [CapPte128::default(); 27],
            slots_cache: [CapPte128::default(); 256],
        }
    }

    pub fn preload_tlb_way(&mut self, way_idx: usize, pte: CapPte128) {
        if way_idx < 27 {
            self.ways_27[way_idx] = pte;
        }
        self.slots_cache[pte.cap_slot_id as usize] = pte;
    }

    pub fn shootdown_slot(&mut self, slot_id: u8) {
        for way in &mut self.ways_27 {
            if way.valid && way.cap_slot_id == slot_id {
                way.valid = false;
            }
        }
        self.slots_cache[slot_id as usize].valid = false;
    }

    pub fn install_pte_in_host_memory(
        &mut self,
        _host_mem: &mut SimulatedHostAxiMemory,
        pte: &CapPte128,
    ) {
        let way = (pte.futhark_tag_r2 as usize) % 27;
        self.preload_tlb_way(way, *pte);
    }

    /// Evaluates the 7-Gate Zero-Trust IOMMU Check on a `VirtualCapHandle64` against `pte`.
    pub const fn evaluate_7gate_check(
        handle: VirtualCapHandle64,
        xfer_blocks: u32,
        req_caps: CapEffectMask,
        is_write: bool,
        pte: &CapPte128,
    ) -> u8 {
        // Gate 1: 64-byte cache-line alignment
        if handle.byte_align() != 0 {
            return TLB_FAULT_UNALIGNED;
        }
        // Gate 2: Charm XVIII (CAP_ROOT_SEAL) non-export
        if (req_caps & CAP_ROOT_SEAL) != 0 || (pte.allowed_caps & CAP_ROOT_SEAL) != 0 {
            return TLB_FAULT_CHARM18_SEAL;
        }
        // Gate 3: Ternary Permission Guard {-1: Guard, 0: RO, +1: RW}
        if !pte.valid || pte.trit_perm < 0 || (is_write && pte.trit_perm <= 0) {
            return TLB_FAULT_TERNARY_GUARD;
        }
        // Gate 4: Temporal Epoch Generation Match (Zero Use-After-Free)
        if handle.cap_slot_id() != pte.cap_slot_id || handle.epoch_gen() != pte.expected_epoch {
            return TLB_FAULT_EPOCH_STALE;
        }
        // Gate 5: 3^3 = 27 Futhark Cube Tag Match
        if handle.futhark_tag_r2() != pte.futhark_tag_r2 {
            return TLB_FAULT_FUTHARK_TAG;
        }
        // Gate 6: 33-bit Carry-Safe Slice Bounds Check
        let end_blocks = (handle.block_offset() as u64) + (xfer_blocks as u64);
        if xfer_blocks == 0 || end_blocks > (pte.max_blocks as u64) {
            return TLB_FAULT_SLICE_BOUNDS;
        }
        // Gate 7: Capability Effect Mask Subset Check
        if (req_caps & pte.allowed_caps) != req_caps {
            return TLB_FAULT_CAP_RIGHTS;
        }
        TLB_STATUS_OK
    }

    pub fn translate_handle(
        &mut self,
        _host_mem: &SimulatedHostAxiMemory,
        handle: VirtualCapHandle64,
        xfer_blocks: u32,
        req_caps: CapEffectMask,
        is_write: bool,
    ) -> TlbTranslationOutput {
        let way_idx = (handle.futhark_tag_r2() as usize) % 27;
        let way_pte = self.ways_27[way_idx];
        let (pte, hit) = if way_pte.valid && way_pte.cap_slot_id == handle.cap_slot_id() {
            (way_pte, true)
        } else {
            (self.slots_cache[handle.cap_slot_id() as usize], false)
        };

        let status = Self::evaluate_7gate_check(handle, xfer_blocks, req_caps, is_write, &pte);
        let phys_addr = if status == TLB_STATUS_OK {
            (pte.phys_base_block + (handle.block_offset() as u64)) << 6
        } else {
            0
        };
        TlbTranslationOutput {
            status,
            phys_addr,
            tlb_hit: hit,
        }
    }
}
