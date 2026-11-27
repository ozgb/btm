// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), formats/tdf.go, formats/tdf_typed.go, formats/tdf_diag.go, formats/ascii.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! TDF text definition files, and FBI unit files (FBI is TDF syntax; there is no
//! FBI-specific parser behaviour).
//!
//! The parser takes bytes and returns a lossless syntax tree: a [`Section`] keeps its
//! items in source order, duplicates included, plus a resolved key view built with
//! retail's duplicate-key policy. Names, keys and values are raw bytes, because the
//! sampled retail files are Windows-1252 and Go compared them as bytes.
//!
//! Case folding is ASCII only: bytes 0x80 and above compare literally. Whitespace
//! trimmed around names, keys and values is exactly space, tab, CR and LF.
//!
//! Duplicate keys (the policy that matters): an assignment whose spelling is
//! byte-identical to the head of its case-insensitive run replaces that entry in place
//! (last write wins). Anything else, including a case variant, is inserted in front of
//! the run. Typed lookups read the head of the run, which is therefore the last
//! variant parsed.
//!
//! Differences from the Go source, all deliberate:
//! - The parser is iterative, not recursive, and `max_depth` is clamped to 1024
//!   ([`HARD_MAX_DEPTH`]) so a custom limit cannot overflow the stack on parse or drop.
//! - The resolved view is built with an ordered map per section and flattened, instead
//!   of by repeated vector insertion. The result is the same vector; the cost is
//!   O(n log n) instead of O(n^2).
//! - `Section::int`, `float` and `bool` return `Option`: the Go versions never
//!   returned an error.
//! - No VFS loader (`LoadTDF`, `WithTDFContext`); callers read the bytes themselves and
//!   may attach a file name with [`Error::with_file`].

use std::collections::BTreeMap;
use std::fmt;

/// Title retail reports parse failures under.
pub const PARSE_ERROR_TITLE: &str = "Parse error in .TDF File!";

/// Largest section nesting depth this parser will accept whatever `Limits` says.
/// Deviation from Go, which recurses on a growable stack; see the module comment.
pub const HARD_MAX_DEPTH: usize = 1024;

/// One of retail's five TDF parse diagnostics. The strings are verbatim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseDiagnostic {
    EqualsNotFound,
    SemicolonMissing,
    ClosingBracket,
    OpeningBrace,
    NextBlock,
}

impl ParseDiagnostic {
    /// The verbatim retail text.
    pub const fn message(self) -> &'static str {
        match self {
            Self::EqualsNotFound => "Data field - '=' not found",
            Self::SemicolonMissing => "Data field - ';' not found",
            Self::ClosingBracket => "Sub-record - closing ']' not found",
            Self::OpeningBrace => "Sub-record - opening '{' not found",
            Self::NextBlock => "End of file - nextblock not zero",
        }
    }
}

impl fmt::Display for ParseDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

/// A retail parse diagnostic plus the context retail prints with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub diagnostic: ParseDiagnostic,
    /// Original spelling of the section active when the parse failed; `root` when none
    /// (the document root, or a section with an empty name).
    pub section: Vec<u8>,
    /// Logical path, filled in by the caller through [`ParseError::with_file`].
    pub file: String,
    /// Byte offset into the comment-blanked source.
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

impl ParseError {
    /// Retail's detail line: ` - name = '<section>' from file <file>`.
    pub fn detail(&self) -> String {
        format!(
            " - name = '{}' from file {}",
            String::from_utf8_lossy(&self.section),
            self.file
        )
    }

    /// A copy naming the logical path. The parser itself has no provenance.
    pub fn with_file(&self, file: &str) -> ParseError {
        ParseError {
            file: file.to_owned(),
            ..self.clone()
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{PARSE_ERROR_TITLE} {}{}",
            self.diagnostic,
            self.detail()
        )
    }
}

/// Everything [`Document::parse`] can fail with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// A limit was zero.
    InvalidLimits,
    DocumentTooLarge {
        size: usize,
        limit: usize,
    },
    /// One of retail's five diagnostics.
    Parse(ParseError),
    ItemLimit {
        limit: usize,
        line: usize,
        column: usize,
    },
    DepthLimit {
        limit: usize,
        line: usize,
        column: usize,
    },
    SectionNameTooLong {
        limit: usize,
        line: usize,
        column: usize,
    },
    KeyTooLong {
        limit: usize,
        line: usize,
        column: usize,
    },
    ValueTooLong {
        limit: usize,
        line: usize,
        column: usize,
    },
    UnterminatedValueComment {
        line: usize,
        column: usize,
    },
}

impl Error {
    /// Attach a logical file name to a parser diagnostic; other errors are unchanged.
    pub fn with_file(self, file: &str) -> Error {
        match self {
            Error::Parse(p) => Error::Parse(p.with_file(file)),
            other => other,
        }
    }

    /// The parser diagnostic, when this error is one.
    pub fn parse_error(&self) -> Option<&ParseError> {
        match self {
            Error::Parse(p) => Some(p),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidLimits => f.write_str("tdf: invalid parse limits"),
            Error::DocumentTooLarge { size, limit } => {
                write!(f, "tdf: document size {size} exceeds limit {limit}")
            }
            Error::Parse(p) => p.fmt(f),
            Error::ItemLimit {
                limit,
                line,
                column,
            } => write!(
                f,
                "tdf: line {line} column {column}: document item count exceeds limit {limit}"
            ),
            Error::DepthLimit {
                limit,
                line,
                column,
            } => write!(
                f,
                "tdf: line {line} column {column}: section nesting exceeds limit {limit}"
            ),
            Error::SectionNameTooLong {
                limit,
                line,
                column,
            } => write!(
                f,
                "tdf: line {line} column {column}: section name exceeds limit {limit}"
            ),
            Error::KeyTooLong {
                limit,
                line,
                column,
            } => write!(
                f,
                "tdf: line {line} column {column}: assignment key exceeds limit {limit}"
            ),
            Error::ValueTooLong {
                limit,
                line,
                column,
            } => write!(
                f,
                "tdf: line {line} column {column}: assignment value exceeds limit {limit}"
            ),
            Error::UnterminatedValueComment { line, column } => write!(
                f,
                "tdf: line {line} column {column}: unterminated block comment in value"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// Bounds on the work done for one untrusted document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_depth: usize,
    pub max_items: usize,
    pub max_value_bytes: usize,
    pub max_name_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_bytes: 16 << 20,
            max_depth: 256,
            max_items: 1 << 18,
            max_value_bytes: 4 << 20,
            max_name_bytes: 1 << 20,
        }
    }
}

/// One `key=value;` assignment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assignment {
    /// Folded key (trimmed, ASCII lower-cased).
    pub key: Vec<u8>,
    /// Trimmed key in its authored spelling.
    pub original_key: Vec<u8>,
    /// Trimmed value.
    pub value: Vec<u8>,
    pub line: usize,
    pub column: usize,
}

