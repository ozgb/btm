// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/hpi_zlib_test.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use miniz_oxide::deflate::core::{
    compress, create_comp_flags_from_zip_params, CompressorOxide, TDEFLFlush, TDEFLStatus,
};

use super::hpi::decode_zlib;

/// Compresses `payload`; with `finish` false the stream is only sync-flushed, so it
/// has no final deflate block and no Adler-32 trailer, as third-party packers write
/// the chunks of the community "V maps.ufo" pack.
fn zlib_stream(payload: &[u8], finish: bool) -> Vec<u8> {
    if finish {
        return miniz_oxide::deflate::compress_to_vec_zlib(payload, 6);
    }
    let mut compressor = CompressorOxide::new(create_comp_flags_from_zip_params(6, 15, 0));
    let mut out = vec![0u8; payload.len() + 1024];
    let (status, consumed, written) =
        compress(&mut compressor, payload, &mut out, TDEFLFlush::Sync);
    assert_eq!(status, TDEFLStatus::Okay);
    assert_eq!(consumed, payload.len());
    out.truncate(written);
    out
}

fn decode(input: &[u8], expected: u64) -> Result<Vec<u8>, String> {
    let mut once = Some(input);
    decode_zlib(
        &mut |buf: &mut Vec<u8>| {
            if let Some(piece) = once.take() {
                buf.extend_from_slice(piece);
            }
            Ok(())
        },
        expected,
    )
}

/// The same stream fed a few bytes at a time, as the streamed chunk path feeds it.
fn decode_in_pieces(input: &[u8], expected: u64, piece: usize) -> Result<Vec<u8>, String> {
    let mut rest = input;
    decode_zlib(
        &mut |buf: &mut Vec<u8>| {
            let take = piece.min(rest.len());
            buf.extend_from_slice(&rest[..take]);
            rest = &rest[take..];
            Ok(())
        },
        expected,
    )
}

#[test]
fn zlib_accepts_stream_without_final_block() {
    let payload = b"total annihilation ".repeat(400);
    let complete = zlib_stream(&payload, true);
    let open = zlib_stream(&payload, false);
    let len = payload.len() as u64;
    let cases: Vec<(&str, &[u8], u64, bool)> = vec![
        ("complete stream", &complete, len, true),
        ("no final block", &open, len, true),
        (
            "no final block, fewer bytes than declared",
            &open,
            len + 1,
            false,
        ),
        (
            "no final block, more bytes than declared",
            &open,
            len - 1,
            false,
        ),
        ("cut inside the data", &open[..open.len() / 2], len, false),
    ];
    for (name, input, expected, valid) in cases {
        for piece in [usize::MAX, 1, 7, 4096] {
            let got = if piece == usize::MAX {
                decode(input, expected)
            } else {
                decode_in_pieces(input, expected, piece)
            };
            assert_eq!(got.is_ok(), valid, "{name} (piece {piece}): {got:?}");
            if valid {
                assert!(got.unwrap() == payload, "{name}: decoded bytes differ");
            }
        }
    }
}

/// Not in Go: the error cases Go's reader reported that the test above does not
/// reach.
#[test]
fn zlib_rejects_a_bad_trailer_and_a_missing_header() {
    let payload = b"total annihilation ".repeat(40);
    let mut stream = zlib_stream(&payload, true);
    let last = stream.len() - 1;
    stream[last] ^= 1;
    assert!(decode(&stream, payload.len() as u64).is_err());
    assert!(decode(&[], 0).is_err());
    assert!(decode(&[0x78], 0).is_err());
}
