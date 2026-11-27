// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/hpi.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! The HPI-family archive reader (`.hpi`, `.ufo`, `.ccx`, `.gp3` and other files
//! with the same HAPI container layout), version 1 archives.
//!
//! Opening an archive reads its header, footer and directory only. A record is
//! read and decoded when it is opened: stored, LZ77 or zlib, in 64 KiB SQSH chunks.

use std::collections::{HashMap, HashSet};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use miniz_oxide::inflate::core::inflate_flags::{
    TINFL_FLAG_HAS_MORE_INPUT, TINFL_FLAG_PARSE_ZLIB_HEADER,
    TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
};
use miniz_oxide::inflate::core::{decompress, DecompressorOxide};
use miniz_oxide::inflate::TINFLStatus;

use super::host;
use super::overlay::{EntryInfo, File, Provenance};
use super::path::{fold_logical_name, join_path, original_join};
use super::provider_index::{EntrySource, ProviderEntry, ProviderIndex};
use super::{show, Error};

pub(crate) const HPI_HEADER_SIZE: u64 = 20;
#[cfg(test)]
pub(crate) const HPI_DIRECTORY_NODE_SIZE: u64 = 8;
pub(crate) const HPI_ENTRY_SIZE: u64 = 9;
pub(crate) const HPI_CHUNK_HEADER_SIZE: u64 = 19;
/// The ceiling for a single decoded file.
pub const DEFAULT_MAX_FILE_BYTES: i64 = 512 << 20;
pub(crate) const HPI_CHUNK_SIZE: u64 = 64 * 1024;
/// A chunk read whole: a full 64 KiB chunk its encoder failed to shrink, with room
/// to spare. A larger stored size is not one retail writes; it is streamed.
pub(crate) const HPI_CHUNK_READ_LIMIT: u64 = 2 * HPI_CHUNK_SIZE + HPI_CHUNK_HEADER_SIZE;
/// The piece size the streamed checksum reads (Go's 32 KiB buffer).
const CHECKSUM_PIECE: usize = 32 << 10;
/// The piece size a streamed payload is decoded from (Go's `bufio` default).
const STREAM_PIECE: usize = 4096;

const VERSION_1: u32 = 0x0001_0000;
const VERSION_BANK: u32 = 0x4B4E_4142;
const FOOTER_TEMPLATE: &[u8; 36] = b"Copyright 0000 Cavedog Entertainment";

/// Random access to archive bytes (Go `io.ReaderAt`).
///
/// `read_exact_at` fills `buf` from `offset` or fails. A read that runs past the
/// end of the data must fail with [`io::ErrorKind::UnexpectedEof`]: at the header
/// and footer that classifies the archive as rejected rather than malformed.
pub trait ReadAt {
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()>;
}

fn slice_read_at(data: &[u8], buf: &mut [u8], offset: u64) -> io::Result<()> {
    let source = usize::try_from(offset)
        .ok()
        .and_then(|start| data.get(start..)?.get(..buf.len()));
    match source {
        Some(source) => {
            buf.copy_from_slice(source);
            Ok(())
        }
        None => Err(io::Error::from(io::ErrorKind::UnexpectedEof)),
    }
}

impl ReadAt for [u8] {
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        slice_read_at(self, buf, offset)
    }
}

impl ReadAt for Vec<u8> {
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        slice_read_at(self, buf, offset)
    }
}

impl<T: ReadAt + ?Sized> ReadAt for Arc<T> {
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        (**self).read_exact_at(buf, offset)
    }
}

impl<T: ReadAt + ?Sized> ReadAt for Box<T> {
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        (**self).read_exact_at(buf, offset)
    }
}

impl<T: ReadAt + ?Sized> ReadAt for &T {
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        (**self).read_exact_at(buf, offset)
    }
}

/// A host file read at offsets. The handle is shared, so each read seeks under a
/// lock; Go's `os.File.ReadAt` used `pread`, which this matches in result.
struct HostFile(Mutex<std::fs::File>);

impl ReadAt for HostFile {
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        let mut file = self
            .0
            .lock()
            .map_err(|_| io::Error::other("archive file lock poisoned"))?;
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(buf)
    }
}

/// Defensive limits while indexing and decoding an archive. The defaults are
/// generous for retail data while preventing malformed input from requesting
/// unbounded allocations. Nonpositive limits select the defaults.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ArchiveOptions {
    pub max_directory_bytes: i64,
    pub max_file_bytes: i64,
    pub verify_checksums: bool,
    pub skip_checksums: bool,
    pub allow_bank: bool,
    /// Directory expansion limits are host safety policy, not retail limits.
    /// Entries include repeated references and hidden duplicates.
    pub max_directory_entries: i64,
    /// Depth includes the root and empty directories.
    pub max_directory_depth: i64,
    /// Counts name bytes scanned (including each NUL) and both joined path lengths
    /// before normalization.
    pub max_directory_string_bytes: i64,
}

impl ArchiveOptions {
    /// The options with every nonpositive limit replaced by its default. Checksums
    /// are verified unless `skip_checksums` is set.
    pub fn with_defaults(mut self) -> ArchiveOptions {
        if self.max_directory_bytes <= 0 {
            self.max_directory_bytes = 128 << 20;
        }
        if self.max_file_bytes <= 0 {
            self.max_file_bytes = DEFAULT_MAX_FILE_BYTES;
        }
        // These defaults leave at least 33x entry, 16x depth and 128x string-work
        // headroom over the reference install.
        if self.max_directory_entries <= 0 {
            self.max_directory_entries = 1 << 16;
        }
        if self.max_directory_depth <= 0 {
            self.max_directory_depth = 64;
        }
        if self.max_directory_string_bytes <= 0 {
            self.max_directory_string_bytes = 16 << 20;
        }
        if !self.skip_checksums {
            self.verify_checksums = true;
        }
        self
    }
}

