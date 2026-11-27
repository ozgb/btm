// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/vfs.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! The overlay: mounts in precedence order, and the lookups that resolve through
//! them.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use super::hpi::{Archive, ArchiveOptions, ReadAt, DEFAULT_MAX_FILE_BYTES};
use super::layout::Layout;
use super::loose::LooseProvider;
use super::path::{clean_path, fold_logical_name};
use super::provider_index::{EntrySource, ProviderEntry, ProviderIndex};
use super::rehome::RehomedArchive;
use super::{host, show, Error};

/// The provider that won an overlay lookup.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Provenance {
    pub logical_path: Vec<u8>,
    pub original_path: Vec<u8>,
    /// `"hpi"`, `"directory"`, or empty for a synthetic entry.
    pub provider_type: &'static str,
    /// The archive name, or the loose file's host path. Empty when unknown.
    pub source_path: PathBuf,
    /// The absolute loose-mount root; empty for archives.
    pub mount_root: PathBuf,
    pub priority: i64,
    pub mount_order: usize,
    /// `"stored"`, `"lz77"` or `"zlib"` for archive files; empty otherwise.
    pub compression: &'static str,
}

impl Provenance {
    /// The portable identity of the winning provider: the archive file name for
    /// archives, or the path relative to the loose mount root. It never embeds an
    /// absolute host path, so manifests and their hashes are stable across machines.
    pub fn provider_id(&self) -> String {
        let has_source = !self.source_path.as_os_str().is_empty();
        if !self.mount_root.as_os_str().is_empty() && has_source {
            if let Some(rel) = host::rel_below(&self.mount_root, &self.source_path) {
                if !rel.starts_with("..") {
                    return rel;
                }
            }
        }
        if has_source {
            return host::base(&self.source_path);
        }
        self.provider_type.to_string()
    }
}

/// A logical file or directory, described without opening its data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntryInfo {
    pub name: Vec<u8>,
    pub path: Vec<u8>,
    pub original_path: Vec<u8>,
    pub size: u64,
    pub is_dir: bool,
    pub source: Provenance,
}

#[derive(Debug)]
enum FileData {
    Host(std::fs::File),
    Memory(Cursor<Vec<u8>>),
}

/// An open logical file: a host file streaming from disk, or an archive record
/// decoded whole into memory.
#[derive(Debug)]
pub struct File {
    data: FileData,
    info: EntryInfo,
}

impl File {
    pub(crate) fn memory(data: Vec<u8>, info: EntryInfo) -> File {
        File {
            data: FileData::Memory(Cursor::new(data)),
            info,
        }
    }

    fn host(path: &Path, info: EntryInfo) -> Result<File, Error> {
        let file = std::fs::File::open(path).map_err(|source| Error::Open {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(File {
            data: FileData::Host(file),
            info,
        })
    }

    /// The entry the overlay found this file under.
    pub fn info(&self) -> &EntryInfo {
        &self.info
    }

    /// The complete file when opening it already produced it (an archive record);
    /// `None` for a host file (Go `WholeFile`).
    pub fn whole(&self) -> Option<&[u8]> {
        match &self.data {
            FileData::Memory(cursor) => Some(cursor.get_ref()),
            FileData::Host(_) => None,
        }
    }

    /// The file's contents: the decoded buffer when there is one (whatever the read
    /// position), otherwise the rest of the host file (Go `WholeBytes`).
    pub fn into_bytes(self) -> io::Result<Vec<u8>> {
        match self.data {
            FileData::Memory(cursor) => Ok(cursor.into_inner()),
            FileData::Host(mut file) => {
                let mut data = Vec::new();
                file.read_to_end(&mut data)?;
                Ok(data)
            }
        }
    }

    /// Reads from `offset` without moving the read position (Go `ReadAt`). Fewer
    /// bytes than `buf.len()` are returned only at the end of the file.
    pub fn read_at(&mut self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        match &mut self.data {
            FileData::Memory(cursor) => {
                let data = cursor.get_ref();
                let start = usize::try_from(offset)
                    .unwrap_or(usize::MAX)
                    .min(data.len());
                let available = data.get(start..).unwrap_or(&[]);
                let count = available.len().min(buf.len());
                if let (Some(to), Some(from)) = (buf.get_mut(..count), available.get(..count)) {
                    to.copy_from_slice(from);
                }
                Ok(count)
            }
            FileData::Host(file) => {
                let saved = file.stream_position()?;
                file.seek(SeekFrom::Start(offset))?;
                let mut filled = 0;
                let result = loop {
                    match file.read(buf.get_mut(filled..).unwrap_or(&mut [])) {
                        Ok(0) => break Ok(filled),
                        Ok(read) => {
                            filled += read;
                            if filled == buf.len() {
                                break Ok(filled);
                            }
                        }
                        Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                        Err(error) => break Err(error),
                    }
                };
                file.seek(SeekFrom::Start(saved))?;
                result
            }
        }
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match &mut self.data {
            FileData::Host(file) => file.read(buf),
            FileData::Memory(cursor) => cursor.read(buf),
        }
    }
}

impl Seek for File {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match &mut self.data {
            FileData::Host(file) => file.seek(pos),
            FileData::Memory(cursor) => cursor.seek(pos),
        }
    }
}

/// One mounted provider.
pub(crate) enum Provider {
    Loose(LooseProvider),
    Archive(Archive),
    Rehomed(RehomedArchive),
}

impl Provider {
    fn index(&self) -> &ProviderIndex {
        match self {
            Provider::Loose(loose) => &loose.index,
            Provider::Archive(archive) => &archive.index,
            Provider::Rehomed(rehomed) => &rehomed.index,
        }
    }

