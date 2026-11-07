//! `src/fpga_axis_ternary.rs`
//!
//! Cycle-Accurate Rust RTL Co-Simulator & AXI4-Stream Driver for `felix_ternary_64x64_axis_top`.

use crate::{
    felix_phoenix::{
        felix_ternary_gemm_i32, pack_spqt_base243_byte, CapEffectMask, CAP_ARENA, CAP_IO, CAP_M8,
    },
    ljodatal_18_charms::CAP_ROOT_SEAL,
    omni_subsystems::futhark_cipher_simd::RokCipherKey,
};
use serde::{Deserialize, Serialize};

pub const AXIS_CMD_LOAD_WEIGHT_ROW: u8 = 0x1;
pub const AXIS_CMD_EXEC_TGEMM_RAW: u8 = 0x2;
pub const AXIS_CMD_EXEC_TGEMM_PQ: u8 = 0x3;
pub const AXIS_CMD_DECODE_FUTHARK27: u8 = 0x4;
pub const AXIS_CMD_EXEC_NAND_ARRAY: u8 = 0x5;

pub const AXIS_STATUS_OK: u8 = 0x0;
pub const AXIS_STATUS_YIELD_HOTSWAP: u8 = 0x1;
pub const AXIS_STATUS_CAP_VIOLATION: u8 = 0xE;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AxisBeat512 {
    pub tdata: [u8; 64],
    pub tuser: u16,
    pub tlast: bool,
}

impl Serialize for AxisBeat512 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AxisBeat512", 3)?;
        s.serialize_field("tdata", &&self.tdata[..])?;
        s.serialize_field("tuser", &self.tuser)?;
        s.serialize_field("tlast", &self.tlast)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for AxisBeat512 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            tdata: Vec<u8>,
            tuser: u16,
            tlast: bool,
        }
        let h = Helper::deserialize(deserializer)?;
        let mut arr = [0u8; 64];
        let len = h.tdata.len().min(64);
        arr[..len].copy_from_slice(&h.tdata[..len]);
        Ok(AxisBeat512 {
            tdata: arr,
            tuser: h.tuser,
            tlast: h.tlast,
        })
    }
}

/// Exact quarter-unit Q2.2 integer numerator `M in {-12 .. +12}` matching `decode_e2m1_q2_num` in SV.
#[inline]
pub const fn decode_e2m1_q2_numerator(nibble: u8) -> i32 {
    let mag = match nibble & 0x07 {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        5 => 6,
        6 => 8,
        _ => 12,
    };
    if (nibble & 0x08) != 0 {
        -mag
    } else {
        mag
    }
}

/// Cycle-accurate RTL model of `rok_futhark_27_spqt_decoder` (Module 1 in `felix_ternary_64x64_axis.sv`).
pub fn rtl_rok_futhark_27_spqt_decoder(
    b_in: u8,
    rok_enable: bool,
    key: RokCipherKey,
    base_phase_mod3: u8,
) -> (u8, u8, [i8; 5]) {
    let b_valid = if b_in <= 242 { b_in } else { 0 };

    let ge_81 = u8::from(b_valid >= 81);
    let ge_162 = u8::from(b_valid >= 162);
    let t4_raw = ge_81 + ge_162;
    let r3 = b_valid - 81 * t4_raw;

    let ge_27 = u8::from(r3 >= 27);
    let ge_54 = u8::from(r3 >= 54);
    let t3_raw = ge_27 + ge_54;
    let r2_raw = r3 - 27 * t3_raw;

    let t0_raw = r2_raw % 3;
    let t1_raw = (r2_raw / 3) % 3;
    let t2_raw = r2_raw / 9;

    let inv_mod3 = |x: u8| -> u8 { (3 - (x % 3)) % 3 };
    let add3_mod3 = |a: u8, b: u8, c: u8| -> u8 { (a + b + c) % 3 };

    let mut u_trit = [0u8; 5];
    if rok_enable {
        let inv_k0 = inv_mod3(key.shift_stave_k0);
        let inv_k1 = inv_mod3(key.shift_row_k1);
        let inv_k2 = inv_mod3(key.shift_aett_k2);
        let inv_pos = inv_mod3(base_phase_mod3);

        u_trit[0] = add3_mod3(t0_raw, inv_k0, inv_pos);
        u_trit[1] = add3_mod3(t1_raw, inv_k1, inv_pos);
        let aett_uninv = if key.rok_invert_aett {
            (2 + 3 - t2_raw) % 3
        } else {
            t2_raw
        };
        u_trit[2] = add3_mod3(aett_uninv, inv_k2, inv_pos);
        u_trit[3] = t3_raw;
        u_trit[4] = t4_raw;
    } else {
        let raw_5 = [t0_raw, t1_raw, t2_raw, t3_raw, t4_raw];
        let mut ph = base_phase_mod3 % 3;
        for m in 0..5 {
            u_trit[m] = add3_mod3(raw_5[m], inv_mod3(ph), 0);
            ph = add3_mod3(ph, key.bend_k % 3, 0);
        }
    }

    let futhark_r2 = u_trit[0] + 3 * u_trit[1] + 9 * u_trit[2];
    let rok_q2 = t3_raw + 3 * t4_raw;
    let bal_trits: [i8; 5] = core::array::from_fn(|m| (u_trit[m] as i8) - 1);

    (futhark_r2, rok_q2, bal_trits)
}

