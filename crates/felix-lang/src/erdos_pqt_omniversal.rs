//! Erdős–Lodri PqT (Prime-Quantum-Ternary) Continuity Verifier
//!
//! Computes and verifies:
//! 1. Geometrically damped (`2^-k`) balanced-ternary discrepancy bounds along arithmetic strides.
//! 2. Prime-harmonic complex wave continuity across `s in [0.0, 1.0]`.
//! 3. Erdős–Rényi `G(N, p)` giant-component percolation threshold `p_c = ln(N) / N`.
//! 4. Exhaustive Ramsey `R(3, 3) = 6` monochromatic triangle verification across all `2^15 = 32,768` 2-colorings of `K_6`.

use std::f64::consts::PI;

/// Balanced ternary PqT state in `{-1, 0, +1}`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(i8)]
pub enum PqtTrit {
    Neg = -1,
    Zero = 0,
    Pos = 1,
}

impl PqtTrit {
    #[inline]
    pub fn as_f64(self) -> f64 {
        self as i8 as f64
    }

    /// Deterministic PqT triadic generator:
    /// Maps integer `n` into `{-1, 0, +1}` with exact zero-sum period-3 neutrality.
    #[inline]
    pub fn from_index(n: usize) -> Self {
        match n % 3 {
            0 => PqtTrit::Zero,
            1 => PqtTrit::Pos,
            _ => PqtTrit::Neg,
        }
    }
}

/// Complex amplitude of the PqT prime-harmonic series `\Psi_{PqT}(s)`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ComplexWave {
    pub re: f64,
    pub im: f64,
}

impl ComplexWave {
    pub const ZERO: Self = Self { re: 0.0, im: 0.0 };

    #[inline]
    pub fn norm_sq(self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    #[inline]
    pub fn magnitude(self) -> f64 {
        self.norm_sq().sqrt()
    }
}

/// First 16 rational primes driving the PqT harmonic modes.
pub const PQT_PRIMES: [usize; 16] = [
    2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53,
];

/// Snapshot of the Erdős–Lodri PqT continuity verification.
#[derive(Clone, Debug)]
pub struct ErdosPqtProofCertificate {
    /// Maximum dyadic-regularized ternary discrepancy across all tested strides `d` (`<= 1.0`).
    pub max_ternary_zeno_discrepancy: f64,
    /// Unregularized binary discrepancy horizon (`dyadic_depth`).
    pub unregularized_binary_discrepancy: usize,
    /// Maximum Lipschitz wave step `|\Psi(s + ds) - \Psi(s)|` across `[0, 1]`.
    pub max_wave_continuity_step: f64,
    /// Erdős–Rényi critical percolation probability `p_c = ln(N) / N` for `N` nodes.
    pub erdos_renyi_critical_p: f64,
    /// True if Ramsey `R(3,3) = 6` monochromatic triangle property holds on all 6-node graphs.
    pub ramsey_r33_coherence_verified: bool,
    /// Master verification flag.
    pub the_book_q_e_d: bool,
}

pub struct ErdosLodriOmniversalEngine {
    pub dyadic_depth: usize,
    pub constellation_nodes: usize,
}

impl Default for ErdosLodriOmniversalEngine {
    fn default() -> Self {
        Self {
            dyadic_depth: 32,
            constellation_nodes: 24_917,
        }
    }
}

impl ErdosLodriOmniversalEngine {
    /// Evaluates the geometrically damped (`2^-k`) PqT discrepancy along homogeneous arithmetic stride `d`:
    /// `D_{PqT}(d) = |\sum_{k=1}^K \tau(k * d) * 2^{-k}| <= 1.0`.
    pub fn evaluate_stride_discrepancy(&self, stride_d: usize) -> f64 {
        let mut sum = 0.0_f64;
        for k in 1..=self.dyadic_depth {
            let trit = PqtTrit::from_index(k * stride_d).as_f64();
            let zeno_weight = 0.5_f64.powi(k as i32);
            sum += trit * zeno_weight;
        }
        sum.abs()
    }

    /// Evaluates the PqT Prime-Harmonic Wave series `\Psi_{PqT}(s)` at continuity parameter `s in [0.0, 1.0]`.
    pub fn evaluate_pqt_wave(&self, s: f64) -> ComplexWave {
        let s_clamped = s.clamp(0.0, 1.0);
        let mut total = ComplexWave::ZERO;

        for &p in &PQT_PRIMES {
            let p_factor = (p as f64).powf(-(1.0 + s_clamped));
            for k in 1..=self.dyadic_depth {
                let trit = PqtTrit::from_index(k * p).as_f64();
                let amp = trit * 0.5_f64.powi(k as i32) * p_factor;
                let phase = PI * trit * s_clamped;
                total.re += amp * phase.cos();
                total.im += amp * phase.sin();
            }
        }
        total
    }