    pub(crate) fn lookup(&self, name: &[u8]) -> Option<&ProviderEntry> {
        self.index().lookup(name)
    }

    fn children(&self, parent: &[u8]) -> Vec<&ProviderEntry> {
        self.index().children(parent)
    }

    fn audit_entries(&self) -> &[EntryInfo] {
        match self {
            Provider::Loose(loose) => &loose.indexed_entries,
            Provider::Archive(archive) => archive.audit_entries(),
            Provider::Rehomed(rehomed) => &rehomed.audit,
        }
    }

    fn retail_entries(&self) -> Box<dyn Iterator<Item = &EntryInfo> + '_> {
        match self {
            Provider::Loose(loose) => Box::new(loose.indexed_entries.iter()),
            Provider::Archive(archive) => Box::new(archive.retail_entries()),
            Provider::Rehomed(rehomed) => Box::new(rehomed.retail.iter()),
        }
    }

    fn archive(&self) -> Option<&Archive> {
        match self {
            Provider::Loose(_) => None,
            Provider::Archive(archive) => Some(archive),
            Provider::Rehomed(rehomed) => Some(&rehomed.archive),
        }
    }

    fn open(&self, entry: &ProviderEntry) -> Result<File, Error> {
        match &entry.source {
            EntrySource::Dir => Err(Error::IsDir(show(&entry.info.path))),
            EntrySource::Loose(path) => File::host(path, entry.info.clone()),
            EntrySource::Record(record) => match self.archive() {
                Some(archive) => archive.open_record(*record, &entry.info),
                None => Err(Error::Unsupported("archive record in a loose mount")),
            },
        }
    }

    /// A partial read, or `None` when the entry cannot do better than a full read
    /// (Go's nil `readRange`).
    fn read_range(
        &self,
        entry: &ProviderEntry,
        offset: u64,
        length: Option<usize>,
    ) -> Option<Result<Vec<u8>, Error>> {
        match &entry.source {
            EntrySource::Dir => None,
            EntrySource::Loose(path) => Some(super::loose::read_file_range(path, offset, length)),
            EntrySource::Record(record) => self
                .archive()
                .map(|archive| archive.read_record_range(*record, offset, length)),
        }
    }
}

struct MountedProvider {
    provider: Provider,
    priority: i64,
    order: usize,
}

/// An overlay of loose directories and HPI-family archives. Higher priorities win;
/// equal-priority mounts added later win.
#[derive(Default)]
pub struct Fs {
    /// Kept in search order at all times: priority descending, then mount order
    /// descending.
    mounts: Vec<MountedProvider>,
    next_order: usize,
    /// Canonical full path to the portable label of its provider.
    mounted_by: HashMap<String, String>,
    /// Mount-time observations worth reporting.
    notes: Vec<String>,
}

impl std::fmt::Debug for Fs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fs")
            .field("mounts", &self.mounts.len())
            .field("next_order", &self.next_order)
            .field("notes", &self.notes)
            .finish()
    }
}

/// One mounted provider in precedence order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProviderInfo {
    /// The archive path, or the directory root for a loose mount.
    pub id: String,
    /// `"hpi"`, `"directory"`, ...
    pub provider_type: &'static str,
    pub priority: i64,
    pub mount_order: usize,
    pub files: usize,
}

/// An explicit content-overlay tier. Higher tiers win.
pub type MountTier = u8;

/// The per-extension mount tier policy: the tier each archive family joins, and
/// the revision string that identifies the policy itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountPlan {
    pub revision: String,
    pub hpi: MountTier,
    pub ccx: MountTier,
    pub gp3: MountTier,
    pub ufo: MountTier,
    pub loose: MountTier,
}

/// The default tier policy. It is a deliberate catalog policy, not a reproduction
/// of retail's loader: retail mounts loose data, then the literal `rev31.GP3`, then
/// CCX, UFO and HPI, in Windows directory enumeration order. This mounts every
/// `.gp3/.gp4/.gpf/.swx` file at the revision tier, sorts archives lexically inside
/// each tier so the equal-tier winner is stable across filesystems, and records
/// every shadowed mount in the manifest.
pub fn default_retail_mount_plan() -> MountPlan {
    MountPlan {
        revision: "retail-binary-tiers-openta-lexical-1".to_string(),
        hpi: 1,
        ufo: 2,
        ccx: 3,
        gp3: 4,
        loose: 10,
    }
}

impl MountPlan {
    /// Rejects a plan that leaves any tier or the revision unset.
    pub fn validate(&self) -> Result<(), Error> {
        if self.revision.is_empty()
            || self.hpi == 0
            || self.ccx == 0
            || self.gp3 == 0
            || self.ufo == 0
            || self.loose == 0
        {
            return Err(Error::InvalidMountPlan);
        }
        Ok(())
    }
}

