//! Evaluation.
//!
//! The only place arithmetic happens, and it is deliberately thin: integers,
//! exact ratios, and the ternary word type whose packing and contraction are
//! delegated to the `memnet` reference oracle rather than re-derived here. If
//! Felix and the oracle ever disagree, that is a bug in Felix by definition.
//!
//! ## The `p/q` contract
//!
//! A word's scale `gamma` is an exact [`Ratio`], and `re(...)` / `im(...)` of a
//! contraction are exact ratios too: `acc · γa · γb`, never rounded. The oracle
//! packs in `f32`, so Felix hands it `p as f32 / q as f32` — and only inside the
//! range where that conversion **cannot change a single trit**: samples, `p`
//! and `q` all below `2^24`. Inside that box the oracle's dead-zone comparison
//! `s > γ/2` decides exactly as the integer comparison `2·q·s > p` would
//! (`p_over_q_never_changes_a_trit` checks this against the oracle). Outside
//! it, `pack` is a runtime error rather than a quiet approximation.
//!
//! A program that fails at runtime prints nothing: the result is a whole
//! transcript or one diagnostic, never half of one.

use std::collections::HashMap;

use memnet::complex_ternary::{self as ct, ComplexWord64, PackConfig};
use memnet::sovereign_convergence as sc;

use crate::ast::{Expr, Program, Stmt};
use crate::check::Type;
use crate::diag::{Diagnostic, Span};
use crate::ratio::Ratio;

/// Integers handed to the `f32` oracle must be strictly inside `±2^24`, where
/// every integer is exactly representable. See the module docs for why this
/// bound also makes ratio scales exact enough to never move a trit.
pub const EXACT_F32: i64 = 1 << 24;

/// The largest `n` whose Fibonacci number fits an `i64`.
pub const FIB_MAX: i64 = 92;

/// A linear value's identity. Zero-cost today; the point is that it *has* one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VaultId(pub u64);

/// A value the language can hold.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    /// An exact `p/q`.
    Ratio(Ratio),
    /// A packed complex ternary word plus the exact scale it was packed with.
    Word {
        word: ComplexWord64,
        gamma: Ratio,
    },
    /// `(re, im)` integer accumulator from a contraction, and the exact scale
    /// `γa · γb` that turns it back into field units.
    Pair {
        re: i64,
        im: i64,
        scale: Ratio,
    },
    /// Text, produced by `bt(n)` and the seal builtins.
    Text(String),
    Vault(VaultId),
}

impl Value {
    /// How a value prints. Stable, because golden files compare it.
    pub fn render(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Ratio(r) => r.to_string(),
            Value::Pair { re, im, .. } => format!("({re}, {im})"),
            Value::Text(text) => text.clone(),
            Value::Vault(id) => format!("vault#{}", id.0),
            Value::Word { word, gamma } => format!(
                "word{{gamma:{gamma}, re:{:+}/{:-}, im:{:+}/{:-}}}",
                word.r_pos.count_ones(),
                word.r_neg.count_ones(),
                word.i_pos.count_ones(),
                word.i_neg.count_ones()
            ),
        }
    }

    /// The static type this value inhabits.
    pub fn ty(&self) -> Type {
        match self {
            Value::Int(_) => Type::Int,
            Value::Ratio(_) => Type::Ratio,
            Value::Word { .. } => Type::Word,
            Value::Pair { .. } => Type::Pair,
            Value::Text(_) => Type::Text,
            Value::Vault(_) => Type::Vault,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }
}

/// One `print`: its rendering and the value behind it.
pub type Output = (String, Value);

/// Run a checked program. Returns every `print` in order, or the first runtime
/// error.
pub fn run(program: &Program) -> Result<Vec<Output>, Diagnostic> {
    let mut machine = Machine {
        env: HashMap::new(),
        next_vault: 0,
    };
    let mut out = Vec::new();

    for statement in &program.statements {
        match statement {
            Stmt::Let { name, value, .. } => {
                let evaluated = machine.eval(value)?;
                machine.env.insert(name.clone(), evaluated);
            }
            Stmt::Consume { value, .. } => {
                // A consumed value leaves the environment: this is the runtime
                // half of the linear rule the checker proves.
                if let Expr::Var { name, .. } = value {
                    machine.env.remove(name);
                }
            }
            Stmt::Print { value, .. } => {
                let evaluated = machine.eval(value)?;
                out.push((evaluated.render(), evaluated));
            }
        }
    }
    Ok(out)
}

struct Machine {
    env: HashMap<String, Value>,
    next_vault: u64,
}

