//! Felix v0.1 — a small language with two ideas that can be proved.
//!
//! The scaffold this grew out of proposed a compiler, a VM, an LSP, an MCP
//! server, four stdlib modules and a triple-forge push, in one pass. That is a
//! specification wearing a repository's clothes: none of those pieces can be
//! called real until something pins their semantics.
//!
//! So this is the spine instead, and only what a test can hold:
//!
//! 1. **Linear values.** A `linear` binding must be consumed exactly once.
//!    Using it twice, or letting it fall out of scope, is a compile error with
//!    both uses named. This is the whole reason a memory-owning runtime is
//!    worth typing; it is checkable in ~80 lines and it is checked here.
//! 2. **One rendering of the ternary datapath.** `pack(...)` and `<~>` are not
//!    reimplemented from prose. They call the `memnet` reference oracle, and the
//!    differential tests fail the build if Felix and the oracle ever disagree by
//!    one bit.
//!
//! What is deliberately absent, and stays absent until a test demands it: a VM,
//! an LSP, an MCP surface, `mem8`/`engram`/`entheai` stdlib modules, and any
//! claim that a language runtime is a mind. Those are separate slices with their
//! own acceptance tests.

pub mod ast;
pub mod check;
pub mod codegen_v2;
pub mod diag;
pub mod eval;
pub mod lexer;
pub mod parser;
pub mod typeck;

pub use ast::{Expr, Program, Stmt};
pub use diag::{Diagnostic, Span};
pub use eval::{Value, VaultId};
pub use lexer::Token;

/// Parse, check, and run a program. The one entry point most callers want.
pub fn run(source: &str) -> Result<Vec<(String, Value)>, Vec<Diagnostic>> {
    let program = parser::parse(source).map_err(|d| vec![d])?;
    check::check(&program)?;
    Ok(eval::run(&program))
}
