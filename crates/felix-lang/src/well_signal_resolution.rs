//! `felix/crates/felix-lang/src/well_signal_resolution.rs`
//!
//! Zero-Allocation (`#![no_std]` compatible) 4-Register `WellRecord` Parser,
//! `SignalResolutionTable` (`handle`, `name`, `container`, `address`), and
//! `[u8; 512]` `StackMarkdownBuf` (`core::fmt::Write`) Markdown Table Formatter.

use core::fmt;

/// Fixed capacity of the stack-allocated Markdown report buffer (fits inside a 512B DMA/UART page).
pub const SIGNAL_MARKDOWN_BUF_CAP: usize = 512;

/// Balanced ternary state for a single metadata sub-check.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(i8)]
pub enum SignalTrit {
    Contradictory = -1,
    Empty = 0,
    Present = 1,
}

impl SignalTrit {
    #[inline(always)]
    pub const fn as_i8(self) -> i8 {
        self as i8
    }

    /// Maps `{-1, 0, +1}` to unsigned `{0, 1, 2}` for 3^3 = 27 cube indexing.
    #[inline(always)]
    pub const fn to_unsigned_trit(self) -> u8 {
        ((self as i8) + 1) as u8
    }
}

/// Identifies each of the four registers in `## the signal resolution`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RegisterKind {
    Handle = 0,
    Name = 1,
    Container = 2,
    Address = 3,
}

impl RegisterKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Handle => "handle",
            Self::Name => "name",
            Self::Container => "container",
            Self::Address => "address",
        }
    }
}

/// A single 3-trit row in the signal resolution table.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RegisterResolution {
    pub register: RegisterKind,
    pub trits: [SignalTrit; 3],
    pub labels: [&'static str; 3],
}

impl RegisterResolution {
    /// Positive numerator (`0..=3`) of present (`+1`) signals in this register.
    #[inline]
    pub const fn numerator(&self) -> u8 {
        let mut sum = 0u8;
        let mut i = 0usize;
        while i < 3 {
            if (self.trits[i] as i8) > 0 {
                sum += 1;
            }
            i += 1;
        }
        sum
    }

    /// Net algebraic ternary sum in `-3..=3`.
    #[inline]
    pub const fn algebraic_sum(&self) -> i8 {
        (self.trits[0] as i8) + (self.trits[1] as i8) + (self.trits[2] as i8)
    }

    /// Packs the 3 trits `(t0, t1, t2)` into a `3^3 = 27` Futhark cube index `0..=26`.
    #[inline]
    pub const fn cube27_index(&self) -> u8 {
        let u0 = self.trits[0].to_unsigned_trit();
        let u1 = self.trits[1].to_unsigned_trit();
        let u2 = self.trits[2].to_unsigned_trit();
        u0 + 3 * u1 + 9 * u2
    }
}

/// Zero-allocation borrowed view of `## the well`.
/// All string slices borrow directly from the caller's input buffer.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct WellRecord<'a> {
    pub source: Option<&'a str>,
    pub kind: Option<&'a str>,
    pub title: Option<&'a str>,
    pub artist: Option<&'a str>,
    pub album: Option<&'a str>,
    pub year: Option<u16>,
    pub year_malformed: bool,
    pub genre: Option<&'a str>,
    pub url: Option<&'a str>,
    pub duration: Option<&'a str>,
    pub track_count: Option<u16>,
}

impl<'a> WellRecord<'a> {
    /// Parses a raw markdown/key-value "well" block in a single zero-allocation pass.
    /// Treats missing fields or literal `"empty"` (case-insensitive) as `None` (`SignalTrit::Empty`).
    pub fn parse_zero_alloc(raw_well: &'a str) -> Self {
        let mut rec = Self::default();

        for raw_line in raw_well.lines() {
            let trimmed = raw_line.trim();
            // Strip optional markdown bullet prefix (`- ` or `* `)
            let line = if let Some(rest) = trimmed.strip_prefix('-') {
                rest.trim()
            } else if let Some(rest) = trimmed.strip_prefix('*') {
                rest.trim()
            } else {
                trimmed
            };

            let Some((raw_key, raw_val)) = line.split_once(':') else {
                continue;
            };
            let key = raw_key.trim();
            let val = normalize_well_value(raw_val);

            if eq_ignore_ascii_case(key, "source") {
                rec.source = val;
            } else if eq_ignore_ascii_case(key, "kind") {
                rec.kind = val;
            } else if eq_ignore_ascii_case(key, "title") {
                rec.title = val;
            } else if eq_ignore_ascii_case(key, "artist") || eq_ignore_ascii_case(key, "handle") {
                rec.artist = val;
            } else if eq_ignore_ascii_case(key, "album") {
                rec.album = val;
            } else if eq_ignore_ascii_case(key, "year") {
                if let Some(y_str) = val {
                    match parse_u16_no_alloc(y_str) {
                        Some(y) if (1000..=9999).contains(&y) => rec.year = Some(y),
                        _ => rec.year_malformed = true,
                    }
                }
            } else if eq_ignore_ascii_case(key, "genre") {
                rec.genre = val;
            } else if eq_ignore_ascii_case(key, "url") || eq_ignore_ascii_case(key, "address") {
                rec.url = val;
            } else if eq_ignore_ascii_case(key, "duration") {
                rec.duration = val;
            } else if eq_ignore_ascii_case(key, "track_count")
                || eq_ignore_ascii_case(key, "tracks")
            {
                if let Some(t_str) = val {
                    rec.track_count = parse_u16_no_alloc(t_str);
                }
            }
        }

        rec
    }

