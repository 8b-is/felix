//! `felix/crates/felix-lang/src/well_signal_spqt_espnow.rs`
//!
//! 3-Byte SPQT Base-243 Signal Table Packer (`[239, 205, 238]` Lap 1, `[239, 233, 104]` Lap 2)
//! with $\mathbb{Z}_3$ Parity Checksum Trit $t_{14}$ and ESP32-S3 ESP-NOW Capability Frame Embedding.

use crate::{
    agp_cap_iommu_tlb::{AgpCapIommuTlbSim, VirtualCapHandle64, TLB_STATUS_OK},
    agp_dma_ring_sim::SimulatedHostAxiMemory,
    esp32s3_espnow_mesh::{
        compute_frame_pcc_seal, encode_pcm640_to_quant_tern_pq, EspNowFrameKind, EspNowMeshFault,
        EspNowQuantVoiceFrame, EspNowRadioHal, MeshPoleNode, M8_ESPNOW_MAGIC, VOICE_FRAME_TRITS,
    },
    felix_phoenix::{pack_spqt_base243_byte, unpack_spqt_base243_byte, CapEffectMask},
    ljodatal_18_charms::CAP_ROOT_SEAL,
    well_signal_resolution::{
        RegisterKind, RegisterResolution, SignalResolutionTable, SignalTrit, StackMarkdownBuf,
        WellRecord,
    },
};

/// Exact byte length of the packed 4-register `SignalResolutionTable` in SPQT Base-243 (`3` bytes = `15` trits).
pub const WELL_SIGNAL_SPQT_BYTES: usize = 3;
/// Total trits stored in 3 SPQT Base-243 bytes (`12` register trits + `2` structural trits + `1` Z_3 parity trit).
pub const WELL_SIGNAL_TOTAL_TRITS: usize = 15;
/// Wire size of the ESP-NOW Capability + Voice + Well-Signal frame (`220` bytes <= `250`-byte ESP-NOW MTU).
pub const M8_ESPNOW_WELL_FRAME_BYTES: usize = 220;

/// Flag bit in `flags` (`bit 5`) indicating that `well_signal_spqt3` carries an active `SignalResolutionTable`.
pub const FLAG_HAS_WELL_SPQT3: u8 = 0x20;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WellSpqtDecodeError {
    InvalidBase243Byte { byte_index: usize, value: u8 },
    Z3ParityMismatch { sum_mod3: u8 },
    EspNowFault(EspNowMeshFault),
}

// ============================================================================
// 1. 15-Trit <-> 3-Byte SPQT Base-243 Packer & Unpacker (with Z_3 Parity & Bend)
// ============================================================================

/// Flattens a 4-register `SignalResolutionTable` into 15 balanced trits `{-1, 0, +1}`:
///   - `trits[0..3]`:   `handle` register (`handle`, `album`, `year`)
///   - `trits[3..6]`:   `name` register (`title`, `self-description`, `genre`)
///   - `trits[6..9]`:   `container` register (`kind`, `album`, `genre`)
///   - `trits[9..12]`:  `address` register (`url`, `duration`, `track count`)
///   - `trits[12]`:     `handle_is_unattached_title_case` (`+1` = true, `0` = handle empty, `-1` = qualified)
///   - `trits[13]`:     `is_thin_object` (`+1` = thin video object, `-1` = containerized, `0` = kind empty)
///   - `trits[14]`:     Exact $\mathbb{Z}_3$ parity trit ensuring $\sum_{i=0}^{14} (t_i + 1) \equiv 0 \pmod 3$.
pub const fn flatten_signal_table_to_15_trits(
    table: &SignalResolutionTable,
) -> [i8; WELL_SIGNAL_TOTAL_TRITS] {
    let mut trits = [0i8; WELL_SIGNAL_TOTAL_TRITS];

    let mut r = 0usize;
    while r < 4 {
        let mut c = 0usize;
        while c < 3 {
            trits[r * 3 + c] = table.registers[r].trits[c].as_i8();
            c += 1;
        }
        r += 1;
    }

    trits[12] = if table.registers[0].trits[0].as_i8() <= 0 {
        0
    } else if table.handle_is_unattached_title_case {
        1
    } else {
        -1
    };

    trits[13] = if table.registers[2].trits[0].as_i8() <= 0 {
        0
    } else if table.is_thin_object {
        1
    } else {
        -1
    };

    let mut sum_u: u8 = 0;
    let mut i = 0usize;
    while i < 14 {
        sum_u += (trits[i] + 1) as u8;
        i += 1;
    }
    let rem = sum_u % 3;
    let u14 = (3 - rem) % 3;
    trits[14] = (u14 as i8) - 1;

    trits
}

