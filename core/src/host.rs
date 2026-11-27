//! Hosting wind_meteors: the replay of a recording of tests/replay/wind_meteors/ (docs/replay-format.md, "Replaying";
//! specs/engine-core-transition.md, section 4.4), judged by the core gate's `host` group.
use crate::commands::json_str;
use crate::loader::{Err, Loader};
use crate::rand::Rng;
use std::collections::{HashMap, VecDeque};
use wasmi::{Caller, Linker, Memory};

const MANIFEST: &str = include_str!("game_manifest.json");
const MODULE: &str = "wind_meteors";
const SCRATCH: usize = 983040;
const GAME_OFF: u64 = 0x1000;
const SIM_OFF: u64 = 0x50000;
const WEAPONS_OFF: u64 = 11507;
const WEAPON_SIZE: u64 = 277;
const WEAPON_FLAGS: u64 = 273;
const TICKS_OFF: u64 = 232007;

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
    block_base: u64,
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
                "g_blockBase" => Some(self.block_base as u32),
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

fn apply_outs(caller: &mut Caller<'_, State>, e: &Entry, ptrs: &[u32]) -> Result<(), wasmi::Error> {
    let mem = caller.data().mem.unwrap();
    let (m, st) = mem.data_and_store_mut(&mut *caller);
    for (n, v) in &e.outs {
        if let Some(rest) = n.strip_prefix("file#") {
            let _ = rest;
            let which = if n.ends_with(".root") { 0 } else if n.ends_with(".current") { 4 } else { continue };
            let h = v.strip_prefix("rec#").and_then(|x| x.parse::<u32>().ok()).unwrap_or(0);
            let a = ptrs.first().copied().unwrap_or(0) as usize + which;
            if a + 4 > m.len() {
                return Result::Err(wasmi::Error::new("import TdfFile_LoadFile: pointer argument 0 is outside the module memory"));
            }
            m[a..a + 4].copy_from_slice(&h.to_le_bytes());
        } else if n == "dst" {
            let a = ptrs.get(1).copied().unwrap_or(0) as usize;
            let mut b = unescape(v);
            b.push(0);
            if a + b.len() > m.len() {
                return Result::Err(wasmi::Error::new("import TdfRecord_GetFieldString: the buffer runs past the module memory"));
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
    linker.func_wrap("services", "FatalError", |c: Caller<'_, State>, msg: i32| -> Result<(), wasmi::Error> {
        let mem = c.data().mem.unwrap();
        let m = mem.data(&c);
        let a = msg as u32 as usize;
        if a >= m.len() {
            return Result::Err(wasmi::Error::new("import FatalError: pointer argument 0 is outside the module memory"));
        }
        let text = String::from_utf8_lossy(&read_cstr(m, a)).into_owned();
        Result::Err(wasmi::Error::new(text))
    })?;
    linker.func_wrap("services", "BuildDataPath", |mut c: Caller<'_, State>, buf: i32, _d: i32, _n: i32, _e: i32| -> Result<i32, wasmi::Error> {
        let e = next_entry(&mut c, "services.BuildDataPath");
        let mem = c.data().mem.unwrap();
        let m = mem.data_mut(&mut c);
        let a = buf as u32 as usize;
        if a >= m.len() {
            return Result::Err(wasmi::Error::new("import BuildDataPath: pointer argument 0 is outside the module memory"));
        }
        if let Some(Entry { result: Some(r), .. }) = &e {
            let mut b = unescape(r);
            b.push(0);
            if a + b.len() > m.len() {
                return Result::Err(wasmi::Error::new("import BuildDataPath: the buffer runs past the module memory"));
            }
            m[a..a + b.len()].copy_from_slice(&b);
        }
        Ok(buf)
    })?;
    linker.func_wrap("services", "TdfFile_Construct", |mut c: Caller<'_, State>, file: i32| -> Result<(), wasmi::Error> {
        let _ = next_entry(&mut c, "services.TdfFile_Construct");
        let mem = c.data().mem.unwrap();
        let m = mem.data_mut(&mut c);
        let a = file as u32 as usize;
        if a.saturating_add(12) > m.len() {
            return Result::Err(wasmi::Error::new("import TdfFile_Construct: pointer argument 0 is outside the module memory"));
        }
        m[a..a + 12].fill(0);
        Ok(())
    })?;
    linker.func_wrap("services", "TdfFile_Destruct", |mut c: Caller<'_, State>, _file: i32| {
        let _ = next_entry(&mut c, "services.TdfFile_Destruct");
    })?;
    linker.func_wrap("services", "TdfFile_LoadFile", |mut c: Caller<'_, State>, file: i32, _p: i32| -> Result<i32, wasmi::Error> {
        let e = next_entry(&mut c, "services.TdfFile_LoadFile");
        if let Some(en) = &e {
            apply_outs(&mut c, en, &[file as u32])?;
        }
        Ok(result_i32(&e, 0))
    })?;
    linker.func_wrap("services", "TdfFile_SelectRecord", |mut c: Caller<'_, State>, file: i32, _n: i32| -> Result<i32, wasmi::Error> {
        let e = next_entry(&mut c, "services.TdfFile_SelectRecord");
        if let Some(en) = &e {
            apply_outs(&mut c, en, &[file as u32])?;
        }
        Ok(result_i32(&e, 0))
    })?;
    linker.func_wrap("services", "TdfRecord_GetFieldInt", |mut c: Caller<'_, State>, _r: i32, _k: i32, _d: i32| -> i32 {
        let e = next_entry(&mut c, "services.TdfRecord_GetFieldInt");
        result_i32(&e, 0)
    })?;
    linker.func_wrap("services", "TdfRecord_GetFieldDouble", |mut c: Caller<'_, State>, _r: i32, _k: i32, _d: f64| -> f64 {
        let e = next_entry(&mut c, "services.TdfRecord_GetFieldDouble");
        match e.and_then(|e| e.result) {
            Some(r) => f64::from_bits(parse_f64(&r)),
            None => 0.0,
        }
    })?;
    linker.func_wrap(
        "services",
        "TdfRecord_GetFieldString",
        |mut c: Caller<'_, State>, _r: i32, dst: i32, _k: i32, size: i32, _d: i32| -> Result<i32, wasmi::Error> {
            let e = next_entry(&mut c, "services.TdfRecord_GetFieldString");
            if size < 1 {
                return Result::Err(wasmi::Error::new("import TdfRecord_GetFieldString: the buffer size is below 1"));
            }
            let mlen = c.data().mem.map(|m| m.data(&c).len()).unwrap_or(0);
            if dst < 0 || dst as usize + size as usize > mlen {
                return Result::Err(wasmi::Error::new("import TdfRecord_GetFieldString: the buffer runs past the module memory"));
            }
            if let Some(en) = &e {
                apply_outs(&mut c, en, &[0, dst as u32])?;
            }
            Ok(result_i32(&e, 0))
        },
    )?;
    linker.func_wrap("services", "HapiBank_OpenAccount", |mut c: Caller<'_, State>, _b: i32, _n: i32| -> i32 {
        let e = next_entry(&mut c, "services.HapiBank_OpenAccount");
        result_i32(&e, 0)
    })?;
    linker.func_wrap("services", "HapiBank_SetIntegerItem", |mut c: Caller<'_, State>, _b: i32, _n: i32, _v: i32| -> i32 {
        let e = next_entry(&mut c, "services.HapiBank_SetIntegerItem");
        result_i32(&e, 0)
    })?;
    linker.func_wrap("services", "HapiBank_GetIntegerItem", |mut c: Caller<'_, State>, _b: i32, _n: i32, _d: i32| -> i32 {
        let e = next_entry(&mut c, "services.HapiBank_GetIntegerItem");
        result_i32(&e, 0)
    })?;
    linker.func_wrap("services", "Emit", |mut c: Caller<'_, State>, effect: i32, size: i32| -> Result<(), wasmi::Error> {
        let _ = next_entry(&mut c, "services.Emit");
        let mem = c.data().mem.unwrap();
        let m = mem.data(&c);
        let (a, n) = (effect as u32 as usize, size as u32 as usize);
        if n < 4 || n > 256 {
            return Result::Err(wasmi::Error::new("import Emit: the size is outside an effect"));
        }
        if a + n > m.len() {
            return Result::Err(wasmi::Error::new("import Emit: the effect runs past the module memory"));
        }
        let kind = i32::from_le_bytes([m[a], m[a + 1], m[a + 2], m[a + 3]]);
        if kind != 80 {
            return Result::Err(wasmi::Error::new("import Emit: the kind is not in the module's publishes table"));
        }
        if n != 36 {
            return Result::Err(wasmi::Error::new("import Emit: the size is not the payload's size of its kind"));
        }
        let weapon = i32::from_le_bytes([m[a + 4], m[a + 5], m[a + 6], m[a + 7]]);
        if !(-1..256).contains(&weapon) {
            return Result::Err(wasmi::Error::new("import Emit: the weapon slot is outside the table of weapons"));
        }
        Ok(())
    })?;
    linker.func_wrap("weapons", "FindWeaponByName", |mut c: Caller<'_, State>, _n: i32| -> Result<i32, wasmi::Error> {
        let e = next_entry(&mut c, "weapons.FindWeaponByName");
        if let Some(en) = &e {
            apply_outs(&mut c, en, &[])?;
        }
        let bb = c.data().block_base;
        Ok(result_i32(&e, bb))
    })?;
    Ok(())
}

