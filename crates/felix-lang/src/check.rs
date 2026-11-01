//! The linear-capability checker.
//!
//! One rule, enforced with both locations named: **a `linear` binding must be
//! used exactly once, and that one use must be `consume(...)`.** Everything else
//! is an error.
//!
//! Why this is the interesting part of the language rather than a decoration:
//! a value that owns memory (a profile vault, a wave grid) must not be copied,
//! and must not be silently dropped. Ordinary reference counting hides both
//! mistakes until runtime. A use-exactly-once rule turns them into compile
//! errors, and it costs about eighty lines because the language has no loops,
//! no functions and no aliasing constructs to reason about yet.

use std::collections::HashMap;

use crate::ast::{Expr, Program, Stmt};
use crate::diag::{Diagnostic, Span};

/// Where a value was bound, for diagnostics.
struct Binding {
    name_span: Span,
    linear: bool,
    uses: Vec<(Span, bool)>, // (site, is_consume)
}

/// Check every linear rule. Returns all problems, not just the first, so a
/// caller sees the whole bill in one pass.
pub fn check(program: &Program) -> Result<(), Vec<Diagnostic>> {
    let mut bindings: HashMap<String, Binding> = HashMap::new();
    let mut problems = Vec::new();

    for statement in &program.statements {
        match statement {
            Stmt::Let {
                name,
                name_span,
                linear,
                value,
                ..
            } => {
                collect_uses(value, false, &mut bindings);

                if bindings.contains_key(name) {
                    problems.push(Diagnostic::error(
                        *name_span,
                        format!("`{name}` is already bound in this scope"),
                    ));
                    continue;
                }
                // Every name is tracked, so shadowing is an error; only linear
                // ones have their use count enforced.
                bindings.insert(
                    name.clone(),
                    Binding {
                        name_span: *name_span,
                        linear: *linear,
                        uses: Vec::new(),
                    },
                );
            }
            Stmt::Consume { value, .. } => collect_uses(value, true, &mut bindings),
            Stmt::Print { value, .. } => collect_uses(value, false, &mut bindings),
        }
    }

    for (name, binding) in &bindings {
        if !binding.linear {
            continue;
        }
        match binding.uses.len() {
            0 => problems.push(Diagnostic::error(
                binding.name_span,
                format!(
                    "linear value `{name}` is never consumed; it must be retired with `consume({name})`"
                ),
            )),
            1 if !binding.uses[0].1 => problems.push(Diagnostic::error(
                binding.uses[0].0,
                format!("linear value `{name}` is used here without being consumed"),
            )),
            1 => {}
            _ => {
                let (first_span, _) = binding.uses[0];
                let mut diagnostic = Diagnostic::error(
                    binding.name_span,
                    format!(
                        "linear value `{name}` is used {} times; it may be consumed exactly once",
                        binding.uses.len()
                    ),
                );
                for (span, _) in &binding.uses {
                    diagnostic = diagnostic.with_note(*span, "used here");
                }
                debug_assert!(first_span.line >= 1);
                problems.push(diagnostic);
            }
        }
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems)
    }
}

fn collect_uses(expr: &Expr, inside_consume: bool, bindings: &mut HashMap<String, Binding>) {
    match expr {
        Expr::Int(_) => {}
        Expr::List { items, .. } => {
            for item in items {
                collect_uses(item, inside_consume, bindings);
            }
        }
        Expr::Var { name, span } => {
            if let Some(binding) = bindings.get_mut(name) {
                binding.uses.push((*span, inside_consume));
            }
        }
        Expr::Call { args, .. } => {
            for arg in args {
                collect_uses(arg, inside_consume, bindings);
            }
        }
        Expr::Contract { left, right, .. } => {
            collect_uses(left, inside_consume, bindings);
            collect_uses(right, inside_consume, bindings);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn problems(source: &str) -> Vec<Diagnostic> {
        check(&parse(source).expect("parses"))
            .err()
            .unwrap_or_default()
    }

    #[test]
    fn a_consumed_linear_value_is_fine() {
        let source = "linear let v = new_vault(); consume(v);";
        assert!(check(&parse(source).unwrap()).is_ok());
    }

    #[test]
    fn an_unconsumed_linear_value_is_an_error() {
        let found = problems("linear let v = new_vault();");
        assert_eq!(found.len(), 1);
        assert!(
            found[0].message.contains("never consumed"),
            "{}",
            found[0].message
        );
    }

    #[test]
    fn consuming_twice_is_an_error_that_names_both_sites() {
        let found = problems("linear let v = new_vault(); consume(v); consume(v);");
        assert_eq!(found.len(), 1);
        assert!(
            found[0].message.contains("exactly once"),
            "{}",
            found[0].message
        );
        assert_eq!(found[0].notes.len(), 2, "both uses should be listed");
    }

    #[test]
    fn using_a_linear_value_outside_consume_is_an_error() {
        let found = problems("linear let v = new_vault(); print(v);");
        assert_eq!(found.len(), 1);
        assert!(
            found[0].message.contains("without being consumed"),
            "{}",
            found[0].message
        );
    }

    #[test]
    fn ordinary_bindings_are_unrestricted() {
        assert!(check(&parse("let a = 1; print(a); print(a);").unwrap()).is_ok());
    }

    #[test]
    fn rebinding_a_name_in_one_scope_is_an_error() {
        let found = problems("let a = 1; let a = 2;");
        assert_eq!(found.len(), 1);
        assert!(
            found[0].message.contains("already bound"),
            "{}",
            found[0].message
        );
    }
}