/// Packs a `SignalResolutionTable` into 3 SPQT Base-243 bytes (`[u8; 3]`, each in `0..=242`),
/// optionally applying the division-free `(bend_k * i) mod 3` cyclic RF spectral-whitening shift.
pub fn pack_signal_table_spqt3(
    table: &SignalResolutionTable,
    bend_k: u8,
) -> [u8; WELL_SIGNAL_SPQT_BYTES] {
    let raw_15 = flatten_signal_table_to_15_trits(table);
    let k_step = bend_k % 3;
    let mut r_mod3 = 0u8;
    let mut bent_15 = [0i8; WELL_SIGNAL_TOTAL_TRITS];

    for i in 0..WELL_SIGNAL_TOTAL_TRITS {
        let u_raw = (raw_15[i] + 1) as u8;
        let mut u_bent = u_raw + r_mod3;
        if u_bent >= 3 {
            u_bent -= 3;
        }
        bent_15[i] = (u_bent as i8) - 1;

        r_mod3 += k_step;
        if r_mod3 >= 3 {
            r_mod3 -= 3;
        }
    }

    [
        pack_spqt_base243_byte(&bent_15[0..5]),
        pack_spqt_base243_byte(&bent_15[5..10]),
        pack_spqt_base243_byte(&bent_15[10..15]),
    ]
}

