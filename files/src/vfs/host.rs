//! Host path and string helpers that reproduce the Go standard-library behaviour the
//! ported namespace relies on (`path/filepath`, `path.Base`, `strings.ToLower`,
//! `strings.EqualFold`). This file is not ported from Nanolathe; it is written for
//! this crate so that the ported code keeps Go's semantics on every platform.

use std::ffi::OsStr;
use std::io;
use std::path::{Component, Path, PathBuf};

/// Whether `c` separates host path elements (Go `os.IsPathSeparator`).
fn is_separator(c: char) -> bool {
    c == '/' || (cfg!(windows) && c == '\\')
}

/// The host path as a string. Go keeps host names as strings; on Windows they are
/// always valid UTF-8 (unpaired surrogates become U+FFFD), which is what a lossy
/// conversion produces. On Unix Go keeps raw bytes; this keeps the lossy text.
pub(crate) fn lossy(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// The bytes Go would hold for one host file name (`fs.DirEntry.Name()`): raw bytes on
/// Unix, the UTF-8 conversion elsewhere.
pub(crate) fn name_bytes(name: &OsStr) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        name.as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        name.to_string_lossy().into_owned().into_bytes()
    }
}

/// Go `filepath.Clean`: lexical, `..` resolved against preceding elements, and an
/// empty result is `.`.
pub(crate) fn clean(path: &Path) -> PathBuf {
    let mut out: Vec<Component<'_>> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match out.last() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                // `..` at the root stays at the root.
                Some(Component::RootDir) => {}
                _ => out.push(component),
            },
            other => out.push(other),
        }
    }
    if out.is_empty() {
        return PathBuf::from(".");
    }
    out.iter().collect()
}

/// Go `filepath.Abs`: an absolute, cleaned path.
pub(crate) fn abs(path: &Path) -> io::Result<PathBuf> {
    std::path::absolute(path).map(|absolute| clean(&absolute))
}

/// Go `filepath.Base` over the lossy host string.
pub(crate) fn base(path: &Path) -> String {
    let text = lossy(path);
    let mut rest: &str = &text;
    if rest.is_empty() {
        return ".".to_string();
    }
    if cfg!(windows) {
        rest = strip_volume(rest);
    }
    let trimmed = rest.trim_end_matches(is_separator);
    if trimmed.is_empty() {
        return std::path::MAIN_SEPARATOR.to_string();
    }
    match trimmed.rfind(is_separator) {
        Some(at) => trimmed.get(at + 1..).unwrap_or(trimmed).to_string(),
        None => trimmed.to_string(),
    }
}

/// The drive-letter volume Go's `filepath.VolumeName` strips on Windows. UNC volumes
/// are not split off; a UNC path's base is still its last element.
fn strip_volume(path: &str) -> &str {
    let bytes = path.as_bytes();
    if bytes.len() >= 2
        && bytes.get(1) == Some(&b':')
        && bytes.first().is_some_and(u8::is_ascii_alphabetic)
    {
        return path.get(2..).unwrap_or(path);
    }
    path
}

/// Go `filepath.Ext`: the suffix from the final `.` of the last element, or "".
pub(crate) fn ext(name: &str) -> &str {
    for (at, c) in name.char_indices().rev() {
        if is_separator(c) {
            return "";
        }
        if c == '.' {
            return name.get(at..).unwrap_or("");
        }
    }
    ""
}

/// Go `filepath.ToSlash`.
pub(crate) fn to_slash(path: &str) -> String {
    if cfg!(windows) {
        path.replace('\\', "/")
    } else {
        path.to_string()
    }
}

/// Go `path.Base`: slash-separated only, whatever the host.
pub(crate) fn slash_base(path: &str) -> &str {
    if path.is_empty() {
        return ".";
    }
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return "/";
    }
    match trimmed.rfind('/') {
        Some(at) => trimmed.get(at + 1..).unwrap_or(trimmed),
        None => trimmed,
    }
}