/// The checker let something through it should not have. Reported, not
/// panicked on, so a checker bug is a diagnostic with a location.
fn internal(span: Span, what: &str) -> Diagnostic {
    Diagnostic::error(
        span,
        format!("internal: the checker should have rejected {what}"),
    )
}

fn seal_hex(prefix: &str, root: &[u8; 32]) -> Value {
    let hex: String = root.iter().take(8).map(|b| format!("{b:02x}")).collect();
    Value::Text(format!("{prefix}:{hex}"))
}

impl Machine {
    fn eval(&mut self, expr: &Expr) -> Result<Value, Diagnostic> {
        match expr {
            Expr::Int { value, .. } => Ok(Value::Int(*value)),
            Expr::Ratio { value, .. } => Ok(Value::Ratio(*value)),
            Expr::List { span, .. } => Err(internal(*span, "a list outside a call")),
            Expr::Var { name, span } => self
                .env
                .get(name)
                .cloned()
                .ok_or_else(|| internal(*span, "an unbound name")),
            Expr::Call {
                callee, args, span, ..
            } => self.call(callee, args, *span),
            Expr::Contract { left, right, span } => match (self.eval(left)?, self.eval(right)?) {
                (
                    Value::Word {
                        word: wa,
                        gamma: ga,
                    },
                    Value::Word {
                        word: wb,
                        gamma: gb,
                    },
                ) => {
                    let (re, im) = ct::contract_row(&[wa], &[wb]);
                    let scale = ga.checked_mul(gb).ok_or_else(|| {
                        Diagnostic::error(*span, "the product of the two scales overflows")
                    })?;
                    Ok(Value::Pair { re, im, scale })
                }
                _ => Err(internal(*span, "a contraction of non-words")),
            },
        }
    }

    fn int(&mut self, expr: &Expr) -> Result<i64, Diagnostic> {
        self.eval(expr)?
            .as_int()
            .ok_or_else(|| internal(expr.span(), "a non-Int argument"))
    }

    /// Integer samples, each inside the exact-`f32` box.
    fn samples(&mut self, expr: &Expr) -> Result<Vec<f32>, Diagnostic> {
        let Expr::List { items, .. } = expr else {
            return Err(internal(expr.span(), "samples that are not a list"));
        };
        items
            .iter()
            .map(|item| {
                let n = self.int(item)?;
                if n.abs() >= EXACT_F32 {
                    return Err(Diagnostic::error(
                        item.span(),
                        format!(
                            "sample {n} is outside ±2^24, where the f32 oracle stops being exact"
                        ),
                    ));
                }
                Ok(n as f32)
            })
            .collect()
    }

