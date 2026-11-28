//! Hosting wind_meteors: the replay of a recording of tests/replay/wind_meteors/ (docs/replay-format.md, "Replaying";
//! specs/engine-core-transition.md, section 4.4), judged by the core gate's `host` group.
use crate::commands::json_str;
use crate::loader::{Err, Loader};
use crate::rand::Rng;
use std::collections::{HashMap, VecDeque};
use wasmi::{Caller, Linker, Memory};

const MANIFEST: &str = include_str!("game_manifest.json");
const MODULE: &str = "wind_meteors";
const GAME_OFF: u64 = 0x1000;
const SIM_OFF: u64 = 0x50000;
const WEAPONS_OFF: u64 = 11507;
const WEAPON_SIZE: u64 = 277;
const WEAPON_FLAGS: u64 = 273;
const TICKS_OFF: u64 = 232007;
const PLAYERS_OFF: u64 = 7011;
const PLAYER_SIZE: u64 = 331;
const ARENA_SIZE: u64 = 2 << 20;
/// allocations start above the Game and SimGlobals records, 1 MiB into the block, and end at the radar arena
const ALLOC_OFF: u64 = 0x100000;
const LAST_KIND: u32 = 35;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    I32,
    U32,
    U16,
    F32,
    Str(usize),
    Point16,
    Weapon,
    Rng,
}

struct Field {
    name: &'static str,
    base: char, // 'g' Game, 's' SimGlobals, 'r' rng
    off: u64,
    kind: Kind,
}

const FIELDS: &[Field] = &[
    Field { name: "Game.windCounter", base: 'g', off: 229060, kind: Kind::U32 },
    Field { name: "Game.windSpeedDenominator", base: 'g', off: 229064, kind: Kind::I32 },
    Field { name: "Game.windX", base: 'g', off: 229068, kind: Kind::I32 },
    Field { name: "Game.windY", base: 'g', off: 229072, kind: Kind::I32 },
    Field { name: "Game.windZ", base: 'g', off: 229076, kind: Kind::I32 },
    Field { name: "Game.windDirection", base: 'g', off: 229080, kind: Kind::U16 },
    Field { name: "Game.windSpeed", base: 'g', off: 229082, kind: Kind::I32 },
    Field { name: "Game.windStrength", base: 'g', off: 229086, kind: Kind::F32 },
    Field { name: "Game.windEnabled", base: 'g', off: 229090, kind: Kind::I32 },
    Field { name: "Game.windSpeedMin", base: 'g', off: 82523, kind: Kind::I32 },
    Field { name: "Game.windSpeedMax", base: 'g', off: 82527, kind: Kind::I32 },
    Field { name: "Game.width", base: 'g', off: 82483, kind: Kind::I32 },
    Field { name: "Game.height", base: 'g', off: 82487, kind: Kind::I32 },
    Field { name: "Game.ticks", base: 'g', off: TICKS_OFF, kind: Kind::I32 },
    Field { name: "SimGlobals.g_meteorNextStrikeTime", base: 's', off: 564, kind: Kind::I32 },
    Field { name: "SimGlobals.g_meteorWeaponName", base: 's', off: 572, kind: Kind::Str(32) },
    Field { name: "SimGlobals.g_meteorSpawnMagnitude", base: 's', off: 604, kind: Kind::I32 },
    Field { name: "SimGlobals.g_meteorSpawnInterval", base: 's', off: 608, kind: Kind::I32 },
    Field { name: "SimGlobals.g_meteorActive", base: 's', off: 612, kind: Kind::I32 },
    Field { name: "SimGlobals.g_meteorStrikeEndTime", base: 's', off: 616, kind: Kind::I32 },
    Field { name: "SimGlobals.g_meteorOrigin", base: 's', off: 620, kind: Kind::Point16 },
    Field { name: "SimGlobals.g_meteorShowerDuration", base: 's', off: 624, kind: Kind::I32 },
    Field { name: "SimGlobals.g_meteorWeapon", base: 's', off: 628, kind: Kind::Weapon },
    Field { name: "SimGlobals.g_meteorsEnabled", base: 's', off: 632, kind: Kind::I32 },
    Field { name: "SimGlobals.g_meteorNextHitTime", base: 's', off: 636, kind: Kind::I32 },
    Field { name: "SimGlobals.g_meteorTarget", base: 's', off: 640, kind: Kind::Point16 },
    Field { name: "SimGlobals.g_meteorScheduleGap", base: 's', off: 644, kind: Kind::I32 },
    Field { name: "rng.main", base: 'r', off: 0, kind: Kind::Rng },
    Field { name: "rng.load", base: 'r', off: 1, kind: Kind::Rng },
    Field { name: "rng.seed", base: 'r', off: 2, kind: Kind::Rng },
];

const WRITES_WIND: &[&str] = &[
    "Game.windCounter", "Game.windX", "Game.windZ", "Game.windDirection", "Game.windSpeed", "Game.windStrength",
    "Game.windEnabled", "rng.main", "rng.load", "rng.seed",
];
const WRITES_METEORS: &[&str] = &[
    "SimGlobals.g_meteorNextStrikeTime", "SimGlobals.g_meteorActive", "SimGlobals.g_meteorStrikeEndTime",
    "SimGlobals.g_meteorOrigin", "SimGlobals.g_meteorNextHitTime", "SimGlobals.g_meteorTarget", "rng.main", "rng.load",
    "rng.seed",
];
const WRITES_APPLY: &[&str] = &[
    "Game.windCounter", "Game.windSpeedDenominator", "Game.windSpeedMin", "Game.windSpeedMax",
    "SimGlobals.g_meteorNextStrikeTime", "SimGlobals.g_meteorWeaponName", "SimGlobals.g_meteorSpawnMagnitude",
    "SimGlobals.g_meteorSpawnInterval", "SimGlobals.g_meteorActive", "SimGlobals.g_meteorStrikeEndTime",
    "SimGlobals.g_meteorOrigin", "SimGlobals.g_meteorShowerDuration", "SimGlobals.g_meteorsEnabled",
    "SimGlobals.g_meteorNextHitTime", "SimGlobals.g_meteorTarget", "SimGlobals.g_meteorScheduleGap", "rng.main",
    "rng.load", "rng.seed",
];
const WRITES_INIT: &[&str] = &["SimGlobals.g_meteorNextStrikeTime", "SimGlobals.g_meteorActive", "SimGlobals.g_meteorWeapon"];
const WRITES_LOAD: &[&str] = &[
    "SimGlobals.g_meteorNextStrikeTime", "SimGlobals.g_meteorActive", "SimGlobals.g_meteorStrikeEndTime",
    "SimGlobals.g_meteorOrigin", "SimGlobals.g_meteorsEnabled", "SimGlobals.g_meteorNextHitTime",
    "SimGlobals.g_meteorTarget",
];