/// One file record of the directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HpiRecord {
    pub(crate) data_offset: u64,
    pub(crate) size: u64,
    pub(crate) compression: u8,
}

/// An indexed HPI-family archive.
pub struct Archive {
    name: PathBuf,
    reader: Box<dyn ReadAt + Send + Sync>,
    size: u64,
    options: ArchiveOptions,
    key: u8,
    pub(crate) index: ProviderIndex,
    indexed_entries: Vec<EntryInfo>,
    /// Enumeration keeps source order, but only beneath reachable directories.
    retail_entry_indices: Vec<usize>,
}

impl std::fmt::Debug for Archive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Archive")
            .field("name", &self.name)
            .field("size", &self.size)
            .field("entries", &self.indexed_entries.len())
            .finish_non_exhaustive()
    }
}

/// The positive value of a limit that `with_defaults` has made positive.
fn limit(value: i64) -> u64 {
    value.unsigned_abs()
}

fn le_u32(bytes: &[u8], at: usize) -> Option<u32> {
    let word = bytes.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes([
        *word.first()?,
        *word.get(1)?,
        *word.get(2)?,
        *word.get(3)?,
    ]))
}

fn quote_bytes(bytes: &[u8]) -> String {
    format!("\"{}\"", bytes.escape_ascii())
}

impl Archive {
    /// Opens an archive from a host path. Header and directory bytes are read now;
    /// file data is not touched until a file is opened.
    pub fn open(path: impl AsRef<Path>, options: ArchiveOptions) -> Result<Archive, Error> {
        let path = path.as_ref();
        let file = std::fs::File::open(path).map_err(|source| Error::Open {
            path: path.to_path_buf(),
            source,
        })?;
        let size = file
            .metadata()
            .map_err(|source| Error::io_at(path, source))?
            .len();
        Archive::new(host::clean(path), HostFile(Mutex::new(file)), size, options)
    }

    /// Indexes an archive from any random-access reader of `size` bytes, which also
    /// serves archives extracted from another archive.
    pub fn new<R>(
        name: impl Into<PathBuf>,
        reader: R,
        size: u64,
        options: ArchiveOptions,
    ) -> Result<Archive, Error>
    where
        R: ReadAt + Send + Sync + 'static,
    {
        if size < HPI_HEADER_SIZE {
            return Err(Error::RejectedArchive("archive is too small".to_string()));
        }
        let mut archive = Archive {
            name: name.into(),
            reader: Box::new(reader),
            size,
            options: options.with_defaults(),
            key: 0,
            index: ProviderIndex::default(),
            indexed_entries: Vec::new(),
            retail_entry_indices: Vec::new(),
        };
        archive.read_index()?;
        Ok(archive)
    }

    /// The name the archive reports as its source path.
    pub fn name(&self) -> &Path {
        &self.name
    }

    pub(crate) fn set_mount_info(&mut self, priority: i64, order: usize) {
        for entry in self.index.entries.values_mut() {
            entry.info.source.priority = priority;
            entry.info.source.mount_order = order;
        }
        for info in &mut self.indexed_entries {
            info.source.priority = priority;
            info.source.mount_order = order;
        }
    }