    /// Implements "A later lap holding a longer record can overrule any line above":
    /// Any populated field in `later` overrides the corresponding slot in `self`.
    pub const fn overrule_with_later_lap(self, later: Self) -> Self {
        Self {
            source: if later.source.is_some() {
                later.source
            } else {
                self.source
            },
            kind: if later.kind.is_some() {
                later.kind
            } else {
                self.kind
            },
            title: if later.title.is_some() {
                later.title
            } else {
                self.title
            },
            artist: if later.artist.is_some() {
                later.artist
            } else {
                self.artist
            },
            album: if later.album.is_some() {
                later.album
            } else {
                self.album
            },
            year: if later.year.is_some() {
                later.year
            } else {
                self.year
            },
            year_malformed: later.year_malformed,
            genre: if later.genre.is_some() {
                later.genre
            } else {
                self.genre
            },
            url: if later.url.is_some() {
                later.url
            } else {
                self.url
            },
            duration: if later.duration.is_some() {
                later.duration
            } else {
                self.duration
            },
            track_count: if later.track_count.is_some() {
                later.track_count
            } else {
                self.track_count
            },
        }
    }

    /// Evaluates the 4-register ternary signal resolution table (`handle`, `name`, `container`, `address`).
    pub fn resolve_signals(&self) -> SignalResolutionTable {
        let handle_trit = match self.artist {
            Some(_) => SignalTrit::Present,
            None => SignalTrit::Empty,
        };
        let album_trit = match self.album {
            Some(_) => SignalTrit::Present,
            None => SignalTrit::Empty,
        };
        let year_trit = if self.year_malformed {
            SignalTrit::Contradictory
        } else if self.year.is_some() {
            SignalTrit::Present
        } else {
            SignalTrit::Empty
        };

        let title_trit = match self.title {
            Some(_) => SignalTrit::Present,
            None => SignalTrit::Empty,
        };
        let self_desc_trit = match self.title {
            Some(t) if title_carries_own_category(t) => SignalTrit::Present,
            _ => SignalTrit::Empty,
        };
        let genre_trit = match self.genre {
            Some(_) => SignalTrit::Present,
            None => SignalTrit::Empty,
        };
        let kind_trit = match self.kind {
            Some(_) => SignalTrit::Present,
            None => SignalTrit::Empty,
        };

        let url_trit = match self.url {
            Some(u) if u.starts_with("http://") || u.starts_with("https://") => SignalTrit::Present,
            Some(_) => SignalTrit::Contradictory,
            None => SignalTrit::Empty,
        };
        let duration_trit = match self.duration {
            Some(_) => SignalTrit::Present,
            None => SignalTrit::Empty,
        };
        let track_count_trit = match self.track_count {
            Some(c) if c > 0 => SignalTrit::Present,
            _ => SignalTrit::Empty,
        };

        let registers = [
            RegisterResolution {
                register: RegisterKind::Handle,
                trits: [handle_trit, album_trit, year_trit],
                labels: ["handle", "album", "year"],
            },
            RegisterResolution {
                register: RegisterKind::Name,
                trits: [title_trit, self_desc_trit, genre_trit],
                labels: ["title", "self-description", "genre"],
            },
            RegisterResolution {
                register: RegisterKind::Container,
                trits: [kind_trit, album_trit, genre_trit],
                labels: ["kind", "album", "genre"],
            },
            RegisterResolution {
                register: RegisterKind::Address,
                trits: [url_trit, duration_trit, track_count_trit],
                labels: ["url", "duration", "track count"],
            },
        ];

        let handle_is_unattached_title_case = self
            .artist
            .map(is_unattached_title_case_personal_name)
            .unwrap_or(false);

        let is_thin_object = self.kind.is_some() && self.album.is_none() && self.genre.is_none();

        let pcc_seal_u64 = compute_resolution_fnv1a(&registers);

        SignalResolutionTable {
            registers,
            handle_is_unattached_title_case,
            is_thin_object,
            pcc_seal_u64,
        }
    }
}

