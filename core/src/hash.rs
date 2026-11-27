//! The `hash` group of the core gate: the rows (`phase`, `world`, `tick`) of a test-mode dump.
//!
//! A port of the walk of docs/trace-format.md (sections 3 to 8, 10, 11) over the lines of a dump.

use crate::Call;
use std::collections::HashMap;

const OFFSET_BASIS: u64 = 0xCBF2_9CE4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01B3;
const END: u32 = 0xFFFF_FFFF;
const PHASES: [&str; 9] = ["net", "units", "projectiles", "explosions", "players", "features", "wind", "meteors", "tick"];
const DUMPED_ONLY: [&str; 30] = [
    "results.", "results[", "economy.", "economy[", "features.", "features[", "packets.", "packets[",
    "windMeteors.", "windMeteors[", "targeting.", "targeting[", "los_radar.", "los_radar[", "map.", "map[",
    "playersEvents.", "playersEvents[", "aiEvents.", "aiEvents[", "weaponsEvents.", "weaponsEvents[",
    "ordersEvents.", "ordersEvents[", "movementEvents.", "movementEvents[",
    "unitsEvents.", "unitsEvents[", "scriptsEvents.", "scriptsEvents[",
];
const REPORTS: [&str; 8] = [
    "economyState.", "aiState.", "mapState.", "playersState.", "weaponsState.", "ordersState.", "movementState.",
    "scriptsState.",
];

type R<T> = Result<T, ()>;

// ---------------------------------------------------------------------------------------------- hasher

struct Hasher {
    h: u64,
}

impl Hasher {
    fn new() -> Hasher {
        Hasher { h: OFFSET_BASIS }
    }
    fn reset(&mut self) {
        self.h = OFFSET_BASIS;
    }
    fn run(&mut self, data: &[u8]) {
        let mut h = self.h;
        let mut chunks = data.chunks_exact(8);
        for c in &mut chunks {
            let w = u64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]);
            h = (h ^ w).wrapping_mul(PRIME);
        }
        for &b in chunks.remainder() {
            h = (h ^ b as u64).wrapping_mul(PRIME);
        }
        self.h = h;
    }
    fn h8(&mut self, x: u64) {
        self.run(&[(x & 0xff) as u8]);
    }
    fn h32(&mut self, x: u64) {
        let x = x as u32;
        self.run(&x.to_le_bytes());
    }
    fn end(&mut self) {
        self.h32(END as u64);
    }
    fn pointer(&mut self, tag: u64, index: u64) {
        self.h8(tag);
        self.h32(index);
    }
}

// ---------------------------------------------------------------------------------------------- values

fn hex_digit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

fn strip_0x(v: &[u8]) -> &[u8] {
    if v.len() >= 2 && v[0] == b'0' && (v[1] == b'x' || v[1] == b'X') {
        &v[2..]
    } else {
        v
    }
}

/// The number of a hex value (`0x...`), as Python's int(value, 16).
fn hexval(v: &[u8]) -> R<u128> {
    let d = strip_0x(v);
    if d.is_empty() {
        return Err(());
    }
    let mut x: u128 = 0;
    for &c in d {
        let n = hex_digit(c).ok_or(())? as u128;
        if x >> 124 != 0 {
            return Err(());
        }
        x = (x << 4) | n;
    }
    Ok(x)
}

/// The bytes of `hex:` digits (lowest address first).
fn hex_bytes(d: &[u8]) -> R<Vec<u8>> {
    if d.len() % 2 != 0 {
        return Err(());
    }
    let mut out = Vec::with_capacity(d.len() / 2);
    for p in d.chunks_exact(2) {
        out.push((hex_digit(p[0]).ok_or(())? << 4) | hex_digit(p[1]).ok_or(())?);
    }
    Ok(out)
}

/// The `width` little-endian bytes of the number `0x...` (an error if it does not fit).
fn num_bytes(v: &[u8], width: usize) -> R<Vec<u8>> {
    let d = strip_0x(v);
    if d.is_empty() {
        return Err(());
    }
    let mut be: Vec<u8> = Vec::with_capacity(d.len() / 2 + 1);
    let mut it = d.iter().rev();
    // least significant digit first
    loop {
        let lo = match it.next() {
            Some(&c) => hex_digit(c).ok_or(())?,
            None => break,
        };
        let hi = match it.next() {
            Some(&c) => hex_digit(c).ok_or(())?,
            None => 0,
        };
        be.push((hi << 4) | lo);
    }
    // be is little-endian now
    while be.len() > width {
        if be.pop() != Some(0) {
            return Err(());
        }
    }
    be.resize(width, 0);
    Ok(be)
}

fn to_idx(d: &[u8]) -> Option<u64> {
    if d.is_empty() || d.len() > 18 {
        return None;
    }
    let mut x = 0u64;
    for &c in d {
        if !c.is_ascii_digit() {
            return None;
        }
        x = x * 10 + (c - b'0') as u64;
    }
    Some(x)
}

// ---------------------------------------------------------------------------------------------- schema

struct Field {
    name: String,
    offset: u64,
    size: usize,
    ptr: bool,
    mask: Option<u64>,
}

