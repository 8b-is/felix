//! 6-Step Statistical Inference Pipeline (`SelfOtherHypercubeEngine`)
//!
//! Models the 6 empirical inference tasks from Nisbett & Ross (*Human Inference*, 1980),
//! Daryl Bem's Self-Perception Theory, and Harold Kelley's Covariation Model:
//! 1. Collecting Data (`task1_collect`)
//! 2. Coding Data into balanced-ternary trits and 6-bit `Q_6` coordinates (`task2_code`)
//! 3. Recalling Coded Tuples (`task3_recall_coherence`)
//! 4. Assessing Covariations via Pearson correlation `r` (`task4_assess_covariation`)
//! 5. Inferring Causal Relations via interventional difference-in-means (`task5_infer_causal_relations`)
//! 6. Testing Hypotheses (`task6_test_hypotheses`)

/// Balanced-ternary trit in `{-1, 0, +1}`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(i8)]
pub enum Trit {
    SelfPole = -1,
    ZeroHole = 0,
    OtherPole = 1,
}

/// A single paired observation across two channels (`self_signal`, `other_signal`) and time (`timestamp_us`).
#[derive(Copy, Clone, Debug)]
pub struct SocialWaveObservation {
    pub timestamp_us: u64,
    pub self_signal: f64,
    pub other_signal: f64,
    pub intervention_do_active: bool,
}

/// Coded tuple representation with a 6-bit coordinate (`0..64`) in `Q_6`.
#[derive(Copy, Clone, Debug)]
pub struct CodedWaveTuple {
    pub vertex_q6: u8,
    pub self_trit: Trit,
    pub other_trit: Trit,
    pub phase_coherence: f64,
    pub causal_weight: f64,
}

/// Summary report of the 6-step inference pipeline.
#[derive(Clone, Debug)]
pub struct HypercubeInferenceCertificate {
    pub collected_count: usize,
    pub coded_tuples_count: usize,
    pub recalled_coherence_mean: f64,
    /// Pearson correlation `r` in `[-1.0, 1.0]`.
    pub covariation_r: f64,
    /// Interventional difference-in-means `E[Y | do(X=1)] - E[Y | do(X=0)]`.
    pub causal_do_effect: f64,
    /// Combined quadratic error and correlation score.
    pub hypothesis_free_energy_f: f64,
    /// Number of coded tuples with matching ternary quantization (`self_trit == other_trit`).
    pub open_hypercube_wormholes: usize,
    /// True when `recalled_coherence_mean > 0.85` and `covariation_r > 0.80`.
    pub self_other_isomorphism_verified: bool,
}

/// Executes the 6 inference tasks across paired observations.
pub struct SelfOtherHypercubeEngine {
    pub collected_buffer: Vec<SocialWaveObservation>,
    pub mem8_vault: Vec<CodedWaveTuple>,
}

impl Default for SelfOtherHypercubeEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SelfOtherHypercubeEngine {
    pub fn new() -> Self {
        Self {
            collected_buffer: Vec::new(),
            mem8_vault: Vec::new(),
        }
    }

    /// TASK 1: COLLECTING DATA — Appends a paired observation.
    pub fn task1_collect(&mut self, obs: SocialWaveObservation) {
        self.collected_buffer.push(obs);
    }

    /// Quantizes a continuous signal into `{-1, 0, +1}` using threshold `tau`.
    #[inline]
    fn quantize_trit(val: f64, tau: f64) -> Trit {
        if val > tau {
            Trit::OtherPole
        } else if val < -tau {
            Trit::SelfPole
        } else {
            Trit::ZeroHole
        }
    }

    /// TASK 2: CODING DATA — Encodes raw observations into 6-bit `Q_6` coordinates and trits.
    pub fn task2_code(&mut self, threshold_tau: f64) {
        self.mem8_vault.clear();
        for (idx, obs) in self.collected_buffer.iter().enumerate() {
            let s_trit = Self::quantize_trit(obs.self_signal, threshold_tau);
            let o_trit = Self::quantize_trit(obs.other_signal, threshold_tau);

            let b0 = u8::from(obs.self_signal >= 0.0);
            let b1 = u8::from(obs.other_signal >= 0.0);
            let b2 = u8::from((obs.self_signal - obs.other_signal).abs() < threshold_tau);
            let b3 = u8::from(obs.intervention_do_active);
            let b4 = u8::from(s_trit == o_trit);
            let b5 = u8::from(idx.is_multiple_of(2));
            let vertex_q6 = b0 | (b1 << 1) | (b2 << 2) | (b3 << 3) | (b4 << 4) | (b5 << 5);

            let diff = (obs.self_signal - obs.other_signal).abs();
            let coherence = (-diff).exp();

            self.mem8_vault.push(CodedWaveTuple {
                vertex_q6,
                self_trit: s_trit,
                other_trit: o_trit,
                phase_coherence: coherence,
                causal_weight: if obs.intervention_do_active { 1.0 } else { 0.5 },
            });
        }
    }

