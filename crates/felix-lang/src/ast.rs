//! The syntax tree. Small enough to read in one sitting, which is the point.

use crate::diag::Span;

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

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    /// `[a, b, c]`
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
            Expr::Int(_) => Span::default(),
            Expr::List { span, .. } => *span,
            Expr::Var { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::Contract { span, .. } => *span,
        }
    }
}