/// A section's contents, in source order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    Assignment(Assignment),
    Section(Section),
}

/// A lossless TDF section. The document root is an unnamed section.
///
/// Key lookups use a resolved view built once when the section closes (see the module
/// comment for the duplicate policy). `items` and `resolved` never change afterwards.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Section {
    name: Vec<u8>,
    original_name: Vec<u8>,
    line: usize,
    column: usize,
    items: Vec<Item>,
    /// Indices into `items` of the surviving assignments, in case-insensitive key order;
    /// within one fold-equal run the most recently inserted entry comes first.
    resolved: Vec<usize>,
}

/// The parsed syntax tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    root: Section,
}

impl Document {
    /// Parse under the default limits.
    pub fn parse(data: &[u8]) -> Result<Document, Error> {
        Self::parse_with_limits(data, Limits::default())
    }

    /// Parse under explicit limits.
    pub fn parse_with_limits(data: &[u8], limits: Limits) -> Result<Document, Error> {
        if limits.max_bytes == 0
            || limits.max_depth == 0
            || limits.max_items == 0
            || limits.max_value_bytes == 0
            || limits.max_name_bytes == 0
        {
            return Err(Error::InvalidLimits);
        }
        if data.len() > limits.max_bytes {
            return Err(Error::DocumentTooLarge {
                size: data.len(),
                limit: limits.max_bytes,
            });
        }
        // Comments are blanked to spaces before the grammar runs, preserving every
        // offset. The parser's own inline comment handling stays as a safety net.
        let blanked = blank_comments(data);
        let mut parser = Parser {
            data: &blanked,
            pos: 0,
            line: 1,
            column: 1,
            items: 0,
            limits,
            max_depth: limits.max_depth.min(HARD_MAX_DEPTH),
        };
        let root = parser.run()?;
        Ok(Document { root })
    }

    /// The unnamed root section.
    pub fn root(&self) -> &Section {
        &self.root
    }
}

/// Trim only the four whitespace bytes the TDF parser recognises.
fn trim_semantic(value: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = value.len();
    while start < end && value.get(start).is_some_and(|b| is_semantic_space(*b)) {
        start += 1;
    }
    while end > start
        && end
            .checked_sub(1)
            .and_then(|i| value.get(i))
            .is_some_and(|b| is_semantic_space(*b))
    {
        end -= 1;
    }
    value.get(start..end).unwrap_or(&[])
}

fn is_semantic_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}

/// Fold a name for comparison. ASCII only: the code-page behaviour for bytes >= 0x80 is
/// untraced upstream, so those bytes compare literally.
///
/// Matches `FoldASCII` in `formats/ascii.go`: only `A..=Z` are lower-cased.
fn fold_name(name: &[u8]) -> Vec<u8> {
    trim_semantic(name).to_ascii_lowercase()
}

impl Section {
    /// Folded name (empty for the root).
    pub fn name(&self) -> &[u8] {
        &self.name
    }

    /// Trimmed name as authored.
    pub fn original_name(&self) -> &[u8] {
        &self.original_name
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }

    /// Every item in source order, duplicates included.
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// Nested sections in source order.
    pub fn sections(&self) -> impl Iterator<Item = &Section> {
        self.items.iter().filter_map(|item| match item {
            Item::Section(s) => Some(s),
            Item::Assignment(_) => None,
        })
    }

    /// Assignments in source order, duplicates included.
    pub fn assignments(&self) -> impl Iterator<Item = &Assignment> {
        self.items.iter().filter_map(|item| match item {
            Item::Assignment(a) => Some(a),
            Item::Section(_) => None,
        })
    }

    fn resolved_at(&self, rank: usize) -> Option<&Assignment> {
        let index = *self.resolved.get(rank)?;
        match self.items.get(index)? {
            Item::Assignment(a) => Some(a),
            Item::Section(_) => None,
        }
    }

    /// The resolved vector in case-insensitive key order, retaining case variants.
    pub fn resolved_assignments(&self) -> impl Iterator<Item = &Assignment> {
        (0..self.resolved.len()).filter_map(|rank| self.resolved_at(rank))
    }

    /// Rank of the lower bound of `fold`'s run in the resolved vector.
    fn lower_bound(&self, fold: &[u8]) -> usize {
        self.resolved.partition_point(|&index| {
            match self.items.get(index) {
                Some(Item::Assignment(a)) => a.key.as_slice() < fold,
                // Unreachable: `resolved` only holds assignment indices.
                _ => true,
            }
        })
    }

    fn lookup_resolved(&self, key: &[u8]) -> Option<&Assignment> {
        let fold = fold_name(key);
        let found = self.resolved_at(self.lower_bound(&fold))?;
        (found.key == fold).then_some(found)
    }

    /// Values of every distinct key spelling matching `key`, in resolved order (newest
    /// first). Identical spellings collapsed to their last value.
    pub fn values(&self, key: impl AsRef<[u8]>) -> Vec<&[u8]> {
        let fold = fold_name(key.as_ref());
        self.resolved_assignments()
            .filter(|a| a.key == fold)
            .map(|a| a.value.as_slice())
            .collect()
    }

    /// The head of `key`'s fold run: the last case variant parsed.
    pub fn first_value(&self, key: impl AsRef<[u8]>) -> Option<&[u8]> {
        self.lookup_resolved(key.as_ref())
            .map(|a| a.value.as_slice())
    }

    /// The upper bound of `key`'s fold run: the earliest-inserted variant still held.
    /// For keys authored with one spelling this equals [`Section::first_value`].
    pub fn last_value(&self, key: impl AsRef<[u8]>) -> Option<&[u8]> {
        let fold = fold_name(key.as_ref());
        let lo = self.lower_bound(&fold);
        let mut last = None;
        for rank in lo..self.resolved.len() {
            match self.resolved_at(rank) {
                Some(a) if a.key == fold => last = Some(a.value.as_slice()),
                _ => break,
            }
        }
        last
    }

    /// The first nested section with this name (case-insensitive).
    pub fn section(&self, name: impl AsRef<[u8]>) -> Option<&Section> {
        let fold = fold_name(name.as_ref());
        self.sections().find(|s| s.name == fold)
    }

    /// Every nested section with this name, in source order: retail retains duplicate
    /// sibling sections.
    pub fn sections_named(&self, name: impl AsRef<[u8]>) -> Vec<&Section> {
        let fold = fold_name(name.as_ref());
        self.sections().filter(|s| s.name == fold).collect()
    }

    /// Integer through the lookup shared by the typed family. `None` when the key is
    /// absent. (The Go version also returned an error that was always nil.)
    pub fn int(&self, key: impl AsRef<[u8]>) -> Option<i64> {
        self.first_value(key)
            .map(|v| i64::from(parse_tdf_integer(v)))
    }

    /// Float through the floating accessor; `None` when the key is absent.
    pub fn float(&self, key: impl AsRef<[u8]>) -> Option<f64> {
        self.first_value(key).map(parse_tdf_float)
    }

