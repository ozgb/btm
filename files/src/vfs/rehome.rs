// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/rehome.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! Archives authored in the retail directories, presented under the directories a
//! content layout renames them to.
//!
//! A content set whose layout reads `units` from `unitsE` hides every file under
//! `units`, so an archive mounted beside it with its files under `units` would add
//! nothing. Rehomed, its `units/x.fbi` answers as `unitse/x.fbi`, beside the content
//! set's own, and the layout's view finds it under the retail name. Faction add-ons
//! mount this way under a mod whose layout renames directories.

use super::hpi::Archive;
use super::layout::Layout;
use super::overlay::EntryInfo;
use super::path::clean_path;
use super::provider_index::{ProviderEntry, ProviderIndex};

#[derive(Debug)]
pub(crate) struct RehomedArchive {
    pub(crate) archive: Archive,
    pub(crate) index: ProviderIndex,
    pub(crate) audit: Vec<EntryInfo>,
    pub(crate) retail: Vec<EntryInfo>,
}

impl Layout {
    /// The folded path a retail-named archive path takes under the layout.
    pub(crate) fn home(&self, name: &[u8]) -> Vec<u8> {
        clean_path(&self.rewrite(name)).unwrap_or_else(|_| name.to_vec())
    }
}

fn rehome_infos(infos: impl Iterator<Item = EntryInfo>, layout: &Layout) -> Vec<EntryInfo> {
    infos
        .map(|mut info| {
            let home = layout.home(&info.path);
            info.source.logical_path = home.clone();
            info.path = home;
            info
        })
        .collect()
}

impl RehomedArchive {
    /// The rehomed view of `archive`, after its mount information is set. An entry
    /// the archive itself authors under a renamed directory keeps that path and
    /// wins over a rehomed one of the same path.
    ///
    /// Go walked its entry map in random order. That order matters only when two
    /// paths rehome to one path that the archive does not author (a table mapping
    /// two retail names to one target); here entries are walked in path order, so
    /// the greatest such path wins, deterministically.
    pub(crate) fn new(archive: Archive, layout: &Layout) -> RehomedArchive {
        let mut index = ProviderIndex::default();
        for (name, entry) in &archive.index.entries {
            let home = layout.home(name);
            if home != *name && archive.index.entries.contains_key(&home) {
                continue;
            }
            let mut info = entry.info.clone();
            info.path = home.clone();
            info.source.logical_path = home.clone();
            // The opened file reports this entry's info: the path the overlay found
            // it under.
            index.entries.insert(
                home,
                ProviderEntry {
                    info,
                    source: entry.source.clone(),
                },
            );
        }
        let audit = rehome_infos(archive.audit_entries().iter().cloned(), layout);
        let retail = rehome_infos(archive.retail_entries().cloned(), layout);
        RehomedArchive {
            archive,
            index,
            audit,
            retail,
        }
    }
}
