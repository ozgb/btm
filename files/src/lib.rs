//! Total Annihilation 3.1c data loading for btm.
//!
//! Ported from Nanolathe (MIT), commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
//! See `LICENSE-nanolathe`. Each ported file names its upstream path in its header.
//!
//! - [`vfs`]: HPI archives and loose files behind one case-insensitive namespace.
//! - [`tdf`]: TDF and FBI text files.
//! - [`cob`]: COB script files (the file, not the VM).
//! - [`gaf`]: GAF image archives.
//! - [`three_do`]: 3DO models.
//!
//! The parsers take bytes and never touch the file system. Nothing here panics on bad
//! input, and no allocation is larger than the input allows.

#![forbid(unsafe_code)]
// Bad input must never panic: these hold outside tests.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![warn(clippy::indexing_slicing)]

mod digest;

pub mod cob;
pub mod gaf;
pub mod tdf;
pub mod three_do;
pub mod vfs;