    /// Integer consumed as a flag; `None` when the key is absent.
    pub fn bool(&self, key: impl AsRef<[u8]>) -> Option<bool> {
        self.first_value(key).map(|v| parse_tdf_integer(v) != 0)
    }

    /// Retail's CRT decimal conversion. Only an absent key returns the default; an
    /// authored empty or unparsable value returns zero.
    pub fn int_value(&self, key: impl AsRef<[u8]>, default: i32) -> i32 {
        self.first_value(key).map_or(default, parse_tdf_integer)
    }

    /// CRT decimal floating conversion; the default applies only to an absent key.
    pub fn float_value(&self, key: impl AsRef<[u8]>, default: f64) -> f64 {
        self.first_value(key).map_or(default, parse_tdf_float)
    }

    /// Authored float to 16.16: multiply by 65536 and truncate toward zero. The default
    /// is returned verbatim; it is already in 16.16 units.
    pub fn fixed_value(&self, key: impl AsRef<[u8]>, default: i32) -> i32 {
        self.first_value(key)
            .map_or(default, |v| fixed_from_authored(parse_tdf_float(v)))
    }

    /// The text and whether the key was authored. A string field can tell an authored
    /// empty value from a missing key.
    pub fn string_value<'a>(
        &'a self,
        key: impl AsRef<[u8]>,
        default: &'a [u8],
    ) -> (&'a [u8], bool) {
        match self.first_value(key) {
            Some(v) => (v, true),
            None => (default, false),
        }
    }

    /// Tries `<language><key>` first (when `language` is non-empty), then the plain key.
    pub fn language_string<'a>(
        &'a self,
        language: impl AsRef<[u8]>,
        key: impl AsRef<[u8]>,
        default: &'a [u8],
    ) -> (&'a [u8], bool) {
        let language = language.as_ref();
        if !language.is_empty() {
            let mut joined = language.to_vec();
            joined.extend_from_slice(key.as_ref());
            if let Some(v) = self.first_value(&joined) {
                return (v, true);
            }
        }
        self.string_value(key, default)
    }

    /// The stored text and whether the key was authored.
    pub fn raw_value(&self, key: impl AsRef<[u8]>) -> Option<&[u8]> {
        self.first_value(key)
    }

    /// Nonzero integer test with a caller default. Retail has no universal boolean
    /// parser; packed flags keep the integer low bit, which is the caller's business.
    pub fn bool_value(&self, key: impl AsRef<[u8]>, default: bool) -> bool {
        self.first_value(key)
            .map_or(default, |v| parse_tdf_integer(v) != 0)
    }
}

/// Retail's 32-bit decimal `atoi`: leading whitespace, optional sign, decimal digit
/// prefix, trailing bytes ignored, zero when there is no digit, wrapping at 32 bits, no
/// hexadecimal.
pub fn parse_tdf_integer(value: &[u8]) -> i32 {
    let mut i = 0;
    while value.get(i).is_some_and(|b| is_integer_space(*b)) {
        i += 1;
    }
    let mut negative = false;
    if let Some(sign @ (b'+' | b'-')) = value.get(i).copied() {
        negative = sign == b'-';
        i += 1;
    }
    let mut magnitude: u32 = 0;
    while let Some(d) = value.get(i).copied().filter(u8::is_ascii_digit) {
        magnitude = magnitude.wrapping_mul(10).wrapping_add(u32::from(d - b'0'));
        i += 1;
    }
    // Reinterpreting the wrapped magnitude as signed is the Go `int32(magnitude)`.
    let result = i32::from_ne_bytes(magnitude.to_ne_bytes());
    if negative {
        result.wrapping_neg()
    } else {
        result
    }
}

fn is_integer_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Mirrors the CRT decimal floating conversion: leading whitespace, optional sign,
/// digits with an optional fraction and an `e`/`E`/`d`/`D` exponent, trailing junk
/// ignored, zero for unparsable text.
///
/// PROVISIONAL: number parsing pending specs/files.md and the files gate. The prefix
/// scanner follows Go; the conversion of the prefix uses Rust std (`f64::from_str`),
/// where Go used `strconv.ParseFloat`. Unsure cases are listed in the report: very long
/// mantissas, extreme exponents, exponent digit strings longer than an i64, and whether
/// retail rounds any of these differently from correctly-rounded binary64.
pub fn parse_tdf_float(value: &[u8]) -> f64 {
    // PROVISIONAL: number parsing pending specs/files.md and the files gate.
    let start = value
        .iter()
        .position(|b| !matches!(b, b' ' | b'\t' | b'\r' | b'\n' | 0x0b | 0x0c))
        .unwrap_or(value.len());
    let trimmed = value.get(start..).unwrap_or(&[]);
    let (mut seen_digit, mut seen_dot, mut seen_exp) = (false, false, false);
    let mut end = 0;
    while let Some(c) = trimmed.get(end).copied() {
        let prev = end.checked_sub(1).and_then(|p| trimmed.get(p)).copied();
        match c {
            b'0'..=b'9' => seen_digit = true,
            b'+' | b'-' if end == 0 || matches!(prev, Some(b'e' | b'E' | b'd' | b'D')) => {}
            b'.' if !seen_dot && !seen_exp => seen_dot = true,
            b'e' | b'E' | b'd' | b'D' if seen_digit && !seen_exp => seen_exp = true,
            _ => break,
        }
        end += 1;
    }
    if !seen_digit {
        return 0.0;
    }
    // An unfinished exponent leaves the mantissa in effect. The linked CRT also accepts
    // d/D for the same decimal exponent.
    let mut prefix: Vec<u8> = trimmed.get(..end).unwrap_or(&[]).to_vec();
    while prefix
        .last()
        .is_some_and(|b| matches!(b, b'e' | b'E' | b'd' | b'D' | b'+' | b'-'))
    {
        prefix.pop();
    }
    for b in &mut prefix {
        if matches!(*b, b'd' | b'D') {
            *b = b'e';
        }
    }
    // Overflow gives signed infinity, not failed-text zero; the accessor returns the
    // binary64 result without checking status.
    std::str::from_utf8(&prefix)
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0)
}

/// The one authored-float to 16.16 conversion: multiply by 65536 and truncate toward
/// zero into a signed 64-bit value, keeping its low 32 bits.
///
/// PROVISIONAL: Go delegated this to `internal/sim/numeric` (`TruncateFloat64ToLow32`),
/// which is unavailable. Inferred from the upstream tests: values representable in an
/// i64 truncate and keep the low word (`32768.0` gives `i32::MIN`, `-32768.5` gives
/// `2147450880`); infinity, NaN and anything outside the i64 range give the x86
/// "integer indefinite" value 0x8000_0000_0000_0000, whose low word is 0.
pub fn fixed_from_authored(value: f64) -> i32 {
    // PROVISIONAL: number parsing pending specs/files.md and the files gate.
    let scaled = value * 65536.0;
    // 2^63 as f64 is exact; the valid i64 range is [-2^63, 2^63).
    const LIMIT: f64 = 9_223_372_036_854_775_808.0;
    if scaled.is_nan() || scaled >= LIMIT || scaled < -LIMIT {
        return 0;
    }
    // In range, so `as` is an exact truncation toward zero; the second cast keeps the
    // low 32 bits.
    (scaled as i64) as i32
}