/// Reports the providers that hold a path, best precedence first (Go's optional
/// `Sources` view).
pub trait SourcesView {
    fn sources(&self, name: &[u8]) -> Vec<EntryInfo>;
}

/// A stable identity of the mounted content set (Go's optional `ManifestHash`).
pub trait ManifestView {
    fn manifest_hash(&self) -> Result<String, Error>;
}

/// Partial reads (Go `RangeReader`). A header probe over a whole install must not
/// decompress every archived file to read its first bytes.
pub trait RangeReader {
    /// `length` bytes from `offset`; `None` reads to the end, and a length past the
    /// end is clamped to it. An offset at the end is an empty read; past it, an
    /// error.
    fn read_file_range(
        &self,
        name: &[u8],
        offset: u64,
        length: Option<usize>,
    ) -> Result<Vec<u8>, Error>;
}

/// Directory enumeration in provider order rather than sorted order (Go
/// `retailDirReader`).
pub trait RetailDirReader {
    fn retail_read_dir(&self, name: &[u8]) -> Result<Vec<EntryInfo>, Error>;
}

/// The read surface content loaders need (Go `FSOps`). The `as_*` methods answer
/// Go's type assertions for the optional views; each defaults to `None`.
pub trait FsOps {
    fn open(&self, name: &[u8]) -> Result<File, Error>;
    fn read_file_limit(&self, name: &[u8], max: u64) -> Result<Vec<u8>, Error>;
    fn read_dir(&self, name: &[u8]) -> Result<Vec<EntryInfo>, Error>;
    fn stat(&self, name: &[u8]) -> Result<EntryInfo, Error>;
    fn cache_stamp(&self, name: &[u8]) -> Result<String, Error>;

    fn as_sources(&self) -> Option<&dyn SourcesView> {
        None
    }
    fn as_manifest(&self) -> Option<&dyn ManifestView> {
        None
    }
    fn as_range_reader(&self) -> Option<&dyn RangeReader> {
        None
    }
    fn as_retail_dir_reader(&self) -> Option<&dyn RetailDirReader> {
        None
    }
}

/// One root of [`Fs::mount_game_roots`].
///
/// `archives`, when set, admits only the archives whose file names it accepts;
/// `no_loose` leaves the directory's loose files unmounted. `extra` names archive
/// files kept elsewhere that mount as if they sat in `dir`: each joins its
/// extension's tier and the tier's lexical order by file name, and replaces a
/// same-named archive of `dir`. `archives` does not filter them. `extra_layout`
/// gives the layout an extra archive is presented under; `extra_tier` gives an extra
/// archive a tier other than its extension's, which must be below the loose tier.
#[derive(Default)]
pub struct GameRoot {
    pub dir: PathBuf,
    pub archives: Option<ArchiveFilter>,
    pub no_loose: bool,
    pub extra: Vec<PathBuf>,
    pub extra_layout: Option<LayoutChooser>,
    pub extra_tier: Option<TierChooser>,
}

/// Admits an archive of a root by its file name.
pub type ArchiveFilter = Box<dyn Fn(&str) -> bool>;
/// The layout an extra archive is presented under, by its full path.
pub type LayoutChooser = Box<dyn Fn(&Path) -> Layout>;
/// An extra archive's tier override, by its full path; `None` keeps its extension's.
pub type TierChooser = Box<dyn Fn(&Path) -> Option<MountTier>>;

/// A shared read view, as [`Layout::apply`] takes and returns it.
pub type SharedFs<'a> = std::sync::Arc<dyn FsOps + Send + Sync + 'a>;

impl GameRoot {
    /// A root that mounts everything in `dir`.
    pub fn new(dir: impl Into<PathBuf>) -> GameRoot {
        GameRoot {
            dir: dir.into(),
            ..GameRoot::default()
        }
    }
}

/// One archive a game directory mount plans.
struct GameArchive {
    name: String,
    full: PathBuf,
    layout: Layout,
    /// An extra archive's tier override; it replaces its extension's tier.
    tier: Option<MountTier>,
}

/// Whether a game directory's file is one the mount discovers as an archive: an
/// HPI, UFO or CCX, or a patch-tier GP3, GP4, GPF or SWX.
pub fn is_archive_name(name: &str) -> bool {
    matches!(
        host::lower(host::ext(name)).as_str(),
        ".hpi" | ".ufo" | ".ccx" | ".gp3" | ".gp4" | ".gpf" | ".swx"
    )
}

/// Folds a provider path the way retail's mount dedup does: full path, compared
/// case-insensitively.
fn canonical_mount_key(name: &Path) -> String {
    let absolute = host::abs(name).unwrap_or_else(|_| name.to_path_buf());
    host::lower(&host::lossy(&host::clean(&absolute)))
}

/// Go `time.Time.UnixNano` of a modification time.
fn unix_nanos(time: std::time::SystemTime) -> i64 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(after) => i64::try_from(after.as_nanos()).unwrap_or(i64::MAX),
        Err(before) => i64::try_from(before.duration().as_nanos())
            .map(|nanos| -nanos)
            .unwrap_or(i64::MIN),
    }
}