    /// TASK 3: RECALLING DATA — Computes mean coherence and count of matching trit pairs.
    pub fn task3_recall_coherence(&self) -> (f64, usize) {
        if self.mem8_vault.is_empty() {
            return (0.0, 0);
        }
        let sum_coh: f64 = self.mem8_vault.iter().map(|t| t.phase_coherence).sum();
        let matching = self
            .mem8_vault
            .iter()
            .filter(|t| t.self_trit == t.other_trit)
            .count();
        (sum_coh / (self.mem8_vault.len() as f64), matching)
    }

    /// TASK 4: ASSESSING COVARIATIONS — Computes Pearson correlation `r` between `self_signal` and `other_signal`.
    pub fn task4_assess_covariation(&self) -> f64 {
        let n = self.collected_buffer.len() as f64;
        if n < 2.0 {
            return 0.0;
        }
        let mean_s = self.collected_buffer.iter().map(|o| o.self_signal).sum::<f64>() / n;
        let mean_o = self.collected_buffer.iter().map(|o| o.other_signal).sum::<f64>() / n;

        let mut num = 0.0;
        let mut den_s = 0.0;
        let mut den_o = 0.0;
        for o in &self.collected_buffer {
            let ds = o.self_signal - mean_s;
            let do_ = o.other_signal - mean_o;
            num += ds * do_;
            den_s += ds * ds;
            den_o += do_ * do_;
        }
        let denom = (den_s * den_o).sqrt().max(1e-12);
        (num / denom).clamp(-1.0, 1.0)
    }

    /// TASK 5: INFERRING CAUSAL RELATIONS — Difference-in-means estimator across `intervention_do_active`.
    pub fn task5_infer_causal_relations(&self) -> f64 {
        let mut do_sum = 0.0;
        let mut do_cnt = 0usize;
        let mut obs_sum = 0.0;
        let mut obs_cnt = 0usize;

        for o in &self.collected_buffer {
            let joint_response = 0.5 * (o.self_signal + o.other_signal);
            if o.intervention_do_active {
                do_sum += joint_response;
                do_cnt += 1;
            } else {
                obs_sum += joint_response;
                obs_cnt += 1;
            }
        }
        let e_do = if do_cnt > 0 { do_sum / (do_cnt as f64) } else { 0.0 };
        let e_obs = if obs_cnt > 0 { obs_sum / (obs_cnt as f64) } else { 0.0 };
        e_do - e_obs
    }

    /// TASK 6: TESTING HYPOTHESES — Runs coding, recall, covariation, and causal estimation.
    pub fn task6_test_hypotheses(&mut self, threshold_tau: f64) -> HypercubeInferenceCertificate {
        self.task2_code(threshold_tau);
        let (recalled_coherence, wormholes) = self.task3_recall_coherence();
        let cov_r = self.task4_assess_covariation();
        let causal_effect = self.task5_infer_causal_relations();

        let prediction_error = (1.0 - recalled_coherence).max(0.0);
        let free_energy_f = 0.5 * prediction_error * prediction_error - 0.5 * cov_r.abs();

        HypercubeInferenceCertificate {
            collected_count: self.collected_buffer.len(),
            coded_tuples_count: self.mem8_vault.len(),
            recalled_coherence_mean: recalled_coherence,
            covariation_r: cov_r,
            causal_do_effect: causal_effect,
            hypothesis_free_energy_f: free_energy_f,
            open_hypercube_wormholes: wormholes,
            self_other_isomorphism_verified: recalled_coherence > 0.85 && cov_r > 0.80,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_six_task_hypercube_inference_pipeline() {
        let mut engine = SelfOtherHypercubeEngine::new();
        let samples = [
            (100, -0.8, -0.75, false),
            (200, -0.4, -0.38, false),
            (300, 0.0, 0.02, false),
            (400, 0.45, 0.42, true),
            (500, 0.85, 0.82, true),
            (600, 0.95, 0.92, true),
        ];
        for &(ts, s, o, do_act) in &samples {
            engine.task1_collect(SocialWaveObservation {
                timestamp_us: ts,
                self_signal: s,
                other_signal: o,
                intervention_do_active: do_act,
            });
        }

        let cert = engine.task6_test_hypotheses(0.2);
        assert_eq!(cert.collected_count, 6);
        assert_eq!(cert.coded_tuples_count, 6);
        assert!(cert.recalled_coherence_mean > 0.95);
        assert!(cert.covariation_r > 0.99);
        assert!(cert.causal_do_effect > 1.0);
        assert_eq!(cert.open_hypercube_wormholes, 6);
        assert!(cert.self_other_isomorphism_verified);
    }
}