enum Plan {
    Single(usize),
    Bytes(Vec<usize>, usize),
}

struct Table {
    fields: Vec<Field>,
    plan: Vec<Plan>,
}

struct Schema {
    tags: HashMap<String, u64>,
    tables: HashMap<String, Table>,
    wind: Vec<Field>,
    game_scalars: Vec<Field>,
    g_meteors: Vec<Field>,
    g_game: Vec<Field>,
    g_pathfinder: Vec<Field>,
    trace_format: i64,
}

fn field_name(name: &str, offset: u64) -> String {
    if name.is_empty() {
        format!("+0x{:x}", offset)
    } else {
        name.to_string()
    }
}

fn parse_field(v: &serde_json::Value) -> R<Field> {
    let offset = v.get("offset").and_then(|x| x.as_u64()).unwrap_or(0);
    let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("");
    let size = v.get("size").and_then(|x| x.as_u64()).ok_or(())? as usize;
    let kind = v.get("kind").and_then(|x| x.as_str()).unwrap_or("");
    Ok(Field {
        name: field_name(name, offset),
        offset,
        size,
        ptr: kind == "pointer" || kind == "ref",
        mask: v.get("mask").and_then(|x| x.as_u64()),
    })
}

fn parse_fields(v: Option<&serde_json::Value>) -> R<Vec<Field>> {
    v.and_then(|x| x.as_array()).ok_or(())?.iter().map(parse_field).collect()
}

impl Schema {
    fn load(text: &[u8]) -> R<Schema> {
        let d: serde_json::Value = serde_json::from_slice(text).map_err(|_| ())?;
        let mut tags = HashMap::new();
        for t in d.get("targets").and_then(|x| x.as_array()).ok_or(())? {
            tags.insert(
                t.get("name").and_then(|x| x.as_str()).ok_or(())?.to_string(),
                t.get("tag").and_then(|x| x.as_u64()).ok_or(())?,
            );
        }
        let mut tables = HashMap::new();
        for (name, t) in d.get("tables").and_then(|x| x.as_object()).ok_or(())? {
            let fields = parse_fields(t.get("fields"))?;
            let mut by_offset: HashMap<u64, usize> = HashMap::new();
            for (i, f) in fields.iter().enumerate() {
                by_offset.insert(f.offset, i);
            }
            let mut plan = Vec::new();
            for e in t.get("plan").and_then(|x| x.as_array()).ok_or(())? {
                let offset = e.get("offset").and_then(|x| x.as_u64()).ok_or(())?;
                let size = e.get("size").and_then(|x| x.as_u64()).ok_or(())? as usize;
                let kind = e.get("kind").and_then(|x| x.as_str()).unwrap_or("");
                if kind == "pointer" || kind == "ref" || e.get("mask").is_some() {
                    plan.push(Plan::Single(*by_offset.get(&offset).ok_or(())?));
                } else {
                    let mut idx: Vec<usize> = (0..fields.len())
                        .filter(|&i| {
                            let f = &fields[i];
                            f.offset >= offset && f.offset + f.size as u64 <= offset + size as u64 && !f.ptr && f.mask.is_none()
                        })
                        .collect();
                    idx.sort_by_key(|&i| fields[i].offset);
                    plan.push(Plan::Bytes(idx, size));
                }
            }
            tables.insert(name.clone(), Table { fields, plan });
        }
        let g = d.get("globals").ok_or(())?;
        Ok(Schema {
            tags,
            tables,
            wind: parse_fields(d.get("wind"))?,
            game_scalars: parse_fields(d.get("game_scalars"))?,
            g_meteors: parse_fields(g.get("meteors"))?,
            g_game: parse_fields(g.get("game"))?,
            g_pathfinder: parse_fields(g.get("pathfinder"))?,
            trace_format: d.get("trace_format").and_then(|x| x.as_i64()).ok_or(())?,
        })
    }

    fn table(&self, name: &str) -> R<&Table> {
        self.tables.get(name).ok_or(())
    }
}

// ---------------------------------------------------------------------------------------------- walker

type Line<'a> = (&'a [u8], &'a [u8]);

struct Walker<'a> {
    s: &'a Schema,
    lines: &'a [Line<'a>],
    pos: usize,
    hash: Hasher,
    live: Vec<u64>,
    scripted: std::collections::HashSet<u64>,
    buf: Vec<u8>,
}

fn path_is(path: &[u8], parts: &[&str]) -> bool {
    let mut p = path;
    for part in parts {
        let b = part.as_bytes();
        if p.len() < b.len() || &p[..b.len()] != b {
            return false;
        }
        p = &p[b.len()..];
    }
    p.is_empty()
}

/// `prefix digits suffix` as a full match: the digits' number.
fn match_index(path: &[u8], prefix: &str, suffix: &str) -> Option<u64> {
    let (p, s) = (prefix.as_bytes(), suffix.as_bytes());
    if path.len() < p.len() + s.len() + 1 || &path[..p.len()] != p || &path[path.len() - s.len()..] != s {
        return None;
    }
    to_idx(&path[p.len()..path.len() - s.len()])
}

