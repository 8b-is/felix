//! Evaluation.
//!
//! The only place arithmetic happens, and it is deliberately thin: integers,
//! and the ternary word type whose packing and contraction are delegated to the
//! `memnet` reference oracle rather than re-derived here. If Felix and the
//! oracle ever disagree, that is a bug in Felix by definition.

use std::collections::HashMap;

use memnet::complex_ternary::{self as ct, ComplexWord64, PackConfig};
use memnet::sovereign_convergence as sc;

use crate::ast::{Expr, Program, Stmt};

fn hex_prefix_8(bytes: &[u8; 32]) -> String {
    let mut out = String::with_capacity(16);
    for &b in &bytes[..8] {
        use std::fmt::Write;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

/// A linear value's identity. Zero-cost today; the point is that it *has* one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VaultId(pub u64);

/// A value the language can hold.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    /// A packed complex ternary word plus the scale it was packed with.
    Word {
        word: ComplexWord64,
        gamma: f32,
    },
    /// `(re, im)` integer accumulator from a contraction.
    Pair(i64, i64),
    /// Text, produced by `bt(n)`.
    Text(String),
    Vault(VaultId),
    Unit,
}

impl Value {
    /// How a value prints. Stable, because tests compare it.
    pub fn render(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Pair(re, im) => format!("({re}, {im})"),
            Value::Text(text) => text.clone(),
            Value::Vault(id) => format!("vault#{}", id.0),
            Value::Word { word, gamma } => format!(
                "word{{gamma:{gamma}, re:{:+}/{:-}, im:{:+}/{:-}}}",
                word.r_pos.count_ones(),
                word.r_neg.count_ones(),
                word.i_pos.count_ones(),
                word.i_neg.count_ones()
            ),
            Value::Unit => "()".to_string(),
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }
}

/// Run a checked program. Returns every `print` in order.
pub fn run(program: &Program) -> Vec<(String, Value)> {
    let mut env: HashMap<String, Value> = HashMap::new();
    let mut out = Vec::new();
    let mut next_vault = 0u64;

    for statement in &program.statements {
        match statement {
            Stmt::Let { name, value, .. } => {
                let evaluated = eval(value, &env, &mut next_vault);
                env.insert(name.clone(), evaluated);
            }
            Stmt::Consume { value, .. } => {
                let evaluated = eval(value, &env, &mut next_vault);
                if let Value::Vault(_) = evaluated {
                    // A consumed vault leaves the environment: this is the
                    // runtime half of the linear rule the checker proves.
                    if let Expr::Var { name, .. } = value {
                        env.remove(name);
                    }
                }
            }
            Stmt::Print { value, .. } => {
                let evaluated = eval(value, &env, &mut next_vault);
                out.push((evaluated.render(), evaluated));
            }
        }
    }
    out
}

fn eval(expr: &Expr, env: &HashMap<String, Value>, next_vault: &mut u64) -> Value {
    match expr {
        Expr::Int(value) => Value::Int(*value),
        Expr::List { .. } => Value::Unit, // lists only exist as call arguments
        Expr::Var { name, .. } => env.get(name).cloned().unwrap_or(Value::Unit),
        Expr::Call { callee, args, .. } => call(callee, args, env, next_vault),
        Expr::Contract { left, right, .. } => {
            let a = eval(left, env, next_vault);
            let b = eval(right, env, next_vault);
            match (a, b) {
                (Value::Word { word: wa, .. }, Value::Word { word: wb, .. }) => {
                    let (re, im) = ct::contract_row(&[wa], &[wb]);
                    Value::Pair(re, im)
                }
                _ => Value::Unit,
            }
        }
    }
}

fn call(callee: &str, args: &[Expr], env: &HashMap<String, Value>, next_vault: &mut u64) -> Value {
    match (callee, args) {
        // pack([samples...], gamma) — integer samples scaled into the wavefield.
        ("pack", [Expr::List { items, .. }, gamma]) => {
            let gamma = eval(gamma, env, next_vault).as_int().unwrap_or(1) as f32;
            let samples: Vec<f32> = items
                .iter()
                .map(|item| eval(item, env, next_vault).as_int().unwrap_or(0) as f32)
                .collect();
            // The imaginary quadrature is zero for the plain sample list; a
            // future `packiq` would take pairs. Stating it beats implying it.
            let quadrature = vec![0.0f32; samples.len()];
            let words = ct::pack_row_memoryless(
                &samples,
                &quadrature,
                PackConfig {
                    gamma,
                    carrier_rad: 0.0,
                },
            );
            Value::Word {
                word: words.first().copied().unwrap_or(ComplexWord64::ZERO),
                gamma,
            }
        }

        // fib(n) — the NAND-only cascade from the oracle.
        ("fib", [n]) => Value::Int(ct::fib_nand(
            eval(n, env, next_vault).as_int().unwrap_or(0).max(0) as usize,
        ) as i64),

        // bt(n) — balanced ternary, least significant digit first.
        ("bt", [n]) => Value::Text(ct::balanced_ternary(
            eval(n, env, next_vault).as_int().unwrap_or(0),
        )),

        ("new_vault", []) => {
            *next_vault += 1;
            Value::Vault(VaultId(*next_vault))
        }

        ("genesis_seal", []) => {
            let (root, _) = sc::derive_learning_genesis();
            Value::Text(format!("linosv:{}", hex_prefix_8(&root)))
        }

        ("aurora_seal", []) => {
            let seal = sc::derive_aurora_dyad_seal();
            Value::Text(format!("aurora:{}", hex_prefix_8(&seal)))
        }

        ("paprika_root", []) => {
            let root = sc::derive_spherepopfold_root();
            Value::Text(format!("paprika:{}", hex_prefix_8(&root)))
        }

        ("spherepop_fold", [Expr::List { items, .. }, gamma]) => {
            let gamma = eval(gamma, env, next_vault).as_int().unwrap_or(1) as f32;
            let mut re = [0.0f32; 64];
            let im = [0.0f32; 64];
            for (i, item) in items.iter().take(64).enumerate() {
                re[i] = eval(item, env, next_vault).as_int().unwrap_or(0) as f32;
            }
            let (word, _) = sc::spherepop_fold_64(&re, &im, 0.0);
            Value::Word { word, gamma }
        }

        ("toss_freedom", [step]) => {
            let s = eval(step, env, next_vault).as_int().unwrap_or(1);
            let idx = s.saturating_sub(1).clamp(0, 8) as usize;
            Value::Int(sc::TOSS_FREEDOM_LADDER[idx] as i64)
        }

        _ => Value::Unit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{check, parser};

    fn run_ok(source: &str) -> Vec<(String, Value)> {
        let program = parser::parse(source).expect("parses");
        check::check(&program).expect("checks");
        run(&program)
    }

    fn rendered(source: &str) -> Vec<String> {
        run_ok(source).into_iter().map(|(text, _)| text).collect()
    }

    #[test]
    fn integers_print() {
        assert_eq!(rendered("let a = 41; print(a);"), vec!["41"]);
    }

    #[test]
    fn the_gate_cascade_is_reachable_from_the_language() {
        assert_eq!(rendered("print(fib(12));"), vec!["144"]);
        assert_eq!(rendered("print(fib(0));"), vec!["0"]);
    }

    #[test]
    fn balanced_ternary_is_reachable_from_the_language() {
        assert_eq!(rendered("print(bt(5));"), vec!["--+"]);
        assert_eq!(rendered("print(bt(144));"), vec!["00+--+"]);
    }

    #[test]
    fn packing_is_bit_identical_to_the_oracle() {
        let source = "let w = pack([3, -3, 0, 0], 2); print(w);";
        let value = &run_ok(source)[0].1;
        match value {
            Value::Word { word, .. } => {
                let oracle = ct::pack_row_memoryless(
                    &[3.0, -3.0, 0.0, 0.0],
                    &[0.0, 0.0, 0.0, 0.0],
                    PackConfig {
                        gamma: 2.0,
                        carrier_rad: 0.0,
                    },
                );
                assert_eq!(*word, oracle[0], "felix must not invent its own packing");
            }
            other => panic!("expected a word, got {other:?}"),
        }
    }

    #[test]
    fn contraction_is_bit_identical_to_the_oracle() {
        let source = "let a = pack([3, -3, 0, 0], 2); \
                      let b = pack([1, 1, 1, 1], 1); \
                      print(a <~> b);";
        let value = &run_ok(source)[0].1;

        let a = ct::pack_row_memoryless(
            &[3.0, -3.0, 0.0, 0.0],
            &[0.0; 4],
            PackConfig {
                gamma: 2.0,
                carrier_rad: 0.0,
            },
        )[0];
        let b = ct::pack_row_memoryless(
            &[1.0; 4],
            &[0.0; 4],
            PackConfig {
                gamma: 1.0,
                carrier_rad: 0.0,
            },
        )[0];
        let (re, im) = ct::contract_row(&[a], &[b]);

        assert_eq!(*value, Value::Pair(re, im));
    }

    #[test]
    fn contraction_of_a_word_with_itself_is_pure_real_energy() {
        let rendered = rendered("let a = pack([2, -2, 2], 1); print(a <~> a);");
        assert!(rendered[0].ends_with(", 0)"), "got {}", rendered[0]);
    }

    #[test]
    fn vault_identities_are_distinct() {
        // An ordinary (non-linear) binding can be printed, which is how a vault
        // id becomes observable. A *linear* vault cannot be printed at all — the
        // checker rejects it before evaluation, and `check.rs` proves that.
        let printed = rendered("let a = new_vault(); let b = new_vault(); print(a); print(b);");
        assert_eq!(printed, vec!["vault#1".to_string(), "vault#2".to_string()]);
    }

    #[test]
    fn a_consumed_linear_vault_runs_without_error() {
        let program = parser::parse("linear let v = new_vault(); consume(v);").unwrap();
        check::check(&program).expect("exactly one consume is legal");
        assert!(run(&program).is_empty());
    }

    #[test]
    fn aurora_dyad_genesis_example_executes_and_matches_oracle() {
        let source = include_str!("../../../examples/03_aurora_dyad_genesis.flx");
        let lines = rendered(source);
        assert_eq!(lines[0], "linosv:9e41d593714be891");
        assert_eq!(lines[1], "aurora:121f3afae721cf03");
        assert_eq!(lines[2], "paprika:050be6d3c621720f");
        assert_eq!(lines[5], "8");
        assert_eq!(lines[6], "4");
        assert_eq!(lines[7], "4");
        assert_eq!(lines[8], "1");
    }
}