/// Overwrites comment spans with ASCII spaces, preserving every offset. Newlines inside
/// block comments are kept so line numbers stay meaningful.
///
/// `//` blanks to end of line; `/* */` blanks the span. An unterminated block retains
/// its final body byte for the grammar; an opener with no body has both delimiter bytes
/// blanked. Comments cannot appear inside a value, and trailing comments after `;` are
/// blanked too: both follow from blanking before the grammar runs.
fn blank_comments(src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    let len = out.len();
    let mut i = 0;
    while i < len {
        let here = out.get(i).copied();
        let next = out.get(i + 1).copied();
        if here != Some(b'/') {
            i += 1;
            continue;
        }
        match next {
            Some(b'/') => {
                let mut j = i;
                while j < len && out.get(j).copied() != Some(b'\n') {
                    if let Some(b) = out.get_mut(j) {
                        *b = b' ';
                    }
                    j += 1;
                }
                i += 1;
            }
            Some(b'*') => {
                let mut end = len;
                // The retail unfinished-block walk leaves its final body byte. An opener
                // alone has no body; both delimiter bytes are blanked.
                if i + 2 < end {
                    end -= 1;
                }
                let mut j = i + 2;
                while j + 1 < len {
                    if out.get(j).copied() == Some(b'*') && out.get(j + 1).copied() == Some(b'/') {
                        end = j + 2;
                        break;
                    }
                    j += 1;
                }
                for k in i..end {
                    if let Some(b) = out.get_mut(k) {
                        if *b != b'\n' {
                            *b = b' ';
                        }
                    }
                }
                // Go: `i = end - 1` then the loop's `i++`.
                i = end;
            }
            _ => i += 1,
        }
    }
    out
}

/// One section being built: its items so far, and for each fold-equal key a run of
/// indices into `items` stored oldest first (so the run head is the last element).
struct Open {
    section: Section,
    runs: BTreeMap<Vec<u8>, Vec<usize>>,
}

impl Open {
    fn new(section: Section) -> Open {
        Open {
            section,
            runs: BTreeMap::new(),
        }
    }

    fn push_assignment(&mut self, assignment: Assignment) {
        let index = self.section.items.len();
        let run = self.runs.entry(assignment.key.clone()).or_default();
        // Compare byte for byte against the run head only; an identical spelling
        // replaces it in place, anything else goes in front of the run.
        let head_matches = run
            .last()
            .and_then(|&head| self.section.items.get(head))
            .is_some_and(|item| {
                matches!(item, Item::Assignment(a) if a.original_key == assignment.original_key)
            });
        if head_matches {
            if let Some(slot) = run.last_mut() {
                *slot = index;
            }
        } else {
            run.push(index);
        }
        self.section.items.push(Item::Assignment(assignment));
    }

    /// Flatten the runs into the resolved vector: key order, newest first within a run.
    fn finish(mut self) -> Section {
        let mut resolved = Vec::new();
        for run in self.runs.values() {
            resolved.extend(run.iter().rev().copied());
        }
        self.section.resolved = resolved;
        self.section
    }
}