    fn read_index(&mut self) -> Result<(), Error> {
        let mut header = [0u8; HPI_HEADER_SIZE as usize];
        if let Err(error) = self.reader.read_exact_at(&mut header, 0) {
            return Err(classify_gate_read("header", error));
        }
        if header.get(0..4) != Some(b"HAPI".as_slice()) {
            return Err(Error::RejectedArchive(format!(
                "marker {}",
                quote_bytes(header.get(0..4).unwrap_or(&[]))
            )));
        }
        let word = |at: usize| le_u32(&header, at).unwrap_or(0);
        let version = word(4);
        if version != VERSION_1 && !(self.options.allow_bank && version == VERSION_BANK) {
            return Err(Error::RejectedArchive(format!(
                "unsupported version 0x{version:08x}"
            )));
        }
        // Footer: retail reads the 36 trailing bytes, overwrites the four edition
        // bytes with "0000", and requires the result to equal the template. The
        // accepted shape is "Copyright <any four bytes> Cavedog Entertainment".
        if version == VERSION_1 {
            let missing = || {
                Error::RejectedArchive(format!("missing footer {}", quote_bytes(FOOTER_TEMPLATE)))
            };
            let footer_len = FOOTER_TEMPLATE.len() as u64;
            if self.size < footer_len {
                return Err(missing());
            }
            let mut tail = [0u8; 36];
            if let Err(error) = self.reader.read_exact_at(&mut tail, self.size - footer_len) {
                return Err(classify_gate_read("footer", error));
            }
            if let Some(edition) = tail.get_mut(10..14) {
                edition.copy_from_slice(b"0000");
            }
            if &tail != FOOTER_TEMPLATE {
                return Err(missing());
            }
        }
        let blob_size = u64::from(word(8));
        let header_key_word = word(12);
        let root_offset = u64::from(word(16));
        // Retail reads directory-blob-size bytes from offset 0, so the blob
        // contains the header.
        if blob_size < HPI_HEADER_SIZE || blob_size > self.size {
            return Err(Error::MalformedArchive(format!(
                "invalid directory blob size 0x{blob_size:x}"
            )));
        }
        if root_offset < HPI_HEADER_SIZE || root_offset >= blob_size {
            return Err(Error::MalformedArchive(format!(
                "invalid root offset 0x{root_offset:x}"
            )));
        }
        let blob_len = match usize::try_from(blob_size) {
            Ok(len) if blob_size <= limit(self.options.max_directory_bytes) => len,
            _ => {
                return Err(Error::MalformedArchive(
                    "directory is too large".to_string(),
                ))
            }
        };
        let mut blob = vec![0u8; blob_len];
        self.reader
            .read_exact_at(&mut blob, 0)
            .map_err(|error| Error::MalformedArchive(format!("directory: {error}")))?;
        // Only the low stored byte participates. Stored 0 and 255 both select plain
        // data: the latter derives a zero working key. Go computed
        // `^(b>>6 | b<<2)` on a byte, whose shift drops the high bits: that is a
        // rotation left by two.
        let key_byte = header_key_word as u8;
        if key_byte != 0 {
            self.key = !key_byte.rotate_left(2);
        }
        if self.key != 0 {
            if let Some(body) = blob.get_mut(HPI_HEADER_SIZE as usize..) {
                decrypt(body, HPI_HEADER_SIZE, self.key);
            }
        }

        let root = EntryInfo {
            is_dir: true,
            source: Provenance {
                provider_type: "hpi",
                source_path: self.name.clone(),
                ..Provenance::default()
            },
            ..EntryInfo::default()
        };
        self.index.entries.insert(
            Vec::new(),
            ProviderEntry {
                info: root,
                source: EntrySource::Dir,
            },
        );
        let mut view = DirectoryView {
            bytes: &blob,
            start: 0,
            end: blob_size,
            entries_left: limit(self.options.max_directory_entries),
            strings_left: limit(self.options.max_directory_string_bytes),
        };
        let mut sink = IndexSink {
            name: &self.name,
            archive_size: self.size,
            max_file_bytes: limit(self.options.max_file_bytes),
            max_depth: limit(self.options.max_directory_depth),
            index: &mut self.index,
            indexed_entries: &mut self.indexed_entries,
            retail_entry_indices: &mut self.retail_entry_indices,
        };
        view.walk(&mut sink, root_offset)
    }

