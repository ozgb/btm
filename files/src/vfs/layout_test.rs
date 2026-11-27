// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/layout_test.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::borrow::Cow;
use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex};

use super::overlay::{ManifestView, RangeReader, RetailDirReader, SourcesView};
use super::{EntryInfo, Error, File, FsOps, Layout, SharedFs};

/// Answers from a fixed path set and records the paths it was asked for, so a test
/// can assert what the wrapper handed down as well as what it handed back.
struct RecordingFs {
    files: HashSet<Vec<u8>>,
    asked: Mutex<Vec<Vec<u8>>>,
}

impl RecordingFs {
    fn note(&self, name: &[u8]) -> Vec<u8> {
        self.asked.lock().unwrap().push(name.to_vec());
        name.to_ascii_lowercase()
    }

    fn asked(&self) -> Vec<Vec<u8>> {
        self.asked.lock().unwrap().clone()
    }

    fn info(path: &[u8]) -> EntryInfo {
        let name = match path.iter().rposition(|&b| b == b'/') {
            Some(at) => &path[at + 1..],
            None => path,
        };
        EntryInfo {
            name: name.to_vec(),
            path: path.to_vec(),
            size: 1,
            ..EntryInfo::default()
        }
    }

    fn missing(name: &[u8]) -> Error {
        Error::NotFound(String::from_utf8_lossy(name).into_owned())
    }
}

/// The recording view with a chosen set of optional capabilities: Go's
/// `recordingFS`, `richFS` (sources and manifest), `rangedFS` (rich plus ranges),
/// `orderedFS` and `orderedRangedFS`.
#[derive(Clone)]
struct View {
    inner: Arc<RecordingFs>,
    rich: bool,
    ranged: bool,
    ordered: bool,
}

impl View {
    fn plain(inner: &Arc<RecordingFs>) -> View {
        View {
            inner: Arc::clone(inner),
            rich: false,
            ranged: false,
            ordered: false,
        }
    }
    fn rich(inner: &Arc<RecordingFs>) -> View {
        View {
            rich: true,
            ..View::plain(inner)
        }
    }
    fn ranged(inner: &Arc<RecordingFs>) -> View {
        View {
            ranged: true,
            ..View::rich(inner)
        }
    }
    fn ordered(inner: &Arc<RecordingFs>, ranged: bool) -> View {
        View {
            ordered: true,
            ranged,
            ..View::plain(inner)
        }
    }
}

impl FsOps for View {
    fn open(&self, name: &[u8]) -> Result<File, Error> {
        let path = self.inner.note(name);
        if !self.inner.files.contains(&path) {
            return Err(RecordingFs::missing(name));
        }
        Ok(File::memory(Vec::new(), RecordingFs::info(&path)))
    }

    fn read_file_limit(&self, name: &[u8], _max: u64) -> Result<Vec<u8>, Error> {
        if !self.inner.files.contains(&self.inner.note(name)) {
            return Err(RecordingFs::missing(name));
        }
        Ok(b"payload".to_vec())
    }

    fn read_dir(&self, name: &[u8]) -> Result<Vec<EntryInfo>, Error> {
        let mut prefix = self.inner.note(name);
        if !prefix.is_empty() {
            prefix.push(b'/');
        }
        let out: Vec<EntryInfo> = self
            .inner
            .files
            .iter()
            .filter(|path| path.starts_with(&prefix) && !path[prefix.len()..].contains(&b'/'))
            .map(|path| RecordingFs::info(path))
            .collect();
        if out.is_empty() {
            return Err(RecordingFs::missing(name));
        }
        Ok(out)
    }

    fn stat(&self, name: &[u8]) -> Result<EntryInfo, Error> {
        let path = self.inner.note(name);
        if !self.inner.files.contains(&path) {
            return Err(RecordingFs::missing(name));
        }
        Ok(RecordingFs::info(&path))
    }

    fn cache_stamp(&self, name: &[u8]) -> Result<String, Error> {
        let path = self.inner.note(name);
        if !self.inner.files.contains(&path) {
            return Err(RecordingFs::missing(name));
        }
        Ok(format!("stamp:{}", String::from_utf8_lossy(&path)))
    }

    fn as_sources(&self) -> Option<&dyn SourcesView> {
        self.rich.then_some(self as &dyn SourcesView)
    }
    fn as_manifest(&self) -> Option<&dyn ManifestView> {
        self.rich.then_some(self as &dyn ManifestView)
    }
    fn as_range_reader(&self) -> Option<&dyn RangeReader> {
        self.ranged.then_some(self as &dyn RangeReader)
    }
    fn as_retail_dir_reader(&self) -> Option<&dyn RetailDirReader> {
        self.ordered.then_some(self as &dyn RetailDirReader)
    }
}

