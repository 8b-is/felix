//! Recursive-descent parser.
//!
//! Grammar, in full:
//!
//! ```text
//! program    := statement* EOF
//! statement  := "linear"? "let" IDENT "=" expr ";"
//!             | "consume" "(" expr ")" ";"
//!             | "print"   "(" expr ")" ";"
//! expr       := primary ( "<~>" primary )*
//! primary    := INT | IDENT | "[" [ expr ("," expr)* ] "]"
//!             | IDENT "(" [ expr ("," expr)* ] ")"
//! ```
//!
//! Note what is *not* here: assignment, mutation, loops, functions, modules.
//! A grammar that fits on a postcard is one a second implementation can match.

use crate::ast::{Expr, Program, Stmt};
use crate::diag::{Diagnostic, Span};
use crate::lexer::{scan, Spanned, Token};

pub fn parse(source: &str) -> Result<Program, Diagnostic> {
    let tokens = scan(source)?;
    let mut parser = Parser { tokens, cursor: 0 };
    parser.program()
}

struct Parser {
    tokens: Vec<Spanned>,
    cursor: usize,
}

impl Parser {
    fn peek(&self) -> &Spanned {
        &self.tokens[self.cursor.min(self.tokens.len() - 1)]
    }

    fn bump(&mut self) -> Spanned {
        let token = self.peek().clone();
        if self.cursor < self.tokens.len() - 1 {
            self.cursor += 1;
        }
        token
    }

    fn expect(&mut self, want: Token) -> Result<Spanned, Diagnostic> {
        let found = self.peek().clone();
        if found.token == want {
            Ok(self.bump())
        } else {
            Err(Diagnostic::error(
                found.span,
                format!(
                    "expected {}, found {}",
                    want.describe(),
                    found.token.describe()
                ),
            ))
        }
    }

    fn program(&mut self) -> Result<Program, Diagnostic> {
        let mut statements = Vec::new();
        while self.peek().token != Token::Eof {
            statements.push(self.statement()?);
        }
        Ok(Program { statements })
    }

    fn statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.peek().span;

        if self.peek().token == Token::Linear {
            self.bump();
            let keyword = self.expect(Token::Let)?;
            return self.let_rest(start.merge(keyword.span), true);
        }

