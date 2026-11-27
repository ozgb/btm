// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), internal/cob/load.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! COB compiled-script files: the loader, not the VM.
//!
//! [`Program::load`] decodes one COB file into an immutable [`Program`]: the code
//! words, the script entry table, the piece names and the static-variable count. It
//! takes bytes and never touches the file system.
//!
//! Layout (all fields little-endian `u32`; offsets are absolute file offsets; code
//! addresses are word indexes into the code section): a 44-byte header of 11 words,
//! then tables the header points at. Header word 0x14 is the record count of a trailing
//! 8-byte record table at word 0x28; the loader only bounds-checks that table.
//!
//! Differences from the Go source, all deliberate:
//! - Names are raw bytes (`Vec<u8>`), as in Go's byte strings. Lookups are
//!   case-sensitive and the first of duplicate names wins; every authored entry stays
//!   reachable by script id.
//! - `Program::scripts` is a `BTreeMap`, not a Go map, so iteration order is
//!   deterministic.
//! - The total size of all script and piece names is capped at the input length.
//!   Go copied each name out separately, so many table entries pointing at one long
//!   string could allocate far more than the input holds. A compiler never shares
//!   name strings, so an honest file never hits the cap.
//! - Input larger than 4 GiB is rejected; Go truncated its length to 32 bits.
//! - Not ported: `LoadFromFS` and `CachedLoader` (VFS access), and the VM-side
//!   `buildAnimation` ranges derived from the bytecode.

use std::collections::BTreeMap;
use std::fmt;

/// Nanolathe's host-safety limit for one VM's static storage, in bytes. It is not a
/// recovered retail limit: COB has no file-backed static data with which to bound its
/// declared count.
pub const MAX_PROGRAM_STATIC_BYTES: usize = 4 << 20;

/// Size of the COB header: 11 little-endian `u32` words.
const HEADER_SIZE: usize = 44;

/// Counts above this are rejected as implausible, and it keeps `count * 4` in `u32`.
const MAX_COUNT: u32 = 1 << 28;

/// Whether a name belongs to a script or a piece.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameKind {
    Script,
    Piece,
}

impl fmt::Display for NameKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            NameKind::Script => "script",
            NameKind::Piece => "piece",
        })
    }
}

