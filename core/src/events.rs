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

    /// Records an event: one record per six arguments, at least one; false when it does not fit in a test-mode run.
    pub fn record(&mut self, kind: u32, flags: u32, args: &[u32]) -> bool {
        let n = args.len().div_ceil(6).max(1) as u64;
        if self.tail.wrapping_sub(self.head) as u64 + n > CAP as u64 {
            if self.test_mode {
                return false;
            }
            self.dropped = self.dropped.wrapping_add(1);
            return true;
        }
        for i in 0..n as usize {
            let mut r = Rec { kind: kind as u16, flags: flags as u16, tick: self.tick, args: [0; 6] };
            if i > 0 {
                r.flags |= 0x8000;
            }
            for (j, a) in args.iter().skip(i * 6).take(6).enumerate() {
                r.args[j] = *a;
            }
            self.ring[self.tail as usize % CAP] = r;
            self.tail = self.tail.wrapping_add(1);
        }
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
                let need = args.len().div_ceil(6).max(1) as u64;
                let mut left = count;
                while left > 0 {
                    if q.tail.wrapping_sub(q.head) as u64 + need > CAP as u64 && !q.test_mode {
                        // a full queue in play mode only counts the drops
                        q.dropped = q.dropped.wrapping_add(left as u32);
                        break;
                    }
                    if !q.record(kind, flags, &args) {
                        ok = false;
                        break;
                    }
                    left -= 1;
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
