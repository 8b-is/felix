//! Exact rationals: `p/q`, always reduced, denominator always positive.
//!
//! This is the whole of Felix's number tower above `Int`, and it exists for one
//! reason: a block scale like `1/3` must survive from the source text to the
//! printed result without being rounded on the way. Every operation is done in
//! `i128` and reduced before it is narrowed back to `i64`; anything that does
//! not fit is an error the caller reports, never a wrap.
//!
//! ## `ultraTweak` Zero-Division Hot Paths
//! - [`gcd_binary_u64`] & [`gcd_binary_u128`]: Stein's binary GCD (`trailing_zeros`
//!   hardware bit-scan + shift-subtract) eliminates all integer modulo `%` divisions
//!   from the GCD reduction loop, with a 64-bit fast-path when operands fit in `u64`.
//! - Coprime (`g == 1`) fast-path in [`Ratio::reduce`] bypasses 128-bit software
//!   division (`__divti3`) whenever numerator and denominator are already reduced.
//! - [`div3_u8_fast`] & [`divmod3_u8_fast`]: Granlund-Montgomery reciprocal multiply-shift
//!   `((v as u16 * 171) >> 9)` replaces hardware `/ 3` and `% 3` for `u8` (`0..=255`).

use std::fmt;

/// Stein's binary GCD on `u64` using hardware `trailing_zeros()` (`rbit+clz` / `tzcnt`)
/// and shift-subtract (`0` integer division instructions inside the loop).
#[inline(always)]
pub const fn gcd_binary_u64(mut a: u64, mut b: u64) -> u64 {
    if a == 0 {
        return b;
    }
    if b == 0 {
        return a;
    }
    let shift = (a | b).trailing_zeros();
    a >>= a.trailing_zeros();
    loop {
        b >>= b.trailing_zeros();
        if a > b {
            let tmp = a;
            a = b;
            b = tmp;
        }
        b -= a;
        if b == 0 {
            return a << shift;
        }
    }
}

/// Stein's binary GCD on `u128` with automatic fast-path dispatch to [`gcd_binary_u64`]
/// whenever both operands fit in `u64` (avoiding 128-bit software `__udivmodti4`).
#[inline(always)]
pub const fn gcd_binary_u128(mut a: u128, mut b: u128) -> u128 {
    if a <= (u64::MAX as u128) && b <= (u64::MAX as u128) {
        return gcd_binary_u64(a as u64, b as u64) as u128;
    }
    if a == 0 {
        return b;
    }
    if b == 0 {
        return a;
    }
    let shift = (a | b).trailing_zeros();
    a >>= a.trailing_zeros();
    loop {
        if a <= (u64::MAX as u128) && b <= (u64::MAX as u128) {
            return (gcd_binary_u64(a as u64, b as u64) as u128) << shift;
        }
        b >>= b.trailing_zeros();
        if a > b {
            let tmp = a;
            a = b;
            b = tmp;
        }
        b -= a;
        if b == 0 {
            return a << shift;
        }
    }
}

/// Exact division by `3` for `u8` (`0..=255`) using the Granlund-Montgomery Q9 reciprocal
/// `((v as u16 * 171) >> 9) as u8` (`0` division instructions; exact for all `v <= 509`).
#[inline(always)]
pub const fn div3_u8_fast(v: u8) -> u8 {
    (((v as u16) * 171) >> 9) as u8
}

/// Simultaneous quotient and remainder `(v / 3, v % 3)` for `u8` (`0..=255`) using
/// [`div3_u8_fast`] and `rem = v - q * 3` (`0` division instructions).
#[inline(always)]
pub const fn divmod3_u8_fast(v: u8) -> (u8, u8) {
    let q = div3_u8_fast(v);
    let r = v - q * 3;
    (q, r)
}

#[inline(always)]
const fn i128_to_i64_const(x: i128) -> Option<i64> {
    if x >= (i64::MIN as i128) && x <= (i64::MAX as i128) {
        Some(x as i64)
    } else {
        None
    }
}

