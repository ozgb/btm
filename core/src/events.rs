//! The sim event queue (docs/sim-events.md, "The queue"; specs/engine-core.md, section 5).
use crate::Call;

const CAP: usize = 4096;

#[derive(Clone, Copy, Default)]
pub struct Rec {
    pub kind: u16,
    pub flags: u16,
    pub tick: u32,
    pub args: [u32; 6],
}

pub struct Queue {
    pub ring: Vec<Rec>,
    pub head: u32,
    pub tail: u32,
    pub dropped: u32,
    pub test_mode: bool,
    pub tick: u32,
}

impl Queue {
    pub fn new() -> Queue {
        Queue { ring: vec![Rec::default(); CAP], head: 0, tail: 0, dropped: 0, test_mode: true, tick: 0 }
    }

    /// Records an event of up to twelve arguments; false when it does not fit in a test-mode run.
    pub fn record(&mut self, kind: u32, flags: u32, args: &[u32]) -> bool {
        let n: u32 = if args.len() > 6 { 2 } else { 1 };
        if self.tail.wrapping_sub(self.head) as u64 + n as u64 > CAP as u64 {
            if self.test_mode {
                return false;
            }
            self.dropped = self.dropped.wrapping_add(1);
            return true;
        }
        let mut first = Rec { kind: kind as u16, flags: flags as u16, tick: self.tick, args: [0; 6] };
        for (i, a) in args.iter().take(6).enumerate() {
            first.args[i] = *a;
        }
        let i = self.tail as usize % CAP;
        self.ring[i] = first;
        if n == 2 {
            let mut second = Rec { kind: kind as u16, flags: (flags as u16) | 0x8000, tick: self.tick, args: [0; 6] };
            for (i, a) in args.iter().skip(6).take(6).enumerate() {
                second.args[i] = *a;
            }
            self.ring[(self.tail as usize + 1) % CAP] = second;
        }
        self.tail = self.tail.wrapping_add(n);
        true
    }

    pub fn pending(&self) -> Vec<Rec> {
        let mut v = Vec::new();
        let mut i = self.head;
        while i != self.tail {
            v.push(self.ring[i as usize % CAP]);
            i = i.wrapping_add(1);
        }
        v
    }

    pub fn drain(&mut self) -> Vec<Rec> {
        let v = self.pending();
        self.head = self.tail;
        v
    }
}

fn rec_json(r: &Rec) -> String {
    format!(
        "[{},{},{},[{}]]",
        r.kind,
        r.flags,
        r.tick,
        r.args.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(",")
    )
}

fn num(s: &str) -> i64 {
    s.trim().parse::<i64>().unwrap_or(0)
}

pub fn run(calls: &[Call]) -> Vec<String> {
    let mut q = Queue::new();
    let mut out = Vec::new();
    for c in calls {
        let a = &c.args;
        let v: String = match c.name.as_str() {
            "mode" => {
                q.test_mode = a.first().map(|s| s.as_str()) != Some("play");
                "null".into()
            }
            "tick" => {
                q.tick = num(&a[0]) as u32;
                "null".into()
            }
            "record" | "repeat" => {
                let (count, rest) = if c.name == "repeat" { (num(&a[0]), &a[1..]) } else { (1, &a[..]) };
                let kind = num(&rest[0]) as u32;
                let flags = num(&rest[1]) as u32;
                let args: Vec<u32> = rest[2..].iter().map(|s| num(s) as u32).collect();
                let mut ok = true;
                for _ in 0..count {
                    if !q.record(kind, flags, &args) {
                        ok = false;
                        break;
                    }
                }
                if ok {
                    format!("[{},{},{}]", q.head, q.tail, q.dropped)
                } else {
                    out.push(format!("{}\t\"full\"", c.name));
                    break;
                }
            }
            "counters" => format!("[{},{},{},0]", q.head, q.tail, q.dropped),
            "ring" => rec_json(&q.ring[(num(&a[0]) as usize) % CAP]),
            "pending" => format!("[{}]", q.pending().iter().map(rec_json).collect::<Vec<_>>().join(",")),
            "drain" => format!("[{}]", q.drain().iter().map(rec_json).collect::<Vec<_>>().join(",")),
            _ => "null".into(),
        };
        out.push(format!("{}\t{}", c.name, v));
    }
    out
}