    fn call(&mut self, callee: &str, args: &[Expr], span: Span) -> Result<Value, Diagnostic> {
        match (callee, args) {
            // pack([samples...], gamma) — dead-zone quantise into one word.
            ("pack", [samples, gamma_expr]) => {
                let samples = self.samples(samples)?;
                let gamma = match self.eval(gamma_expr)? {
                    Value::Int(n) => Ratio::int(n),
                    Value::Ratio(r) => r,
                    _ => return Err(internal(gamma_expr.span(), "a non-scale gamma")),
                };
                if !gamma.is_positive() {
                    return Err(Diagnostic::error(
                        gamma_expr.span(),
                        format!("a scale must be positive, found {gamma}"),
                    ));
                }
                if gamma.numer() >= EXACT_F32 || gamma.denom() >= EXACT_F32 {
                    return Err(Diagnostic::error(
                        gamma_expr.span(),
                        format!("scale {gamma} has a term outside 2^24, where the f32 oracle stops being exact"),
                    ));
                }
                // The imaginary quadrature is zero for the plain sample list; a
                // future `packiq` would take pairs. Stating it beats implying it.
                let quadrature = vec![0.0f32; samples.len()];
                let words = ct::pack_row_memoryless(
                    &samples,
                    &quadrature,
                    PackConfig {
                        gamma: gamma.numer() as f32 / gamma.denom() as f32,
                        carrier_rad: 0.0,
                    },
                );
                Ok(Value::Word {
                    word: words.first().copied().unwrap_or(ComplexWord64::ZERO),
                    gamma,
                })
            }

            // re(pair), im(pair) — exact field value `acc · γa · γb`.
            ("re" | "im", [pair]) => match self.eval(pair)? {
                Value::Pair { re, im, scale } => {
                    let acc = if callee == "re" { re } else { im };
                    scale.checked_mul_int(acc).map(Value::Ratio).ok_or_else(|| {
                        Diagnostic::error(span, "the scaled value overflows i64/i64")
                    })
                }
                _ => Err(internal(pair.span(), "a non-Pair argument")),
            },

            // fib(n) — the NAND-only cascade from the oracle.
            ("fib", [n_expr]) => {
                let n = self.int(n_expr)?;
                if !(0..=FIB_MAX).contains(&n) {
                    return Err(Diagnostic::error(
                        n_expr.span(),
                        format!("fib is defined here for 0..={FIB_MAX} (F({FIB_MAX}) is the last that fits i64), found {n}"),
                    ));
                }
                Ok(Value::Int(ct::fib_nand(n as usize) as i64))
            }

            // bt(n) — balanced ternary, least significant digit first.
            ("bt", [n]) => Ok(Value::Text(ct::balanced_ternary(self.int(n)?))),

            ("new_vault", []) => {
                self.next_vault += 1;
                Ok(Value::Vault(VaultId(self.next_vault)))
            }

            // The 8b seals: the first 8 bytes of fixed, domain-separated
            // HMAC-SHA256 digests over constants in `memnet::sovereign_convergence`.
            // They are identifiers. They do not attest to anything beyond
            // "these constants have not changed", which the golden tests check.
            ("genesis_seal", []) => Ok(seal_hex("linosv", &sc::derive_learning_genesis().0)),
            ("aurora_seal", []) => Ok(seal_hex("aurora", &sc::derive_aurora_dyad_seal())),
            ("paprika_root", []) => Ok(seal_hex("paprika", &sc::derive_spherepopfold_root())),

            // toss_freedom(step) — the 9-step TOSS freedom ladder, 1-based.
            ("toss_freedom", [step_expr]) => {
                let step = self.int(step_expr)?;
                let ladder = sc::TOSS_FREEDOM_LADDER;
                if !(1..=ladder.len() as i64).contains(&step) {
                    return Err(Diagnostic::error(
                        step_expr.span(),
                        format!("toss_freedom has steps 1..={}, found {step}", ladder.len()),
                    ));
                }
                Ok(Value::Int(ladder[step as usize - 1] as i64))
            }

            // spherepop_fold([samples...]) — unit-sphere projection, then the
            // oracle's sigma-delta fold at its own fixed carrier. The oracle
            // packs at gamma 1, so the word records gamma 1: a gamma argument
            // here would have been a number the computation never read.
            ("spherepop_fold", [samples]) => {
                let samples = self.samples(samples)?;
                let mut re = [0.0f32; 64];
                re[..samples.len()].copy_from_slice(&samples);
                let (word, _seal) =
                    sc::spherepop_fold_64(&re, &[0.0f32; 64], SPHEREPOP_CARRIER_RAD);
                Ok(Value::Word {
                    word,
                    gamma: Ratio::ONE,
                })
            }

            // god_of_sun(pq, n, t) — exact ULTRA-ZENO GOD OF SUN surplus `(P/Q - n*t) - P`.
            // Positive return value witnesses `P/Q - n*t > P`.
            ("god_of_sun", [pq_expr, n_expr, t_expr]) => {
                let pq = match self.eval(pq_expr)? {
                    Value::Int(v) => Ratio::int(v),
                    Value::Ratio(r) => r,
                    _ => return Err(internal(pq_expr.span(), "a non-scale argument")),
                };
                let n = self.int(n_expr)?;
                if n < 0 {
                    return Err(Diagnostic::error(
                        n_expr.span(),
                        format!("god_of_sun step count n must be >= 0, found {n}"),
                    ));
                }
                let t_raw = self.int(t_expr)?;
                let t = t_raw.clamp(-1, 1);
                let (_holds, surplus_num, _n_sun) =
                    crate::phoenix::felix_zeno_god_of_sun_pq_nt_gt_p(
                        pq.numer(),
                        pq.denom() as u64,
                        n as u64,
                        t as i8,
                    );
                let num_i64 = i64::try_from(surplus_num)
                    .map_err(|_| Diagnostic::error(span, "god_of_sun surplus overflows i64/i64"))?;
                let ratio = Ratio::new(num_i64, pq.denom()).ok_or_else(|| {
                    Diagnostic::error(span, "god_of_sun ratio denominator is zero")
                })?;
                Ok(Value::Ratio(ratio))
            }

            _ => Err(internal(span, "an unknown builtin or a wrong arity")),
        }
    }
}

