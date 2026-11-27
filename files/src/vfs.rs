// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/doc.go, vfs/vfs.go, vfs/hpi.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! The logical content namespace: an overlay of loose directories and HPI-family
//! archives resolved by mount order.
//!
//! Mounting indexes names and metadata only. Archive payloads are read and
//! decompressed when the corresponding file is opened, which keeps startup
//! independent of the size of the installed game data.
//!
//! Logical names are byte strings (`&[u8]`), as they were in Go: archive names are
//! legacy code-page bytes and distinct bytes must stay distinct. Case folding of
//! logical names is ASCII only; both `/` and `\` separate components.
//!
//! Higher mount priorities win; among equal priorities the later mount wins.
//! [`Fs::read_dir`] lists the union of a directory sorted by logical path;
//! [`Fs::retail_read_dir`] keeps provider order instead.
//!
//! Differences from the Go source, all deliberate:
//! - No nil readers or nil views: [`Archive::new`] takes a reader by value and
//!   [`Layout::apply`] takes an `Arc`, so Go's nil checks have no equivalent.
//! - Byte offsets are `u64` and "read to the end" is `None`, where Go used a
//!   negative `int64` offset (refused) and a negative `int` length.
//! - Optional capabilities (Go's interface type assertions) are the `as_*` methods
//!   of [`FsOps`].
//! - The archive directory walk and the loose directory walk are iterative, so a
//!   custom depth limit cannot overflow the stack.
//! - Archive files are closed when dropped; [`Fs::close`] drops every mount.
//! - Writing archives is not ported.

use std::fmt;
use std::io;
use std::path::PathBuf;

mod host;
mod hpi;
mod layout;
mod loose;
mod manifest;
mod overlay;
mod path;
mod provider_index;
mod rehome;
mod resource;

#[cfg(test)]
mod hpi_write;
#[cfg(test)]
mod test_support;

#[cfg(test)]
mod hpi_bounds_test;
#[cfg(test)]
mod hpi_contract_test;
#[cfg(test)]
mod hpi_duplicates_test;
#[cfg(test)]
mod hpi_limits_test;
#[cfg(test)]
mod hpi_write_test;
#[cfg(test)]
mod hpi_zlib_test;
#[cfg(test)]
mod layout_test;
#[cfg(test)]
mod mount_rejection_test;
#[cfg(test)]
mod mount_test;
#[cfg(test)]
mod path_bytes_test;
#[cfg(test)]
mod resource_test;
#[cfg(test)]
mod roots_test;
#[cfg(test)]
mod sha256_test;
#[cfg(test)]
mod vfs_range_test;

pub use hpi::{Archive, ArchiveOptions, ReadAt, DEFAULT_MAX_FILE_BYTES};
pub use layout::Layout;
pub use manifest::{ManifestOptions, ManifestRecord};
pub use overlay::{
    default_retail_mount_plan, is_archive_name, ArchiveFilter, EntryInfo, File, Fs, FsOps,
    GameRoot, LayoutChooser, ManifestView, MountPlan, MountTier, PinnedMount, Provenance,
    ProviderInfo, RangeReader, RetailDirReader, SharedFs, SourcesView, TierChooser,
};
pub use resource::resource_path;

/// Errors of the namespace and the archive reader.
///
/// Go used wrapped sentinel errors; the predicates (`is_not_found` and the rest)
/// answer what `errors.Is` answered, looking through [`Error::MountRoot`] and
/// [`Error::Manifest`].
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// No mount holds the logical path (Go `ErrNotFound`). Also a range offset
    /// past the end of a loose file.
    NotFound(String),
    /// The path names a directory where a file was needed (Go `ErrIsDir`).
    IsDir(String),
    /// The file exceeds the caller's read limit (Go `ErrTooLarge`).
    TooLarge(String),
    /// The archive's header, directory or chunk stream does not decode (Go
    /// `ErrMalformedArchive`).
    MalformedArchive(String),
    /// The archive failed the container gate: marker, version, footer or a short
    /// header read (Go `ErrRejectedArchive`). It is also a malformed archive.
    RejectedArchive(String),
    /// An absolute path or a `..` component.
    InvalidPath(String),
    /// A mount plan leaves a tier or its revision unset.
    InvalidMountPlan,
    /// A [`GameRoot::extra_tier`] override is not below the loose tier.
    ExtraTier {
        archive: PathBuf,
        tier: MountTier,
        loose: MountTier,
    },
    /// A loose mount root that is not a directory.
    NotADirectory(PathBuf),
    /// A host file could not be opened (Go `*os.PathError` with `Op == "open"`).
    Open { path: PathBuf, source: io::Error },
    /// Any other host I/O failure, including a failing archive reader.
    Io {
        path: Option<PathBuf>,
        source: io::Error,
    },
    /// A failure while mounting one root of [`Fs::mount_game_roots`].
    MountRoot { root: PathBuf, source: Box<Error> },
    /// A failure while hashing a manifest entry.
    Manifest { path: String, source: Box<Error> },
    /// A forwarded capability the wrapped view does not have.
    Unsupported(&'static str),
}