    /// Retail enumeration: forward entry order inside the directories the backward
    /// component lookup selects.
    pub(crate) fn retail_entries(&self) -> impl Iterator<Item = &EntryInfo> + '_ {
        self.retail_entry_indices
            .iter()
            .filter_map(|&index| self.indexed_entries.get(index))
    }

    /// Every authored entry, shadowed subtrees included, for diagnostics.
    pub(crate) fn audit_entries(&self) -> &[EntryInfo] {
        &self.indexed_entries
    }

    #[cfg(test)]
    pub(crate) fn lookup(&self, name: &[u8]) -> Option<&ProviderEntry> {
        self.index.lookup(name)
    }

    pub(crate) fn open_record(&self, record: HpiRecord, info: &EntryInfo) -> Result<File, Error> {
        let data = self.read_record_range(record, 0, None)?;
        // The data was decoded for this open alone, so the handle can hand it over
        // whole without a second copy.
        Ok(File::memory(data, info.clone()))
    }

    /// Decodes only the part of a record the requested range needs. `None` means
    /// "to the end"; a length past the end is clamped to it.
    ///
    /// A compressed record is a table of chunk sizes followed by that many SQSH
    /// chunks, and every chunk but the last decodes to exactly 64 KiB. The chunks
    /// before the range are stepped over with arithmetic alone.
    pub(crate) fn read_record_range(
        &self,
        record: HpiRecord,
        offset: u64,
        length: Option<usize>,
    ) -> Result<Vec<u8>, Error> {
        if record.size > limit(self.options.max_file_bytes) || usize::try_from(record.size).is_err()
        {
            return Err(Error::MalformedArchive(
                "file exceeds size limit".to_string(),
            ));
        }
        if offset > record.size {
            return Err(Error::MalformedArchive(format!(
                "range offset {offset} outside {}-byte file",
                record.size
            )));
        }
        let remaining = record.size - offset;
        let length: usize = match length {
            Some(length) if (length as u64) <= remaining => length,
            // remaining <= record.size, which fits a usize (checked above).
            _ => usize::try_from(remaining).unwrap_or(usize::MAX),
        };
        if record.compression == 0 {
            // Stored records were checked to lie inside the archive at index time,
            // so this allocation is bounded by the archive.
            self.check_archive_range(record.data_offset.saturating_add(offset), length as u64)?;
            let mut data = vec![0u8; length];
            self.read_archive_bytes(record.data_offset.saturating_add(offset), &mut data)?;
            return Ok(data);
        }
        let chunk_count = record.size.div_ceil(HPI_CHUNK_SIZE);
        let table_len = usize::try_from(chunk_count)
            .ok()
            .and_then(|count| count.checked_mul(4))
            .ok_or_else(|| Error::MalformedArchive("chunk table too large".to_string()))?;
        self.check_archive_range(record.data_offset, table_len as u64)?;
        let mut table = vec![0u8; table_len];
        self.read_archive_bytes(record.data_offset, &mut table)?;
        if length == 0 {
            return Ok(Vec::new());
        }
        let first = offset / HPI_CHUNK_SIZE;
        let last = (offset + length as u64 - 1) / HPI_CHUNK_SIZE;
        let stored_size = |chunk: u64| -> u64 {
            usize::try_from(chunk)
                .ok()
                .and_then(|chunk| chunk.checked_mul(4))
                .and_then(|at| le_u32(&table, at))
                .map_or(0, u64::from)
        };
        let mut position = record.data_offset + table_len as u64;
        for chunk in 0..first {
            let stored = stored_size(chunk);
            if stored < HPI_CHUNK_HEADER_SIZE || stored > self.size.saturating_sub(position) {
                return Err(Error::MalformedArchive(format!(
                    "invalid chunk {chunk} size"
                )));
            }
            position += stored;
        }
        // The output grows as chunks decode, never past `length`, so a record that
        // declares more than its chunks produce costs only what they produce.
        let mut result: Vec<u8> = Vec::with_capacity(length.min(HPI_CHUNK_SIZE as usize));
        let mut chunk = first;
        while chunk <= last && chunk < chunk_count {
            let stored = stored_size(chunk);
            if stored < HPI_CHUNK_HEADER_SIZE || stored > self.size.saturating_sub(position) {
                return Err(Error::MalformedArchive(format!(
                    "invalid chunk {chunk} size"
                )));
            }
            let chunk_start = chunk * HPI_CHUNK_SIZE;
            let expected = HPI_CHUNK_SIZE.min(record.size - chunk_start);
            let decoded = self.decode_chunk(chunk, position, stored, expected)?;
            position += stored;
            let from = offset.saturating_sub(chunk_start);
            let tail = usize::try_from(from)
                .ok()
                .and_then(|from| decoded.get(from..))
                .ok_or_else(|| Error::MalformedArchive(format!("chunk {chunk} output size")))?;
            let take = tail.len().min(length - result.len());
            let wanted = result.len() + take;
            if wanted > result.capacity() {
                let grown = result.capacity().saturating_mul(2).max(wanted).min(length);
                result.reserve_exact(grown - result.len());
            }
            result.extend_from_slice(tail.get(..take).unwrap_or(&[]));
            chunk += 1;
        }
        if result.len() != length {
            return Err(Error::MalformedArchive(
                "decompressed size mismatch".to_string(),
            ));
        }
        Ok(result)
    }

    /// Reads and decodes one SQSH chunk. `expected` is the record's chunk span:
    /// 64 KiB for every non-final chunk and the remainder for the final one. A chunk
    /// of plausible size is read whole in one read; a larger one is streamed.
    pub(crate) fn decode_chunk(
        &self,
        index: u64,
        position: u64,
        stored_size: u64,
        expected: u64,
    ) -> Result<Vec<u8>, Error> {
        if stored_size < HPI_CHUNK_HEADER_SIZE {
            return Err(Error::MalformedArchive(format!("chunk {index} size")));
        }
        if stored_size > HPI_CHUNK_READ_LIMIT {
            return self.decode_chunk_streamed(index, position, stored_size, expected);
        }
        // stored_size <= HPI_CHUNK_READ_LIMIT, so this is at most 128 KiB + 19.
        let mut chunk = vec![0u8; stored_size as usize];
        self.read_archive_bytes(position, &mut chunk)?;
        let (header, payload) = chunk.split_at_mut(HPI_CHUNK_HEADER_SIZE as usize);
        let fields = chunk_header(index, header, stored_size, expected)?;
        let sum = payload
            .iter()
            .fold(0u32, |sum, &value| sum.wrapping_add(u32::from(value)));
        if self.options.verify_checksums && sum != fields.checksum {
            return Err(Error::MalformedArchive(format!("chunk {index} checksum")));
        }
        if fields.encoded {
            unscramble(payload, 0);
        }
        let decoded = match fields.method {
            1 => decode_lz77(&mut SliceBytes(payload.iter()), fields.decompressed_size),
            2 => {
                let mut once = Some(&payload[..]);
                decode_zlib(
                    &mut |buf: &mut Vec<u8>| {
                        if let Some(piece) = once.take() {
                            buf.extend_from_slice(piece);
                        }
                        Ok(())
                    },
                    fields.decompressed_size,
                )
            }
            // The chunk byte selects the decoder; another method decodes nothing,
            // which the size check below rejects.
            _ => Ok(Vec::new()),
        };
        check_decoded(index, decoded, fields.decompressed_size)
    }

    /// `decode_chunk` for a chunk too large to buffer whole: the header first, then
    /// the payload streamed twice, for its checksum and to the decoder.
    pub(crate) fn decode_chunk_streamed(
        &self,
        index: u64,
        position: u64,
        stored_size: u64,
        expected: u64,
    ) -> Result<Vec<u8>, Error> {
        let mut header = [0u8; HPI_CHUNK_HEADER_SIZE as usize];
        self.read_archive_bytes(position, &mut header)?;
        let fields = chunk_header(index, &header, stored_size, expected)?;
        let payload_offset = position + HPI_CHUNK_HEADER_SIZE;
        let sum = self
            .checksum_payload(payload_offset, fields.payload_size)
            .map_err(|error| {
                Error::MalformedArchive(format!("chunk {index} checksum payload: {error}"))
            })?;
        if self.options.verify_checksums && sum != fields.checksum {
            return Err(Error::MalformedArchive(format!("chunk {index} checksum")));
        }
        let mut payload = PayloadReader {
            archive: self,
            offset: payload_offset,
            remaining: fields.payload_size,
            index: 0,
            encoded: fields.encoded,
        };
        let decoded = match fields.method {
            1 => decode_lz77(
                &mut StreamBytes {
                    reader: payload,
                    buffer: vec![0u8; STREAM_PIECE],
                    position: 0,
                    filled: 0,
                },
                fields.decompressed_size,
            ),
            2 => {
                let mut piece = vec![0u8; STREAM_PIECE];
                decode_zlib(
                    &mut |buf: &mut Vec<u8>| {
                        let read = payload.read(&mut piece).map_err(|e| e.to_string())?;
                        buf.extend_from_slice(piece.get(..read).unwrap_or(&[]));
                        Ok(())
                    },
                    fields.decompressed_size,
                )
            }
            _ => Ok(Vec::new()),
        };
        check_decoded(index, decoded, fields.decompressed_size)
    }

    fn checksum_payload(&self, offset: u64, size: u64) -> Result<u32, Error> {
        let mut reader = PayloadReader {
            archive: self,
            offset,
            remaining: size,
            index: 0,
            encoded: false,
        };
        let mut buffer = vec![0u8; CHECKSUM_PIECE];
        let mut sum = 0u32;
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                return Ok(sum);
            }
            for &value in buffer.get(..count).unwrap_or(&[]) {
                sum = sum.wrapping_add(u32::from(value));
            }
        }
    }

    fn check_archive_range(&self, offset: u64, len: u64) -> Result<(), Error> {
        if offset > self.size || len > self.size - offset {
            return Err(Error::MalformedArchive(
                "file data outside archive".to_string(),
            ));
        }
        Ok(())
    }

    fn read_archive_bytes(&self, offset: u64, data: &mut [u8]) -> Result<(), Error> {
        self.check_archive_range(offset, data.len() as u64)?;
        self.reader.read_exact_at(data, offset).map_err(Error::io)?;
        if self.key != 0 {
            decrypt(data, offset, self.key);
        }
        Ok(())
    }
}

