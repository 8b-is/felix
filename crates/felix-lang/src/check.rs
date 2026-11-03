//! The checker: types, then the linear-capability rule.
//!
//! Two jobs, both done before a single value exists:
//!
//! 1. **Every expression has a type, and every call matches its builtin.** In
//!    v0.1 an unknown name, a misspelt builtin or a word contracted with an
//!    integer all evaluated to `()` and printed it. That is a stub wearing a
//!    value's clothes, so v1.0 rejects all of them here, with a span.
//! 2. **A `linear` binding must be used exactly once, and that one use must be
//!    `consume(...)`.** Both locations are named when it is not.
//!
//! Why the linear rule is the interesting part of the language rather than a
//! decoration: a value that owns memory (a profile vault, a wave grid) must not
//! be copied, and must not be silently dropped. Ordinary reference counting
//! hides both mistakes until runtime. A use-exactly-once rule turns them into
//! compile errors, and it stays small because the language has no loops, no
//! functions and no aliasing constructs to reason about.

use std::collections::HashMap;
use std::fmt;

use crate::ast::{Expr, Program, Stmt};
use crate::diag::{Diagnostic, Span};

/// The most samples one word holds. `pack` and `spherepop_fold` refuse more
/// rather than silently dropping everything after the 64th.
pub const WORD_NODES: usize = 64;

/// Every type a Felix expression can have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Int,
    Ratio,
    Word,
    Pair,
    Text,
    Vault,
    /// A list literal. Legal only where a builtin asks for samples.
    List,
    /// Already reported; suppresses follow-on errors from the same mistake.
    Error,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Type::Int => "Int",
            Type::Ratio => "Ratio",
            Type::Word => "Word",
            Type::Pair => "Pair",
            Type::Text => "Text",
            Type::Vault => "Vault",
            Type::List => "List",
            Type::Error => "{error}",
        })
    }
}

/// What a builtin parameter accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Param {
    /// An `Int`.
    Int,
    /// A block scale: `Int` or `Ratio`.
    Scale,
    /// A list literal of at most [`WORD_NODES`] `Int`s.
    Samples,
    /// A contraction result.
    Pair,
}

impl Param {
    fn describe(&self) -> &'static str {
        match self {
            Param::Int => "an Int",
            Param::Scale => "an Int or a Ratio",
            Param::Samples => "a list of at most 64 Ints",
            Param::Pair => "a Pair",
        }
    }
}

/// One row of the builtin table. The evaluator implements exactly these.
pub struct Builtin {
    pub name: &'static str,
    pub params: &'static [Param],
    pub returns: Type,
}

/// The whole standard library. Adding a row without an evaluator arm and a
/// test is how v0.1 ended up printing `()`; the `every_builtin_evaluates`
/// test in `eval.rs` walks this table so that cannot happen again.
pub const BUILTINS: &[Builtin] = &[
    Builtin {
        name: "pack",
        params: &[Param::Samples, Param::Scale],
        returns: Type::Word,
    },
    Builtin {
        name: "fib",
        params: &[Param::Int],
        returns: Type::Int,
    },
    Builtin {
        name: "bt",
        params: &[Param::Int],
        returns: Type::Text,
    },
    Builtin {
        name: "new_vault",
        params: &[],
        returns: Type::Vault,
    },
    Builtin {
        name: "re",
        params: &[Param::Pair],
        returns: Type::Ratio,
    },
    Builtin {
        name: "im",
        params: &[Param::Pair],
        returns: Type::Ratio,
    },
    Builtin {
        name: "genesis_seal",
        params: &[],
        returns: Type::Text,
    },
    Builtin {
        name: "aurora_seal",
        params: &[],
        returns: Type::Text,
    },
    Builtin {
        name: "paprika_root",
        params: &[],
        returns: Type::Text,
    },
    Builtin {
        name: "toss_freedom",
        params: &[Param::Int],
        returns: Type::Int,
    },
    Builtin {
        name: "spherepop_fold",
        params: &[Param::Samples],
        returns: Type::Word,
    },
];

pub fn builtin(name: &str) -> Option<&'static Builtin> {
    BUILTINS.iter().find(|b| b.name == name)
}

/// A binding, as the checker saw it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub name: String,
    pub name_span: Span,
    pub ty: Type,
    pub linear: bool,
    /// `(site, is_consume)` for every use, in source order.
    pub uses: Vec<(Span, bool)>,
}

/// What a successful check proves, kept for `felix graph`.
#[derive(Debug, Clone, Default)]
pub struct Checked {
    pub bindings: Vec<Binding>,
}