fn starts_any(path: &[u8], prefixes: &[&str]) -> bool {
    prefixes.iter().any(|p| path.starts_with(p.as_bytes()))
}

impl<'a> Walker<'a> {
    fn new(s: &'a Schema, lines: &'a [Line<'a>]) -> Walker<'a> {
        Walker { s, lines, pos: 0, hash: Hasher::new(), live: Vec::new(), scripted: Default::default(), buf: Vec::new() }
    }

    fn peek(&self) -> Option<&'a [u8]> {
        self.lines.get(self.pos).map(|l| l.0)
    }

    fn peek_is(&self, parts: &[&str]) -> bool {
        match self.peek() {
            Some(p) => path_is(p, parts),
            None => false,
        }
    }

    fn peek_starts(&self, prefix: &str) -> bool {
        self.peek().map_or(false, |p| p.starts_with(prefix.as_bytes()))
    }

    fn skip_dumped(&mut self, prefixes: &[&str]) {
        while self.peek().map_or(false, |p| starts_any(p, prefixes)) {
            self.pos += 1;
        }
    }

    fn take(&mut self, parts: &[&str]) -> R<&'a [u8]> {
        let l = self.lines.get(self.pos).ok_or(())?;
        if !path_is(l.0, parts) {
            return Err(());
        }
        self.pos += 1;
        Ok(l.1)
    }