/// Everything [`Program::load`] and [`Program::validate`] can fail with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// Input longer than `u32::MAX` bytes; COB offsets cannot address it.
    FileTooLarge {
        len: usize,
    },
    HeaderTooShort {
        len: usize,
    },
    UnsupportedVersion {
        version: u32,
    },
    ImplausibleCounts {
        scripts: u32,
        pieces: u32,
        code_len: u32,
    },
    /// Static storage over [`MAX_PROGRAM_STATIC_BYTES`], in words.
    StaticStorage {
        words: usize,
    },
    TableOffsetBeyondFile {
        what: &'static str,
        offset: u32,
        file_len: u32,
    },
    TableOffsetUnaligned {
        what: &'static str,
        offset: u32,
    },
    TableOverflow {
        what: &'static str,
    },
    TableExceedsFile {
        what: &'static str,
        offset: u32,
        need: u32,
        file_len: u32,
    },
    TrailingRecordsOffsetBeyondFile {
        offset: u32,
        file_len: u32,
    },
    ImplausibleTrailingCount {
        count: u32,
    },
    TrailingRecordsExceedFile {
        offset: u32,
        need: u32,
        file_len: u32,
    },
    ScriptIndexOutOfBounds {
        script: u32,
        index: u32,
        code_len: u32,
    },
    ScriptIndexWithEmptyCode {
        script: u32,
        index: u32,
    },
    NameOffsetBeyondFile {
        kind: NameKind,
        index: u32,
        offset: u32,
    },
    StringOffsetBeyondFile {
        kind: NameKind,
        index: u32,
        offset: u32,
    },
    MissingNul {
        kind: NameKind,
        index: u32,
        offset: u32,
    },
    EmptyName {
        kind: NameKind,
        index: u32,
    },
    /// The names together are longer than the input; see the module comment.
    NamesExceedInput {
        limit: usize,
    },
    /// A read past the end that the earlier bounds checks should have excluded. Kept as
    /// an error rather than a panic.
    Truncated {
        offset: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::FileTooLarge { len } => write!(f, "cob: file of {len} bytes is too large"),
            Error::HeaderTooShort { len } => {
                write!(f, "cob: header too short {len} < {HEADER_SIZE}")
            }
            Error::UnsupportedVersion { version } => {
                write!(f, "cob: unsupported version {version} want 4")
            }
            Error::ImplausibleCounts {
                scripts,
                pieces,
                code_len,
            } => write!(
                f,
                "cob: implausibly large counts scripts={scripts} pieces={pieces} codeLen={code_len}"
            ),
            Error::StaticStorage { words } => write!(
                f,
                "cob: static storage {words} words exceeds supported {MAX_PROGRAM_STATIC_BYTES}-byte VM budget"
            ),
            Error::TableOffsetBeyondFile {
                what,
                offset,
                file_len,
            } => write!(
                f,
                "cob: {what} offset {offset:#x} beyond file size {file_len:#x}"
            ),
            Error::TableOffsetUnaligned { what, offset } => {
                write!(f, "cob: {what} offset {offset:#x} not 4-byte aligned")
            }
            Error::TableOverflow { what } => write!(f, "cob: {what} offset overflow"),
            Error::TableExceedsFile {
                what,
                offset,
                need,
                file_len,
            } => write!(
                f,
                "cob: {what} table [{offset:#x} + {need:#x}] exceeds file size {file_len:#x}"
            ),
            Error::TrailingRecordsOffsetBeyondFile { offset, file_len } => write!(
                f,
                "cob: trailing record table offset {offset:#x} beyond file size {file_len:#x}"
            ),
            Error::ImplausibleTrailingCount { count } => {
                write!(f, "cob: implausibly large trailing record count {count}")
            }
            Error::TrailingRecordsExceedFile {
                offset,
                need,
                file_len,
            } => write!(
                f,
                "cob: trailing record table [{offset:#x} + {need:#x}] exceeds file size {file_len:#x}"
            ),
            Error::ScriptIndexOutOfBounds {
                script,
                index,
                code_len,
            } => write!(
                f,
                "cob: script {script} code index {index} out of bounds (codeLen {code_len})"
            ),
            Error::ScriptIndexWithEmptyCode { script, index } => write!(
                f,
                "cob: script {script} code index {index} with empty code section"
            ),
            Error::NameOffsetBeyondFile {
                kind,
                index,
                offset,
            } => write!(f, "cob: {kind} {index} name offset {offset:#x} beyond file"),
            Error::StringOffsetBeyondFile {
                kind,
                index,
                offset,
            } => write!(
                f,
                "cob: {kind} {index} name: cob: string offset {offset:#x} beyond file"
            ),
            Error::MissingNul {
                kind,
                index,
                offset,
            } => write!(
                f,
                "cob: {kind} {index} name: cob: string at {offset:#x} missing NUL terminator"
            ),
            Error::EmptyName { kind, index } => write!(f, "cob: {kind} {index} has empty name"),
            Error::NamesExceedInput { limit } => write!(
                f,
                "cob: script and piece names total more than the {limit}-byte input"
            ),
            Error::Truncated { offset } => write!(f, "cob: read past end of file at {offset:#x}"),
        }
    }
}

impl std::error::Error for Error {}

/// An immutable compiled script definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    code: Vec<u32>,
    scripts: BTreeMap<Vec<u8>, usize>,
    pieces: Vec<Vec<u8>>,
    statics: usize,
    scripts_by_id: Vec<usize>,
    source_checksum: u32,
    script_names: Vec<Vec<u8>>,
}

impl Program {
    /// The opcode word array.
    pub fn code(&self) -> &[u32] {
        &self.code
    }

    /// Script name to word index into [`Program::code`] (not a byte offset). Names are
    /// case-sensitive; exact duplicates resolve to the first table entry.
    pub fn scripts(&self) -> &BTreeMap<Vec<u8>, usize> {
        &self.scripts
    }

    /// Entry point of the named script (case-sensitive).
    pub fn script_index(&self, name: impl AsRef<[u8]>) -> Option<usize> {
        self.scripts.get(name.as_ref()).copied()
    }

    /// The ordered piece name table.
    pub fn pieces(&self) -> &[Vec<u8>] {
        &self.pieces
    }

    /// Count of static variables the script needs (`NumberOfStatics`). The VM
    /// initialises them to zero deterministically; retail leaves them uninitialised.
    pub fn statics(&self) -> usize {
        self.statics
    }

    /// Script entry points in table order, for the script-id operand. Includes
    /// duplicates that [`Program::scripts`] shadows.
    pub fn scripts_by_id(&self) -> &[usize] {
        &self.scripts_by_id
    }

    /// Script names in table order, beside [`Program::scripts_by_id`].
    pub fn script_names(&self) -> &[Vec<u8>] {
        &self.script_names
    }

    /// The four-accumulator checksum of the raw COB bytes this was loaded from.
    pub fn source_checksum(&self) -> u32 {
        self.source_checksum
    }

    /// Rejects a program whose static count cannot be held in the supported per-VM
    /// memory budget. [`Program::load`] already applies this.
    pub fn validate(&self) -> Result<(), Error> {
        if self.statics > MAX_PROGRAM_STATIC_BYTES / 4 {
            return Err(Error::StaticStorage {
                words: self.statics,
            });
        }
        Ok(())
    }