/// A failed header or footer read: a short read rejects the container, anything
/// else is a malformed archive.
fn classify_gate_read(what: &str, error: io::Error) -> Error {
    if error.kind() == io::ErrorKind::UnexpectedEof {
        Error::RejectedArchive(format!("{what}: {error}"))
    } else {
        Error::MalformedArchive(format!("{what}: {error}"))
    }
}

/// Where the directory walk records what it finds.
struct IndexSink<'a> {
    name: &'a Path,
    archive_size: u64,
    max_file_bytes: u64,
    max_depth: u64,
    index: &'a mut ProviderIndex,
    indexed_entries: &'a mut Vec<EntryInfo>,
    retail_entry_indices: &'a mut Vec<usize>,
}

impl IndexSink<'_> {
    fn append(&mut self, info: EntryInfo, visible: bool) {
        if visible {
            self.retail_entry_indices.push(self.indexed_entries.len());
        }
        self.indexed_entries.push(info);
    }

    fn provenance(&self, logical: &[u8], original: &[u8], compression: &'static str) -> Provenance {
        Provenance {
            logical_path: logical.to_vec(),
            original_path: original.to_vec(),
            provider_type: "hpi",
            source_path: self.name.to_path_buf(),
            compression,
            ..Provenance::default()
        }
    }
}

struct DirectoryView<'a> {
    bytes: &'a [u8],
    start: u64,
    end: u64,
    entries_left: u64,
    strings_left: u64,
}

/// One directory being walked: Go's recursive `walkDirectory` call, kept on an
/// explicit stack.
struct Frame {
    node: u64,
    list: u64,
    count: u32,
    next: u32,
    parent: Vec<u8>,
    original_parent: Vec<u8>,
    visible: bool,
    /// The last entry index for each folded name in this directory.
    last_entry: HashMap<Vec<u8>, u32>,
}