/// Cycle-accurate RTL model of `felix_ternary_64x64_axis_top`.
pub struct FpgaTernary64x64AxisCore {
    pub weight_trits: [[i8; 64]; 64],
    pub q_e2m1_lo: [u8; 64],
    pub q_e2m1_hi: [u8; 64],
    pub hotswap_pending: bool,
    pub granted_caps: CapEffectMask,
    pub required_caps: CapEffectMask,
    pub clock_cycles_elapsed: u64,
}

impl FpgaTernary64x64AxisCore {
    pub fn new(granted_caps: CapEffectMask, required_caps: CapEffectMask) -> Self {
        Self {
            weight_trits: [[0i8; 64]; 64],
            q_e2m1_lo: [4u8; 64], // nibble 4 = 1.0
            q_e2m1_hi: [4u8; 64],
            hotswap_pending: false,
            granted_caps,
            required_caps,
            clock_cycles_elapsed: 0,
        }
    }

    #[inline]
    pub fn cap_authorized(&self) -> bool {
        ((self.required_caps & self.granted_caps) == self.required_caps)
            && ((self.required_caps & CAP_ROOT_SEAL) == 0)
    }

    /// Formats a 64-trit row (`trits[0..64]`), two 4-bit E2M1 nibbles (`q_lo`, `q_hi`), and a
    /// `RokCipherKey` into a single 512-bit AXI4-Stream `AXIS_CMD_LOAD_WEIGHT_ROW` beat.
    pub fn build_load_row_beat(
        row_idx: usize,
        trits: &[i8; 64],
        q_lo_nibble: u8,
        q_hi_nibble: u8,
        rok_enable: bool,
        key: RokCipherKey,
    ) -> AxisBeat512 {
        let mut tdata = [0u8; 64];
        // Pack 64 trits into 13 SPQT Base-243 bytes (bytes 0..13)
        // If bend_k is enabled, rotate trits before packing so unbending recovers them
        let pow3 = [1u8, 3, 9, 27, 81];
        for (g, chunk) in trits.chunks(5).enumerate() {
            let mut acc = 0u8;
            for (m, &t) in chunk.iter().enumerate() {
                let u = (t.clamp(-1, 1) + 1) as u8;
                let phase = if !rok_enable && key.bend_k > 0 {
                    let i = g * 5 + m;
                    ((key.bend_k as usize * i) % 3) as u8
                } else {
                    0
                };
                let bent_u = (u + phase) % 3;
                acc += bent_u * pow3[m];
            }
            tdata[g] = acc;
        }
        // Byte 13 holds `{q_hi_nibble[3:0], q_lo_nibble[3:0]}`
        tdata[13] = (q_lo_nibble & 0x0F) | ((q_hi_nibble & 0x0F) << 4);
        // Byte 14 holds `{rok_enable, rok_invert_aett, bend_k[1:0], 4'b0000}`
        tdata[14] = ((u8::from(rok_enable)) << 7)
            | ((u8::from(key.rok_invert_aett)) << 6)
            | ((key.bend_k & 0x03) << 4);
        // Byte 15 holds `{2'b00, shift_k2[1:0], shift_k1[1:0], shift_k0[1:0]}`
        tdata[15] = (key.shift_stave_k0 & 0x03)
            | ((key.shift_row_k1 & 0x03) << 2)
            | ((key.shift_aett_k2 & 0x03) << 4);

        let tuser = (AXIS_CMD_LOAD_WEIGHT_ROW as u16) | (((row_idx as u16) & 0x3F) << 4);
        AxisBeat512 {
            tdata,
            tuser,
            tlast: true,
        }
    }