/// The path of `target` below `root` with `/` separators, `.` when they are equal,
/// or `None` when `target` is not below `root`. It answers Go's
/// `filepath.Rel(root, target)` whenever that result does not start with `..`, which
/// is the only case `Provenance::provider_id` uses.
pub(crate) fn rel_below(root: &Path, target: &Path) -> Option<String> {
    let root = clean(root);
    let target = clean(target);
    let rest = target.strip_prefix(&root).ok()?;
    let parts: Vec<String> = rest
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();
    if parts.is_empty() {
        return Some(".".to_string());
    }
    Some(parts.join("/"))
}

/// Go `unicode.ToLower` for one rune. Rust's lowercase mapping is a single character
/// for every code point except U+0130, whose first character is Go's mapping.
fn lower_char(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// Go `strings.ToLower` for a valid UTF-8 string: rune by rune, with no
/// context-sensitive rules (Rust's `str::to_lowercase` maps a final sigma
/// differently, so it is not used).
pub(crate) fn lower(text: &str) -> String {
    text.chars().map(lower_char).collect()
}

/// One rune as Go's `utf8.DecodeRune` reads it: an invalid or truncated sequence is
/// U+FFFD with width 1.
fn decode_rune(bytes: &[u8]) -> Option<(char, usize)> {
    let first = *bytes.first()?;
    if first < 0x80 {
        return Some((char::from(first), 1));
    }
    let window = bytes.get(..bytes.len().min(4)).unwrap_or(bytes);
    let valid = match std::str::from_utf8(window) {
        Ok(text) => text,
        Err(error) => window
            .get(..error.valid_up_to())
            .and_then(|prefix| std::str::from_utf8(prefix).ok())
            .unwrap_or(""),
    };
    match valid.chars().next() {
        Some(c) => Some((c, c.len_utf8())),
        None => Some((char::REPLACEMENT_CHARACTER, 1)),
    }
}

/// A single-character case mapping, or the character itself.
fn single(mut mapping: impl Iterator<Item = char>, c: char) -> char {
    match (mapping.next(), mapping.next()) {
        (Some(mapped), None) => mapped,
        _ => c,
    }
}

/// Whether two runes are case variants. Go walks `unicode.SimpleFold` orbits; this
/// compares single-character lower and upper mappings, which agrees with Go except
/// in rare orbits. The Turkish dotted and dotless I (U+0130, U+0131) fold to nothing
/// else in Go, and are excluded here to match.
fn fold_equal(a: char, b: char) -> bool {
    if a == b {
        return true;
    }
    let turkish = |c: char| c == '\u{130}' || c == '\u{131}';
    if turkish(a) || turkish(b) {
        return false;
    }
    single(a.to_lowercase(), a) == single(b.to_lowercase(), b)
        || single(a.to_uppercase(), a) == single(b.to_uppercase(), b)
}

/// Go `strings.EqualFold` over byte strings: runes are decoded the way Go decodes
/// them (so two different invalid bytes are both U+FFFD and compare equal), ASCII
/// folds exactly, and other runes fold by [`fold_equal`].
pub(crate) fn equal_fold(mut a: &[u8], mut b: &[u8]) -> bool {
    loop {
        match (decode_rune(a), decode_rune(b)) {
            (None, None) => return true,
            (Some((ra, wa)), Some((rb, wb))) => {
                // Go folds two ASCII runes by A-Z only; a non-ASCII rune can still
                // fold to an ASCII one (K, U+212A, and k).
                let same = if ra.is_ascii() && rb.is_ascii() {
                    ra.eq_ignore_ascii_case(&rb)
                } else {
                    fold_equal(ra, rb)
                };
                if !same {
                    return false;
                }
                a = a.get(wa..).unwrap_or(&[]);
                b = b.get(wb..).unwrap_or(&[]);
            }
            _ => return false,
        }
    }
}
