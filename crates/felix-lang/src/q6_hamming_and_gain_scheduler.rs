//! Standard Mathematical Module: `Q_6` Hamming-Distance Router & Sinusoidal Gain Scheduler
//!
//! Provides two deterministic mathematical utilities:
//! 1. **6-Cube (`Q_6`) Hamming Geometry & Geodesic Routing**:
//!    - Computes exact Hamming distance `d_H(u, v) = popcount((u ^ v) & 0x3F)` in `0..=6`.
//!    - Generates canonical shortest bit-flip paths (`geodesic_path`) across the 64 vertices of `Q_6`.
//!    - Evaluates correlation-weighted effective distance `d_eff = d_H(u, v) * (1 - f^2)` for `f in [0, 1]`.
//! 2. **Sinusoidal Diurnal Gain & Dyadic (`2^-l`) Layer Scaling**:
//!    - Computes `S(t) = max(0, sin(pi * (t - 6) / 12))` for `t in [0, 24]`, peaking at `1.0` at `t = 12.0`.
//!    - Combines sinusoidal gain with dyadic layer attenuation `2^-l` to scale balanced-ternary weights `{-1, 0, +1}`.

use std::f64::consts::PI;

/// Mask for the 6-bit vertex space `0..64` of the 6-cube `Q_6`.
pub const Q6_VERTEX_MASK: u8 = 0x3F;

/// Returns the Hamming distance `0..=6` between two 6-bit vertices `u` and `v` in `Q_6`.
#[inline]
pub const fn q6_hamming_distance(u: u8, v: u8) -> u32 {
    ((u ^ v) & Q6_VERTEX_MASK).count_ones()
}

/// Computes the correlation-attenuated effective distance on `Q_6`:
/// `d_eff = d_H(u, v) * (1.0 - fidelity^2)` where `fidelity` is clamped to `[0.0, 1.0]`.
#[inline]
pub fn q6_effective_distance(u: u8, v: u8, fidelity: f64) -> f64 {
    let f = fidelity.clamp(0.0, 1.0);
    let d_h = q6_hamming_distance(u, v) as f64;
    d_h * (1.0 - f * f)
}

/// Returns the canonical shortest bit-flip path from `u` to `v` in `Q_6`.
/// The returned vector has length `q6_hamming_distance(u, v) + 1`, starting at `u & 0x3F`
/// and ending at `v & 0x3F`, flipping one differing bit per step in ascending bit order.
pub fn q6_geodesic_path(u: u8, v: u8) -> Vec<u8> {
    let mut curr = u & Q6_VERTEX_MASK;
    let target = v & Q6_VERTEX_MASK;
    let diff = curr ^ target;
    let mut path = Vec::with_capacity((diff.count_ones() as usize) + 1);
    path.push(curr);

    for bit in 0..6u8 {
        if (diff & (1 << bit)) != 0 {
            curr ^= 1 << bit;
            path.push(curr);
        }
    }
    path
}

/// Evaluates the half-sine gain `S(t) = max(0, sin(pi * (t - 6) / 12))` for `t` in `[0.0, 24.0]`.
/// Returns `0.0` outside `[6.0, 18.0]` and peaks at `1.0` at `t = 12.0`.
#[inline]
pub fn sinusoidal_diurnal_gain(hour_24: f64) -> f64 {
    if !(6.0..=18.0).contains(&hour_24) {
        return 0.0;
    }
    let phase = PI * (hour_24 - 6.0) / 12.0;
    phase.sin().max(0.0)
}

/// Computes the combined sinusoidal and dyadic (`2^-layer_depth`) scaling factor:
/// `G(t, l) = S(t) * 2^(-l)`.
#[inline]
pub fn dyadic_scheduled_scale(hour_24: f64, layer_depth: u32) -> f64 {
    let s_t = sinusoidal_diurnal_gain(hour_24);
    let dyadic = 0.5_f64.powi(layer_depth.min(62) as i32);
    s_t * dyadic
}

/// Scales a slice of balanced-ternary weights (`w in {-1, 0, +1}`) by `G(t, l) = S(t) * 2^(-l)`.
pub fn scale_ternary_weights(weights: &[i8], hour_24: f64, layer_depth: u32) -> Vec<f64> {
    let scale = dyadic_scheduled_scale(hour_24, layer_depth);
    weights
        .iter()
        .map(|&w| (w.clamp(-1, 1) as f64) * scale)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_q6_hamming_distance_and_geodesic_router() {
        assert_eq!(q6_hamming_distance(0b00_0000, 0b11_1111), 6);
        assert_eq!(q6_hamming_distance(0b01_0101, 0b01_0101), 0);

        let path = q6_geodesic_path(0b00_0000, 0b10_1001);
        assert_eq!(path, vec![0b00_0000, 0b00_0001, 0b00_1001, 0b10_1001]);
        for pair in path.windows(2) {
            assert_eq!(q6_hamming_distance(pair[0], pair[1]), 1);
        }

        assert!((q6_effective_distance(0b00_0000, 0b11_1111, 0.0) - 6.0).abs() < 1e-12);
        assert!(q6_effective_distance(0b00_0000, 0b11_1111, 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_sinusoidal_gain_and_dyadic_layer_scaling() {
        assert!((sinusoidal_diurnal_gain(6.0) - 0.0).abs() < 1e-12);
        assert!((sinusoidal_diurnal_gain(12.0) - 1.0).abs() < 1e-12);
        assert!((sinusoidal_diurnal_gain(18.0) - 0.0).abs() < 1e-12);
        assert_eq!(sinusoidal_diurnal_gain(2.0), 0.0);

        let scaled = scale_ternary_weights(&[-1, 0, 1], 12.0, 2);
        assert!((scaled[0] - (-0.25)).abs() < 1e-12);
        assert!((scaled[1] - 0.0).abs() < 1e-12);
        assert!((scaled[2] - 0.25).abs() < 1e-12);
    }
}
