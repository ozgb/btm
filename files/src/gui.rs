// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), formats/gui.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! GUI panel definitions: TDF text where every top-level section is a gadget.
//!
//! The first gadget (`GADGET0`) also carries the panel header: `totalgadgets`, `panel`,
//! `crdefault`, `escdefault`, `defaultfocus` and an optional `[VERSION]` subsection.
//! `totalgadgets=N` admits `GADGET0..GADGETN`, N+1 in all; the parser does not enforce
//! that, so a caller can compare [`GuiHeader::total_gadgets`] with `gadgets.len()`.
//!
//! Two recoveries, both bounded:
//! - A retail `SCORE.GUI` ends after its last assignment without the gadget's closing
//!   brace. When the parse fails with the end-of-file diagnostic and the last
//!   non-whitespace byte is `;`, a newline and a `}` are appended and the parse retried
//!   ([`Gui::repaired`]). Any other structural error stays an error.
//! - Binary panel resources (a zero byte, then 0xcd, and the word `HEADER` in the first
//!   256 bytes) are not TDF. When the TDF parse fails on such a file, every printable
//!   run of four or more characters becomes a label gadget ([`Gui::binary`]).
//!
//! Differences from the Go source:
//! - Text is byte strings, because TDF values are. Field maps are ordered maps keyed by
//!   the folded key (Go's map order is unspecified, and nothing relied on it).
//! - Go's `isUnterminatedSection` also matched any error whose text contains
//!   "unterminated section". The Rust TDF parser has no such message, so only the
//!   end-of-file diagnostic is tested.
//! - Go's header lookup tried `"defaultFocus"` after `"defaultfocus"`. Keys are folded to
//!   lower case, so the second spelling can never be present and is not looked up.
//! - Go's `fillCommon` returned an error that its helper `guiInt` never produced (it
//!   mapped every failure and every absent key to zero); `fill_common` is infallible.
//! - Integers are `i64`, which is Go's `int` on 64-bit hosts. Each value read from text
//!   is an `i32` first (the TDF integer wraps at 32 bits), so none is out of range.
//! - [`SectionView`] is built recursively, as in Go. Its depth is bounded by the TDF
//!   parser's nesting limit, which is at most 1024.
//! - `LoadGUIFile` (VFS read) is not ported.

use std::collections::{BTreeMap, HashSet};
use std::fmt;

use crate::tdf::{self, parse_tdf_integer, Document, Limits, Section};

/// Everything that can go wrong reading a panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuiError {
    /// The TDF text did not parse, and was not a binary panel or the recoverable
    /// unterminated-gadget case. After a failed repair this is the retry's error.
    Parse(tdf::Error),
    /// The document has no top-level section.
    NoGadgets,
}

impl fmt::Display for GuiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GuiError::Parse(e) => e.fmt(f),
            GuiError::NoGadgets => write!(f, "gui: no gadgets"),
        }
    }
}

impl std::error::Error for GuiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GuiError::Parse(e) => Some(e),
            GuiError::NoGadgets => None,
        }
    }
}

/// A decoded panel: its gadgets (the first carries the header) and the document they
/// came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gui {
    pub gadgets: Vec<Gadget>,
    pub header: GuiHeader,
    /// The source was a binary resource and the gadgets are guessed labels.
    pub binary: bool,
    /// The source needed the end-of-file repair. [`Gui::document`] is the repaired tree,
    /// so a caller never has to parse the bytes again and reach a different answer.
    pub repaired: bool,
    /// The TDF tree the gadgets came from. A binary panel gets an empty one.
    pub document: Document,
}

/// The panel-wide record the first gadget carries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuiHeader {
    pub total_gadgets: i64,
    pub panel: Vec<u8>,
    pub cr_default: Vec<u8>,
    pub esc_default: Vec<u8>,
    pub default_focus: Vec<u8>,
    pub version_major: i64,
    pub version_minor: i64,
    pub version_rev: i64,
    pub has_version: bool,
}

/// One control of a panel: the common record every gadget has plus the per-kind fields
/// the file authored.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Gadget {
    /// The section name as authored.
    pub source_name: Vec<u8>,
    /// Whether the gadget had a `[COMMON]` subsection. Tells an omitted one from one
    /// whose fields all take their defaults; the caller owns the policy for the omitted
    /// case.
    pub has_common: bool,
    pub common: CommonGadget,
    /// The gadget's own assignments by folded key, then `common.<key>` and
    /// `version.<key>` for the two subsections. Where a key repeats, the later one in
    /// source order replaces the earlier (a map store, as in Go).
    pub fields: BTreeMap<Vec<u8>, Vec<u8>>,
    /// The raw section, with every nested section. `None` for a binary panel's gadgets.
    pub section: Option<SectionView>,
}

