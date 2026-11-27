// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), formats/three_do_write.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! Test-only 3DO encoder, used to build fixtures. Writing is out of scope for the
//! crate, so this exists only under `cfg(test)`.

#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::collections::HashMap;

use super::ThreeDo;

/// Serialises a model into the 3DO byte layout: 52-byte object records, 32-byte
/// primitive records, 12-byte vertices, NUL-terminated strings, all offsets
/// absolute. Object i is written at i*52, so `objects[0]` must be the root and the
/// child/sibling links are the parsed indexes. Textured primitives must be quads.
pub(super) fn encode_three_do(model: &ThreeDo) -> Result<Vec<u8>, String> {
    if model.objects.is_empty() || model.root != 0 {
        return Err("3do encode: model must have root object zero".into());
    }
    const OBJECT: usize = 52;
    let mut data = vec![0u8; model.objects.len() * OBJECT];
    let mut strings_at: HashMap<Vec<u8>, u32> = HashMap::new();
    let mut append_string = |data: &mut Vec<u8>, value: &[u8]| -> u32 {
        if let Some(offset) = strings_at.get(value) {
            return *offset;
        }
        let offset = data.len() as u32;
        data.extend_from_slice(value);
        data.push(0);
        strings_at.insert(value.to_vec(), offset);
        offset
    };

    let mut object_name = Vec::new();
    let mut texture_name: Vec<Vec<u32>> = Vec::new();
    for (i, obj) in model.objects.iter().enumerate() {
        if obj.name.is_empty() {
            return Err(format!("3do encode: object {i} has an empty name"));
        }
        object_name.push(append_string(&mut data, &obj.name));
        let mut names = vec![0u32; obj.primitives.len()];
        for (p, primitive) in obj.primitives.iter().enumerate() {
            if primitive.texture_name.is_empty() {
                continue;
            }
            if primitive.vertex_indices.len() != 4 {
                return Err(format!(
                    "3do encode: textured primitive {}[{p}] has {} corners, want 4",
                    String::from_utf8_lossy(&obj.name),
                    primitive.vertex_indices.len()
                ));
            }
            names[p] = append_string(&mut data, &primitive.texture_name);
        }
        texture_name.push(names);
    }
    let align4 = |data: &mut Vec<u8>| {
        while data.len() & 3 != 0 {
            data.push(0);
        }
    };
    align4(&mut data);

    let mut vertex_offset = vec![0u32; model.objects.len()];
    let mut primitive_offset = vec![0u32; model.objects.len()];
    for (i, obj) in model.objects.iter().enumerate() {
        if !obj.vertices.is_empty() {
            vertex_offset[i] = data.len() as u32;
            for v in &obj.vertices {
                data.extend_from_slice(&v.x.to_le_bytes());
                data.extend_from_slice(&v.y.to_le_bytes());
                data.extend_from_slice(&v.z.to_le_bytes());
            }
        }
        let mut index_offset = vec![0u32; obj.primitives.len()];
        for (p, primitive) in obj.primitives.iter().enumerate() {
            if primitive.vertex_indices.is_empty() {
                continue;
            }
            index_offset[p] = data.len() as u32;
            for index in &primitive.vertex_indices {
                if usize::from(*index) >= obj.vertices.len() {
                    return Err(format!(
                        "3do encode: primitive {}[{p}] references vertex {index} of {}",
                        String::from_utf8_lossy(&obj.name),
                        obj.vertices.len()
                    ));
                }
                data.extend_from_slice(&index.to_le_bytes());
            }
        }
        align4(&mut data);
        if !obj.primitives.is_empty() {
            primitive_offset[i] = data.len() as u32;
            for (p, primitive) in obj.primitives.iter().enumerate() {
                data.extend_from_slice(&primitive.color_index.to_le_bytes());
                data.extend_from_slice(&(primitive.vertex_indices.len() as u32).to_le_bytes());
                data.extend_from_slice(&primitive.always_zero.to_le_bytes());
                data.extend_from_slice(&index_offset[p].to_le_bytes());
                data.extend_from_slice(&texture_name[i][p].to_le_bytes());
                data.extend_from_slice(&primitive.unknown1.to_le_bytes());
                data.extend_from_slice(&primitive.unknown2.to_le_bytes());
                data.extend_from_slice(&primitive.is_colored.to_le_bytes());
            }
        }
    }

    let put = |data: &mut Vec<u8>, object: usize, field: usize, value: u32| {
        let at = object * OBJECT + field;
        data[at..at + 4].copy_from_slice(&value.to_le_bytes());
    };
    for (i, obj) in model.objects.iter().enumerate() {
        let version = if obj.version == 0 { 1 } else { obj.version };
        put(&mut data, i, 0, version as u32);
        put(&mut data, i, 4, obj.vertices.len() as u32);
        put(&mut data, i, 8, obj.primitives.len() as u32);
        put(&mut data, i, 12, obj.selection as u32);
        put(&mut data, i, 16, obj.translation[0] as u32);
        put(&mut data, i, 20, obj.translation[1] as u32);
        put(&mut data, i, 24, obj.translation[2] as u32);
        put(&mut data, i, 28, object_name[i]);
        put(&mut data, i, 32, obj.always_zero as u32);
        put(&mut data, i, 36, vertex_offset[i]);
        put(&mut data, i, 40, primitive_offset[i]);
        if obj.next_sibling > 0 {
            if obj.next_sibling as usize >= model.objects.len() {
                return Err(format!(
                    "3do encode: object {} sibling {} outside table",
                    String::from_utf8_lossy(&obj.name),
                    obj.next_sibling
                ));
            }
            put(&mut data, i, 44, obj.next_sibling as u32 * OBJECT as u32);
        }
        if obj.first_child > 0 {
            if obj.first_child as usize >= model.objects.len() {
                return Err(format!(
                    "3do encode: object {} child {} outside table",
                    String::from_utf8_lossy(&obj.name),
                    obj.first_child
                ));
            }
            put(&mut data, i, 48, obj.first_child as u32 * OBJECT as u32);
        }
    }
    Ok(data)
}
