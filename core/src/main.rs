mod commands;
mod console;
mod events;
mod hash;
mod host;
mod loader;
mod rand;
mod runtime;

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
    let calls_text = fs::read(&args[2]).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
    let calls = parse_calls(&calls_text);
    let lines: Vec<String> = match group.as_str() {
        "rand" => rand::run(&calls),
        "events" => events::run(&calls),
        "commands" => commands::run(&calls),
        "console" => console::run(&calls),
        "runtime" => runtime::run(&calls),
        "hash" => hash::run(&calls),
        "loader" | "fuel" => loader::run(&calls),
        "host" => host::run(&calls),
        _ => Vec::new(),
    };
    let mut out = Vec::new();
    for l in lines {
        out.extend_from_slice(l.as_bytes());
        out.push(b'\n');
    }
    fs::File::create(&args[3]).and_then(|mut f| f.write_all(&out)).expect("write OUT");
}
