// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/roots_test.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::hpi_write::ArchiveFile;
use super::test_support::{authored_archive, write_file, TempDir};
use super::{default_retail_mount_plan, Fs, GameRoot, Layout, ManifestOptions};

fn archive(path: impl AsRef<Path>, files: &[ArchiveFile]) {
    write_file(path, authored_archive(files, false));
}

fn source_paths(fs: &Fs, name: &str) -> Vec<PathBuf> {
    fs.sources(name)
        .into_iter()
        .map(|info| info.source.source_path)
        .collect()
}

/// Root precedence is host policy, including archive-over-loose resolution across
/// roots.
#[test]
fn ordered_roots_override_whole_earlier_root() {
    let first = TempDir::new();
    let second = TempDir::new();
    let shared = |data: &str| [ArchiveFile::new("shared.txt", data)];
    archive(first.join("totala1.hpi"), &shared("first base"));
    archive(first.join("patch.ccx"), &shared("first expansion"));
    archive(first.join("rev31.gp3"), &shared("first patch"));
    archive(second.join("totala1.hpi"), &shared("second base"));
    write_file(first.join("shared.txt"), "first loose");

    let mut fs = Fs::new();
    fs.mount_game_directories(&[first.path(), second.path()])
        .unwrap();
    assert_eq!(fs.read_file("shared.txt").unwrap(), b"second base");
    let want = vec![
        second.join("totala1.hpi"),
        first.join("shared.txt"),
        first.join("rev31.gp3"),
        first.join("patch.ccx"),
        first.join("totala1.hpi"),
    ];
    assert_eq!(source_paths(&fs, "shared.txt"), want);
    // The same ordering reaches enumeration and the manifest.
    for entry in fs.read_dir("").unwrap() {
        if entry.name == b"shared.txt" {
            assert_eq!(
                entry.source.source_path, want[0],
                "enumeration lost root priority"
            );
        }
    }
    for record in fs.manifest(ManifestOptions::default()).unwrap() {
        if record.logical_path == b"shared.txt" {
            assert!(
                record.provider_id == "totala1.hpi" && record.shadowed.len() == 4,
                "manifest = {record:?}"
            );
        }
    }
    let mut reversed = Fs::new();
    reversed
        .mount_game_directories(&[second.path(), first.path()])
        .unwrap();
    assert_eq!(reversed.read_file("shared.txt").unwrap(), b"first loose");
    fs.close().unwrap();
    reversed.close().unwrap();
}

#[test]
fn one_root_preserves_existing_manifest() {
    let root = TempDir::new();
    write_file(root.join("x"), "x");
    let mut original = Fs::new();
    let mut layered = Fs::new();
    original.mount_game_directory(root.path()).unwrap();
    layered
        .mount_game_directories(&[root.path(), root.path()])
        .unwrap();
    assert_eq!(
        original.manifest_hash().unwrap(),
        layered.manifest_hash().unwrap(),
        "single/repeated root changed manifest identity"
    );
}

/// An online battle mounts the base install with only the archives its room names
/// and none of its loose files, and the roots keep the priority spans their
/// positions give them.
#[test]
fn game_roots_admit_named_archives_without_loose() {
    let base = TempDir::new();
    let module = TempDir::new();
    archive(
        base.join("totala1.hpi"),
        &[
            ArchiveFile::new("units/base.fbi", "base"),
            ArchiveFile::new("shared.txt", "base"),
        ],
    );
    archive(
        base.join("Extra.ufo"),
        &[ArchiveFile::new("units/extra.fbi", "extra")],
    );
    write_file(base.join("units").join("loose.fbi"), "loose");
    write_file(module.join("shared.txt"), "mod");

    let mut fs = Fs::new();
    let roots = [
        GameRoot {
            archives: Some(Box::new(|name: &str| name == "totala1.hpi")),
            no_loose: true,
            ..GameRoot::new(base.path())
        },
        GameRoot::new(module.path()),
    ];
    fs.mount_game_roots(&roots).unwrap();
    assert_eq!(fs.read_file("units/base.fbi").unwrap(), b"base");
    for left in ["units/extra.fbi", "units/loose.fbi"] {
        assert!(fs.stat(left).is_err(), "{left} is mounted");
    }
    assert_eq!(fs.read_file("shared.txt").unwrap(), b"mod");
    let mut whole = Fs::new();
    whole
        .mount_game_directories(&[base.path(), module.path()])
        .unwrap();
    for path in ["units/base.fbi", "shared.txt"] {
        let (a, b) = (fs.sources(path), whole.sources(path));
        assert!(
            !a.is_empty() && !b.is_empty() && a[0].source.priority == b[0].source.priority,
            "{path}: filtered winner {a:?}, whole {b:?}"
        );
    }
    fs.close().unwrap();
    whole.close().unwrap();
}