/// Unpacks 3 SPQT Base-243 bytes (`[u8; 3]`), reverses the `(bend_k * i) mod 3` cyclic shift,
/// verifies the $\mathbb{Z}_3$ parity trit $t_{14}$, and reconstructs the exact `SignalResolutionTable`.
pub fn unpack_signal_table_spqt3(
    spqt3: &[u8; WELL_SIGNAL_SPQT_BYTES],
    bend_k: u8,
) -> Result<SignalResolutionTable, WellSpqtDecodeError> {
    for (idx, &b) in spqt3.iter().enumerate() {
        if b >= 243 {
            return Err(WellSpqtDecodeError::InvalidBase243Byte {
                byte_index: idx,
                value: b,
            });
        }
    }

    let k_step = bend_k % 3;
    let mut r_mod3 = 0u8;
    let mut unbent_15 = [0i8; WELL_SIGNAL_TOTAL_TRITS];

    for g in 0..WELL_SIGNAL_SPQT_BYTES {
        let five_bent = unpack_spqt_base243_byte(spqt3[g]);
        for m in 0..5 {
            let u_bent = (five_bent[m] + 1) as u8;
            let mut u_unbent = u_bent + 3 - r_mod3;
            if u_unbent >= 3 {
                u_unbent -= 3;
            }
            unbent_15[g * 5 + m] = (u_unbent as i8) - 1;

            r_mod3 += k_step;
            if r_mod3 >= 3 {
                r_mod3 -= 3;
            }
        }
    }

    let sum_u: u8 = unbent_15.iter().map(|&t| (t + 1) as u8).sum();
    if sum_u % 3 != 0 {
        return Err(WellSpqtDecodeError::Z3ParityMismatch {
            sum_mod3: sum_u % 3,
        });
    }

    let to_sig_trit = |v: i8| -> SignalTrit {
        match v {
            1 => SignalTrit::Present,
            -1 => SignalTrit::Contradictory,
            _ => SignalTrit::Empty,
        }
    };

    let registers = [
        RegisterResolution {
            register: RegisterKind::Handle,
            trits: [
                to_sig_trit(unbent_15[0]),
                to_sig_trit(unbent_15[1]),
                to_sig_trit(unbent_15[2]),
            ],
            labels: ["handle", "album", "year"],
        },
        RegisterResolution {
            register: RegisterKind::Name,
            trits: [
                to_sig_trit(unbent_15[3]),
                to_sig_trit(unbent_15[4]),
                to_sig_trit(unbent_15[5]),
            ],
            labels: ["title", "self-description", "genre"],
        },
        RegisterResolution {
            register: RegisterKind::Container,
            trits: [
                to_sig_trit(unbent_15[6]),
                to_sig_trit(unbent_15[7]),
                to_sig_trit(unbent_15[8]),
            ],
            labels: ["kind", "album", "genre"],
        },
        RegisterResolution {
            register: RegisterKind::Address,
            trits: [
                to_sig_trit(unbent_15[9]),
                to_sig_trit(unbent_15[10]),
                to_sig_trit(unbent_15[11]),
            ],
            labels: ["url", "duration", "track count"],
        },
    ];

    let handle_is_unattached_title_case = unbent_15[12] > 0;
    let is_thin_object = unbent_15[13] > 0;

    let mut pcc_seal_u64 = 0xCBF2_9CE4_8422_2325u64;
    for reg in &registers {
        pcc_seal_u64 ^= reg.cube27_index() as u64;
        pcc_seal_u64 = pcc_seal_u64.wrapping_mul(0x0000_0100_0000_01B3);
    }

    Ok(SignalResolutionTable {
        registers,
        handle_is_unattached_title_case,
        is_thin_object,
        pcc_seal_u64,
    })
}

/// Branchless ternary later-lap overruling directly on two 15-trit `SignalResolutionTable` states:
/// Any `Present (+1)` or updated non-empty trit in `later_lap` overrules `Empty (0)` in `earlier_lap`.
pub fn overrule_signal_table_direct(
    earlier: &SignalResolutionTable,
    later: &SignalResolutionTable,
) -> SignalResolutionTable {
    let mut merged_regs = earlier.registers;
    for r in 0..4 {
        for c in 0..3 {
            if later.registers[r].trits[c] != SignalTrit::Empty {
                merged_regs[r].trits[c] = later.registers[r].trits[c];
            }
        }
    }

    let has_album_or_genre = merged_regs[2].trits[1] == SignalTrit::Present
        || merged_regs[2].trits[2] == SignalTrit::Present;
    let has_kind = merged_regs[2].trits[0] == SignalTrit::Present;

    let mut pcc_seal_u64 = 0xCBF2_9CE4_8422_2325u64;
    for reg in &merged_regs {
        pcc_seal_u64 ^= reg.cube27_index() as u64;
        pcc_seal_u64 = pcc_seal_u64.wrapping_mul(0x0000_0100_0000_01B3);
    }

    SignalResolutionTable {
        registers: merged_regs,
        handle_is_unattached_title_case: later.handle_is_unattached_title_case
            || earlier.handle_is_unattached_title_case,
        is_thin_object: has_kind && !has_album_or_genre,
        pcc_seal_u64,
    }
}

// ============================================================================
// 2. 220-Byte ESP-NOW Capability + Voice + Well-Signal Frame (`EspNowWellCapabilityFrame`)
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EspNowWellCapabilityFrame {
    pub inner_voice: EspNowQuantVoiceFrame,
    /// 3 SPQT Base-243 bytes encoding the 12 register trits + 2 structural flags + 1 Z_3 parity trit.
    pub well_signal_spqt3: [u8; WELL_SIGNAL_SPQT_BYTES],
    /// Monotonic lap generation counter (`1` = Lap 1, `2` = Lap 2 overrule, etc.).
    pub well_lap_generation: u8,
    /// 64-bit FNV-1a Proof-Carrying Code seal covering bytes `0..212` (including `well_signal_spqt3`).
    pub frame_pcc_seal_u64: u64,
}

