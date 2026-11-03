//! Spans and diagnostics. Every error names a line, a column, and what was
//! expected, because a language that cannot point at the mistake is a puzzle.

use std::fmt;

/// A byte range in the source, plus its 1-based line and column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Self {
            start,
            end,
            line,
            column,
        }
    }

    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            line: self.line,
            column: self.column,
        }
    }

    /// `12:5`
    pub fn site(&self) -> String {
        format!("{}:{}", self.line, self.column)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
    /// Extra locations that make the message actionable (e.g. both uses).
    pub notes: Vec<(Span, String)>,
}

impl Diagnostic {
    pub fn error(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            notes: Vec::new(),
        }
    }

    pub fn with_note(mut self, span: Span, note: impl Into<String>) -> Self {
        self.notes.push((span, note.into()));
        self
    }

    pub fn render(&self, source: &str) -> String {
        let mut out = format!("error at {}: {}", self.span.site(), self.message);
        for (index, text) in source.lines().enumerate() {
            let line_no = index + 1;
            if line_no == self.span.line {
                out.push_str(&format!("\n  {line_no:>4} | {text}"));
                out.push_str(&format!(
                    "\n       | {}^",
                    " ".repeat(self.span.column.saturating_sub(1))
                ));
            }
        }
        for (span, note) in &self.notes {
            out.push_str(&format!("\n  note at {}: {note}", span.site()));
        }
        out
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.span.site(), self.message)
    }
}

impl std::error::Error for Diagnostic {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gutter_shows_the_one_based_line_number() {
        let source = "let a = 1;\nprint(b);\n";
        let rendered = Diagnostic::error(Span::new(17, 18, 2, 7), "nope").render(source);
        assert!(rendered.contains("     2 | print(b);"), "{rendered}");
        assert!(rendered.contains("       |       ^"), "{rendered}");
    }
}