impl Checked {
    pub fn type_of(&self, name: &str) -> Option<Type> {
        self.bindings.iter().find(|b| b.name == name).map(|b| b.ty)
    }
}

struct Checker {
    bindings: Vec<Binding>,
    index: HashMap<String, usize>,
    problems: Vec<Diagnostic>,
}

/// Check types and the linear rule. Returns all problems, not just the first,
/// sorted by position, so a caller sees the whole bill in one pass.
pub fn check(program: &Program) -> Result<Checked, Vec<Diagnostic>> {
    let mut c = Checker {
        bindings: Vec::new(),
        index: HashMap::new(),
        problems: Vec::new(),
    };

    for statement in &program.statements {
        match statement {
            Stmt::Let {
                name,
                name_span,
                linear,
                value,
                ..
            } => {
                let ty = c.infer(value);
                if ty == Type::List {
                    c.list_outside_call(value.span());
                }
                if c.index.contains_key(name) {
                    c.problems.push(Diagnostic::error(
                        *name_span,
                        format!("`{name}` is already bound in this scope"),
                    ));
                    continue;
                }
                c.index.insert(name.clone(), c.bindings.len());
                c.bindings.push(Binding {
                    name: name.clone(),
                    name_span: *name_span,
                    ty,
                    linear: *linear,
                    uses: Vec::new(),
                });
            }
            Stmt::Consume { value, .. } => c.consume(value),
            Stmt::Print { value, .. } => {
                if c.infer(value) == Type::List {
                    c.list_outside_call(value.span());
                }
            }
        }
    }

    for binding in &c.bindings {
        if !binding.linear {
            continue;
        }
        let name = &binding.name;
        match binding.uses.len() {
            0 => c.problems.push(Diagnostic::error(
                binding.name_span,
                format!(
                    "linear value `{name}` is never consumed; it must be retired with `consume({name})`"
                ),
            )),
            1 if !binding.uses[0].1 => c.problems.push(Diagnostic::error(
                binding.uses[0].0,
                format!("linear value `{name}` is used here without being consumed"),
            )),
            1 => {}
            n => {
                let mut diagnostic = Diagnostic::error(
                    binding.name_span,
                    format!("linear value `{name}` is used {n} times; it may be consumed exactly once"),
                );
                for (span, _) in &binding.uses {
                    diagnostic = diagnostic.with_note(*span, "used here");
                }
                c.problems.push(diagnostic);
            }
        }
    }

    if c.problems.is_empty() {
        Ok(Checked {
            bindings: c.bindings,
        })
    } else {
        c.problems
            .sort_by_key(|d| (d.span.line, d.span.column, d.span.start));
        Err(c.problems)
    }
}

impl Checker {
    fn error(&mut self, span: Span, message: impl Into<String>) -> Type {
        self.problems.push(Diagnostic::error(span, message));
        Type::Error
    }

    fn list_outside_call(&mut self, span: Span) {
        self.error(
            span,
            "a list only exists as a builtin argument, such as `pack([...], 1)`",
        );
    }

    fn consume(&mut self, value: &Expr) {
        let Expr::Var { name, span } = value else {
            self.infer(value);
            self.error(
                value.span(),
                "`consume` takes a linear binding by name, like `consume(v)`",
            );
            return;
        };
        match self.index.get(name).map(|&i| self.bindings[i].linear) {
            Some(true) => {
                let i = self.index[name];
                self.bindings[i].uses.push((*span, true));
            }
            Some(false) => {
                self.error(
                    *span,
                    format!("`{name}` is not linear; only a `linear let` binding can be consumed"),
                );
            }
            None => {
                self.error(*span, format!("`{name}` is not bound"));
            }
        }
    }

    fn infer(&mut self, expr: &Expr) -> Type {
        match expr {
            Expr::Int { .. } => Type::Int,
            Expr::Ratio { .. } => Type::Ratio,
            Expr::List { items, .. } => {
                for item in items {
                    if self.infer(item) == Type::List {
                        self.error(item.span(), "lists do not nest");
                    }
                }
                Type::List
            }
            Expr::Var { name, span } => match self.index.get(name) {
                Some(&i) => {
                    self.bindings[i].uses.push((*span, false));
                    self.bindings[i].ty
                }
                None => self.error(*span, format!("`{name}` is not bound")),
            },
            Expr::Contract { left, right, span } => {
                let (l, r) = (self.infer(left), self.infer(right));
                match (l, r) {
                    (Type::Word, Type::Word) => Type::Pair,
                    (Type::Error, _) | (_, Type::Error) => Type::Error,
                    _ => self.error(
                        *span,
                        format!("`<~>` contracts two Words; found {l} and {r}"),
                    ),
                }
            }
            Expr::Call {
                callee,
                callee_span,
                args,
                ..
            } => self.call(callee, *callee_span, args),
        }
    }