    fn take_if(&mut self, parts: &[&str]) -> Option<&'a [u8]> {
        if self.peek_is(parts) {
            self.take(parts).ok()
        } else {
            None
        }
    }

    fn next_index(&self, prefix: &str, suffix: &str) -> Option<u64> {
        match_index(self.peek()?, prefix, suffix)
    }

    fn take_hex(&mut self, parts: &[&str]) -> R<u128> {
        hexval(self.take(parts)?)
    }

    fn pointer_id(&self, v: &[u8]) -> R<(u64, u64)> {
        if v == b"null" {
            return Ok((0, 0));
        }
        if v == b"heap" {
            return Ok((1, 0));
        }
        if v == b"rec:overflow" {
            return Ok((9, END as u64));
        }
        let c = v.iter().position(|&b| b == b':').ok_or(())?;
        let name = std::str::from_utf8(&v[..c]).map_err(|_| ())?;
        let tag = *self.s.tags.get(name).ok_or(())?;
        let idx = to_idx(&v[c + 1..]).ok_or(())?;
        Ok((tag, idx))
    }

    fn hash_field(&mut self, f: &Field, v: &[u8]) -> R<()> {
        if f.ptr {
            let (t, i) = self.pointer_id(v)?;
            self.hash.pointer(t, i);
        } else if let Some(m) = f.mask {
            let x = hexval(v)? as u64;
            self.hash.h32(x & m);
        } else {
            let b = value_bytes(f, v)?;
            self.hash.run(&b);
        }
        Ok(())
    }

    fn flat_list(&mut self, fields: &[Field], prefix: &str) -> R<()> {
        for f in fields {
            let v = self.take(&[prefix, &f.name])?;
            self.hash_field(f, v)?;
        }
        Ok(())
    }

    fn read_table(&mut self, table: &'a Table, prefix: &str) -> R<Vec<&'a [u8]>> {
        let mut vals = Vec::with_capacity(table.fields.len());
        for f in &table.fields {
            vals.push(self.take(&[prefix, ".", &f.name])?);
        }
        Ok(vals)
    }

    fn table(&mut self, name: &str, prefix: &str) -> R<Vec<&'a [u8]>> {
        let t = self.s.table(name)?;
        let vals = self.read_table(t, prefix)?;
        self.hash_table(t, &vals)?;
        Ok(vals)
    }

    fn hash_table(&mut self, t: &Table, vals: &[&[u8]]) -> R<()> {
        for e in &t.plan {
            match e {
                Plan::Single(i) => self.hash_field(&t.fields[*i], vals[*i])?,
                Plan::Bytes(idx, size) => {
                    let mut raw = std::mem::take(&mut self.buf);
                    raw.clear();
                    for &i in idx {
                        let f = &t.fields[i];
                        append_value_bytes(&mut raw, f, vals[i])?;
                    }
                    if raw.len() != *size {
                        return Err(());
                    }
                    self.hash.run(&raw);
                    self.buf = raw;
                }
            }
        }
        Ok(())
    }

    fn hex_run(&mut self, parts: &[&str]) -> R<Vec<u8>> {
        let v = self.take(parts)?;
        if !v.starts_with(b"hex:") {
            return Err(());
        }
        hex_bytes(&v[4..])
    }

    fn collect_run(&mut self, prefix: &str, suffix: &str, width: Option<usize>) -> R<Vec<u8>> {
        let mut out = Vec::new();
        while self.next_index(prefix, suffix).is_some() {
            let v = self.lines[self.pos].1;
            self.pos += 1;
            if v.starts_with(b"hex:") {
                out.extend_from_slice(&hex_bytes(&v[4..])?);
            } else if v.starts_with(b"0x") {
                let w = width.unwrap_or((v.len() - 2) / 2);
                out.extend_from_slice(&num_bytes(v, w)?);
            } else {
                return Err(());
            }
        }
        Ok(out)
    }

    fn phase_row(&mut self) -> R<u64> {
        self.hash.reset();
        let v = self.take_hex(&["gameTick"])?;
        self.hash.h32(v as u64);
        let v = self.take_hex(&["rng.parkMiller"])?;
        self.hash.h32(v as u64);
        let v = self.take_hex(&["rng.crtHoldrand"])?;
        self.hash.h32(v as u64);
        self.units()?;
        self.projectiles()?;
        self.players()?;
        self.features()?;
        self.wind()?;
        self.meteors()?;
        Ok(self.hash.h)
    }

    fn indexed_tables(&mut self, table: &str, array: &str) -> R<()> {
        let first = self.s.table(table)?.fields.first().ok_or(())?.name.clone();
        let suffix = format!("].{}", first);
        let prefix = format!("{}[", array);
        while let Some(i) = self.next_index(&prefix, &suffix) {
            self.hash.h32(i);
            self.table(table, &format!("{}[{}]", array, i))?;
        }
        Ok(())
    }

    fn units(&mut self) -> R<()> {
        self.hash.h8(1);
        self.indexed_tables("Unit", "units")?;
        self.hash.end();
        Ok(())
    }

    fn projectiles(&mut self) -> R<()> {
        self.hash.h8(2);
        self.indexed_tables("Projectile", "projectiles")?;
        self.hash.end();
        Ok(())
    }

    fn players(&mut self) -> R<()> {
        self.hash.h8(6);
        for i in 0..11 {
            self.hash.h32(i);
            self.table("Player", &format!("players[{}]", i))?;
        }
        self.hash.end();
        Ok(())
    }

    fn features(&mut self) -> R<()> {
        self.hash.h8(7);
        while let Some(i) = self.next_index("features[", "].list") {
            let which = self.take_hex(&["features[", &i.to_string(), "].list"])?;
            self.hash.h32(i);
            self.hash.h8(which as u64);
            let prefix = format!("features[{}]", i);
            let mut done = false;
            for name in ["SpotAnim", "SpotObject"] {
                let t = self.s.table(name)?;
                let n = t.fields.len();
                if self.pos + n <= self.lines.len()
                    && t.fields.iter().enumerate().all(|(k, f)| path_is(self.lines[self.pos + k].0, &[&prefix, ".", &f.name]))
                {
                    self.table(name, &prefix)?;
                    done = true;
                    break;
                }
            }
            if !done {
                return Err(());
            }
        }
        self.hash.end();
        Ok(())
    }

    fn wind(&mut self) -> R<()> {
        self.hash.h8(8);
        let s = self.s;
        self.flat_list(&s.wind, "wind.")?;
        self.hash.end();
        Ok(())
    }

    fn meteors(&mut self) -> R<()> {
        self.hash.h8(9);
        let s = self.s;
        self.flat_list(&s.g_meteors, "meteors.")?;
        self.hash.end();
        Ok(())
    }

    fn world_row(&mut self, tick: u64, reset: bool) -> R<u64> {
        if reset {
            self.hash.reset();
        }
        let mut live = Vec::new();
        let mut scripted = std::collections::HashSet::new();
        for (p, _) in self.lines.iter() {
            if p.starts_with(b"unitsState.units[") {
                if let Some(i) = match_index(p, "unitsState.units[", "].head") {
                    live.push(i);
                }
            } else if p.starts_with(b"scriptsState.units[") {
                if let Some(i) = match_index(p, "scriptsState.units[", "].scale") {
                    scripted.insert(i);
                }
            }
        }
        self.live = live;
        self.scripted = scripted;
        self.game()?;
        self.cells()?;
        self.visibility()?;
        self.explored()?;
        self.spatial()?;
        self.pathfinder()?;
        self.move_classes()?;
        self.orders()?;
        self.motions()?;
        self.order_effects()?;
        self.scripts()?;
        self.los_radar_index()?;
        self.factory_index()?;
        self.unit_resources()?;
        if tick == 1 {
            self.unit_definitions()?;
            self.weapon_definitions()?;
            self.mission()?;
        }
        self.skip_dumped(&DUMPED_ONLY);
        if self.pos != self.lines.len() {
            return Err(());
        }
        Ok(self.hash.h)
    }

    fn game(&mut self) -> R<()> {
        self.hash.h8(10);
        let s = self.s;
        self.flat_list(&s.game_scalars, "game.")?;
        self.flat_list(&s.g_game, "game.")?;
        self.hash.end();
        Ok(())
    }

    fn cells(&mut self) -> R<()> {
        self.hash.h8(11);
        let n = self.take_hex(&["cells.count"])?;
        self.hash.h32(n as u64);
        let run = self.collect_run("cells[", "]", None)?;
        self.hash.run(&run);
        self.hash.end();
        Ok(())
    }

    fn visibility(&mut self) -> R<()> {
        self.hash.h8(12);
        let n = self.take_hex(&["visibility.count"])?;
        self.hash.h32(n as u64);
        let run = self.collect_run("visibility[", "]", None)?;
        self.hash.run(&run);
        self.hash.end();
        Ok(())
    }

    fn explored(&mut self) -> R<()> {
        self.hash.h8(13);
        for who in 0..10 {
            let w = who.to_string();
            let n = self.take_hex(&["players[", &w, "].explored"])?;
            self.hash.h32(n as u64);
            let run = self.collect_run(&format!("players[{}].explored[", w), "]", Some(1))?;
            self.hash.run(&run);
        }
        self.hash.end();
        Ok(())
    }

    fn spatial(&mut self) -> R<()> {
        self.hash.h8(14);
        for name in ["width", "height", "count"] {
            let v = self.take_hex(&["spatial.", name])?;
            self.hash.h32(v as u64);
        }
        let first = self.s.table("Rec")?.fields.first().ok_or(())?.name.clone();
        let suffix = format!("].{}", first);
        while let Some(r) = self.next_index("spatial[", &suffix) {
            self.table("Rec", &format!("spatial[{}]", r))?;
        }
        let overflow = self.present("Rec", "spatial.overflow")?;
        self.hash.h8(overflow as u64);
        if overflow {
            self.table("Rec", "spatial.overflow")?;
        }
        self.hash.end();
        Ok(())
    }

    fn present(&self, table: &str, prefix: &str) -> R<bool> {
        let first = &self.s.table(table)?.fields.first().ok_or(())?.name;
        Ok(self.peek_is(&[prefix, ".", first]))
    }

    fn pathfinder(&mut self) -> R<()> {
        let saved = std::mem::replace(&mut self.hash, Hasher::new());
        let r = self.pathfinder_report();
        self.hash = saved;
        r?;
        self.skip_dumped(&REPORTS);
        Ok(())
    }

    fn pathfinder_report(&mut self) -> R<()> {
        self.hash.h8(16);
        let present = self.present("Pathfinder", "pathfinder")?;
        self.hash.h8(present as u64);
        if present {
            self.table("Pathfinder", "pathfinder")?;
            if self.present("PathfinderSearch", "pathfinder")? {
                self.table("PathfinderSearch", "pathfinder")?;
            }
            self.pathfinder_nodes()?;
            self.pathfinder_cells()?;
        }
        self.hash.end();
        self.hash.h8(17);
        let s = self.s;
        self.flat_list(&s.g_pathfinder, "pathfinder.")?;
        self.hash.end();
        Ok(())
    }

    fn pathfinder_nodes(&mut self) -> R<()> {
        let first = self.s.table("Node")?.fields.first().ok_or(())?.name.clone();
        let suffix = format!("].{}", first);
        while let Some(node) = self.next_index("pathfinder.pool[", &suffix) {
            self.hash.h32(node);
            let prefix = format!("pathfinder.pool[{}]", node);
            self.table("Node", &prefix)?;
            if let Some(p) = self.take_if(&[&prefix, ".penalty"]) {
                let b = num_bytes(p, 2)?;
                self.hash.run(&b);
            }
        }
        let c = self.take_hex(&["pathfinder.heap.count"])?;
        self.hash.h32(c as u64);
        while self.next_index("pathfinder.heap[", "]").is_some() {
            let v = self.lines[self.pos].1;
            self.pos += 1;
            let (t, i) = self.pointer_id(v)?;
            self.hash.pointer(t, i);
        }
        Ok(())
    }

    fn pathfinder_cells(&mut self) -> R<()> {
        while let Some(w) = self.next_index("pathfinder.dirty[", "]") {
            let bits = self.take_hex(&["pathfinder.dirty[", &w.to_string(), "]"])?;
            self.hash.h32(bits as u64);
            while let Some(index) = self.next_index("pathfinder.cells[", "].flags") {
                let p = format!("pathfinder.cells[{}]", index);
                let f = self.take(&[&p, ".flags"])?;
                let b = num_bytes(f, 1)?;
                self.hash.run(&b);
                if let Some(d) = self.take_if(&[&p, ".dir"]) {
                    let b = num_bytes(d, 1)?;
                    self.hash.run(&b);
                }
                if let Some(n) = self.take_if(&[&p, ".node"]) {
                    let b = num_bytes(n, 2)?;
                    self.hash.run(&b);
                }
            }
        }
        Ok(())
    }

    fn move_classes(&mut self) -> R<()> {
        self.hash.h8(18);
        let t = self.s.table("MoveClass")?;
        let wi = t.fields.iter().position(|f| f.offset == 0x10).ok_or(())?;
        let hi = t.fields.iter().position(|f| f.offset == 0x14).ok_or(())?;
        let mut dims = Vec::new();
        for c in 0..32 {
            let vals = self.table("MoveClass", &format!("moveClasses[{}]", c))?;
            dims.push((hexval(vals[wi])?, hexval(vals[hi])?));
        }
        self.hash.end();
        self.hash.h8(19);
        for (c, &(width, height)) in dims.iter().enumerate() {
            if !self.peek_starts(&format!("moveClasses[{}].passMap[", c)) {
                continue;
            }
            let (full, tail) = (height / 16, height & 15);
            self.hash.h32(c as u64);
            let run = self.pass_map_words(c, 0, full * width)?;
            self.hash.run(&run);
            if tail != 0 {
                let run = self.pass_map_words(c, full * width, width)?;
                self.hash.run(&run);
            }
        }
        self.hash.end();
        Ok(())
    }

    fn pass_map_words(&mut self, c: usize, start: u128, count: u128) -> R<Vec<u8>> {
        let mut out = Vec::new();
        let mut k = start;
        while k < start + count {
            out.extend_from_slice(&self.hex_run(&["moveClasses[", &c.to_string(), "].passMap[", &k.to_string(), "]"])?);
            k += 1;
        }
        Ok(out)
    }

    fn orders(&mut self) -> R<()> {
        self.hash.h8(20);
        let live = self.live.clone();
        for i in live {
            self.hash.h32(i);
            for (which, chain) in ["list", "list2"].iter().enumerate() {
                self.hash.h8(which as u64 + 1);
                let mut n = 0u64;
                loop {
                    let prefix = format!("units[{}].{}[{}]", i, chain, n);
                    if !self.present("Order", &prefix)? {
                        break;
                    }
                    self.table("Order", &prefix)?;
                    if self.present("OrderPosY", &prefix)? {
                        self.table("OrderPosY", &prefix)?;
                    }
                    n += 1;
                }
                self.hash.h8(0);
            }
        }
        self.hash.end();
        Ok(())
    }

    fn motions(&mut self) -> R<()> {
        self.hash.h8(21);
        let live = self.live.clone();
        for i in live {
            self.hash.h32(i);
            let prefix = format!("units[{}].motion", i);
            let present = self.present("UnitMotion", &prefix)?;
            self.hash.h8(present as u64);
            if present {
                self.table("UnitMotion", &prefix)?;
            }
        }
        self.hash.end();
        Ok(())
    }

    fn order_effects(&mut self) -> R<()> {
        self.hash.h8(22);
        while let Some(fx) = self.next_index("orderFx[", "].class") {
            let prefix = format!("orderFx[{}]", fx);
            let cls = self.take_hex(&[&prefix, ".class"])?;
            let start = self.pos;
            let dot = format!("{}.", prefix);
            while self.peek_starts(&dot) {
                self.pos += 1;
            }
            let group: &[Line] = &self.lines[start..self.pos];
            let find = |name: &str| -> Option<&'a [u8]> {
                group.iter().rev().find(|l| path_is(l.0, &[&prefix, ".", name])).map(|l| l.1)
            };
            if cls == 4 {
                let t = self.s.table("PathOrderRef")?;
                let mut vals = Vec::new();
                for f in &t.fields {
                    vals.push(find(&f.name).ok_or(())?);
                }
                self.hash_table(t, &vals)?;
            } else if cls == 0 {
                let vd = hexval(find("vtableDelta").ok_or(())?)?;
                self.hash.h32(fx);
                self.hash.h32(cls as u64);
                self.hash.h32(vd as u64);
            } else if !matches!(cls, 1 | 2 | 3 | 5) {
                return Err(());
            }
        }
        self.hash.end();
        Ok(())
    }

    fn scripts(&mut self) -> R<()> {
        self.hash.h8(23);
        let live = self.live.clone();
        for i in live {
            self.hash.h32(i);
            let sc = self.scripted.contains(&i);
            self.hash.h8(sc as u64);
            if sc {
                let prefix = format!("units[{}].script", i);
                let first = self.s.table("PieceTransform")?.fields.first().ok_or(())?.name.clone();
                let pre = format!("{}.transforms[", prefix);
                let suf = format!("].{}", first);
                while let Some(q) = self.next_index(&pre, &suf) {
                    self.table("PieceTransform", &format!("{}.transforms[{}]", prefix, q))?;
                }
            }
        }
        self.hash.end();
        Ok(())
    }

    fn unit_vector(&mut self, path: &str) -> R<()> {
        let n = self.take_hex(&[path, ".size"])?;
        self.hash.h32(n as u64);
        let mut k = 0u64;
        while self.peek_is(&[path, "[", &k.to_string(), "]"]) {
            let v = self.take(&[path, "[", &k.to_string(), "]"])?;
            let (t, i) = self.pointer_id(v)?;
            self.hash.pointer(t, i);
            k += 1;
        }
        Ok(())
    }

    fn los_radar_index(&mut self) -> R<()> {
        self.skip_dumped(&["losRadarState."]);
        while let Some(i) = self.next_index("losRadarIndex[", "].player") {
            let prefix = format!("losRadarIndex[{}]", i);
            self.hash.h8(24);
            let v = self.take_hex(&[&prefix, ".player"])?;
            self.hash.h32(v as u64);
            let v = self.take_hex(&[&prefix, ".hasSpecial"])?;
            self.hash.h32(v as u64);
            self.unit_vector(&format!("{}.visible", prefix))?;
            self.unit_vector(&format!("{}.known", prefix))?;
            self.hash.end();
        }
        Ok(())
    }

    fn factory_index(&mut self) -> R<()> {
        while let Some(i) = self.next_index("unitsFactories[", "].player") {
            let prefix = format!("unitsFactories[{}]", i);
            self.hash.h8(25);
            let v = self.take_hex(&[&prefix, ".player"])?;
            self.hash.h32(v as u64);
            self.unit_vector(&format!("{}.factories", prefix))?;
            self.hash.end();
        }
        self.skip_dumped(&["unitsState."]);
        Ok(())
    }

    fn unit_resources(&mut self) -> R<()> {
        self.hash.h8(27);
        for i in 0..11 {
            self.hash.h32(i);
            let prefix = format!("players[{}].econ", i);
            let present = self.present("UnitResources", &prefix)?;
            self.hash.h8(present as u64);
            if present {
                self.table("UnitResources", &prefix)?;
            }
        }
        self.hash.end();
        Ok(())
    }

    fn string(&mut self, parts: &[&str]) -> R<()> {
        let v = self.take(parts)?;
        if v == b"null" {
            self.hash.h8(0);
            return Ok(());
        }
        if !v.starts_with(b"str:") {
            return Err(());
        }
        let raw = &v[4..];
        self.hash.h8(1);
        self.hash.h32(raw.len() as u64);
        self.hash.run(raw);
        Ok(())
    }

    fn counted_run(&mut self, path: &str) -> R<()> {
        let n = self.take_hex(&[path])?;
        self.hash.h32(n as u64);
        let b = self.hex_run(&[path])?;
        self.hash.run(&b);
        Ok(())
    }

    fn unit_definitions(&mut self) -> R<()> {
        self.hash.h8(29);
        let n = self.take_hex(&["defs.count"])?;
        self.hash.h32(n as u64);
        let first = self.s.table("UnitDefData")?.fields.first().ok_or(())?.name.clone();
        let suffix = format!("].{}", first);
        while let Some(i) = self.next_index("defs[", &suffix) {
            let prefix = format!("defs[{}]", i);
            self.table("UnitDefData", &prefix)?;
            self.counted_run(&format!("{}.yardmap", prefix))?;
            self.counted_run(&format!("{}.ids", prefix))?;
            for m in 0..4 {
                let p = format!("{}.categories[{}]", prefix, m);
                let present = self.peek_is(&[&p]);
                self.hash.h8(present as u64);
                if present {
                    let b = self.hex_run(&[&p])?;
                    self.hash.run(&b);
                }
            }
        }
        self.hash.end();
        Ok(())
    }

    fn weapon_definitions(&mut self) -> R<()> {
        self.hash.h8(30);
        for i in 0..256 {
            let prefix = format!("weapons[{}]", i);
            self.table("WeaponDefData", &prefix)?;
            let present = self.peek_is(&[&prefix, ".damage.entries"]);
            self.hash.h8(present as u64);
            if !present {
                continue;
            }
            let n = self.take_hex(&[&prefix, ".damage.entries"])?;
            self.hash.h32(n as u64);
            let mut k = 0u64;
            while self.peek_is(&[&prefix, ".damage[", &k.to_string(), "].name"]) {
                self.string(&[&prefix, ".damage[", &k.to_string(), "].name"])?;
                let v = self.take_hex(&[&prefix, ".damage[", &k.to_string(), "].value"])?;
                self.hash.h32(v as u64);
                k += 1;
            }
        }
        self.hash.end();
        Ok(())
    }

    fn mission(&mut self) -> R<()> {
        self.hash.h8(31);
        let present = self.present("MissionData", "mission")?;
        self.hash.h8(present as u64);
        if present {
            self.table("MissionData", "mission")?;
            if self.present("MissionStartRest", "mission")? {
                self.table("MissionStartRest", "mission")?;
            }
            let n = self.take_hex(&["mission.units.count"])?;
            self.hash.h32(n as u64);
            let mut k = 0u64;
            while self.peek_is(&["mission.units[", &k.to_string(), "].name"]) {
                let prefix = format!("mission.units[{}]", k);
                for name in ["name", "ident", "initialMission"] {
                    self.string(&[&prefix, ".", name])?;
                }
                let b = self.hex_run(&[&prefix])?;
                self.hash.run(&b);
                k += 1;
            }
            let n = self.take_hex(&["mission.rules.count"])?;
            self.hash.h32(n as u64);
            let b = self.hex_run(&["mission.rules"])?;
            self.hash.run(&b);
            let n = self.take_hex(&["mission.features.count"])?;
            self.hash.h32(n as u64);
            let mut k = 0u64;
            while self.peek_is(&["mission.features[", &k.to_string(), "].name"]) {
                let prefix = format!("mission.features[{}]", k);
                self.string(&[&prefix, ".name"])?;
                let b = self.hex_run(&[&prefix, ".pos"])?;
                self.hash.run(&b);
                k += 1;
            }
        }
        self.hash.end();
        Ok(())
    }
}