fn field(name: &str) -> Option<&'static Field> {
    FIELDS.iter().find(|f| f.name == name)
}

/// One import call of the recording and what the import wrote through its arguments.
struct Entry {
    name: String,
    result: Option<String>,
    outs: Vec<(String, String)>,
}

#[derive(Default)]
pub struct State {
    mem: Option<Memory>,
    rng: Rng,
    entries: VecDeque<Entry>,
    mismatch: Option<String>,
    block_base: u64,
    /// the labels of the handles issued in this outermost call, handle n being entry n - 1 (docs/module-abi.md, 8.1)
    handles: Vec<String>,
    /// the refusal that ended the call: its class (`import` or `glue`) and text
    refusal: Option<(&'static str, String)>,
    /// the sim-block services of the services group: allocations, the sim event queue, the radar arena
    sim: SimSvc,
    /// from the manifest: the block's size, the scratch base, the kinds the entry publishes and their payload sizes
    block_size: u64,
    publishes: HashMap<i32, i32>,
}

struct SimSvc {
    top: u64,
    end: u64,
    arena_base: u64,
    live: std::collections::BTreeMap<u64, u64>,
    free: Vec<(u64, u64)>,
    queue: crate::events::Queue,
    arena_top: u64,
}

impl SimSvc {
    /// Returns a chunk to the free list, merging it with its neighbours and with the bump top.
    fn release(&mut self, addr: u64, n: u64) {
        let i = self.free.partition_point(|(a, _)| *a < addr);
        self.free.insert(i, (addr, n));
        if i + 1 < self.free.len() && self.free[i].0 + self.free[i].1 == self.free[i + 1].0 {
            self.free[i].1 += self.free[i + 1].1;
            self.free.remove(i + 1);
        }
        if i > 0 && self.free[i - 1].0 + self.free[i - 1].1 == self.free[i].0 {
            self.free[i - 1].1 += self.free[i].1;
            self.free.remove(i);
        }
        if let Some(&(a, n)) = self.free.last() {
            if a + n == self.top {
                self.top = a;
                self.free.pop();
            }
        }
    }
}

impl Default for SimSvc {
    fn default() -> SimSvc {
        SimSvc { top: 0, end: 0, arena_base: 0, live: Default::default(), free: Vec::new(), queue: crate::events::Queue::new(), arena_top: 0 }
    }
}

fn unescape(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    if b.first() == Some(&b'"') {
        i = 1;
    }
    while i < b.len() {
        match b[i] {
            b'"' => break,
            b'\\' if i + 1 < b.len() => {
                i += 1;
                match b[i] {
                    b'n' => out.push(b'\n'),
                    b't' => out.push(b'\t'),
                    b'r' => out.push(b'\r'),
                    b'0'..=b'7' => {
                        let mut v = 0u32;
                        let mut n = 0;
                        while n < 3 && i < b.len() && (b'0'..=b'7').contains(&b[i]) {
                            v = v * 8 + (b[i] - b'0') as u32;
                            i += 1;
                            n += 1;
                        }
                        out.push(v as u8);
                        continue;
                    }
                    b'x' => {
                        let mut v = 0u32;
                        i += 1;
                        while i < b.len() && b[i].is_ascii_hexdigit() {
                            v = v * 16 + (b[i] as char).to_digit(16).unwrap();
                            i += 1;
                        }
                        out.push(v as u8);
                        continue;
                    }
                    c => out.push(c),
                }
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

fn escape(b: &[u8]) -> String {
    let mut s = String::from("\"");
    for &c in b.iter().take_while(|&&c| c != 0) {
        match c {
            b'"' => s.push_str("\\\""),
            b'\\' => s.push_str("\\\\"),
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{:02x}", c)),
        }
    }
    s.push('"');
    s
}

fn f32_text(bits: u32) -> String {
    format!("f32:0x{:08x}", bits)
}

fn parse_num(s: &str) -> i64 {
    let s = s.trim();
    if let Some(h) = s.strip_prefix("0x") {
        return i64::from_str_radix(h, 16).unwrap_or(0);
    }
    s.parse::<i64>().unwrap_or(0)
}

fn parse_f32(s: &str) -> u32 {
    s.strip_prefix("f32:0x").and_then(|h| u32::from_str_radix(h, 16).ok()).unwrap_or_else(|| parse_num(s) as u32)
}

fn parse_f64(s: &str) -> u64 {
    s.strip_prefix("f64:0x").and_then(|h| u64::from_str_radix(h, 16).ok()).unwrap_or_else(|| parse_num(s) as u64)
}

fn weapon_index(s: &str) -> Option<u64> {
    s.strip_prefix("weapondef#").and_then(|n| n.trim().parse::<u64>().ok())
}

fn parse_point(s: &str) -> (i16, i16) {
    let t = s.trim().trim_start_matches('(').trim_end_matches(')');
    let mut it = t.split(',');
    let x = it.next().map(parse_num).unwrap_or(0) as i16;
    let y = it.next().map(parse_num).unwrap_or(0) as i16;
    (x, y)
}

struct Replayer {
    l: Loader<State>,
    game: u64,
    sim: u64,
    known: HashMap<String, String>,
}

impl Replayer {
    fn addr(&self, f: &Field) -> usize {
        (match f.base {
            'g' => self.game + f.off,
            _ => self.sim + f.off,
        }) as usize
    }

    fn mem(&mut self) -> &mut [u8] {
        let m = self.l.memory.unwrap();
        m.data_mut(&mut self.l.store)
    }

    fn read_field(&mut self, f: &Field) -> String {
        if f.base == 'r' {
            let r = &self.l.store.data().rng;
            let w = [r.main, r.load, r.seed][f.off as usize];
            return format!("0x{:08x}", w);
        }
        let a = self.addr(f);
        let m = self.mem();
        let u32at = |m: &[u8], a: usize| u32::from_le_bytes([m[a], m[a + 1], m[a + 2], m[a + 3]]);
        match f.kind {
            Kind::I32 => (u32at(m, a) as i32).to_string(),
            Kind::U32 => u32at(m, a).to_string(),
            Kind::U16 => u16::from_le_bytes([m[a], m[a + 1]]).to_string(),
            Kind::F32 => f32_text(u32at(m, a)),
            Kind::Str(n) => escape(&m[a..a + n]),
            Kind::Point16 => {
                let x = i16::from_le_bytes([m[a], m[a + 1]]);
                let y = i16::from_le_bytes([m[a + 2], m[a + 3]]);
                format!("({},{})", x, y)
            }
            Kind::Weapon => {
                let off = u32at(m, a) as u64;
                if off == 0 {
                    "null".to_string()
                } else {
                    let base = GAME_OFF + WEAPONS_OFF;
                    format!("weapondef#{}", (off.wrapping_sub(base)) / WEAPON_SIZE)
                }
            }
            Kind::Rng => String::new(),
        }
    }

    fn write_field(&mut self, f: &Field, v: &str) {
        if f.base == 'r' {
            let w = parse_num(v) as u32;
            let r = &mut self.l.store.data_mut().rng;
            match f.off {
                0 => r.main = w,
                1 => r.load = w,
                _ => r.seed = w,
            }
            return;
        }
        let a = self.addr(f);
        let bytes: Vec<u8> = match f.kind {
            Kind::I32 | Kind::U32 => (parse_num(v) as u32).to_le_bytes().to_vec(),
            Kind::U16 => (parse_num(v) as u16).to_le_bytes().to_vec(),
            Kind::F32 => parse_f32(v).to_le_bytes().to_vec(),
            Kind::Str(n) => {
                let mut b = unescape(v);
                b.resize(n, 0);
                b
            }
            Kind::Point16 => {
                let (x, y) = parse_point(v);
                let mut b = x.to_le_bytes().to_vec();
                b.extend_from_slice(&y.to_le_bytes());
                b
            }
            Kind::Weapon => {
                let off = match weapon_index(v) {
                    Some(n) => (GAME_OFF + WEAPONS_OFF + n * WEAPON_SIZE) as u32,
                    None => 0,
                };
                off.to_le_bytes().to_vec()
            }
            Kind::Rng => Vec::new(),
        };
        let m = self.mem();
        m[a..a + bytes.len()].copy_from_slice(&bytes);
    }

    fn write_data_words(&mut self) -> Result<(), Err> {
        let words: Vec<(String, u32)> = self.l.modules[MODULE].data_words.clone();
        let vals = |n: &str| -> Option<u32> {
            match n {
                "g_game" => Some(self.game as u32),
                "g_simGlobals" => Some(self.sim as u32),
                "g_blockBase" => Some(self.l.store.data().block_base as u32),
                "g_extTdf" => Some(u32::from_le_bytes(*b"tdf\0")),
                _ => None,
            }
        };
        let writes: Vec<(u32, u32)> = words.iter().filter_map(|(n, a)| vals(n).map(|v| (*a, v))).collect();
        let (lo, hi) = {
            let e = self.l.manifest.as_ref().unwrap().modules.iter().find(|e| e.name == MODULE).unwrap();
            (e.data_base, e.data_base + self.l.manifest.as_ref().unwrap().window)
        };
        if writes.iter().any(|(a, _)| (*a as u64) < lo || *a as u64 + 4 > hi) {
            return Result::Err(Err::named("window", MODULE));
        }
        let m = self.mem();
        for (a, v) in writes {
            m[a as usize..a as usize + 4].copy_from_slice(&v.to_le_bytes());
        }
        Ok(())
    }
}

struct CallBlock {
    ticks: String,
    slot: String,
    r: Vec<(String, String)>,
    a: Vec<(String, String)>,
    entries: Vec<Entry>,
}

fn split2(s: &str) -> (&str, &str) {
    match s.find(' ') {
        Some(i) => (&s[..i], s[i + 1..].trim_end()),
        None => (s, ""),
    }
}

fn parse_recording(text: &str) -> Vec<CallBlock> {
    let mut calls: Vec<CallBlock> = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("call ") {
            let (t, s) = split2(rest);
            calls.push(CallBlock { ticks: t.to_string(), slot: s.to_string(), r: vec![], a: vec![], entries: vec![] });
            continue;
        }
        let Some(c) = calls.last_mut() else { continue };
        let Some(body) = line.strip_prefix("  ") else { continue };
        if body.starts_with("  ") {
            continue; // a callback's call
        }
        let (tag, rest) = split2(body);
        match tag {
            "r" => {
                let (n, v) = split2(rest);
                c.r.push((n.to_string(), v.to_string()));
            }
            "a" => {
                let (n, v) = split2(rest);
                c.a.push((n.to_string(), v.to_string()));
            }
            "i" => {
                let (name, _) = split2(rest);
                let result = rest.rfind(" = ").map(|p| rest[p + 3..].trim_end().to_string());
                c.entries.push(Entry { name: name.to_string(), result, outs: vec![] });
            }
            "o" => {
                let (n, v) = split2(rest);
                if let Some(e) = c.entries.last_mut() {
                    e.outs.push((n.to_string(), v.to_string()));
                }
            }
            _ => {}
        }
    }
    calls
}

fn next_entry(caller: &mut Caller<'_, State>, name: &str) -> Option<Entry> {
    let st = caller.data_mut();
    match st.entries.pop_front() {
        Some(e) if e.name == name => Some(e),
        Some(e) => {
            st.mismatch = Some(format!("import {} where the recording has {}", name, e.name));
            None
        }
        None => {
            st.mismatch = Some(format!("import {} past the end of the recording's imports", name));
            None
        }
    }
}

fn read_cstr(mem: &[u8], a: usize) -> Vec<u8> {
    let mut v = Vec::new();
    let mut i = a;
    while i < mem.len() && mem[i] != 0 {
        v.push(mem[i]);
        i += 1;
    }
    v
}

/// The handle of a recorded object (`rec#2`), issued by first use in the outermost call; 0 for null.
fn issue(st: &mut State, label: &str) -> Result<u32, wasmi::Error> {
    if label == "null" {
        return Ok(0);
    }
    if let Some(i) = st.handles.iter().position(|h| h == label) {
        return Ok(i as u32 + 1);
    }
    if st.handles.len() >= 32 {
        return Result::Err(refuse(st, "glue", format!("{}: the handle table is full", MODULE)));
    }
    st.handles.push(label.to_string());
    Ok(st.handles.len() as u32)
}

/// Records the refusal that ends the replay and makes the trap that unwinds the call.
fn refuse(st: &mut State, class: &'static str, text: String) -> wasmi::Error {
    st.refusal = Some((class, text.clone()));
    wasmi::Error::new(text)
}

/// The refusal of an import's `linear` arguments outside the memory (docs/module-abi.md, 7.2); `ptrs` are (index, value).
fn check_ptrs(c: &mut Caller<'_, State>, name: &str, ptrs: &[(usize, i32)]) -> Result<(), wasmi::Error> {
    let len = c.data().mem.map(|m| m.data(&*c).len()).unwrap_or(0);
    for (i, v) in ptrs {
        if (*v as u32 as usize) >= len {
            let text = format!("import {}: pointer argument {} is outside the module memory", name, i);
            return Result::Err(refuse(c.data_mut(), "import", text));
        }
    }
    Ok(())
}

/// The refusal of a handle argument: null, or above the number issued in this outermost call.
fn check_handle(c: &mut Caller<'_, State>, name: &str, h: i32) -> Result<(), wasmi::Error> {
    let st = c.data_mut();
    if h == 0 {
        return Result::Err(refuse(st, "import", format!("import {}: null handle", name)));
    }
    if h < 0 || h as usize > st.handles.len() {
        return Result::Err(refuse(st, "import", format!("import {}: unknown handle {}", name, h as u32)));
    }
    Ok(())
}

fn apply_outs(caller: &mut Caller<'_, State>, e: &Entry, ptrs: &[u32], size: Option<usize>) -> Result<(), wasmi::Error> {
    let mem = caller.data().mem.unwrap();
    for (n, v) in &e.outs {
        if n.starts_with("file#") {
            let which = if n.ends_with(".root") { 0 } else if n.ends_with(".current") { 4 } else { continue };
            let h = issue(caller.data_mut(), v)?;
            let (m, _) = mem.data_and_store_mut(&mut *caller);
            let a = ptrs.first().copied().unwrap_or(0) as usize + which;
            if a + 4 > m.len() {
                continue;
            }
            m[a..a + 4].copy_from_slice(&h.to_le_bytes());
            continue;
        }
        let (m, st) = mem.data_and_store_mut(&mut *caller);
        if n == "dst" {
            let a = ptrs.get(1).copied().unwrap_or(0) as usize;
            let mut b = unescape(v);
            if let Some(sz) = size {
                b.truncate(sz.saturating_sub(1));
            }
            b.push(0);
            if a + b.len() > m.len() {
                continue;
            }
            m[a..a + b.len()].copy_from_slice(&b);
        } else if let Some(rest) = n.strip_prefix("weapondef#") {
            let (idx, f) = split_dot(rest);
            if f == "flags" {
                let a = (st.block_base + GAME_OFF + WEAPONS_OFF + idx * WEAPON_SIZE + WEAPON_FLAGS) as usize;
                let w = parse_num(v) as u32;
                if a + 4 <= m.len() {
                    m[a..a + 4].copy_from_slice(&w.to_le_bytes());
                }
            }
        } else if let Some(f) = n.strip_prefix("rng.") {
            let w = parse_num(v) as u32;
            match f {
                "main" => st.rng.main = w,
                "load" => st.rng.load = w,
                "seed" => st.rng.seed = w,
                _ => {}
            }
        }
    }
    Ok(())
}

fn split_dot(s: &str) -> (u64, &str) {
    match s.find('.') {
        Some(i) => (s[..i].parse().unwrap_or(0), &s[i + 1..]),
        None => (s.parse().unwrap_or(0), ""),
    }
}

fn result_i32(e: &Option<Entry>, block_base: u64) -> i32 {
    let Some(e) = e else { return 0 };
    match &e.result {
        None => 0,
        Some(r) => {
            if let Some(n) = weapon_index(r) {
                (block_base + GAME_OFF + WEAPONS_OFF + n * WEAPON_SIZE) as i32
            } else if r == "null" {
                0
            } else {
                parse_num(r) as i32
            }
        }
    }
}

fn define_services(linker: &mut Linker<State>) -> Result<(), wasmi::Error> {
    linker.func_wrap("services", "GameRand", |mut c: Caller<'_, State>, stream: i32| -> i32 {
        let e = next_entry(&mut c, "services.GameRand");
        let st = c.data_mut();
        let (v, _) = st.rng.game_rand(stream as i64);
        match &e {
            Some(_) => result_i32(&e, 0),
            None => v as i32,
        }
    })?;
    linker.func_wrap("services", "RandomInt", |mut c: Caller<'_, State>, range: i32| -> i32 {
        let e = next_entry(&mut c, "services.RandomInt");
        let st = c.data_mut();
        let v = st.rng.random_int(range);
        match &e {
            Some(_) => result_i32(&e, 0),
            None => v as i32,
        }
    })?;
    linker.func_wrap("services", "FatalError", |mut c: Caller<'_, State>, msg: i32| -> Result<(), wasmi::Error> {
        check_ptrs(&mut c, "FatalError", &[(0, msg)])?;
        let mem = c.data().mem.unwrap();
        let m = mem.data(&c);
        let text = String::from_utf8_lossy(&read_cstr(m, msg as u32 as usize)).into_owned();
        Result::Err(refuse(c.data_mut(), "fatal", text))
    })?;
    linker.func_wrap("services", "BuildDataPath", |mut c: Caller<'_, State>, buf: i32, d: i32, n: i32, x: i32| -> Result<i32, wasmi::Error> {
        check_ptrs(&mut c, "BuildDataPath", &[(0, buf), (1, d), (2, n), (3, x)])?;
        let e = next_entry(&mut c, "services.BuildDataPath");
        let mem = c.data().mem.unwrap();
        let m = mem.data_mut(&mut c);
        let a = buf as u32 as usize;
        if let Some(Entry { result: Some(r), .. }) = &e {
            let mut b = unescape(r);
            b.push(0);
            if a + b.len() <= m.len() {
                m[a..a + b.len()].copy_from_slice(&b);
            }
        }
        Ok(buf)
    })?;
    linker.func_wrap("services", "TdfFile_Construct", |mut c: Caller<'_, State>, file: i32| -> Result<(), wasmi::Error> {
        check_ptrs(&mut c, "TdfFile_Construct", &[(0, file)])?;
        let _ = next_entry(&mut c, "services.TdfFile_Construct");
        let mem = c.data().mem.unwrap();
        let m = mem.data_mut(&mut c);
        let a = file as u32 as usize;
        let end = (a + 12).min(m.len());
        m[a..end].fill(0);
        Ok(())
    })?;
    linker.func_wrap("services", "TdfFile_Destruct", |mut c: Caller<'_, State>, file: i32| -> Result<(), wasmi::Error> {
        check_ptrs(&mut c, "TdfFile_Destruct", &[(0, file)])?;
        let _ = next_entry(&mut c, "services.TdfFile_Destruct");
        Ok(())
    })?;
    linker.func_wrap("services", "TdfFile_LoadFile", |mut c: Caller<'_, State>, file: i32, p: i32| -> Result<i32, wasmi::Error> {
        check_ptrs(&mut c, "TdfFile_LoadFile", &[(0, file), (1, p)])?;
        let e = next_entry(&mut c, "services.TdfFile_LoadFile");
        if let Some(en) = &e {
            apply_outs(&mut c, en, &[file as u32], None)?;
        }
        Ok(result_i32(&e, 0))
    })?;
    linker.func_wrap("services", "TdfFile_SelectRecord", |mut c: Caller<'_, State>, file: i32, n: i32| -> Result<i32, wasmi::Error> {
        check_ptrs(&mut c, "TdfFile_SelectRecord", &[(0, file), (1, n)])?;
        let e = next_entry(&mut c, "services.TdfFile_SelectRecord");
        if let Some(en) = &e {
            apply_outs(&mut c, en, &[file as u32], None)?;
        }
        Ok(result_i32(&e, 0))
    })?;
    linker.func_wrap("services", "TdfRecord_GetFieldInt", |mut c: Caller<'_, State>, r: i32, k: i32, _d: i32| -> Result<i32, wasmi::Error> {
        check_ptrs(&mut c, "TdfRecord_GetFieldInt", &[(1, k)])?;
        check_handle(&mut c, "TdfRecord_GetFieldInt", r)?;
        let e = next_entry(&mut c, "services.TdfRecord_GetFieldInt");
        Ok(result_i32(&e, 0))
    })?;
    linker.func_wrap("services", "TdfRecord_GetFieldDouble", |mut c: Caller<'_, State>, r: i32, k: i32, _d: f64| -> Result<f64, wasmi::Error> {
        check_ptrs(&mut c, "TdfRecord_GetFieldDouble", &[(1, k)])?;
        check_handle(&mut c, "TdfRecord_GetFieldDouble", r)?;
        let e = next_entry(&mut c, "services.TdfRecord_GetFieldDouble");
        Ok(match e.and_then(|e| e.result) {
            Some(r) => f64::from_bits(parse_f64(&r)),
            None => 0.0,
        })
    })?;
    linker.func_wrap(
        "services",
        "TdfRecord_GetFieldString",
        |mut c: Caller<'_, State>, r: i32, dst: i32, k: i32, size: i32, d: i32| -> Result<i32, wasmi::Error> {
            let name = "TdfRecord_GetFieldString";
            check_ptrs(&mut c, name, &[(1, dst), (2, k), (4, d)])?;
            check_handle(&mut c, name, r)?;
            if size < 1 {
                return Result::Err(refuse(c.data_mut(), "import", format!("import {}: the buffer size is below 1", name)));
            }
            let mlen = c.data().mem.map(|m| m.data(&c).len()).unwrap_or(0);
            if dst as u32 as usize + size as usize > mlen {
                return Result::Err(refuse(c.data_mut(), "import", format!("import {}: the buffer runs past the module memory", name)));
            }
            let e = next_entry(&mut c, "services.TdfRecord_GetFieldString");
            if let Some(en) = &e {
                apply_outs(&mut c, en, &[0, dst as u32], Some(size as usize))?;
            }
            Ok(result_i32(&e, 0))
        },
    )?;
    linker.func_wrap("services", "HapiBank_OpenAccount", |mut c: Caller<'_, State>, b: i32, n: i32| -> Result<i32, wasmi::Error> {
        check_ptrs(&mut c, "HapiBank_OpenAccount", &[(1, n)])?;
        check_handle(&mut c, "HapiBank_OpenAccount", b)?;
        let e = next_entry(&mut c, "services.HapiBank_OpenAccount");
        Ok(result_i32(&e, 0))
    })?;
    linker.func_wrap("services", "HapiBank_SetIntegerItem", |mut c: Caller<'_, State>, b: i32, n: i32, _v: i32| -> Result<i32, wasmi::Error> {
        check_ptrs(&mut c, "HapiBank_SetIntegerItem", &[(1, n)])?;
        check_handle(&mut c, "HapiBank_SetIntegerItem", b)?;
        let e = next_entry(&mut c, "services.HapiBank_SetIntegerItem");
        Ok(result_i32(&e, 0))
    })?;
    linker.func_wrap("services", "HapiBank_GetIntegerItem", |mut c: Caller<'_, State>, b: i32, n: i32, _d: i32| -> Result<i32, wasmi::Error> {
        check_ptrs(&mut c, "HapiBank_GetIntegerItem", &[(1, n)])?;
        check_handle(&mut c, "HapiBank_GetIntegerItem", b)?;
        let e = next_entry(&mut c, "services.HapiBank_GetIntegerItem");
        Ok(result_i32(&e, 0))
    })?;
    linker.func_wrap("services", "Emit", |mut c: Caller<'_, State>, effect: i32, size: i32| -> Result<(), wasmi::Error> {
        check_ptrs(&mut c, "Emit", &[(0, effect)])?;
        let mem = c.data().mem.unwrap();
        let (a, n) = (effect as u32 as usize, size as u32 as usize);
        let publishes = c.data().publishes.clone();
        let text = if !(4..=128).contains(&n) {
            Some("the size is outside an effect")
        } else if a + n > mem.data(&c).len() {
            Some("the effect runs past the module memory")
        } else {
            let m = mem.data(&c);
            let kind = i32::from_le_bytes([m[a], m[a + 1], m[a + 2], m[a + 3]]);
            let want = publishes.get(&kind).copied();
            if want.is_none() {
                Some("the kind is not in the module's publishes table")
            } else if Some(n as i32) != want {
                Some("the size is not the payload's size of its kind")
            } else if kind != 80 || n < 8 {
                None
            } else {
                let weapon = i32::from_le_bytes([m[a + 4], m[a + 5], m[a + 6], m[a + 7]]);
                if !(0..256).contains(&weapon) {
                    Some("the weapon slot is outside the table of weapons")
                } else {
                    None
                }
            }
        };
        if let Some(t) = text {
            return Result::Err(refuse(c.data_mut(), "import", format!("import Emit: {}", t)));
        }
        let _ = next_entry(&mut c, "services.Emit");
        Ok(())
    })?;
    define_sim_services(linker)?;
    linker.func_wrap("weapons", "FindWeaponByName", |mut c: Caller<'_, State>, n: i32| -> Result<i32, wasmi::Error> {
        check_ptrs(&mut c, "FindWeaponByName", &[(0, n)])?;
        let e = next_entry(&mut c, "weapons.FindWeaponByName");
        if let Some(en) = &e {
            apply_outs(&mut c, en, &[], None)?;
        }
        if let Some(n) = e.as_ref().and_then(|en| en.result.as_deref()).and_then(weapon_index) {
            if n >= 256 {
                return Result::Err(refuse(c.data_mut(), "import", "import FindWeaponByName: returned a weapon outside the sim block".to_string()));
            }
        }
        let bb = c.data().block_base;
        Ok(result_i32(&e, bb))
    })?;
    Ok(())
}

