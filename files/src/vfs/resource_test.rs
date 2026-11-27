// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/resource_test.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

use super::resource_path;

#[test]
fn resource_path_replaces_last_period_across_whole_path() {
    for (name, want) in [
        ("", "objects3d/.3do"),
        ("model.3do", "objects3d/model.3do"),
        ("model.other", "objects3d/model.3do"),
        ("folder.old/model", "objects3d/folder.3do"),
        ("folder.old/model.new", "objects3d/folder.old/model.3do"),
    ] {
        let got = resource_path(b"objects3d", name.as_bytes(), b"3do");
        assert_eq!(String::from_utf8_lossy(&got), want, "{name:?}");
    }
}
