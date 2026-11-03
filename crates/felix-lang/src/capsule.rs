//! Capsules: a program, the exact transcript it produces, and digests that
//! tie the two together.
//!
//! A capsule is the conformance unit for every other implementation of Felix.
//! The README says this crate is "the reference the others get diffed
//! against"; a capsule is the diff, written down. `felix verify` re-runs the
//! embedded source and refuses the capsule unless the new transcript is the
//! sealed one, byte for byte.
//!
//! ```text
//! felix-capsule/1
//! id: <first 32 hex of sha256(header + source digest + output digest)>
//! source-sha256: <64 hex>
//! output-sha256: <64 hex>
//! source-bytes: <n>
//! output-bytes: <m>
//! --- source
//! <exactly n bytes>
//! --- output
//! <exactly m bytes: one rendered value per line>
//! ```
//!
//! Sections are length-prefixed rather than delimiter-terminated, so no
//! source text can forge a section boundary.
//!
//! Honest scope: the digest is SHA-256 from `memnet` (zero dependencies), not
//! BLAKE3 as `g8t` capsules use. A capsule proves *integrity and
//! reproducibility*. It is not a signature and says nothing about who made it.

use std::fmt;

use crate::diag::Diagnostic;

pub const MAGIC: &str = "felix-capsule/1";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex(&memnet::sha256(bytes))
}

fn capsule_id(source_digest: &str, output_digest: &str) -> String {
    let material = format!("{MAGIC}\n{source_digest}\n{output_digest}\n");
    sha256_hex(material.as_bytes())[..32].to_string()
}

/// The transcript as the CLI prints it: each rendering on its own line.
pub fn transcript(lines: &[String]) -> String {
    lines.iter().map(|line| format!("{line}\n")).collect()
}

/// Parse, check, run, and seal. Fails with the program's own diagnostics.
pub fn seal(source: &str) -> Result<String, Vec<Diagnostic>> {
    let outputs = crate::run(source)?;
    let lines: Vec<String> = outputs.into_iter().map(|(text, _)| text).collect();
    Ok(seal_transcript(source, &transcript(&lines)))
}

fn seal_transcript(source: &str, output: &str) -> String {
    let (sd, od) = (sha256_hex(source.as_bytes()), sha256_hex(output.as_bytes()));
    format!(
        "{MAGIC}\nid: {}\nsource-sha256: {sd}\noutput-sha256: {od}\nsource-bytes: {}\noutput-bytes: {}\n--- source\n{source}\n--- output\n{output}",
        capsule_id(&sd, &od),
        source.len(),
        output.len(),
    )
}

/// A capsule that has been read but not yet re-run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capsule {
    pub id: String,
    pub source_sha256: String,
    pub output_sha256: String,
    pub source: String,
    pub output: String,
}

/// Why a capsule was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    /// The text is not a well-formed `felix-capsule/1`.
    Malformed(String),
    /// A recorded digest or id does not match the bytes it covers.
    Tampered(&'static str),
    /// The embedded source no longer parses, checks, or runs.
    Rejected(Vec<Diagnostic>),
    /// The source runs, but not to the sealed transcript.
    Diverged {
        line: usize,
        sealed: Option<String>,
        now: Option<String>,
    },
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerifyError::Malformed(why) => write!(f, "malformed capsule: {why}"),
            VerifyError::Tampered(what) => write!(f, "tampered capsule: {what} does not match"),
            VerifyError::Rejected(problems) => {
                write!(f, "the sealed source no longer runs:")?;
                for problem in problems {
                    write!(f, "\n  {problem}")?;
                }
                Ok(())
            }
            VerifyError::Diverged { line, sealed, now } => write!(
                f,
                "diverged at output line {line}: sealed {}, now {}",
                sealed
                    .as_deref()
                    .map_or("<end>".into(), |s| format!("`{s}`")),
                now.as_deref().map_or("<end>".into(), |s| format!("`{s}`")),
            ),
        }
    }
}

impl std::error::Error for VerifyError {}

