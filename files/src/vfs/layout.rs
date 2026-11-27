// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/layout.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! Content directory layouts: the first-segment redirection a content set may need
//! so its authored trees are read under the names the loaders ask for.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

use super::host::equal_fold;
use super::overlay::{
    EntryInfo, File, FsOps, ManifestView, RangeReader, RetailDirReader, SharedFs, SourcesView,
};
use super::Error;

/// Redirects the first segment of a logical path. Some content sets author their
/// trees under renamed directories (`weapons` becomes `weaponP`, `units` becomes
/// `unitsE`), so a loader that asks for the retail name finds nothing. A layout
/// carries one rename table: retail directory to shipped directory. An empty layout
/// is the retail case and rewrites nothing.
///
/// Matching is case-insensitive with Go's `strings.EqualFold` (Unicode folding, not
/// the ASCII folding of logical names).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Layout {
    /// Sorted by retail name, so two equal tables produce one order.
    entries: Vec<LayoutEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LayoutEntry {
    /// The directory the loaders ask for, as authored here.
    retail: String,
    /// The directory the content set ships, as it spells it.
    target: String,
}

/// Splits a logical path at its first separator; both slash styles count, since a
/// caller may hand over an authored backslash path.
fn first_segment(name: &[u8]) -> (&[u8], &[u8]) {
    match name.iter().position(|&b| b == b'/' || b == b'\\') {
        Some(at) => name.split_at(at),
        None => (name, &[]),
    }
}

impl Layout {
    /// Builds a layout from a retail-to-target table. Rows whose names are equal
    /// (ignoring case), or whose either name is empty, are dropped. Later rows with
    /// the same retail name replace earlier ones, as keys of Go's map did.
    pub fn new<I, K, V>(table: I) -> Layout
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let table: BTreeMap<String, String> = table
            .into_iter()
            .map(|(retail, target)| (retail.into(), target.into()))
            .collect();
        // The ordered map iterates in Go's sort order (byte-wise string compare);
        // the keys are unique, so Go's unstable sort had no ties to break.
        let entries = table
            .into_iter()
            .filter(|(retail, target)| {
                !retail.is_empty()
                    && !target.is_empty()
                    && !equal_fold(retail.as_bytes(), target.as_bytes())
            })
            .map(|(retail, target)| LayoutEntry { retail, target })
            .collect();
        Layout { entries }
    }

    /// Whether the layout rewrites nothing.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The retail-to-target rows in retail-name order.
    pub fn names(&self) -> Vec<(String, String)> {
        self.entries
            .iter()
            .map(|entry| (entry.retail.clone(), entry.target.clone()))
            .collect()
    }

    /// Maps a requested retail-named path onto the directory the content set ships.
    /// Borrows when no row matches.
    pub(crate) fn rewrite<'a>(&self, name: &'a [u8]) -> Cow<'a, [u8]> {
        self.swap(name, |entry| (&entry.retail, &entry.target))
    }

    /// The inverse of `rewrite`: a path the providers hold back to the retail name.
    pub(crate) fn restore<'a>(&self, name: &'a [u8]) -> Cow<'a, [u8]> {
        self.swap(name, |entry| (&entry.target, &entry.retail))
    }

    fn swap<'a>(
        &self,
        name: &'a [u8],
        pick: impl Fn(&LayoutEntry) -> (&String, &String),
    ) -> Cow<'a, [u8]> {
        if self.entries.is_empty() {
            return Cow::Borrowed(name);
        }
        let (first, rest) = first_segment(name);
        for entry in &self.entries {
            let (from, to) = pick(entry);
            if equal_fold(first, from.as_bytes()) {
                let mut out = Vec::with_capacity(to.len() + rest.len());
                out.extend_from_slice(to.as_bytes());
                out.extend_from_slice(rest);
                return Cow::Owned(out);
            }
        }
        Cow::Borrowed(name)
    }

    fn restore_all(&self, entries: &mut [EntryInfo]) {
        for entry in entries {
            entry.path = self.restore(&entry.path).into_owned();
        }
    }

    /// A read view of `inner` whose lookups take retail-named paths and whose
    /// results carry retail-named paths. An empty layout returns `inner` itself, so
    /// a retail content set keeps the view it mounted with all its capabilities.
    ///
    /// The view always offers the sources and manifest reports (answering empty
    /// when `inner` has none), and offers range reads and retail enumeration only
    /// when `inner` does, so a probe through the layout answers as a probe of
    /// `inner` would.
    pub fn apply<'a>(&self, inner: SharedFs<'a>) -> SharedFs<'a> {
        if self.entries.is_empty() {
            return inner;
        }
        let ranged = inner.as_range_reader().is_some();
        let ordered = inner.as_retail_dir_reader().is_some();
        Arc::new(LayoutFs {
            inner,
            layout: self.clone(),
            ranged,
            ordered,
        })
    }
}