/// Complete 4-register signal resolution output.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SignalResolutionTable {
    pub registers: [RegisterResolution; 4],
    pub handle_is_unattached_title_case: bool,
    pub is_thin_object: bool,
    pub pcc_seal_u64: u64,
}

impl SignalResolutionTable {
    /// Total present signals out of 12 (`4 registers * 3 trits`).
    #[inline]
    pub const fn total_present_out_of_12(&self) -> u8 {
        self.registers[0].numerator()
            + self.registers[1].numerator()
            + self.registers[2].numerator()
            + self.registers[3].numerator()
    }
}

impl fmt::Display for RegisterResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fmt_trit = |t: SignalTrit| -> &'static str {
            match t {
                SignalTrit::Present => "+1",
                SignalTrit::Empty => "0",
                SignalTrit::Contradictory => "-1",
            }
        };
        write!(
            f,
            "{} {} · {} {} · {} {} = {}/3",
            fmt_trit(self.trits[0]),
            self.labels[0],
            fmt_trit(self.trits[1]),
            self.labels[1],
            fmt_trit(self.trits[2]),
            self.labels[2],
            self.numerator()
        )
    }
}

/// Stack-allocated UTF-8 buffer implementing `core::fmt::Write` with deterministic bounds checking.
#[derive(Copy, Clone)]
pub struct StackMarkdownBuf {
    buf: [u8; SIGNAL_MARKDOWN_BUF_CAP],
    cursor: usize,
}

impl Default for StackMarkdownBuf {
    fn default() -> Self {
        Self::new()
    }
}

impl StackMarkdownBuf {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            buf: [0u8; SIGNAL_MARKDOWN_BUF_CAP],
            cursor: 0,
        }
    }

    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.cursor
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.cursor == 0
    }

    #[inline(always)]
    pub const fn remaining_capacity(&self) -> usize {
        SIGNAL_MARKDOWN_BUF_CAP - self.cursor
    }

    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.cursor]
    }

    /// Returns the written slice as a `&str`.
    /// Because `write_str` only appends complete, valid `&str` slices (and rejects partial writes
    /// on overflow), `self.buf[..self.cursor]` is always valid UTF-8.
    #[inline]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.cursor]).unwrap_or("")
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.cursor = 0;
    }
}

impl fmt::Write for StackMarkdownBuf {
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let bytes = s.as_bytes();
        let next_cursor = self.cursor.checked_add(bytes.len()).ok_or(fmt::Error)?;
        if next_cursor > SIGNAL_MARKDOWN_BUF_CAP {
            return Err(fmt::Error);
        }
        self.buf[self.cursor..next_cursor].copy_from_slice(bytes);
        self.cursor = next_cursor;
        Ok(())
    }
}

impl fmt::Debug for StackMarkdownBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StackMarkdownBuf")
            .field("len", &self.cursor)
            .field("remaining", &self.remaining_capacity())
            .field("content", &self.as_str())
            .finish()
    }
}

