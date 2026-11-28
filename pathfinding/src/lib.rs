#![no_std]

use core::ptr::{read_unaligned, read_volatile, write_unaligned};

#[link(wasm_import_module = "pathworld")]
extern "C" {
    #[link_name = "Tick"]
    fn w_tick() -> u32;
    #[link_name = "SearchStepLimit"]
    fn w_step_limit() -> u32;
    #[link_name = "PlayerCount"]
    fn w_player_count() -> i32;
    #[link_name = "IsPlaying"]
    fn w_is_playing(p: i32) -> i32;
    #[link_name = "UnitsPerPlayer"]
    fn w_units_per_player() -> i32;
    #[link_name = "FirstUnit"]
    fn w_first_unit(p: i32) -> u32;
    #[link_name = "LastUnit"]
    fn w_last_unit(p: i32) -> u32;
    #[link_name = "NextUnit"]
    fn w_next_unit(u: u32) -> u32;
    #[link_name = "UnitState"]
    fn w_unit_state(u: u32, out: *mut u8) -> i32;
    #[link_name = "MoveClass"]
    fn w_move_class(i: i32) -> u32;
    #[link_name = "InBounds"]
    fn w_in_bounds(c: u32, x: i32, y: i32) -> i32;
    #[link_name = "Pass"]
    fn w_pass(c: u32, x: i32, y: i32) -> u32;
    #[link_name = "SeenSize"]
    fn w_seen_size(w: *mut i32, h: *mut i32);
    #[link_name = "Seen"]
    fn w_seen(p: i32, x: i32, y: i32) -> i32;
    #[link_name = "LineOfSight"]
    fn w_los(c: u32, a: i32, b: i32, c2: i32, d: i32) -> i32;
    #[link_name = "TerrainPass"]
    fn w_terrain_pass(c: u32, x: i32, y: i32) -> i32;
    #[link_name = "BeginSearch"]
    fn w_begin_search(u: u32);
    #[link_name = "EndSearch"]
    fn w_end_search(u: u32);
}

#[no_mangle]
pub static mut g_game: u32 = 1;
#[no_mangle]
pub static mut g_simGlobals: u32 = 1;
#[no_mangle]
pub static mut g_extTdf: u32 = 1;
#[no_mangle]
pub static mut g_blockBase: u32 = 1;
#[no_mangle]
pub static mut g_region: u32 = 1;


static NEVER: u32 = 0;
fn noop() {}
static TABLE: [fn(); 1] = [noop];

const GAME_PATHFINDER: u32 = 82439;
const GAME_WIDTH: u32 = 82483;
const GAME_HEIGHT: u32 = 82487;

// Head
const H_POOL: u32 = 0;
const H_ITEMS: u32 = 4;
const H_FREE: u32 = 8;
const H_USED: u32 = 12;
const H_CAP: u32 = 16;
const H_COUNT: u32 = 20;
const H_TOP: u32 = 24;
const H_CELLS: u32 = 28;
const H_W: u32 = 32;
const H_H: u32 = 36;
const H_CELLCOUNT: u32 = 40;
const H_DIRTY: u32 = 44;
const H_START_X: u32 = 48;
const H_START_Y: u32 = 50;
const H_FOUND_X: u32 = 52;
const H_FOUND_Y: u32 = 54;
const H_GX: u32 = 56;
const H_GY: u32 = 60;
const H_PROBE: u32 = 64;
const H_RANGE: u32 = 68;
const H_SPT: u32 = 72;
const H_STEPS: u32 = 76;
const H_COST: u32 = 80;
const H_BASE: u32 = 84;
const H_OBJECT: u32 = 88;
const H_PATH: u32 = 92;
const H_TARGET: u32 = 96;
const H_OWNER: u32 = 100;
const H_TURN: u32 = 104;
const H_STEPC: u32 = 112;
const H_PLAYER: u32 = 120;
const H_CURSOR: u32 = 121;
const H_BUDGET: u32 = 161;
// Settings
const S_NODELIM: u32 = 208;
const S_GOALCAP: u32 = 212;
const S_REQCAP: u32 = 216;
const S_REQCOUNT: u32 = 220;
const S_FIRSTFREE: u32 = 224;
const S_MASK: u32 = 228;
const S_EVCAP: u32 = 232;
const S_EVCOUNT: u32 = 236;
const S_EVNEXT: u32 = 240;
const S_SEENW: u32 = 244;
const S_SEENH: u32 = 248;
const S_FPX: u32 = 252;
const S_FPY: u32 = 256;
const S_OFF: u32 = 260; // eight offsets
const S_GOAL: u32 = 292;
const S_CAP: u32 = 324;
const S_LOAD: u32 = 364;
const S_REFRESH: u32 = 404;

const POOL_OFF: u32 = 408;
const EV_SIZE: u32 = 284;
const DX: [i32; 8] = [0, -1, -1, -1, 0, 1, 1, 1];
const DY: [i32; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];

#[inline(always)]
fn r32(a: u32) -> i32 {
    unsafe { read_unaligned(a as *const i32) }
}
#[inline(always)]
fn w32(a: u32, v: i32) {
    unsafe { write_unaligned(a as *mut i32, v) }
}
#[inline(always)]
fn r16(a: u32) -> i32 {
    unsafe { read_unaligned(a as *const i16) as i32 }
}
#[inline(always)]
fn ru16(a: u32) -> u32 {
    unsafe { read_unaligned(a as *const u16) as u32 }
}
#[inline(always)]
fn w16(a: u32, v: i32) {
    unsafe { write_unaligned(a as *mut u16, v as u16) }
}
#[inline(always)]
fn r8(a: u32) -> u32 {
    unsafe { *(a as *const u8) as u32 }
}
#[inline(always)]
fn w8(a: u32, v: u32) {
    unsafe { *(a as *mut u8) = v as u8 }
}
#[inline(always)]
fn game() -> u32 {
    unsafe { read_volatile(&raw const g_game) }
}
#[inline(always)]
fn block_base() -> u32 {
    unsafe { read_volatile(&raw const g_blockBase) }
}
#[inline(always)]
fn region() -> u32 {
    unsafe { read_volatile(&raw const g_region) }
}

