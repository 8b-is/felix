//! Content‑addressed hashing for Felix modules.
//! Provides a deterministic hash of a module's IR + compiler version.
//! Currently uses std::hash for speed; a SHA‑256 implementation will
//! replace this in a later pass (M4/M5) for true content‑addressability.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Hash a module's IR together with the compiler version string.
/// The result is a `u64` that can be used as a cache key.
#[inline]
pub fn compute_ir_hash(ir: &str, compiler_version: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    // hash the IR bytes
    ir.hash(&mut hasher);
    // hash the compiler version
    compiler_version.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_determinism() {
        let ir = "nand (R[0], R[1])";
        let cv = "felix-lang 1.5.0";
        let h1 = compute_ir_hash(ir, cv);
        let h2 = compute_ir_hash(ir, cv);
        assert_eq!(h1, h2);

        // different IR should give different hash (very likely)
        let ir2 = "nand (R[0], R[2])";
        let h3 = compute_ir_hash(ir2, cv);
        // we don't assert inequality here – collisions are possible but rare
        println!("hashes: {} vs {} vs {}", h1, h2, h3);
    }
}