/// The record every gadget begins with. A key the file does not author is zero or empty.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommonGadget {
    pub id: i64,
    pub assoc: i64,
    pub name: Vec<u8>,
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
    pub attributes: i64,
    pub color_foreground: i64,
    pub color_background: i64,
    pub texture_number: i64,
    pub font_number: i64,
    pub active: i64,
    pub common_attributes: i64,
    pub gaf_file: i64,
    pub help: Vec<u8>,
}

/// A generic view of a nested section: its assignments and its child sections. Unknown
/// fields stay available.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SectionView {
    /// The section name as authored.
    pub name: Vec<u8>,
    pub fields: BTreeMap<Vec<u8>, Vec<u8>>,
    pub child: Vec<SectionView>,
}

impl Gui {
    /// Decode a panel under the default TDF limits.
    pub fn parse(data: &[u8]) -> Result<Gui, GuiError> {
        Self::parse_with_limits(data, Limits::default())
    }

    /// Decode a panel under explicit TDF limits.
    pub fn parse_with_limits(data: &[u8], limits: Limits) -> Result<Gui, GuiError> {
        let mut repaired = false;
        let document = match Document::parse_with_limits(data, limits) {
            Ok(document) => document,
            Err(_) if is_binary_gui(data) => return Ok(load_binary_gui(data)),
            Err(err) => {
                if !is_unterminated_section(&err) {
                    return Err(GuiError::Parse(err));
                }
                // A retail SCORE.GUI ends after the final assignment without the
                // enclosing gadget brace. Recover only this bounded EOF-tail case.
                let tail = gui_tail(data).and_then(|end| data.get(end));
                if tail != Some(&b';') {
                    return Err(GuiError::Parse(err));
                }
                let mut fixed = Vec::with_capacity(data.len() + 2);
                fixed.extend_from_slice(data);
                fixed.extend_from_slice(b"\n}");
                // A second failure replaces the first, as in Go.
                let document =
                    Document::parse_with_limits(&fixed, limits).map_err(GuiError::Parse)?;
                repaired = true;
                document
            }
        };

        let mut gadgets = Vec::new();
        for section in document.root().sections() {
            let mut gadget = Gadget {
                source_name: section.original_name().to_vec(),
                section: Some(make_section_view(section)),
                ..Gadget::default()
            };
            for item in section.assignments() {
                gadget.fields.insert(item.key.clone(), item.value.clone());
            }
            if let Some(common) = section.section("common") {
                gadget.has_common = true;
                for item in common.assignments() {
                    let mut key = b"common.".to_vec();
                    key.extend_from_slice(&item.key);
                    gadget.fields.insert(key, item.value.clone());
                }
                gadget.common = fill_common(common);
            }
            // Keep the version subsection if present on this gadget (typically GADGET0).
            if let Some(version) = section.section("version") {
                for item in version.assignments() {
                    let mut key = b"version.".to_vec();
                    key.extend_from_slice(&item.key);
                    gadget.fields.insert(key, item.value.clone());
                }
            }
            gadgets.push(gadget);
        }
        let Some(first) = gadgets.first() else {
            return Err(GuiError::NoGadgets);
        };

        // The header lives on GADGET0.
        let mut header = GuiHeader::default();
        if let Some(v) = field(first, "totalgadgets") {
            header.total_gadgets = atoi(v);
        }
        if let Some(v) = field(first, "panel") {
            header.panel = v.to_vec();
        }
        if let Some(v) = field(first, "crdefault") {
            header.cr_default = v.to_vec();
        }
        if let Some(v) = field(first, "escdefault") {
            header.esc_default = v.to_vec();
        }
        if let Some(v) = field(first, "defaultfocus") {
            header.default_focus = v.to_vec();
        }
        if let Some(major) = field(first, "version.major") {
            header.has_version = true;
            header.version_major = atoi(major);
            header.version_minor = field(first, "version.minor").map_or(0, atoi);
            header.version_rev = field(first, "version.revision").map_or(0, atoi);
        }
        // `totalgadgets=N` admits N+1 gadgets. Retail files follow that; a mismatch is
        // tolerated for mods and left for the caller to compare.
        Ok(Gui {
            gadgets,
            header,
            binary: false,
            repaired,
            document,
        })
    }
}

