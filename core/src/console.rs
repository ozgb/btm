//! Reading a console line (docs/commands.md, sections 3 and 4.2) for the commands that need only the match setup.
use crate::Call;

#[derive(Clone, Copy, Default)]
struct Slot {
    active: bool,
    kind: i64,
    side: i64,
    index: i64,
}

struct Match {
    game_type: i64,
    local: i64,
    viewing: i64,
    code: bool,
    slots: [Slot; 10],
}

/// The C runtime's `atoi`: leading white space, a sign, digits, accumulated modulo 2^32.
pub fn atoi(w: &[u8]) -> i32 {
    let mut i = 0;
    while i < w.len() && (w[i] == 32 || (9..=13).contains(&w[i])) {
        i += 1;
    }
    let mut neg = false;
    if i < w.len() && (w[i] == b'+' || w[i] == b'-') {
        neg = w[i] == b'-';
        i += 1;
    }
    let mut v: u32 = 0;
    while i < w.len() && w[i].is_ascii_digit() {
        v = v.wrapping_mul(10).wrapping_add((w[i] - b'0') as u32);
        i += 1;
    }
    (if neg { 0u32.wrapping_sub(v) } else { v }) as i32
}

fn words(line: &[u8]) -> Vec<&[u8]> {
    let mut out: Vec<&[u8]> = Vec::new();
    let mut i = 0;
    let n = line.len().min(79);
    let line = &line[..n];
    while i < n && out.len() < 20 {
        while i < n && (line[i] == 32 || (9..=13).contains(&line[i])) {
            i += 1;
        }
        if i >= n || line[i] == b'#' {
            break;
        }
        let s = i;
        while i < n && !(line[i] == 32 || (9..=13).contains(&line[i])) && line[i] != b'#' {
            i += 1;
        }
        out.push(&line[s..i]);
        if i < n && line[i] == b'#' {
            break;
        }
    }
    out
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

fn ev(kind: i32, fields: &[[u8; 4]], unit_field: bool) -> Vec<u8> {
    let mut v = kind.to_le_bytes().to_vec();
    if unit_field {
        v.extend_from_slice(&0u32.to_le_bytes());
    }
    for f in fields {
        v.extend_from_slice(f);
    }
    v
}

fn i(x: i64) -> [u8; 4] {
    (x as i32).to_le_bytes()
}

fn f(x: f32) -> [u8; 4] {
    x.to_bits().to_le_bytes()
}

impl Match {
    fn exists(&self, p: i64) -> bool {
        let p = p & 255;
        if p >= 10 {
            return false;
        }
        let s = &self.slots[p as usize];
        s.active && (1..=3).contains(&s.kind) && s.index != 10
    }

    /// Runs a line: the events (name, payload) it publishes; updates the code.
    fn line(&mut self, text: &[u8]) -> Vec<(&'static str, Vec<u8>)> {
        let w = words(text);
        let mut out = Vec::new();
        if w.is_empty() {
            return out;
        }
        let cmd = String::from_utf8_lossy(w[0]).to_ascii_lowercase();
        let arg = |n: usize| -> i64 { w.get(n).map(|x| atoi(x) as i64).unwrap_or(0) };
        // 0 always, 1 skirmish, 2 cheat
        let class = match cmd.as_str() {
            "give" | "nometal" | "noenergy" | "now" => 0,
            "meteor" | "atm" | "view" => 1,
            "sealevel" | "iwin" | "ilose" | "ai" | "kill" | "burnall" | "control" => 2,
            _ => return out,
        };
        let enabled = class == 0 || self.code || (class == 1 && self.game_type == 2);
        if cmd == "now" {
            let ok = w.len() == 6
                && w[1] == b"Film"
                && w[2] == b"Chris"
                && w[3] == b"Include"
                && w[4] == b"Reload"
                && w[5] == b"Assert";
            self.code = ok;
            return out;
        }
        if !enabled {
            return out;
        }
        let local = self.local;
        let local_side = self.slots[(local as usize).min(9)].side;
        match cmd.as_str() {
            "give" => {
                let to = arg(1) & 255;
                if self.exists(to) {
                    let word3 = w.get(3).map(|x| String::from_utf8_lossy(x).to_ascii_lowercase());
                    let resource = match word3.as_deref() {
                        Some("metal") => Some(1),
                        Some("energy") => Some(0),
                        _ => None,
                    };
                    if let Some(r) = resource {
                        let amount = arg(2) as i32 as f32;
                        out.push(("PLAYERS_TRANSFER", ev(196, &[i(local), i(to), i(r), f(amount), i(1)], true)));
                    }
                }
            }
            "meteor" => {
                if w.len() == 1 {
                    out.push(("WIND_METEORS_START", ev(67, &[], false)));
                } else if arg(1) != 0 {
                    out.push(("WIND_METEORS_ENABLE", ev(64, &[], false)));
                } else {
                    out.push(("WIND_METEORS_DISABLE", ev(65, &[], false)));
                }
            }
            "atm" => {
                for r in 0..2 {
                    out.push(("PLAYERS_SET_STORE", ev(197, &[i(self.viewing), i(r), i(1), f(1000.0)], true)));
                }
            }
            "nometal" | "noenergy" => {
                let p = if w.len() == 1 { local } else { arg(1) & 255 };
                if self.exists(p) {
                    let r = if cmd == "nometal" { 1 } else { 0 };
                    out.push(("PLAYERS_SET_STORE", ev(197, &[i(p), i(r), i(0), f(arg(2) as i32 as f32)], true)));
                }
            }
            "sealevel" => out.push(("MAP_SET_SEA_LEVEL", ev(170, &[i(arg(1))], true))),
            "iwin" => {
                let p = if local_side == 1 { 0 } else { 1 };
                out.push(("PLAYERS_KILL_UNITS", ev(193, &[i(p)], true)));
                out.push(("PLAYERS_END_GAME", ev(199, &[i(1)], true)));
            }
            "ilose" => {
                out.push(("PLAYERS_KILL_UNITS", ev(193, &[i(local_side)], true)));
                out.push(("PLAYERS_END_GAME", ev(199, &[i(0)], true)));
            }
            "ai" => {
                let p = if w.len() > 1 { arg(1) & 255 } else { local };
                if self.exists(p) {
                    let t = if self.slots[p as usize].kind == 1 { 2 } else { 1 };
                    out.push(("PLAYERS_SET_TYPE", ev(198, &[i(p), i(t)], true)));
                }
            }
            "kill" => {
                if w.len() == 1 {
                    out.push(("UNITS_KILL_ALL", ev(396, &[], true)));
                } else {
                    out.push(("PLAYERS_KILL_UNITS", ev(193, &[i(arg(1) & 255)], true)));
                }
                out.push(("PLAYERS_DEACTIVATE_CONDITIONS", ev(200, &[], true)));
            }
            "burnall" => out.push(("FEATURES_REMOVE_ALL", ev(36, &[], false))),
            "control" => out.push(("PLAYERS_SET_CONTROL", ev(203, &[i(arg(1) & 255)], true))),
            "view" => out.push(("PLAYERS_SET_VIEW", ev(204, &[i(arg(1) & 255)], true))),
            _ => {}
        }
        out
    }
}

fn num(s: &str) -> i64 {
    s.trim().parse::<i64>().unwrap_or(0)
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2).filter_map(|k| u8::from_str_radix(&s[2 * k..2 * k + 2], 16).ok()).collect()
}