/// Reads at most `max` bytes of a file, checking indexed metadata before opening
/// the provider. Archive opens decode their whole record, so this preserves the
/// caller's budget before any payload allocation.
fn read_file_limit_with(
    open: impl FnOnce(&[u8]) -> Result<File, Error>,
    stat: impl FnOnce(&[u8]) -> Result<EntryInfo, Error>,
    name: &[u8],
    max: u64,
) -> Result<Vec<u8>, Error> {
    if max == 0 {
        return Err(Error::TooLarge(format!("invalid limit {max}")));
    }
    let info = stat(name)?;
    if info.is_dir {
        return Err(Error::IsDir(show(name)));
    }
    if info.size > max {
        return Err(Error::TooLarge(format!(
            "{} is {} bytes (limit {max})",
            show(name),
            info.size
        )));
    }
    let file = open(name)?;
    let data = match file.data {
        FileData::Memory(cursor) => cursor.into_inner(),
        FileData::Host(host_file) => {
            // The extra byte tells an exactly-full file from a truncated read.
            let mut data = Vec::new();
            host_file
                .take(max.saturating_add(1))
                .read_to_end(&mut data)
                .map_err(Error::io)?;
            data
        }
    };
    if data.len() as u64 > max {
        return Err(Error::TooLarge(format!(
            "{} exceeds {max} bytes",
            show(name)
        )));
    }
    Ok(data)
}

impl Fs {
    /// An empty overlay with no mounts.
    pub fn new() -> Fs {
        Fs::default()
    }

    /// The mounted set, best precedence first. Diagnostics name providers with this
    /// rather than with per-entry source paths, which for a loose mount are files.
    pub fn providers(&self) -> Vec<ProviderInfo> {
        let mut infos = Vec::with_capacity(self.mounts.len());
        for mount in &self.mounts {
            let mut info = ProviderInfo {
                priority: mount.priority,
                mount_order: mount.order,
                ..ProviderInfo::default()
            };
            for entry in mount.provider.audit_entries() {
                if entry.path.is_empty() && entry.is_dir {
                    // The provider's own root: for a loose mount this is the
                    // directory, which is the identity to report.
                    info.provider_type = entry.source.provider_type;
                    info.id = host::lossy(&entry.source.source_path);
                    continue;
                }
                if entry.is_dir {
                    continue;
                }
                info.files += 1;
                if info.provider_type.is_empty() {
                    info.provider_type = entry.source.provider_type;
                }
                if info.id.is_empty() {
                    if entry.source.provider_type == "directory" {
                        // Loose entries carry per-file host paths; recover the mount
                        // root by trimming the logical path, which is folded to lower
                        // case and so cannot be compared byte for byte.
                        let host_path = host::to_slash(&host::lossy(&entry.source.source_path));
                        let bytes = host_path.as_bytes();
                        let split = bytes.len().checked_sub(entry.path.len()).filter(|&n| n > 0);
                        let matched = split.and_then(|n| {
                            let tail = bytes.get(n..)?;
                            (fold_logical_name(tail) == entry.path).then_some(n)
                        });
                        info.id = match matched {
                            Some(n) => {
                                let head = bytes.get(..n).unwrap_or(&[]);
                                let head = head.strip_suffix(b"/").unwrap_or(head);
                                String::from_utf8_lossy(head).into_owned()
                            }
                            None => host_path,
                        };
                    } else {
                        info.id = host::lossy(&entry.source.source_path);
                    }
                }
            }
            if info.id.is_empty() {
                info.id = info.provider_type.to_string();
            }
            infos.push(info);
        }
        infos
    }

    /// The mounted provider identities in precedence order.
    pub fn provider_ids(&self) -> Vec<String> {
        self.providers().into_iter().map(|info| info.id).collect()
    }

    /// Mount-time observations a caller should surface: suppressed duplicate mounts
    /// and rejected archive candidates.
    pub fn notes(&self) -> Vec<String> {
        self.notes.clone()
    }

    /// How many mounts are ordered (0 is the winner).
    pub fn mount_count(&self) -> usize {
        self.mounts.len()
    }

    /// Opens a logical file from the `index`-th mount in precedence order alone.
    pub fn open_mount(&self, index: usize, name: impl AsRef<[u8]>) -> Result<File, Error> {
        let name = name.as_ref();
        let (provider, entry) = self.mount_entry(index, name)?;
        open_entry(provider, entry, name)
    }

    fn mount_entry(&self, index: usize, name: &[u8]) -> Result<(&Provider, &ProviderEntry), Error> {
        let logical = clean_path(name)?;
        let mount = self
            .mounts
            .get(index)
            .ok_or_else(|| Error::NotFound(format!("mount {index}")))?;
        match mount.provider.lookup(&logical) {
            Some(entry) => Ok((&mount.provider, entry)),
            None => Err(Error::NotFound(show(name))),
        }
    }

