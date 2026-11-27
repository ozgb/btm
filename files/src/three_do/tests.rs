// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), formats/three_do_bounds_test.go, formats/three_do_model_top_test.go, formats/three_do_write_test.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! Tests. Fixtures are authored here; no game data is used. Tests are allowed to
//! index and unwrap. Added beyond the Go tests: cycle, sharing, depth, offset reuse,
//! count and allocation bounds, a hostile-input sweep, and the deep-chain case that
//! the Go recursion could not take.

#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use super::write::encode_three_do;
use super::*;

fn f(v: f64) -> i32 {
    (v * 65536.0) as i32
}

/// A piece with no links; callers set the topology they need.
fn piece(name: &str) -> ThreeDoObject {
    ThreeDoObject {
        version: 1,
        name: name.as_bytes().to_vec(),
        selection: -1,
        parent: -1,
        first_child: -1,
        next_sibling: -1,
        ..ThreeDoObject::default()
    }
}

fn model(objects: Vec<ThreeDoObject>) -> ThreeDo {
    ThreeDo {
        root: 0,
        objects,
        ..ThreeDo::default()
    }
}

fn v(x: i32, y: i32, z: i32) -> ThreeDoVertex {
    ThreeDoVertex { x, y, z }
}

fn put32(data: &mut [u8], at: usize, value: u32) {
    data[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn get32(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(data[at..at + 4].try_into().unwrap())
}

// ---- three_do_bounds_test.go ----

#[test]
fn aggregate_index_budget() {
    let m = model(vec![ThreeDoObject {
        vertices: vec![v(0, 0, 0)],
        primitives: vec![
            ThreeDoPrimitive {
                vertex_indices: vec![0],
                ..ThreeDoPrimitive::default()
            },
            ThreeDoPrimitive {
                vertex_indices: vec![0],
                ..ThreeDoPrimitive::default()
            },
        ],
        ..piece("root")
    }]);
    let data = encode_three_do(&m).unwrap();
    let limits = ThreeDoLimits {
        max_indices: 1,
        ..ThreeDoLimits::default()
    };
    let err = ThreeDo::parse_with_limits(&data, limits).unwrap_err();
    assert_eq!(err, ThreeDoError::AggregateIndexesExceeded);
    assert!(err.to_string().contains("aggregate polygon indexes"));
}

#[test]
fn long_sibling_chain_uses_bounded_call_depth() {
    const SIBLINGS: usize = 4096;
    let mut objects = vec![ThreeDoObject {
        first_child: 1,
        ..piece("root")
    }];
    for i in 1..=SIBLINGS {
        objects.push(ThreeDoObject {
            parent: 0,
            next_sibling: if i == SIBLINGS { -1 } else { i as i32 + 1 },
            ..piece("piece")
        });
    }
    let data = encode_three_do(&model(objects)).unwrap();
    let limits = ThreeDoLimits {
        max_depth: 1,
        max_objects: SIBLINGS as u32 + 1,
        ..ThreeDoLimits::default()
    };
    let got = ThreeDo::parse_with_limits(&data, limits).unwrap();
    assert_eq!(got.objects.len(), SIBLINGS + 1);
    assert_eq!(got.objects[1].parent, 0);
    assert_eq!(got.objects[SIBLINGS].next_sibling, -1);
}

// ---- three_do_model_top_test.go ----

/// An authored four-piece model where every clause of the model-top walk matters:
/// the deepest vertex is not the highest, the winner sits two levels down and only
/// wins once both ancestors' translations are added, and it is reached through a
/// sibling link rather than a first child.
///
/// ```text
/// base   translation +2   vertices up to +2
///   mid    translation +5   vertices up to +1
///     tip    translation +4   vertices up to +3
///   arm    translation +1   vertices up to +20   (sibling of mid)
/// ```
fn model_top_fixture() -> ThreeDo {
    model(vec![
        ThreeDoObject {
            translation: [0, f(2.0), 0],
            first_child: 1,
            vertices: vec![v(0, 0, 0), v(f(1.0), f(2.0), 0)],
            ..piece("base")
        },
        ThreeDoObject {
            translation: [0, f(5.0), 0],
            first_child: 2,
            next_sibling: 3,
            parent: 0,
            vertices: vec![v(0, 0, 0), v(f(1.0), f(1.0), 0)],
            ..piece("mid")
        },
        ThreeDoObject {
            translation: [0, f(4.0), 0],
            parent: 1,
            vertices: vec![v(0, 0, 0), v(f(1.0), f(3.0), 0)],
            ..piece("tip")
        },
        ThreeDoObject {
            translation: [0, f(1.0), 0],
            parent: 0,
            vertices: vec![v(0, 0, 0), v(f(1.0), f(20.0), 0)],
            ..piece("arm")
        },
    ])
}

/// The candidates are base 2+2 = 4, mid 1+5+2 = 8, tip 3+4+5+2 = 14 and arm
/// 20+1+2 = 23, so the walk must return 23 world units.
#[test]
fn model_top_walk_accumulates_ancestor_translations() {
    const WANT: i32 = 23 << 16;
    assert_eq!(model_top_fixture().model_top(), WANT);
    // The same answer must survive the byte layout.
    let data = encode_three_do(&model_top_fixture()).unwrap();
    let loaded = ThreeDo::parse(&data).unwrap();
    assert_eq!(loaded.model_top(), WANT);
}

/// The accumulator starts at zero, so a model entirely below its origin reports 0.
#[test]
fn model_top_walk_is_floored_at_zero() {
    let below = model(vec![
        ThreeDoObject {
            translation: [0, f(-3.0), 0],
            first_child: 1,
            vertices: vec![v(0, f(-1.0), 0), v(f(1.0), f(-8.0), 0)],
            ..piece("sunk")
        },
        ThreeDoObject {
            translation: [0, f(-4.0), 0],
            parent: 0,
            vertices: vec![v(0, f(-2.0), 0)],
            ..piece("deeper")
        },
    ]);
    assert_eq!(below.model_top(), 0);
    assert_eq!(ThreeDo::default().model_top(), 0);
}

#[test]
fn model_top_terminates_on_looping_links() {
    // Not reachable through `parse`, but the fields are public.
    let mut m = model_top_fixture();
    m.objects[2].first_child = 0; // tip's child is the root
    m.objects[3].next_sibling = 1; // arm points back at mid
    let _ = m.model_top();
    m.root = 99;
    assert_eq!(m.model_top(), 0);
}

// ---- three_do_write_test.go ----

/// A hand-built model: a root with a ground plate and a textured quad, one child
/// holding a flat triangle, and a sibling emit point.
fn authored() -> ThreeDo {
    model(vec![
        ThreeDoObject {
            selection: 0,
            first_child: 1,
            vertices: vec![
                v(f(-4.0), 0, f(-4.0)),
                v(f(4.0), 0, f(-4.0)),
                v(f(4.0), 0, f(4.0)),
                v(f(-4.0), 0, f(4.0)),
                v(f(-2.0), f(1.0), f(-2.0)),
                v(f(2.0), f(1.0), f(-2.0)),
                v(f(2.0), f(3.0), f(-2.0)),
                v(f(-2.0), f(3.0), f(-2.0)),
            ],
            primitives: vec![
                ThreeDoPrimitive {
                    vertex_indices: vec![0, 3, 2, 1],
                    is_colored: 1,
                    ..ThreeDoPrimitive::default()
                },
                ThreeDoPrimitive {
                    color_index: 0x0344_E9FF,
                    vertex_indices: vec![4, 5, 6, 7],
                    texture_name: b"rm_plate".to_vec(),
                    unknown1: 7,
                    ..ThreeDoPrimitive::default()
                },
            ],
            ..piece("base")
        },
        ThreeDoObject {
            translation: [0, f(3.0), 0],
            next_sibling: 2,
            parent: 0,
            vertices: vec![v(0, 0, 0), v(f(1.0), 0, 0), v(0, f(1.0), 0)],
            primitives: vec![ThreeDoPrimitive {
                color_index: 69,
                vertex_indices: vec![0, 1, 2],
                is_colored: 1,
                ..ThreeDoPrimitive::default()
            }],
            ..piece("turret")
        },
        ThreeDoObject {
            translation: [0, f(4.0), f(-1.0)],
            parent: 0,
            vertices: vec![v(0, 0, 0)],
            ..piece("flare")
        },
    ])
}

#[test]
fn encode_round_trip() {
    let want = authored();
    let data = encode_three_do(&want).unwrap();
    let got = ThreeDo::parse(&data).unwrap();
    assert_eq!(got.root, 0);
    assert_eq!(got.objects.len(), want.objects.len());
    for (w, g) in want.objects.iter().zip(&got.objects) {
        assert_eq!(
            (
                &g.name,
                g.translation,
                g.selection,
                g.parent,
                g.first_child,
                g.next_sibling
            ),
            (
                &w.name,
                w.translation,
                w.selection,
                w.parent,
                w.first_child,
                w.next_sibling
            )
        );
        assert_eq!(g.vertices, w.vertices);
        assert_eq!(g.primitives.len(), w.primitives.len());
        for (wp, gp) in w.primitives.iter().zip(&g.primitives) {
            assert_eq!(gp.color_index, wp.color_index);
            assert_eq!(gp.texture_name, wp.texture_name);
            assert_eq!(gp.is_colored, wp.is_colored);
            assert_eq!(gp.unknown1, wp.unknown1);
            assert_eq!(gp.vertex_indices, wp.vertex_indices);
        }
    }
    // The lossless parser keeps authored fields and primitive order, so a second
    // encode reproduces the bytes exactly.
    assert_eq!(encode_three_do(&got).unwrap(), data);
}

#[test]
fn encode_rejects_textured_triangle() {
    let mut m = authored();
    m.objects[1].primitives[0].texture_name = b"rm_plate".to_vec();
    assert!(
        encode_three_do(&m).is_err(),
        "textured triangle must be rejected"
    );
}

#[test]
fn load_preserves_authored_primitive_order_and_selection() {
    let mut m = authored();
    m.objects[0].selection = 1;
    let data = encode_three_do(&m).unwrap();
    let got = ThreeDo::parse(&data).unwrap();
    assert_eq!(got.raw, data);
    assert_eq!(got.content_hash, crate::digest::sha256(&data));
    let object = &got.objects[0];
    assert_eq!(object.selection, 1);
    assert_eq!(object.primitives.len(), 2);
    assert!(object.primitives[0].texture_name.is_empty());
    assert_eq!(object.primitives[1].texture_name, b"rm_plate");
    for (i, p) in object.primitives.iter().enumerate() {
        assert_ne!(
            p.source_offset, 0,
            "primitive {i} source offset was not kept"
        );
        assert_eq!(
            get32(&got.raw, p.source_offset as usize),
            p.color_index,
            "primitive {i}"
        );
    }
    assert_eq!(encode_three_do(&got).unwrap(), data);
}

// ---- added: hostile input ----

fn two_objects() -> Vec<u8> {
    let m = model(vec![
        ThreeDoObject {
            first_child: 1,
            vertices: vec![v(0, 0, 0)],
            ..piece("root")
        },
        ThreeDoObject {
            parent: 0,
            ..piece("child")
        },
    ]);
    encode_three_do(&m).unwrap()
}

#[test]
fn rejects_short_files_and_zero_limits() {
    assert_eq!(
        ThreeDo::parse(&[0; 51]).unwrap_err(),
        ThreeDoError::RootTruncated
    );
    let data = two_objects();
    let limits = ThreeDoLimits {
        max_string_bytes: 0,
        ..ThreeDoLimits::default()
    };
    assert_eq!(
        ThreeDo::parse_with_limits(&data, limits).unwrap_err(),
        ThreeDoError::InvalidLimits
    );
}

#[test]
fn child_link_to_an_ancestor_is_a_cycle() {
    // Object 1 (at 52) gets itself as a child: an active ancestor, so a cycle. (A
    // zero link ends a chain, so the offset-zero reuse check is defensive only.)
    let mut data = two_objects();
    put32(&mut data, 52 + 48, 52);
    assert_eq!(
        ThreeDo::parse(&data).unwrap_err(),
        ThreeDoError::ObjectCycle { offset: 52 }
    );
}

#[test]
fn second_reference_to_a_finished_object_is_shared() {
    let mut data = two_objects();
    // The root's sibling points at the child again, after the child was parsed.
    put32(&mut data, 44, 52);
    assert_eq!(
        ThreeDo::parse(&data).unwrap_err(),
        ThreeDoError::SharedObject { offset: 52 }
    );
}

#[test]
fn depth_limit_applies_to_children() {
    let one_deep = ThreeDoLimits {
        max_depth: 1,
        ..ThreeDoLimits::default()
    };
    // A child sits at depth 1, which max_depth = 1 allows.
    ThreeDo::parse_with_limits(&two_objects(), one_deep).unwrap();
    // A grandchild sits at depth 2, which it does not.
    let m = model(vec![
        ThreeDoObject {
            first_child: 1,
            ..piece("a")
        },
        ThreeDoObject {
            parent: 0,
            first_child: 2,
            ..piece("b")
        },
        ThreeDoObject {
            parent: 1,
            ..piece("c")
        },
    ]);
    let data = encode_three_do(&m).unwrap();
    assert_eq!(
        ThreeDo::parse_with_limits(&data, one_deep).unwrap_err(),
        ThreeDoError::DepthExceeded
    );
    ThreeDo::parse(&data).unwrap();
}

#[test]
fn object_count_limit() {
    let data = two_objects();
    let limits = ThreeDoLimits {
        max_objects: 1,
        ..ThreeDoLimits::default()
    };
    assert_eq!(
        ThreeDo::parse_with_limits(&data, limits).unwrap_err(),
        ThreeDoError::ObjectCountExceeded
    );
}

#[test]
fn declared_counts_are_checked_against_the_file_before_allocating() {
    // The root declares two billion vertices in a 52-byte file with offsets at 0.
    let mut data = vec![0u8; 60];
    put32(&mut data, 0, 1);
    put32(&mut data, 4, 0x7fff_ffff);
    put32(&mut data, 28, 52); // name offset, one NUL byte
    let limits = ThreeDoLimits {
        max_vertices: u32::MAX,
        ..ThreeDoLimits::default()
    };
    let err = ThreeDo::parse_with_limits(&data, limits).unwrap_err();
    assert!(matches!(err, ThreeDoError::ObjectVertices { .. }), "{err}");
    // And within the default limits the declared count is refused outright.
    assert_eq!(
        ThreeDo::parse(&data).unwrap_err(),
        ThreeDoError::GeometryCountExceeded
    );
    // A negative count is refused.
    put32(&mut data, 4, 0xffff_ffff);
    assert!(matches!(
        ThreeDo::parse(&data).unwrap_err(),
        ThreeDoError::NegativeCount { .. }
    ));
}

#[test]
fn bad_version_and_bad_vertex_index() {
    let mut data = two_objects();
    put32(&mut data, 0, 2);
    assert_eq!(
        ThreeDo::parse(&data).unwrap_err(),
        ThreeDoError::BadVersion {
            offset: 0,
            version: 2
        }
    );
    let m = model(vec![ThreeDoObject {
        vertices: vec![v(0, 0, 0)],
        primitives: vec![ThreeDoPrimitive {
            vertex_indices: vec![0],
            ..ThreeDoPrimitive::default()
        }],
        ..piece("root")
    }]);
    let mut data = encode_three_do(&m).unwrap();
    // The primitive table follows the index list; point its only index past the end.
    let prim = get32(&data, 40) as usize;
    let index_at = get32(&data, prim + 12) as usize;
    data[index_at] = 5;
    assert!(matches!(
        ThreeDo::parse(&data).unwrap_err(),
        ThreeDoError::VertexIndexOutOfRange { index: 5, .. }
    ));
}

#[test]
fn unterminated_string_is_an_error() {
    let mut data = two_objects();
    let last = data.len() - 1;
    data[last] = b'x'; // the final NUL of the last string
                       // The root's name is earlier and terminated; break every terminator.
    for b in data[104..].iter_mut() {
        if *b == 0 {
            *b = b'x';
        }
    }
    assert!(ThreeDo::parse(&data).is_err());
}

/// A chain far deeper than any call stack would take. The walk is iterative, and
/// `model_top` and drop are too.
#[test]
fn very_deep_chain_does_not_overflow_the_stack() {
    const DEPTH: usize = 20_000;
    let mut data = vec![0u8; DEPTH * 52 + 8];
    let name_at = DEPTH * 52;
    data[name_at] = b'n';
    for i in 0..DEPTH {
        let base = i * 52;
        put32(&mut data, base, 1);
        put32(&mut data, base + 12, u32::MAX); // selection -1
        put32(&mut data, base + 28, name_at as u32);
        if i + 1 < DEPTH {
            put32(&mut data, base + 48, ((i + 1) * 52) as u32);
        }
    }
    let limits = ThreeDoLimits {
        max_depth: u32::MAX,
        ..ThreeDoLimits::default()
    };
    let got = ThreeDo::parse_with_limits(&data, limits).unwrap();
    assert_eq!(got.objects.len(), DEPTH);
    assert_eq!(got.objects[DEPTH - 1].parent, DEPTH as i32 - 2);
    assert_eq!(got.model_top(), 0);
    // The default depth limit refuses it cleanly.
    assert_eq!(
        ThreeDo::parse(&data).unwrap_err(),
        ThreeDoError::DepthExceeded
    );
}

/// Every truncation, and a byte flip at every position, of a small model.
#[test]
fn truncations_and_flips_never_panic() {
    let data = encode_three_do(&authored()).unwrap();
    for cut in 0..data.len() {
        let _ = ThreeDo::parse(&data[..cut]);
    }
    for i in 0..data.len() {
        for flip in [0x01u8, 0x80, 0xff] {
            let mut d = data.clone();
            d[i] ^= flip;
            if let Ok(m) = ThreeDo::parse(&d) {
                let _ = m.model_top();
            }
        }
    }
}
