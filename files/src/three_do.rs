// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), formats/three_do.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! 3DO models: reading only.
//!
//! A model is a tree of pieces. Each piece has a pivot translation, vertices and
//! primitives (faces). The file stores pieces as 52-byte records linked by absolute
//! offsets; [`ThreeDo::parse`] resolves those links to indexes into
//! [`ThreeDo::objects`], in file-walk order (a piece, then its children, then its
//! siblings), and keeps the source-level words.
//!
//! Differences from the Go original, all deliberate:
//!
//! - Go parses with mutual recursion bounded by `MaxDepth`. Here the walk keeps its
//!   own stack, so no limit can overflow the call stack; the object, depth, cycle
//!   and sharing checks are the same, in the same order.
//! - Go's `ModelTop` recurses and would not terminate on a hand-built model whose
//!   child links loop. [`ThreeDo::model_top`] is iterative and visits each piece at
//!   most once, so it always terminates. Parsed models are trees, so results agree.
//! - Names are byte strings (Go strings are bytes), not forced to UTF-8.
//! - `ContentHash` is SHA-256 from the `sha2` crate, via `crate::digest`.

#[cfg(test)]
mod tests;
#[cfg(test)]
mod write;

use std::collections::HashSet;
use std::fmt;

use crate::digest::sha256;

/// Bounds on the object, vertex and primitive counts one model may declare. A zero
/// field is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreeDoLimits {
    pub max_depth: u32,
    pub max_objects: u32,
    pub max_vertices: u32,
    pub max_primitives: u32,
    pub max_polygon_vertices: u32,
    pub max_indices: u64,
    pub max_string_bytes: u32,
}

impl Default for ThreeDoLimits {
    /// The decode bounds used when a caller states none.
    fn default() -> Self {
        ThreeDoLimits {
            max_depth: 64,
            max_objects: 1 << 16,
            max_vertices: 1 << 22,
            max_primitives: 1 << 22,
            max_polygon_vertices: 1 << 16,
            max_indices: 1 << 22,
            max_string_bytes: 1 << 20,
        }
    }
}

/// One model-space vertex in the file's own integer units (16.16 fixed point).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ThreeDoVertex {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// One face: its vertex indices, its texture name and the per-face words the file
/// carries.
///
/// `always_zero` is an optional auxiliary file offset whose target layout and
/// meaning are unknown; its raw word is kept.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThreeDoPrimitive {
    pub color_index: u32,
    pub vertex_indices: Vec<u16>,
    /// Empty when the face has no texture.
    pub texture_name: Vec<u8>,
    pub always_zero: i32,
    pub unknown1: i32,
    pub unknown2: i32,
    pub is_colored: i32,
    pub source_offset: u32,
}

/// The source-level piece record plus resolved topology indexes. `parent`,
/// `first_child` and `next_sibling` index [`ThreeDo::objects`]; -1 is none.
///
/// `always_zero` is an optional auxiliary file offset whose target layout and
/// meaning are unknown; its raw word is kept.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThreeDoObject {
    pub source_offset: u32,
    pub version: i32,
    pub always_zero: i32,
    pub selection: i32,
    pub translation: [i32; 3],
    pub name: Vec<u8>,
    pub vertices: Vec<ThreeDoVertex>,
    pub primitives: Vec<ThreeDoPrimitive>,
    pub parent: i32,
    pub first_child: i32,
    pub next_sibling: i32,
}

/// A decoded model: the piece hierarchy with each piece's vertices, primitives and
/// pivot.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThreeDo {
    /// SHA-256 of `raw`.
    pub content_hash: [u8; 32],
    /// The one source backing store used during parsing, copied from the input.
    pub raw: Vec<u8>,
    pub root: i32,
    pub objects: Vec<ThreeDoObject>,
}

/// Why a range or string read failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeFault {
    /// The offset is negative or the count times the stride overflows.
    InvalidOffsetOrCount,
    /// `offset + size` is past the end of the file.
    OutsideFile { offset: u32, size: u64 },
    /// A string starts outside the file.
    StringOutsideFile { offset: u32 },
    /// No NUL within the string byte limit or the file.
    UnterminatedString { offset: u32 },
}

