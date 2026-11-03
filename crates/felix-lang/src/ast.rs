//! The syntax tree. Small enough to read in one sitting, which is the point.

use crate::diag::Span;
use crate::ratio::Ratio;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// `let name = expr;` or `linear let name = expr;`
    Let {
        name: String,
        name_span: Span,
        linear: bool,
        value: Expr,
        span: Span,
    },
    /// `consume(expr);` — retires a linear value.
    Consume { value: Expr, span: Span },
    /// `print(expr);`
    Print { value: Expr, span: Span },
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. } | Stmt::Consume { span, .. } | Stmt::Print { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int {
        value: i64,
        span: Span,
    },
    /// `p/q`, reduced at scan time.
    Ratio {
        value: Ratio,
        span: Span,
    },
    /// `[a, b, c]` — only meaningful as a builtin argument; the checker says so.
    List {
        items: Vec<Expr>,
        span: Span,
    },
    Var {
        name: String,
        span: Span,
    },
    Call {
        callee: String,
        callee_span: Span,
        args: Vec<Expr>,
        span: Span,
    },
    /// `a <~> b`
    Contract {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Int { span, .. }
            | Expr::Ratio { span, .. }
            | Expr::List { span, .. }
            | Expr::Var { span, .. }
            | Expr::Call { span, .. }
            | Expr::Contract { span, .. } => *span,
        }
    }

    /// One-line source form, used by `felix graph` labels. Lists longer than
    /// four items are elided with their length so a node stays readable.
    pub fn summary(&self) -> String {
        match self {
            Expr::Int { value, .. } => value.to_string(),
            Expr::Ratio { value, .. } => value.to_string(),
            Expr::Var { name, .. } => name.clone(),
            Expr::List { items, .. } if items.len() > 4 => format!("[…{}]", items.len()),
            Expr::List { items, .. } => format!(
                "[{}]",
                items
                    .iter()
                    .map(Expr::summary)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Expr::Call { callee, args, .. } => format!(
                "{callee}({})",
                args.iter()
                    .map(Expr::summary)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Expr::Contract { left, right, .. } => {
                format!("{} <~> {}", left.summary(), right.summary())
            }
        }
    }
}