impl SourcesView for View {
    fn sources(&self, name: &[u8]) -> Vec<EntryInfo> {
        match self.stat(name) {
            Ok(info) => vec![info],
            Err(_) => Vec::new(),
        }
    }
}

impl ManifestView for View {
    fn manifest_hash(&self) -> Result<String, Error> {
        Ok("manifest".to_string())
    }
}

impl RangeReader for View {
    fn read_file_range(
        &self,
        name: &[u8],
        offset: u64,
        length: Option<usize>,
    ) -> Result<Vec<u8>, Error> {
        if !self.inner.files.contains(&self.inner.note(name)) {
            return Err(RecordingFs::missing(name));
        }
        let payload: &[u8] = b"payload";
        if offset > payload.len() as u64 {
            return Err(RecordingFs::missing(name));
        }
        let mut payload = &payload[offset as usize..];
        if let Some(length) = length {
            if length < payload.len() {
                payload = &payload[..length];
            }
        }
        Ok(payload.to_vec())
    }
}

impl RetailDirReader for View {
    fn retail_read_dir(&self, name: &[u8]) -> Result<Vec<EntryInfo>, Error> {
        self.inner.note(name);
        Ok(vec![
            RecordingFs::info(b"downloadp/z.tdf"),
            RecordingFs::info(b"downloadp/a.tdf"),
        ])
    }
}

fn escalation_layout() -> Layout {
    Layout::new([
        ("units", "unitsE"),
        ("weapons", "weaponE"),
        ("gamedata", "gamedatE"),
    ])
}

fn recording(paths: &[&str]) -> Arc<RecordingFs> {
    Arc::new(RecordingFs {
        files: paths.iter().map(|path| path.as_bytes().to_vec()).collect(),
        asked: Mutex::new(Vec::new()),
    })
}

/// The wrapper's whole contract: the request goes down renamed, the result comes
/// back retail-named.
#[test]
fn layout_redirects_the_first_segment_both_ways() {
    let inner = recording(&[
        "unitse/armcom.fbi",
        "weapone/guns.tdf",
        "objects3d/armcom.3do",
    ]);
    let view = escalation_layout().apply(Arc::new(View::rich(&inner)));

    let info = view.stat(b"units/ARMCOM.FBI").unwrap();
    assert_eq!(info.path, b"units/armcom.fbi");
    assert_eq!(inner.asked().last().unwrap(), b"unitsE/ARMCOM.FBI");

    view.read_file_limit(b"weapons/guns.tdf", 1 << 10).unwrap();
    assert_eq!(
        view.cache_stamp(b"units/armcom.fbi").unwrap(),
        "stamp:unitse/armcom.fbi"
    );

    let entries = view.read_dir(b"units").unwrap();
    assert!(
        entries.len() == 1 && entries[0].path == b"units/armcom.fbi",
        "read_dir entries = {entries:?}"
    );

    // A path whose first segment is not in the table is untouched, and a segment
    // that only looks like one is not rewritten in the middle.
    view.stat(b"objects3d/armcom.3do").unwrap();
    assert!(
        view.stat(b"objects3d/units/armcom.3do")
            .is_err_and(|e| e.is_not_found()),
        "the layout rewrote a segment that was not the first"
    );

    // Sources carries the same reversal, because a missing-file diagnostic names
    // the providers it searched for a retail-named path.
    let sources = view.as_sources().unwrap().sources(b"units/armcom.fbi");
    assert!(
        sources.len() == 1 && sources[0].path == b"units/armcom.fbi",
        "sources = {sources:?}"
    );
    assert_eq!(
        view.as_manifest().unwrap().manifest_hash().unwrap(),
        "manifest"
    );
}

/// The wrapper does not pretend a plain view has the overlay's optional reports.
#[test]
fn layout_without_optional_views_answers_as_though_unwrapped() {
    let inner = recording(&["unitse/armcom.fbi"]);
    let view = escalation_layout().apply(Arc::new(View::plain(&inner)));
    let sources = view.as_sources().unwrap().sources(b"units/armcom.fbi");
    assert!(sources.is_empty(), "sources = {sources:?}, want none");
    assert_eq!(view.as_manifest().unwrap().manifest_hash().unwrap(), "");
}

/// A retail install keeps its concrete view: callers probing for optional reports
/// must still find them.
#[test]
fn empty_layout_returns_the_view_unchanged() {
    let inner: SharedFs<'static> = Arc::new(View::plain(&recording(&["units/armcom.fbi"])));
    let layouts = [
        Layout::default(),
        Layout::new(Vec::<(String, String)>::new()),
        Layout::new(BTreeMap::<String, String>::new()),
        Layout::new([("units", "UNITS")]),
        Layout::new([("", "x"), ("y", "")]),
    ];
    for layout in layouts {
        assert!(
            layout.names().is_empty(),
            "layout {:?} should have no rows",
            layout.names()
        );
        let applied = layout.apply(Arc::clone(&inner));
        assert!(
            Arc::ptr_eq(&applied, &inner),
            "an empty layout wrapped the view"
        );
    }
}

