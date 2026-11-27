// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/vfs.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! The `std::fs` provider for loose directories.

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use super::host;
use super::overlay::{EntryInfo, Provenance};
use super::path::{join_path, original_join};
use super::provider_index::{EntrySource, ProviderEntry, ProviderIndex};
use super::Error;

/// A loose directory indexed at mount time: names and metadata, no contents.
#[derive(Debug)]
pub(crate) struct LooseProvider {
    pub(crate) index: ProviderIndex,
    pub(crate) indexed_entries: Vec<EntryInfo>,
}

/// One host directory still being listed.
struct Pending {
    dirname: PathBuf,
    parent: Vec<u8>,
    original_parent: Vec<u8>,
    items: std::vec::IntoIter<Item>,
}

struct Item {
    name: Vec<u8>,
    os_name: std::ffi::OsString,
    is_dir: bool,
    size: u64,
}

/// The children of a host directory in Go's `os.ReadDir` order (sorted by name),
/// without symlinks: a loose mount is a logical content root, and a host symlink
/// must not become an alternate path to files outside it.
fn list(dirname: &Path) -> Result<std::vec::IntoIter<Item>, Error> {
    let mut items = Vec::new();
    for item in fs::read_dir(dirname).map_err(|source| Error::io_at(dirname, source))? {
        let item = item.map_err(|source| Error::io_at(dirname, source))?;
        let file_type = item
            .file_type()
            .map_err(|source| Error::io_at(item.path(), source))?;
        if file_type.is_symlink() {
            continue;
        }
        let is_dir = file_type.is_dir();
        // Go took the size from DirEntry.Info (not following links), 0 on error.
        let size = if is_dir {
            0
        } else {
            item.metadata().map(|metadata| metadata.len()).unwrap_or(0)
        };
        let os_name = item.file_name();
        items.push(Item {
            name: host::name_bytes(&os_name),
            os_name,
            is_dir,
            size,
        });
    }
    items.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(items.into_iter())
}

impl LooseProvider {
    pub(crate) fn new(root: &Path, priority: i64, order: usize) -> Result<LooseProvider, Error> {
        let absolute = host::abs(root).map_err(|source| Error::io_at(root, source))?;
        let metadata = fs::metadata(&absolute).map_err(|source| Error::io_at(&absolute, source))?;
        if !metadata.is_dir() {
            return Err(Error::NotADirectory(root.to_path_buf()));
        }
        let mut provider = LooseProvider {
            index: ProviderIndex::default(),
            indexed_entries: Vec::new(),
        };
        let root_info = EntryInfo {
            is_dir: true,
            source: Provenance {
                provider_type: "directory",
                source_path: absolute.clone(),
                mount_root: absolute.clone(),
                priority,
                mount_order: order,
                ..Provenance::default()
            },
            ..EntryInfo::default()
        };
        provider.index.entries.insert(
            Vec::new(),
            ProviderEntry {
                info: root_info,
                source: EntrySource::Dir,
            },
        );
        provider.index_tree(&absolute, priority, order)?;
        Ok(provider)
    }

    /// Go's recursive `indexDir`, with an explicit stack. Entries are recorded in
    /// the same pre-order. Go inserted a directory's table entry after indexing its
    /// children; inserting it before is equivalent, since no descendant has the
    /// directory's own path.
    fn index_tree(&mut self, root: &Path, priority: i64, order: usize) -> Result<(), Error> {
        let mut stack = vec![Pending {
            dirname: root.to_path_buf(),
            parent: Vec::new(),
            original_parent: Vec::new(),
            items: list(root)?,
        }];
        while let Some(pending) = stack.last_mut() {
            let Some(item) = pending.items.next() else {
                stack.pop();
                continue;
            };
            let logical = join_path(&pending.parent, &item.name)?;
            let original = original_join(&pending.original_parent, &item.name);
            let full = pending.dirname.join(&item.os_name);
            let info = EntryInfo {
                name: item.name.clone(),
                path: logical.clone(),
                original_path: original.clone(),
                size: item.size,
                is_dir: item.is_dir,
                source: Provenance {
                    logical_path: logical.clone(),
                    original_path: original.clone(),
                    provider_type: "directory",
                    source_path: full.clone(),
                    mount_root: root.to_path_buf(),
                    priority,
                    mount_order: order,
                    compression: "",
                },
            };
            self.indexed_entries.push(info.clone());
            if item.is_dir {
                self.index.entries.insert(
                    logical.clone(),
                    ProviderEntry {
                        info,
                        source: EntrySource::Dir,
                    },
                );
                let items = list(&full)?;
                stack.push(Pending {
                    dirname: full,
                    parent: logical,
                    original_parent: original,
                    items,
                });
            } else {
                self.index.entries.insert(
                    logical,
                    ProviderEntry {
                        info,
                        source: EntrySource::Loose(full),
                    },
                );
            }
        }
        Ok(())
    }
}

/// Reads a byte range of a loose file without reading the rest, under the archive
/// reader's contract: an offset past the end is refused, `None` reads to the end,
/// and any length is clamped to what remains, so the allocation follows the file
/// rather than the caller's word. The refusal is `NotFound`, naming this provider.
pub(crate) fn read_file_range(
    path: &Path,
    offset: u64,
    length: Option<usize>,
) -> Result<Vec<u8>, Error> {
    let mut handle = fs::File::open(path).map_err(|source| Error::Open {
        path: path.to_path_buf(),
        source,
    })?;
    let size = handle
        .metadata()
        .map_err(|source| Error::io_at(path, source))?
        .len();
    if offset > size {
        return Err(Error::NotFound(format!(
            "range offset {offset} outside {size}-byte file"
        )));
    }
    let remaining = usize::try_from(size - offset)
        .map_err(|_| Error::TooLarge(format!("{} exceeds {} bytes", path.display(), usize::MAX)))?;
    let length = match length {
        Some(length) if length <= remaining => length,
        _ => remaining,
    };
    let mut data = vec![0u8; length];
    handle
        .seek(SeekFrom::Start(offset))
        .map_err(|source| Error::io_at(path, source))?;
    // A file that shrank since the stat returns what it has, as Go's ReadAt did.
    let mut filled = 0;
    while filled < data.len() {
        match handle.read(data.get_mut(filled..).unwrap_or(&mut [])) {
            Ok(0) => break,
            Ok(read) => filled += read,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(source) => return Err(Error::io_at(path, source)),
        }
    }
    data.truncate(filled);
    Ok(data)
}