fn append_value_bytes(out: &mut Vec<u8>, f: &Field, v: &[u8]) -> R<()> {
    if v.starts_with(b"hex:") {
        out.extend_from_slice(&hex_bytes(&v[4..])?);
        Ok(())
    } else if v.starts_with(b"0x") {
        out.extend_from_slice(&num_bytes(v, f.size)?);
        Ok(())
    } else {
        Err(())
    }
}

fn value_bytes(f: &Field, v: &[u8]) -> R<Vec<u8>> {
    let mut out = Vec::new();
    append_value_bytes(&mut out, f, v)?;
    Ok(out)
}

// ---------------------------------------------------------------------------------------------- driver

fn parse_line(line: &[u8]) -> Option<(u64, usize, &[u8], &[u8])> {
    let a = line.iter().position(|&b| b == b' ')?;
    let rest = &line[a + 1..];
    let b = rest.iter().position(|&b| b == b' ')?;
    let rest2 = &rest[b + 1..];
    let (path, value) = match rest2.iter().position(|&b| b == b' ') {
        Some(c) => (&rest2[..c], &rest2[c + 1..]),
        None => (rest2, &rest2[rest2.len()..]),
    };
    let tick = to_idx(&line[..a])?;
    let phase = PHASES.iter().position(|p| p.as_bytes() == &rest[..b])?;
    Some((tick, phase, path, value))
}