impl fmt::Display for RangeFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RangeFault::InvalidOffsetOrCount => write!(f, "invalid offset/count"),
            RangeFault::OutsideFile { offset, size } => {
                write!(f, "range 0x{offset:x}+{size} outside file")
            }
            RangeFault::StringOutsideFile { offset } => {
                write!(f, "string offset 0x{offset:x} outside file")
            }
            RangeFault::UnterminatedString { offset } => {
                write!(f, "unterminated string at 0x{offset:x}")
            }
        }
    }
}

/// Everything that can go wrong reading a 3DO. The `Display` text follows the Go
/// messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreeDoError {
    /// The file is shorter than one 52-byte object record.
    RootTruncated,
    /// A limit is zero.
    InvalidLimits,
    /// A non-root object points at offset 0.
    ObjectOffsetReused,
    DepthExceeded,
    ObjectOutsideFile {
        offset: u32,
    },
    ObjectCycle {
        offset: u32,
    },
    SharedObject {
        offset: u32,
    },
    ObjectCountExceeded,
    BadVersion {
        offset: u32,
        version: i32,
    },
    NegativeCount {
        offset: u32,
    },
    GeometryCountExceeded,
    ObjectName {
        offset: u32,
        fault: RangeFault,
    },
    ObjectVertices {
        offset: u32,
        fault: RangeFault,
    },
    ObjectPrimitives {
        offset: u32,
        fault: RangeFault,
    },
    InvalidPolygonVertexCount {
        primitive: usize,
        offset: u32,
        count: i32,
    },
    AggregateIndexesExceeded,
    PrimitiveIndexes {
        primitive: usize,
        fault: RangeFault,
    },
    VertexIndexOutOfRange {
        primitive: usize,
        index: u16,
        vertex_count: i32,
    },
    PrimitiveTexture {
        primitive: usize,
        fault: RangeFault,
    },
}

impl fmt::Display for ThreeDoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ThreeDoError::RootTruncated => write!(f, "3do: root object is truncated"),
            ThreeDoError::InvalidLimits => write!(f, "3do: invalid decode limits"),
            ThreeDoError::ObjectOffsetReused => write!(f, "3do: object offset 0 is reused"),
            ThreeDoError::DepthExceeded => write!(f, "3do: hierarchy depth exceeds limit"),
            ThreeDoError::ObjectOutsideFile { offset } => {
                write!(f, "3do: object at 0x{offset:x} is outside file")
            }
            ThreeDoError::ObjectCycle { offset } => write!(f, "3do: object cycle at 0x{offset:x}"),
            ThreeDoError::SharedObject { offset } => {
                write!(f, "3do: shared object at 0x{offset:x}")
            }
            ThreeDoError::ObjectCountExceeded => write!(f, "3do: object count exceeds limit"),
            ThreeDoError::BadVersion { offset, version } => {
                write!(f, "3do: object at 0x{offset:x} has version {version}")
            }
            ThreeDoError::NegativeCount { offset } => {
                write!(f, "3do: negative count at 0x{offset:x}")
            }
            ThreeDoError::GeometryCountExceeded => write!(f, "3do: geometry count exceeds limits"),
            ThreeDoError::ObjectName { offset, fault } => {
                write!(f, "3do: object 0x{offset:x} name: {fault}")
            }
            ThreeDoError::ObjectVertices { offset, fault } => {
                write!(f, "3do: object 0x{offset:x} vertices: {fault}")
            }
            ThreeDoError::ObjectPrimitives { offset, fault } => {
                write!(f, "3do: object 0x{offset:x} primitives: {fault}")
            }
            ThreeDoError::InvalidPolygonVertexCount {
                primitive,
                offset,
                count,
            } => write!(
                f,
                "3do: primitive {primitive} at object 0x{offset:x} has invalid vertex count {count}"
            ),
            ThreeDoError::AggregateIndexesExceeded => {
                write!(f, "3do: aggregate polygon indexes exceed limit")
            }
            ThreeDoError::PrimitiveIndexes { primitive, fault } => {
                write!(f, "3do: primitive {primitive} indexes: {fault}")
            }
            ThreeDoError::VertexIndexOutOfRange {
                primitive,
                index,
                vertex_count,
            } => write!(
                f,
                "3do: primitive {primitive} vertex index {index} exceeds vertex count {vertex_count}"
            ),
            ThreeDoError::PrimitiveTexture { primitive, fault } => {
                write!(f, "3do: primitive {primitive} texture: {fault}")
            }
        }
    }
}