/// `p/q` with `gcd(|p|, q) == 1` and `q > 0`. Zero is `0/1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ratio {
    p: i64,
    q: i64,
}

impl Ratio {
    pub const ZERO: Ratio = Ratio { p: 0, q: 1 };
    pub const ONE: Ratio = Ratio { p: 1, q: 1 };

    /// Reduce `p/q`. `None` when `q == 0` or the reduced form does not fit `i64`.
    #[inline(always)]
    pub const fn new(p: i64, q: i64) -> Option<Ratio> {
        if q == 0 {
            return None;
        }
        if p == 0 {
            return Some(Self::ZERO);
        }
        // Fast path when neither p nor q is i64::MIN: pure u64 binary GCD!
        if p != i64::MIN && q != i64::MIN {
            let (p_norm, q_norm) = if q < 0 { (-p, -q) } else { (p, q) };
            let g = gcd_binary_u64(p_norm.unsigned_abs(), q_norm as u64);
            if g <= 1 {
                return Some(Ratio {
                    p: p_norm,
                    q: q_norm,
                });
            }
            let g_i64 = g as i64;
            return Some(Ratio {
                p: p_norm / g_i64,
                q: q_norm / g_i64,
            });
        }
        Self::reduce(p as i128, q as i128)
    }

    /// An integer, exactly.
    #[inline(always)]
    pub const fn int(value: i64) -> Ratio {
        Ratio { p: value, q: 1 }
    }

    #[inline(always)]
    const fn reduce(p: i128, q: i128) -> Option<Ratio> {
        if q == 0 {
            return None;
        }
        if p == 0 {
            return Some(Self::ZERO);
        }
        let (p, q) = if q < 0 { (-p, -q) } else { (p, q) };
        let g = gcd_binary_u128(p.unsigned_abs(), q.unsigned_abs());
        let (p_red, q_red) = if g <= 1 {
            (p, q)
        } else {
            let g_i128 = g as i128;
            (p / g_i128, q / g_i128)
        };
        let Some(p_i64) = i128_to_i64_const(p_red) else {
            return None;
        };
        let Some(q_i64) = i128_to_i64_const(q_red) else {
            return None;
        };
        Some(Ratio { p: p_i64, q: q_i64 })
    }

    #[inline(always)]
    pub const fn numer(&self) -> i64 {
        self.p
    }

    #[inline(always)]
    pub const fn denom(&self) -> i64 {
        self.q
    }

    #[inline(always)]
    pub const fn is_positive(&self) -> bool {
        self.p > 0
    }

    /// Exact product. `None` on overflow after reduction.
    #[inline(always)]
    pub const fn checked_mul(self, other: Ratio) -> Option<Ratio> {
        if self.p == 0 || other.p == 0 {
            return Some(Self::ZERO);
        }
        // Cross-reduce first using 64-bit Stein's binary GCD so that products which fit are never refused.
        let g1 = gcd_binary_u64(self.p.unsigned_abs(), other.q as u64);
        let g2 = gcd_binary_u64(other.p.unsigned_abs(), self.q as u64);
        let p_lhs = if g1 <= 1 {
            self.p as i128
        } else {
            (self.p as i128) / (g1 as i128)
        };
        let p_rhs = if g2 <= 1 {
            other.p as i128
        } else {
            (other.p as i128) / (g2 as i128)
        };
        let q_lhs = if g2 <= 1 {
            self.q as i128
        } else {
            (self.q as i128) / (g2 as i128)
        };
        let q_rhs = if g1 <= 1 {
            other.q as i128
        } else {
            (other.q as i128) / (g1 as i128)
        };
        // Because self and other are already reduced and cross-reduced, gcd(|p|, q) == 1!
        let p = p_lhs * p_rhs;
        let q = q_lhs * q_rhs;
        let Some(p_i64) = i128_to_i64_const(p) else {
            return None;
        };
        let Some(q_i64) = i128_to_i64_const(q) else {
            return None;
        };
        Some(Ratio { p: p_i64, q: q_i64 })
    }