struct Parser<'a> {
    data: &'a [u8],
    pos: usize,
    line: usize,
    column: usize,
    items: usize,
    limits: Limits,
    max_depth: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.data.get(self.pos.checked_add(offset)?).copied()
    }

    fn advance(&mut self) {
        let Some(b) = self.peek() else {
            return;
        };
        if b == b'\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        self.pos += 1;
    }

    fn diag(&self, diagnostic: ParseDiagnostic, section: Option<&Section>) -> Error {
        let name = section
            .map(|s| s.original_name.as_slice())
            .filter(|n| !n.is_empty())
            .unwrap_or(b"root");
        Error::Parse(ParseError {
            diagnostic,
            section: name.to_vec(),
            file: String::new(),
            offset: self.pos,
            line: self.line,
            column: self.column,
        })
    }

    /// Parses the whole document with an explicit stack of open sections, so nesting
    /// depth never becomes native recursion. The base of the stack is the root.
    fn run(&mut self) -> Result<Section, Error> {
        let mut stack = vec![Open::new(Section::default())];
        loop {
            self.skip_space_and_comments();
            let Some(byte) = self.peek() else {
                if stack.len() > 1 {
                    return Err(
                        self.diag(ParseDiagnostic::NextBlock, stack.last().map(|o| &o.section))
                    );
                }
                break;
            };
            match byte {
                b'}' => {
                    self.advance();
                    // At the top level a stray terminator ends the parse silently;
                    // anything after it is ignored.
                    if stack.len() == 1 {
                        break;
                    }
                    let Some(closed) = stack.pop() else { break };
                    let child = closed.finish();
                    // Host compatibility extension: consume an adjacent semicolon after
                    // a section. Retail resumes immediately after the brace.
                    if self.peek() == Some(b';') {
                        self.advance();
                    }
                    self.count_item()?;
                    match stack.last_mut() {
                        Some(parent) => parent.section.items.push(Item::Section(child)),
                        None => break,
                    }
                }
                b'[' => {
                    let child = self
                        .parse_section_header(stack.last().map(|o| &o.section), stack.len() - 1)?;
                    stack.push(Open::new(child));
                }
                _ => {
                    let assignment = self.parse_assignment(stack.last().map(|o| &o.section))?;
                    self.count_item()?;
                    if let Some(top) = stack.last_mut() {
                        top.push_assignment(assignment);
                    }
                }
            }
        }
        // Reached only with the root on the stack; a nested section would have
        // returned NextBlock above.
        stack.pop().map(Open::finish).ok_or(Error::InvalidLimits)
    }

    /// Counts one item against the document limit. Go checks this when the item is
    /// appended, which for a section is after its whole body has parsed.
    fn count_item(&mut self) -> Result<(), Error> {
        if self.items >= self.limits.max_items {
            return Err(Error::ItemLimit {
                limit: self.limits.max_items,
                line: self.line,
                column: self.column,
            });
        }
        self.items += 1;
        Ok(())
    }

    /// Parses `[name] {` and returns the new, empty section. `depth` is the number of
    /// sections already open (not counting the root).
    fn parse_section_header(
        &mut self,
        parent: Option<&Section>,
        depth: usize,
    ) -> Result<Section, Error> {
        if depth >= self.max_depth {
            return Err(Error::DepthLimit {
                limit: self.max_depth,
                line: self.line,
                column: self.column,
            });
        }
        let (line, column) = (self.line, self.column);
        self.advance(); // '['
        let start = self.pos;
        while self.peek().is_some_and(|b| b != b']') {
            self.advance();
        }
        if self.pos >= self.data.len() {
            return Err(self.diag(ParseDiagnostic::ClosingBracket, parent));
        }
        let original = trim_semantic(self.data.get(start..self.pos).unwrap_or(&[])).to_vec();
        if self.pos - start > self.limits.max_name_bytes {
            return Err(Error::SectionNameTooLong {
                limit: self.limits.max_name_bytes,
                line: self.line,
                column: self.column,
            });
        }
        self.advance(); // ']'
        self.skip_space_and_comments();
        if self.peek() != Some(b'{') {
            return Err(self.diag(ParseDiagnostic::OpeningBrace, parent));
        }
        self.advance();
        Ok(Section {
            name: fold_name(&original),
            original_name: original,
            line,
            column,
            items: Vec::new(),
            resolved: Vec::new(),
        })
    }

    fn parse_assignment(&mut self, section: Option<&Section>) -> Result<Assignment, Error> {
        let (line, column) = (self.line, self.column);
        let start = self.pos;
        while self.peek().is_some_and(|b| b != b'=') {
            self.advance();
        }
        if self.pos >= self.data.len() {
            return Err(self.diag(ParseDiagnostic::EqualsNotFound, section));
        }
        let original_key = trim_semantic(self.data.get(start..self.pos).unwrap_or(&[])).to_vec();
        if self.pos - start > self.limits.max_name_bytes {
            return Err(Error::KeyTooLong {
                limit: self.limits.max_name_bytes,
                line: self.line,
                column: self.column,
            });
        }
        self.advance(); // '='
        let value = self.read_value(section)?;
        Ok(Assignment {
            key: fold_name(&original_key),
            original_key,
            value,
            line,
            column,
        })
    }

    /// Reads a field value. The terminator is the next `;` found by a forward scan to
    /// the end of the text: a newline is not a terminator, so a field whose `;` is
    /// missing swallows every following line into its value. Only running out of text
    /// without a `;` is the fatal `Data field - ';' not found`.
    fn read_value(&mut self, section: Option<&Section>) -> Result<Vec<u8>, Error> {
        let mut value: Vec<u8> = Vec::new();
        while let Some(b) = self.peek() {
            if b == b';' {
                self.advance();
                return Ok(trim_semantic(&value).to_vec());
            }
            // The two comment branches are a safety net: `blank_comments` has already
            // removed every comment, so they only run for pre-blanked callers.
            if b == b'/' && self.peek_at(1) == Some(b'/') {
                self.advance();
                self.advance();
                while self.peek().is_some_and(|c| c != b'\n') {
                    self.advance();
                }
                value.push(b' ');
                continue;
            }
            if b == b'/' && self.peek_at(1) == Some(b'*') {
                self.advance();
                self.advance();
                while let Some(c) = self.peek() {
                    if c == b'*' && self.peek_at(1) == Some(b'/') {
                        self.advance();
                        self.advance();
                        break;
                    }
                    self.advance();
                }
                // Go quirk kept: the error is skipped when the text happens to end in
                // "*/" even though this comment never closed (e.g. "/*/").
                if self.pos >= self.data.len() && !self.data.ends_with(b"*/") {
                    return Err(Error::UnterminatedValueComment {
                        line: self.line,
                        column: self.column,
                    });
                }
                value.push(b' ');
                continue;
            }
            if value.len() >= self.limits.max_value_bytes {
                return Err(Error::ValueTooLong {
                    limit: self.limits.max_value_bytes,
                    line: self.line,
                    column: self.column,
                });
            }
            value.push(b);
            self.advance();
        }
        Err(self.diag(ParseDiagnostic::SemicolonMissing, section))
    }

    fn skip_space_and_comments(&mut self) {
        while let Some(b) = self.peek() {
            if is_semantic_space(b) {
                self.advance();
                continue;
            }
            if b == b'/' && self.peek_at(1) == Some(b'/') {
                self.advance();
                self.advance();
                while self.peek().is_some_and(|c| c != b'\n') {
                    self.advance();
                }
                continue;
            }
            if b == b'/' && self.peek_at(1) == Some(b'*') {
                self.advance();
                self.advance();
                let mut closed = false;
                while let Some(c) = self.peek() {
                    if c == b'*' && self.peek_at(1) == Some(b'/') {
                        self.advance();
                        self.advance();
                        closed = true;
                        break;
                    }
                    self.advance();
                }
                if !closed {
                    return;
                }
                continue;
            }
            break;
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    fn parse(text: impl AsRef<[u8]>) -> Document {
        Document::parse(text.as_ref()).unwrap()
    }

    fn first_section(doc: &Document) -> &Section {
        doc.root().sections().next().unwrap()
    }

    fn parse_err(text: &str) -> ParseError {
        match Document::parse(text.as_bytes()) {
            Err(Error::Parse(p)) => p,
            other => panic!("want a parse diagnostic for {text:?}, got {other:?}"),
        }
    }

    // ---- tdf_access_test.go ----

    #[test]
    fn parse_integer_uses_retail_decimal_prefix() {
        let cases: [(&str, &[u8], i32); 7] = [
            ("whitespace and negative sign", b" \t\r\n-12tail", -12),
            ("positive sign", b"+7", 7),
            ("empty", b"", 0),
            ("non numeric", b"true", 0),
            ("hex stops after decimal zero", b"0x10", 0),
            ("trailing junk", b"42abc", 42),
            ("wraps to 32 bits", b"4294967297", 1),
        ];
        for (name, raw, want) in cases {
            assert_eq!(parse_tdf_integer(raw), want, "{name}");
        }
    }

    #[test]
    fn accessors_use_retail_duplicate_and_numeric_semantics() {
        let doc = parse(
            "\n[DUP] {\n    value=first;\n    value=last;\n    empty=;\n    number=  -12junk;\n    hex=0x10;\n    textual=true;\n    numeric=2trailing;\n}\n[DUP] { value=other-section; }\n",
        );
        let section = doc.root().section("dup").unwrap();
        assert_eq!(section.last_value("value"), Some(&b"last"[..]));
        assert_eq!(section.int("empty"), Some(0));
        assert_eq!(section.int("number"), Some(-12));
        assert_eq!(section.int("hex"), Some(0));
        assert_eq!(section.bool("textual"), Some(false));
        assert_eq!(section.bool("numeric"), Some(true));
        assert_eq!(section.int("missing"), None);
        // Duplicate sibling sections are retained; the first-match accessor takes the first.
        assert_eq!(doc.root().sections_named("DUP").len(), 2);
        assert_eq!(section.first_value("value"), Some(&b"last"[..]));
    }

    #[test]
    fn duplicate_key_policy() {
        let doc = parse("[A]\n{\nkey=1;\nkey=2;\nKEY=3;\n}\n");
        let sec = first_section(&doc);
        // Three authored records collapse to two distinct spellings, newest first.
        assert_eq!(sec.values("key"), vec![&b"3"[..], &b"2"[..]]);
        // Head of the fold run: KEY was parsed last, so it was inserted in front.
        assert_eq!(sec.first_value("key"), Some(&b"3"[..]));
        assert_eq!(sec.int_value("key", -99), 3);
        // last_value is the upper bound of the run.
        assert_eq!(sec.last_value("key"), Some(&b"2"[..]));
        assert_eq!(sec.int("key"), Some(3));
    }

    #[test]
    fn identical_spelling_last_wins() {
        let doc = parse("[C]\n{\nk=1;\nk=2;\nz=5;\n}\n");
        let sec = first_section(&doc);
        assert_eq!(sec.first_value("k"), Some(&b"2"[..]));
        // The replaced assignment still exists in source order.
        assert_eq!(sec.assignments().count(), 3);
        assert_eq!(sec.resolved_assignments().count(), 2);
    }

    #[test]
    fn case_variant_last_parsed_wins() {
        let doc = parse("[UNITINFO]\n{\nMaxWaterDepth=0;\nMaxSlope=10;\nmaxwaterdepth=255;\n}\n");
        let sec = first_section(&doc);
        assert_eq!(sec.int_value("maxwaterdepth", -1), 255);
        assert_eq!(sec.int_value("MaxWaterDepth", -1), 255);
        assert_eq!(sec.values("maxwaterdepth"), vec![&b"255"[..], &b"0"[..]]);
        assert_eq!(sec.int_value("maxslope", -1), 10);
    }

    #[test]
    fn case_variant_reversed_order() {
        let doc = parse("[A]\n{\nmaxwaterdepth=255;\nMaxWaterDepth=0;\n}\n");
        assert_eq!(first_section(&doc).int_value("maxwaterdepth", -1), 0);
    }

    #[test]
    fn repeated_spelling_behind_a_variant_is_inserted_not_replaced() {
        let doc = parse("[A]\n{\nk=1;\nK=2;\nk=3;\n}\n");
        let sec = first_section(&doc);
        assert_eq!(sec.int_value("k", -1), 3);
        assert_eq!(
            sec.values("k"),
            vec![&b"3"[..], &b"2"[..], &b"1"[..]],
            "the head comparison only sees the run head, so `k` keeps two entries"
        );
    }

    // ---- tdf_semantics_test.go ----

    #[test]
    fn semantic_values_trim_only_authored_whitespace() {
        let doc = parse(
            "[ A ] {\n  spaced = \t\r\n value with interior  spaces \n\r\t ;\n  empty = \t\r\n ;\n  vertical = \x0b retained \x0c ;\n}\n",
        );
        let section = doc.root().section("a").unwrap();
        for (key, want) in [
            ("spaced", &b"value with interior  spaces"[..]),
            ("empty", b""),
            ("vertical", b"\x0b retained \x0c"),
        ] {
            assert_eq!(section.string_value(key, b"missing"), (want, true), "{key}");
        }
    }

    #[test]
    fn trim_retains_present_empty_key() {
        let doc = parse("[A]{ \t\r\n = value; }");
        let section = doc.root().section("A").unwrap();
        assert_eq!(section.first_value(""), Some(&b"value"[..]));
    }

    #[test]
    fn byte_names_preserve_high_bytes_and_ascii_case() {
        let doc = parse(b"[\xc0]\n{\n\xc0=one;\n\xe0=two;\nAscii=first;\nascii=last;\n}\n");
        let section = doc.root().section(b"\xc0").unwrap();
        assert_eq!(section.first_value(b"\xc0"), Some(&b"one"[..]));
        assert_eq!(section.first_value(b"\xe0"), Some(&b"two"[..]));
        assert_eq!(section.first_value("ASCII"), Some(&b"last"[..]));
    }

    #[test]
    fn grammar_scans_forward_and_top_level_terminator_stops() {
        let doc = parse("[ multi\nline ] \n {\n key[ still } = value;\n}\n}\n[ignored]{key=no;}");
        assert_eq!(doc.root().sections().count(), 1);
        let section = doc.root().section("multi\nline").unwrap();
        assert_eq!(section.first_value("key[ still }"), Some(&b"value"[..]));
    }

    #[test]
    fn diagnostics_carry_exact_reason_section_and_logical_file() {
        let cases = [
            (
                "missing equals",
                "[A]{ field }",
                ParseDiagnostic::EqualsNotFound,
                "A",
            ),
            (
                "missing semicolon",
                "[A]{ field=value",
                ParseDiagnostic::SemicolonMissing,
                "A",
            ),
            (
                "missing closing bracket",
                "[A",
                ParseDiagnostic::ClosingBracket,
                "root",
            ),
            (
                "missing opening brace",
                "[A] field=value;",
                ParseDiagnostic::OpeningBrace,
                "root",
            ),
            (
                "unterminated section",
                "[A]{ field=value;",
                ParseDiagnostic::NextBlock,
                "A",
            ),
        ];
        for (name, input, diagnostic, section) in cases {
            let logical = "gamedata/authored.tdf";
            let err = Document::parse(input.as_bytes())
                .unwrap_err()
                .with_file(logical);
            let Error::Parse(parse_err) = &err else {
                panic!("{name}: want ParseError, got {err:?}");
            };
            assert_eq!(parse_err.diagnostic, diagnostic, "{name}");
            let want = format!(
                "{PARSE_ERROR_TITLE} {} - name = '{section}' from file {logical}",
                diagnostic.message()
            );
            assert_eq!(parse_err.to_string(), want, "{name}");
            assert_eq!(err.to_string(), want, "{name}");
            assert!(err.parse_error().is_some());
        }
    }

    #[test]
    fn comment_blanking_keeps_diagnostic_offset() {
        let data = "// comment\n[A]{ value";
        let err = parse_err(data);
        assert_eq!(err.diagnostic, ParseDiagnostic::EqualsNotFound);
        assert_eq!(err.offset, data.len());
    }

    #[test]
    fn semantic_trim_does_not_use_unicode_or_vertical_whitespace() {
        let doc = parse("[A]{\n\x0bkey=1;\n}\n");
        let section = doc.root().section("A").unwrap();
        assert_eq!(section.first_value("key"), None);
        assert_eq!(section.first_value("\x0bkey"), Some(&b"1"[..]));
    }

    #[test]
    fn diagnostic_does_not_drop_reason() {
        let err = Document::parse(b"[A]{broken}").unwrap_err();
        assert!(err
            .to_string()
            .contains(ParseDiagnostic::EqualsNotFound.message()));
    }

    // ---- tdf_typed_test.go ----

    #[test]
    fn typed_accessors() {
        let doc = parse("[A] { i=  -12abc; f=2.5xyz; fx=1.5; empty=; junk=zzz; }");
        let section = doc.root().section("a").unwrap();
        assert_eq!(section.int_value("i", 99), -12, "trailing junk ignored");
        assert_eq!(
            section.int_value("missing", 99),
            99,
            "absent returns default"
        );
        assert_eq!(section.int_value("junk", 99), 0, "unparsable returns 0");
        assert_eq!(section.float_value("f", 9.0), 2.5);
        // The fixed-point default is stored verbatim: it is already 16.16.
        assert_eq!(section.fixed_value("fx", 65536), 98304);
        assert_eq!(section.fixed_value("missing", 65536), 65536);
        assert_eq!(section.string_value("empty", b"fallback"), (&b""[..], true));
        assert_eq!(
            section.string_value("missing", b"fallback"),
            (&b"fallback"[..], false)
        );
        assert!(section.bool_value("i", false));
        assert!(!section.bool_value("missing", false));
        assert_eq!(section.raw_value("empty"), Some(&b""[..]));
        assert_eq!(section.float("f"), Some(2.5));
        assert_eq!(section.float("missing"), None);
    }

    #[test]
    fn language_string_prefers_prefixed_key() {
        let doc = parse("[A] { germandescription=hallo; description=hello; other=x; }");
        let section = doc.root().section("a").unwrap();
        assert_eq!(
            section.language_string("german", "description", b"d"),
            (&b"hallo"[..], true)
        );
        assert_eq!(
            section.language_string("french", "description", b"d"),
            (&b"hello"[..], true)
        );
        assert_eq!(
            section.language_string("", "description", b"d"),
            (&b"hello"[..], true)
        );
        assert_eq!(
            section.language_string("german", "nope", b"d"),
            (&b"d"[..], false)
        );
    }

    #[test]
    fn fixed_from_authored_truncates() {
        assert_eq!(fixed_from_authored(-1.5), -98304);
        assert_eq!(fixed_from_authored(0.000_000_1), 0);
    }

    #[test]
    fn fixed_from_authored_retains_signed_64_low_word() {
        let cases: [(f64, i32); 4] = [
            (32768.0, -2_147_483_648),
            (65536.0, 0),
            (-32768.5, 2_147_450_880),
            (f64::INFINITY, 0),
        ];
        for (input, want) in cases {
            assert_eq!(fixed_from_authored(input), want, "{input}");
        }
        // Not in the upstream tests: other out-of-range inputs (PROVISIONAL).
        assert_eq!(fixed_from_authored(f64::NAN), 0);
        assert_eq!(fixed_from_authored(f64::NEG_INFINITY), 0);
        assert_eq!(fixed_from_authored(1e300), 0);
    }

    #[test]
    fn float_decimal_scanner_edges() {
        let cases: [(&[u8], f64); 16] = [
            (b"\x0b\x0c -1.25tail", -1.25),
            (b"1.25D2", 125.0),
            (b"5d-1", 0.5),
            (b".5E+1junk", 5.0),
            (b"12e", 12.0),
            (b"12d-", 12.0),
            (b"12E+oops", 12.0),
            (b"12e+-3", 12.0),
            (b"12-3", 12.0),
            (b"-junk", 0.0),
            (b"+.", 0.0),
            (b"Inf", 0.0),
            (b"NaN", 0.0),
            (b"0x1p2", 0.0),
            (b"1.#INF", 1.0),
            (b"1,5", 1.0),
        ];
        for (text, want) in cases {
            let got = parse_tdf_float(text);
            assert_eq!(got.to_bits(), want.to_bits(), "{:?} gave {got}", text);
        }
    }

    #[test]
    fn float_retains_range_result_and_zero_sign() {
        let cases: [(&[u8], f64); 8] = [
            (b"1e400", f64::INFINITY),
            (b"-1D400", f64::NEG_INFINITY),
            (b"1e99999", f64::INFINITY),
            (b"-1e99999", f64::NEG_INFINITY),
            (b"1e-400", 0.0),
            (b"-1e-99999", -0.0),
            (b"-0", -0.0),
            (b"-0e99999", -0.0),
        ];
        for (text, want) in cases {
            let got = parse_tdf_float(text);
            assert_eq!(got.to_bits(), want.to_bits(), "{:?} gave {got}", text);
        }
    }

    #[test]
    fn float_extra_edges_for_the_provisional_converter() {
        // Beyond the upstream cases: pin what the Rust std converter does so a change
        // is noticed. None of these are retail-verified.
        assert_eq!(parse_tdf_float(b"1.e2"), 100.0);
        assert_eq!(parse_tdf_float(b"5."), 5.0);
        assert_eq!(parse_tdf_float(b"-.5"), -0.5);
        assert_eq!(parse_tdf_float(b"1e99999999999999999999999"), f64::INFINITY);
        assert_eq!(
            parse_tdf_float(b"1e-99999999999999999999999").to_bits(),
            0.0f64.to_bits()
        );
        assert_eq!(parse_tdf_float(b""), 0.0);
    }

    // ---- tdf_diag_test.go ----

    #[test]
    fn comment_blanking_preserves_offsets() {
        let cases = [
            ("line", "a=1; // note\nb=2;", "a=1;        \nb=2;"),
            ("block", "a=/*x*/1;", "a=     1;"),
            ("block with newline", "a=1;/*\n*/b=2;", "a=1;  \n  b=2;"),
            (
                "unterminated retains final body byte",
                "a=1; /* trailing",
                "a=1;           g",
            ),
            ("trailing opener", "a=1;/*", "a=1;  "),
            ("one body byte", "/*x", "  x"),
            ("slash alone", "a=1/2;", "a=1/2;"),
        ];
        for (name, input, want) in cases {
            let got = blank_comments(input.as_bytes());
            assert_eq!(got.len(), input.len(), "{name}: length changed");
            assert_eq!(String::from_utf8(got).unwrap(), want, "{name}");
        }
    }

    #[test]
    fn parse_diagnostics_verbatim() {
        let cases = [
            (
                "missing equals",
                "[A] { key }",
                ParseDiagnostic::EqualsNotFound,
            ),
            (
                "missing semicolon",
                "[A] { key=value",
                ParseDiagnostic::SemicolonMissing,
            ),
            (
                "missing closing bracket",
                "[A\n",
                ParseDiagnostic::ClosingBracket,
            ),
            (
                "missing opening brace",
                "[A] key=1;",
                ParseDiagnostic::OpeningBrace,
            ),
            (
                "unterminated block",
                "[A] { key=1;",
                ParseDiagnostic::NextBlock,
            ),
        ];
        for (name, input, want) in cases {
            let err = parse_err(input);
            assert_eq!(err.diagnostic, want, "{name}");
            assert!(err.to_string().starts_with(PARSE_ERROR_TITLE), "{name}");
            let detail = err.with_file("gamedata/example.tdf").detail();
            assert!(detail.starts_with(" - "), "{name}: {detail}");
            assert!(
                detail.contains(" from file gamedata/example.tdf"),
                "{name}: {detail}"
            );
        }
        assert_eq!(
            ParseDiagnostic::NextBlock.message(),
            "End of file - nextblock not zero"
        );
    }

    #[test]
    fn duplicate_key_policy_upper_bound() {
        let doc = parse("[A] { key=1; key=2; KEY=3; }");
        let section = doc.root().section("a").unwrap();
        assert!(!section.values("key").is_empty());
        assert_eq!(section.last_value("key"), Some(&b"2"[..]));
    }

    #[test]
    fn unterminated_comment_retains_grammar_byte() {
        let err = parse_err("/*x");
        assert_eq!(err.diagnostic, ParseDiagnostic::EqualsNotFound);
        let doc = parse("[A]{x=1;/*}");
        let section = doc.root().section("A").unwrap();
        assert_eq!(section.int_value("x", 0), 1);
    }

    // ---- additions: behaviour Go left implicit, and limits ----

    #[test]
    fn empty_and_comment_only_documents_parse_to_an_empty_root() {
        assert_eq!(parse("").root().items().len(), 0);
        assert_eq!(
            parse("  // nothing\n /* at all */ ").root().items().len(),
            0
        );
    }

    #[test]
    fn fbi_style_unitinfo_parses_with_trailing_comments() {
        let doc = parse(
            "[UNITINFO]\n\t{\n\tUnitName=ARMCOM;\t// the commander\n\tMaxVelocity=1.25; /* pixels */\n\tMovementClass=KBOTDS2;\n\t}\n",
        );
        let unit = doc.root().section("unitinfo").unwrap();
        assert_eq!(unit.string_value("unitname", b""), (&b"ARMCOM"[..], true));
        assert_eq!(unit.fixed_value("maxvelocity", 0), 81920);
        assert_eq!(unit.original_name(), b"UNITINFO");
        assert_eq!(unit.name(), b"unitinfo");
    }

    #[test]
    fn semicolon_after_section_is_consumed() {
        let doc = parse("[A]{x=1;};[B]{y=2;}");
        assert_eq!(doc.root().sections().count(), 2);
    }

    #[test]
    fn nested_sections_and_positions() {
        let doc = parse("[A]\n{\n  [B] { c=1; }\n  d=2;\n}\n");
        let a = doc.root().section("a").unwrap();
        let b = a.section("b").unwrap();
        assert_eq!((a.line(), a.column()), (1, 1));
        assert_eq!((b.line(), b.column()), (3, 3));
        assert_eq!(b.int_value("c", 0), 1);
        assert_eq!(a.int_value("d", 0), 2);
        let c = b.assignments().next().unwrap();
        assert_eq!((c.line, c.column), (3, 9));
    }

    #[test]
    fn missing_semicolon_swallows_following_lines() {
        let doc = parse("[A]{ a=1\nb=2;\nc=3; }");
        let section = doc.root().section("a").unwrap();
        assert_eq!(section.first_value("a"), Some(&b"1\nb=2"[..]));
        assert_eq!(section.first_value("b"), None);
        assert_eq!(section.int_value("c", 0), 3);
    }

    #[test]
    fn resolved_order_is_by_folded_key() {
        let doc = parse("[A]{ b=1; A=2; a=3; C=4; }");
        let keys: Vec<&[u8]> = doc
            .root()
            .section("a")
            .unwrap()
            .resolved_assignments()
            .map(|a| a.original_key.as_slice())
            .collect();
        assert_eq!(keys, vec![&b"a"[..], b"A", b"b", b"C"]);
    }

    #[test]
    fn limits_are_enforced() {
        let limits = Limits {
            max_bytes: 8,
            ..Limits::default()
        };
        assert!(matches!(
            Document::parse_with_limits(b"[A]{x=1;}", limits),
            Err(Error::DocumentTooLarge { size: 9, limit: 8 })
        ));
        let zero = Limits {
            max_items: 0,
            ..Limits::default()
        };
        assert_eq!(
            Document::parse_with_limits(b"", zero),
            Err(Error::InvalidLimits)
        );

        let items = Limits {
            max_items: 2,
            ..Limits::default()
        };
        assert!(Document::parse_with_limits(b"a=1;b=2;", items).is_ok());
        assert!(matches!(
            Document::parse_with_limits(b"a=1;b=2;c=3;", items),
            Err(Error::ItemLimit { limit: 2, .. })
        ));
        // A section counts as one item, after its body.
        assert!(matches!(
            Document::parse_with_limits(b"[A]{x=1;y=2;}", items),
            Err(Error::ItemLimit { limit: 2, .. })
        ));

        let depth = Limits {
            max_depth: 2,
            ..Limits::default()
        };
        assert!(Document::parse_with_limits(b"[A]{[B]{x=1;}}", depth).is_ok());
        assert!(matches!(
            Document::parse_with_limits(b"[A]{[B]{[C]{x=1;}}}", depth),
            Err(Error::DepthLimit { limit: 2, .. })
        ));

        let value = Limits {
            max_value_bytes: 3,
            ..Limits::default()
        };
        assert!(Document::parse_with_limits(b"a=abc;", value).is_ok());
        assert!(matches!(
            Document::parse_with_limits(b"a=abcd;", value),
            Err(Error::ValueTooLong { limit: 3, .. })
        ));

        let name = Limits {
            max_name_bytes: 2,
            ..Limits::default()
        };
        assert!(matches!(
            Document::parse_with_limits(b"abc=1;", name),
            Err(Error::KeyTooLong { limit: 2, .. })
        ));
        assert!(matches!(
            Document::parse_with_limits(b"[abc]{}", name),
            Err(Error::SectionNameTooLong { limit: 2, .. })
        ));
    }

    #[test]
    fn deep_nesting_is_bounded_without_recursion() {
        let mut text = String::new();
        for _ in 0..100_000 {
            text.push_str("[a]{");
        }
        let limits = Limits {
            max_depth: usize::MAX,
            ..Limits::default()
        };
        match Document::parse_with_limits(text.as_bytes(), limits) {
            Err(Error::DepthLimit { limit, .. }) => assert_eq!(limit, HARD_MAX_DEPTH),
            other => panic!("want DepthLimit, got {other:?}"),
        }
        // Exactly at the cap parses and drops cleanly.
        let mut ok = "[a]{".repeat(HARD_MAX_DEPTH);
        ok.push_str(&"}".repeat(HARD_MAX_DEPTH));
        assert!(Document::parse_with_limits(ok.as_bytes(), limits).is_ok());
    }

    #[test]
    fn hostile_inputs_never_panic() {
        let inputs: [&[u8]; 12] = [
            b"[",
            b"]",
            b"{",
            b"}",
            b"=",
            b";",
            b"/",
            b"/*",
            b"//",
            b"[a]{",
            b"[a]{b=",
            b"\xff\xfe\x00",
        ];
        for input in inputs {
            let _ = Document::parse(input);
        }
        // Every prefix of a well-formed document.
        let sample = b"[A]{ a=1; /* c */ [B]{ b=2; } // x\n}";
        for end in 0..=sample.len() {
            let _ = Document::parse(&sample[..end]);
        }
    }
}