impl std::error::Error for ThreeDoError {}

fn u32_at(data: &[u8], pos: usize) -> Option<u32> {
    let end = pos.checked_add(4)?;
    match data.get(pos..end)? {
        [a, b, c, d] => Some(u32::from_le_bytes([*a, *b, *c, *d])),
        _ => None,
    }
}

/// Go reads these words as `int32`; the cast reinterprets the same four bytes.
fn i32_at(data: &[u8], pos: usize) -> Option<i32> {
    u32_at(data, pos).map(|v| v as i32)
}

fn u16_at(data: &[u8], pos: usize) -> Option<u16> {
    let end = pos.checked_add(2)?;
    match data.get(pos..end)? {
        [a, b] => Some(u16::from_le_bytes([*a, *b])),
        _ => None,
    }
}

/// `count * stride` bytes at `off`, which must lie in the file. A negative offset or
/// an overflowing size is invalid; Go rejects an offset past the end even for a
/// zero count, and so does this.
fn slice_at(data: &[u8], off: i32, count: u64, stride: u64) -> Result<&[u8], RangeFault> {
    let Ok(start) = u32::try_from(off) else {
        return Err(RangeFault::InvalidOffsetOrCount);
    };
    let Some(size) = count.checked_mul(stride) else {
        return Err(RangeFault::InvalidOffsetOrCount);
    };
    let start_usize = usize::try_from(start).unwrap_or(usize::MAX);
    let outside = RangeFault::OutsideFile {
        offset: start,
        size,
    };
    if start_usize > data.len() {
        return Err(outside);
    }
    let size_usize = usize::try_from(size).map_err(|_| outside)?;
    if size_usize > data.len() - start_usize {
        return Err(outside);
    }
    data.get(start_usize..start_usize + size_usize)
        .ok_or(outside)
}

/// The NUL-terminated string at `off`, searching at most `max` bytes.
fn string_at(data: &[u8], off: i32, max: u32) -> Result<Vec<u8>, RangeFault> {
    let start = match u32::try_from(off) {
        Ok(s) if usize::try_from(s).is_ok_and(|s| s < data.len()) => s,
        _ => {
            // Negative offsets print as their unsigned form, as Go's `%x` does not;
            // the value is only for the message.
            return Err(RangeFault::StringOutsideFile {
                offset: u32::try_from(off).unwrap_or(0),
            });
        }
    };
    let tail = data
        .get(usize::try_from(start).unwrap_or(usize::MAX)..)
        .unwrap_or(&[]);
    let cap = usize::try_from(max).unwrap_or(usize::MAX);
    let tail = tail.get(..cap).unwrap_or(tail);
    match tail.iter().position(|b| *b == 0) {
        Some(n) => Ok(tail.get(..n).unwrap_or(&[]).to_vec()),
        None => Err(RangeFault::UnterminatedString { offset: start }),
    }
}

/// One sibling chain being walked: the objects at one depth under one parent.
struct Level {
    parent: i32,
    depth: u32,
    /// The next sibling's file offset; 0 ends the chain (once `started`).
    next_off: u32,
    started: bool,
    /// Index of the first object of the chain, -1 until one is parsed.
    first: i32,
    /// Index of the last object parsed in this chain, -1 before the first.
    previous: i32,
    /// The parent object's file offset, which stays active while this chain runs.
    owner_off: u32,
}

struct Parser<'a> {
    data: &'a [u8],
    limits: ThreeDoLimits,
    objects: Vec<ThreeDoObject>,
    active: HashSet<u32>,
    seen: HashSet<u32>,
    total_vertices: u32,
    total_primitives: u32,
    total_indices: u64,
}