#[inline(always)]
fn bucket(v: u32, mask: u32) -> u32 {
    (v.wrapping_mul(0x9E3779B1) >> 16) & mask
}

struct Pf {
    b: u32,
}

#[derive(Clone, Copy)]
struct Walker {
    x: i32,
    y: i32,
    d: i32,
}

impl Pf {
    #[inline(always)]
    fn g(&self, off: u32) -> i32 {
        r32(self.b + off)
    }
    #[inline(always)]
    fn s(&self, off: u32, v: i32) {
        w32(self.b + off, v)
    }
    #[inline(always)]
    fn rbase(&self) -> u32 {
        self.b.wrapping_sub(block_base())
    }
    #[inline(always)]
    fn off(&self, i: u32) -> u32 {
        self.b + r32(self.b + S_OFF + 4 * i) as u32
    }

    // ---- cells ----
    fn cell_addr(&self, x: i32, y: i32) -> u32 {
        let w = self.g(H_W);
        self.off(2).wrapping_add((w.wrapping_mul(y).wrapping_add(x) as u32).wrapping_mul(4))
    }
    fn dirty_idx(&self, i: i32) {
        let i = i as u32;
        let a = self.off(3) + 4 * (i / 256);
        w32(a, r32(a) | (1i32 << ((i / 8) % 32)));
    }
    fn on_grid(&self, x: i32, y: i32) -> bool {
        (x as u32) < (self.g(H_W) as u32) && (y as u32) < (self.g(H_H) as u32)
    }
    fn cell_index(&self, x: i32, y: i32) -> i32 {
        self.g(H_W).wrapping_mul(y).wrapping_add(x)
    }

