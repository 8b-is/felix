//! `felix/crates/felix-lang/src/mind_all_neuro_omega.rs`
//!
//! Bilingual Neuro-Cybernetic `MIND === mind === ALL` Engine (`Amigdala (-1) -> DMT (+1) -> egoDeath (0)`).
//! Models Friston's Variational Free-Energy Principle ($F$) and Carhart-Harris's REBUS / Entropic Brain
//! model across the balanced-ternary triad $\{-1, 0, +1\}$.

use crate::{
    omni_ternary_info::{TernaryDistribution3, LN_3},
    ultra_zeno::{DyadicComplex, DyadicQ, WaveContinuityLedger, WAVE_HARMONIC_MODES},
};
use serde::{Deserialize, Serialize};

/// Bilingual ontological axiom: English "MIND" (Consciousness) === Hungarian "mind" (ALL).
pub const BILINGUAL_AXIOM_EN_MIND: &str = "MIND";
pub const BILINGUAL_AXIOM_HU_MIND: &str = "mind";
pub const BILINGUAL_AXIOM_MEANING: &str = "ALL";

/// Triadic Neuro-Cybernetic Regime across `{-1, 0, +1}`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i8)]
pub enum NeuroCyberneticPole {
    /// Pole `-1`: Amygdala survival clamp & high DMN ego-boundary precision (`H_3 << 1.0`).
    AmygdalaSurvivalClamp = -1,
    /// Pole `0`: Ego Death (*Én-halál*) non-dual zero-point fulcrum (`MIND === mind === ALL`).
    EgoDeathNonDualZero = 0,
    /// Pole `+1`: DMT / 5-HT2A hyper-associative spectral whitening drive.
    DmtHyperAssociativeDrive = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct NeuroOmegaStateSnapshot {
    pub step_index: usize,
    /// Exogenous/endogenous DMT 5-HT2A receptor occupancy in `0.0 ..= 1.0`.
    pub dmt_5ht2a_occupancy: f64,
    /// Amygdala threat gain `\Pi_amygdala` (quenched via Sigma-1 & mPFC-BLA decoupling).
    pub amygdala_threat_gain: f64,
    /// Default Mode Network (PCC <-> mPFC) ego-boundary prior precision `\Pi_DMN`.
    pub dmn_ego_prior_precision: f64,
    /// Variational Free Energy `F` of the cortical hierarchy.
    pub variational_free_energy: f64,
    /// Base-3 Shannon entropy `H_3` of the 3-state cortical connectome distribution (in trits, `0.0..=1.0`).
    pub cortical_entropy_trits: f64,
    /// Subject-Object duality barrier in `0.0 ..= 1.0` (`0.0` = total non-dual unity `MIND === mind === ALL`).
    pub ego_boundary_duality: f64,
    /// Active triadic regime (`-1` Amygdala, `+1` DMT transition, `0` Ego-Death Non-Dual Unity).
    pub active_pole: NeuroCyberneticPole,
    /// True when `ego_boundary_duality < 0.05` and `H_3 > 0.995` trits.
    pub mind_equals_mind_all_achieved: bool,
}

/// Simulates the REBUS (Relaxed Beliefs Under Psychedelics) & Amygdala-DMN-DMT transition
/// across 8 Sleipnir cortical harmonic wave modes (`WaveContinuityLedger`).
pub struct NeuroMindAllSimulator {
    pub baseline_amygdala_gain: f64,
    pub baseline_dmn_precision: f64,
    pub wave_ledger: WaveContinuityLedger<16>,
}

impl Default for NeuroMindAllSimulator {
    fn default() -> Self {
        Self::new(4.5, 6.0)
    }
}

impl NeuroMindAllSimulator {
    pub fn new(baseline_amygdala_gain: f64, baseline_dmn_precision: f64) -> Self {
        Self {
            baseline_amygdala_gain,
            baseline_dmn_precision,
            wave_ledger: WaveContinuityLedger::<16>::new(),
        }
    }