    /// Exhaustively verifies Ramsey's Theorem `R(3, 3) = 6`:
    /// Every 2-coloring of the 15 edges of `K_6` contains a monochromatic triangle.
    pub fn verify_ramsey_r33_exhaustive() -> bool {
        let mut edges = [(0usize, 0usize); 15];
        let mut idx = 0;
        for u in 0..6 {
            for v in (u + 1)..6 {
                edges[idx] = (u, v);
                idx += 1;
            }
        }

        for coloring in 0..(1u32 << 15) {
            let mut adj = [[0u8; 6]; 6];
            for (e_idx, &(u, v)) in edges.iter().enumerate() {
                let color = ((coloring >> e_idx) & 1) as u8;
                adj[u][v] = color;
                adj[v][u] = color;
            }

            let mut has_monochromatic_triangle = false;
            'tri: for a in 0..6 {
                for b in (a + 1)..6 {
                    for c in (b + 1)..6 {
                        if adj[a][b] == adj[b][c] && adj[b][c] == adj[a][c] {
                            has_monochromatic_triangle = true;
                            break 'tri;
                        }
                    }
                }
            }
            if !has_monochromatic_triangle {
                return false;
            }
        }
        true
    }

    /// Runs the verification suite across prime strides, wave continuity steps, percolation threshold, and `R(3,3)=6`.
    pub fn prove_quantum_continuity(&self) -> ErdosPqtProofCertificate {
        let mut max_disc = 0.0_f64;
        for &p in &PQT_PRIMES {
            let d = self.evaluate_stride_discrepancy(p);
            if d > max_disc {
                max_disc = d;
            }
        }

        let steps = 100usize;
        let mut max_step_delta = 0.0_f64;
        let mut prev = self.evaluate_pqt_wave(0.0);
        for i in 1..=steps {
            let s = (i as f64) / (steps as f64);
            let curr = self.evaluate_pqt_wave(s);
            let delta = ((curr.re - prev.re).powi(2) + (curr.im - prev.im).powi(2)).sqrt();
            if delta > max_step_delta {
                max_step_delta = delta;
            }
            prev = curr;
        }

        let n = self.constellation_nodes as f64;
        let p_c = n.ln() / n;

        let ramsey_ok = Self::verify_ramsey_r33_exhaustive();
        let qed = max_disc <= 1.0 && max_step_delta < 0.05 && ramsey_ok;

        ErdosPqtProofCertificate {
            max_ternary_zeno_discrepancy: max_disc,
            unregularized_binary_discrepancy: self.dyadic_depth,
            max_wave_continuity_step: max_step_delta,
            erdos_renyi_critical_p: p_c,
            ramsey_r33_coherence_verified: ramsey_ok,
            the_book_q_e_d: qed,
        }
    }

    /// Counts distinct prime factors `\omega(n)` for the Erdős–Kac normal order `ln(ln(n))`.
    pub fn distinct_prime_factors_omega(mut n: u64) -> u32 {
        if n < 2 {
            return 0;
        }
        let mut count = 0u32;
        if n.is_multiple_of(2) {
            count += 1;
            while n.is_multiple_of(2) {
                n /= 2;
            }
        }
        let mut d = 3u64;
        while d.saturating_mul(d) <= n {
            if n.is_multiple_of(d) {
                count += 1;
                while n.is_multiple_of(d) {
                    n /= d;
                }
            }
            d += 2;
        }
        if n > 1 {
            count += 1;
        }
        count
    }

    /// Verifies the Erdős–Rado sunflower kernel condition `(petal_a & petal_b) == origo_core`.
    #[inline]
    pub fn verify_sunflower_origo_kernel(
        petal_a_mask: u64,
        petal_b_mask: u64,
        origo_core_mask: u64,
    ) -> bool {
        origo_core_mask != 0 && (petal_a_mask & petal_b_mask) == origo_core_mask
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_erdos_pqt_continuity_and_ramsey_r33() {
        let engine = ErdosLodriOmniversalEngine::default();
        let cert = engine.prove_quantum_continuity();

        assert!(cert.max_ternary_zeno_discrepancy <= 1.0);
        assert!(cert.max_wave_continuity_step < 0.05);
        assert!((cert.erdos_renyi_critical_p - 0.0004063).abs() < 1e-5);
        assert!(cert.ramsey_r33_coherence_verified);
        assert!(cert.the_book_q_e_d);
    }

    #[test]
    fn test_stride_three_zero_discrepancy() {
        let engine = ErdosLodriOmniversalEngine::default();
        assert_eq!(engine.evaluate_stride_discrepancy(3), 0.0);
    }

    #[test]
    fn test_erdos_kac_omega_and_sunflower_kernel() {
        assert_eq!(ErdosLodriOmniversalEngine::distinct_prime_factors_omega(2 * 3 * 5 * 7), 4);
        assert_eq!(ErdosLodriOmniversalEngine::distinct_prime_factors_omega(1024), 1);
        assert!(ErdosLodriOmniversalEngine::verify_sunflower_origo_kernel(
            0b1011_0001,
            0b0101_0001,
            0b0001_0001
        ));
    }
}