/// The view [`Layout::apply`] returns. It never mounts or closes, and leaves the
/// overlay's per-mount surface alone: that reports what is on disk.
struct LayoutFs<'a> {
    inner: SharedFs<'a>,
    layout: Layout,
    ranged: bool,
    ordered: bool,
}

impl FsOps for LayoutFs<'_> {
    fn open(&self, name: &[u8]) -> Result<File, Error> {
        self.inner.open(&self.layout.rewrite(name))
    }

    fn read_file_limit(&self, name: &[u8], max: u64) -> Result<Vec<u8>, Error> {
        self.inner.read_file_limit(&self.layout.rewrite(name), max)
    }

    fn read_dir(&self, name: &[u8]) -> Result<Vec<EntryInfo>, Error> {
        let mut entries = self.inner.read_dir(&self.layout.rewrite(name))?;
        self.layout.restore_all(&mut entries);
        Ok(entries)
    }

    fn stat(&self, name: &[u8]) -> Result<EntryInfo, Error> {
        let mut info = self.inner.stat(&self.layout.rewrite(name))?;
        info.path = self.layout.restore(&info.path).into_owned();
        Ok(info)
    }

    fn cache_stamp(&self, name: &[u8]) -> Result<String, Error> {
        self.inner.cache_stamp(&self.layout.rewrite(name))
    }

    fn as_sources(&self) -> Option<&dyn SourcesView> {
        Some(self)
    }

    fn as_manifest(&self) -> Option<&dyn ManifestView> {
        Some(self)
    }

    fn as_range_reader(&self) -> Option<&dyn RangeReader> {
        if self.ranged {
            Some(self)
        } else {
            None
        }
    }

    fn as_retail_dir_reader(&self) -> Option<&dyn RetailDirReader> {
        if self.ordered {
            Some(self)
        } else {
            None
        }
    }
}

impl SourcesView for LayoutFs<'_> {
    /// Empty when the wrapped view has no such report, as a caller probing it
    /// directly would observe.
    fn sources(&self, name: &[u8]) -> Vec<EntryInfo> {
        let Some(sources) = self.inner.as_sources() else {
            return Vec::new();
        };
        let mut infos = sources.sources(&self.layout.rewrite(name));
        self.layout.restore_all(&mut infos);
        infos
    }
}

impl ManifestView for LayoutFs<'_> {
    /// The wrapped view's identity, or the empty identity when it has none.
    fn manifest_hash(&self) -> Result<String, Error> {
        match self.inner.as_manifest() {
            Some(manifest) => manifest.manifest_hash(),
            None => Ok(String::new()),
        }
    }
}

impl RangeReader for LayoutFs<'_> {
    /// The result is bytes, not provenance, so nothing is restored.
    fn read_file_range(
        &self,
        name: &[u8],
        offset: u64,
        length: Option<usize>,
    ) -> Result<Vec<u8>, Error> {
        match self.inner.as_range_reader() {
            Some(ranged) => ranged.read_file_range(&self.layout.rewrite(name), offset, length),
            None => Err(Error::Unsupported("range reads")),
        }
    }
}

impl RetailDirReader for LayoutFs<'_> {
    fn retail_read_dir(&self, name: &[u8]) -> Result<Vec<EntryInfo>, Error> {
        match self.inner.as_retail_dir_reader() {
            Some(ordered) => {
                let mut entries = ordered.retail_read_dir(&self.layout.rewrite(name))?;
                self.layout.restore_all(&mut entries);
                Ok(entries)
            }
            None => Err(Error::Unsupported("retail directory enumeration")),
        }
    }
}
