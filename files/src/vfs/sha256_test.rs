//! Known-answer tests for the SHA-256 the manifest uses. Not ported: Go relied on
//! `crypto/sha256`.

#![allow(clippy::indexing_slicing)]

use crate::digest::{hex, Sha256};

fn digest(parts: &[&[u8]]) -> String {
    let mut sha = Sha256::new();
    for part in parts {
        sha.update(part);
    }
    hex(&sha.finish())
}

#[test]
fn sha256_known_answers() {
    assert_eq!(
        digest(&[b""]),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        digest(&[b"abc"]),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let two_blocks: &[u8] = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
    assert_eq!(
        digest(&[two_blocks]),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
    // The same input split across updates.
    assert_eq!(
        digest(&[&two_blocks[..5], &two_blocks[5..]]),
        digest(&[two_blocks])
    );
    let million = vec![b'a'; 1_000_000];
    assert_eq!(
        digest(&[&million]),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    );
}