    /// The mount-pinned read view for shadow inspection.
    pub fn pinned(&self, index: usize) -> PinnedMount<'_> {
        PinnedMount { fs: self, index }
    }

    /// Every indexed provider entry, including entries the overlay shadows, sorted
    /// by path, then priority and mount order descending, then authored path. For
    /// diagnostic tools; content consumers use `read_dir`, `stat` and `open`.
    ///
    /// Go's `sort.Slice` is not stable; this sort is. They differ only among entries
    /// equal on every key (duplicate authored entries of one archive), which keep
    /// their index order here.
    pub fn entries(&self) -> Vec<EntryInfo> {
        let mut result: Vec<EntryInfo> = self
            .mounts
            .iter()
            .flat_map(|mount| mount.provider.audit_entries().iter().cloned())
            .collect();
        result.sort_by(|a, b| {
            a.path
                .cmp(&b.path)
                .then_with(|| b.source.priority.cmp(&a.source.priority))
                .then_with(|| b.source.mount_order.cmp(&a.source.mount_order))
                .then_with(|| a.original_path.cmp(&b.original_path))
        });
        result
    }

    /// A cheap host-side freshness token for a winning logical file. It is cache
    /// metadata only and never part of a manifest hash.
    pub fn cache_stamp(&self, name: impl AsRef<[u8]>) -> Result<String, Error> {
        let info = self.stat(name)?;
        Ok(stamp_for(&info))
    }

    /// Indexes a loose directory's names and metadata without reading contents. An
    /// equal full path already mounted is suppressed.
    pub fn mount_directory(&mut self, root: impl AsRef<Path>, priority: i64) -> Result<(), Error> {
        let root = root.as_ref();
        if self.already_mounted(root, "loose root") {
            return Ok(());
        }
        let provider = LooseProvider::new(root, priority, self.next_order)?;
        let label = format!("loose root (mount {})", self.next_order);
        self.remember_mount(root, label);
        self.insert_mount(Provider::Loose(provider), priority);
        Ok(())
    }

    /// Opens and indexes an archive. Only its header and directory are read here.
    /// An equal full path already mounted is suppressed, and answers `None`.
    pub fn mount_archive(
        &mut self,
        path: impl AsRef<Path>,
        priority: i64,
    ) -> Result<Option<&Archive>, Error> {
        let path = path.as_ref();
        if self.already_mounted(path, &host::base(path)) {
            return Ok(None);
        }
        let mut archive = Archive::open(path, ArchiveOptions::default())?;
        self.remember_mount(path, host::base(path));
        archive.set_mount_info(priority, self.next_order);
        let at = self.insert_mount(Provider::Archive(archive), priority);
        Ok(self
            .mounts
            .get(at)
            .and_then(|mount| mount.provider.archive()))
    }

    /// Mounts an archive read from any random-access reader, such as a nested
    /// archive or a test fixture. Reader mounts are not deduplicated.
    pub fn mount_archive_reader<R>(
        &mut self,
        name: impl Into<PathBuf>,
        reader: R,
        size: u64,
        priority: i64,
        options: ArchiveOptions,
    ) -> Result<&Archive, Error>
    where
        R: ReadAt + Send + Sync + 'static,
    {
        let mut archive = Archive::new(name, reader, size, options)?;
        archive.set_mount_info(priority, self.next_order);
        let at = self.insert_mount(Provider::Archive(archive), priority);
        self.mounts
            .get(at)
            .and_then(|mount| mount.provider.archive())
            .ok_or(Error::Unsupported("mounted archive lookup"))
    }

    /// Mounts an archive presented under `layout`'s directories; an empty layout
    /// mounts it as `mount_archive` does.
    pub(crate) fn mount_archive_rehomed(
        &mut self,
        path: &Path,
        priority: i64,
        layout: &Layout,
    ) -> Result<bool, Error> {
        if layout.is_empty() {
            return self
                .mount_archive(path, priority)
                .map(|archive| archive.is_some());
        }
        if self.already_mounted(path, &host::base(path)) {
            return Ok(false);
        }
        let mut archive = Archive::open(path, ArchiveOptions::default())?;
        self.remember_mount(path, host::base(path));
        archive.set_mount_info(priority, self.next_order);
        let rehomed = RehomedArchive::new(archive, layout);
        self.insert_mount(Provider::Rehomed(rehomed), priority);
        Ok(true)
    }

    /// Discovers the standard HPI-family files with the default tier policy.
    pub fn mount_game_directory(&mut self, root: impl AsRef<Path>) -> Result<(), Error> {
        self.mount_game_directory_with_plan(root, &default_retail_mount_plan())
    }

    /// Adds complete roots in load order: every provider of a later root outranks
    /// every provider of an earlier one. Repeated paths keep first-mount dedup.
    pub fn mount_game_directories<P: AsRef<Path>>(&mut self, roots: &[P]) -> Result<(), Error> {
        let game: Vec<GameRoot> = roots
            .iter()
            .map(|root| GameRoot::new(root.as_ref()))
            .collect();
        self.mount_game_roots(&game)
    }

    /// `mount_game_directories` for roots that may admit only some of their
    /// archives or none of their loose files. Every root keeps the priority span its
    /// position gives it, so leaving providers out never reorders the rest.
    pub fn mount_game_roots(&mut self, roots: &[GameRoot]) -> Result<(), Error> {
        let plan = default_retail_mount_plan();
        let top = [plan.hpi, plan.ufo, plan.ccx, plan.gp3, plan.loose]
            .into_iter()
            .max()
            .unwrap_or(plan.loose);
        let span = i64::from(top) * 10;
        for (i, root) in roots.iter().enumerate() {
            let base = i64::try_from(i).unwrap_or(i64::MAX).saturating_mul(span);
            self.mount_game_directory_filtered(&root.dir, &plan, base, root)
                .map_err(|source| Error::MountRoot {
                    root: root.dir.clone(),
                    source: Box::new(source),
                })?;
        }
        Ok(())
    }

    /// Mounts an installation directory under an explicit tier policy.
    pub fn mount_game_directory_with_plan(
        &mut self,
        root: impl AsRef<Path>,
        plan: &MountPlan,
    ) -> Result<(), Error> {
        let root = root.as_ref();
        self.mount_game_directory_filtered(root, plan, 0, &GameRoot::new(root))
    }

    /// Records and reports a suppressed duplicate.
    fn already_mounted(&mut self, name: &Path, label: &str) -> bool {
        let key = canonical_mount_key(name);
        if let Some(first) = self.mounted_by.get(&key) {
            let note = format!("duplicate mount suppressed: {label} (already mounted as {first})");
            self.notes.push(note);
            return true;
        }
        false
    }

    /// Only successful mounts are remembered: failed candidates stay eligible for a
    /// later attempt.
    fn remember_mount(&mut self, name: &Path, label: String) {
        self.mounted_by.insert(canonical_mount_key(name), label);
    }

    fn mount_game_directory_filtered(
        &mut self,
        root: &Path,
        plan: &MountPlan,
        base_priority: i64,
        options: &GameRoot,
    ) -> Result<(), Error> {
        plan.validate()?;
        let mut listing: Vec<(Vec<u8>, String, PathBuf, bool)> = Vec::new();
        for item in std::fs::read_dir(root).map_err(|source| Error::io_at(root, source))? {
            let item = item.map_err(|source| Error::io_at(root, source))?;
            let file_name = item.file_name();
            // Go's DirEntry.IsDir does not follow symlinks; neither does file_type.
            let is_dir = item.file_type().map(|t| t.is_dir()).unwrap_or(false);
            listing.push((
                host::name_bytes(&file_name),
                file_name.to_string_lossy().into_owned(),
                root.join(&file_name),
                is_dir,
            ));
        }
        // Go's os.ReadDir returns entries sorted by name.
        listing.sort_by(|a, b| a.0.cmp(&b.0));

        let replaced: HashSet<String> = options
            .extra
            .iter()
            .map(|full| host::lower(&host::base(full)))
            .collect();
        let mut archives: Vec<GameArchive> = Vec::new();
        for (_, name, full, is_dir) in listing {
            if is_dir || !is_archive_name(&name) || replaced.contains(&host::lower(&name)) {
                continue;
            }
            if options.archives.as_ref().is_none_or(|admit| admit(&name)) {
                archives.push(GameArchive {
                    name,
                    // Go's filepath.Join cleans the joined path.
                    full: host::clean(&full),
                    layout: Layout::default(),
                    tier: None,
                });
            }
        }
        for full in &options.extra {
            if is_archive_name(&host::lossy(full)) {
                archives.push(GameArchive {
                    name: host::base(full),
                    full: full.clone(),
                    layout: options
                        .extra_layout
                        .as_ref()
                        .map(|layout| layout(full))
                        .unwrap_or_default(),
                    tier: options.extra_tier.as_ref().and_then(|tier| tier(full)),
                });
            }
        }
        // Retail consumes Windows directory enumeration order; this sorts so the
        // equal-tier winner is stable across filesystems. Equal-priority mounts
        // added later win, so lexically later names take precedence in a tier. The
        // full paths compare as Go strings, byte by byte.
        archives.sort_by(|a, b| {
            host::lower(&a.name)
                .cmp(&host::lower(&b.name))
                .then_with(|| a.name.cmp(&b.name))
                .then_with(|| host::lossy(&a.full).cmp(&host::lossy(&b.full)))
        });
        // Retail's ten-new-archive budget restarts on each mount pass; repeated
        // passes mount every valid local HPI. Mount that converged set here.
        for entry in &archives {
            let tier = match entry.tier {
                Some(tier) => {
                    // An add-on layer must stay below the loose host files.
                    if tier == 0 || tier >= plan.loose {
                        return Err(Error::ExtraTier {
                            archive: entry.full.clone(),
                            tier,
                            loose: plan.loose,
                        });
                    }
                    tier
                }
                None => match host::lower(host::ext(&entry.name)).as_str() {
                    ".ccx" => plan.ccx,
                    ".gp3" | ".gp4" | ".gpf" | ".swx" => plan.gp3,
                    ".ufo" => plan.ufo,
                    _ => plan.hpi,
                },
            };
            let priority = base_priority + i64::from(tier) * 10;
            if let Err(error) = self.mount_archive_rehomed(&entry.full, priority, &entry.layout) {
                // Only the container gate and an unavailable candidate are discovery
                // rejections. Indexing and later payload errors keep their own
                // failure boundary.
                let reason = match &error {
                    // The provider identity below replaces the host path.
                    Error::Open { source, .. } => source.to_string(),
                    Error::RejectedArchive(_) => error.to_string(),
                    _ => return Err(error),
                };
                self.notes.push(format!(
                    "nanolathe: archive rejected: logical path {name}, providers searched [{name}], expected HPI container: {reason}",
                    name = entry.name
                ));
            }
        }
        if options.no_loose {
            return Ok(());
        }
        self.mount_directory(root, base_priority + i64::from(plan.loose) * 10)
    }

    /// Places one mount in lookup order and takes the next order number. A new
    /// mount carries the highest order number, so among equal priorities it goes
    /// first: before the first mount whose priority is not greater. Returns the
    /// mount's position.
    fn insert_mount(&mut self, provider: Provider, priority: i64) -> usize {
        let at = self
            .mounts
            .iter()
            .position(|mount| mount.priority <= priority)
            .unwrap_or(self.mounts.len());
        self.mounts.insert(
            at,
            MountedProvider {
                provider,
                priority,
                order: self.next_order,
            },
        );
        self.next_order += 1;
        at
    }

    fn winner(&self, logical: &[u8]) -> Option<(&Provider, &ProviderEntry)> {
        self.mounts.iter().find_map(|mount| {
            mount
                .provider
                .lookup(logical)
                .map(|entry| (&mount.provider, entry))
        })
    }

    /// The winning provider's file for a logical name.
    pub fn open(&self, name: impl AsRef<[u8]>) -> Result<File, Error> {
        let name = name.as_ref();
        let logical = clean_path(name)?;
        match self.winner(&logical) {
            Some((provider, entry)) => open_entry(provider, entry, name),
            None => Err(Error::NotFound(show(name))),
        }
    }

    /// The complete contents of the winning file.
    pub fn read_file(&self, name: impl AsRef<[u8]>) -> Result<Vec<u8>, Error> {
        self.open(name)?.into_bytes().map_err(Error::io)
    }

    /// At most `max` bytes of the winning file; a larger file is an error before
    /// anything is decoded.
    pub fn read_file_limit(&self, name: impl AsRef<[u8]>, max: u64) -> Result<Vec<u8>, Error> {
        read_file_limit_with(|n| self.open(n), |n| self.stat(n), name.as_ref(), max)
    }

    /// `length` bytes of a file from `offset` (see [`RangeReader`]). Every provider
    /// answers the same way; one that can decode part of a file does so, otherwise
    /// the file is read whole and sliced.
    pub fn read_file_range(
        &self,
        name: impl AsRef<[u8]>,
        offset: u64,
        length: Option<usize>,
    ) -> Result<Vec<u8>, Error> {
        let name = name.as_ref();
        let logical = clean_path(name)?;
        if let Some((provider, entry)) = self.winner(&logical) {
            if let Some(result) = provider.read_range(entry, offset, length) {
                return result;
            }
        }
        // The fallback keeps the archive reader's ceiling.
        let data = self.read_file_limit(name, DEFAULT_MAX_FILE_BYTES.unsigned_abs())?;
        let start = usize::try_from(offset)
            .ok()
            .filter(|&start| start <= data.len())
            .ok_or_else(|| {
                Error::NotFound(format!(
                    "range offset {offset} outside {}-byte file",
                    data.len()
                ))
            })?;
        let rest = data.get(start..).unwrap_or(&[]);
        let end = match length {
            Some(length) if length < rest.len() => length,
            _ => rest.len(),
        };
        Ok(rest.get(..end).unwrap_or(&[]).to_vec())
    }

    /// The winning entry for a logical name.
    pub fn stat(&self, name: impl AsRef<[u8]>) -> Result<EntryInfo, Error> {
        let name = name.as_ref();
        let logical = clean_path(name)?;
        match self.winner(&logical) {
            Some((_, entry)) => Ok(entry.info.clone()),
            None => Err(Error::NotFound(show(name))),
        }
    }

    /// Every mount that holds a logical path, in the precedence order `open`
    /// resolves: the first is the winner, the rest are shadowed.
    pub fn sources(&self, name: impl AsRef<[u8]>) -> Vec<EntryInfo> {
        let Ok(logical) = clean_path(name.as_ref()) else {
            return Vec::new();
        };
        self.mounts
            .iter()
            .filter_map(|mount| mount.provider.lookup(&logical))
            .map(|entry| entry.info.clone())
            .collect()
    }

    /// The union of a directory's children, each collision resolved as `open`
    /// resolves it, sorted by logical path.
    pub fn read_dir(&self, name: impl AsRef<[u8]>) -> Result<Vec<EntryInfo>, Error> {
        let name = name.as_ref();
        let logical = clean_path(name)?;
        if let Ok(info) = self.stat(&logical) {
            if !info.is_dir {
                return Err(Error::IsDir(show(name)));
            }
        }
        // First mount wins each path; the ordered map is the sorted result.
        let mut winners: BTreeMap<Vec<u8>, EntryInfo> = BTreeMap::new();
        for mount in &self.mounts {
            for entry in mount.provider.children(&logical) {
                winners
                    .entry(entry.info.path.clone())
                    .or_insert_with(|| entry.info.clone());
            }
        }
        let result: Vec<EntryInfo> = winners.into_values().collect();
        if result.is_empty() && !logical.is_empty() && self.stat(&logical).is_err() {
            return Err(Error::NotFound(show(name)));
        }
        Ok(result)
    }

    /// The winning direct children of a directory in provider enumeration order.
    /// Retail's wildcard scans do not sort; `read_dir` keeps its sorted contract.
    pub fn retail_read_dir(&self, name: impl AsRef<[u8]>) -> Result<Vec<EntryInfo>, Error> {
        let name = name.as_ref();
        let logical = clean_path(name)?;
        let mut prefix = logical.clone();
        if !prefix.is_empty() {
            prefix.push(b'/');
        }
        let mut seen: HashSet<Vec<u8>> = HashSet::new();
        let mut result = Vec::new();
        for mount in &self.mounts {
            for info in mount.provider.retail_entries() {
                let Some(rest) = info.path.strip_prefix(prefix.as_slice()) else {
                    continue;
                };
                if info.path.is_empty() || rest.is_empty() || rest.contains(&b'/') {
                    continue;
                }
                if !seen.insert(fold_logical_name(&info.path)) {
                    continue;
                }
                result.push(info.clone());
            }
        }
        if result.is_empty() && !logical.is_empty() && self.stat(&logical).is_err() {
            return Err(Error::NotFound(show(name)));
        }
        Ok(result)
    }

    /// Drops every mount, closing archive files.
    pub fn close(&mut self) -> Result<(), Error> {
        self.mounts.clear();
        Ok(())
    }
}

