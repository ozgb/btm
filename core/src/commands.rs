//! The delivery of the commands of a tick (docs/commands.md, sections 1 and 2).
use crate::Call;
use std::fs;

fn rank(key: &str) -> u32 {
    match key {
        "interface" => 0,
        "spawn" => 1,
        "order" => 2,
        "console" => 3,
        _ => 4,
    }
}

/// The lines of `commands.txt` for the commands of a script, in the order of delivery.
fn deliver(text: &str) -> Vec<String> {
    let mut cmds: Vec<(u64, u32, u64, usize, String)> = Vec::new();
    for (i, l) in text.lines().enumerate() {
        let mut it = l.splitn(4, ' ');
        let (first, key) = (it.next().unwrap_or(""), it.next().unwrap_or(""));
        if i == 0 && first == "bt-script" || first == "units" || first == "end" || first.is_empty() {
            continue;
        }
        let tick = it.next().unwrap_or("0");
        let rest = it.next().unwrap_or("0");
        let ln: u64 = first.parse().unwrap_or(0);
        let t: u64 = tick.parse().unwrap_or(0);
        cmds.push((t, rank(key), ln, i, format!("{} {} {} {}", tick, key, first, rest)));
    }
    cmds.sort_by(|a, b| (a.0, a.1, if a.1 == 0 { 0 } else { a.2 }, a.3).cmp(&(b.0, b.1, if b.1 == 0 { 0 } else { b.2 }, b.3)));
    cmds.into_iter().map(|c| c.4).collect()
}

pub fn run(calls: &[Call]) -> Vec<String> {
    let mut out = Vec::new();
    for c in calls {
        let v = match c.name.as_str() {
            "deliver" => {
                let text = c.args.first().and_then(|p| fs::read(p).ok()).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
                let lines = deliver(&text);
                format!("[{}]", lines.iter().map(|s| json_str(s)).collect::<Vec<_>>().join(","))
            }
            _ => "null".into(),
        };
        out.push(format!("{}\t{}", c.name, v));
    }
    out
}

pub fn json_str(s: &str) -> String {
    let mut o = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}