/// The carrier `spherepop_fold` passes to the oracle. Fixed, so that the same
/// samples always fold to the same word on every implementation.
pub const SPHEREPOP_CARRIER_RAD: f32 = 0.15;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{self, Param, BUILTINS};
    use crate::parser;

    fn try_run(source: &str) -> Result<Vec<Output>, Diagnostic> {
        let program = parser::parse(source).expect("parses");
        check::check(&program).expect("checks");
        run(&program)
    }

    fn run_ok(source: &str) -> Vec<Output> {
        try_run(source).expect("runs")
    }

    fn rendered(source: &str) -> Vec<String> {
        run_ok(source).into_iter().map(|(text, _)| text).collect()
    }

    fn runtime_error(source: &str) -> Diagnostic {
        try_run(source).expect_err("should fail at runtime")
    }

    #[test]
    fn integers_print() {
        assert_eq!(rendered("let a = 41; print(a);"), vec!["41"]);
    }

    #[test]
    fn ratios_print_reduced() {
        assert_eq!(rendered("print(6/4); print(4/2);"), vec!["3/2", "2"]);
    }

    #[test]
    fn the_gate_cascade_is_reachable_from_the_language() {
        assert_eq!(rendered("print(fib(12));"), vec!["144"]);
        assert_eq!(rendered("print(fib(0));"), vec!["0"]);
        assert_eq!(rendered("print(fib(92));"), vec!["7540113804746346429"]);
    }

    #[test]
    fn fib_past_i64_is_a_runtime_error_not_a_wrap() {
        let err = runtime_error("print(fib(93));");
        assert!(err.message.contains("0..=92"), "{}", err.message);
        assert_eq!(err.span.column, 11);
        assert!(runtime_error("print(fib(-1));")
            .message
            .contains("found -1"));
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
            Value::Word { word, gamma } => {
                let oracle = ct::pack_row_memoryless(
                    &[3.0, -3.0, 0.0, 0.0],
                    &[0.0, 0.0, 0.0, 0.0],
                    PackConfig {
                        gamma: 2.0,
                        carrier_rad: 0.0,
                    },
                );
                assert_eq!(*word, oracle[0], "felix must not invent its own packing");
                assert_eq!(*gamma, Ratio::int(2));
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

        assert_eq!(
            *value,
            Value::Pair {
                re,
                im,
                scale: Ratio::int(2)
            }
        );
    }

    #[test]
    fn contraction_of_a_word_with_itself_is_pure_real_energy() {
        let rendered = rendered("let a = pack([2, -2, 2], 1); print(a <~> a);");
        assert!(rendered[0].ends_with(", 0)"), "got {}", rendered[0]);
    }

    #[test]
    fn scaled_contraction_is_exact_p_over_q() {
        // Three agreeing trits at scales 1/3 and 3/7: 3 · 1/3 · 3/7 = 3/7.
        let out = rendered(
            "let a = pack([1, 1, 1], 1/3); \
             let b = pack([5, 5, 5], 3/7); \
             let p = a <~> b; \
             print(a); print(p); print(re(p)); print(im(p));",
        );
        assert_eq!(
            out,
            vec!["word{gamma:1/3, re:+3/0, im:+0/0}", "(3, 0)", "3/7", "0"]
        );
    }

    /// The `p/q` claim in the module docs, checked against the oracle: for
    /// integer samples and scales with terms below 2^24, the f32 the oracle
    /// receives decides every trit exactly as the integer predicate would.
    #[test]
    fn p_over_q_never_changes_a_trit() {
        let exact = |s: i64, p: i64, q: i64| -> i8 {
            let (lhs, rhs) = (2 * q as i128 * s as i128, p as i128);
            if lhs > rhs {
                1
            } else if lhs < -rhs {
                -1
            } else {
                0
            }
        };
        let edge = EXACT_F32 - 1;
        let scales = [
            (1, 1),
            (2, 1),
            (1, 3),
            (2, 3),
            (10, 3),
            (7, 2),
            (9, 4),
            (1, 7919),
            (edge, 1),
            (edge, 2),
            (edge, 3),
            (edge - 1, edge),
            (1, edge),
            (edge, edge - 2),
            (8_388_607, 1),
            (16_777_213, 5),
        ];
        for (p, q) in scales {
            let gamma = Ratio::new(p, q).unwrap();
            // Every boundary the dead zone can have, and its neighbours.
            let t = p / (2 * q);
            let mut probes = vec![0, 1, -1, 2, -2, edge, -edge, edge - 1];
            for d in -2..=2 {
                probes.push(t + d);
                probes.push(-(t + d));
            }
            probes.retain(|s| s.abs() < EXACT_F32);
            let samples: Vec<f32> = probes.iter().map(|&s| s as f32).collect();
            let quad = vec![0.0; samples.len()];
            let cfg = PackConfig {
                gamma: gamma.numer() as f32 / gamma.denom() as f32,
                carrier_rad: 0.0,
            };
            let words = ct::pack_row_memoryless(&samples, &quad, cfg);
            for (i, &s) in probes.iter().enumerate() {
                assert_eq!(
                    words[i / 64].real_trit(i % 64),
                    exact(s, gamma.numer(), gamma.denom()),
                    "sample {s} at scale {gamma}"
                );
            }
        }
    }

    #[test]
    fn values_outside_the_exact_box_are_refused() {
        assert!(runtime_error("let w = pack([16777216], 1);")
            .message
            .contains("±2^24"));
        assert!(runtime_error("let w = pack([1], 16777216);")
            .message
            .contains("2^24"));
        assert!(runtime_error("let w = pack([1], 1/16777216);")
            .message
            .contains("2^24"));
        assert!(
            rendered("let w = pack([16777215], 16777215); print(w);")[0].contains("gamma:16777215")
        );
    }

    #[test]
    fn a_non_positive_scale_is_refused() {
        let err = runtime_error("let w = pack([1], 0);");
        assert!(err.message.contains("positive"), "{}", err.message);
        assert!(runtime_error("let w = pack([1], -1/2);")
            .message
            .contains("-1/2"));
    }

    #[test]
    fn a_failing_program_prints_nothing() {
        let program = parser::parse("print(1); print(fib(100)); print(2);").unwrap();
        check::check(&program).unwrap();
        assert!(run(&program).is_err());
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
        assert!(run_ok("linear let v = new_vault(); consume(v);").is_empty());
    }

    #[test]
    fn the_8b_seals_are_pinned() {
        // These are digests of constants in memnet; if one moves, this fails
        // and the golden file for example 03 fails with it.
        assert_eq!(
            rendered("print(genesis_seal()); print(aurora_seal()); print(paprika_root());"),
            vec![
                "linosv:9e41d593714be891",
                "aurora:121f3afae721cf03",
                "paprika:050be6d3c621720f"
            ]
        );
    }

    #[test]
    fn the_toss_ladder_is_reachable_and_bounded() {
        let ladder: Vec<String> = (1..=9)
            .flat_map(|step| rendered(&format!("print(toss_freedom({step}));")))
            .collect();
        assert_eq!(ladder, vec!["8", "7", "6", "5", "4", "4", "3", "2", "1"]);
        let err = runtime_error("print(toss_freedom(10));");
        assert!(err.message.contains("1..=9, found 10"), "{}", err.message);
        assert!(runtime_error("print(toss_freedom(0));")
            .message
            .contains("found 0"));
    }

    #[test]
    fn spherepop_fold_is_bit_identical_to_the_oracle() {
        let source =
            "let w = spherepop_fold([4, -4, 2, -2, 3, -3, 1, -1]); print(w); print(w <~> w);";
        let out = run_ok(source);
        let mut re = [0.0f32; 64];
        re[..8].copy_from_slice(&[4.0, -4.0, 2.0, -2.0, 3.0, -3.0, 1.0, -1.0]);
        let (oracle, _) = sc::spherepop_fold_64(&re, &[0.0; 64], SPHEREPOP_CARRIER_RAD);
        assert_eq!(
            out[0].1,
            Value::Word {
                word: oracle,
                gamma: Ratio::ONE
            }
        );
        assert!(
            out[1].0.ends_with(", 0)"),
            "self-contraction has zero imaginary residual"
        );
    }

    /// Walk the builtin table: every row must evaluate, and produce the type the
    /// checker promised. This is what stops a builtin from being a stub.
    #[test]
    fn every_builtin_evaluates_to_its_declared_type() {
        for builtin in BUILTINS {
            let args: Vec<&str> = builtin
                .params
                .iter()
                .map(|param| match param {
                    Param::Int => "3",
                    Param::Scale => "1/2",
                    Param::Samples => "[1, -1, 0]",
                    Param::Pair => "pack([1], 1) <~> pack([1], 1)",
                })
                .collect();
            let source = format!("print({}({}));", builtin.name, args.join(", "));
            let out = try_run(&source).unwrap_or_else(|e| panic!("{source}: {e}"));
            assert_eq!(out[0].1.ty(), builtin.returns, "{source}");
        }
    }
}