impl Parser<'_> {
    /// Parses the object record at `off`. Returns its index, its sibling offset and
    /// its child offset (0 for none).
    fn parse_object(
        &mut self,
        off: u32,
        parent: i32,
        depth: u32,
    ) -> Result<(i32, u32, u32), ThreeDoError> {
        let data = self.data;
        let limits = self.limits;
        if off == 0 && !self.objects.is_empty() {
            return Err(ThreeDoError::ObjectOffsetReused);
        }
        if depth > limits.max_depth {
            return Err(ThreeDoError::DepthExceeded);
        }
        let base = usize::try_from(off).unwrap_or(usize::MAX);
        if base.checked_add(52).is_none_or(|end| end > data.len()) {
            return Err(ThreeDoError::ObjectOutsideFile { offset: off });
        }
        if self.active.contains(&off) {
            return Err(ThreeDoError::ObjectCycle { offset: off });
        }
        if self.seen.contains(&off) {
            return Err(ThreeDoError::SharedObject { offset: off });
        }
        if u32::try_from(self.objects.len()).map_or(true, |n| n >= limits.max_objects) {
            return Err(ThreeDoError::ObjectCountExceeded);
        }
        self.active.insert(off);
        self.seen.insert(off);
        let index = i32::try_from(self.objects.len()).unwrap_or(i32::MAX);
        let outside = ThreeDoError::ObjectOutsideFile { offset: off };
        let read = |n: usize| i32_at(data, base + n).ok_or(outside.clone());
        let version = read(0)?;
        let vertex_count = read(4)?;
        let primitive_count = read(8)?;
        let selection = read(12)?;
        let translation = [read(16)?, read(20)?, read(24)?];
        let always_zero = read(32)?;
        let (name_offset, vertex_offset, primitive_offset) = (read(28)?, read(36)?, read(40)?);
        let (sibling_offset, child_offset) = (read(44)?, read(48)?);
        if version != 1 {
            return Err(ThreeDoError::BadVersion {
                offset: off,
                version,
            });
        }
        let (Ok(vertex_n), Ok(primitive_n)) =
            (u32::try_from(vertex_count), u32::try_from(primitive_count))
        else {
            return Err(ThreeDoError::NegativeCount { offset: off });
        };
        if u64::from(self.total_vertices) + u64::from(vertex_n) > u64::from(limits.max_vertices)
            || u64::from(self.total_primitives) + u64::from(primitive_n)
                > u64::from(limits.max_primitives)
        {
            return Err(ThreeDoError::GeometryCountExceeded);
        }
        let name = string_at(data, name_offset, limits.max_string_bytes)
            .map_err(|fault| ThreeDoError::ObjectName { offset: off, fault })?;
        let vertex_bytes = slice_at(data, vertex_offset, u64::from(vertex_n), 12)
            .map_err(|fault| ThreeDoError::ObjectVertices { offset: off, fault })?;
        // The slice check above ties the count to the file size before this reserve.
        let mut vertices = Vec::with_capacity(vertex_bytes.len() / 12);
        for b in vertex_bytes.as_chunks::<12>().0 {
            if let (Some(x), Some(y), Some(z)) = (i32_at(b, 0), i32_at(b, 4), i32_at(b, 8)) {
                vertices.push(ThreeDoVertex { x, y, z });
            }
        }
        self.total_vertices += vertex_n;
        let primitive_bytes = slice_at(data, primitive_offset, u64::from(primitive_n), 32)
            .map_err(|fault| ThreeDoError::ObjectPrimitives { offset: off, fault })?;
        let mut primitives = Vec::with_capacity(primitive_bytes.len() / 32);
        for (n, b) in primitive_bytes.as_chunks::<32>().0.iter().enumerate() {
            let word = |at: usize| i32_at(b, at).unwrap_or(0);
            let count = word(4);
            let polygon_vertices = match u32::try_from(count) {
                Ok(c) if c <= limits.max_polygon_vertices => c,
                _ => {
                    return Err(ThreeDoError::InvalidPolygonVertexCount {
                        primitive: n,
                        offset: off,
                        count,
                    })
                }
            };
            let mut indices: Vec<u16> = Vec::new();
            if polygon_vertices > 0 {
                let want = u64::from(polygon_vertices);
                if want > limits.max_indices.saturating_sub(self.total_indices) {
                    return Err(ThreeDoError::AggregateIndexesExceeded);
                }
                let index_bytes = slice_at(data, word(12), want, 2).map_err(|fault| {
                    ThreeDoError::PrimitiveIndexes {
                        primitive: n,
                        fault,
                    }
                })?;
                indices.reserve_exact(index_bytes.len() / 2);
                for j in 0..index_bytes.len() / 2 {
                    let value = u16_at(index_bytes, j * 2).unwrap_or(0);
                    if u32::from(value) >= vertex_n {
                        return Err(ThreeDoError::VertexIndexOutOfRange {
                            primitive: n,
                            index: value,
                            vertex_count,
                        });
                    }
                    indices.push(value);
                }
                self.total_indices += want;
            }
            let texture_offset = word(16);
            let texture_name = if texture_offset != 0 {
                string_at(data, texture_offset, limits.max_string_bytes).map_err(|fault| {
                    ThreeDoError::PrimitiveTexture {
                        primitive: n,
                        fault,
                    }
                })?
            } else {
                Vec::new()
            };
            primitives.push(ThreeDoPrimitive {
                color_index: u32::from_le_bytes([
                    b.first().copied().unwrap_or(0),
                    b.get(1).copied().unwrap_or(0),
                    b.get(2).copied().unwrap_or(0),
                    b.get(3).copied().unwrap_or(0),
                ]),
                vertex_indices: indices,
                texture_name,
                always_zero: word(8),
                unknown1: word(20),
                unknown2: word(24),
                is_colored: word(28),
                // Go adds in `uint32`, wrapping; the table is inside the file, so
                // this cannot wrap for a file under 4 GiB.
                source_offset: (primitive_offset as u32).wrapping_add((n as u32).wrapping_mul(32)),
            });
        }
        self.total_primitives += primitive_n;
        self.objects.push(ThreeDoObject {
            source_offset: off,
            version,
            always_zero,
            selection,
            translation,
            name,
            vertices,
            primitives,
            parent,
            first_child: -1,
            next_sibling: -1,
        });
        // Offsets are stored as signed words; Go converts them with `uint32(...)`.
        Ok((index, sibling_offset as u32, child_offset as u32))
    }
}