    fn call(&mut self, callee: &str, callee_span: Span, args: &[Expr]) -> Type {
        let Some(builtin) = builtin(callee) else {
            for arg in args {
                self.infer(arg);
            }
            let known: Vec<&str> = BUILTINS.iter().map(|b| b.name).collect();
            return self.error(
                callee_span,
                format!(
                    "unknown function `{callee}`; the builtins are {}",
                    known.join(", ")
                ),
            );
        };

        if args.len() != builtin.params.len() {
            for arg in args {
                self.infer(arg);
            }
            let wanted: Vec<&str> = builtin.params.iter().map(Param::describe).collect();
            return self.error(
                callee_span,
                format!(
                    "`{callee}` takes {} argument{} ({}), found {}",
                    builtin.params.len(),
                    if builtin.params.len() == 1 { "" } else { "s" },
                    if wanted.is_empty() {
                        "none".to_string()
                    } else {
                        wanted.join("; ")
                    },
                    args.len()
                ),
            );
        }

        let mut ok = true;
        for (arg, param) in args.iter().zip(builtin.params) {
            ok &= self.argument(callee, arg, *param);
        }
        if ok {
            builtin.returns
        } else {
            Type::Error
        }
    }

    /// Check one argument against its parameter. `false` if anything failed.
    fn argument(&mut self, callee: &str, arg: &Expr, param: Param) -> bool {
        if param == Param::Samples {
            let Expr::List { items, span } = arg else {
                self.infer(arg);
                self.error(arg.span(), format!("`{callee}` wants {}", param.describe()));
                return false;
            };
            let mut ok = true;
            if items.len() > WORD_NODES {
                self.error(
                    *span,
                    format!(
                        "`{callee}` packs one {WORD_NODES}-node word; this list has {} samples",
                        items.len()
                    ),
                );
                ok = false;
            }
            for item in items {
                match self.infer(item) {
                    Type::Int => {}
                    Type::Error => ok = false,
                    other => {
                        self.error(item.span(), format!("samples must be Ints, found {other}"));
                        ok = false;
                    }
                }
            }
            return ok;
        }

        let ty = self.infer(arg);
        let accepted = match param {
            Param::Int => ty == Type::Int,
            Param::Scale => matches!(ty, Type::Int | Type::Ratio),
            Param::Pair => ty == Type::Pair,
            Param::Samples => unreachable!("handled above"),
        };
        match (accepted, ty) {
            (true, _) => true,
            (false, Type::Error) => false,
            (false, Type::List) => {
                self.error(
                    arg.span(),
                    format!("`{callee}` wants {} here, not a list", param.describe()),
                );
                false
            }
            (false, other) => {
                self.error(
                    arg.span(),
                    format!("`{callee}` wants {}, found {other}", param.describe()),
                );
                false
            }
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

    fn only(source: &str) -> Diagnostic {
        let mut found = problems(source);
        assert_eq!(found.len(), 1, "{found:#?}");
        found.remove(0)
    }

    #[test]
    fn a_consumed_linear_value_is_fine() {
        let source = "linear let v = new_vault(); consume(v);";
        assert!(check(&parse(source).unwrap()).is_ok());
    }

    #[test]
    fn an_unconsumed_linear_value_is_an_error() {
        let found = only("linear let v = new_vault();");
        assert!(
            found.message.contains("never consumed"),
            "{}",
            found.message
        );
    }

    #[test]
    fn consuming_twice_is_an_error_that_names_both_sites() {
        let found = only("linear let v = new_vault(); consume(v); consume(v);");
        assert!(found.message.contains("exactly once"), "{}", found.message);
        assert_eq!(found.notes.len(), 2, "both uses should be listed");
    }

    #[test]
    fn using_a_linear_value_outside_consume_is_an_error() {
        let found = only("linear let v = new_vault(); print(v);");
        assert!(
            found.message.contains("without being consumed"),
            "{}",
            found.message
        );
    }

    #[test]
    fn ordinary_bindings_are_unrestricted() {
        assert!(check(&parse("let a = 1; print(a); print(a);").unwrap()).is_ok());
    }

    #[test]
    fn rebinding_a_name_in_one_scope_is_an_error() {
        let found = only("let a = 1; let a = 2;");
        assert!(found.message.contains("already bound"), "{}", found.message);
    }

    #[test]
    fn an_unbound_name_is_an_error_not_unit() {
        let found = only("print(ghost);");
        assert!(
            found.message.contains("`ghost` is not bound"),
            "{}",
            found.message
        );
        assert_eq!(found.span.column, 7);
    }

    #[test]
    fn use_before_let_is_unbound() {
        let found = only("print(a); let a = 1;");
        assert!(found.message.contains("not bound"), "{}", found.message);
    }

    #[test]
    fn an_unknown_function_is_an_error_that_lists_the_real_ones() {
        let found = only("print(pakc([1], 1));");
        assert!(
            found.message.contains("unknown function `pakc`"),
            "{}",
            found.message
        );
        assert!(found.message.contains("pack"), "{}", found.message);
    }

    #[test]
    fn arity_is_checked() {
        let found = only("let w = pack([1, 2]);");
        assert!(
            found.message.contains("takes 2 arguments"),
            "{}",
            found.message
        );
        let found = only("let v = new_vault(1);");
        assert!(
            found.message.contains("takes 0 arguments (none)"),
            "{}",
            found.message
        );
    }

    #[test]
    fn contraction_needs_two_words() {
        let found = only("let w = pack([1], 1); print(w <~> 3);");
        assert!(
            found.message.contains("found Word and Int"),
            "{}",
            found.message
        );
    }

    #[test]
    fn a_contraction_result_cannot_be_contracted_again() {
        let found = only("let a = pack([1], 1); print(a <~> a <~> a);");
        assert!(
            found.message.contains("found Pair and Word"),
            "{}",
            found.message
        );
    }

    #[test]
    fn samples_must_be_a_literal_list_of_ints_no_longer_than_a_word() {
        let found = only("let w = pack([1, 1/2], 1);");
        assert!(
            found.message.contains("samples must be Ints, found Ratio"),
            "{}",
            found.message
        );

        let long = format!("let w = pack([{}], 1);", vec!["1"; 65].join(", "));
        let found = only(&long);
        assert!(
            found.message.contains("this list has 65 samples"),
            "{}",
            found.message
        );

        let found = only("let n = 3; let w = pack(n, 1);");
        assert!(
            found.message.contains("a list of at most 64 Ints"),
            "{}",
            found.message
        );
    }

    #[test]
    fn a_scale_may_be_an_int_or_a_ratio_but_nothing_else() {
        assert!(check(&parse("let w = pack([1], 1/3);").unwrap()).is_ok());
        assert!(check(&parse("let w = pack([1], 2);").unwrap()).is_ok());
        let found = only("let v = new_vault(); let w = pack([1], v);");
        assert!(
            found.message.contains("an Int or a Ratio, found Vault"),
            "{}",
            found.message
        );
    }

    #[test]
    fn lists_cannot_be_bound_or_printed() {
        let found = only("let l = [1, 2];");
        assert!(
            found.message.contains("builtin argument"),
            "{}",
            found.message
        );
        let found = only("print([1]);");
        assert!(
            found.message.contains("builtin argument"),
            "{}",
            found.message
        );
    }

    #[test]
    fn consume_takes_a_linear_name() {
        let found = only("let v = new_vault(); consume(v);");
        assert!(found.message.contains("is not linear"), "{}", found.message);
        let found = only("consume(new_vault());");
        assert!(found.message.contains("by name"), "{}", found.message);
    }

    #[test]
    fn one_mistake_is_reported_once() {
        // `ghost` is unbound; the contraction and the call must not pile on.
        let found = only("let w = pack([1], 1); print(re(w <~> ghost));");
        assert!(found.message.contains("ghost"), "{}", found.message);
    }

    #[test]
    fn problems_come_back_in_source_order() {
        let found = problems("linear let v = new_vault();\nprint(nope);\nprint(nada);");
        let lines: Vec<usize> = found.iter().map(|d| d.span.line).collect();
        assert_eq!(lines, vec![1, 2, 3]);
    }

    #[test]
    fn the_checked_program_remembers_types() {
        let checked =
            check(&parse("let w = pack([1], 1); let p = w <~> w; let r = re(p);").unwrap())
                .unwrap();
        assert_eq!(checked.type_of("w"), Some(Type::Word));
        assert_eq!(checked.type_of("p"), Some(Type::Pair));
        assert_eq!(checked.type_of("r"), Some(Type::Ratio));
    }
}
