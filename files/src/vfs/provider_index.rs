// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/provider_index.go, vfs/vfs.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! The logical-path table every provider keeps.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::hpi::HpiRecord;
use super::overlay::EntryInfo;

/// Where an indexed entry's bytes come from. Go stored `open` and `readRange`
/// closures on the entry; here the entry names its source and the provider reads it.
#[derive(Debug, Clone)]
pub(crate) enum EntrySource {
    /// A directory: nothing to open.
    Dir,
    /// A loose host file.
    Loose(PathBuf),
    /// An archive record of the provider's archive.
    Record(HpiRecord),
}

#[derive(Debug, Clone)]
pub(crate) struct ProviderEntry {
    pub(crate) info: EntryInfo,
    pub(crate) source: EntrySource,
}

/// The logical-path table, with the lookup and one-level enumeration the overlay
/// resolves through. Go kept a hash map and `children` returned map order; this is
/// an ordered map, so `children` is sorted by path. The overlay imposes its own
/// order on what it publishes either way.
#[derive(Debug, Clone, Default)]
pub(crate) struct ProviderIndex {
    pub(crate) entries: BTreeMap<Vec<u8>, ProviderEntry>,
}

impl ProviderIndex {
    pub(crate) fn lookup(&self, name: &[u8]) -> Option<&ProviderEntry> {
        self.entries.get(name)
    }

    pub(crate) fn children(&self, parent: &[u8]) -> Vec<&ProviderEntry> {
        let mut prefix = parent.to_vec();
        if !prefix.is_empty() {
            prefix.push(b'/');
        }
        self.entries
            .range(prefix.clone()..)
            .take_while(|(name, _)| name.starts_with(&prefix))
            .filter(|(name, _)| {
                let rest = name.get(prefix.len()..).unwrap_or(&[]);
                !name.is_empty() && !rest.is_empty() && !rest.contains(&b'/')
            })
            .map(|(_, entry)| entry)
            .collect()
    }
}