fn field<'a>(gadget: &'a Gadget, key: &str) -> Option<&'a [u8]> {
    gadget.fields.get(key.as_bytes()).map(Vec::as_slice)
}

/// GUI header fields use the same wrapped decimal accessor as COMMON: the 32-bit TDF
/// integer, sign-extended. Absent text is empty, which reads as zero.
fn atoi(value: &[u8]) -> i64 {
    i64::from(parse_tdf_integer(value))
}

fn is_binary_gui(data: &[u8]) -> bool {
    let head = data.get(..data.len().min(256)).unwrap_or(&[]);
    data.len() >= 16
        && data.first() == Some(&0)
        && data.get(1) == Some(&0xcd)
        && head.windows(6).any(|w| w == b"HEADER")
}

/// Index of the last byte that is not space, tab, CR or LF.
fn gui_tail(data: &[u8]) -> Option<usize> {
    data.iter()
        .rposition(|b| !matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
}

/// The bounded EOF-tail malformation retail's own GUI files carry: a gadget section that
/// runs to the end of the file without its closing brace, which the parser reports as
/// the verbatim end-of-file diagnostic.
fn is_unterminated_section(err: &tdf::Error) -> bool {
    err.parse_error()
        .is_some_and(|p| p.diagnostic == tdf::ParseDiagnostic::NextBlock)
}

fn load_binary_gui(data: &[u8]) -> Gui {
    let mut labels = binary_gui_labels(data);
    if labels.is_empty() {
        labels.push(b"binary GUI resource".to_vec());
    }
    let mut gadgets = Vec::with_capacity(labels.len());
    for (index, label) in labels.into_iter().enumerate() {
        let is_header = label.eq_ignore_ascii_case(b"HEADER");
        let mut fields = BTreeMap::new();
        if !is_header {
            fields.insert(b"text".to_vec(), label.clone());
        }
        // `index` is bounded by the input length, so this cannot overflow an i64.
        let row = i64::try_from(index).unwrap_or(i64::MAX / 24);
        gadgets.push(Gadget {
            source_name: format!("BINARY{index}").into_bytes(),
            has_common: true,
            common: CommonGadget {
                id: if is_header { 0 } else { 5 },
                name: label,
                x: 32,
                y: 24 + row * 24,
                width: 576,
                height: 20,
                active: 1,
                ..CommonGadget::default()
            },
            fields,
            section: None,
        });
    }
    Gui {
        gadgets,
        header: GuiHeader::default(),
        binary: true,
        repaired: false,
        // A binary panel has no TDF tree, but callers walk `document` unconditionally.
        document: Document::default(),
    }
}

/// Every run of four or more label characters, trimmed, first occurrence of each.
fn binary_gui_labels(data: &[u8]) -> Vec<Vec<u8>> {
    let mut seen: HashSet<Vec<u8>> = HashSet::new();
    let mut labels = Vec::new();
    let mut start = 0;
    while start < data.len() {
        if !data.get(start).is_some_and(|&b| is_gui_string_start(b)) {
            start += 1;
            continue;
        }
        let mut end = start + 1;
        while data.get(end).is_some_and(|&b| is_gui_string_byte(b)) {
            end += 1;
        }
        if end - start >= 4 {
            let mut label = data.get(start..end).unwrap_or(&[]);
            // Go's `strings.TrimSpace`. A label starts with a letter and holds no
            // whitespace but the space, so only trailing spaces can go.
            while let Some((&b' ', rest)) = label.split_last() {
                label = rest;
            }
            if seen.insert(label.to_vec()) {
                labels.push(label.to_vec());
            }
        }
        start = end;
    }
    labels
}

fn is_gui_string_start(value: u8) -> bool {
    value.is_ascii_alphabetic()
}

fn is_gui_string_byte(value: u8) -> bool {
    is_gui_string_start(value)
        || value.is_ascii_digit()
        || matches!(value, b' ' | b'_' | b'-' | b'!' | b'?' | b'.')
}

/// Read the COMMON record. Strings use the same resolved lookup as every typed field:
/// the last parsed case variant. An absent key, or one with no number in it, is zero.
fn fill_common(section: &Section) -> CommonGadget {
    let int = |key: &str| section.int(key).unwrap_or(0);
    CommonGadget {
        name: section.string_value("name", b"").0.to_vec(),
        help: section.string_value("help", b"").0.to_vec(),
        id: int("id"),
        assoc: int("assoc"),
        x: int("xpos"),
        y: int("ypos"),
        width: int("width"),
        height: int("height"),
        attributes: int("attribs"),
        color_foreground: int("colorf"),
        color_background: int("colorb"),
        texture_number: int("texturenumber"),
        font_number: int("fontnumber"),
        active: int("active"),
        common_attributes: int("commonattribs"),
        gaf_file: int("gaffile"),
    }
}

fn make_section_view(section: &Section) -> SectionView {
    let mut view = SectionView {
        name: section.original_name().to_vec(),
        ..SectionView::default()
    };
    for item in section.items() {
        match item {
            tdf::Item::Assignment(a) => {
                view.fields.insert(a.key.clone(), a.value.clone());
            }
            tdf::Item::Section(child) => view.child.push(make_section_view(child)),
        }
    }
    view
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::indexing_slicing,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic
    )]

    use super::*;

    /// Port of `TestLoadGUIRetainsSectionWithoutCommon`.
    #[test]
    fn retains_section_without_common() {
        let gui = Gui::parse(
            b"
[GADGET0] { [COMMON] { id=0; name=HEADER; } }
[GADGET1] { text=raw-only; [EXTRA] { value=retained; } }
",
        )
        .unwrap();
        assert_eq!(gui.gadgets.len(), 2);
        let got = &gui.gadgets[1];
        assert!(!got.has_common, "GADGET1 unexpectedly has COMMON");
        assert_eq!(
            got.fields.get(b"text".as_slice()),
            Some(&b"raw-only".to_vec())
        );
        let section = got.section.as_ref().unwrap();
        assert_eq!(section.child.len(), 1);
        assert_eq!(section.child[0].name, b"EXTRA");
        assert_eq!(
            section.child[0].fields.get(b"value".as_slice()),
            Some(&b"retained".to_vec())
        );
        assert!(gui.gadgets[0].has_common);
        assert_eq!(gui.gadgets[0].common.name, b"HEADER");
        assert_eq!(
            gui.gadgets[0].fields.get(b"common.name".as_slice()),
            Some(&b"HEADER".to_vec())
        );
        assert!(!gui.binary && !gui.repaired);
    }

    /// Port of `TestLoadGUICommonStringsUseTypedLookup`.
    #[test]
    fn common_strings_use_typed_lookup() {
        let gui = Gui::parse(
            b"
[GADGET0] {
  [COMMON] {
    name=earlier;
    NAME=later;
    help=first help;
    HELP=last help;
  }
}
",
        )
        .unwrap();
        assert_eq!(
            gui.gadgets[0].common.name, b"later",
            "common name should be the last parsed case variant"
        );
        assert_eq!(gui.gadgets[0].common.help, b"last help");
    }

    /// Port of `TestGUIHeaderIntegersUseWrappedDecimalPrefix` (gui_integer_test.go).
    #[test]
    fn header_integers_use_wrapped_decimal_prefix() {
        let panel = Gui::parse(
            b"[GADGET0] {
        [COMMON] { id=0; }
        totalgadgets=18446744073709551617tail;
        [VERSION] { major=4294967299suffix; minor=-4294967297; revision=2147483648; }
    }",
        )
        .unwrap();
        let h = &panel.header;
        assert_eq!(h.total_gadgets, 1);
        assert_eq!(h.version_major, 3);
        assert_eq!(h.version_minor, -1);
        assert_eq!(h.version_rev, -2_147_483_648);
        assert!(h.has_version);
    }

    #[test]
    fn header_fields_and_common_record() {
        let gui = Gui::parse(
            b"[GADGET0]
{
  [COMMON]
  {
    id=0; assoc=7; name=HEADER; xpos=-4; ypos=5; width=640; height=480;
    attribs=3; colorf=11; colorb=12; texturenumber=2; fontnumber=1; active=1;
    commonattribs=9; gaffile=4; help=helpful;
  }
  totalgadgets=1;
  panel=MAIN;
  crdefault=OK;
  escdefault=CANCEL;
  defaultfocus=GO;
}
[GADGET1]
{
  [COMMON] { id=1; }
}
",
        )
        .unwrap();
        let c = &gui.gadgets[0].common;
        assert_eq!(
            (c.id, c.assoc, c.x, c.y, c.width, c.height),
            (0, 7, -4, 5, 640, 480)
        );
        assert_eq!(
            (c.attributes, c.color_foreground, c.color_background),
            (3, 11, 12)
        );
        assert_eq!(
            (
                c.texture_number,
                c.font_number,
                c.active,
                c.common_attributes,
                c.gaf_file
            ),
            (2, 1, 1, 9, 4)
        );
        assert_eq!(c.help, b"helpful");
        let h = &gui.header;
        assert_eq!(h.total_gadgets, 1);
        assert_eq!(h.panel, b"MAIN");
        assert_eq!(h.cr_default, b"OK");
        assert_eq!(h.esc_default, b"CANCEL");
        assert_eq!(h.default_focus, b"GO");
        assert!(!h.has_version);
        assert_eq!(gui.gadgets.len() as i64, h.total_gadgets + 1);
        assert_eq!(gui.gadgets[1].source_name, b"GADGET1");
        // A COMMON with absent keys is all zero.
        assert_eq!(gui.gadgets[1].common.width, 0);
    }

    #[test]
    fn version_without_minor_and_revision_reads_zero() {
        let gui = Gui::parse(b"[GADGET0] { [VERSION] { major=4; } }").unwrap();
        let h = &gui.header;
        assert!(h.has_version);
        assert_eq!((h.version_major, h.version_minor, h.version_rev), (4, 0, 0));
    }

    #[test]
    fn repeated_field_keeps_the_later_value() {
        let gui = Gui::parse(b"[GADGET0] { text=one; TEXT=two; text=three; }").unwrap();
        assert_eq!(
            gui.gadgets[0].fields.get(b"text".as_slice()),
            Some(&b"three".to_vec())
        );
    }

    #[test]
    fn no_gadgets_is_an_error() {
        assert_eq!(Gui::parse(b""), Err(GuiError::NoGadgets));
        assert_eq!(Gui::parse(b"top=level;"), Err(GuiError::NoGadgets));
        assert_eq!(GuiError::NoGadgets.to_string(), "gui: no gadgets");
    }

    #[test]
    fn unterminated_final_gadget_is_repaired() {
        let source = b"[GADGET0]\n{\n  [COMMON]\n  {\n    id=0;\n  }\n  totalgadgets=0;\n";
        // Not repaired: no closing brace, and the text does not end in ';' either.
        let mut ends_with_brace = source.to_vec();
        ends_with_brace.extend_from_slice(b"  [EXTRA]\n  {\n");
        assert!(matches!(
            Gui::parse(&ends_with_brace),
            Err(GuiError::Parse(_))
        ));
        let gui = Gui::parse(source).unwrap();
        assert!(gui.repaired);
        assert!(!gui.binary);
        assert_eq!(gui.gadgets.len(), 1);
        assert_eq!(gui.gadgets[0].common.id, 0);
        assert!(gui.document.root().section("gadget0").is_some());
        // The same bytes, with trailing whitespace, repair the same way.
        let mut spaced = source.to_vec();
        spaced.extend_from_slice(b" \r\n\t\n");
        assert!(Gui::parse(&spaced).unwrap().repaired);
        // A complete file is not flagged.
        let mut closed = source.to_vec();
        closed.extend_from_slice(b"}\n");
        assert!(!Gui::parse(&closed).unwrap().repaired);
    }

    #[test]
    fn other_structural_errors_stay_errors() {
        // A missing semicolon is not the bounded EOF-tail case.
        assert!(matches!(
            Gui::parse(b"[GADGET0]\n{\nid=0\n}\n"),
            Err(GuiError::Parse(_))
        ));
        // A truncated file whose tail is not ';'.
        assert!(matches!(
            Gui::parse(b"[GADGET0]\n{\nid=0;\n[X]\n{\n"),
            Err(GuiError::Parse(_))
        ));
    }

    fn binary_fixture() -> Vec<u8> {
        let mut data = vec![0u8, 0xcd];
        data.extend_from_slice(b"HEADER");
        data.extend_from_slice(&[0xff, 0x01, 0x80]);
        data.extend_from_slice(b"Start Game\x00ab\x00Start Game\x00OK  \x07Quit-Now!\x00");
        data
    }

    #[test]
    fn binary_panel_becomes_label_gadgets() {
        let gui = Gui::parse(&binary_fixture()).unwrap();
        assert!(gui.binary);
        assert!(!gui.repaired);
        assert!(gui.document.root().sections().next().is_none());
        let names: Vec<&[u8]> = gui
            .gadgets
            .iter()
            .map(|g| g.common.name.as_slice())
            .collect();
        // "ab" is too short; the second "Start Game" repeats; "OK  " trims to "OK".
        assert_eq!(names, [&b"HEADER"[..], b"Start Game", b"OK", b"Quit-Now!"]);
        let header = &gui.gadgets[0];
        assert_eq!(header.common.id, 0);
        assert!(header.fields.is_empty(), "the header has no text field");
        assert_eq!(header.source_name, b"BINARY0");
        assert!(header.has_common && header.section.is_none());
        let start = &gui.gadgets[1];
        assert_eq!(start.common.id, 5);
        assert_eq!(
            start.fields.get(b"text".as_slice()),
            Some(&b"Start Game".to_vec())
        );
        assert_eq!(
            (
                start.common.x,
                start.common.y,
                start.common.width,
                start.common.height
            ),
            (32, 24 + 24, 576, 20)
        );
        assert_eq!(start.common.active, 1);
        assert_eq!(gui.gadgets[3].common.y, 24 + 3 * 24);
    }

    /// `HEADER` is itself a label run, so no input that passes the binary test can have
    /// zero labels; the placeholder is Go's fallback and is checked directly.
    #[test]
    fn binary_panel_with_no_labels_gets_a_placeholder() {
        let gui = load_binary_gui(b"");
        assert!(gui.binary);
        assert_eq!(gui.gadgets.len(), 1);
        assert_eq!(gui.gadgets[0].common.name, b"binary GUI resource");
        assert_eq!(gui.gadgets[0].common.id, 5);
        assert_eq!(
            gui.gadgets[0].fields.get(b"text".as_slice()),
            Some(&b"binary GUI resource".to_vec())
        );
        assert_eq!(gui.gadgets[0].common.y, 24);
    }

    #[test]
    fn binary_signature_needs_every_part() {
        // Too short, wrong lead bytes, or HEADER past the first 256 bytes: plain TDF
        // errors, not panels.
        let short = [0u8, 0xcd, b'H', b'E', b'A', b'D', b'E', b'R'];
        assert!(matches!(Gui::parse(&short), Err(GuiError::Parse(_))));
        let mut wrong = binary_fixture();
        wrong[1] = 0xce;
        assert!(matches!(Gui::parse(&wrong), Err(GuiError::Parse(_))));
        let mut late = vec![0u8, 0xcd];
        late.resize(300, 0xff);
        late.extend_from_slice(b"HEADER");
        assert!(matches!(Gui::parse(&late), Err(GuiError::Parse(_))));
    }

    /// Hostile: a file of one repeated label character, and one of many short labels,
    /// must not allocate more than the input holds.
    #[test]
    fn binary_labels_are_bounded_by_the_input() {
        let mut data = vec![0u8, 0xcd];
        data.extend_from_slice(b"HEADER\0");
        data.extend_from_slice(&vec![b'a'; 100_000]);
        let gui = Gui::parse(&data).unwrap();
        assert_eq!(gui.gadgets.len(), 2);
        let mut data = vec![0u8, 0xcd];
        data.extend_from_slice(b"HEADER\0");
        for i in 0..5000 {
            data.extend_from_slice(format!("label{i}\0").as_bytes());
        }
        let gui = Gui::parse(&data).unwrap();
        assert_eq!(gui.gadgets.len(), 5001);
    }

    #[test]
    fn nesting_depth_is_limited_by_the_tdf_parser() {
        let mut text = Vec::new();
        for i in 0..2000 {
            text.extend_from_slice(format!("[S{i}]{{").as_bytes());
        }
        assert!(matches!(Gui::parse(&text), Err(GuiError::Parse(_))));
        // Within the limit, every level is kept.
        let mut text = Vec::new();
        for i in 0..100 {
            text.extend_from_slice(format!("[S{i}]{{").as_bytes());
        }
        text.extend_from_slice(&[b'}'; 100]);
        let gui = Gui::parse(&text).unwrap();
        let mut depth = 0;
        let mut view = gui.gadgets[0].section.as_ref().unwrap();
        while let Some(child) = view.child.first() {
            depth += 1;
            view = child;
        }
        assert_eq!(depth, 99);
    }

    #[test]
    fn every_truncation_is_handled_without_panic() {
        let sample = b"[GADGET0]\n{\n  [COMMON]\n  {\n    id=0;\n  }\n  totalgadgets=1;\n  [VERSION]\n  {\n    major=1;\n  }\n}\n[GADGET1]\n{\n  [COMMON]\n  {\n    id=1;\n  }\n}\n";
        assert!(Gui::parse(sample).is_ok());
        for len in 0..sample.len() {
            let _ = Gui::parse(&sample[..len]);
        }
        let binary = binary_fixture();
        for len in 0..binary.len() {
            let _ = Gui::parse(&binary[..len]);
        }
    }
}
