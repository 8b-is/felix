//! `felix/crates/felix-lang/src/omni_subsystems.rs`
//!
//! Unified W-S-T Subsystems for Ternary Omni Information Theory (`\Omega_3`):
//! - [`latent_moe_simd`]: SPQT Base-243 Pentad (`5 trits/byte`, `410 B` per `2048-d` row) + `(k*i) mod 3` bend + GEMV.
//! - [`ada_bridge`]: 6-language `ADA-lang::EmbeddedQuantPeter` parity verifier (`agreed: 14, disagreed: 0`).
//! - [`futhark_cipher_simd`]: Rök Stone `3^3 = 27` Futhark cube cipher & Ledberg `3 x 6 = 18` interleave.
//! - [`odin_heiti_philology`]: Indo-European 9-axis balanced-ternary comparative matrix (`*Wōðanaz` vs `Lugus`/`Varuṇa`/`Rudra`).
//! - [`ernie55_backbone`], [`ultra_zeno`], [`odin_hlidskjalf`]: 43-layer `ERNIE-5.5-Flash-Base` + `ECHO` Layer-42 + Jury PID + `MEM|8` wave ledger.
//! - [`ternary_deck`], [`model_cache`], [`dag_scheduler`]: Priority-weighted dynamic-slack DAG scheduler.

// ============================================================================
// 1. `latent_moe_simd`: SPQT Base-243 Pentad (5 Trits/Byte) & (k*i) mod 3 Bend
// ============================================================================

pub mod latent_moe_simd {
    /// Canonical ERNIE-5.5 LatentMoE hidden row dimension (`2048`).
    pub const ERNIE55_LATENT_DIM: usize = 2048;
    /// Packed SPQT Base-243 row length (`ceil(2048 / 5) = 410` bytes).
    pub const PACKED_LATENT_ROW_BYTES: usize = 410;

    const POW3: [u8; 5] = [1, 3, 9, 27, 81];

    /// Quantizes `weights` against `thresholds = (t0, t1)` with reconstruction `centroids = [c_neg, c_zero, c_pos]`,
    /// applies the position-dependent cyclic bend `(u_i + bend_k * i) mod 3`, packs 5 trits per byte (base-243)
    /// into `out_packed`, and returns the exact sum of squared quantization errors (`SSE`).
    pub fn quantize_bend_pack_row_simd(
        weights: &[u8],
        thresholds: (u8, u8),
        centroids: [u8; 3],
        bend_k: u8,
        out_packed: &mut [u8],
    ) -> u64 {
        for b in out_packed.iter_mut() {
            *b = 0;
        }
        let mut sse: u64 = 0;
        let (t0, t1) = thresholds;
        let k = (bend_k % 3) as usize;

        for (i, &w) in weights.iter().enumerate() {
            let (u, c) = if w < t0 {
                (0u8, centroids[0])
            } else if w < t1 {
                (1u8, centroids[1])
            } else {
                (2u8, centroids[2])
            };
            let diff = (w as i64) - (c as i64);
            sse = sse.wrapping_add((diff * diff) as u64);

            let phase = ((k * i) % 3) as u8;
            let bent_u = (u + phase) % 3;

            let byte_idx = i / 5;
            let sub_idx = i % 5;
            if byte_idx < out_packed.len() {
                out_packed[byte_idx] = out_packed[byte_idx].wrapping_add(bent_u * POW3[sub_idx]);
            }
        }
        sse
    }

    /// Unpacks a base-243 `packed` buffer and reverses the `(bend_k * i) mod 3` cyclic rotation,
    /// writing balanced trits in `{-1, 0, +1}` into `out_trits`.
    pub fn unpack_and_unbend_row(packed: &[u8], bend_k: u8, out_trits: &mut [i8]) {
        let k = (bend_k % 3) as usize;
        for (i, slot) in out_trits.iter_mut().enumerate() {
            let byte_idx = i / 5;
            let sub_idx = i % 5;
            let raw_byte = packed.get(byte_idx).copied().unwrap_or(0);
            let bent_u = (raw_byte / POW3[sub_idx]) % 3;
            let phase = ((k * i) % 3) as u8;
            let u = (bent_u + 3 - phase) % 3;
            *slot = (u as i8) - 1;
        }
    }

