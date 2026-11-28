mod bigalloc;
mod commands;
mod console;
mod events;
mod hash;
mod host;
mod loader;
mod nan;
mod rand;
mod runtime;

#[global_allocator]
static ALLOC: bigalloc::BigAlloc = bigalloc::BigAlloc;

use std::fs;
use std::io::Write;

/// One call of a vector: the name and its tab separated arguments.
pub struct Call {
    pub name: String,
    pub args: Vec<String>,
}

fn parse_calls(text: &str) -> Vec<Call> {
    text.lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let mut it = l.split('\t');
            let name = it.next().unwrap_or("").to_string();
            Call { name, args: it.map(|s| s.to_string()).collect() }
        })
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("usage: core GROUP CALLS OUT");
        std::process::exit(2);
    }
    let group = &args[1];
    let calls_text = match fs::read(&args[2]) {
        Ok(b) => String::from_utf8_lossy(&b).into_owned(),
        Err(e) => {
            eprintln!("cannot read {}: {e}", args[2]);
            std::process::exit(2);
        }
    };
    let calls = parse_calls(&calls_text);
    if !matches!(group.as_str(), "rand" | "events" | "commands" | "console" | "runtime" | "hash" | "loader" | "fuel" | "host") {
        eprintln!("unknown group {group}");
        std::process::exit(2);
    }
    let run_group = |calls: &[Call]| -> Option<Vec<String>> {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match group.as_str() {
            "rand" => rand::run(calls),
            "events" => events::run(calls),
            "commands" => commands::run(calls),
            "console" => console::run(calls),
            "runtime" => runtime::run(calls),
            "hash" => hash::run(calls),
            "host" => host::run(calls),
            _ => loader::run(calls),
        }))
        .ok()
    };
    // a panic loses no output: the lines of the longest prefix of the calls that runs are written
    let lines: Vec<String> = match run_group(&calls) {
        Some(l) => l,
        None => {
            let (mut lo, mut hi) = (0, calls.len());
            while lo + 1 < hi {
                let mid = (lo + hi) / 2;
                if run_group(&calls[..mid]).is_some() {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            run_group(&calls[..lo]).unwrap_or_default()
        }
    };
    let mut out = Vec::new();
    for l in lines {
        out.extend_from_slice(l.as_bytes());
        out.push(b'\n');
    }
    fs::File::create(&args[3]).and_then(|mut f| f.write_all(&out)).expect("write OUT");
}
