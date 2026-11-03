//! Tokens and the scanner.
//!
//! The alphabet is small on purpose: `let`, `linear`, `print`, `consume`,
//! integers (with `-`), exact ratios `p/q`, identifiers, and the two operators
//! that matter — `<~>` for contraction and `,`/`(`/`)`/`[`/`]` for grouping.
//! Anything else is a diagnostic, not a silent skip.

use crate::diag::{Diagnostic, Span};
use crate::ratio::Ratio;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Let,
    Linear,
    Print,
    Consume,
    Ident(String),
    Int(i64),
    /// `p/q`, already reduced.
    Ratio(Ratio),
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
            Token::Ratio(value) => format!("ratio `{value}`"),
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
            c if c == '-' || c.is_ascii_digit() => {
                let negative = c == '-';
                let (numer, mut j) = digits(&bytes, if negative { i + 1 } else { i });
                let Some(numer) = numer else {
                    return Err(Diagnostic::error(start, "`-` must be followed by digits"));
                };
                let numer = if negative { -numer } else { numer };
                let to_i64 = |value: i128, end: usize| {
                    i64::try_from(value).map_err(|_| {
                        Diagnostic::error(
                            Span::new(i, end, line, column),
                            "integer literal does not fit in 64 bits",
                        )
                    })
                };

                // `p/q`, written without spaces. `//` is still a comment.
                let token = if bytes.get(j) == Some(&'/')
                    && bytes.get(j + 1).is_some_and(|d| d.is_ascii_digit())
                {
                    let (denom, end) = digits(&bytes, j + 1);
                    j = end;
                    let (p, q) = (to_i64(numer, j)?, to_i64(denom.unwrap_or(0), j)?);
                    match Ratio::new(p, q) {
                        Some(ratio) => Token::Ratio(ratio),
                        None => {
                            return Err(Diagnostic::error(
                                Span::new(i, j, line, column),
                                "a ratio needs a positive denominator, like `3/4`",
                            ))
                        }
                    }
                } else {
                    Token::Int(to_i64(numer, j)?)
                };
                push(&mut tokens, token, Span::new(i, j, line, column));
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
            '/' => {
                return Err(Diagnostic::error(
                    start,
                    "a ratio is written without spaces, like `3/4`; `//` starts a comment",
                ));
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

/// Decimal digits from `from`, accumulated in `i128` so that one digit past
/// `i64` is a diagnostic rather than a debug-build panic. `None` if there are
/// no digits at all. Saturates far outside `i64`, which still fails to narrow.
fn digits(bytes: &[char], from: usize) -> (Option<i128>, usize) {
    let mut j = from;
    let mut value = 0i128;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        value = value
            .saturating_mul(10)
            .saturating_add((bytes[j] as u8 - b'0') as i128);
        j += 1;
    }
    ((j > from).then_some(value), j)
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

    #[test]
    fn ratios_are_one_reduced_token() {
        assert_eq!(
            kinds("6/4 -1/3"),
            vec![
                Token::Ratio(Ratio::new(3, 2).unwrap()),
                Token::Ratio(Ratio::new(-1, 3).unwrap()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn a_ratio_with_a_zero_denominator_is_an_error() {
        let err = scan("let g = 3/0;").unwrap_err();
        assert!(
            err.message.contains("positive denominator"),
            "{}",
            err.message
        );
        assert_eq!(err.span.column, 9);
    }

    #[test]
    fn a_spaced_slash_is_named_not_skipped() {
        let err = scan("let g = 3 / 4;").unwrap_err();
        assert!(err.message.contains("without spaces"), "{}", err.message);
    }

    #[test]
    fn a_comment_right_after_a_number_is_still_a_comment() {
        assert_eq!(kinds("7// seven"), vec![Token::Int(7), Token::Eof]);
    }

    #[test]
    fn literals_outside_i64_are_diagnostics_not_panics() {
        assert_eq!(
            kinds("-9223372036854775808"),
            vec![Token::Int(i64::MIN), Token::Eof]
        );
        let err = scan("9223372036854775808").unwrap_err();
        assert!(err.message.contains("64 bits"), "{}", err.message);
        assert!(scan("99999999999999999999999999999999999999999").is_err());
    }
}