fn open_entry(provider: &Provider, entry: &ProviderEntry, name: &[u8]) -> Result<File, Error> {
    if entry.info.is_dir {
        return Err(Error::IsDir(show(name)));
    }
    provider.open(entry)
}

/// The stamp of an entry: its provider identity, size and mount position, and the
/// host file's size and modification time when the source path names one. Go took
/// `path.Base` of the source path, which splits on `/` only.
fn stamp_for(info: &EntryInfo) -> String {
    let source = host::lossy(&info.source.source_path);
    let mut stamp = format!(
        "{}|{}|{}|{}|{}",
        info.source.provider_type,
        host::slash_base(&source),
        info.size,
        info.source.priority,
        info.source.mount_order
    );
    if !info.source.source_path.as_os_str().is_empty() {
        if let Ok(metadata) = std::fs::metadata(&info.source.source_path) {
            let modified = metadata.modified().map(unix_nanos).unwrap_or(0);
            stamp = format!("{stamp}|{}|{modified}", metadata.len());
        }
    }
    stamp
}

impl FsOps for Fs {
    fn open(&self, name: &[u8]) -> Result<File, Error> {
        Fs::open(self, name)
    }
    fn read_file_limit(&self, name: &[u8], max: u64) -> Result<Vec<u8>, Error> {
        Fs::read_file_limit(self, name, max)
    }
    fn read_dir(&self, name: &[u8]) -> Result<Vec<EntryInfo>, Error> {
        Fs::read_dir(self, name)
    }
    fn stat(&self, name: &[u8]) -> Result<EntryInfo, Error> {
        Fs::stat(self, name)
    }
    fn cache_stamp(&self, name: &[u8]) -> Result<String, Error> {
        Fs::cache_stamp(self, name)
    }
    fn as_sources(&self) -> Option<&dyn SourcesView> {
        Some(self)
    }
    fn as_manifest(&self) -> Option<&dyn ManifestView> {
        Some(self)
    }
    fn as_range_reader(&self) -> Option<&dyn RangeReader> {
        Some(self)
    }
    fn as_retail_dir_reader(&self) -> Option<&dyn RetailDirReader> {
        Some(self)
    }
}

