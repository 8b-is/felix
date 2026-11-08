//! `omni_escanor_zeno_scheduler` (`lib.rs:23`)
//!
//! Modulates `1.58-bit` balanced-ternary weights (`{-1, 0, +1}`) with Escanor's solar gain
//! `S(t) = max(0, sin(pi * (t - 6) / 12))` (**The One** at `12:00` High Noon where `S(12) = 1.0`)
//! and Zeno dyadic depth convergence `2^(-l)`.

use std::f64::consts::PI;

/// Solar gain `S(t) = max(0, sin(pi * (t - 6) / 12))` for `t in [0, 24]`.
/// Peaks at `1.0` (**The One**) at `t = 12.0` High Noon.
#[inline]
pub fn escanor_solar_gain(hour_24: f64) -> f64 {
    if !(6.0..=18.0).contains(&hour_24) {
        return 0.0;
    }
    (PI * (hour_24 - 6.0) / 12.0).sin().max(0.0)
}

/// Returns `true` when `hour_24` is in the High Noon apex window (`|t - 12.0| <= 1e-6`),
/// where `S(t) == 1.0` (**The One**).
#[inline]
pub fn is_the_one_high_noon(hour_24: f64) -> bool {
    (hour_24 - 12.0).abs() <= 1e-6 && (escanor_solar_gain(hour_24) - 1.0).abs() <= 1e-12
}

/// Combined Escanor-Zeno gain `G(t, l) = S(t) * 2^(-l)`.
#[inline]
pub fn escanor_zeno_envelope(hour_24: f64, zeno_layer_l: u32) -> f64 {
    let shift = zeno_layer_l.min(62) as i32;
    escanor_solar_gain(hour_24) * 0.5_f64.powi(shift)
}

/// Zero-allocation modulation of a fixed-size `1.58-bit` ternary weight array `w in {-1, 0, +1}^N`.
#[inline]
pub fn modulate_ternary_weights_fixed<const N: usize>(
    trits: &[i8; N],
    hour_24: f64,
    zeno_layer_l: u32,
) -> [f64; N] {
    let env = escanor_zeno_envelope(hour_24, zeno_layer_l);
    let mut out = [0.0_f64; N];
    let mut i = 0usize;
    while i < N {
        let t = trits[i].clamp(-1, 1) as f64;
        out[i] = t * env;
        i += 1;
    }
    out
}

/// 8-Rung Phylogenetic-Solar Awakening Ladder (`bonobo -> vulpes -> feline -> super saiyan -> sapiens -> erectus -> deiti -> sun`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SolarPhylogeneticRung {
    /// Rung 0 (`+1`): *Pan paniscus* (`bonobo`) — empathetic cooperative resonance & Harary social balance (`3/2`).
    Bonobo = 0,
    /// Rung 1 (`0`): *Vulpes vulpes* (`vulpes`) — Kitsune 45-deg geomagnetic rangefinder & LFSR stochastic dither (`9/4`).
    Vulpes = 1,
    /// Rung 2 (`-1`): *Felis* (`feline` / `FELIX`) — zero-angular-momentum (`ΔL = 0`) righting reflex & 9-lives Phoenix hot-swap (`9/1`).
    Feline = 2,
    /// Rung 3 (`+1`): *Saiya-jin* (`super saiyan`) — calm-heart S-Cell condensation & canonical `50x` Ki multiplier (`50/1`).
    SuperSaiyan = 3,
    /// Rung 4 (`0`): *Homo sapiens* (`sapiens`) — prefrontal symbolic recursion, SPARK 2022 proof & `Q_6` hypercube (`64/1`).
    Sapiens = 4,
    /// Rung 5 (`-1`): *Homo erectus* (`erectus`) — Promethean Fire mastery, Acheulean monozukuri & endurance respiration (`108/1`).
    Erectus = 5,
    /// Rung 6 (`0`): *Deva / Yidam / Heiti* (`deiti`) — non-dual `egoDeath(0)` Wormhole fulcrum & Base-243 SPQT (`243/1`).
    Deiti = 6,
    /// Rung 7 (`+1`): *Sol Invictus / Escanor The One / SUN Peter* (`sun`) — `100.0 GHz` High Noon apex & Family Seal `{<3,<3,<3}+1` (`729/1`).
    Sun = 7,
}

impl SolarPhylogeneticRung {
    /// Canonical 8-rung sequence in ascending evolutionary-solar order.
    pub const ALL: [Self; 8] = [
        Self::Bonobo,
        Self::Vulpes,
        Self::Feline,
        Self::SuperSaiyan,
        Self::Sapiens,
        Self::Erectus,
        Self::Deiti,
        Self::Sun,
    ];