    // ---- goal area ----
    fn goal_dist(&self, x: i32, y: i32) -> i32 {
        let a = self.b + S_GOAL;
        let kind = r32(a);
        match kind {
            1 | 2 => {
                let cx = r16(a + 4);
                let cy = r16(a + 6);
                let dx = (x.wrapping_sub(cx)).wrapping_abs();
                let dy = (y.wrapping_sub(cy)).wrapping_abs();
                let (hi, lo) = if dx > dy { (dx, dy) } else { (dy, dx) };
                let e = hi.wrapping_mul(18).wrapping_add(lo.wrapping_mul(7));
                let r0 = r32(a + 8);
                if kind == 1 {
                    if e < r0 { 0 } else { e.wrapping_sub(r0) }
                } else {
                    let r1 = r32(a + 12);
                    if e > r1 {
                        e.wrapping_sub(r1)
                    } else if e < r0 {
                        r0.wrapping_sub(e)
                    } else {
                        0
                    }
                }
            }
            3 => {
                let left = r32(a + 16);
                let right = r32(a + 20);
                let top = r32(a + 24);
                let bottom = r32(a + 28);
                let ox = if x < left { left.wrapping_sub(x) } else if x > right { x.wrapping_sub(right) } else { 0 };
                let oy = if y < top { top.wrapping_sub(y) } else if y > bottom { y.wrapping_sub(bottom) } else { 0 };
                if ox == 0 && oy == 0 {
                    let mut m = x.wrapping_sub(left);
                    let c = [right.wrapping_sub(x), y.wrapping_sub(top), bottom.wrapping_sub(y)];
                    for v in c {
                        if v < m {
                            m = v;
                        }
                    }
                    m.wrapping_mul(16)
                } else if ox == 0 {
                    oy.wrapping_mul(16)
                } else if oy == 0 {
                    ox.wrapping_mul(16)
                } else {
                    let (hi, lo) = if ox > oy { (ox, oy) } else { (oy, ox) };
                    hi.wrapping_mul(16).wrapping_add(lo.wrapping_mul(6))
                }
            }
            _ => 0,
        }
    }
    fn in_goal(&self, x: i32, y: i32) -> bool {
        let a = self.b + S_GOAL;
        let kind = r32(a);
        match kind {
            1 | 2 => {
                let cx = r16(a + 4);
                let cy = r16(a + 6);
                let dx = x.wrapping_sub(cx);
                let dy = y.wrapping_sub(cy);
                let d2 = dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy));
                let q0 = (r32(a + 8) / 16).wrapping_mul(r32(a + 8) / 16);
                if kind == 1 {
                    d2 <= q0
                } else {
                    let q1 = (r32(a + 12) / 16).wrapping_mul(r32(a + 12) / 16);
                    q0 <= d2 && d2 <= q1
                }
            }
            3 => {
                let left = r32(a + 16);
                let right = r32(a + 20);
                let top = r32(a + 24);
                let bottom = r32(a + 28);
                ((x == left || x == right) && top <= y && y <= bottom) || ((y == top || y == bottom) && left <= x && x <= right)
            }
            _ => false,
        }
    }
    fn est(&self, x: i32, y: i32) -> i32 {
        let d = self.goal_dist(x, y);
        (((self.g(H_COST) as i64) * (d as i64)) >> 16) as i32
    }

    // Calls f on each goal cell in order, up to the capacity.
    #[allow(unused_assignments)]
    fn goal_cells(&self, mut f: impl FnMut(i32, i32)) {
        let a = self.b + S_GOAL;
        let cap = self.g(S_GOALCAP);
        let mut n: i32 = 0;
        let kind = r32(a);
        macro_rules! emit {
            ($x:expr, $y:expr) => {
                if n >= cap {
                    return;
                }
                n += 1;
                f(($x) as i16 as i32, ($y) as i16 as i32);
            };
        }
        match kind {
            1 => {
                let cx = r16(a + 4);
                let cy = r16(a + 6);
                emit!(cx, cy);
            }
            2 => {
                let cx = r16(a + 4);
                let cy = r16(a + 6);
                let k = r32(a + 12).wrapping_add(r32(a + 8)) / 32;
                emit!(cx, cy.wrapping_add(k));
            }
            3 => {
                let left = r32(a + 16);
                let right = r32(a + 20);
                let top = r32(a + 24);
                let bottom = r32(a + 28);
                let mut i = left;
                while i <= right {
                    emit!(i, top);
                    emit!(i, bottom);
                    if i == i32::MAX {
                        break;
                    }
                    i += 1;
                }
                let mut j = top.wrapping_add(1);
                while j < bottom {
                    emit!(left, j);
                    emit!(right, j);
                    j += 1;
                }
            }
            _ => {}
        }
    }

    // ---- cell state ----
    fn cell_state(&self, x: i32, y: i32) -> u32 {
        let owner = self.g(H_OWNER) as u32;
        unsafe {
            if w_in_bounds(owner, x, y) == 0 {
                return 0;
            }
            let vx = (x >> 1).wrapping_add(self.g(S_FPX) >> 2);
            let vy = (y >> 1).wrapping_add(self.g(S_FPY) >> 2);
            if vx >= self.g(S_SEENW) || vy >= self.g(S_SEENH) {
                return 0;
            }
            if w_seen(r8(self.b + H_PLAYER) as i32, vx, vy) == 0 {
                return 2;
            }
            w_pass(owner, x, y)
        }
    }

    // ---- events ----
    fn add_event(&self, kind: i32, status: i32) -> Option<u32> {
        let n = self.g(S_EVCOUNT);
        if n >= self.g(S_EVCAP) {
            return None;
        }
        let rec = self.off(7) + (n as u32) * EV_SIZE;
        w32(rec, kind);
        w32(rec + 4, status);
        w32(rec + 8, self.g(H_OBJECT));
        w32(rec + 12, self.g(H_PATH));
        w32(rec + 16, self.g(H_TARGET));
        w32(rec + 20, unsafe { w_tick() } as i32);
        w32(rec + 24, 0);
        self.s(S_EVCOUNT, n + 1);
        Some(rec)
    }
    fn end_search(&self) {
        unsafe { w_end_search(self.g(H_OBJECT) as u32) };
        self.s(H_OBJECT, 0);
        self.s(H_OWNER, 0);
    }
    fn fail(&self) {
        self.add_event(2, 2);
        self.end_search();
    }

    // ---- open list ----
    fn node(&self, k: u32) -> u32 {
        self.b + POOL_OFF + 20 * k
    }
    fn item_addr(&self, i: i32) -> u32 {
        self.b + POOL_OFF + 20 * (self.g(S_NODELIM) as u32) + 4 * (i as u32)
    }
    fn item_node(&self, i: i32) -> u32 {
        let v = r32(self.item_addr(i)) as u32;
        v.wrapping_sub(self.rbase()).wrapping_sub(POOL_OFF) / 20
    }
    fn place(&self, k: u32, i: i32) {
        w32(self.item_addr(i), self.rbase().wrapping_add(POOL_OFF).wrapping_add(20 * k) as i32);
        w32(self.node(k), i);
    }
    fn f_of(&self, k: u32) -> i32 {
        r32(self.node(k) + 12)
    }
    fn sift_up(&self, mut i: i32) {
        if i == 0 {
            return;
        }
        let n = self.item_node(i);
        let nf = self.f_of(n);
        while i > 0 {
            let p = (i - 1) / 2;
            let pn = self.item_node(p);
            if nf < self.f_of(pn) {
                self.place(pn, i);
                i = p;
            } else {
                break;
            }
        }
        self.place(n, i);
    }
    fn sift_down(&self, mut i: i32) {
        let n = self.item_node(i);
        let nf = self.f_of(n);
        let count = self.g(H_COUNT);
        loop {
            let l = 2 * i + 1;
            let c;
            if 2 * i + 2 < count {
                let rn = self.item_node(2 * i + 2);
                let ln = self.item_node(l);
                c = if self.f_of(rn) < self.f_of(ln) { 2 * i + 2 } else { l };
            } else if l < count {
                c = l;
            } else {
                break;
            }
            let cn = self.item_node(c);
            if self.f_of(cn) >= nf {
                break;
            }
            self.place(cn, i);
            i = c;
        }
        self.place(n, i);
    }
    fn remove(&self, k: u32) {
        let j = r32(self.node(k));
        w32(self.node(k), self.g(H_FREE));
        self.s(H_FREE, k as i32);
        let count = self.g(H_COUNT) - 1;
        self.s(H_COUNT, count);
        if j < count {
            let m = self.item_node(count);
            self.place(m, j);
            self.sift_down(j);
        }
    }
    fn push(&self, x: i32, y: i32, g: i32, f: i32, pen: i32, steps: i32) -> u32 {
        let set = |k: u32| {
            let n = self.node(k);
            w16(n + 4, x);
            w16(n + 6, y);
            w32(n + 8, g);
            w32(n + 12, f);
            w16(n + 16, pen);
            w16(n + 18, steps);
        };
        if self.g(H_TOP) == 1 {
            let k = self.item_node(0);
            set(k);
            self.sift_down(0);
            self.s(H_TOP, 0);
            return k;
        }
        let mut cap = self.g(H_CAP);
        let count = self.g(H_COUNT);
        if count == cap {
            let lim = self.g(S_NODELIM);
            let nc = cap.wrapping_add(cap / 2).wrapping_add(16);
            cap = if nc < lim { nc } else { lim };
            self.s(H_CAP, cap);
            if self.g(H_POOL) == 0 {
                self.s(H_POOL, self.rbase().wrapping_add(POOL_OFF) as i32);
                self.s(H_ITEMS, self.rbase().wrapping_add(POOL_OFF + 20 * lim as u32) as i32);
            }
            if count == cap {
                return 0;
            }
        }
        let fh = self.g(H_FREE);
        let k = if fh != -1 {
            self.s(H_FREE, r32(self.node(fh as u32)));
            fh as u32
        } else {
            let u = self.g(H_USED);
            self.s(H_USED, u + 1);
            u as u32
        };
        set(k);
        self.place(k, count);
        self.s(H_COUNT, count + 1);
        self.sift_up(count);
        k
    }
    fn lower(&self, k: u32) {
        if self.g(H_TOP) == 0 {
            self.sift_up(r32(self.node(k)));
        } else {
            let t = self.item_node(0);
            self.sift_up(r32(self.node(k)));
            if r32(self.node(t)) != 0 {
                self.s(H_TOP, 0);
                self.remove(t);
            }
        }
    }
    fn take(&self) -> u32 {
        if self.g(H_TOP) == 1 {
            self.s(H_TOP, 0);
            let k = self.item_node(0);
            self.remove(k);
        }
        let k = self.item_node(0);
        self.s(H_TOP, 1);
        k
    }
    fn size(&self) -> i32 {
        self.g(H_COUNT) - self.g(H_TOP)
    }

    // ---- probe ----
    fn visit(&self, x: i32, y: i32, d: i32) -> bool {
        let i = self.cell_index(x, y);
        self.dirty_idx(i);
        let a = self.cell_addr(x, y);
        w8(a + 1, d as u32);
        let f = r8(a) | 8;
        w8(a, f);
        f & 4 != 0
    }
    fn turn(&self) {
        self.s(H_STEPS, self.g(H_STEPS) + 1);
    }
    fn on_line(&self, x: i32, y: i32, nx: i32, ny: i32) -> bool {
        let mut gx = self.g(H_GX).wrapping_sub(x);
        let mut gy = self.g(H_GY).wrapping_sub(y);
        let mut px = nx.wrapping_sub(x);
        let mut py = ny.wrapping_sub(y);
        if gx < 0 {
            gx = gx.wrapping_neg();
            px = px.wrapping_neg();
        }
        if gy < 0 {
            gy = gy.wrapping_neg();
            py = py.wrapping_neg();
        }
        (py == 0 && 0 < px && px <= gx) || (px == gx && 0 < py && py <= gy)
    }
    fn probe(&self) -> i32 {
        let mut x = r16(self.b + H_START_X);
        let mut y = r16(self.b + H_START_Y);
        let mut best = self.est(x, y);
        if self.cell_state(x, y) == 0 {
            return best;
        }
        loop {
            let d;
            loop {
                self.turn();
                if best == 0 {
                    return 0;
                }
                let gx = self.g(H_GX);
                let gy = self.g(H_GY);
                let dd = if gx < x {
                    2
                } else if gx > x {
                    6
                } else if gy > y {
                    4
                } else {
                    0
                };
                let nx = x + DX[dd];
                let ny = y + DY[dd];
                if self.cell_state(nx, ny) == 0 {
                    d = dd as i32;
                    break;
                }
                x = nx;
                y = ny;
                if self.visit(x, y, dd as i32) {
                    return 0;
                }
                let h = self.est(x, y);
                if h < best {
                    best = h;
                }
            }
            let mut a = Walker { x, y, d: (d + 2) & 7 };
            let mut b = a;
            let mut started = false;
            'wall: loop {
                self.turn();
                let stop = (a.d - 3) & 7;
                a.d = (a.d - 2) & 7;
                while self.cell_state(a.x + DX[a.d as usize], a.y + DY[a.d as usize]) == 0 {
                    if a.d == stop {
                        return best;
                    }
                    a.d = (a.d + 1) & 7;
                }
                if a.x == b.x && a.y == b.y && a.d == b.d && started {
                    return best;
                }
                a.x += DX[a.d as usize];
                a.y += DY[a.d as usize];
                started = true;
                if self.visit(a.x, a.y, a.d) {
                    return 0;
                }
                if self.on_line(x, y, a.x, a.y) {
                    x = a.x;
                    y = a.y;
                    break 'wall;
                }
                let h = self.est(a.x, a.y);
                if h < best {
                    best = h;
                }
                let stop = (b.d + 3) & 7;
                b.d = (b.d + 2) & 7;
                while self.cell_state(b.x - DX[b.d as usize], b.y - DY[b.d as usize]) == 0 {
                    if b.d == stop {
                        return best;
                    }
                    b.d = (b.d - 1) & 7;
                }
                if a.x == b.x && a.y == b.y && a.d == b.d {
                    return best;
                }
                b.x -= DX[b.d as usize];
                b.y -= DY[b.d as usize];
                if self.visit(b.x, b.y, b.d) {
                    return 0;
                }
                if self.on_line(x, y, b.x, b.y) {
                    x = b.x;
                    y = b.y;
                    break 'wall;
                }
                let h = self.est(b.x, b.y);
                if h < best {
                    best = h;
                }
            }
        }
    }

    // ---- expansion ----
    fn expand(&self, x: i32, y: i32, g: i32, s: i32) {
        let c = self.cell_addr(x, y);
        w8(c, 2);
        let range = self.g(H_RANGE);
        let probe = self.g(H_PROBE);
        let mut t = -range;
        while t <= range {
            let u = (t & 7) as u32;
            let e = ((r8(c + 1) + u) & 7) as usize;
            let nx = x + DX[e];
            let ny = y + DY[e];
            t += 1;
            if !self.on_grid(nx, ny) {
                continue;
            }
            let mut add = (r8(self.b + H_TURN + u) as i32) + (r8(self.b + H_STEPC + e as u32) as i32);
            if u != 0 && s < 5 {
                add += 75;
            }
            let na = self.cell_addr(nx, ny);
            let fl = r8(na);
            match fl & 3 {
                0 => {
                    self.dirty_idx(self.cell_index(nx, ny));
                    let r = self.cell_state(nx, ny);
                    if r == 0 && fl & 8 == 0 {
                        w8(na, fl | 3);
                    } else {
                        let h = self.est(nx, ny);
                        w8(na, fl | if h <= probe { 5 } else { 1 });
                        w8(na + 1, e as u32);
                        let p = if r > 1 { 0 } else { 30 };
                        let g2 = g.wrapping_add(add).wrapping_add(p);
                        let st = if u != 0 { 1 } else { s + 1 };
                        let k = self.push(nx, ny, g2, g2.wrapping_add(h), p, st);
                        w16(na + 2, k as i32);
                    }
                }
                1 => {
                    let k = ru16(na + 2);
                    let n = self.node(k);
                    let g2 = g.wrapping_add(add).wrapping_add(r16(n + 16));
                    let ng = r32(n + 8);
                    if g2 < ng {
                        w8(na + 1, e as u32);
                        w32(n + 12, r32(n + 12).wrapping_add(g2.wrapping_sub(ng)));
                        w32(n + 8, g2);
                        w16(n + 18, if u != 0 { 1 } else { s + 1 });
                        self.lower(k);
                    }
                }
                _ => {}
            }
        }
    }

    // ---- route ----
    fn route(&self, fx: i32, fy: i32) {
        w16(self.b + H_FOUND_X, fx);
        w16(self.b + H_FOUND_Y, fy);
        let sx = r16(self.b + H_START_X);
        let sy = r16(self.b + H_START_Y);
        let mut ring = [0i32; 128];
        let mut n: u32 = 0;
        let mut dir = r8(self.cell_addr(fx, fy) + 1) as i8 as i32;
        ring[0] = fx;
        ring[1] = fy;
        n += 1;
        let (mut kx, mut ky) = (fx, fy);
        while kx != sx || ky != sy {
            let d = r8(self.cell_addr(kx, ky) + 1) as i8 as i32;
            if d != dir {
                dir = d;
                let i = ((n & 63) * 2) as usize;
                ring[i] = kx;
                ring[i + 1] = ky;
                n += 1;
            }
            kx -= DX[(dir & 7) as usize];
            ky -= DY[(dir & 7) as usize];
        }
        let i = ((n & 63) * 2) as usize;
        ring[i] = sx;
        ring[i + 1] = sy;
        n += 1;
        let m = if n > 64 { 64 } else { n };
        if let Some(rec) = self.add_event(2, if n > 64 { 1 } else { 0 }) {
            w32(rec + 24, m as i32);
            let fpx = self.g(S_FPX);
            let fpy = self.g(S_FPY);
            for i in 0..m {
                let e = (n - 1 - i) & 63;
                let x = ring[(e * 2) as usize];
                let y = ring[(e * 2 + 1) as usize];
                let p = rec + 28 + 4 * i;
                w16(p, (2 * x + fpx) * 8);
                w16(p + 2, (2 * y + fpy) * 8);
            }
        }
        self.end_search();
    }

    // ---- requests ----
    fn entry(&self, e: u32) -> u32 {
        self.off(4) + 56 * (e - 1)
    }
    fn find_by_requester(&self, q: u32) -> u32 {
        let mask = self.g(S_MASK) as u32;
        let mut e = r32(self.off(5) + 4 * bucket(q, mask)) as u32;
        while e != 0 {
            let a = self.entry(e);
            if r32(a) as u32 == q {
                return e;
            }
            e = r32(a + 48) as u32;
        }
        0
    }
    fn find_by_unit(&self, u: u32) -> u32 {
        let mask = self.g(S_MASK) as u32;
        let mut e = r32(self.off(6) + 4 * bucket(u, mask)) as u32;
        while e != 0 {
            let a = self.entry(e);
            if r32(a + 4) as u32 == u {
                return e;
            }
            e = r32(a + 52) as u32;
        }
        0
    }
    // Unlinks e from the chain starting at the word `head`, whose links are at `link` in an entry.
    fn unlink(&self, head: u32, link: u32, e: u32) {
        let next = r32(self.entry(e) + link);
        if r32(head) as u32 == e {
            w32(head, next);
            return;
        }
        let mut cur = r32(head) as u32;
        while cur != 0 {
            let a = self.entry(cur) + link;
            if r32(a) as u32 == e {
                w32(a, next);
                return;
            }
            cur = r32(a) as u32;
        }
    }
    fn unlink_unit(&self, e: u32) {
        let mask = self.g(S_MASK) as u32;
        let u = r32(self.entry(e) + 4) as u32;
        self.unlink(self.off(6) + 4 * bucket(u, mask), 52, e);
    }
    fn remove_entry(&self, e: u32) {
        let mask = self.g(S_MASK) as u32;
        let a = self.entry(e);
        let q = r32(a) as u32;
        self.unlink(self.off(5) + 4 * bucket(q, mask), 48, e);
        self.unlink_unit(e);
        w32(a, 0);
        w32(a + 48, self.g(S_FIRSTFREE));
        self.s(S_FIRSTFREE, e as i32);
        self.s(S_REQCOUNT, self.g(S_REQCOUNT) - 1);
    }

    // ---- search start ----
    fn start_search(&self, goal: u32, state: u32) {
        self.s(H_OWNER, r32(state + 12));
        for i in 0..8 {
            w32(self.b + S_GOAL + 4 * i, r32(goal + 4 * i));
        }
        self.s(S_FPX, r32(state + 16));
        self.s(S_FPY, r32(state + 20));
        let mut sw = 0i32;
        let mut sh = 0i32;
        unsafe { w_seen_size(&mut sw, &mut sh) };
        self.s(S_SEENW, sw);
        self.s(S_SEENH, sh);
        let sx = r16(state + 4);
        let sy = r16(state + 6);
        w16(self.b + H_START_X, sx);
        w16(self.b + H_START_Y, sy);
        unsafe { w_begin_search(self.g(H_OBJECT) as u32) };
        let tc = [0u8, 40, 60, 80, 100, 80, 60, 40];
        let sc = [16u8, 22, 16, 22, 16, 22, 16, 22];
        for i in 0..8 {
            w8(self.b + H_TURN + i as u32, tc[i] as u32);
            w8(self.b + H_STEPC + i as u32, sc[i] as u32);
        }
        // clear flags
        let words = ((self.g(H_CELLCOUNT) as u32).wrapping_add(255) / 256) as u32;
        let dirty = self.off(3);
        let cells = self.off(2);
        for j in 0..words {
            let a = dirty + 4 * j;
            let v = r32(a) as u32;
            if v != 0 {
                w32(a, 0);
                for bit in 0..32u32 {
                    if v & (1 << bit) != 0 {
                        let first = 256 * j + 8 * bit;
                        for c in 0..8 {
                            w8(cells + 4 * (first + c), 0);
                        }
                    }
                }
            }
        }
        // goal cells
        let mut best_d: Option<i32> = None;
        self.goal_cells(|x, y| {
            if self.on_grid(x, y) {
                w8(self.cell_addr(x, y), 4);
                self.dirty_idx(self.cell_index(x, y));
            }
            let dx = sx.wrapping_sub(x);
            let dy = sy.wrapping_sub(y);
            let d = dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy));
            let better = match best_d {
                None => true,
                Some(b) => d < b,
            };
            if better {
                best_d = Some(d);
                self.s(H_GX, x);
                self.s(H_GY, y);
            }
        });
        let h0 = self.est(sx, sy);
        let mut open = false;
        if self.in_goal(sx, sy) {
            self.add_event(0, 0);
            self.fail();
        } else if !self.on_grid(sx, sy) {
            self.add_event(0, 1);
            self.fail();
        } else {
            let v = self.probe();
            self.s(H_PROBE, v);
            if v == 0 {
                self.add_event(0, 0);
                open = true;
            } else if v >= h0 {
                self.add_event(0, 1);
                self.fail();
            } else {
                self.add_event(0, 1);
                open = true;
            }
        }
        if open {
            self.s(H_COUNT, 0);
            self.s(H_USED, 0);
            self.s(H_TOP, 0);
            self.s(H_FREE, -1);
            self.dirty_idx(self.cell_index(sx, sy));
            let a = self.cell_addr(sx, sy);
            w8(a, r8(a) | 1);
            let hd = ru16(state + 8);
            w8(a + 1, ((hd + 0x1000) & 0xFFFF) / 8192);
            let k = self.push(sx, sy, 0, h0, 0, 100);
            w16(a + 2, k as i32);
            self.s(H_RANGE, 4);
        }
    }

    fn idle_turn(&self) {
        self.s(H_STEPS, 1);
        let mut player = r8(self.b + H_PLAYER);
        while r32(self.b + H_BUDGET + 4 * player) <= 0 {
            player = if player == 9 { 0 } else { player + 1 };
        }
        w8(self.b + H_PLAYER, player);
        let la = self.b + S_LOAD + 4 * player;
        w32(la, r32(la) + 1);
        let ca = self.b + H_CURSOR + 4 * player;
        let cur = r32(ca) as u32;
        let p = player as i32;
        let last = unsafe { w_last_unit(p) };
        let u = if cur == last { unsafe { w_first_unit(p) } } else { unsafe { w_next_unit(cur) } };
        w32(ca, u as i32);
        let e = self.find_by_unit(u);
        if e == 0 {
            return;
        }
        let a = self.entry(e);
        if unsafe { w_tick() } < (r32(a + 12) as u32) {
            return;
        }
        let mut state = [0u32; 6];
        let sp = state.as_mut_ptr() as *mut u8;
        if unsafe { w_unit_state(u, sp) } == 0 {
            return;
        }
        // copy the entry
        let mut copy = [0i32; 8];
        for i in 0..8 {
            copy[i] = r32(a + 16 + 4 * i as u32);
        }
        let requester = r32(a);
        let target = r32(a + 8);
        self.remove_entry(e);
        self.s(H_PATH, requester);
        self.s(H_OBJECT, u as i32);
        self.s(H_STEPS, self.g(H_STEPS) + 100);
        self.s(H_COST, r32(self.b + S_CAP + 4 * player));
        self.s(H_TARGET, target);
        self.start_search(copy.as_ptr() as u32, sp as u32);
    }

    fn search_turn(&self) {
        while self.size() > 0 {
            self.s(H_STEPS, self.g(H_STEPS) + 1);
            let k = self.take();
            let n = self.node(k);
            let x = r16(n + 4);
            let y = r16(n + 6);
            let g = r32(n + 8);
            let s = r16(n + 18);
            if r8(self.cell_addr(x, y)) & 4 != 0 {
                self.route(x, y);
                return;
            }
            self.expand(x, y, g, s);
            self.s(H_RANGE, 2);
            if self.g(H_STEPS) >= 100 {
                return;
            }
        }
    }
}