/// The diagnostic order and the round trip every provenance path depends on.
#[test]
fn layout_rows_are_ordered_and_reversible() {
    let layout = escalation_layout();
    let rows = layout.names();
    let want = [
        ("gamedata", "gamedatE"),
        ("units", "unitsE"),
        ("weapons", "weaponE"),
    ];
    assert_eq!(rows.len(), want.len());
    for (row, want) in rows.iter().zip(want) {
        assert_eq!((row.0.as_str(), row.1.as_str()), want);
    }
    for path in [
        "units/armcom.fbi",
        "weapons",
        "gamedata/sidedata.tdf",
        "maps/x.ota",
        "",
    ] {
        let path = path.as_bytes();
        let back = layout.restore(&layout.rewrite(path)).into_owned();
        assert_eq!(
            back,
            path,
            "restore(rewrite({:?}))",
            String::from_utf8_lossy(path)
        );
    }
    // An authored backslash path splits on the same first separator.
    assert_eq!(
        &*layout.rewrite(b"units\\armcom.fbi"),
        b"unitsE\\armcom.fbi"
    );
}

/// Go counted allocations: none for an unmapped lookup, one for a mapped one. The
/// Rust equivalent is a borrowed result for an unmapped path and a single owned
/// buffer otherwise.
#[test]
fn layout_lookup_allocates_only_the_rewritten_path() {
    let layout = escalation_layout();
    assert!(matches!(
        layout.rewrite(b"objects3d/armcom.3do"),
        Cow::Borrowed(_)
    ));
    assert!(matches!(layout.rewrite(b"units/armcom.fbi"), Cow::Owned(_)));
    assert!(matches!(
        layout.restore(b"unitse/armcom.fbi"),
        Cow::Owned(_)
    ));
}

/// A profiled content set keeps the header-range fast path, and a wrapper over a
/// view without the capability does not advertise one, because the probe is what
/// selects the fallback.
#[test]
fn layout_forwards_range_reads_under_the_rewritten_path() {
    let inner = recording(&["unitse/armcom.fbi"]);
    let view = escalation_layout().apply(Arc::new(View::ranged(&inner)));
    let ranged = view
        .as_range_reader()
        .expect("view over a range reader does not offer ranges");
    let data = ranged
        .read_file_range(b"units/ARMCOM.FBI", 2, Some(3))
        .unwrap();
    assert_eq!(data, b"ylo");
    assert_eq!(inner.asked(), vec![b"unitsE/ARMCOM.FBI".to_vec()]);

    let plain = escalation_layout().apply(Arc::new(View::rich(&recording(&["unitse/armcom.fbi"]))));
    assert!(
        plain.as_range_reader().is_none(),
        "view over a range-less FS advertises ranges"
    );
    // The other optional reports survive.
    assert_eq!(
        plain
            .as_sources()
            .unwrap()
            .sources(b"units/armcom.fbi")
            .len(),
        1
    );
    let sources = view.as_sources().unwrap().sources(b"units/armcom.fbi");
    assert!(
        sources.len() == 1 && sources[0].path == b"units/armcom.fbi",
        "{sources:?}"
    );
    assert_eq!(
        view.as_manifest().unwrap().manifest_hash().unwrap(),
        "manifest"
    );
}

/// Nonlexical order matters when the download pass appends builder products. All
/// combinations, so preserving one capability cannot hide another.
#[test]
fn layout_preserves_retail_enumeration_capability_and_order() {
    for has_range in [false, true] {
        for has_order in [false, true] {
            let inner = recording(&["downloadp/z.tdf", "downloadp/a.tdf"]);
            let source = if has_order {
                View::ordered(&inner, has_range)
            } else if has_range {
                View::ranged(&inner)
            } else {
                View::plain(&inner)
            };
            let view = Layout::new([("download", "downloadP")]).apply(Arc::new(source));
            let ordered = view.as_retail_dir_reader();
            assert_eq!(
                ordered.is_some(),
                has_order,
                "range={has_range} order={has_order}"
            );
            assert_eq!(
                view.as_range_reader().is_some(),
                has_range,
                "range={has_range} order={has_order}"
            );
            if let Some(ordered) = ordered {
                let entries = ordered.retail_read_dir(b"download").unwrap();
                assert!(
                    entries.len() == 2
                        && entries[0].path == b"download/z.tdf"
                        && entries[1].path == b"download/a.tdf",
                    "ordered entries = {entries:?}"
                );
                assert_eq!(inner.asked(), vec![b"downloadP".to_vec()]);
            } else {
                let entries = view.read_dir(b"download").unwrap();
                assert_eq!(entries.len(), 2, "fallback entries = {entries:?}");
            }
        }
    }
}