fn replay(recording: &str, module_path: &str) -> Result<Vec<Vec<String>>, Err> {
    let mut rp = Replayer {
        l: Loader::new(State { block_base: 1048576, ..Default::default() }),
        game: 1048576 + GAME_OFF,
        sim: 1048576 + SIM_OFF,
        known: HashMap::new(),
        block_base: 1048576,
    };
    rp.l.set_manifest(MANIFEST.as_bytes())?;
    rp.l.create_memory()?;
    let mem = rp.l.memory.unwrap();
    rp.l.store.data_mut().mem = Some(mem);
    let wasm = std::fs::read(module_path).map_err(|_| Err::named("compile", MODULE))?;
    rp.l.load(MODULE, &wasm, define_services)?;
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
                rp.mem()[SCRATCH..SCRATCH + 256].copy_from_slice(&buf);
                args.push(SCRATCH as i32);
                args.push(find("size").map(|s| parse_num(&s) as i32).unwrap_or(0));
            }
            "LoadMeteorDefaults" => {
                params_addr = SCRATCH;
                rp.mem()[SCRATCH..SCRATCH + 52].fill(0);
                args.push(SCRATCH as i32);
            }
            "SaveMeteors" | "LoadMeteors" => args.push(1),
            _ => {}
        }
        if std::env::var("CORE_DEBUG").is_ok() {
            eprintln!("call {} {} {:?}", c.ticks, c.slot, args);
        }
        let res = rp.l.call(MODULE, &c.slot, &args);
        if let Result::Err(e) = res {
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