impl DirectoryView<'_> {
    fn offset(&self, pos: u64, length: u64) -> Result<usize, Error> {
        // Check the upper endpoint before subtracting it, so a pointer past the
        // directory cannot wrap the subtraction.
        if pos < self.start || pos > self.end || length > self.end - pos {
            return Err(Error::MalformedArchive(format!(
                "directory pointer 0x{pos:x} length {length}"
            )));
        }
        usize::try_from(pos - self.start)
            .map_err(|_| Error::MalformedArchive("pointer overflow".to_string()))
    }

    fn u32(&self, pos: u64) -> Result<u32, Error> {
        let index = self.offset(pos, 4)?;
        le_u32(self.bytes, index)
            .ok_or_else(|| Error::MalformedArchive(format!("directory pointer 0x{pos:x} length 4")))
    }

    fn byte(&self, pos: u64) -> Result<u8, Error> {
        let index = self.offset(pos, 1)?;
        self.bytes
            .get(index)
            .copied()
            .ok_or_else(|| Error::MalformedArchive(format!("directory pointer 0x{pos:x} length 1")))
    }

    fn cstring(&mut self, pos: u64) -> Result<Vec<u8>, Error> {
        let index = self.offset(pos, 1)?;
        let rest = self.bytes.get(index..).unwrap_or(&[]);
        // Limit the scan itself, rather than charging only after a long shared name
        // has already consumed work. Each visit pays again.
        let budget = usize::try_from(self.strings_left).unwrap_or(usize::MAX);
        let scan = rest.get(..budget).unwrap_or(rest);
        match scan.iter().position(|&b| b == 0) {
            Some(end) => {
                self.strings_left -= end as u64 + 1;
                Ok(scan.get(..end).unwrap_or(&[]).to_vec())
            }
            None if scan.len() < rest.len() => Err(Error::MalformedArchive(
                "directory string work limit".to_string(),
            )),
            None => Err(Error::MalformedArchive(format!(
                "unterminated name at 0x{pos:x}"
            ))),
        }
    }

    /// Enters one directory node: Go's checks at the top of `walkDirectory`.
    fn enter(
        &mut self,
        sink: &IndexSink<'_>,
        on_stack: &mut HashSet<u64>,
        node: u64,
        parent: Vec<u8>,
        original_parent: Vec<u8>,
        visible: bool,
    ) -> Result<Frame, Error> {
        if on_stack.len() as u64 >= sink.max_depth {
            return Err(Error::MalformedArchive("directory depth limit".to_string()));
        }
        if !on_stack.insert(node) {
            return Err(Error::MalformedArchive(format!(
                "directory cycle at 0x{node:x}"
            )));
        }
        let count = self.u32(node)?;
        let list = u64::from(self.u32(node + 4)?);
        if list < self.start
            || list > self.end
            || u64::from(count) > (self.end - list) / HPI_ENTRY_SIZE
        {
            return Err(Error::MalformedArchive(
                "entry list overflows directory".to_string(),
            ));
        }
        // Reserve the entire list before building its lookup map or scanning names.
        // Revisited and shadowed subtrees consume the same global budget.
        if u64::from(count) > self.entries_left {
            return Err(Error::MalformedArchive("directory entry limit".to_string()));
        }
        self.entries_left -= u64::from(count);
        // Lookup chooses each component from the directory's last matching entry;
        // earlier duplicates contribute no reachable descendants. Every subtree is
        // still validated and every authored entry kept for diagnostics.
        let mut last_entry = HashMap::new();
        for i in 0..count {
            let name_offset = self.u32(list + u64::from(i) * HPI_ENTRY_SIZE)?;
            let name = self.cstring(u64::from(name_offset))?;
            last_entry.insert(fold_logical_name(&name), i);
        }
        Ok(Frame {
            node,
            list,
            count,
            next: 0,
            parent,
            original_parent,
            visible,
            last_entry,
        })
    }

    fn walk(&mut self, sink: &mut IndexSink<'_>, root_offset: u64) -> Result<(), Error> {
        let mut on_stack: HashSet<u64> = HashSet::new();
        let root = self.enter(
            sink,
            &mut on_stack,
            root_offset,
            Vec::new(),
            Vec::new(),
            true,
        )?;
        let mut stack = vec![root];
        while let Some(frame) = stack.last_mut() {
            if frame.next >= frame.count {
                on_stack.remove(&frame.node);
                stack.pop();
                continue;
            }
            let i = frame.next;
            frame.next += 1;
            let entry_offset = frame.list + u64::from(i) * HPI_ENTRY_SIZE;
            let name_offset = self.u32(entry_offset)?;
            let data_offset = u64::from(self.u32(entry_offset + 4)?);
            let flag = self.byte(entry_offset + 8)?;
            let name = self.cstring(u64::from(name_offset))?;
            // Charge both joined inputs before join_path allocates or normalizes
            // them, in widened arithmetic.
            let mut path_bytes = frame.parent.len() as u64
                + frame.original_parent.len() as u64
                + 2 * name.len() as u64;
            if !frame.parent.is_empty() {
                path_bytes += 1;
            }
            if !frame.original_parent.is_empty() {
                path_bytes += 1;
            }
            if path_bytes > self.strings_left {
                return Err(Error::MalformedArchive(
                    "directory string work limit".to_string(),
                ));
            }
            self.strings_left -= path_bytes;
            let logical = join_path(&frame.parent, &name).map_err(|error| {
                Error::MalformedArchive(format!("entry {:?}: {error}", show(&name)))
            })?;
            let original = original_join(&frame.original_parent, &name);
            // Go read a missing map key as 0; every name was inserted by `enter`,
            // so the key is always present, but the default is kept.
            let last = frame
                .last_entry
                .get(&fold_logical_name(&name))
                .copied()
                .unwrap_or(0);
            let visible = frame.visible;
            let lookup_visible = visible && last == i;
            if flag & 1 != 0 {
                let info = EntryInfo {
                    name: name.clone(),
                    path: logical.clone(),
                    original_path: original.clone(),
                    is_dir: true,
                    source: sink.provenance(&logical, &original, ""),
                    ..EntryInfo::default()
                };
                if lookup_visible {
                    sink.index.entries.insert(
                        logical.clone(),
                        ProviderEntry {
                            info: info.clone(),
                            source: EntrySource::Dir,
                        },
                    );
                }
                sink.append(info, visible);
                let child = self.enter(
                    sink,
                    &mut on_stack,
                    data_offset,
                    logical,
                    original,
                    lookup_visible,
                )?;
                stack.push(child);
                continue;
            }
            let record_offset = data_offset;
            let data = u64::from(self.u32(record_offset)?);
            let size = u64::from(self.u32(record_offset + 4)?);
            let compression = self.byte(record_offset + 8)?;
            if compression > 2 {
                return Err(Error::MalformedArchive(format!(
                    "file {:?} has compression {compression}",
                    show(&logical)
                )));
            }
            if size > sink.max_file_bytes {
                return Err(Error::MalformedArchive(format!(
                    "file {:?} exceeds size limit",
                    show(&logical)
                )));
            }
            let record = HpiRecord {
                data_offset: data,
                size,
                compression,
            };
            if compression == 0
                && (record.data_offset > sink.archive_size
                    || record.size > sink.archive_size - record.data_offset)
            {
                return Err(Error::MalformedArchive(format!(
                    "stored file {:?} is outside archive",
                    show(&logical)
                )));
            }
            let info = EntryInfo {
                name,
                path: logical.clone(),
                original_path: original.clone(),
                size,
                is_dir: false,
                source: sink.provenance(&logical, &original, compression_name(compression)),
            };
            if lookup_visible {
                sink.index.entries.insert(
                    logical,
                    ProviderEntry {
                        info: info.clone(),
                        source: EntrySource::Record(record),
                    },
                );
            }
            sink.append(info, visible);
        }
        Ok(())
    }
}