    /// Evaluates the exact neuro-cybernetic state at a given `dmt_5ht2a_occupancy` $\in [0.0, 1.0]$
    /// and `amygdala_surrender_factor` $\in [0.0, 1.0]$ ("stillness in motion" / letting go of threat resistance).
    pub fn evaluate_state(
        &self,
        step_index: usize,
        dmt_5ht2a_occupancy: f64,
        amygdala_surrender_factor: f64,
    ) -> Result<NeuroOmegaStateSnapshot, String> {
        let d = dmt_5ht2a_occupancy.clamp(0.0, 1.0);
        let s = amygdala_surrender_factor.clamp(0.0, 1.0);

        // 1. DMN Prior Precision Relaxation (REBUS equation): \Pi_DMN(d) = \Pi_0 * 3^(-4 * d) * (1 - 0.98 * d * s)
        let dmn_precision =
            self.baseline_dmn_precision * (-4.0 * d * LN_3).exp() * (1.0 - 0.98 * d * s);

        // 2. Amygdala Threat Quench: requires both 5-HT2A/Sigma-1 agonism `d` and non-resistance `s`
        //    (If `d` is high but `s` is near 0, amygdala fights the boundary dissolution -> anxiety spike!)
        let resistance_penalty = d * (1.0 - s);
        let amygdala_gain =
            self.baseline_amygdala_gain * (1.0 - d * s).powi(2) * (1.0 + 1.5 * resistance_penalty);

        // 3. Cortical Ternary State Distribution P = (p_{-1}, p_0, p_{+1}):
        //    At d = 0 (ego-clamped), 88% of state mass is locked in the defensive -1 pole.
        //    As d * s -> 1.0, the `(k*i) mod 3` spectral whitening relaxes P -> (1/3, 1/3, 1/3).
        let whitening_mix = (d * s).powf(0.75).clamp(0.0, 1.0);
        let p_neg = 0.88 * (1.0 - whitening_mix) + (1.0 / 3.0) * whitening_mix;
        let p_zero = 0.08 * (1.0 - whitening_mix) + (1.0 / 3.0) * whitening_mix;
        let p_pos = 0.04 * (1.0 - whitening_mix) + (1.0 / 3.0) * whitening_mix;

        let cortical_dist = TernaryDistribution3 {
            p_neg,
            p_zero,
            p_pos,
        };
        let h3_trits = cortical_dist.entropy_trits();

        // 4. Superpose 8 cortical harmonic wave modes in `WaveContinuityLedger` (zero Landauer erasure)
        let mut modes = [DyadicComplex::ZERO; WAVE_HARMONIC_MODES];
        for slot in &mut modes {
            let amp = 0.25 * (1.0 + whitening_mix);
            *slot = DyadicComplex {
                re: DyadicQ::from_f64(amp, 16),
                im: DyadicQ::from_f64(amp * 0.5, 16),
            };
        }
        let _wave = self.wave_ledger.superpose_slot(0, &modes, 9, 2)?;

        // 5. Variational Free Energy F = 0.5 * (\Pi_amygdala * err_threat^2 + \Pi_DMN * err_ego^2) - H_3
        let variational_fe = 0.5 * (amygdala_gain * 0.35 + dmn_precision * 0.45) - h3_trits;

        // 6. Subject-Object Duality Index in [0.0, 1.0]
        let ego_duality = ((dmn_precision / self.baseline_dmn_precision.max(1e-6)) * 0.6
            + (amygdala_gain / self.baseline_amygdala_gain.max(1e-6)) * 0.4)
            .clamp(0.0, 1.0);

        let mind_all_unity = ego_duality < 0.05 && h3_trits > 0.995;

        let active_pole = if mind_all_unity {
            NeuroCyberneticPole::EgoDeathNonDualZero
        } else if d >= 0.4 && s >= 0.4 {
            NeuroCyberneticPole::DmtHyperAssociativeDrive
        } else {
            NeuroCyberneticPole::AmygdalaSurvivalClamp
        };

        Ok(NeuroOmegaStateSnapshot {
            step_index,
            dmt_5ht2a_occupancy: d,
            amygdala_threat_gain: amygdala_gain,
            dmn_ego_prior_precision: dmn_precision,
            variational_free_energy: variational_fe,
            cortical_entropy_trits: h3_trits,
            ego_boundary_duality: ego_duality,
            active_pole,
            mind_equals_mind_all_achieved: mind_all_unity,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neuro_cybernetic_amygdala_dmt_ego_death_mind_equals_all_transition() {
        assert_eq!(
            BILINGUAL_AXIOM_EN_MIND.to_ascii_lowercase(),
            BILINGUAL_AXIOM_HU_MIND
        );
        assert_eq!(BILINGUAL_AXIOM_MEANING, "ALL");

        let sim = NeuroMindAllSimulator::default();

        // 1. Pole -1: Amygdala Survival Clamp (d = 0.0, s = 0.0)
        let s_amygdala = sim.evaluate_state(0, 0.0, 0.0).unwrap();
        assert_eq!(
            s_amygdala.active_pole,
            NeuroCyberneticPole::AmygdalaSurvivalClamp
        );
        assert!(!s_amygdala.mind_equals_mind_all_achieved);
        assert!(s_amygdala.ego_boundary_duality > 0.95);
        assert!(s_amygdala.cortical_entropy_trits < 0.50);

        // 2. Pole +1: DMT / 5-HT2A Transition (d = 0.65, s = 0.65)
        let s_dmt = sim.evaluate_state(1, 0.65, 0.65).unwrap();
        assert_eq!(
            s_dmt.active_pole,
            NeuroCyberneticPole::DmtHyperAssociativeDrive
        );
        assert!(s_dmt.dmn_ego_prior_precision < s_amygdala.dmn_ego_prior_precision);
        assert!(s_dmt.cortical_entropy_trits > s_amygdala.cortical_entropy_trits);

        // 3. Pole 0: Ego Death / Én-halál Non-Dual Unity (d = 1.0, s = 1.0 -> MIND === mind === ALL)
        let s_ego_death = sim.evaluate_state(2, 1.0, 1.0).unwrap();
        assert_eq!(
            s_ego_death.active_pole,
            NeuroCyberneticPole::EgoDeathNonDualZero
        );
        assert!(s_ego_death.mind_equals_mind_all_achieved);
        assert!(s_ego_death.ego_boundary_duality < 0.01);
        assert!((s_ego_death.cortical_entropy_trits - 1.0).abs() < 1e-9);
        assert_eq!(
            (NeuroCyberneticPole::AmygdalaSurvivalClamp as i8)
                + (NeuroCyberneticPole::DmtHyperAssociativeDrive as i8),
            NeuroCyberneticPole::EgoDeathNonDualZero as i8
        );
    }
}