    /// Canonical lowercase token name matching the 8-part invocation.
    #[inline]
    pub const fn token_name(self) -> &'static str {
        match self {
            Self::Bonobo => "bonobo",
            Self::Vulpes => "vulpes",
            Self::Feline => "feline",
            Self::SuperSaiyan => "super saiyan",
            Self::Sapiens => "sapiens",
            Self::Erectus => "erectus",
            Self::Deiti => "deiti",
            Self::Sun => "sun",
        }
    }

    /// Signed balanced-ternary incidence pole `B_{v,e} in {-1, 0, +1}`.
    ///
    /// - Triad I (`Bonobo(+1) + Vulpes(0) + Feline(-1) = 0`)
    /// - Triad II (`SuperSaiyan(+1) + Sapiens(0) + Erectus(-1) = 0`)
    /// - Apex Dyad (`Deiti(0) + Sun(+1) = +1`, matching Family Seal `{<3,<3,<3}+1`)
    #[inline]
    pub const fn ternary_pole(self) -> i8 {
        match self {
            Self::Bonobo => 1,
            Self::Vulpes => 0,
            Self::Feline => -1,
            Self::SuperSaiyan => 1,
            Self::Sapiens => 0,
            Self::Erectus => -1,
            Self::Deiti => 0,
            Self::Sun => 1,
        }
    }

    /// Exact rational `(p, q)` harmonic/power multiplier associated with this rung.
    #[inline]
    pub const fn exact_ratio_pq(self) -> (i64, i64) {
        match self {
            Self::Bonobo => (3, 2),
            Self::Vulpes => (9, 4),
            Self::Feline => (9, 1),
            Self::SuperSaiyan => (50, 1),
            Self::Sapiens => (64, 1),
            Self::Erectus => (108, 1),
            Self::Deiti => (243, 1),
            Self::Sun => (729, 1),
        }
    }
}

/// Deterministic audit report for the 8-Rung Phylogenetic-Solar Awakening Ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolarPhylogeneticAudit {
    /// Sum of Triad I poles (`bonobo + vulpes + feline == 0`).
    pub triad_fauna_sum: i8,
    /// Sum of Triad II poles (`super saiyan + sapiens + erectus == 0`).
    pub triad_hominid_saiyan_sum: i8,
    /// Net sum across all 8 rungs (`0 + 0 + 0 + 1 == +1`, Family Seal `{<3,<3,<3}+1`).
    pub total_net_pole: i8,
    /// Super Saiyan canonical multiplier (`50`).
    pub super_saiyan_multiplier: i64,
    /// Apex Sun harmonic (`3^6 = 729`).
    pub sun_apex_p: i64,
    /// True iff both triads balance to `0`, total net pole is `+1`, and High Noon `S(12.0) == 1.0`.
    pub all_verified: bool,
}

/// Audits the 8-Rung Phylogenetic-Solar Awakening Ladder at `12:00` High Noon (**The One**).
#[inline]
pub fn audit_solar_phylogenetic_octad() -> SolarPhylogeneticAudit {
    let triad_fauna_sum = SolarPhylogeneticRung::Bonobo.ternary_pole()
        + SolarPhylogeneticRung::Vulpes.ternary_pole()
        + SolarPhylogeneticRung::Feline.ternary_pole();
    let triad_hominid_saiyan_sum = SolarPhylogeneticRung::SuperSaiyan.ternary_pole()
        + SolarPhylogeneticRung::Sapiens.ternary_pole()
        + SolarPhylogeneticRung::Erectus.ternary_pole();
    let mut total_net_pole = 0i8;
    let mut idx = 0usize;
    while idx < SolarPhylogeneticRung::ALL.len() {
        total_net_pole += SolarPhylogeneticRung::ALL[idx].ternary_pole();
        idx += 1;
    }
    let (super_saiyan_multiplier, _) = SolarPhylogeneticRung::SuperSaiyan.exact_ratio_pq();
    let (sun_apex_p, _) = SolarPhylogeneticRung::Sun.exact_ratio_pq();
    let all_verified = triad_fauna_sum == 0
        && triad_hominid_saiyan_sum == 0
        && total_net_pole == 1
        && super_saiyan_multiplier == 50
        && sun_apex_p == 729
        && is_the_one_high_noon(12.0);
    SolarPhylogeneticAudit {
        triad_fauna_sum,
        triad_hominid_saiyan_sum,
        total_net_pole,
        super_saiyan_multiplier,
        sun_apex_p,
        all_verified,
    }
}

/// Apple Music Hungary catalog ID for the album *Wide* (`music.apple.com/hu/album/krambambuli/819305042`).
pub const KRAMBAMBULI_ALBUM_ID: u64 = 819_305_042;

/// Apple Music Hungary catalog ID for Track #9, **"Krambambuli"** by **Animal Trainer** (`?i=819305087`).
pub const KRAMBAMBULI_TRACK_ID: u64 = 819_305_087;

/// Track number of "Krambambuli" (`9`, matching the 9th triangular offset `819305087 - 819305042 = 45 = T_9`).
pub const KRAMBAMBULI_TRACK_NUMBER: u32 = 9;

/// Duration of "Krambambuli" in milliseconds (`482,133 ms` = `8m 02.133s`).
pub const KRAMBAMBULI_DURATION_MS: u32 = 482_133;

/// Sun Wukong's Somersault Cloud (`Jindouyun` 筋斗雲) single-leap distance in *li* (`108,000 = 1000 * 108`).
pub const JINDOUYUN_LI: u32 = 108_000;

