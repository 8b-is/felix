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
}