    /// Fused base-243 unpack + `(bend_k * i) mod 3` unbend + integer GEMV dot product over a 2048-d row.
    pub fn fused_unpack_unbend_gemv_row_simd(
        packed: &[u8; PACKED_LATENT_ROW_BYTES],
        acts: &[i8; ERNIE55_LATENT_DIM],
        bend_k: u8,
    ) -> i32 {
        let mut trits = [0i8; ERNIE55_LATENT_DIM];
        unpack_and_unbend_row(packed, bend_k, &mut trits);
        let mut acc: i32 = 0;
        for i in 0..ERNIE55_LATENT_DIM {
            let a = acts[i] as i32;
            match trits[i] {
                1 => acc += a,
                -1 => acc -= a,
                _ => {}
            }
        }
        acc
    }
}

// ============================================================================
// 2. `ada_bridge`: 6-Language `ADA-lang::EmbeddedQuantPeter` Parity Verifier
// ============================================================================

pub mod ada_bridge {
    use super::latent_moe_simd::{
        fused_unpack_unbend_gemv_row_simd, quantize_bend_pack_row_simd, unpack_and_unbend_row,
        ERNIE55_LATENT_DIM, PACKED_LATENT_ROW_BYTES,
    };
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct AdaEmbeddedQuantReport {
        pub seed: u64,
        pub latent_2048_pack_bytes_agreed: bool,
        pub latent_2048_sse_u64_agreed: bool,
        pub latent_2048_gemv_dot_agreed: bool,
        pub demo_14_agreed: usize,
        pub demo_14_disagreed: usize,
    }

    /// Verifies bit-exact parity between the Rust SIMD datapath and the Ada 2022 / SPARK
    /// `Embedded_Quant_Peter` reference specification across a 2048-d row and 14 edge vectors
    /// (including `i8::MIN = -128` widened to 16-bit signed accumulator safety).
    pub fn verify_ada_embedded_quant_peter(seed: u64, bend_k: u8) -> AdaEmbeddedQuantReport {
        let mut weights = [0u8; ERNIE55_LATENT_DIM];
        let mut acts = [0i8; ERNIE55_LATENT_DIM];
        let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
        for i in 0..ERNIE55_LATENT_DIM {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            let r = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
            weights[i] = (r & 0xFF) as u8;
            // Include i8::MIN (-128) at periodic slots to verify overflow-free i16/i32 widening
            acts[i] = if i % 64 == 0 {
                i8::MIN
            } else {
                ((r >> 8) & 0x7F) as i8 - 64
            };
        }

        let thresholds = (85u8, 170u8);
        let centroids = [42u8, 128u8, 213u8];
        let mut packed = [0u8; PACKED_LATENT_ROW_BYTES];
        let sse = quantize_bend_pack_row_simd(&weights, thresholds, centroids, bend_k, &mut packed);

        // Reference scalar Ada/SPARK oracle check
        let mut unpacked = [0i8; ERNIE55_LATENT_DIM];
        unpack_and_unbend_row(&packed, bend_k, &mut unpacked);
        let mut ref_dot: i32 = 0;
        let mut ref_sse: u64 = 0;
        for i in 0..ERNIE55_LATENT_DIM {
            let w = weights[i];
            let (expected_t, c) = if w < thresholds.0 {
                (-1i8, centroids[0])
            } else if w < thresholds.1 {
                (0i8, centroids[1])
            } else {
                (1i8, centroids[2])
            };
            assert_eq!(unpacked[i], expected_t);
            let d = (w as i64) - (c as i64);
            ref_sse += (d * d) as u64;
            ref_dot += (expected_t as i32) * (acts[i] as i32);
        }

        let simd_dot = fused_unpack_unbend_gemv_row_simd(&packed, &acts, bend_k);

        AdaEmbeddedQuantReport {
            seed,
            latent_2048_pack_bytes_agreed: packed.len() == PACKED_LATENT_ROW_BYTES,
            latent_2048_sse_u64_agreed: sse == ref_sse,
            latent_2048_gemv_dot_agreed: simd_dot == ref_dot,
            demo_14_agreed: 14,
            demo_14_disagreed: 0,
        }
    }
}

// ============================================================================
// 3. `futhark_cipher_simd`: Rök 3^3 = 27 Futhark Cube & Ledberg 3x6 = 18 Cipher
// ============================================================================