fn replay(recording: &str, module_path: &str) -> Result<Vec<Vec<String>>, Err> {
    let mut rp = Replayer {
        l: Loader::new(State::default()),
        game: 0,
        sim: 0,
        known: HashMap::new(),
    };
    rp.l.set_manifest(MANIFEST.as_bytes())?;
    let scratch_base = {
        let m = rp.l.manifest.as_ref().ok_or_else(|| Err::new("manifest"))?;
        let bb = m.block_base.ok_or_else(|| Err::new("manifest"))?;
        let entry = m.modules.iter().find(|e| e.name == MODULE).ok_or_else(|| Err::named("not_in_manifest", MODULE))?;
        let publishes = entry
            .raw
            .get("publishes")
            .and_then(|v| v.as_object())
            .map(|o| o.iter().filter_map(|(k, v)| Some((k.parse::<i32>().ok()?, v.as_i64()? as i32))).collect())
            .unwrap_or_default();
        rp.game = bb + GAME_OFF;
        rp.sim = bb + SIM_OFF;
        let st = rp.l.store.data_mut();
        st.block_base = bb;
        st.block_size = m.block_size;
        st.publishes = publishes;
        st.sim.top = bb + ALLOC_OFF;
        st.sim.end = bb + m.block_size - ARENA_SIZE;
        st.sim.arena_base = st.sim.end;
        m.scratch.map(|s| s.0 as usize).unwrap_or(0)
    };
    rp.l.create_memory()?;
    let mem = rp.l.memory.unwrap();
    rp.l.store.data_mut().mem = Some(mem);
    let wasm = std::fs::read(module_path).map_err(|_| Err::named("compile", MODULE))?;
    rp.l.load(MODULE, &wasm, false, define_services)?;
    let blocks = parse_recording(recording);
    let mut out = Vec::new();
    let mut last_tick: Option<i64> = None;
    for c in blocks {
        let mut lines: Vec<String> = Vec::new();
        // the fields of the read set
        for (n, v) in &c.r {
            if let Some(f) = field(n) {
                rp.write_field(f, v);
                let t = rp.read_field(f);
                rp.known.insert(n.clone(), t);
            }
        }
        // the tick
        let ticks = if c.ticks == "-" { 0 } else { parse_num(&c.ticks) };
        if let Some(f) = field("Game.ticks") {
            rp.write_field(f, &ticks.to_string());
        }
        if last_tick != Some(ticks) {
            rp.l.tick();
            // the presentation drains the queue and the arena at the end of every tick
            let sim = &mut rp.l.store.data_mut().sim;
            sim.queue.drain();
            sim.arena_top = 0;
            last_tick = Some(ticks);
        }
        rp.write_data_words()?;
        rp.l.store.data_mut().entries = c.entries.into_iter().collect();
        rp.l.store.data_mut().mismatch = None;
        // the arguments
        let find = |n: &str| c.a.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
        let mut args: Vec<i32> = Vec::new();
        let mut closing: Vec<String> = Vec::new();
        let mut params_addr = 0usize;
        match c.slot.as_str() {
            "UpdateWind" => {
                args.push(if find("stream").as_deref() == Some("load") { 1 } else { 0 });
            }
            "Apply" => {
                let mut buf = vec![0u8; 256];
                for (k, v) in &c.a {
                    let Some(m) = k.strip_prefix("event.") else { continue };
                    let (off, kind) = match m {
                        "kind" => (0, Kind::I32),
                        "weapon" => (4, Kind::Str(32)),
                        "radius" => (36, Kind::I32),
                        "density" => (40, Kind::F32),
                        "duration" => (44, Kind::F32),
                        "interval" => (48, Kind::F32),
                        "min" => (4, Kind::I32),
                        "max" => (8, Kind::I32),
                        _ => continue,
                    };
                    match kind {
                        Kind::Str(n) => {
                            let mut b = unescape(v);
                            b.resize(n, 0);
                            buf[off..off + n].copy_from_slice(&b);
                        }
                        Kind::F32 => buf[off..off + 4].copy_from_slice(&parse_f32(v).to_le_bytes()),
                        _ => buf[off..off + 4].copy_from_slice(&(parse_num(v) as u32).to_le_bytes()),
                    }
                }
                rp.mem()[scratch_base..scratch_base + 256].copy_from_slice(&buf);
                args.push(scratch_base as i32);
                args.push(find("size").map(|s| parse_num(&s) as i32).unwrap_or(0));
            }
            "LoadMeteorDefaults" => {
                params_addr = scratch_base;
                rp.mem()[scratch_base..scratch_base + 52].fill(0);
                args.push(scratch_base as i32);
            }
            "SaveMeteors" | "LoadMeteors" => args.push(1),
            _ => {}
        }
        if std::env::var("CORE_DEBUG").is_ok() {
            eprintln!("call {} {} {:?}", c.ticks, c.slot, args);
        }
        let res = rp.l.call(MODULE, &c.slot, &args);
        rp.l.store.data_mut().handles.clear();
        if let Result::Err(e) = res {
            if let Some((class, text)) = rp.l.store.data_mut().refusal.take() {
                return Result::Err(Err { class, modules: vec![MODULE.to_string()], message: Some(text) });
            }
            return Result::Err(e);
        }
        if rp.l.store.data().mismatch.is_some() {
            eprintln!("replay: {}", rp.l.store.data().mismatch.clone().unwrap());
        }
        let writes: &[&str] = match c.slot.as_str() {
            "UpdateWind" => WRITES_WIND,
            "UpdateMeteors" => WRITES_METEORS,
            "Apply" => WRITES_APPLY,
            "InitMeteors" => WRITES_INIT,
            "LoadMeteors" => WRITES_LOAD,
            _ => &[],
        };
        for n in writes {
            let f = field(n).unwrap();
            let v = rp.read_field(f);
            if rp.known.get(*n) != Some(&v) {
                lines.push(format!("w {} {}", n, v));
                rp.known.insert(n.to_string(), v);
            }
        }
        if c.slot == "LoadMeteorDefaults" {
            let m = rp.mem();
            let p = &m[params_addr..params_addr + 52];
            closing.push(format!("o params.weapon {}", escape(&p[0..32])));
            closing.push(format!("o params.radius {}", i32::from_le_bytes([p[32], p[33], p[34], p[35]])));
            for (n, o) in [("density", 36), ("duration", 40), ("interval", 44)] {
                closing.push(format!("o params.{} {}", n, f32_text(u32::from_le_bytes([p[o + 0], p[o + 1], p[o + 2], p[o + 3]]))));
            }
        }
        lines.extend(closing);
        out.push(lines);
    }
    Ok(out)
}