    /// Exact `self * n`.
    #[inline(always)]
    pub const fn checked_mul_int(self, n: i64) -> Option<Ratio> {
        self.checked_mul(Ratio::int(n))
    }

    /// Exact sum `self + other`. `None` on overflow after reduction.
    #[inline(always)]
    pub const fn checked_add(self, other: Ratio) -> Option<Ratio> {
        if self.q == other.q {
            return Self::reduce((self.p as i128) + (other.p as i128), self.q as i128);
        }
        let g = gcd_binary_u64(self.q as u64, other.q as u64) as i128;
        let q1_div_g = (self.q as i128) / g;
        let q2_div_g = (other.q as i128) / g;
        let num = (self.p as i128) * q2_div_g + (other.p as i128) * q1_div_g;
        let den = q1_div_g * (other.q as i128);
        Self::reduce(num, den)
    }

    /// Precomputes the Q16 Granlund-Montgomery reciprocal multiplier `ceil((p << 16) / q)`
    /// for positive `p/q`, enabling division-free `((x as u64 * mult_q16) >> 16)` scaling.
    #[inline(always)]
    pub const fn q16_reciprocal_multiplier(self) -> u64 {
        if self.p <= 0 || self.q <= 0 {
            return 0;
        }
        let num = (self.p as u64) << 16;
        let den = self.q as u64;
        num.div_ceil(den)
    }
}

impl fmt::Display for Ratio {
    /// `3/2`, or just `2` when the denominator is one.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.q == 1 {
            write!(f, "{}", self.p)
        } else {
            write!(f, "{}/{}", self.p, self.q)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduces_and_normalises_the_sign() {
        assert_eq!(Ratio::new(6, 4).unwrap(), Ratio::new(3, 2).unwrap());
        assert_eq!(Ratio::new(3, -6).unwrap().to_string(), "-1/2");
        assert_eq!(Ratio::new(0, -7).unwrap(), Ratio::ZERO);
        assert_eq!(Ratio::new(8, 4).unwrap().to_string(), "2");
    }

    #[test]
    fn a_zero_denominator_is_refused() {
        assert!(Ratio::new(1, 0).is_none());
    }

    #[test]
    fn products_are_exact_and_cross_reduced() {
        let third = Ratio::new(1, 3).unwrap();
        assert_eq!(third.checked_mul(Ratio::int(3)).unwrap(), Ratio::ONE);
        // (big/3) * (3/big) fits only because it is cross-reduced first.
        let big = i64::MAX;
        let a = Ratio::new(big, 3).unwrap();
        let b = Ratio::new(3, big).unwrap();
        assert_eq!(a.checked_mul(b).unwrap(), Ratio::ONE);
    }

    #[test]
    fn overflow_is_reported_not_wrapped() {
        let big = Ratio::int(i64::MAX);
        assert!(big.checked_mul_int(2).is_none());
        assert!(Ratio::new(i64::MIN, -1).is_none());
    }

    #[test]
    fn binary_gcd_and_div3_fast_reciprocal_are_bit_exact() {
        for v in 0u8..=255u8 {
            assert_eq!(div3_u8_fast(v), v / 3);
            assert_eq!(divmod3_u8_fast(v), (v / 3, v % 3));
        }
        assert_eq!(gcd_binary_u64(0, 42), 42);
        assert_eq!(gcd_binary_u64(42, 0), 42);
        assert_eq!(gcd_binary_u64(243, 81), 81);
        assert_eq!(gcd_binary_u64(1_000_000_007, 1_000_000_009), 1);
        assert_eq!(
            gcd_binary_u128((u64::MAX as u128) * 6, (u64::MAX as u128) * 9),
            (u64::MAX as u128) * 3
        );
        let solar = Ratio::new(31, 32).unwrap();
        let lunar = Ratio::new(1, 32).unwrap();
        assert_eq!(solar.checked_add(lunar), Some(Ratio::ONE));
        assert_eq!(Ratio::new(1, 3).unwrap().q16_reciprocal_multiplier(), 21846);
    }
}