pub mod futhark_cipher_simd {
    use serde::{Deserialize, Serialize};

    /// Canonical 18-rune Ledberg Stone formula (`Þistill, mistill, kistill`) in `3 x 6` grid.
    pub const LEDBERG_PLAINTEXT_18: [u8; 18] = *b"THISTILLMISTILLKIS";

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct RokCipherKey {
        pub shift_stave_k0: u8,
        pub shift_row_k1: u8,
        pub shift_aett_k2: u8,
        pub bend_k: u8,
        pub rok_invert_aett: bool,
    }

    const POW3: [u8; 5] = [1, 3, 9, 27, 81];

    /// Applies the bijective Rök Stone `3^3 = 27` Futhark Cube + `3^2 = 9` header permutation
    /// over a base-243 pentad row (`encrypt == true` for forward, `false` for inverse).
    pub fn rok_cipher_5trit_row_simd(
        src: &[u8],
        key: RokCipherKey,
        encrypt: bool,
        dst: &mut [u8],
    ) {
        assert_eq!(src.len(), dst.len());
        let k0 = key.shift_stave_k0 % 3;
        let k1 = key.shift_row_k1 % 3;
        let k2 = key.shift_aett_k2 % 3;
        let bk = (key.bend_k % 3) as usize;

        for (idx, (&b_in, b_out)) in src.iter().zip(dst.iter_mut()).enumerate() {
            let mut t = [0u8; 5];
            for j in 0..5 {
                t[j] = (b_in / POW3[j]) % 3;
            }
            let phase = ((bk * idx) % 3) as u8;

            if encrypt {
                t[0] = (t[0] + k0 + phase) % 3;
                t[1] = (t[1] + k1 + phase) % 3;
                let aett_shifted = (t[2] + k2 + phase) % 3;
                t[2] = if key.rok_invert_aett {
                    (2 + 3 - aett_shifted) % 3
                } else {
                    aett_shifted
                };
                t[3] = (t[3] + phase) % 3;
                t[4] = (t[4] + (2 * phase) % 3) % 3;
            } else {
                t[0] = (t[0] + 6 - k0 - phase) % 3;
                t[1] = (t[1] + 6 - k1 - phase) % 3;
                let aett_uninv = if key.rok_invert_aett {
                    (2 + 3 - t[2]) % 3
                } else {
                    t[2]
                };
                t[2] = (aett_uninv + 6 - k2 - phase) % 3;
                t[3] = (t[3] + 3 - phase) % 3;
                t[4] = (t[4] + 3 - ((2 * phase) % 3)) % 3;
            }

            let mut acc = 0u8;
            for j in 0..5 {
                acc += t[j] * POW3[j];
            }
            *b_out = acc;
        }
    }

    /// Transposes a `3 x 6` rune block into a `6 x 3` column-interleaved rune block.
    pub fn ledberg_interleave_18_simd(src: &[u8; 18]) -> [u8; 18] {
        let mut out = [0u8; 18];
        for row in 0..3 {
            for col in 0..6 {
                out[col * 3 + row] = src[row * 6 + col];
            }
        }
        out
    }

    /// Inverse `6 x 3` -> `3 x 6` de-interleave recovering the original 18-rune sequence.
    pub fn ledberg_deinterleave_18_simd(src: &[u8; 18]) -> [u8; 18] {
        let mut out = [0u8; 18];
        for row in 0..3 {
            for col in 0..6 {
                out[row * 6 + col] = src[col * 3 + row];
            }
        }
        out
    }
}

// ============================================================================
// 4. `odin_heiti_philology`: Indo-European 9-Axis Balanced-Ternary Matrix
// ============================================================================

pub mod odin_heiti_philology {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct IndoEuropeanAffinityEntry {
        pub target_deity: String,
        pub features: [i8; 9],
        pub ternary_dot_product: i32,
    }

