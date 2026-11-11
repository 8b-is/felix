#![no_std]

use core::sync::atomic::{AtomicU64, Ordering};

/// Packed `(p, q)` root weight cell (`(p as u64) << 32 | (q as u64)`), initialized to `7/1`.
static CURRENT_ROOT_WEIGHT_PACKED: AtomicU64 = AtomicU64::new((7u64 << 32) | 1u64);

/// Read the current root weight as a reduced tuple `(numerator, denominator)`.
#[inline]
pub fn current_root_weight() -> (u32, u32) {
    let packed = CURRENT_ROOT_WEIGHT_PACKED.load(Ordering::Acquire);
    ((packed >> 32) as u32, (packed & 0xFFFF_FFFF) as u32)
}

/// Write a new reduced root weight `(p, q)`. The fraction is reduced by
/// dividing by the greatest common divisor (`q == 0` is clamped to `1`).
pub fn set_root_weight(p: u32, q: u32) {
    #[inline]
    const fn gcd(mut a: u32, mut b: u32) -> u32 {
        while b != 0 {
            let t = b;
            b = a % b;
            a = t;
        }
        a
    }
    let denom = if q == 0 { 1 } else { q };
    let g = gcd(p, denom);
    let red_p = if g == 0 { 0 } else { p / g };
    let red_q = if g == 0 { 1 } else { denom / g };
    let packed = ((red_p as u64) << 32) | (red_q as u64);
    CURRENT_ROOT_WEIGHT_PACKED.store(packed, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_weight_reduction_and_roundtrip() {
        set_root_weight(14, 2);
        assert_eq!(current_root_weight(), (7, 1));
        set_root_weight(21, 14);
        assert_eq!(current_root_weight(), (3, 2));
        set_root_weight(7, 1);
    }
}
