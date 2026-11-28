//! The random streams of the game logic and the generator of `RandomInt` (specs/engine-core.md, section 5).
use crate::Call;

/// The three generators of the logic, each one 32-bit word.
#[derive(Default, Clone, Copy)]
pub struct Rng {
    pub main: u32,
    pub load: u32,
    pub seed: u32,
}

impl Rng {
    fn stream(&mut self, which: i64) -> &mut u32 {
        if which == 1 {
            &mut self.load
        } else {
            &mut self.main
        }
    }

    /// `GameRand`: one draw from a stream (0 main, 1 load): the value and the new word.
    pub fn game_rand(&mut self, which: i64) -> (u32, u32) {
        let w = self.stream(which);
        *w = w.wrapping_mul(214013).wrapping_add(2531011);
        ((*w >> 16) & 0x7fff, *w)
    }

    pub fn srand(&mut self, which: i64, seed: u32) {
        *self.stream(which) = seed;
    }

    /// The shuffle draw of a range `d`: one draw, widened 15 bits at a time while the range is above `m`.
    pub fn shuffle_draw(&mut self, which: i64, d: u32) -> (u32, u32) {
        let (v, w) = self.game_rand(which);
        let mut r = v;
        let mut m: u32 = 0x7fff;
        while m < d && m != 0xffff_ffff {
            m = (m << 15) | 0x7fff;
            r = (r << 15) | 0x7fff;
        }
        (if d == 0 { 0 } else { r % d }, w)
    }

    pub fn seed_int(&mut self, s: u32) {
        self.seed = (s ^ 0x66e2_9572) | 1;
    }

    /// `RandomInt(range)`.
    pub fn random_int(&mut self, range: i32) -> u32 {
        if range < 2 {
            return 0;
        }
        let g = self.seed;
        let q = g / 127773;
        let mut t = g.wrapping_mul(16807).wrapping_sub(q.wrapping_mul(2147483647));
        if (t as i32) <= 0 {
            t = t.wrapping_add(2147483647);
        }
        self.seed = t;
        t % (range as u32)
    }
}

fn num(s: &str) -> i64 {
    s.trim().parse::<i64>().unwrap_or(0)
}

pub fn run(calls: &[Call]) -> Vec<String> {
    let mut rng = Rng::default();
    let mut out = Vec::new();
    for c in calls {
        let a = &c.args;
        let v = match c.name.as_str() {
            "srand" => {
                let which = if a.first().map(|s| s.as_str()) == Some("load") { 1 } else { 0 };
                let seed = num(&a[1]) as u32;
                rng.srand(which, seed);
                format!("{}", *rng.stream(which))
            }
            "rand" => {
                let which = if a[0] == "load" { 1 } else { 0 };
                let (v, w) = rng.game_rand(which);
                format!("[{},{}]", v, w)
            }
            "shuffle" => {
                let which = if a[0] == "load" { 1 } else { 0 };
                let (v, w) = rng.shuffle_draw(which, num(&a[1]) as u32);
                format!("[{},{}]", v, w)
            }
            "seed" => {
                rng.seed_int(num(&a[0]) as u32);
                format!("{}", rng.seed)
            }
            "int" => {
                let v = rng.random_int(num(&a[0]) as i32);
                format!("[{},{}]", v, rng.seed)
            }
            "state" => format!("[{},{},{}]", rng.main, rng.load, rng.seed),
            _ => "null".to_string(),
        };
        out.push(format!("{}\t{}", c.name, v));
    }
    out
}
