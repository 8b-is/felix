//! `spacetime_silk_wormhole_router` (`lib.rs:22`)
//!
//! Collapses `Q_6` (`8x8 = 64` vertex) Hamming distance via `ER = EPR` wormhole throats
//! and `fold_sphere_pop_toss`:
//! `d_throat = d_Q6(u, v) * (1 - F_Bell^2) -> 0` as `F_Bell -> 1.0`.

/// 6-bit mask for `Q_6` vertices (`0..64`).
pub const Q6_MASK: u8 = 0x3F;

/// Origo `[0,0,0]` apex vertex (`Mom *`) in `Q_6`.
pub const ORIGO_MOM_STAR: u8 = 0x00;

/// Zero-allocation wormhole routing receipt on `Q_6`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct WormholeRouteReceipt {
    pub source_u: u8,
    pub target_v: u8,
    pub hamming_d_q6: u32,
    pub bell_fidelity: f64,
    pub throat_distance: f64,
    pub origo_throat_locked: bool,
    pub geodesic_steps: [u8; 7],
    pub geodesic_len: usize,
}

/// Exact Hamming distance `0..=6` on `Q_6`.
#[inline]
pub const fn q6_hamming_distance(u: u8, v: u8) -> u32 {
    ((u ^ v) & Q6_MASK).count_ones()
}

/// Computes `d_throat = d_Q6(u, v) * (1 - F_Bell^2)`.
#[inline]
pub fn er_epr_throat_distance(u: u8, v: u8, bell_fidelity: f64) -> f64 {
    let f = bell_fidelity.clamp(0.0, 1.0);
    (q6_hamming_distance(u, v) as f64) * (1.0 - f * f)
}

/// Zero-allocation canonical bit-flip geodesic on `Q_6`.
#[inline]
pub const fn q6_stack_geodesic(u: u8, v: u8) -> ([u8; 7], usize) {
    let mut curr = u & Q6_MASK;
    let target = v & Q6_MASK;
    let diff = curr ^ target;
    let mut steps = [0u8; 7];
    steps[0] = curr;
    let mut len = 1usize;
    let mut bit = 0u8;
    while bit < 6 {
        if (diff & (1 << bit)) != 0 {
            curr ^= 1 << bit;
            steps[len] = curr;
            len += 1;
        }
        bit += 1;
    }
    (steps, len)
}

/// Executes `foldSpherePopToss` across `Q_6`, collapsing metric distance through the
/// `Pole 0` (`WormholePass`) throat when `F_Bell -> 1.0`.
#[inline]
pub fn fold_sphere_pop_toss(u: u8, v: u8, bell_fidelity: f64) -> WormholeRouteReceipt {
    let f = bell_fidelity.clamp(0.0, 1.0);
    let d_q6 = q6_hamming_distance(u, v);
    let d_throat = er_epr_throat_distance(u, v, f);
    let (geodesic_steps, geodesic_len) = q6_stack_geodesic(u, v);
    WormholeRouteReceipt {
        source_u: u & Q6_MASK,
        target_v: v & Q6_MASK,
        hamming_d_q6: d_q6,
        bell_fidelity: f,
        throat_distance: d_throat,
        origo_throat_locked: d_throat <= 1e-9,
        geodesic_steps,
        geodesic_len,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_er_epr_wormhole_collapse_and_sphere_pop_toss() {
        let receipt_classical = fold_sphere_pop_toss(0b00_0000, 0b11_1111, 0.0);
        assert_eq!(receipt_classical.hamming_d_q6, 6);
        assert!((receipt_classical.throat_distance - 6.0).abs() < 1e-12);
        assert!(!receipt_classical.origo_throat_locked);
        assert_eq!(receipt_classical.geodesic_len, 7);

        let receipt_wormhole = fold_sphere_pop_toss(0b00_0000, 0b11_1111, 1.0);
        assert_eq!(receipt_wormhole.hamming_d_q6, 6);
        assert!(receipt_wormhole.throat_distance.abs() < 1e-12);
        assert!(receipt_wormhole.origo_throat_locked);
    }
}