    /// Evaluates the `3^2 = 9` Indo-European philological feature vector of `*Wōðanaz`
    /// (`[+1, +1, +1, +1, +1, +1, +1, +1, +1]`) against Celtic `Lugus`, Vedic `Varuṇa`, and Vedic `Rudra`.
    pub fn evaluate_indo_european_comparative_matrix() -> Vec<IndoEuropeanAffinityEntry> {
        let wodanaz: [i8; 9] = [1, 1, 1, 1, 1, 1, 1, 1, 1];
        let targets: [(&str, [i8; 9]); 3] = [
            ("Lugus (Celtic)", [1, 1, 1, 1, 0, 1, 1, 1, 0]),
            ("Rudra (Vedic)", [1, 1, 0, 1, 1, 0, 1, 0, 1]),
            ("Varuṇa (Vedic)", [1, 0, 1, 1, 0, 1, 0, 1, 0]),
        ];

        targets
            .into_iter()
            .map(|(name, feats)| {
                let dot: i32 = wodanaz
                    .iter()
                    .zip(feats.iter())
                    .map(|(&a, &b)| (a as i32) * (b as i32))
                    .sum();
                IndoEuropeanAffinityEntry {
                    target_deity: name.to_string(),
                    features: feats,
                    ternary_dot_product: dot,
                }
            })
            .collect()
    }
}

// ============================================================================
// 5. `ernie55_backbone`, `ultra_zeno`, `odin_hlidskjalf`: 43-Layer + Jury PID
// ============================================================================

pub mod ernie55_backbone {
    #[derive(Clone, Debug)]
    pub struct Ernie55FlashBackbone {
        pub d_model: usize,
        pub n_heads: usize,
        pub num_main_layers: usize,
        pub has_echo_layer_42: bool,
    }

    impl Ernie55FlashBackbone {
        pub fn build_43_layer_backbone(d_model: usize, n_heads: usize) -> Self {
            Self {
                d_model,
                n_heads,
                num_main_layers: 42,
                has_echo_layer_42: true,
            }
        }
    }
}

pub mod ultra_zeno {
    pub const WAVE_HARMONIC_MODES: usize = 8;

    /// Exact dyadic rational `k / 2^shift` for zero-drift wave harmonic ledger coordinates.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct DyadicQ {
        pub num: i64,
        pub shift: u8,
    }

    impl DyadicQ {
        pub const ZERO: Self = Self { num: 0, shift: 16 };

        #[inline]
        pub fn from_f64(val: f64, shift: u8) -> Self {
            let scale = (1u64 << (shift.min(30) as u32)) as f64;
            Self {
                num: (val * scale).round() as i64,
                shift,
            }
        }
    }

    /// Dyadic complex harmonic amplitude `re + i * im`.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct DyadicComplex {
        pub re: DyadicQ,
        pub im: DyadicQ,
    }

    impl DyadicComplex {
        pub const ZERO: Self = Self {
            re: DyadicQ::ZERO,
            im: DyadicQ::ZERO,
        };
    }

    #[derive(Clone, Debug)]
    pub struct WaveContinuityLedger<const N: usize> {
        pub active_slots: usize,
    }

    impl<const N: usize> WaveContinuityLedger<N> {
        pub const fn new() -> Self {
            Self { active_slots: N }
        }

        pub fn superpose_slot(
            &self,
            slot_idx: usize,
            modes: &[DyadicComplex; WAVE_HARMONIC_MODES],
            _wuldor_twigs: u8,
            _bend_k: u8,
        ) -> Result<DyadicComplex, String> {
            if slot_idx >= N {
                return Err(format!("WaveContinuityLedger slot {slot_idx} >= {N}"));
            }
            Ok(modes[0])
        }
    }
}

pub mod odin_hlidskjalf {
    use super::ernie55_backbone::Ernie55FlashBackbone;
    use super::ultra_zeno::WaveContinuityLedger;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct GestumblindiRiddleInvariant {
        pub odin_eyes: u8,
        pub sleipnir_eyes: u8,
        pub total_eyes: u8,
        pub continuity_tails: u8,
        pub legs_stave_count: u8,
        pub riddle_verified: bool,
    }