impl ThreeDo {
    /// Decodes a model under the default limits.
    pub fn parse(data: &[u8]) -> Result<ThreeDo, ThreeDoError> {
        ThreeDo::parse_with_limits(data, ThreeDoLimits::default())
    }

    /// Decodes a model under explicit bounds. The input is copied into
    /// [`ThreeDo::raw`]; nothing larger than the input is reserved before the counts
    /// it declares have been checked against it.
    pub fn parse_with_limits(data: &[u8], limits: ThreeDoLimits) -> Result<ThreeDo, ThreeDoError> {
        if data.len() < 52 {
            return Err(ThreeDoError::RootTruncated);
        }
        if limits.max_depth == 0
            || limits.max_objects == 0
            || limits.max_vertices == 0
            || limits.max_primitives == 0
            || limits.max_polygon_vertices == 0
            || limits.max_indices == 0
            || limits.max_string_bytes == 0
        {
            return Err(ThreeDoError::InvalidLimits);
        }
        let raw = data.to_vec();
        let content_hash = sha256(&raw);
        let mut parser = Parser {
            data: &raw,
            limits,
            objects: Vec::new(),
            active: HashSet::new(),
            seen: HashSet::new(),
            total_vertices: 0,
            total_primitives: 0,
            total_indices: 0,
        };
        // Go recurses: a piece, its child chain, then its next sibling. This is the
        // same walk with the call stack kept in `levels`. The root is the first
        // piece of a depth-zero chain whose later pieces are the root's siblings.
        let mut levels = vec![Level {
            parent: -1,
            depth: 0,
            next_off: 0,
            started: false,
            first: -1,
            previous: -1,
            owner_off: 0,
        }];
        let mut root = -1;
        while let Some(top) = levels.len().checked_sub(1) {
            let (finished, off, parent, depth) = match levels.get(top) {
                Some(l) => (l.started && l.next_off == 0, l.next_off, l.parent, l.depth),
                None => break,
            };
            if finished {
                let Some(done) = levels.pop() else { break };
                if done.parent >= 0 {
                    let owner = usize::try_from(done.parent).unwrap_or(usize::MAX);
                    if let Some(o) = parser.objects.get_mut(owner) {
                        o.first_child = done.first;
                    }
                    parser.active.remove(&done.owner_off);
                } else {
                    root = done.first;
                }
                continue;
            }
            let (index, sibling, child) = parser.parse_object(off, parent, depth)?;
            if let Some(level) = levels.get_mut(top) {
                if level.previous >= 0 {
                    let prev = usize::try_from(level.previous).unwrap_or(usize::MAX);
                    if let Some(p) = parser.objects.get_mut(prev) {
                        p.next_sibling = index;
                    }
                } else {
                    level.first = index;
                }
                level.previous = index;
                level.next_off = sibling;
                level.started = true;
            }
            if child != 0 {
                levels.push(Level {
                    parent: index,
                    depth: depth + 1,
                    next_off: child,
                    started: false,
                    first: -1,
                    previous: -1,
                    owner_off: off,
                });
            } else {
                parser.active.remove(&off);
            }
        }
        let objects = parser.objects;
        Ok(ThreeDo {
            content_hash,
            raw,
            root,
            objects,
        })
    }

