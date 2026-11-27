//! Bounds-checked little-endian reads for the map and presentation parsers.
//!
//! Go indexed the slice and relied on an earlier length check; these return `None`
//! instead, so a missed check is an error and never a panic.

/// The `u16` at `pos`, or `None` when it does not fit inside `data`.
pub(crate) fn u16_at(data: &[u8], pos: usize) -> Option<u16> {
    let bytes: [u8; 2] = data.get(pos..pos.checked_add(2)?)?.try_into().ok()?;
    Some(u16::from_le_bytes(bytes))
}

/// The `u32` at `pos`, or `None` when it does not fit inside `data`.
pub(crate) fn u32_at(data: &[u8], pos: usize) -> Option<u32> {
    let bytes: [u8; 4] = data.get(pos..pos.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}