impl EspNowWellCapabilityFrame {
    /// Constructs a 220-byte ESP-NOW frame embedding both the 640-trit `quantTernP/Q` voice stream,
    /// the `.m8` `VirtualCapHandle64`, and the 3-byte SPQT `SignalResolutionTable`.
    #[allow(clippy::too_many_arguments)]
    pub fn from_voice_and_well_table(
        origin_pole: MeshPoleNode,
        seq_num: u16,
        virt_cap_handle: VirtualCapHandle64,
        declared_caps: CapEffectMask,
        triadic_vclock: [u32; 3],
        pcm_640: &[i16; VOICE_FRAME_TRITS],
        base_step_q: u16,
        bend_k: u8,
        command_id: u8,
        well_table: &SignalResolutionTable,
        well_lap_generation: u8,
    ) -> Result<Self, EspNowMeshFault> {
        if (declared_caps & CAP_ROOT_SEAL) != 0 {
            return Err(EspNowMeshFault::Charm18RootSealRfLeak);
        }

        let (pred_init, p_plane, q_plane, wave_q15) =
            encode_pcm640_to_quant_tern_pq(pcm_640, base_step_q, bend_k);
        let spqt3 = pack_signal_table_spqt3(well_table, bend_k);

        let inner_voice = EspNowQuantVoiceFrame {
            magic: M8_ESPNOW_MAGIC,
            frame_kind: EspNowFrameKind::M8CrdtCapabilitySync,
            bend_k: bend_k % 3,
            rok_invert_aett: true,
            origin_pole,
            seq_num,
            virt_cap_handle,
            declared_caps,
            predictor_init_i16: pred_init,
            base_step_q,
            triadic_vclock,
            sleipnir_wave_q15: wave_q15,
            p_plane_base243: p_plane,
            q_plane_e2m1: q_plane,
            futhark_tag_r2: virt_cap_handle.futhark_tag_r2(),
            command_id,
            pcc_fnv1a_seal_u64: 0,
        };

        let mut wrapper = Self {
            inner_voice,
            well_signal_spqt3: spqt3,
            well_lap_generation,
            frame_pcc_seal_u64: 0,
        };
        let wire = wrapper.to_wire_bytes_220();
        wrapper.frame_pcc_seal_u64 = u64::from_le_bytes(wire[212..220].try_into().unwrap());
        Ok(wrapper)
    }

    /// Serializes the complete frame into a 220-byte stack array (`<= 250`-byte ESP-NOW MTU).
    pub fn to_wire_bytes_220(&self) -> [u8; M8_ESPNOW_WELL_FRAME_BYTES] {
        let mut buf = [0u8; M8_ESPNOW_WELL_FRAME_BYTES];
        let base_216 = self.inner_voice.to_wire_bytes();

        buf[0..208].copy_from_slice(&base_216[0..208]);
        buf[4] |= FLAG_HAS_WELL_SPQT3;

        buf[208..211].copy_from_slice(&self.well_signal_spqt3);
        buf[211] = self.well_lap_generation;

        let seal = compute_frame_pcc_seal(&buf[0..212]);
        buf[212..220].copy_from_slice(&seal.to_le_bytes());
        buf
    }