    /// The model's top extent in 16.16 world units, as the retail loader helper
    /// computes it for the unit definition.
    ///
    /// The walk covers a piece and its siblings, taking the maximum of
    /// `vertex.y + piece.translation.y` over every vertex and, for each child
    /// subtree, the recursive result plus this piece's own Y translation. The
    /// accumulator starts at zero and only grows, so the result is floored at zero:
    /// a model entirely below its origin reports 0.
    ///
    /// There is deliberately no minimum-Y counterpart.
    ///
    /// The sums wrap at 32 bits, as Go's `int32` arithmetic does. Each piece is
    /// visited at most once, so a hand-built model with looping links still ends.
    pub fn model_top(&self) -> i32 {
        if self.objects.is_empty() {
            return 0;
        }
        struct Frame {
            /// The next piece of this sibling chain, as an index (negative: done).
            index: i32,
            top: i32,
            /// The translation to add to this chain's result in the parent.
            parent_ty: i32,
        }
        let mut visited = vec![false; self.objects.len()];
        let mut stack = vec![Frame {
            index: self.root,
            top: 0, // the model-top accumulator starts at zero
            parent_ty: 0,
        }];
        loop {
            let Some(frame) = stack.last_mut() else {
                return 0;
            };
            let next = usize::try_from(frame.index)
                .ok()
                .and_then(|i| Some((i, self.objects.get(i)?)))
                .filter(|(i, _)| !visited.get(*i).copied().unwrap_or(true));
            let Some((i, obj)) = next else {
                let Some(done) = stack.pop() else { return 0 };
                match stack.last_mut() {
                    Some(parent) => {
                        let y = done.top.wrapping_add(done.parent_ty);
                        if y > parent.top {
                            parent.top = y;
                        }
                    }
                    None => return done.top,
                }
                continue;
            };
            if let Some(v) = visited.get_mut(i) {
                *v = true;
            }
            let ty = obj.translation[1];
            for v in &obj.vertices {
                let y = v.y.wrapping_add(ty);
                if y > frame.top {
                    frame.top = y;
                }
            }
            frame.index = obj.next_sibling;
            if obj.first_child >= 0 {
                stack.push(Frame {
                    index: obj.first_child,
                    top: 0,
                    parent_ty: ty,
                });
            }
        }
    }
}