fn compression_name(compression: u8) -> &'static str {
    match compression {
        0 => "stored",
        1 => "lz77",
        2 => "zlib",
        _ => "unknown",
    }
}

struct ChunkFields {
    method: u8,
    encoded: bool,
    payload_size: u64,
    decompressed_size: u64,
    checksum: u32,
}

/// Validates a chunk header against its table entry: the marker, the payload size
/// the entry implies and the decompressed span. The chunk header selects the
/// decoder; retail does not require it to match the record's compression byte.
fn chunk_header(
    index: u64,
    header: &[u8],
    stored_size: u64,
    expected: u64,
) -> Result<ChunkFields, Error> {
    if header.get(0..4) != Some(b"SQSH".as_slice()) {
        return Err(Error::MalformedArchive(format!("chunk {index} marker")));
    }
    let fields_error = || Error::MalformedArchive(format!("chunk {index} size fields"));
    let payload_size = u64::from(le_u32(header, 7).ok_or_else(fields_error)?);
    let decompressed_size = u64::from(le_u32(header, 11).ok_or_else(fields_error)?);
    if payload_size != stored_size - HPI_CHUNK_HEADER_SIZE || decompressed_size != expected {
        return Err(fields_error());
    }
    Ok(ChunkFields {
        method: header.get(5).copied().unwrap_or(0),
        encoded: header.get(6).copied().unwrap_or(0) != 0,
        payload_size,
        decompressed_size,
        checksum: le_u32(header, 15).ok_or_else(fields_error)?,
    })
}

fn check_decoded(
    index: u64,
    decoded: Result<Vec<u8>, String>,
    decompressed_size: u64,
) -> Result<Vec<u8>, Error> {
    let decoded =
        decoded.map_err(|error| Error::MalformedArchive(format!("chunk {index}: {error}")))?;
    if decoded.len() as u64 != decompressed_size {
        return Err(Error::MalformedArchive(format!(
            "chunk {index} output size"
        )));
    }
    Ok(decoded)
}

/// Reverses the SQSH payload transform over `data`, the payload's bytes from
/// `index` on. Go computed `byte(uint16(b) - uint16(pos)) ^ byte(pos)`; the low
/// byte of that wrapping subtraction is the wrapping subtraction of the low bytes.
pub(crate) fn unscramble(data: &mut [u8], index: u64) {
    for (i, value) in data.iter_mut().enumerate() {
        let position = index.wrapping_add(i as u64) as u8;
        *value = value.wrapping_sub(position) ^ position;
    }
}

/// The archive-wide obfuscation: each byte is XORed with the low byte of its
/// absolute offset (taken modulo 2^32, as Go's `uint32` conversion wrapped) and the
/// key, and inverted.
fn decrypt(data: &mut [u8], absolute_offset: u64, key: u8) {
    for (i, value) in data.iter_mut().enumerate() {
        let position = absolute_offset.wrapping_add(i as u64) as u32;
        *value = ((position as u8) ^ key) ^ !*value;
    }
}

/// A chunk payload read from the archive in bounded pieces. The checksum is taken
/// before the payload transform, matching the SQSH wire order.
struct PayloadReader<'a> {
    archive: &'a Archive,
    offset: u64,
    remaining: u64,
    index: u64,
    encoded: bool,
}

impl PayloadReader<'_> {
    /// Fills up to `data.len()` bytes; 0 means the payload is exhausted.
    fn read(&mut self, data: &mut [u8]) -> Result<usize, Error> {
        if self.remaining == 0 {
            return Ok(0);
        }
        let count = usize::try_from(self.remaining)
            .unwrap_or(usize::MAX)
            .min(data.len());
        let piece = data.get_mut(..count).unwrap_or(&mut []);
        self.archive.read_archive_bytes(self.offset, piece)?;
        if self.encoded {
            unscramble(piece, self.index);
        }
        self.offset += count as u64;
        self.remaining -= count as u64;
        self.index += count as u64;
        Ok(count)
    }
}

/// A byte at a time, for the LZ77 decoder (Go `io.ByteReader`). `None` is the end
/// of input or a read error; Go's decoder treated both as truncation.
pub(crate) trait ByteSource {
    fn read_byte(&mut self) -> Option<u8>;
}

pub(crate) struct SliceBytes<'a>(pub(crate) std::slice::Iter<'a, u8>);

impl ByteSource for SliceBytes<'_> {
    fn read_byte(&mut self) -> Option<u8> {
        self.0.next().copied()
    }
}

/// A buffered streamed payload (Go's `bufio.Reader` over the payload reader).
struct StreamBytes<'a> {
    reader: PayloadReader<'a>,
    buffer: Vec<u8>,
    position: usize,
    filled: usize,
}