impl SignalResolutionTable {
    /// Streams `the signal` column description for a given `RegisterKind` directly into `w`
    /// by borrowing slices from `well`—requiring zero intermediate strings or heap allocations.
    pub fn write_register_signal_prose<W: fmt::Write>(
        &self,
        reg_kind: RegisterKind,
        well: &WellRecord<'_>,
        w: &mut W,
    ) -> fmt::Result {
        match reg_kind {
            RegisterKind::Handle => {
                if self.handle_is_unattached_title_case && well.album.is_none() {
                    w.write_str("a full personal name, title case, unattached")
                } else if self.handle_is_unattached_title_case && well.album.is_some() {
                    w.write_str("a full personal name, title case, album-bound")
                } else if well.artist.is_some() {
                    w.write_str("credited handle, qualified")
                } else {
                    w.write_str("handle empty")
                }
            }
            RegisterKind::Name => {
                let name_reg = &self.registers[1];
                let has_title = name_reg.trits[0] == SignalTrit::Present;
                let has_self_desc = name_reg.trits[1] == SignalTrit::Present;
                let has_genre = name_reg.trits[2] == SignalTrit::Present;

                if has_title && has_self_desc && !has_genre {
                    w.write_str("title carries its own category — a name, then a practice")
                } else if has_title && has_self_desc && has_genre {
                    w.write_str("title carries its own category, genre confirmed")
                } else if has_title {
                    w.write_str("named title, external category")
                } else {
                    w.write_str("title empty")
                }
            }
            RegisterKind::Container => {
                let kind_str = well.kind.unwrap_or("empty");
                write!(w, "kind is {kind_str}, ")?;

                if let Some(alb) = well.album {
                    write!(w, "album {alb}, ")?;
                } else {
                    w.write_str("album empty, ")?;
                }

                if let Some(gen_str) = well.genre {
                    write!(w, "genre {gen_str}")
                } else {
                    w.write_str("genre empty")
                }
            }
            RegisterKind::Address => {
                if well.url.is_some() && well.duration.is_none() && well.track_count.is_none() {
                    w.write_str("one url, one file")
                } else if well.url.is_some() {
                    w.write_str("one url")?;
                    if let Some(dur) = well.duration {
                        write!(w, ", duration {dur}")?;
                    }
                    if let Some(cnt) = well.track_count {
                        write!(w, ", {cnt} tracks")?;
                    }
                    Ok(())
                } else {
                    w.write_str("address empty")
                }
            }
        }
    }

    /// Formats the complete 4-register `## the signal resolution` Markdown table
    /// into any `core::fmt::Write` sink (`StackMarkdownBuf`, UART, or DMA buffer).
    pub fn write_markdown_table<W: fmt::Write>(
        &self,
        well: &WellRecord<'_>,
        w: &mut W,
    ) -> fmt::Result {
        w.write_str("| register | the signal | the ternary |\n")?;
        w.write_str("|---|---|---|\n")?;

        for (idx, reg) in self.registers.iter().enumerate() {
            write!(w, "| {} | ", reg.register.label())?;
            self.write_register_signal_prose(reg.register, well, w)?;
            write!(w, " | {reg} |")?;
            if idx + 1 < self.registers.len() {
                w.write_str("\n")?;
            }
        }
        Ok(())
    }

    /// Convenience helper returning a stack-allocated `StackMarkdownBuf` (`[u8; 512]`)
    /// populated with the rendered Markdown table.
    pub fn render_to_stack_buf(
        &self,
        well: &WellRecord<'_>,
    ) -> Result<StackMarkdownBuf, fmt::Error> {
        let mut buf = StackMarkdownBuf::new();
        self.write_markdown_table(well, &mut buf)?;
        Ok(buf)
    }
}

// ============================================================================
// Zero-Allocation String & Lexer Helpers
// ============================================================================

/// Vocabulary of practice/form terms that allow a compound title to carry its own category
/// (`+1 self-description`) even when the `genre` field is empty.
const SELF_DESCRIBING_PRACTICE_TERMS: [&str; 12] = [
    "mantra",
    "prayer",
    "chant",
    "sadhana",
    "puja",
    "hymn",
    "meditation",
    "invocation",
    "liturgy",
    "sonata",
    "prelude",
    "nocturne",
];

#[inline]
fn normalize_well_value(raw: &str) -> Option<&str> {
    let v = raw.trim();
    if v.is_empty() || eq_ignore_ascii_case(v, "empty") || eq_ignore_ascii_case(v, "none") {
        None
    } else {
        Some(v)
    }
}

#[inline]
fn eq_ignore_ascii_case(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.as_bytes()
        .iter()
        .zip(b.as_bytes().iter())
        .all(|(&x, &y)| x.eq_ignore_ascii_case(&y))
}

#[inline]
fn parse_u16_no_alloc(s: &str) -> Option<u16> {
    let bytes = s.trim().as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let mut acc: u16 = 0;
    for &b in bytes {
        if !b.is_ascii_digit() {
            return None;
        }
        acc = acc.checked_mul(10)?.checked_add((b - b'0') as u16)?;
    }
    Some(acc)
}

/// Checks whether `title` is a compound phrase (at least 2 whitespace-separated tokens)
/// where a trailing token names a recognized musical/spiritual practice.
fn title_carries_own_category(title: &str) -> bool {
    let mut token_count = 0usize;
    let mut has_practice_term = false;

    for raw_tok in title.split_whitespace() {
        let tok = raw_tok.trim_matches(|c: char| !c.is_alphanumeric());
        if tok.is_empty() {
            continue;
        }
        token_count += 1;
        if token_count >= 2 {
            for &term in &SELF_DESCRIBING_PRACTICE_TERMS {
                if eq_ignore_ascii_case(tok, term) {
                    has_practice_term = true;
                }
            }
        }
    }

    token_count >= 2 && has_practice_term
}