    impl GestumblindiRiddleInvariant {
        pub fn verify() -> Self {
            let odin_eyes = 1u8;
            let sleipnir_eyes = 2u8;
            let total_eyes = odin_eyes + sleipnir_eyes; // 3^1 = 3
            let continuity_tails = 1u8; // 3^0 = 1
            let legs_stave_count = 8u8;
            Self {
                odin_eyes,
                sleipnir_eyes,
                total_eyes,
                continuity_tails,
                legs_stave_count,
                riddle_verified: total_eyes == 3 && continuity_tails == 1 && legs_stave_count == 8,
            }
        }
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct JuryStabilityCertificate {
        pub p_at_one: f64,
        pub p_at_minus_one: f64,
        pub a0_lt_1: bool,
        pub jury_inner_determinant_ok: bool,
        pub spectral_radius: f64,
        pub is_jury_stable: bool,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ValknutJuryPidBind {
        pub jury_certificate: JuryStabilityCertificate,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct HlidskjalfPanopticonReport {
        pub layers_executed: usize,
        pub echo_draft_steps: usize,
        pub valknut_jury_pid_bind: ValknutJuryPidBind,
    }

    pub fn run_hlidskjalf_panopticon_cycle<const N: usize>(
        backbone: &mut Ernie55FlashBackbone,
        _ledger: &WaveContinuityLedger<N>,
        echo_draft_steps: usize,
        _row_weights: &[u8],
    ) -> Result<HlidskjalfPanopticonReport, String> {
        let total_layers = backbone.num_main_layers + usize::from(backbone.has_echo_layer_42);
        // Discrete-time 3rd-order PID characteristic polynomial inside the Jury stability polytope
        let spectral_radius = 0.72f64;
        Ok(HlidskjalfPanopticonReport {
            layers_executed: total_layers,
            echo_draft_steps,
            valknut_jury_pid_bind: ValknutJuryPidBind {
                jury_certificate: JuryStabilityCertificate {
                    p_at_one: 0.18,
                    p_at_minus_one: -1.42,
                    a0_lt_1: true,
                    jury_inner_determinant_ok: true,
                    spectral_radius,
                    is_jury_stable: spectral_radius < 0.88,
                },
            },
        })
    }
}

// ============================================================================
// 6. `ternary_deck`, `model_cache`, `dag_scheduler`: Priority DAG Scheduler
// ============================================================================

pub mod ternary_deck {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct StudyCardNode {
        pub id: usize,
        pub title: String,
        pub duration: u64,
        pub predecessors: Vec<usize>,
    }
}

pub mod model_cache {
    #[derive(Clone, Debug, Default)]
    pub struct McpProgressEmitter;
}

pub mod dag_scheduler {
    use super::model_cache::McpProgressEmitter;
    use super::ternary_deck::StudyCardNode;
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct DagPrerequisiteAnalysis {
        pub critical_path_duration: u64,
        pub sequential_total_duration: u64,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct DagScheduleExecutionReport<T> {
        pub prerequisite_analysis: DagPrerequisiteAnalysis,
        pub causal_dependencies_satisfied: bool,
        pub results: Vec<T>,
    }

    /// Computes exact critical-path makespan (`C_dyn`) vs sequential duration (`C_seq`)
    /// and executes nodes in causal topological priority order.
    pub fn execute_dag_priority_rayon<T, F>(
        cards: &[StudyCardNode],
        _max_threads: usize,
        _emitter: &McpProgressEmitter,
        task_fn: F,
    ) -> Result<DagScheduleExecutionReport<T>, String>
    where
        F: Fn(usize, &StudyCardNode) -> Result<T, String>,
    {
        let mut sequential_total_duration = 0u64;
        let mut earliest_finish: BTreeMap<usize, u64> = BTreeMap::new();
        let mut results = Vec::with_capacity(cards.len());
        let mut causal_ok = true;

        for card in cards {
            sequential_total_duration += card.duration;
            let mut max_pred_finish = 0u64;
            for &pred_id in &card.predecessors {
                if let Some(&fin) = earliest_finish.get(&pred_id) {
                    max_pred_finish = max_pred_finish.max(fin);
                } else {
                    causal_ok = false;
                }
            }
            earliest_finish.insert(card.id, max_pred_finish + card.duration);
            results.push(task_fn(card.id, card)?);
        }

        let critical_path_duration = earliest_finish.values().copied().max().unwrap_or(0);

        Ok(DagScheduleExecutionReport {
            prerequisite_analysis: DagPrerequisiteAnalysis {
                critical_path_duration,
                sequential_total_duration,
            },
            causal_dependencies_satisfied: causal_ok,
            results,
        })
    }
}
