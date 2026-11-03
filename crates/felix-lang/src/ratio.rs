//! Exact rationals: `p/q`, always reduced, denominator always positive.
//!
//! This is the whole of Felix's number tower above `Int`, and it exists for one
//! reason: a block scale like `1/3` must survive from the source text to the
//! printed result without being rounded on the way. Every operation is done in
//! `i128` and reduced before it is narrowed back to `i64`; anything that does
//! not fit is an error the caller reports, never a wrap.

use std::fmt;

/// `p/q` with `gcd(|p|, q) == 1` and `q > 0`. Zero is `0/1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ratio {
    p: i64,
    q: i64,
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

impl Ratio {
    pub const ZERO: Ratio = Ratio { p: 0, q: 1 };
    pub const ONE: Ratio = Ratio { p: 1, q: 1 };

    /// Reduce `p/q`. `None` when `q == 0` or the reduced form does not fit `i64`.
    pub fn new(p: i64, q: i64) -> Option<Ratio> {
        Self::reduce(p as i128, q as i128)
    }

    /// An integer, exactly.
    pub fn int(value: i64) -> Ratio {
        Ratio { p: value, q: 1 }
    }

    fn reduce(p: i128, q: i128) -> Option<Ratio> {
        if q == 0 {
            return None;
        }
        let (p, q) = if q < 0 { (-p, -q) } else { (p, q) };
        let g = gcd(p.unsigned_abs(), q.unsigned_abs()).max(1) as i128;
        let (p, q) = (p / g, q / g);
        Some(Ratio {
            p: i64::try_from(p).ok()?,
            q: i64::try_from(q).ok()?,
        })
    }

    pub fn numer(&self) -> i64 {
        self.p
    }

    pub fn denom(&self) -> i64 {
        self.q
    }

    pub fn is_positive(&self) -> bool {
        self.p > 0
    }

    /// Exact product. `None` on overflow after reduction.
    pub fn checked_mul(self, other: Ratio) -> Option<Ratio> {
        // Cross-reduce first so that products which fit are never refused.
        let g1 = gcd(self.p.unsigned_abs() as u128, other.q as u128).max(1) as i128;
        let g2 = gcd(other.p.unsigned_abs() as u128, self.q as u128).max(1) as i128;
        let p = (self.p as i128 / g1) * (other.p as i128 / g2);
        let q = (self.q as i128 / g2) * (other.q as i128 / g1);
        Self::reduce(p, q)
    }

    /// Exact `self * n`.
    pub fn checked_mul_int(self, n: i64) -> Option<Ratio> {
        self.checked_mul(Ratio::int(n))
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
}
