// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/mount_test.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! The mount tests that need no retail install. `TestMountRetailInstall`,
//! `TestLooseShadowsArchive`, `TestManifestHashStable` and
//! `TestReadFileRangeMatchesWholeFile` read the reference install and are not
//! ported.

#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use super::host;
use super::hpi_write::ArchiveFile;
use super::test_support::{authored_archive, write_file, TempDir};
use super::{default_retail_mount_plan, Fs, ManifestOptions};

/// An equal full path mounts once. The second attempt must not change the mount
/// count or the manifest.
#[test]
fn duplicate_mount_suppressed() {
    let dir = TempDir::new();
    write_file(dir.join("readme.txt"), "x");
    write_file(
        dir.join("fixture.hpi"),
        authored_archive(&[ArchiveFile::new("file", "data")], false),
    );
    let mut fs = Fs::new();
    fs.mount_game_directory(dir.path()).unwrap();
    let before = fs.mount_count();
    let first = fs.manifest_hash().unwrap();
    fs.mount_game_directory_with_plan(dir.path(), &default_retail_mount_plan())
        .unwrap();
    assert_eq!(
        fs.mount_count(),
        before,
        "duplicate full path must be suppressed"
    );
    assert_eq!(
        fs.manifest_hash().unwrap(),
        first,
        "manifest hash changed across a suppressed duplicate mount"
    );
    let notes = fs.notes().join("\n");
    assert!(
        !notes.contains(&host::lossy(dir.path()))
            && notes.contains("fixture.hpi")
            && notes.contains("loose root"),
        "duplicate diagnostics must use portable provider labels: {notes}"
    );
    fs.close().unwrap();
}

/// The same content at two different host roots produces the same manifest hash,
/// because a provider ID is an archive base name or a root-relative path.
#[test]
fn manifest_hash_portable() {
    let build = |root: &std::path::Path| {
        write_file(
            root.join("gamedata").join("moveinfo.tdf"),
            "[CLASS0]\n{\nName=box;\n}\n",
        );
        write_file(
            root.join("units").join("armcom.fbi"),
            "[UNITINFO]\n{\nunitname=ARMCOM;\n}\n",
        );
    };
    let a = TempDir::new();
    let b = TempDir::new();
    let deep = b.join("deeper").join("nested").join("install");
    build(a.path());
    build(&deep);

    let mut fs_a = Fs::new();
    fs_a.mount_directory(a.path(), 10).unwrap();
    let mut fs_b = Fs::new();
    fs_b.mount_directory(&deep, 10).unwrap();
    let hash_a = fs_a.manifest_hash().unwrap();
    let hash_b = fs_b.manifest_hash().unwrap();
    assert_eq!(hash_a, hash_b, "manifest hash depends on host root");
    let separator = std::path::MAIN_SEPARATOR_STR.repeat(2);
    for record in fs_a.manifest(ManifestOptions::default()).unwrap() {
        assert!(
            !record.provider_id.contains(&separator)
                && !std::path::Path::new(&record.provider_id).is_absolute(),
            "provider ID {:?} leaks a host path",
            record.provider_id
        );
    }
    fs_a.close().unwrap();
    fs_b.close().unwrap();
}

/// More than ten local HPIs are ordinary content: retail's budget is per mount
/// pass, so every provider stays available without a warning.
#[test]
fn more_than_ten_hpis_mount_without_warning() {
    let dir = TempDir::new();
    let archive = authored_archive(&[ArchiveFile::new("file", "data")], false);
    for i in 0..11 {
        write_file(dir.join(format!("dummy{i}.hpi")), &archive);
    }
    let mut fs = Fs::new();
    fs.mount_game_directory_with_plan(dir.path(), &default_retail_mount_plan())
        .unwrap();
    assert!(
        fs.notes().is_empty(),
        "valid archives produced mount warnings: {:?}",
        fs.notes()
    );
    assert_eq!(fs.sources("file").len(), 11, "want all 11 archives");
    fs.close().unwrap();
}

/// Not in Go: the manifest names the shadowed provider and hashes the winning
/// bytes, using only authored fixtures (the Go version of this check,
/// `TestLooseShadowsArchive`, needs the retail install).
#[test]
fn loose_shadows_archive_with_authored_fixtures() {
    let dir = TempDir::new();
    write_file(
        dir.join("base.hpi"),
        authored_archive(
            &[ArchiveFile::new("gamedata/moveinfo.tdf", "archive")],
            true,
        ),
    );
    let overlay = TempDir::new();
    write_file(overlay.join("gamedata").join("moveinfo.tdf"), "abc");
    let mut fs = Fs::new();
    fs.mount_game_directory(dir.path()).unwrap();
    fs.mount_directory(overlay.path(), 200).unwrap();
    assert_eq!(fs.read_file("gamedata/moveinfo.tdf").unwrap(), b"abc");
    let records = fs
        .manifest(ManifestOptions {
            hash: true,
            max_hash_size: 0,
        })
        .unwrap();
    let record = records
        .iter()
        .find(|record| record.logical_path == b"gamedata/moveinfo.tdf")
        .expect("manifest is missing the overlaid path");
    assert_eq!(record.provider_id, "gamedata/moveinfo.tdf");
    assert_eq!(record.shadowed, vec!["base.hpi".to_string()]);
    assert_eq!(
        record.hash,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    fs.close().unwrap();
}