impl Error {
    /// The innermost error, through the wrapping variants.
    fn innermost(&self) -> &Error {
        let mut error = self;
        loop {
            match error {
                Error::MountRoot { source, .. } | Error::Manifest { source, .. } => {
                    error = source;
                }
                other => return other,
            }
        }
    }

    /// Go `errors.Is(err, ErrNotFound)`.
    pub fn is_not_found(&self) -> bool {
        matches!(self.innermost(), Error::NotFound(_))
    }

    /// Go `errors.Is(err, ErrIsDir)`.
    pub fn is_dir(&self) -> bool {
        matches!(self.innermost(), Error::IsDir(_))
    }

    /// Go `errors.Is(err, ErrTooLarge)`.
    pub fn is_too_large(&self) -> bool {
        matches!(self.innermost(), Error::TooLarge(_))
    }

    /// Go `errors.Is(err, ErrMalformedArchive)`: true for rejected archives too.
    pub fn is_malformed_archive(&self) -> bool {
        matches!(
            self.innermost(),
            Error::MalformedArchive(_) | Error::RejectedArchive(_)
        )
    }

    /// Go `errors.Is(err, ErrRejectedArchive)`.
    pub fn is_rejected_archive(&self) -> bool {
        matches!(self.innermost(), Error::RejectedArchive(_))
    }

    pub(crate) fn io(source: io::Error) -> Error {
        Error::Io { path: None, source }
    }

    pub(crate) fn io_at(path: impl Into<PathBuf>, source: io::Error) -> Error {
        Error::Io {
            path: Some(path.into()),
            source,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotFound(detail) => write!(f, "vfs: file not found: {detail}"),
            Error::IsDir(detail) => write!(f, "vfs: path is a directory: {detail}"),
            Error::TooLarge(detail) => write!(f, "vfs: file exceeds read limit: {detail}"),
            Error::MalformedArchive(detail) => write!(f, "vfs: malformed HPI archive: {detail}"),
            Error::RejectedArchive(detail) => {
                write!(
                    f,
                    "vfs: malformed HPI archive: rejected container: {detail}"
                )
            }
            Error::InvalidPath(message) => f.write_str(message),
            Error::InvalidMountPlan => f.write_str("vfs: invalid mount plan"),
            Error::ExtraTier {
                archive,
                tier,
                loose,
            } => write!(
                f,
                "archive {}: extra tier {tier} is not below the loose tier {loose}",
                archive.display()
            ),
            Error::NotADirectory(root) => write!(f, "vfs: {} is not a directory", root.display()),
            Error::Open { path, source } => write!(f, "open {}: {source}", path.display()),
            Error::Io {
                path: Some(path),
                source,
            } => write!(f, "{}: {source}", path.display()),
            Error::Io { path: None, source } => write!(f, "{source}"),
            Error::MountRoot { root, source } => {
                write!(
                    f,
                    "mounting root {:?}: {source}",
                    root.display().to_string()
                )
            }
            Error::Manifest { path, source } => write!(f, "vfs: manifest read {path}: {source}"),
            Error::Unsupported(what) => write!(f, "vfs: {what} is not supported by this view"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Open { source, .. } | Error::Io { source, .. } => Some(source),
            Error::MountRoot { source, .. } | Error::Manifest { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Lossy display of a logical byte path, for messages.
pub(crate) fn show(name: &[u8]) -> String {
    String::from_utf8_lossy(name).into_owned()
}