fn pf() -> Option<Pf> {
    if r32(game() + GAME_PATHFINDER) == 0 {
        None
    } else {
        Some(Pf { b: region() })
    }
}

fn layout(w: i32, h: i32, s: i32) -> [u32; 12] {
    // 0 N, 1 C, 2 P, 3 B, 4 pool, 5 items, 6 cells, 7 dirty, 8 requests, 9 reqbuckets, 10 unitbuckets, 11 events; size in [12]
    let c = (w.wrapping_mul(h) as u32).wrapping_add(7) & !7;
    let n = if c + 1 < 65536 { c + 1 } else { 65536 };
    let p = (s as u32).wrapping_add(16);
    let mut b = 16;
    while b < p && b < 0x8000_0000 {
        b *= 2;
    }
    let items = POOL_OFF + 20 * n;
    let cells = items + 4 * n;
    let dirty = cells + 4 * c;
    let req = dirty + 4 * ((c + 255) / 256);
    let rb = req + 56 * p;
    let ub = rb + 4 * b;
    let ev = ub + 4 * b;
    [n, c, p, b, POOL_OFF, items, cells, dirty, req, rb, ub, ev]
}

#[no_mangle]
pub extern "C" fn RegionSize(map: u32) -> u32 {
    let l = layout(r32(map), r32(map + 4), r32(map + 8));
    l[11] + EV_SIZE * 512
}