/// An extra archive mounts as if it sat in the root: in its tier's lexical order
/// by file name, replacing a same-named archive of the folder.
#[test]
fn game_root_extra_archives_join_the_folders_tier() {
    let base = TempDir::new();
    let library = TempDir::new();
    let write = |root: &TempDir, name: &str, data: &str| {
        archive(
            root.join(name),
            &[
                ArchiveFile::new("shared.txt", data),
                ArchiveFile::new(format!("{name}.txt"), data),
            ],
        );
    };
    write(&base, "totala1.hpi", "base");
    write(&base, "Alpha.ufo", "alpha");
    write(&base, "Zulu.ufo", "zulu");
    write(&base, "Mike.ufo", "folder mike");
    write(&library, "MIKE.ufo", "library mike");
    write(&library, "Bravo.ufo", "bravo");
    let mut fs = Fs::new();
    let roots = [GameRoot {
        archives: Some(Box::new(|name: &str| name != "Bravo.ufo")),
        extra: vec![library.join("MIKE.ufo"), library.join("Bravo.ufo")],
        ..GameRoot::new(base.path())
    }];
    fs.mount_game_roots(&roots).unwrap();
    let want = vec![
        base.join("Zulu.ufo"),
        library.join("MIKE.ufo"),
        library.join("Bravo.ufo"),
        base.join("Alpha.ufo"),
        base.join("totala1.hpi"),
    ];
    assert_eq!(source_paths(&fs, "shared.txt"), want);
    assert!(
        fs.stat("Bravo.ufo.txt").is_ok(),
        "the root's archive filter left out an extra archive"
    );
    fs.close().unwrap();
}

/// An extra archive with a layout answers under the layout's directories: a
/// content set that reads units from unitsE finds the archive's units/ files beside
/// its own, through the layout's view under the retail name, while every other
/// directory keeps its path.
#[test]
fn game_root_extra_layout_rehomes_an_archive() {
    let base = TempDir::new();
    let module = TempDir::new();
    let library = TempDir::new();
    archive(
        base.join("totala1.hpi"),
        &[ArchiveFile::new("units/armcom.fbi", "base")],
    );
    archive(
        module.join("mod.gp3"),
        &[ArchiveFile::new("unitsE/armcom.fbi", "mod")],
    );
    let faction = library.join("faction.ufo");
    archive(
        &faction,
        &[
            ArchiveFile::new("Units/XenoCom.fbi", "xeno"),
            ArchiveFile::new("objects3d/xenocom.3do", "model"),
        ],
    );
    let layout = Layout::new([("units", "unitsE")]);
    let mut fs = Fs::new();
    let chosen = layout.clone();
    let faction_path = faction.clone();
    let roots = [
        GameRoot {
            extra: vec![faction.clone()],
            extra_layout: Some(Box::new(move |full: &Path| {
                if full == faction_path {
                    chosen.clone()
                } else {
                    Layout::default()
                }
            })),
            ..GameRoot::new(base.path())
        },
        GameRoot::new(module.path()),
    ];
    fs.mount_game_roots(&roots).unwrap();
    let fs = Arc::new(fs);
    let view = layout.apply(fs.clone());
    let mut names: Vec<Vec<u8>> = view
        .read_dir(b"units")
        .unwrap()
        .into_iter()
        .map(|entry| entry.path)
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec![b"units/armcom.fbi".to_vec(), b"units/xenocom.fbi".to_vec()],
        "units = the mod's and the archive's"
    );
    assert_eq!(
        view.read_file_limit(b"units/xenocom.fbi", 64).unwrap(),
        b"xeno"
    );
    assert_eq!(
        view.read_file_limit(b"units/armcom.fbi", 64).unwrap(),
        b"mod"
    );
    assert!(
        fs.stat("units/xenocom.fbi").is_err(),
        "the archive still answers under the renamed retail directory"
    );
    view.stat(b"objects3d/xenocom.3do").unwrap();
    let file = fs.open("unitse/xenocom.fbi").unwrap();
    let info = file.info();
    assert!(
        info.path == b"unitse/xenocom.fbi"
            && info.source.logical_path == b"unitse/xenocom.fbi"
            && info.original_path == b"Units/XenoCom.fbi",
        "opened file info {info:?}"
    );
}

/// An extra tier override mounts an add-on layer above the retail tiers: a faction
/// archive sits over a fix archive that replaced the patch, while the loose host
/// files stay on top.
#[test]
fn game_root_extra_tier_orders_an_addon_layer() {
    let base = TempDir::new();
    let library = TempDir::new();
    let one = |data: &str| [ArchiveFile::new("units/shared.fbi", data)];
    archive(base.join("Alpha.ufo"), &one("content"));
    archive(base.join("rev31.gp3"), &one("base patch"));
    let fix = library.join("rev31.gp3");
    archive(&fix, &one("fix"));
    let faction = library.join("Faction.ufo");
    archive(&faction, &one("faction"));
    let overlay = default_retail_mount_plan().gp3 + 1;

    let mut fix_only = Fs::new();
    fix_only
        .mount_game_roots(&[GameRoot {
            extra: vec![fix.clone()],
            ..GameRoot::new(base.path())
        }])
        .unwrap();
    assert_eq!(
        fix_only.read_file("units/shared.fbi").unwrap(),
        b"fix",
        "want the fix archive over the base patch and the content archive"
    );

    let mut layered = Fs::new();
    let faction_path = faction.clone();
    layered
        .mount_game_roots(&[GameRoot {
            extra: vec![fix.clone(), faction.clone()],
            extra_tier: Some(Box::new(move |full: &Path| {
                (full == faction_path).then_some(overlay)
            })),
            ..GameRoot::new(base.path())
        }])
        .unwrap();
    assert_eq!(
        layered.read_file("units/shared.fbi").unwrap(),
        b"faction",
        "want the faction archive over the fix archive"
    );

    let mut bad = Fs::new();
    let result = bad.mount_game_roots(&[GameRoot {
        extra: vec![faction.clone()],
        extra_tier: Some(Box::new(|_: &Path| Some(default_retail_mount_plan().loose))),
        ..GameRoot::new(base.path())
    }]);
    assert!(
        result.is_err(),
        "an override at the loose tier was admitted"
    );
    fix_only.close().unwrap();
    layered.close().unwrap();
    bad.close().unwrap();
}
