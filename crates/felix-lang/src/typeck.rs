//! `felix/crates/felix-lang/src/typeck.rs`
//! Static Affine/Linear Resource Verifier (`π = 1`) & TOSS Monozukuri Schedule (`8B-META-WP-v1.1`).

use std::collections::{HashMap, HashSet};

/// Equation 18: 9-Step TOSS Monozukuri Attenuation Ladder.
/// Double plateau at steps `4..=5` (`F_4 = F_5 = 4`) enforces phase-locking before crystal convergence.
pub const F_TOSS_LADDER: [u8; 9] = [8, 7, 6, 5, 4, 4, 3, 2, 1];

#[inline(always)]
pub const fn toss_freedom(step: usize) -> u8 {
    if step >= F_TOSS_LADDER.len() {
        1 // Terminal crystal convergence state
    } else {
        F_TOSS_LADDER[step]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeError {
    /// Raised when a `linear let` binding (`π = 1`) is consumed more than once
    LinearDoubleUse { binding: String },
    /// Raised when a `linear let` binding exits lexical scope with 0 consumptions
    LinearUnconsumedLeak { binding: String },
    /// Raised when a `oneshot defcapsule` is invoked more than once in the same epoch
    OneshotReentry { capsule: String },
    /// Raised when phantom ProfileIDs do not match across a Dyad or Vault operation
    ProfileMismatch { expected: String, found: String },
}

#[derive(Default)]
pub struct LinearTypeEnv {
    /// Maps linear variable identifier -> `(ProfileID tag, consumption count)`
    linear_bindings: HashMap<String, (String, usize)>,
    /// Tracks invoked `oneshot defcapsule` names within the current execution turn
    invoked_capsules: HashSet<String>,
}

impl LinearTypeEnv {
    pub fn bind_linear(&mut self, name: &str, profile_tag: &str) {
        self.linear_bindings
            .insert(name.to_string(), (profile_tag.to_string(), 0));
    }

    pub fn consume_linear(&mut self, name: &str, expected_profile: &str) -> Result<(), TypeError> {
        if let Some((prof, count)) = self.linear_bindings.get_mut(name) {
            if prof != expected_profile {
                return Err(TypeError::ProfileMismatch {
                    expected: expected_profile.to_string(),
                    found: prof.clone(),
                });
            }
            if *count >= 1 {
                return Err(TypeError::LinearDoubleUse {
                    binding: name.to_string(),
                });
            }
            *count += 1;
            Ok(())
        } else {
            Ok(())
        }
    }

    pub fn enter_oneshot_capsule(&mut self, capsule_name: &str) -> Result<(), TypeError> {
        if !self.invoked_capsules.insert(capsule_name.to_string()) {
            return Err(TypeError::OneshotReentry {
                capsule: capsule_name.to_string(),
            });
        }
        Ok(())
    }

    /// Verifies at scope exit that every `linear let` resource was consumed exactly once (`π = 1`).
    pub fn verify_scope_exit(&self) -> Result<(), TypeError> {
        for (name, (_prof, count)) in &self.linear_bindings {
            if *count == 0 {
                return Err(TypeError::LinearUnconsumedLeak {
                    binding: name.clone(),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_felix_linear_double_use_and_oneshot_reentry_rejected() {
        let mut env = LinearTypeEnv::default();
        env.bind_linear("vault_cap", "Profile::Aurora");

        assert!(env.consume_linear("vault_cap", "Profile::Aurora").is_ok());
        assert_eq!(
            env.consume_linear("vault_cap", "Profile::Aurora"),
            Err(TypeError::LinearDoubleUse {
                binding: "vault_cap".to_string()
            })
        );
        assert!(env.verify_scope_exit().is_ok());

        assert!(env.enter_oneshot_capsule("AuroraDyadCapsule").is_ok());
        assert_eq!(
            env.enter_oneshot_capsule("AuroraDyadCapsule"),
            Err(TypeError::OneshotReentry {
                capsule: "AuroraDyadCapsule".to_string()
            })
        );
    }

    #[test]
    fn test_toss_monozukuri_double_plateau() {
        assert_eq!(toss_freedom(0), 8);
        assert_eq!(toss_freedom(4), 4);
        assert_eq!(toss_freedom(5), 4); // Phase-locking monozukuri shelf
        assert_eq!(toss_freedom(8), 1); // Terminal crystal state
    }
}