#[no_mangle]
pub extern "C" fn Init(map: u32, reg: u32, _size: u32) {
    let (w, h, s) = (r32(map), r32(map + 4), r32(map + 8));
    let l = layout(w, h, s);
    let p = Pf { b: reg };
    let rbase = reg.wrapping_sub(block_base());
    p.s(S_NODELIM, l[0] as i32);
    p.s(S_GOALCAP, 2 * (w + h) + 256);
    p.s(S_REQCAP, l[2] as i32);
    p.s(S_MASK, (l[3] - 1) as i32);
    p.s(S_EVCAP, 512);
    // offsets: pool, items, cells, dirty, requests, requester buckets, unit buckets, events
    for i in 0..8 {
        p.s(S_OFF + 4 * i, l[4 + i as usize] as i32);
    }
    p.s(S_FIRSTFREE, 1);
    for i in 0..10 {
        p.s(S_CAP + 4 * i, 0x18000);
    }
    for e in 1..l[2] {
        w32(reg + l[8] + 56 * (e - 1) + 48, (e + 1) as i32);
    }
    p.s(H_FREE, -1);
    let gw = r32(game() + GAME_WIDTH);
    let gh = r32(game() + GAME_HEIGHT);
    p.s(H_W, gw);
    p.s(H_H, gh);
    p.s(H_CELLCOUNT, (gw.wrapping_mul(gh).wrapping_add(7)) & !7);
    p.s(H_CELLS, rbase.wrapping_add(l[6]) as i32);
    p.s(H_DIRTY, rbase.wrapping_add(l[7]) as i32);
    let lim = unsafe { w_step_limit() };
    p.s(H_SPT, if lim == 0 { 0x535 } else { lim as i32 });
    p.s(H_BASE, 0x18000);
    for i in 0..10 {
        w32(reg + H_CURSOR + 4 * i, unsafe { w_first_unit(i as i32) } as i32);
    }
    w32(game() + GAME_PATHFINDER, rbase as i32);
    // keep the imports no slot calls
    // The imports no slot needs sit behind a test that never holds; the table entry keeps the table import.
    unsafe {
        if read_volatile(&NEVER) != 0 {
            let f = read_volatile(&TABLE[0]);
            f();
            let c = w_move_class(0);
            w_los(c, 0, 0, 0, 0);
            w_terrain_pass(c, 0, 0);
        }
    }
}