        match self.peek().token.clone() {
            Token::Let => {
                let keyword = self.bump();
                self.let_rest(keyword.span, false)
            }
            Token::Consume => {
                let keyword = self.bump();
                self.expect(Token::LParen)?;
                let value = self.expr()?;
                let close = self.expect(Token::RParen)?;
                self.expect(Token::Semi)?;
                Ok(Stmt::Consume {
                    value,
                    span: keyword.span.merge(close.span),
                })
            }
            Token::Print => {
                let keyword = self.bump();
                self.expect(Token::LParen)?;
                let value = self.expr()?;
                let close = self.expect(Token::RParen)?;
                self.expect(Token::Semi)?;
                Ok(Stmt::Print {
                    value,
                    span: keyword.span.merge(close.span),
                })
            }
            other => Err(Diagnostic::error(
                start,
                format!(
                    "expected a statement (`let`, `linear let`, `consume`, `print`), found {}",
                    other.describe()
                ),
            )),
        }
    }

    /// Any identifier. Written by hand because `expect` compares tokens for
    /// equality, and no placeholder `Ident` can equal the one in the source.
    fn ident(&mut self, what: &str) -> Result<(String, Span), Diagnostic> {
        let token = self.bump();
        match token.token {
            Token::Ident(name) => Ok((name, token.span)),
            other => Err(Diagnostic::error(
                token.span,
                format!("expected {what}, found {}", other.describe()),
            )),
        }
    }

    fn let_rest(&mut self, start: Span, linear: bool) -> Result<Stmt, Diagnostic> {
        let (name, name_span) = self.ident("a name")?;
        self.expect(Token::Assign)?;
        let value = self.expr()?;
        let semi = self.expect(Token::Semi)?;
        Ok(Stmt::Let {
            name,
            name_span,
            linear,
            value,
            span: start.merge(semi.span),
        })
    }

    fn expr(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.primary()?;
        while self.peek().token == Token::Contract {
            self.bump();
            let right = self.primary()?;
            let span = left.span().merge(right.span());
            left = Expr::Contract {
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn primary(&mut self) -> Result<Expr, Diagnostic> {
        let token = self.bump();
        match token.token {
            Token::Int(value) => Ok(Expr::Int(value)),
            Token::LBracket => {
                let mut items = Vec::new();
                if self.peek().token != Token::RBracket {
                    loop {
                        items.push(self.expr()?);
                        if self.peek().token == Token::Comma {
                            self.bump();
                            continue;
                        }
                        break;
                    }
                }
                let close = self.expect(Token::RBracket)?;
                Ok(Expr::List {
                    items,
                    span: token.span.merge(close.span),
                })
            }
            Token::Ident(name) => {
                if self.peek().token == Token::LParen {
                    let open = self.bump();
                    let mut args = Vec::new();
                    if self.peek().token != Token::RParen {
                        loop {
                            args.push(self.expr()?);
                            if self.peek().token == Token::Comma {
                                self.bump();
                                continue;
                            }
                            break;
                        }
                    }
                    let close = self.expect(Token::RParen)?;
                    Ok(Expr::Call {
                        callee: name,
                        callee_span: token.span,
                        args,
                        span: token.span.merge(close.span).merge(open.span),
                    })
                } else {
                    Ok(Expr::Var {
                        name,
                        span: token.span,
                    })
                }
            }
            other => Err(Diagnostic::error(
                token.span,
                format!("expected an expression, found {}", other.describe()),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_let_and_a_print() {
        let program = parse("let a = 1; print(a);").unwrap();
        assert_eq!(program.statements.len(), 2);
        assert!(matches!(
            program.statements[0],
            Stmt::Let { linear: false, .. }
        ));
    }

    #[test]
    fn linear_let_is_distinguished() {
        let program = parse("linear let v = new_vault();").unwrap();
        match &program.statements[0] {
            Stmt::Let { linear, name, .. } => {
                assert!(*linear);
                assert_eq!(name, "v");
            }
            other => panic!("expected a let, got {other:?}"),
        }
    }

    #[test]
    fn calls_nest_and_take_lists() {
        let program = parse("let w = pack([1, -2], 3);").unwrap();
        match &program.statements[0] {
            Stmt::Let { value, .. } => match value {
                Expr::Call { callee, args, .. } => {
                    assert_eq!(callee, "pack");
                    assert_eq!(args.len(), 2);
                }
                other => panic!("expected a call, got {other:?}"),
            },
            other => panic!("expected a let, got {other:?}"),
        }
    }

    #[test]
    fn contraction_is_left_associative() {
        let program = parse("let d = a <~> b <~> c;").unwrap();
        match &program.statements[0] {
            Stmt::Let { value, .. } => match value {
                Expr::Contract { left, .. } => {
                    assert!(matches!(**left, Expr::Contract { .. }), "left-nested");
                }
                other => panic!("expected a contract, got {other:?}"),
            },
            other => panic!("expected a let, got {other:?}"),
        }
    }

    #[test]
    fn a_missing_semicolon_points_at_the_problem() {
        let err = parse("let a = 1").unwrap_err();
        assert!(err.message.contains("`;`"), "{}", err.message);
    }

    #[test]
    fn a_stray_token_after_the_statement_is_an_error() {
        let err = parse("let a = 1; 2;").unwrap_err();
        assert!(err.message.contains("statement"), "{}", err.message);
    }

    #[test]
    fn empty_input_is_a_valid_empty_program() {
        assert!(parse("").unwrap().statements.is_empty());
    }
}