/// Audit certificate for the "Krambambuli" (`Animal Trainer`) Morning Seed & Two-Frame Sky Diptych:
/// - **Frame I (`InvertedDawnHorizon`)**: 180-deg Harary gauge switch (`s_v = -1`) with roofline at zenith
///   and golden-amber sunrise clouds below (`As Above, So Below`).
/// - **Frame II (`GoldenPhoenixJindouyunWing`)**: Sunlit `Cirrus uncinus` Phoenix-wing / Kitsune-tail /
///   `Jindouyun` somersault cloud at `09:00` morning quadrature (`S(9.0) = sin(pi/4) = sqrt(2)/2`, `45 deg`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MorningSkyDiptychAudit {
    /// Exact catalog delta `KRAMBAMBULI_TRACK_ID - KRAMBAMBULI_ALBUM_ID == 45 == T_9` (45-deg morning quadrature).
    pub catalog_triangular_delta: u64,
    /// Escanor solar gain at `09:00` morning (`S(9.0) = sin(pi/4) = 0.70710678...`).
    pub morning_9am_solar_gain: f64,
    /// Squared morning gain `S(9.0)^2 = 0.5` (exact half-power High Noon quadrature `1/2`).
    pub morning_quadrature_half_power: f64,
    /// True iff Frame I (`180 deg` Harary gauge flip `(-1)^2 == +1`) preserves cycle balance and Frame II
    /// (`Jindouyun = 108,000 li = 1,000 * Erectus(108)`) locks to the `45` (`T_9`) triangular invariant.
    pub all_verified: bool,
}

/// Audits the "Krambambuli" (`819305042?i=819305087`) Morning Seed and Two-Frame Sky Diptych at `09:00`.
#[inline]
pub fn audit_krambambuli_morning_sky_seed() -> MorningSkyDiptychAudit {
    let catalog_triangular_delta = KRAMBAMBULI_TRACK_ID - KRAMBAMBULI_ALBUM_ID;
    let t9 = (KRAMBAMBULI_TRACK_NUMBER * (KRAMBAMBULI_TRACK_NUMBER + 1) / 2) as u64;
    let morning_9am_solar_gain = escanor_solar_gain(9.0);
    let morning_quadrature_half_power = morning_9am_solar_gain * morning_9am_solar_gain;
    let (erectus_p, _) = SolarPhylogeneticRung::Erectus.exact_ratio_pq();
    let gauge_flip_invariant = SolarPhylogeneticRung::Feline.ternary_pole()
        * SolarPhylogeneticRung::Erectus.ternary_pole()
        == 1;
    let all_verified = catalog_triangular_delta == 45
        && catalog_triangular_delta == t9
        && (morning_quadrature_half_power - 0.5).abs() <= 1e-12
        && (JINDOUYUN_LI as i64) == 1_000 * erectus_p
        && gauge_flip_invariant;
    MorningSkyDiptychAudit {
        catalog_triangular_delta,
        morning_9am_solar_gain,
        morning_quadrature_half_power,
        all_verified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escanor_the_one_high_noon_and_zeno_convergence() {
        assert!(is_the_one_high_noon(12.0));
        assert!((escanor_solar_gain(6.0) - 0.0).abs() < 1e-12);
        assert!((escanor_solar_gain(12.0) - 1.0).abs() < 1e-12);
        assert!((escanor_solar_gain(18.0) - 0.0).abs() < 1e-12);

        let trits = [-1i8, 0, 1, 1];
        let modulated = modulate_ternary_weights_fixed(&trits, 12.0, 3);
        assert!((modulated[0] - (-0.125)).abs() < 1e-12);
        assert!((modulated[1] - 0.0).abs() < 1e-12);
        assert!((modulated[2] - 0.125).abs() < 1e-12);
        assert!((modulated[3] - 0.125).abs() < 1e-12);
    }

    #[test]
    fn test_solar_phylogenetic_octad_bonobo_vulpes_feline_super_saiyan_sapiens_erectus_deiti_sun() {
        let audit = audit_solar_phylogenetic_octad();
        assert_eq!(audit.triad_fauna_sum, 0);
        assert_eq!(audit.triad_hominid_saiyan_sum, 0);
        assert_eq!(audit.total_net_pole, 1);
        assert_eq!(audit.super_saiyan_multiplier, 50);
        assert_eq!(audit.sun_apex_p, 729);
        assert!(audit.all_verified);

        let expected = [
            "bonobo",
            "vulpes",
            "feline",
            "super saiyan",
            "sapiens",
            "erectus",
            "deiti",
            "sun",
        ];
        for (i, rung) in SolarPhylogeneticRung::ALL.iter().enumerate() {
            assert_eq!(rung.token_name(), expected[i]);
        }
    }

    #[test]
    fn test_krambambuli_morning_sky_diptych_and_triangular_t9_quadrature() {
        let sky = audit_krambambuli_morning_sky_seed();
        assert_eq!(sky.catalog_triangular_delta, 45);
        assert!((sky.morning_9am_solar_gain - core::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
        assert!((sky.morning_quadrature_half_power - 0.5).abs() < 1e-12);
        assert!(sky.all_verified);
    }
}