impl ByteSource for StreamBytes<'_> {
    fn read_byte(&mut self) -> Option<u8> {
        if self.position >= self.filled {
            self.filled = self.reader.read(&mut self.buffer).ok()?;
            self.position = 0;
            if self.filled == 0 {
                return None;
            }
        }
        let value = self.buffer.get(self.position).copied();
        self.position += 1;
        value
    }
}

/// Decodes one zlib chunk payload of exactly `expected` bytes.
///
/// `input` appends the next piece of compressed data to its buffer, or nothing at
/// the end of the input. The rules are Go's: fewer bytes than declared is an error;
/// more is an error; a stream that stops after every declared byte without a final
/// deflate block or Adler-32 trailer is accepted (third-party packers write such
/// chunks, and retail compares only the produced length); a bad trailer is an error.
pub(crate) fn decode_zlib(
    input: &mut dyn FnMut(&mut Vec<u8>) -> Result<(), String>,
    expected: u64,
) -> Result<Vec<u8>, String> {
    let expected = usize::try_from(expected)
        .ok()
        .filter(|&expected| expected < usize::MAX)
        .ok_or_else(|| "zlib output is too large".to_string())?;
    // One byte past the declared size, so an overlong stream is seen.
    let mut out = vec![0u8; expected + 1];
    let mut out_pos = 0usize;
    let mut decompressor = Box::<DecompressorOxide>::default();
    let mut buffer: Vec<u8> = Vec::new();
    let mut start = 0usize;
    let mut consumed_total = 0usize;
    let mut eof = false;
    let mut refill = |buffer: &mut Vec<u8>, eof: &mut bool| -> Result<(), String> {
        let before = buffer.len();
        input(buffer)?;
        if buffer.len() == before {
            *eof = true;
        }
        Ok(())
    };
    let truncated = loop {
        if start == buffer.len() && !eof {
            buffer.clear();
            start = 0;
            refill(&mut buffer, &mut eof)?;
        }
        let mut flags = TINFL_FLAG_PARSE_ZLIB_HEADER | TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF;
        if !eof {
            flags |= TINFL_FLAG_HAS_MORE_INPUT;
        }
        let pending = buffer.get(start..).unwrap_or(&[]);
        let (status, consumed, written) =
            decompress(&mut decompressor, pending, &mut out, out_pos, flags);
        start += consumed;
        consumed_total += consumed;
        out_pos += written;
        match status {
            TINFLStatus::Done => break false,
            TINFLStatus::NeedsMoreInput if !eof => {
                if start < buffer.len() {
                    // Keep what the decoder left and append the next piece.
                    buffer.drain(..start);
                    start = 0;
                    refill(&mut buffer, &mut eof)?;
                }
            }
            TINFLStatus::NeedsMoreInput | TINFLStatus::FailedCannotMakeProgress => break true,
            TINFLStatus::HasMoreOutput => {
                return Err(format!("zlib produced more than {expected} bytes"))
            }
            TINFLStatus::Adler32Mismatch => return Err("zlib: invalid checksum".to_string()),
            _ => return Err("zlib: invalid stream".to_string()),
        }
    };
    if truncated && consumed_total < 2 {
        // Go's zlib reader fails before decoding when the two header bytes are
        // missing.
        return Err("zlib: truncated header".to_string());
    }
    if out_pos < expected {
        return Err(format!("zlib produced fewer than {expected} bytes"));
    }
    if out_pos > expected {
        return Err(format!("zlib produced more than {expected} bytes"));
    }
    out.truncate(expected);
    Ok(out)
}

/// Decodes an LZ77 payload of exactly `expected` bytes. Retail terminates on a zero
/// match position, then checks the output size; reaching the declared size alone is
/// not a terminator.
pub(crate) fn decode_lz77(source: &mut dyn ByteSource, expected: u64) -> Result<Vec<u8>, String> {
    let expected = usize::try_from(expected).map_err(|_| "LZ77 output is too large".to_string())?;
    let mut output: Vec<u8> = Vec::with_capacity(expected);
    let mut window = [0u8; 4096];
    let mut write = 1usize;
    let mut append = |output: &mut Vec<u8>, window: &mut [u8; 4096], value: u8| {
        if output.len() >= expected {
            return Err("LZ77 output exceeds expected size".to_string());
        }
        output.push(value);
        if let Some(slot) = window.get_mut(write) {
            *slot = value;
        }
        write = (write + 1) & 0x0fff;
        Ok(())
    };
    loop {
        let tag = source
            .read_byte()
            .ok_or_else(|| "LZ77 tag is truncated".to_string())?;
        for bit in 0..8 {
            if tag & (1 << bit) == 0 {
                let value = source
                    .read_byte()
                    .ok_or_else(|| "LZ77 literal is truncated".to_string())?;
                append(&mut output, &mut window, value)?;
                continue;
            }
            let low = source
                .read_byte()
                .ok_or_else(|| "LZ77 match is truncated".to_string())?;
            let high = source
                .read_byte()
                .ok_or_else(|| "LZ77 match is truncated".to_string())?;
            let word = u16::from(low) | (u16::from(high) << 8);
            let mut position = usize::from(word >> 4);
            let length = usize::from(word & 0x0f) + 2;
            if position == 0 {
                if output.len() != expected {
                    return Err("LZ77 terminator before expected output".to_string());
                }
                return Ok(output);
            }
            for _ in 0..length {
                let value = window.get(position).copied().unwrap_or(0);
                append(&mut output, &mut window, value)?;
                position = (position + 1) & 0x0fff;
            }
        }
    }
}