#[no_mangle]
pub extern "C" fn Shutdown() -> u32 {
    let a = game() + GAME_PATHFINDER;
    let v = r32(a) as u32;
    w32(a, 0);
    if v == 0 { 0 } else { block_base().wrapping_add(v) }
}

#[no_mangle]
pub extern "C" fn Submit(req: u32) {
    let p = match pf() {
        Some(p) => p,
        None => return,
    };
    let q = r32(req + 40) as u32;
    if p.g(H_OBJECT) != 0 && p.g(H_PATH) as u32 == q {
        return;
    }
    let mask = p.g(S_MASK) as u32;
    let unit = r32(req) as u32;
    let mut e = p.find_by_requester(q);
    if e != 0 {
        let a = p.entry(e);
        if r32(a + 4) as u32 != unit {
            p.unlink_unit(e);
            w32(a + 4, unit as i32);
            let ub = p.off(6) + 4 * bucket(unit, mask);
            w32(a + 52, r32(ub));
            w32(ub, e as i32);
        }
    } else {
        if p.g(S_FIRSTFREE) == 0 {
            return;
        }
        e = p.g(S_FIRSTFREE) as u32;
        let a = p.entry(e);
        p.s(S_FIRSTFREE, r32(a + 48));
        p.s(S_REQCOUNT, p.g(S_REQCOUNT) + 1);
        w32(a, q as i32);
        w32(a + 4, unit as i32);
        let rb = p.off(5) + 4 * bucket(q, mask);
        w32(a + 48, r32(rb));
        w32(rb, e as i32);
        let ub = p.off(6) + 4 * bucket(unit, mask);
        w32(a + 52, r32(ub));
        w32(ub, e as i32);
    }
    let a = p.entry(e);
    w32(a + 8, r32(req + 44));
    w32(a + 12, r32(req + 4));
    for i in 0..8 {
        w32(a + 16 + 4 * i, r32(req + 8 + 4 * i));
    }
}