    /// Verifies the 220-byte ESP-NOW wire frame, checks Charm XVIII (`CAP_ROOT_SEAL`),
    /// validates the 64-bit FNV-1a PCC seal, and unpacks + parity-checks the 3-byte SPQT
    /// `SignalResolutionTable`.
    pub fn from_wire_bytes_220_verified(
        buf: &[u8; M8_ESPNOW_WELL_FRAME_BYTES],
    ) -> Result<(Self, SignalResolutionTable), WellSpqtDecodeError> {
        if buf[0..4] != M8_ESPNOW_MAGIC {
            return Err(WellSpqtDecodeError::EspNowFault(
                EspNowMeshFault::InvalidMagicHeader,
            ));
        }

        let expected_seal = compute_frame_pcc_seal(&buf[0..212]);
        let wire_seal = u64::from_le_bytes(buf[212..220].try_into().unwrap());
        if wire_seal != expected_seal {
            return Err(WellSpqtDecodeError::EspNowFault(
                EspNowMeshFault::ByzantinePccSealMismatch {
                    expected: expected_seal,
                    received: wire_seal,
                },
            ));
        }

        let mut inner_buf = [0u8; 216];
        inner_buf[0..208].copy_from_slice(&buf[0..208]);
        inner_buf[4] &= !FLAG_HAS_WELL_SPQT3;
        let inner_seal = compute_frame_pcc_seal(&inner_buf[0..208]);
        inner_buf[208..216].copy_from_slice(&inner_seal.to_le_bytes());

        let inner_voice = EspNowQuantVoiceFrame::from_wire_bytes_verified(&inner_buf)
            .map_err(WellSpqtDecodeError::EspNowFault)?;

        let spqt3: [u8; 3] = [buf[208], buf[209], buf[210]];
        let well_table = unpack_signal_table_spqt3(&spqt3, inner_voice.bend_k)?;
        let well_lap_generation = buf[211];

        Ok((
            Self {
                inner_voice,
                well_signal_spqt3: spqt3,
                well_lap_generation,
                frame_pcc_seal_u64: wire_seal,
            },
            well_table,
        ))
    }

    /// Also embeds `well_signal_spqt3` directly into the existing 216-byte `EspNowQuantVoiceFrame`
    /// by storing the 3 SPQT bytes in the last 3 bytes of the Q-plane/trailer metadata when
    /// running in pure metadata-sync mode (`EspNowFrameKind::M8CrdtCapabilitySync`).
    pub fn embed_into_216_byte_frame(
        mut voice_frame: EspNowQuantVoiceFrame,
        well_table: &SignalResolutionTable,
        well_lap_generation: u8,
    ) -> [u8; 216] {
        let spqt3 = pack_signal_table_spqt3(well_table, voice_frame.bend_k);
        voice_frame.frame_kind = EspNowFrameKind::M8CrdtCapabilitySync;
        voice_frame.q_plane_e2m1[7] = spqt3[0];
        voice_frame.q_plane_e2m1[8] = spqt3[1];
        voice_frame.q_plane_e2m1[9] = spqt3[2];
        voice_frame.command_id = well_lap_generation;
        voice_frame.to_wire_bytes()
    }

    /// Extracts the 3-byte SPQT `SignalResolutionTable` and lap generation from a verified
    /// 216-byte `EspNowQuantVoiceFrame`.
    pub fn extract_from_216_byte_frame(
        voice_frame: &EspNowQuantVoiceFrame,
    ) -> Result<(SignalResolutionTable, u8), WellSpqtDecodeError> {
        let spqt3 = [
            voice_frame.q_plane_e2m1[7],
            voice_frame.q_plane_e2m1[8],
            voice_frame.q_plane_e2m1[9],
        ];
        let table = unpack_signal_table_spqt3(&spqt3, voice_frame.bend_k)?;
        Ok((table, voice_frame.command_id))
    }
}