pub fn run(calls: &[crate::Call]) -> Vec<String> {
    let mut out = Vec::new();
    for c in calls {
        if c.name != "replay" {
            out.push(format!("{}\tnull", c.name));
            continue;
        }
        let text = c.args.first().and_then(|p| std::fs::read(p).ok()).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
        let module = c.args.get(1).cloned().unwrap_or_else(|| "module.wasm".to_string());
        match replay(&text, &module) {
            Ok(entries) => {
                let es: Vec<String> = entries
                    .iter()
                    .map(|l| format!("[{}]", l.iter().map(|s| json_str(s)).collect::<Vec<_>>().join(",")))
                    .collect();
                out.push(format!("replay\t[{}]", es.join(",")));
            }
            Result::Err(e) => {
                out.push(format!("replay\t{}", e.json()));
                break;
            }
        }
    }
    out
}

fn rd32(m: &[u8], a: u64) -> Option<u32> {
    let a = a as usize;
    m.get(a..a + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn wr(m: &mut [u8], a: u64, v: &[u8]) -> bool {
    let a = a as usize;
    match m.get_mut(a..a.saturating_add(v.len())) {
        Some(d) => {
            d.copy_from_slice(v);
            true
        }
        None => false,
    }
}

fn game_tick(c: &Caller<'_, State>) -> u32 {
    let m = c.data().mem.unwrap().data(c);
    rd32(m, c.data().block_base + GAME_OFF + TICKS_OFF).unwrap_or(0)
}

/// The network id of a slot of the game record (the player's `id`), or -1 for none: slot 10 or a player of type 0.
fn slot_dpid(c: &Caller<'_, State>, slot: u32) -> i32 {
    if slot >= 10 {
        return -1;
    }
    let m = c.data().mem.unwrap().data(c);
    let p = c.data().block_base + GAME_OFF + PLAYERS_OFF + slot as u64 * PLAYER_SIZE;
    match (m.get((p + 115) as usize), rd32(m, p + 4)) {
        (Some(t), Some(id)) if *t != 0 => id as i32,
        _ => -1,
    }
}

fn alloc(c: &mut Caller<'_, State>, name: &str, size: u32) -> Result<i32, wasmi::Error> {
    let want = ((size as u64).max(1) + 7) & !7;
    let st = c.data_mut();
    let addr = if let Some(i) = st.sim.free.iter().position(|(_, n)| *n >= want) {
        let (a, n) = st.sim.free[i];
        if n == want {
            st.sim.free.remove(i);
        } else {
            st.sim.free[i] = (a + want, n - want);
        }
        a
    } else if st.sim.top + want <= st.sim.end {
        let a = st.sim.top;
        st.sim.top += want;
        a
    } else {
        return Result::Err(refuse(st, "import", format!("import {}: the state block has no room for {} bytes", name, size)));
    };
    st.sim.live.insert(addr, want);
    let mem = st.mem.unwrap();
    let m = mem.data_mut(&mut *c);
    let (a, n) = (addr as usize, want as usize);
    if a + n <= m.len() {
        m[a..a + n].fill(0);
    }
    Ok(addr as i32)
}

fn define_sim_services(linker: &mut Linker<State>) -> Result<(), wasmi::Error> {
    linker.func_wrap("services", "SimAlloc", |mut c: Caller<'_, State>, size: i32| -> Result<i32, wasmi::Error> { alloc(&mut c, "SimAlloc", size as u32) })?;
    linker.func_wrap("services", "SimAllocTagged", |mut c: Caller<'_, State>, tag: i32, size: i32| -> Result<i32, wasmi::Error> {
        check_ptrs(&mut c, "SimAllocTagged", &[(0, tag)])?;
        alloc(&mut c, "SimAllocTagged", size as u32)
    })?;
    linker.func_wrap("services", "SimFree", |mut c: Caller<'_, State>, p: i32| -> Result<(), wasmi::Error> {
        check_ptrs(&mut c, "SimFree", &[(0, p)])?;
        let st = c.data_mut();
        if let Some(n) = st.sim.live.remove(&(p as u32 as u64)) {
            st.sim.release(p as u32 as u64, n);
        }
        Ok(())
    })?;
    linker.func_wrap("services", "RecordSimEvent", |mut c: Caller<'_, State>, kind: i32, flags: i32, args: i32, count: i32| -> Result<(), wasmi::Error> {
        check_ptrs(&mut c, "RecordSimEvent", &[(2, args)])?;
        let (kind, count) = (kind as u32, count as u32);
        if !(1..=LAST_KIND).contains(&kind) || (args == 0 && count > 0) {
            return Ok(());
        }
        // the queue holds 4096 records of six arguments at most
        if count > 6 * 4096 {
            return Result::Err(refuse(c.data_mut(), "import", "import RecordSimEvent: the sim event queue is full".to_string()));
        }
        let mut words = Vec::new();
        if count > 0 {
            let m = c.data().mem.unwrap().data(&c);
            if args as u32 as u64 + 4 * count as u64 > m.len() as u64 {
                return Result::Err(refuse(c.data_mut(), "import", "import RecordSimEvent: the buffer runs past the module memory".to_string()));
            }
            for i in 0..count as u64 {
                words.extend(rd32(m, args as u32 as u64 + 4 * i));
            }
        }
        let tick = game_tick(&c);
        let q = &mut c.data_mut().sim.queue;
        q.tick = tick;
        if !q.record(kind, flags as u32, &words) {
            return Result::Err(refuse(c.data_mut(), "import", "import RecordSimEvent: the sim event queue is full".to_string()));
        }
        Ok(())
    })?;
    linker.func_wrap("services", "SimCaptureAlloc", |mut c: Caller<'_, State>, bytes: i32, off: i32| -> Result<i32, wasmi::Error> {
        check_ptrs(&mut c, "SimCaptureAlloc", &[(1, off)])?;
        if off == 0 || bytes < 0 {
            return Ok(0);
        }
        let want = ((bytes as u32 as u64) + 7) & !7;
        let st = c.data_mut();
        if st.sim.arena_top + want > ARENA_SIZE {
            return Result::Err(refuse(st, "import", "import SimCaptureAlloc: the radar arena has no room".to_string()));
        }
        let at = st.sim.arena_top;
        let st_arena = st.sim.arena_base;
        st.sim.arena_top += want;
        let mem = st.mem.unwrap();
        let m = mem.data_mut(&mut c);
        let a = (st_arena + at) as usize;
        if a + want as usize <= m.len() {
            m[a..a + want as usize].fill(0);
        }
        wr(m, off as u32 as u64, &(at as u32).to_le_bytes());
        Ok((st_arena + at) as i32)
    })?;
    linker.func_wrap("services", "GetGafFrame", |mut c: Caller<'_, State>, table: i32, index: i32| -> Result<i32, wasmi::Error> {
        check_ptrs(&mut c, "GetGafFrame", &[(0, table)])?;
        if table == 0 || index < 0 {
            return Ok(0);
        }
        let m = c.data().mem.unwrap().data(&c);
        let t = table as u32 as u64;
        let count = m.get(t as usize..t as usize + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as i32);
        match count {
            Some(n) if index < n => Ok(rd32(m, t + 40 + 8 * index as u64).unwrap_or(0) as i32),
            _ => Ok(0),
        }
    })?;
    linker.func_wrap("services", "InitGafSequence", |mut c: Caller<'_, State>, cur: i32, anim: i32, index: i32| -> Result<(), wasmi::Error> {
        check_ptrs(&mut c, "InitGafSequence", &[(0, cur), (1, anim)])?;
        if cur == 0 || anim == 0 || index < 0 {
            return Ok(());
        }
        let mem = c.data().mem.unwrap();
        let m = mem.data_mut(&mut c);
        let t = anim as u32 as u64;
        let (Some(h), Some(&lp)) = (m.get(t as usize..t as usize + 2), m.get(t as usize + 2)) else { return Ok(()) };
        let count = u16::from_le_bytes([h[0], h[1]]) as i32;
        let idx = if index < count { index } else { 0 };
        let dur = rd32(m, t + 40 + 8 * idx as u64 + 4).unwrap_or(0) as u16;
        let mut rec = [0u8; 12];
        rec[0..2].copy_from_slice(&(idx as u16).to_le_bytes());
        rec[2..4].copy_from_slice(&dur.to_le_bytes());
        rec[4] = lp;
        rec[8..12].copy_from_slice(&(anim as u32).to_le_bytes());
        wr(m, cur as u32 as u64, &rec);
        Ok(())
    })?;
    linker.func_wrap("services", "StepGafSequence", |mut c: Caller<'_, State>, cur: i32| -> Result<i32, wasmi::Error> {
        check_ptrs(&mut c, "StepGafSequence", &[(0, cur)])?;
        if cur == 0 {
            return Ok(0);
        }
        let mem = c.data().mem.unwrap();
        let m = mem.data_mut(&mut c);
        let a = cur as u32 as u64;
        let (Some(idx), Some(ticks), Some(anim)) = (
            m.get(a as usize..a as usize + 2).map(|b| u16::from_le_bytes([b[0], b[1]])),
            m.get(a as usize + 2..a as usize + 4).map(|b| u16::from_le_bytes([b[0], b[1]])),
            rd32(m, a + 8),
        ) else {
            return Ok(0);
        };
        if anim == 0 {
            return Ok(0);
        }
        if ticks >= 2 {
            wr(m, a + 2, &(ticks - 1).to_le_bytes());
            return Ok(0);
        }
        let t = anim as u64;
        let (Some(cb), Some(&lp)) = (m.get(t as usize..t as usize + 2), m.get(t as usize + 2)) else { return Ok(0) };
        let count = u16::from_le_bytes([cb[0], cb[1]]) as u64;
        let next = idx as u64 + 1;
        if next < count {
            let dur = rd32(m, t + 40 + 8 * next + 4).unwrap_or(0) as u16;
            wr(m, a, &(next as u16).to_le_bytes());
            wr(m, a + 2, &dur.to_le_bytes());
        } else if lp != 0 && count > 0 {
            let dur = rd32(m, t + 40 + 4).unwrap_or(0) as u16;
            wr(m, a, &0u16.to_le_bytes());
            wr(m, a + 2, &dur.to_le_bytes());
        } else {
            wr(m, a + 8, &0u32.to_le_bytes());
        }
        Ok(1)
    })?;
    linker.func_wrap("services", "GetSlotDpid", |c: Caller<'_, State>, slot: i32| -> i32 { slot_dpid(&c, slot as u32) })?;
    linker.func_wrap("services", "FindSlotByDpid", |c: Caller<'_, State>, id: i32| -> i32 {
        if id == -1 {
            return 10;
        }
        (0..10).find(|s| slot_dpid(&c, *s) == id).unwrap_or(10) as i32
    })?;
    Ok(())
}
