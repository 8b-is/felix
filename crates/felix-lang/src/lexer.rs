//! Tokens and the scanner.
//!
//! The alphabet is small on purpose: `let`, `linear`, `print`, `consume`,
//! integers (with `-`), identifiers, and the two operators that matter —
//! `<~>` for contraction and `,`/`(`/`)`/`[`/`]` for grouping. Anything else is
//! a diagnostic, not a silent skip.

use crate::diag::{Diagnostic, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Let,
    Linear,
    Print,
    Consume,
    Ident(String),
    Int(i64),
    Contract, // <~>
    Assign,
    Comma,
    Semi,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Eof,
}

impl Token {
    /// How a token should read in an error message.
    pub fn describe(&self) -> String {
        match self {
            Token::Let => "`let`".into(),
            Token::Linear => "`linear`".into(),
            Token::Print => "`print`".into(),
            Token::Consume => "`consume`".into(),
            Token::Ident(name) => format!("identifier `{name}`"),
            Token::Int(value) => format!("integer `{value}`"),
            Token::Contract => "`<~>`".into(),
            Token::Assign => "`=`".into(),
            Token::Comma => "`,`".into(),
            Token::Semi => "`;`".into(),
            Token::LParen => "`(`".into(),
            Token::RParen => "`)`".into(),
            Token::LBracket => "`[`".into(),
            Token::RBracket => "`]`".into(),
            Token::Eof => "end of input".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spanned {
    pub token: Token,
    pub span: Span,
}

/// Scan `source` into tokens, always ending with [`Token::Eof`].
pub fn scan(source: &str) -> Result<Vec<Spanned>, Diagnostic> {
    let bytes: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0usize;
    let (mut line, mut column) = (1usize, 1usize);

    while i < bytes.len() {
        let ch = bytes[i];

        if ch == '\n' {
            i += 1;
            line += 1;
            column = 1;
            continue;
        }
        if ch.is_whitespace() {
            i += 1;
            column += 1;
            continue;
        }

        // Line comments: `// ...`
        if ch == '/' && bytes.get(i + 1) == Some(&'/') {
            while i < bytes.len() && bytes[i] != '\n' {
                i += 1;
                column += 1;
            }
            continue;
        }

        let start = Span::new(i, i + 1, line, column);
        let push = |tokens: &mut Vec<Spanned>, token: Token, span: Span| {
            tokens.push(Spanned { token, span })
        };

        match ch {
            '(' => {
                push(&mut tokens, Token::LParen, start);
                i += 1;
                column += 1;
            }
            ')' => {
                push(&mut tokens, Token::RParen, start);
                i += 1;
                column += 1;
            }
            '[' => {
                push(&mut tokens, Token::LBracket, start);
                i += 1;
                column += 1;
            }
            ']' => {
                push(&mut tokens, Token::RBracket, start);
                i += 1;
                column += 1;
            }
            ',' => {
                push(&mut tokens, Token::Comma, start);
                i += 1;
                column += 1;
            }
            ';' => {
                push(&mut tokens, Token::Semi, start);
                i += 1;
                column += 1;
            }
            '=' => {
                push(&mut tokens, Token::Assign, start);
                i += 1;
                column += 1;
            }
            '<' => {
                // `<~>` is the contraction operator and nothing else starts with
                // `<`, so a near-miss is worth naming precisely.
                if bytes.get(i + 1) == Some(&'~') && bytes.get(i + 2) == Some(&'>') {
                    push(
                        &mut tokens,
                        Token::Contract,
                        Span::new(i, i + 3, line, column),
                    );
                    i += 3;
                    column += 3;
                } else {
                    return Err(Diagnostic::error(
                        start,
                        "expected `<~>`, the contraction operator",
                    ));
                }
            }
            '-' => {
                let mut j = i + 1;
                let mut value = 0i64;
                let mut digits = 0usize;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    value = value * 10 + (bytes[j] as u8 - b'0') as i64;
                    j += 1;
                    digits += 1;
                }
                if digits == 0 {
                    return Err(Diagnostic::error(start, "`-` must be followed by digits"));
                }
                push(
                    &mut tokens,
                    Token::Int(-value),
                    Span::new(i, j, line, column),
                );
                column += j - i;
                i = j;
            }
            c if c.is_ascii_digit() => {
                let mut j = i;
                let mut value = 0i64;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    value = value * 10 + (bytes[j] as u8 - b'0') as i64;
                    j += 1;
                }
                push(
                    &mut tokens,
                    Token::Int(value),
                    Span::new(i, j, line, column),
                );
                column += j - i;
                i = j;
            }
            c if c.is_alphabetic() || c == '_' => {
                let mut j = i;
                let mut word = String::new();
                while j < bytes.len() && (bytes[j].is_alphanumeric() || bytes[j] == '_') {
                    word.push(bytes[j]);
                    j += 1;
                }
                let token = match word.as_str() {
                    "let" => Token::Let,
                    "linear" => Token::Linear,
                    "print" => Token::Print,
                    "consume" => Token::Consume,
                    _ => Token::Ident(word),
                };
                push(&mut tokens, token, Span::new(i, j, line, column));
                column += j - i;
                i = j;
            }
            other => {
                return Err(Diagnostic::error(
                    start,
                    format!("unexpected character `{other}`"),
                ));
            }
        }
    }

    tokens.push(Spanned {
        token: Token::Eof,
        span: Span::new(bytes.len(), bytes.len(), line, column),
    });
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<Token> {
        scan(source).unwrap().into_iter().map(|s| s.token).collect()
    }

    #[test]
    fn scans_a_small_program() {
        assert_eq!(
            kinds("linear let v = new_vault(); consume(v);"),
            vec![
                Token::Linear,
                Token::Let,
                Token::Ident("v".into()),
                Token::Assign,
                Token::Ident("new_vault".into()),
                Token::LParen,
                Token::RParen,
                Token::Semi,
                Token::Consume,
                Token::LParen,
                Token::Ident("v".into()),
                Token::RParen,
                Token::Semi,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn negative_integers_are_one_token() {
        assert_eq!(
            kinds("pack([-3, 0, 4], 2)"),
            vec![
                Token::Ident("pack".into()),
                Token::LParen,
                Token::LBracket,
                Token::Int(-3),
                Token::Comma,
                Token::Int(0),
                Token::Comma,
                Token::Int(4),
                Token::RBracket,
                Token::Comma,
                Token::Int(2),
                Token::RParen,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn comments_are_skipped_and_lines_are_counted() {
        let tokens = scan("// header\nlet a = 1;\n// trailing\n").unwrap();
        assert_eq!(tokens[0].token, Token::Let);
        assert_eq!(tokens[0].span.line, 2);
        assert_eq!(tokens.last().unwrap().token, Token::Eof);
    }

    #[test]
    fn a_near_miss_operator_names_what_it_wanted() {
        let err = scan("a <~ b").unwrap_err();
        assert!(err.message.contains("<~>"), "{}", err.message);
        assert_eq!(err.span.column, 3);
    }

    #[test]
    fn a_stray_character_is_reported_with_its_column() {
        let err = scan("let a = 1 # 2").unwrap_err();
        assert!(err.message.contains('#'), "{}", err.message);
        assert_eq!(err.span.column, 11);
    }

    #[test]
    fn a_bare_minus_is_an_error_not_a_zero() {
        assert!(scan("let a = - ;").is_err());
    }
}