#[no_mangle]
pub extern "C" fn Cancel(requester: u32) {
    let p = match pf() {
        Some(p) => p,
        None => return,
    };
    if p.g(H_PATH) as u32 == requester {
        p.s(H_OBJECT, 0);
        p.s(H_PATH, 0);
        p.s(H_TARGET, 0);
        p.s(H_OWNER, 0);
    }
    let e = p.find_by_requester(requester);
    if e != 0 {
        p.remove_entry(e);
    }
    let next = p.g(S_EVNEXT);
    let count = p.g(S_EVCOUNT);
    let evs = p.off(7);
    let mut w = next;
    let mut i = next;
    while i < count {
        let rec = evs + (i as u32) * EV_SIZE;
        if r32(rec + 12) as u32 != requester {
            if i != w {
                let dst = evs + (w as u32) * EV_SIZE;
                unsafe { core::ptr::copy(rec as *const u8, dst as *mut u8, EV_SIZE as usize) };
            }
            w += 1;
        }
        i += 1;
    }
    p.s(S_EVCOUNT, w);
}

#[no_mangle]
pub extern "C" fn CellsChanged(_l: i32, _t: i32, _r: i32, _b: i32, _c: i32, _cause: i32) {}

#[no_mangle]
pub extern "C" fn Step() {
    let p = match pf() {
        Some(p) => p,
        None => return,
    };
    p.s(S_EVCOUNT, 0);
    p.s(S_EVNEXT, 0);
    let players = unsafe { w_player_count() };
    if players == 0 {
        return;
    }
    let lim = unsafe { w_step_limit() };
    let spt = if lim == 0 { 0x535 } else { lim as i32 };
    p.s(H_SPT, spt);
    let share = spt.wrapping_div(players);
    let rc = p.g(S_REFRESH) + 1;
    p.s(S_REFRESH, rc);
    if rc >= 150 {
        p.s(S_REFRESH, 0);
        for i in 0..10u32 {
            let v = p.g(S_LOAD + 4 * i).wrapping_div(unsafe { w_units_per_player() });
            let base = p.g(H_BASE);
            let c = if v < 1 {
                base.wrapping_mul(6)
            } else if v < 2 {
                base.wrapping_mul(3)
            } else {
                base
            };
            p.s(S_CAP + 4 * i, c);
            p.s(S_LOAD + 4 * i, 0);
        }
    }
    let mut total: i32 = 0;
    for i in 0..10u32 {
        if unsafe { w_is_playing(i as i32) } != 0 {
            let a = p.b + H_BUDGET + 4 * i;
            let v = r32(a).wrapping_add(share);
            w32(a, v);
            total = total.wrapping_add(v);
        }
    }
    while total > 0 && p.g(S_EVCOUNT) + 3 <= p.g(S_EVCAP) {
        p.s(H_STEPS, 0);
        if p.g(H_OBJECT) == 0 {
            p.idle_turn();
        } else if p.size() > 0 {
            p.search_turn();
        } else {
            p.fail();
        }
        let st = p.g(H_STEPS);
        total = total.wrapping_sub(st);
        let a = p.b + H_BUDGET + 4 * r8(p.b + H_PLAYER);
        w32(a, r32(a).wrapping_sub(st));
    }
}