pub fn run(calls: &[Call]) -> Vec<String> {
    let mut m = Match { game_type: 2, local: 0, viewing: 0, code: false, slots: [Slot::default(); 10] };
    let mut out = Vec::new();
    for c in calls {
        let a = &c.args;
        let v = match c.name.as_str() {
            "match" => {
                m.game_type = num(&a[0]);
                m.local = num(&a[1]);
                m.viewing = num(&a[2]);
                m.code = num(&a[3]) & 1 != 0;
                "null".to_string()
            }
            "slot" => {
                let n = num(&a[0]) as usize;
                if n < 10 {
                    m.slots[n] = Slot { active: num(&a[1]) != 0, kind: num(&a[2]), side: num(&a[3]), index: num(&a[4]) };
                }
                "null".to_string()
            }
            "line" => {
                let text = unhex(a.first().map(|s| s.as_str()).unwrap_or(""));
                let evs = m.line(&text);
                let list: Vec<String> = evs.iter().map(|(n, p)| format!("[\"{}\",\"{}\"]", n, hex(p))).collect();
                format!("{{\"events\":[{}],\"code\":{}}}", list.join(","), m.code as i32)
            }
            _ => "null".to_string(),
        };
        out.push(format!("{}\t{}", c.name, v));
    }
    out
}