/// Mesh Node Helper: Receives a 216-byte ESP-NOW Well Capability Sync frame on an
/// ESP32-S3 node, validates it through the 27-way Futhark IOMMU TLB, overrules the node's
/// stored `SignalResolutionTable` if `incoming_lap >= active_lap`, and renders the updated
/// Markdown table into a stack-allocated `StackMarkdownBuf` (`[u8; 512]`).
pub fn receive_and_overrule_well_on_esp32s3<R: EspNowRadioHal>(
    _radio: &mut R,
    wire_216: &[u8; 216],
    iommu: &mut AgpCapIommuTlbSim,
    host_mem: &SimulatedHostAxiMemory,
    active_table: &mut Option<(SignalResolutionTable, u8)>,
    well_context: &WellRecord<'_>,
) -> Result<StackMarkdownBuf, WellSpqtDecodeError> {
    let frame = EspNowQuantVoiceFrame::from_wire_bytes_verified(wire_216)
        .map_err(WellSpqtDecodeError::EspNowFault)?;

    let tlb_out = iommu.translate_handle(
        host_mem,
        frame.virt_cap_handle,
        1,
        frame.declared_caps,
        false,
    );
    if tlb_out.status != TLB_STATUS_OK {
        return Err(WellSpqtDecodeError::EspNowFault(
            EspNowMeshFault::IommuCapabilityFault(tlb_out.status),
        ));
    }

    let (incoming_table, incoming_lap) =
        EspNowWellCapabilityFrame::extract_from_216_byte_frame(&frame)?;

    let resolved_table = match *active_table {
        Some((prev_table, prev_lap)) if incoming_lap >= prev_lap => {
            let merged = overrule_signal_table_direct(&prev_table, &incoming_table);
            *active_table = Some((merged, incoming_lap));
            merged
        }
        Some((prev_table, _)) => prev_table,
        None => {
            *active_table = Some((incoming_table, incoming_lap));
            incoming_table
        }
    };

    let markdown_buf = resolved_table
        .render_to_stack_buf(well_context)
        .map_err(|_| WellSpqtDecodeError::EspNowFault(EspNowMeshFault::MtuExceeded(512)))?;

    Ok(markdown_buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        agp_cap_iommu_tlb::CapPte128,
        esp32s3_espnow_mesh::{Esp32S3MeshNodeBridge, ESPNOW_BROADCAST_MAC, M8_ESPNOW_FRAME_BYTES},
        felix_phoenix::{CAP_ARENA, CAP_IO, CAP_M8},
    };

    struct DummyRadio;
    impl EspNowRadioHal for DummyRadio {
        fn send_espnow_frame(
            &mut self,
            _dst_mac: &[u8; 6],
            _payload: &[u8; M8_ESPNOW_FRAME_BYTES],
        ) -> Result<(), EspNowMeshFault> {
            Ok(())
        }
    }

    const WELL_LAP_1: &str = r#"
- source: youtube
- kind: video
- title: Mahakala mantra prayer
- artist: Alexander Hacker
- album: empty
- year: 2026
- genre: empty
- url: https://music.youtube.com/watch?v=pL18Xr_26ts&si=PVlUHWbxr2Fja3t-
"#;

    #[test]
    fn test_well_signal_spqt3_packing_z3_parity_and_esp32s3_espnow_embedding() {
        let well_1 = WellRecord::parse_zero_alloc(WELL_LAP_1);
        let table_1 = well_1.resolve_signals();

        let spqt3_unbent_lap1 = pack_signal_table_spqt3(&table_1, 0);
        assert_eq!(spqt3_unbent_lap1, [239, 205, 238]);

        let unpacked_unbent = unpack_signal_table_spqt3(&spqt3_unbent_lap1, 0)
            .expect("Unbent 3-byte SPQT table must pass Z_3 parity and unpack cleanly");
        assert_eq!(unpacked_unbent, table_1);

        let spqt3_bent_lap1 = pack_signal_table_spqt3(&table_1, 2);
        let unpacked_bent = unpack_signal_table_spqt3(&spqt3_bent_lap1, 2)
            .expect("Bent (k=2) 3-byte SPQT table must pass Z_3 parity and unpack cleanly");
        assert_eq!(unpacked_bent, table_1);

        let mut corrupted_spqt3 = spqt3_unbent_lap1;
        corrupted_spqt3[1] += 1;
        assert!(matches!(
            unpack_signal_table_spqt3(&corrupted_spqt3, 0),
            Err(WellSpqtDecodeError::Z3ParityMismatch { .. })
        ));

        let cap_handle = VirtualCapHandle64::encode(1, 4, 12, 0, 0);
        let pcm_640 = [256i16; VOICE_FRAME_TRITS];

        let well_frame_220 = EspNowWellCapabilityFrame::from_voice_and_well_table(
            MeshPoleNode::AllFatherIngress,
            1,
            cap_handle,
            CAP_IO | CAP_ARENA | CAP_M8,
            [1, 0, 0],
            &pcm_640,
            512,
            2,
            0x01,
            &table_1,
            1,
        )
        .unwrap();

        let wire_220 = well_frame_220.to_wire_bytes_220();
        assert_eq!(wire_220.len(), M8_ESPNOW_WELL_FRAME_BYTES);
        let (decoded_220, decoded_table_1) =
            EspNowWellCapabilityFrame::from_wire_bytes_220_verified(&wire_220)
                .expect("220-byte ESP-NOW well capability frame must verify cleanly");
        assert_eq!(decoded_220.well_lap_generation, 1);
        assert_eq!(decoded_table_1, table_1);

        let mut host_mem = SimulatedHostAxiMemory::new();
        let mut iommu = AgpCapIommuTlbSim::new(0x0010_0000);
        iommu.install_pte_in_host_memory(
            &mut host_mem,
            &CapPte128 {
                phys_base_block: 0x8000_0000 >> 6,
                max_blocks: 32,
                allowed_caps: CAP_IO | CAP_ARENA | CAP_M8,
                expected_epoch: 1,
                cap_slot_id: 4,
                futhark_tag_r2: 12,
                trit_perm: 1,
                valid: true,
            },
        );

        let mut radio = DummyRadio;
        let mut node_allfather = Esp32S3MeshNodeBridge::new(MeshPoleNode::AllFatherIngress);
        let base_wire = node_allfather
            .transmit_voice_capability_frame(
                &mut radio,
                &ESPNOW_BROADCAST_MAC,
                &pcm_640,
                cap_handle,
                CAP_IO | CAP_ARENA | CAP_M8,
                512,
                2,
                1,
            )
            .unwrap();
        let base_frame = EspNowQuantVoiceFrame::from_wire_bytes_verified(&base_wire).unwrap();

        let wire_216_lap1 =
            EspNowWellCapabilityFrame::embed_into_216_byte_frame(base_frame, &table_1, 1);
        let mut active_state: Option<(SignalResolutionTable, u8)> = None;

        let md_lap1 = receive_and_overrule_well_on_esp32s3(
            &mut radio,
            &wire_216_lap1,
            &mut iommu,
            &host_mem,
            &mut active_state,
            &well_1,
        )
        .expect("Lap 1 ESP-NOW reception and stack markdown rendering must succeed");

        assert_eq!(md_lap1.len(), 444);
        assert_eq!(active_state.unwrap().0.total_present_out_of_12(), 6);

        let well_delta_2 =
            WellRecord::parse_zero_alloc("- genre: Devotional Chant\n- duration: 02:23\n");
        let well_2 = well_1.overrule_with_later_lap(well_delta_2);
        let table_2 = well_2.resolve_signals();
        assert_eq!(pack_signal_table_spqt3(&table_2, 0), [239, 233, 104]);

        let wire_216_lap2 =
            EspNowWellCapabilityFrame::embed_into_216_byte_frame(base_frame, &table_2, 2);
        let md_lap2 = receive_and_overrule_well_on_esp32s3(
            &mut radio,
            &wire_216_lap2,
            &mut iommu,
            &host_mem,
            &mut active_state,
            &well_2,
        )
        .expect("Lap 2 ESP-NOW overrule and stack markdown rendering must succeed");

        assert_eq!(active_state.unwrap().0.total_present_out_of_12(), 9);
        assert!(md_lap2.as_str().contains("= 3/3 |"));
    }
}