#[no_mangle]
pub extern "C" fn Poll(out: u32, size: u32, points: u32, capacity: i32) -> i32 {
    let p = match pf() {
        Some(p) => p,
        None => return 0,
    };
    let next = p.g(S_EVNEXT);
    if next >= p.g(S_EVCOUNT) {
        return 0;
    }
    let rec = p.off(7) + (next as u32) * EV_SIZE;
    p.s(S_EVNEXT, next + 1);
    let kind = r32(rec);
    let mut pay = [0u8; 28];
    let put = |pay: &mut [u8; 28], at: usize, v: i32| {
        pay[at..at + 4].copy_from_slice(&v.to_le_bytes());
    };
    put(&mut pay, 0, kind);
    put(&mut pay, 4, r32(rec + 8));
    put(&mut pay, 8, r32(rec + 12));
    put(&mut pay, 12, r32(rec + 16));
    put(&mut pay, 16, r32(rec + 20));
    let psize: u32 = match kind {
        0 => {
            put(&mut pay, 20, r32(rec + 4));
            24
        }
        1 => {
            put(&mut pay, 20, r32(rec + 24));
            24
        }
        2 => {
            put(&mut pay, 20, r32(rec + 4));
            put(&mut pay, 24, r32(rec + 24));
            28
        }
        _ => 20,
    };
    let n = if size < psize { size } else { psize };
    for i in 0..n {
        w8(out + i, pay[i as usize] as u32);
    }
    let count = r32(rec + 24);
    let m = if count < capacity { count } else { capacity };
    if m > 0 {
        unsafe { core::ptr::copy_nonoverlapping((rec + 28) as *const u8, points as *mut u8, 4 * m as usize) };
    }
    psize as i32
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}