    /// Parses COB bytes.
    ///
    /// Every count and offset read from the header is checked against the input length
    /// before anything is reserved, so allocation is bounded by the input size.
    pub fn load(data: &[u8]) -> Result<Program, Error> {
        if data.len() < HEADER_SIZE {
            return Err(Error::HeaderTooShort { len: data.len() });
        }
        let file_len =
            u32::try_from(data.len()).map_err(|_| Error::FileTooLarge { len: data.len() })?;
        let source_checksum = content_checksum(data);

        let version = read_u32(data, 0x00)?; // VersionSignature
        let num_scripts = read_u32(data, 0x04)?; // NumberOfScripts
        let num_pieces = read_u32(data, 0x08)?; // NumberOfPieces
        let code_len = read_u32(data, 0x0C)?; // CodeLength, in u32 words
        let num_statics = read_u32(data, 0x10)?; // NumberOfStatics
                                                 // Word 0x14 is the record count for the trailing 8-byte record table whose
                                                 // pointer is word 0x28. It is not reserved, and 0x28 is not a separate "first
                                                 // script name" offset. Both are zero or coincidental in the whole retail corpus.
        let num_trailing_records = read_u32(data, 0x14)?;
        let off_script_code_index = read_u32(data, 0x18)?;
        let off_script_name_offsets = read_u32(data, 0x1C)?;
        let off_piece_name_offsets = read_u32(data, 0x20)?;
        let off_script_code = read_u32(data, 0x24)?;
        let off_trailing_records = read_u32(data, 0x28)?;

        // Divergence from retail, which never reads the version word: reject a non-4
        // file rather than walking a Kingdoms (version 6, 52-byte header) layout as if
        // it were a TA one.
        if version != 4 {
            return Err(Error::UnsupportedVersion { version });
        }
        if num_scripts > MAX_COUNT || num_pieces > MAX_COUNT || code_len > MAX_COUNT {
            return Err(Error::ImplausibleCounts {
                scripts: num_scripts,
                pieces: num_pieces,
                code_len,
            });
        }
        let statics = usize::try_from(num_statics).unwrap_or(usize::MAX);
        if statics > MAX_PROGRAM_STATIC_BYTES / 4 {
            return Err(Error::StaticStorage { words: statics });
        }

        validate_table(
            "script code index array",
            off_script_code_index,
            num_scripts,
            file_len,
        )?;
        validate_table(
            "script name offset array",
            off_script_name_offsets,
            num_scripts,
            file_len,
        )?;
        validate_table(
            "piece name offset array",
            off_piece_name_offsets,
            num_pieces,
            file_len,
        )?;
        if code_len > 0 {
            validate_table("code", off_script_code, code_len, file_len)?;
        } else if off_script_code != 0 && off_script_code > file_len {
            // Zero-length code section: a non-zero offset must still be inside the file.
            return Err(Error::TableOffsetBeyondFile {
                what: "code",
                offset: off_script_code,
                file_len,
            });
        }

        // The trailing record table: span bounds only. Retail relocates the second word
        // of each record; the surveyed scripts carry none and no consumer was found, so
        // an in-bounds span is accepted without interpreting or retaining the records.
        // Open question upstream: what non-empty records and their targets mean.
        if off_trailing_records != 0 && off_trailing_records > file_len {
            return Err(Error::TrailingRecordsOffsetBeyondFile {
                offset: off_trailing_records,
                file_len,
            });
        }
        if num_trailing_records != 0 {
            if num_trailing_records > MAX_COUNT {
                return Err(Error::ImplausibleTrailingCount {
                    count: num_trailing_records,
                });
            }
            // At most 2^31, so this cannot overflow.
            let need = num_trailing_records.saturating_mul(8);
            match off_trailing_records.checked_add(need) {
                Some(end) if end <= file_len => {}
                _ => {
                    return Err(Error::TrailingRecordsExceedFile {
                        offset: off_trailing_records,
                        need,
                        file_len,
                    })
                }
            }
        }

        // Every table below was bounds-checked against the input, so these allocations
        // are at most input-sized.
        let code = read_words(data, off_script_code, code_len)?;

        let script_indexes = read_words(data, off_script_code_index, num_scripts)?;
        for (i, &index) in (0u32..).zip(&script_indexes) {
            if code_len != 0 && index >= code_len {
                return Err(Error::ScriptIndexOutOfBounds {
                    script: i,
                    index,
                    code_len,
                });
            }
            if code_len == 0 && index != 0 {
                return Err(Error::ScriptIndexWithEmptyCode { script: i, index });
            }
        }

        let script_name_offs = read_words(data, off_script_name_offsets, num_scripts)?;
        for (i, &offset) in (0u32..).zip(&script_name_offs) {
            if offset >= file_len {
                return Err(Error::NameOffsetBeyondFile {
                    kind: NameKind::Script,
                    index: i,
                    offset,
                });
            }
        }
        let piece_name_offs = read_words(data, off_piece_name_offsets, num_pieces)?;
        for (i, &offset) in (0u32..).zip(&piece_name_offs) {
            if offset >= file_len {
                return Err(Error::NameOffsetBeyondFile {
                    kind: NameKind::Piece,
                    index: i,
                    offset,
                });
            }
        }

        let mut name_budget = data.len();
        let mut scripts = BTreeMap::new();
        let mut script_names = Vec::with_capacity(script_name_offs.len());
        for (i, (&offset, &index)) in (0u32..).zip(script_name_offs.iter().zip(&script_indexes)) {
            let name = read_name(data, NameKind::Script, i, offset, &mut name_budget)?;
            // Retail scans names case-sensitively in table order and returns the first
            // match. Later duplicates remain callable by script index through
            // `scripts_by_id`.
            scripts.entry(name.clone()).or_insert_with(|| widen(index));
            script_names.push(name);
        }

        let mut pieces = Vec::with_capacity(piece_name_offs.len());
        for (i, &offset) in (0u32..).zip(&piece_name_offs) {
            pieces.push(read_name(
                data,
                NameKind::Piece,
                i,
                offset,
                &mut name_budget,
            )?);
        }

        let program = Program {
            code,
            scripts,
            pieces,
            statics,
            scripts_by_id: script_indexes.iter().map(|&v| widen(v)).collect(),
            source_checksum,
            script_names,
        };
        program.validate()?;
        Ok(program)
    }
}