/// Verifies whether `handle` is an unattached personal name in Title Case
/// (at least 2 words, each starting with an uppercase letter, with no `feat.`, `&`, `/`, or `,`).
fn is_unattached_title_case_personal_name(handle: &str) -> bool {
    if handle.contains('&') || handle.contains(',') || handle.contains('/') {
        return false;
    }
    let mut words = 0usize;
    for tok in handle.split_whitespace() {
        if eq_ignore_ascii_case(tok, "feat")
            || eq_ignore_ascii_case(tok, "feat.")
            || eq_ignore_ascii_case(tok, "ft.")
            || eq_ignore_ascii_case(tok, "vs.")
        {
            return false;
        }
        let mut chars = tok.chars();
        let Some(first) = chars.next() else {
            continue;
        };
        if !first.is_uppercase() || !chars.all(|c| c.is_lowercase()) {
            return false;
        }
        words += 1;
    }
    words >= 2
}

const fn compute_resolution_fnv1a(regs: &[RegisterResolution; 4]) -> u64 {
    let mut h = 0xCBF2_9CE4_8422_2325u64;
    let mut r = 0usize;
    while r < 4 {
        let idx = regs[r].cube27_index();
        h ^= idx as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
        r += 1;
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOG_POST_WELL_LAP_1: &str = r#"
- source: youtube
- kind: video
- title: Mahakala mantra prayer
- artist: Alexander Hacker
- album: empty
- year: 2026
- genre: empty
- url: https://music.youtube.com/watch?v=pL18Xr_26ts&si=PVlUHWbxr2Fja3t-
"#;

    const EXPECTED_BLOG_MARKDOWN_TABLE: &str = "\
| register | the signal | the ternary |
|---|---|---|
| handle | a full personal name, title case, unattached | +1 handle · 0 album · +1 year = 2/3 |
| name | title carries its own category — a name, then a practice | +1 title · +1 self-description · 0 genre = 2/3 |
| container | kind is video, album empty, genre empty | +1 kind · 0 album · 0 genre = 1/3 |
| address | one url, one file | +1 url · 0 duration · 0 track count = 1/3 |";

    #[test]
    fn test_lap1_mahakala_well_and_later_lap_overrule() {
        let lap1_well = WellRecord::parse_zero_alloc(BLOG_POST_WELL_LAP_1);
        assert_eq!(lap1_well.source, Some("youtube"));
        assert_eq!(lap1_well.kind, Some("video"));
        assert_eq!(lap1_well.title, Some("Mahakala mantra prayer"));
        assert_eq!(lap1_well.artist, Some("Alexander Hacker"));
        assert_eq!(lap1_well.album, None);
        assert_eq!(lap1_well.year, Some(2026));
        assert_eq!(lap1_well.genre, None);

        let res1 = lap1_well.resolve_signals();
        assert!(res1.is_thin_object);
        assert!(res1.handle_is_unattached_title_case);
        assert_eq!(res1.registers[0].numerator(), 2);
        assert_eq!(res1.registers[1].numerator(), 2);
        assert_eq!(res1.registers[2].numerator(), 1);
        assert_eq!(res1.registers[3].numerator(), 1);
        assert_eq!(res1.total_present_out_of_12(), 6);

        let stack_buf = res1
            .render_to_stack_buf(&lap1_well)
            .expect("444-byte Markdown table must fit inside [u8; 512]");
        assert_eq!(stack_buf.as_str(), EXPECTED_BLOG_MARKDOWN_TABLE);
        assert_eq!(stack_buf.len(), 444);
        assert_eq!(stack_buf.remaining_capacity(), 512 - 444);

        let raw_lap_2 = "- genre: Devotional Chant\n- duration: 02:23\n";
        let lap2_delta = WellRecord::parse_zero_alloc(raw_lap_2);
        let merged_well = lap1_well.overrule_with_later_lap(lap2_delta);
        let res2 = merged_well.resolve_signals();

        assert!(!res2.is_thin_object);
        assert_eq!(res2.registers[0].numerator(), 2);
        assert_eq!(res2.registers[1].numerator(), 3);
        assert_eq!(res2.registers[2].numerator(), 2);
        assert_eq!(res2.registers[3].numerator(), 2);
        assert_eq!(res2.total_present_out_of_12(), 9);
        assert_ne!(res1.pcc_seal_u64, res2.pcc_seal_u64);
    }
}