    /// Processes a single-beat AXI4-Stream command (`LOAD_WEIGHT_ROW`, `DECODE_FUTHARK27`, or `EXEC_NAND_ARRAY`).
    pub fn send_single_beat(&mut self, beat: &AxisBeat512) -> Option<AxisBeat512> {
        self.clock_cycles_elapsed += 1;

        if !self.cap_authorized() {
            let mut reply = [0u8; 64];
            for chunk in reply.chunks_exact_mut(4) {
                chunk.copy_from_slice(&0xDEAD_CA90u32.to_le_bytes());
            }
            return Some(AxisBeat512 {
                tdata: reply,
                tuser: AXIS_STATUS_CAP_VIOLATION as u16,
                tlast: true,
            });
        }

        if self.hotswap_pending {
            return Some(AxisBeat512 {
                tdata: [0u8; 64],
                tuser: AXIS_STATUS_YIELD_HOTSWAP as u16,
                tlast: true,
            });
        }

        let cmd = (beat.tuser & 0x0F) as u8;
        let row_idx = ((beat.tuser >> 4) & 0x3F) as usize;

        let rok_byte = beat.tdata[14];
        let rok_enable = (rok_byte & 0x80) != 0;
        let rok_inv_aett = (rok_byte & 0x40) != 0;
        let bend_k = (rok_byte >> 4) & 0x03;
        let shift_byte = beat.tdata[15];
        let key = RokCipherKey {
            shift_stave_k0: shift_byte & 0x03,
            shift_row_k1: (shift_byte >> 2) & 0x03,
            shift_aett_k2: (shift_byte >> 4) & 0x03,
            bend_k,
            rok_invert_aett: rok_inv_aett,
        };

        match cmd {
            AXIS_CMD_LOAD_WEIGHT_ROW => {
                let mut row_trits = [0i8; 64];
                for g in 0..13 {
                    let group_mod3 = ((g * 5) % 3) as u8;
                    let base_phase = (bend_k * group_mod3) % 3;
                    let (_r2, _q2, five) =
                        rtl_rok_futhark_27_spqt_decoder(beat.tdata[g], rok_enable, key, base_phase);
                    for m in 0..5 {
                        if g * 5 + m < 64 {
                            row_trits[g * 5 + m] = five[m];
                        }
                    }
                }
                self.weight_trits[row_idx] = row_trits;
                self.q_e2m1_lo[row_idx] = beat.tdata[13] & 0x0F;
                self.q_e2m1_hi[row_idx] = (beat.tdata[13] >> 4) & 0x0F;
                None
            }
            AXIS_CMD_DECODE_FUTHARK27 => {
                let mut reply = [0u8; 64];
                for g in 0..16 {
                    let group_mod3 = ((g * 5) % 3) as u8;
                    let base_phase = (bend_k * group_mod3) % 3;
                    // For g >= 14, if not overwritten in tdata, use cipher byte
                    let (r2, _q2, _five) =
                        rtl_rok_futhark_27_spqt_decoder(beat.tdata[g], rok_enable, key, base_phase);
                    reply[g] = r2;
                }
                Some(AxisBeat512 {
                    tdata: reply,
                    tuser: AXIS_STATUS_OK as u16,
                    tlast: true,
                })
            }
            AXIS_CMD_EXEC_NAND_ARRAY => {
                let mut pos_mask = 0u64;
                let mut neg_mask = 0u64;
                for c in 0..64 {
                    let a = self.weight_trits[0][c];
                    let b = self.weight_trits[1][c];
                    let nand_t = -(a.min(b));
                    if nand_t == 1 {
                        pos_mask |= 1u64 << c;
                    } else if nand_t == -1 {
                        neg_mask |= 1u64 << c;
                    }
                }
                let mut reply = [0u8; 64];
                reply[0..8].copy_from_slice(&pos_mask.to_le_bytes());
                reply[8..16].copy_from_slice(&neg_mask.to_le_bytes());
                Some(AxisBeat512 {
                    tdata: reply,
                    tuser: AXIS_STATUS_OK as u16,
                    tlast: true,
                })
            }
            _ => None,
        }
    }

    /// Streams a 64-element `i16` activation vector (`2` input beats of 512 bits) into the
    /// $64 \times 64$ contraction array and returns the `4` output AXI4-Stream beats
    /// (`64 x i32` results, with `tlast == true` on beat 3).
    pub fn execute_tgemm_64x64_stream(
        &mut self,
        acts: &[i16; 64],
        use_pq_e2m1_scale: bool,
    ) -> Result<([i32; 64], Vec<AxisBeat512>), u8> {
        // 2 input beats + 1 eval cycle + 4 output beats = 7 clock cycles
        self.clock_cycles_elapsed += 7;

        if !self.cap_authorized() {
            return Err(AXIS_STATUS_CAP_VIOLATION);
        }
        if self.hotswap_pending {
            return Err(AXIS_STATUS_YIELD_HOTSWAP);
        }

        let mut results = [0i32; 64];
        for r in 0..64 {
            // Exact 17-bit sign-extended PE accumulation across cols 0..31 and cols 32..63
            let sum_lo = felix_ternary_gemm_i32(&acts[0..32], &self.weight_trits[r][0..32]);
            let sum_hi = felix_ternary_gemm_i32(&acts[32..64], &self.weight_trits[r][32..64]);

            results[r] = if use_pq_e2m1_scale {
                let m_lo = decode_e2m1_q2_numerator(self.q_e2m1_lo[r]);
                let m_hi = decode_e2m1_q2_numerator(self.q_e2m1_hi[r]);
                sum_lo * m_lo + sum_hi * m_hi
            } else {
                sum_lo + sum_hi
            };
        }

        // Pack 64 i32 outputs into 4 AXI4-Stream 512-bit beats (16 i32 words per beat)
        let mut out_beats = Vec::with_capacity(4);
        for beat_idx in 0..4usize {
            let mut tdata = [0u8; 64];
            for lane in 0..16usize {
                let val = results[beat_idx * 16 + lane];
                tdata[lane * 4..(lane + 1) * 4].copy_from_slice(&val.to_le_bytes());
            }
            let tuser = (AXIS_STATUS_OK as u16) | ((beat_idx as u16) << 4);
            out_beats.push(AxisBeat512 {
                tdata,
                tuser,
                tlast: beat_idx == 3,
            });
        }

        Ok((results, out_beats))
    }
}