fn block_rows(
    schema: &Schema,
    key: (u64, usize),
    buf: &[u8],
    spans: &[(u32, u32, u32)],
    rows: &mut Vec<(u64, &'static str, u64)>,
) -> R<()> {
    // each span: start of the path in buf, length of the path, length of the value (which follows after a space)
    let lines: Vec<Line> = spans
        .iter()
        .map(|&(st, pl, vl)| {
            let st = st as usize;
            let pl = pl as usize;
            (&buf[st..st + pl], &buf[st + pl + 1..st + pl + 1 + vl as usize])
        })
        .collect();
    let (tick, phase) = key;
    let mut w = Walker::new(schema, &lines);
    if phase != 8 {
        let h = w.phase_row()?;
        rows.push((tick, PHASES[phase], h));
    }
    if phase == 7 {
        let h = w.world_row(tick, true)?;
        rows.push((tick, "world", h));
    }
    if phase == 8 {
        w.phase_row()?;
        let h = w.world_row(tick, false)?;
        rows.push((tick, "tick", h));
    }
    Ok(())
}

/// Reads the dump block by block (the blocks of a dump are contiguous), so that only one block is in memory.
fn compute(path: &str, schema: &Schema) -> R<Vec<(u64, &'static str, u64)>> {
    use std::io::BufRead;
    let f = std::fs::File::open(path).map_err(|_| ())?;
    let mut rd = std::io::BufReader::with_capacity(1 << 20, f);
    let mut line: Vec<u8> = Vec::new();
    rd.read_until(b'\n', &mut line).map_err(|_| ())?;
    if line.last() == Some(&b'\n') {
        line.pop();
    }
    if line != format!("bt-dump {}", schema.trace_format).as_bytes() {
        return Err(());
    }
    let mut rows = Vec::new();
    let mut key: Option<(u64, usize)> = None;
    let mut buf: Vec<u8> = Vec::new();
    let mut spans: Vec<(u32, u32, u32)> = Vec::new();
    let mut seen: std::collections::HashSet<(u64, usize)> = std::collections::HashSet::new();
    loop {
        line.clear();
        let n = rd.read_until(b'\n', &mut line).map_err(|_| ())?;
        if n == 0 {
            break;
        }
        if line.last() == Some(&b'\n') {
            line.pop();
        }
        if line.is_empty() {
            continue;
        }
        let (tick, phase, path, value) = parse_line(&line).ok_or(())?;
        let k = (tick, phase);
        if key != Some(k) {
            if let Some(prev) = key {
                block_rows(schema, prev, &buf, &spans, &mut rows)?;
                buf.clear();
                spans.clear();
            }
            if !seen.insert(k) {
                return Err(());
            }
            key = Some(k);
        }
        let st = buf.len() as u32;
        buf.extend_from_slice(path);
        buf.push(b' ');
        buf.extend_from_slice(value);
        spans.push((st, path.len() as u32, value.len() as u32));
    }
    if let Some(prev) = key {
        block_rows(schema, prev, &buf, &spans, &mut rows)?;
    }
    Ok(rows)
}

fn rows_call(call: &Call) -> String {
    let attempt = || -> R<String> {
        if call.args.len() < 2 {
            return Err(());
        }
        let schema = Schema::load(&std::fs::read(&call.args[1]).map_err(|_| ())?)?;
        let rows = compute(&call.args[0], &schema)?;
        let items: Vec<String> = rows.iter().map(|(t, p, h)| format!("[{},\"{}\",\"{:016x}\"]", t, p, h)).collect();
        Ok(format!("{}\t[{}]", call.name, items.join(",")))
    };
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(attempt)) {
        Ok(Ok(s)) => s,
        _ => format!("{}\t[]", call.name),
    }
}

pub fn run(calls: &[Call]) -> Vec<String> {
    calls.iter().map(rows_call).collect()
}