impl SourcesView for Fs {
    fn sources(&self, name: &[u8]) -> Vec<EntryInfo> {
        Fs::sources(self, name)
    }
}

impl ManifestView for Fs {
    fn manifest_hash(&self) -> Result<String, Error> {
        Fs::manifest_hash(self)
    }
}

impl RangeReader for Fs {
    fn read_file_range(
        &self,
        name: &[u8],
        offset: u64,
        length: Option<usize>,
    ) -> Result<Vec<u8>, Error> {
        Fs::read_file_range(self, name, offset, length)
    }
}

impl RetailDirReader for Fs {
    fn retail_read_dir(&self, name: &[u8]) -> Result<Vec<EntryInfo>, Error> {
        Fs::retail_read_dir(self, name)
    }
}

/// A read view of one mount. File and metadata operations keep the inspected
/// provider's identity and never fall through to a lower mount; `read_dir` remains
/// the overlay's enumeration.
#[derive(Debug, Clone, Copy)]
pub struct PinnedMount<'a> {
    fs: &'a Fs,
    index: usize,
}

impl FsOps for PinnedMount<'_> {
    fn open(&self, name: &[u8]) -> Result<File, Error> {
        self.fs.open_mount(self.index, name)
    }
    fn read_file_limit(&self, name: &[u8], max: u64) -> Result<Vec<u8>, Error> {
        read_file_limit_with(|n| self.open(n), |n| self.stat(n), name, max)
    }
    fn read_dir(&self, name: &[u8]) -> Result<Vec<EntryInfo>, Error> {
        self.fs.read_dir(name)
    }
    /// A missing pinned file does not acquire an overlay identity.
    fn stat(&self, name: &[u8]) -> Result<EntryInfo, Error> {
        let (_, entry) = self.fs.mount_entry(self.index, name)?;
        Ok(entry.info.clone())
    }
    fn cache_stamp(&self, name: &[u8]) -> Result<String, Error> {
        let info = self.stat(name)?;
        Ok(stamp_for(&info))
    }
}
