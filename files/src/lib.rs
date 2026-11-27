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
//! - [`tnt`]: TNT terrain (tiles, height and feature grid, minimap).
//! - [`ota`]: OTA map and mission descriptions (TDF with a `GlobalHeader`).
//! - [`pcx`]: PCX images.
//! - [`pal`]: PAL palettes.
//! - [`fnt`]: FNT bitmap fonts.
//! - [`gui`]: GUI panel definitions (TDF gadgets, and binary panels).
//! - [`wav`]: WAV sound metadata (RIFF, DIGI and raw).
//!
//! The parsers take bytes and never touch the file system. Nothing here panics on bad
//! input, and no allocation is larger than the input allows.

#![forbid(unsafe_code)]
// Bad input must never panic: these hold outside tests.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![warn(clippy::indexing_slicing)]

mod digest;
mod le;

pub mod cob;
pub mod fnt;
pub mod gaf;
pub mod gui;
pub mod ota;
pub mod pal;
pub mod pcx;
pub mod tdf;
pub mod three_do;
pub mod tnt;
pub mod vfs;
pub mod wav;