/// Read a capsule and check its internal digests. Does not run anything.
pub fn parse(text: &str) -> Result<Capsule, VerifyError> {
    let malformed = |why: &str| VerifyError::Malformed(why.to_string());
    let mut rest = text;
    let mut take_line = |want: &str| -> Result<String, VerifyError> {
        let (line, tail) = rest
            .split_once('\n')
            .ok_or_else(|| malformed(&format!("missing `{want}` line")))?;
        rest = tail;
        Ok(line.to_string())
    };

    if take_line("magic")? != MAGIC {
        return Err(malformed("first line is not `felix-capsule/1`"));
    }
    let mut field = |key: &str| -> Result<String, VerifyError> {
        let line = take_line(key)?;
        line.strip_prefix(&format!("{key}: "))
            .map(str::to_string)
            .ok_or_else(|| malformed(&format!("expected `{key}: ...`, found `{line}`")))
    };
    let id = field("id")?;
    let source_sha256 = field("source-sha256")?;
    let output_sha256 = field("output-sha256")?;
    let count = |s: String, key: &str| {
        s.parse::<usize>()
            .map_err(|_| malformed(&format!("`{key}` is not a byte count")))
    };
    let source_bytes = count(field("source-bytes")?, "source-bytes")?;
    let output_bytes = count(field("output-bytes")?, "output-bytes")?;
    drop(field);

    let mut cut = |marker: &str, len: usize| -> Result<String, VerifyError> {
        let body = rest
            .strip_prefix(marker)
            .ok_or_else(|| malformed(&format!("expected `{}`", marker.trim())))?;
        let section = body
            .get(..len)
            .ok_or_else(|| malformed("a section is shorter than its byte count"))?;
        rest = &body[len..];
        Ok(section.to_string())
    };
    let source = cut("--- source\n", source_bytes)?;
    let output = cut("\n--- output\n", output_bytes)?;
    if !rest.is_empty() {
        return Err(malformed("trailing bytes after the output section"));
    }

    if sha256_hex(source.as_bytes()) != source_sha256 {
        return Err(VerifyError::Tampered("source-sha256"));
    }
    if sha256_hex(output.as_bytes()) != output_sha256 {
        return Err(VerifyError::Tampered("output-sha256"));
    }
    if capsule_id(&source_sha256, &output_sha256) != id {
        return Err(VerifyError::Tampered("id"));
    }
    Ok(Capsule {
        id,
        source_sha256,
        output_sha256,
        source,
        output,
    })
}

/// Parse a capsule, re-run its source, and demand the sealed transcript.
pub fn verify(text: &str) -> Result<Capsule, VerifyError> {
    let capsule = parse(text)?;
    let outputs = crate::run(&capsule.source).map_err(VerifyError::Rejected)?;
    let now: Vec<String> = outputs.into_iter().map(|(text, _)| text).collect();
    let sealed: Vec<&str> = capsule.output.lines().collect();

    for line in 0..sealed.len().max(now.len()) {
        let (s, n) = (sealed.get(line).copied(), now.get(line).map(String::as_str));
        if s != n {
            return Err(VerifyError::Diverged {
                line: line + 1,
                sealed: s.map(str::to_string),
                now: n.map(str::to_string),
            });
        }
    }
    // Line-equal but byte-different (e.g. a missing final newline) is still a
    // divergence: the digest covers bytes, so the comparison does too.
    if transcript(&now) != capsule.output {
        return Err(VerifyError::Tampered("output framing"));
    }
    Ok(capsule)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROGRAM: &str = "let a = pack([1, 1], 1/2);\nprint(a <~> a);\nprint(re(a <~> a));\n";

    #[test]
    fn a_sealed_capsule_verifies() {
        let text = seal(PROGRAM).unwrap();
        let capsule = verify(&text).unwrap();
        assert_eq!(capsule.source, PROGRAM);
        assert_eq!(capsule.output, "(2, 0)\n1/2\n");
        assert_eq!(capsule.id.len(), 32);
    }

    #[test]
    fn sealing_is_deterministic() {
        assert_eq!(seal(PROGRAM).unwrap(), seal(PROGRAM).unwrap());
    }

    #[test]
    fn a_program_that_does_not_check_cannot_be_sealed() {
        assert!(seal("linear let v = new_vault();").is_err());
    }

    #[test]
    fn editing_the_source_is_caught() {
        let text = seal(PROGRAM).unwrap().replace("1/2);", "1/3);");
        assert_eq!(verify(&text), Err(VerifyError::Tampered("source-sha256")));
    }

    #[test]
    fn editing_the_output_is_caught() {
        let text = seal(PROGRAM).unwrap().replace("(2, 0)", "(3, 0)");
        assert_eq!(verify(&text), Err(VerifyError::Tampered("output-sha256")));
    }

    #[test]
    fn a_consistent_forgery_is_caught_by_rerunning() {
        // Re-seal a lying transcript with correct digests: only the re-run
        // can tell, and it does.
        let forged = seal_transcript(PROGRAM, "(2, 0)\n1/3\n");
        assert!(parse(&forged).is_ok(), "digests are self-consistent");
        assert_eq!(
            verify(&forged),
            Err(VerifyError::Diverged {
                line: 2,
                sealed: Some("1/3".into()),
                now: Some("1/2".into()),
            })
        );
    }

    #[test]
    fn a_source_containing_a_section_marker_cannot_forge_a_boundary() {
        let tricky = "// --- output\nprint(1);\n";
        let text = seal(tricky).unwrap();
        assert_eq!(verify(&text).unwrap().source, tricky);
    }

    #[test]
    fn garbage_is_malformed_not_a_panic() {
        for text in [
            "",
            "felix-capsule/1\n",
            "nope\n",
            "felix-capsule/1\nid: x\n",
        ] {
            assert!(
                matches!(parse(text), Err(VerifyError::Malformed(_))),
                "{text:?}"
            );
        }
        let truncated = seal(PROGRAM).unwrap();
        let truncated = &truncated[..truncated.len() - 3];
        assert!(matches!(parse(truncated), Err(VerifyError::Malformed(_))));
    }
}