fn widen(value: u32) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

/// The retail four-accumulator byte checksum. Each lane is eight bits and wraps
/// independently; the packed result is lane order additive, xor, indexed-additive,
/// indexed-xor, low byte first.
pub fn content_checksum(data: &[u8]) -> u32 {
    let (mut add, mut xor, mut indexed_add, mut indexed_xor) = (0u8, 0u8, 0u8, 0u8);
    for (i, &b) in data.iter().enumerate() {
        // Only the low byte of the index matters to every lane.
        let low = i.to_le_bytes()[0];
        add = add.wrapping_add(b);
        xor ^= b;
        indexed_add = indexed_add.wrapping_add(low ^ b);
        indexed_xor ^= low.wrapping_add(b);
    }
    u32::from(add)
        | u32::from(xor) << 8
        | u32::from(indexed_add) << 16
        | u32::from(indexed_xor) << 24
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32, Error> {
    let end = offset.checked_add(4).ok_or(Error::Truncated { offset })?;
    let bytes: [u8; 4] = data
        .get(offset..end)
        .and_then(|s| s.try_into().ok())
        .ok_or(Error::Truncated { offset })?;
    Ok(u32::from_le_bytes(bytes))
}

/// Reads `count` words at `offset`. The caller has already checked that the span lies
/// inside `data`, so `count` cannot cause an oversized reservation.
fn read_words(data: &[u8], offset: u32, count: u32) -> Result<Vec<u32>, Error> {
    let start = widen(offset);
    let len = widen(count)
        .checked_mul(4)
        .ok_or(Error::Truncated { offset: start })?;
    let end = start
        .checked_add(len)
        .ok_or(Error::Truncated { offset: start })?;
    let span = data
        .get(start..end)
        .ok_or(Error::Truncated { offset: start })?;
    // `span` is exactly `count` words long, so there is no remainder.
    let (words, _) = span.as_chunks::<4>();
    Ok(words.iter().map(|w| u32::from_le_bytes(*w)).collect())
}

/// Checks that a table of `count` words at `offset` lies inside the file. Offsets are
/// absolute file offsets. A zero count is always accepted.
fn validate_table(what: &'static str, offset: u32, count: u32, file_len: u32) -> Result<(), Error> {
    if count == 0 {
        return Ok(());
    }
    if offset > file_len {
        return Err(Error::TableOffsetBeyondFile {
            what,
            offset,
            file_len,
        });
    }
    if !offset.is_multiple_of(4) {
        return Err(Error::TableOffsetUnaligned { what, offset });
    }
    // `count` is at most 2^28 for every caller, so `* 4` fits in u32; saturating keeps
    // the function total anyway.
    let need = count.saturating_mul(4);
    match offset.checked_add(need) {
        None => Err(Error::TableOverflow { what }),
        Some(end) if end > file_len => Err(Error::TableExceedsFile {
            what,
            offset,
            need,
            file_len,
        }),
        Some(_) => Ok(()),
    }
}

/// Reads a NUL-terminated, non-empty name at an absolute file offset and charges its
/// length to `budget`.
fn read_name(
    data: &[u8],
    kind: NameKind,
    index: u32,
    offset: u32,
    budget: &mut usize,
) -> Result<Vec<u8>, Error> {
    let start = widen(offset);
    let tail = data.get(start..).ok_or(Error::StringOffsetBeyondFile {
        kind,
        index,
        offset,
    })?;
    let len = tail.iter().position(|&b| b == 0).ok_or(Error::MissingNul {
        kind,
        index,
        offset,
    })?;
    if len == 0 {
        return Err(Error::EmptyName { kind, index });
    }
    *budget = budget
        .checked_sub(len)
        .ok_or(Error::NamesExceedInput { limit: data.len() })?;
    Ok(tail.get(..len).unwrap_or(&[]).to_vec())
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

    fn put(buf: &mut [u8], at: usize, value: u32) {
        buf[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn get(buf: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(buf[at..at + 4].try_into().unwrap())
    }

    /// Builds a synthetic COB: 44-byte header, code at the header end, then tables, then
    /// strings contiguous. Authored here, never copied from game data.
    fn make_cob(code: &[u32], scripts: &[&str], indexes: &[u32], pieces: &[&str]) -> Vec<u8> {
        make_cob_with_statics(0, code, scripts, indexes, pieces)
    }

    fn make_cob_with_statics(
        statics: u32,
        code: &[u32],
        scripts: &[&str],
        indexes: &[u32],
        pieces: &[&str],
    ) -> Vec<u8> {
        let n_scripts = scripts.len() as u32;
        let n_pieces = pieces.len() as u32;
        let code_len = code.len() as u32;
        let off_code = 44u32;
        let off_script_idx = off_code + code_len * 4;
        let off_script_name_off = off_script_idx + n_scripts * 4;
        let off_piece_name_off = off_script_name_off + n_scripts * 4;
        let str_start = off_piece_name_off + n_pieces * 4;
        let total_str: usize = scripts.iter().chain(pieces).map(|s| s.len() + 1).sum();
        let mut buf = vec![0u8; str_start as usize + total_str];
        put(&mut buf, 0x00, 4);
        put(&mut buf, 0x04, n_scripts);
        put(&mut buf, 0x08, n_pieces);
        put(&mut buf, 0x0C, code_len);
        put(&mut buf, 0x10, statics);
        put(&mut buf, 0x14, 0);
        put(&mut buf, 0x18, off_script_idx);
        put(&mut buf, 0x1C, off_script_name_off);
        put(&mut buf, 0x20, off_piece_name_off);
        put(&mut buf, 0x24, off_code);
        put(&mut buf, 0x28, if n_scripts > 0 { str_start } else { 0 });
        for (i, w) in code.iter().enumerate() {
            put(&mut buf, off_code as usize + i * 4, *w);
        }
        let mut cur = str_start as usize;
        let mut script_offs = Vec::new();
        let mut piece_offs = Vec::new();
        for name in scripts {
            script_offs.push(cur as u32);
            buf[cur..cur + name.len()].copy_from_slice(name.as_bytes());
            cur += name.len() + 1;
        }
        for name in pieces {
            piece_offs.push(cur as u32);
            buf[cur..cur + name.len()].copy_from_slice(name.as_bytes());
            cur += name.len() + 1;
        }
        for (i, idx) in indexes.iter().enumerate() {
            put(&mut buf, off_script_idx as usize + i * 4, *idx);
        }
        for (i, off) in script_offs.iter().enumerate() {
            put(&mut buf, off_script_name_off as usize + i * 4, *off);
        }
        for (i, off) in piece_offs.iter().enumerate() {
            put(&mut buf, off_piece_name_off as usize + i * 4, *off);
        }
        buf
    }

    fn minimal() -> Vec<u8> {
        make_cob(&[0x1006_5000], &["Create"], &[0], &["base"])
    }

    #[test]
    fn load_valid_minimal() {
        let data = minimal();
        let prog = Program::load(&data).unwrap();
        assert_eq!(prog.code(), &[0x1006_5000]);
        assert_eq!(prog.pieces(), &[b"base".to_vec()]);
        assert_eq!(prog.script_index("Create"), Some(0));
        assert_eq!(prog.statics(), 0);
        assert_eq!(prog.source_checksum(), content_checksum(&data));
        assert_eq!(prog.scripts().len(), 1);
        assert_eq!(prog.script_names(), &[b"Create".to_vec()]);
    }

    #[test]
    fn load_statics() {
        for want in [0u32, 1, 3, 7] {
            let data = make_cob_with_statics(want, &[0x1006_5000], &["Create"], &[0], &["base"]);
            assert_eq!(Program::load(&data).unwrap().statics(), want as usize);
        }
    }

    #[test]
    fn load_rejects_excessive_static_storage() {
        let words = (MAX_PROGRAM_STATIC_BYTES / 4 + 1) as u32;
        let data = make_cob_with_statics(words, &[0x1006_5000], &["Create"], &[0], &["base"]);
        let err = Program::load(&data).unwrap_err();
        assert!(matches!(err, Error::StaticStorage { .. }));
        assert!(err.to_string().contains("static storage"));
        // The limit itself is accepted.
        let ok = make_cob_with_statics(words - 1, &[0x1006_5000], &["Create"], &[0], &["base"]);
        assert_eq!(Program::load(&ok).unwrap().statics(), (words - 1) as usize);
        // `validate` applies the same bound to a program built elsewhere.
        let mut prog = Program::load(&minimal()).unwrap();
        assert_eq!(prog.validate(), Ok(()));
        prog.statics = MAX_PROGRAM_STATIC_BYTES / 4 + 1;
        assert!(matches!(prog.validate(), Err(Error::StaticStorage { .. })));
    }

    #[test]
    fn load_valid_multiple_scripts() {
        let code = [
            0x1002_1001,
            0x0000_0000,
            0x1006_5000,
            0x1002_2000,
            0x1006_5000,
            0x1006_5000,
        ];
        let scripts = ["SmokeUnit", "Create", "Killed"];
        let data = make_cob(&code, &scripts, &[0, 2, 4], &["base"]);
        let prog = Program::load(&data).unwrap();
        assert_eq!(prog.code().len(), code.len());
        for name in scripts {
            let idx = prog.script_index(name).unwrap();
            assert!(idx < prog.code().len(), "{name}");
        }
        assert_eq!(prog.pieces().len(), 1);
        let keys: Vec<&[u8]> = prog.scripts().keys().map(Vec::as_slice).collect();
        assert_eq!(keys, vec![&b"Create"[..], b"Killed", b"SmokeUnit"]);
    }

    #[test]
    fn load_empty() {
        let prog = Program::load(&make_cob(&[], &[], &[], &[])).unwrap();
        assert!(prog.code().is_empty());
        assert!(prog.scripts().is_empty());
        assert!(prog.pieces().is_empty());
        assert_eq!(prog.statics(), 0);
    }

    #[test]
    fn load_code_offset_relocation() {
        // Offsets are absolute file offsets: build a file where code is not directly
        // after the header but padded.
        let data = make_cob(&[0x1006_5000, 0x1006_5000], &["Create"], &[1], &["base"]);
        let pad = 16usize;
        let orig_code = get(&data, 0x24) as usize;
        let new_code = orig_code + pad;
        let mut new_data = vec![0u8; data.len() + pad];
        new_data[..44].copy_from_slice(&data[..44]);
        put(&mut new_data, 0x24, new_code as u32);
        let orig_script_idx = get(&data, 0x18) as usize;
        let orig_script_name = get(&data, 0x1C) as usize;
        let orig_piece_name = get(&data, 0x20) as usize;
        put(&mut new_data, 0x18, (orig_script_idx + pad) as u32);
        put(&mut new_data, 0x1C, (orig_script_name + pad) as u32);
        put(&mut new_data, 0x20, (orig_piece_name + pad) as u32);
        let orig_first = get(&data, 0x28);
        if orig_first != 0 {
            put(&mut new_data, 0x28, orig_first + pad as u32);
        }
        new_data[new_code..].copy_from_slice(&data[orig_code..]);
        // String offsets inside the tables are absolute too.
        put(
            &mut new_data,
            orig_script_name + pad,
            get(&data, orig_script_name) + pad as u32,
        );
        put(
            &mut new_data,
            orig_piece_name + pad,
            get(&data, orig_piece_name) + pad as u32,
        );
        let prog = Program::load(&new_data).unwrap();
        assert_eq!(prog.code().len(), 2);
        assert_eq!(prog.code()[1], 0x1006_5000);
        assert_eq!(prog.script_index("Create"), Some(1));
    }

    #[test]
    fn load_header_too_short() {
        let err = Program::load(&[0x04, 0x00]).unwrap_err();
        assert_eq!(err, Error::HeaderTooShort { len: 2 });
        assert!(err.to_string().contains("header too short"));
        assert!(matches!(
            Program::load(&[]),
            Err(Error::HeaderTooShort { len: 0 })
        ));
    }

    #[test]
    fn load_bad_version() {
        let mut data = minimal();
        put(&mut data, 0x00, 6); // Kingdoms
        let err = Program::load(&data).unwrap_err();
        assert_eq!(err, Error::UnsupportedVersion { version: 6 });
        assert!(err.to_string().contains("unsupported version"));
    }

    /// Header word 0x14 is the record count of the trailing record table at 0x28. A
    /// non-zero count whose table lies inside the file loads; only a table that runs off
    /// the end is refused.
    #[test]
    fn load_trailing_record_count() {
        let data = minimal();
        let mut with_records = data.clone();
        with_records.extend_from_slice(&[0u8; 8]);
        put(&mut with_records, 0x14, 1);
        put(&mut with_records, 0x28, data.len() as u32);
        assert!(Program::load(&with_records).is_ok());

        let mut off_end = data.clone();
        put(&mut off_end, 0x14, 1);
        put(&mut off_end, 0x28, data.len() as u32 - 4);
        let err = Program::load(&off_end).unwrap_err();
        assert!(matches!(err, Error::TrailingRecordsExceedFile { .. }));
        assert!(err.to_string().contains("trailing record table"));

        // Offset itself beyond the file, with and without a count.
        let mut beyond = data.clone();
        put(&mut beyond, 0x28, data.len() as u32 + 1);
        assert!(matches!(
            Program::load(&beyond),
            Err(Error::TrailingRecordsOffsetBeyondFile { .. })
        ));
        // Implausible count, and a count whose span overflows u32.
        let mut huge = data.clone();
        put(&mut huge, 0x14, MAX_COUNT + 1);
        assert!(matches!(
            Program::load(&huge),
            Err(Error::ImplausibleTrailingCount { .. })
        ));
        let mut wrap = data.clone();
        put(&mut wrap, 0x14, MAX_COUNT);
        put(&mut wrap, 0x28, data.len() as u32);
        assert!(matches!(
            Program::load(&wrap),
            Err(Error::TrailingRecordsExceedFile { .. })
        ));
    }

    #[test]
    fn load_code_offset_out_of_bounds() {
        let mut data = minimal();
        put(&mut data, 0x24, 0xFFFF_FFF0);
        assert!(matches!(
            Program::load(&data),
            Err(Error::TableOffsetBeyondFile { what: "code", .. })
        ));
    }

    #[test]
    fn load_script_index_out_of_bounds() {
        let mut data = minimal();
        let off = get(&data, 0x18) as usize;
        put(&mut data, off, 5); // codeLen is 1
        let err = Program::load(&data).unwrap_err();
        assert!(matches!(err, Error::ScriptIndexOutOfBounds { .. }));
        assert!(err.to_string().contains("out of bounds"));
    }

    #[test]
    fn load_script_index_with_empty_code() {
        let mut data = make_cob(&[], &["Create"], &[0], &[]);
        let off = get(&data, 0x18) as usize;
        assert!(Program::load(&data).is_ok());
        put(&mut data, off, 1);
        assert!(matches!(
            Program::load(&data),
            Err(Error::ScriptIndexWithEmptyCode { .. })
        ));
    }

    #[test]
    fn load_name_offset_out_of_bounds() {
        let mut data = minimal();
        let off = get(&data, 0x1C) as usize;
        let len = data.len() as u32;
        put(&mut data, off, len + 10);
        assert!(matches!(
            Program::load(&data),
            Err(Error::NameOffsetBeyondFile {
                kind: NameKind::Script,
                ..
            })
        ));
    }

    #[test]
    fn load_missing_nul() {
        // No pieces, so the only string is the script name; with every NUL replaced
        // nothing terminates it.
        let mut data = make_cob(&[0x1006_5000], &["Create"], &[0], &[]);
        let off = get(&data, 0x1C) as usize;
        let str_off = get(&data, off) as usize;
        for b in &mut data[str_off..] {
            if *b == 0 {
                *b = b'X';
            }
        }
        let err = Program::load(&data).unwrap_err();
        assert!(matches!(err, Error::MissingNul { .. }));
        assert!(err.to_string().contains("missing NUL"));
    }

    #[test]
    fn load_duplicate_script_name_keeps_first_exact_match() {
        // Name lookup takes the first case-sensitive match; the index array retains
        // every authored entry, duplicates included.
        let code = [0x1006_5000; 3];
        let data = make_cob(
            &code,
            &["Create", "Create", "create"],
            &[2, 0, 1],
            &["base"],
        );
        let prog = Program::load(&data).unwrap();
        assert_eq!(prog.script_index("Create"), Some(2));
        assert_eq!(prog.script_index("create"), Some(1));
        assert_eq!(
            prog.script_index("CREATE"),
            None,
            "lookup must not fold case"
        );
        assert_eq!(prog.scripts_by_id(), &[2, 0, 1]);
        assert_eq!(prog.script_names().len(), 3);
    }

    #[test]
    fn load_unaligned_offset() {
        let mut data = minimal();
        put(&mut data, 0x24, 45);
        let err = Program::load(&data).unwrap_err();
        assert!(matches!(err, Error::TableOffsetUnaligned { .. }));
        assert!(err.to_string().contains("aligned"));
    }

    #[test]
    fn load_empty_script_name() {
        let mut data = minimal();
        let off = get(&data, 0x1C) as usize;
        let str_off = get(&data, off) as usize;
        data[str_off] = 0;
        let err = Program::load(&data).unwrap_err();
        assert_eq!(
            err,
            Error::EmptyName {
                kind: NameKind::Script,
                index: 0
            }
        );
        assert!(err.to_string().contains("empty name"));
    }

    #[test]
    fn load_empty_piece_name() {
        let mut data = minimal();
        let off = get(&data, 0x20) as usize;
        let str_off = get(&data, off) as usize;
        data[str_off] = 0;
        assert!(matches!(
            Program::load(&data),
            Err(Error::EmptyName {
                kind: NameKind::Piece,
                ..
            })
        ));
    }

    #[test]
    fn load_truncated_table() {
        let data = make_cob(
            &[0x1006_5000, 0x1006_5000],
            &["A", "B"],
            &[0, 1],
            &["p1", "p2"],
        );
        assert!(Program::load(&data[..data.len() - 5]).is_err());
    }

    // ---- additions: bounds on hostile headers ----

    #[test]
    fn counts_larger_than_the_input_do_not_allocate() {
        // Counts at the plausibility ceiling with a tiny file: rejected by the bounds
        // check before anything is reserved.
        for (word, name) in [(0x04, "scripts"), (0x08, "pieces"), (0x0C, "code")] {
            let mut data = minimal();
            put(&mut data, word, MAX_COUNT);
            assert!(
                matches!(Program::load(&data), Err(Error::TableExceedsFile { .. })),
                "{name}"
            );
            put(&mut data, word, MAX_COUNT + 1);
            assert!(
                matches!(Program::load(&data), Err(Error::ImplausibleCounts { .. })),
                "{name}"
            );
        }
        let mut wrap = minimal();
        put(&mut wrap, 0x04, 1 << 20);
        put(&mut wrap, 0x18, 0xFFFF_FFF0);
        assert!(matches!(
            Program::load(&wrap),
            Err(Error::TableOffsetBeyondFile { .. })
        ));
        let mut overflow = minimal();
        put(&mut overflow, 0x04, 4);
        put(&mut overflow, 0x18, (u32::MAX - 3) & !3);
        let _ = Program::load(&overflow);
    }

    #[test]
    fn names_may_not_exceed_the_input_in_total() {
        // Many table entries all pointing at one long string would copy it many times.
        let long = "x".repeat(1000);
        let mut data = make_cob(&[0x1006_5000], &[long.as_str(), "b"], &[0, 0], &[]);
        let name_tab = get(&data, 0x1C) as usize;
        let first = get(&data, name_tab);
        // Grow the script count and the table so many entries share the long string.
        let n = 64usize;
        while !data.len().is_multiple_of(4) {
            data.push(0);
        }
        let table_off = data.len();
        for _ in 0..n {
            data.extend_from_slice(&first.to_le_bytes());
        }
        let idx_off = data.len();
        data.extend_from_slice(&vec![0u8; n * 4]);
        put(&mut data, 0x04, n as u32);
        put(&mut data, 0x1C, table_off as u32);
        put(&mut data, 0x18, idx_off as u32);
        // Table + index array are 512 bytes, the long string 1000: 64 copies exceed it.
        assert!(matches!(
            Program::load(&data),
            Err(Error::NamesExceedInput { .. })
        ));
    }

    #[test]
    fn checksum_lanes() {
        assert_eq!(content_checksum(&[]), 0);
        // One byte 0x05 at index 0: add=5, xor=5, indexed add=0^5=5, indexed xor=(0+5)=5.
        assert_eq!(content_checksum(&[5]), 0x0505_0505);
        // Index 1, byte 0xFF: indexed add is 1^0xFF=0xFE, indexed xor is (1+0xFF)&0xFF=0.
        assert_eq!(content_checksum(&[0, 0xFF]), 0x00FE_FFFF);
    }

    #[test]
    fn hostile_inputs_never_panic() {
        let data = make_cob(
            &[0x1006_5000, 0x1006_5000],
            &["A", "B"],
            &[0, 1],
            &["p1", "p2"],
        );
        // Every prefix, and every single-word corruption of the header.
        for end in 0..=data.len() {
            let _ = Program::load(&data[..end]);
        }
        for word in 0..11 {
            for value in [0u32, 1, 4, 5, 43, 44, 0x7FFF_FFFF, 0xFFFF_FFFF] {
                let mut copy = data.clone();
                put(&mut copy, word * 4, value);
                let _ = Program::load(&copy);
            }
        }
    }
}
