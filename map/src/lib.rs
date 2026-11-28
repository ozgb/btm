#![no_std]

use core::ptr::{read_unaligned, read_volatile, write_unaligned};

#[link(wasm_import_module = "los_radar")]
extern "C" {
    #[link_name = "GetMapCell"]
    fn los_get_map_cell(x: i32, y: i32) -> u32;
    #[link_name = "GetMapCellAtPosition"]
    fn los_get_map_cell_at_position(pos: *const u8) -> u32;
}

#[link(wasm_import_module = "features")]
extern "C" {
    #[link_name = "GetCellFeature"]
    fn feat_get_cell_feature(cell: u32) -> i32;
}

#[link(wasm_import_module = "services")]
extern "C" {
    #[link_name = "RandomInt"]
    fn svc_random_int(n: i32) -> i32;
    #[link_name = "SimAlloc"]
    fn svc_alloc(n: i32) -> u32;
    #[link_name = "SimFree"]
    fn svc_free(p: u32);
    #[link_name = "Emit"]
    fn svc_emit(p: *const u32, n: i32);
}

#[no_mangle]
pub static mut g_game: u32 = 1;
#[no_mangle]
pub static mut g_simGlobals: u32 = 1;
#[no_mangle]
pub static mut g_extTdf: u32 = 1;
#[no_mangle]
pub static mut g_blockBase: u32 = 1;

static NEVER: u32 = 0;
fn noop() {}
static TABLE: [fn(); 1] = [noop];

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

// ---------------------------------------------------------------- memory

#[inline(always)]
fn r8(a: u32) -> u32 {
    unsafe { read_unaligned(a as *const u8) as u32 }
}
#[inline(always)]
fn r16(a: u32) -> u32 {
    unsafe { read_unaligned(a as *const u16) as u32 }
}
#[inline(always)]
fn ri16(a: u32) -> i32 {
    unsafe { read_unaligned(a as *const i16) as i32 }
}
#[inline(always)]
fn r32(a: u32) -> u32 {
    unsafe { read_unaligned(a as *const u32) }
}
#[inline(always)]
fn ri32(a: u32) -> i32 {
    unsafe { read_unaligned(a as *const i32) }
}
#[inline(always)]
fn w8(a: u32, v: u32) {
    unsafe { write_unaligned(a as *mut u8, v as u8) }
}
#[inline(always)]
fn w16(a: u32, v: u32) {
    unsafe { write_unaligned(a as *mut u16, v as u16) }
}
#[inline(always)]
fn w32(a: u32, v: u32) {
    unsafe { write_unaligned(a as *mut u32, v) }
}

#[inline(always)]
fn base() -> u32 {
    unsafe { read_volatile(&raw const g_blockBase) }
}
#[inline(always)]
fn game() -> u32 {
    unsafe { read_volatile(&raw const g_game) }
}
#[inline(always)]
fn sim() -> u32 {
    unsafe { read_volatile(&raw const g_simGlobals) }
}

// Game
const G_PLAYER_INDEX: u32 = 10819;
const G_BASE_X: u32 = 82467;
const G_BASE_Y: u32 = 82471;
const G_MAP_W: u32 = 82475;
const G_MAP_H: u32 = 82479;
const G_WIDTH: u32 = 82483;
const G_HEIGHT: u32 = 82487;
const G_FEAT_COUNT: u32 = 82515;
const G_FEATURES: u32 = 82543;
const G_VIS_MASK: u32 = 82547;
const G_SEA: u32 = 82559;
const G_MAP_FLAGS: u32 = 82561;
const G_CELLS: u32 = 82567;
const G_GRID: u32 = 82591;
const G_GRID_W: u32 = 82595;
const G_GRID_H: u32 = 82599;
const G_OVERFLOW: u32 = 82615;
const G_UNITS: u32 = 82775;
const G_UNITS_END: u32 = 82779;
const G_TICKS: u32 = 232007;
const G_PROGRESS: u32 = 232819;

// SimGlobals
const S_PLACE_H: u32 = 48144;
const S_PLACE_METAL: u32 = 48148;
const S_PASS_MAPS: u32 = 48188;
const S_QUIRK_SKIP_YARD: u32 = 49212 + 8;

// Cell
const CELL_SIZE: u32 = 13;
// Unit
const UNIT_SIZE: u32 = 280;
const U_MOTION: u32 = 0;
const U_POS: u32 = 106;
const U_CELL: u32 = 118;
const U_FOOT: u32 = 126;
const U_BUCKET: u32 = 130;
const U_CARRIER: u32 = 134;
const U_CARGO: u32 = 138;
const U_CARGO_NEXT: u32 = 142;
const U_DEF: u32 = 146;
const U_PLAYER: u32 = 150;
const U_ID: u32 = 168;
const U_TRANSPORT_PIECE: u32 = 249;
const U_PLAYER_INDEX: u32 = 255;
const U_COB: u32 = 271;
const U_FLAGS: u32 = 272;
const MOTION_STAMP: u32 = 38;

const F_STRUCT: u32 = 0x2000_0000;
const F_ALIVE: u32 = 0x1000_0000;
const F_PENDING: u32 = 0x0800_0000;
const F_OVERLAP: u32 = 0x0400_0000;

// PassMap
const PM_SIZE: u32 = 32;

struct Env {
    base: u32,
    game: u32,
    cells: u32,
    w: i32,
    h: i32,
    units: u32,
    feats: u32,
    fc: i32,
    sea: i32,
}

#[inline(always)]
fn env() -> Env {
    let b = base();
    let g = game();
    Env {
        base: b,
        game: g,
        cells: b.wrapping_add(r32(g + G_CELLS)),
        w: ri32(g + G_WIDTH),
        h: ri32(g + G_HEIGHT),
        units: b.wrapping_add(r32(g + G_UNITS)),
        feats: b.wrapping_add(r32(g + G_FEATURES)),
        fc: ri32(g + G_FEAT_COUNT),
        sea: r8(g + G_SEA) as i32,
    }
}

impl Env {
    #[inline(always)]
    fn cell(&self, idx: i32) -> u32 {
        self.cells.wrapping_add((idx as u32).wrapping_mul(CELL_SIZE))
    }
    #[inline(always)]
    fn cell_xy(&self, x: i32, y: i32) -> u32 {
        self.cell(y.wrapping_mul(self.w).wrapping_add(x))
    }
    #[inline(always)]
    fn unit_id(&self, id: u32) -> u32 {
        self.units.wrapping_add(id.wrapping_mul(UNIT_SIZE))
    }
    #[inline(always)]
    fn off(&self, off: u32) -> u32 {
        self.base.wrapping_add(off)
    }
    #[inline(always)]
    fn ticks(&self) -> u32 {
        r32(self.game + G_TICKS)
    }
    #[inline(always)]
    fn feat(&self, v: u32) -> u32 {
        self.feats.wrapping_add(v.wrapping_mul(256))
    }

    // The feature record of the cell with index idx, or 0.
    fn feat_rec(&self, idx: i32) -> u32 {
        let v = r16(self.cell(idx) + 8);
        if v < 0xfffb {
            if (v as i32) < self.fc {
                return self.feat(v);
            }
            return 0;
        }
        if v == 0xfffe {
            let c = self.cell(idx);
            let n = idx.wrapping_sub(
                (r8(c + 10) as i32).wrapping_mul(self.w).wrapping_add(r8(c + 11) as i32),
            );
            let v2 = r16(self.cell(n) + 8);
            if v2 < 0xfffb {
                return self.feat(v2);
            }
        }
        0
    }

    fn blocked(&self, idx: i32) -> bool {
        let v = r16(self.cell(idx) + 8);
        if v == 0xffff {
            return false;
        }
        if v < 0xfffb {
            if (v as i32) >= self.fc {
                return true;
            }
            return r16(self.feat(v) + 254) & 0x40 != 0;
        }
        if v != 0xfffe {
            return true;
        }
        let c = self.cell(idx);
        let n = idx.wrapping_sub(
            (r8(c + 10) as i32).wrapping_mul(self.w).wrapping_add(r8(c + 11) as i32),
        );
        let v2 = r16(self.cell(n) + 8);
        if v2 >= 0xfffb {
            return false;
        }
        r16(self.feat(v2) + 254) & 0x40 != 0
    }

    fn stamp_addr(&self, unit: u32) -> u32 {
        let m = r32(unit + U_MOTION);
        if m == 0 {
            0
        } else {
            self.off(m) + MOTION_STAMP
        }
    }
}

// ---------------------------------------------------------------- pass maps

#[derive(Clone, Copy)]
struct Cls {
    fx: i32,
    fz: i32,
    lo: i32,
    hi: i32,
    max_s: u32,
    bad_s: u32,
    max_ws: u32,
    bad_ws: u32,
    window: u32,
    width: u32,
    height: u32,
    cells: u32,
}

fn pm_addr(k: u32) -> u32 {
    sim() + S_PASS_MAPS + k * PM_SIZE
}

fn cls(e: &Env, k: u32) -> Cls {
    let p = pm_addr(k);
    Cls {
        fx: ri16(p),
        fz: ri16(p + 2),
        lo: e.sea - ri16(p + 4),
        hi: e.sea - ri16(p + 6),
        max_s: r8(p + 8),
        bad_s: r8(p + 9),
        max_ws: r8(p + 10),
        bad_ws: r8(p + 11),
        window: r32(p + 28),
        width: r32(p + 16),
        height: r32(p + 20),
        cells: r32(p + 24),
    }
}

// The pass value of the cell (x, y), which is on the map.
fn pass_value(e: &Env, c: &Cls, x: i32, y: i32) -> u32 {
    let idx = y.wrapping_mul(e.w).wrapping_add(x);
    if e.blocked(idx) {
        return 0;
    }
    let ca = e.cell(idx);
    let u = r16(ca);
    if u != 0 {
        let ua = e.unit_id(u);
        let m = r32(ua + U_MOTION);
        if m == 0 {
            return 0;
        }
        if r32(e.off(m) + MOTION_STAMP) < c.window {
            return 0;
        }
    }
    let low = r8(ca + 6) as i32;
    let high = r8(ca + 5) as i32;
    if low < c.lo {
        return 0;
    }
    if high > c.hi {
        return 0;
    }
    let slope = (high.wrapping_sub(low) as u32) & 0xff;
    if low < e.sea {
        if slope > c.bad_ws {
            return if slope <= c.max_ws { 1 } else { 0 };
        }
    } else if slope > c.bad_s {
        return if slope <= c.max_s { 1 } else { 0 };
    }
    3
}

fn pm_write(c: &Cls, x: u32, y: u32, v: u32) {
    let wa = c
        .cells
        .wrapping_add(base())
        .wrapping_add((y >> 4).wrapping_mul(c.width).wrapping_add(x).wrapping_mul(4));
    let sh = 2 * (y & 15);
    let old = r32(wa);
    w32(wa, (old & !(3u32 << sh)) | (v << sh));
}

// The footprint value of class c at (x, y).
fn footprint_value(e: &Env, c: &Cls, x: i32, y: i32) -> u32 {
    let (fx, fz) = (c.fx, c.fz);
    if x < 0 || y < 0 || x.wrapping_add(fx) >= e.w || y.wrapping_add(fz) >= e.h {
        return 0;
    }
    let mut yy = y;
    while yy < y + fz {
        let mut xx = x;
        while xx < x + fx {
            if pass_value(e, c, xx, yy) == 0 {
                return 0;
            }
            xx += 1;
        }
        yy += 1;
    }
    if x >= 1 && y >= 1 && x + fx <= e.w - 2 && y + fz <= e.h - 2 {
        let mut yy = y - 1;
        while yy <= y + fz {
            let mut xx = x - 1;
            while xx <= x + fx {
                if pass_value(e, c, xx, yy) != 3 {
                    return 1;
                }
                xx += 1;
            }
            yy += 1;
        }
        return 3;
    }
    1
}

fn rebuild(k: u32, cx: i32, cy: i32, sx: i32, sy: i32) {
    let e = env();
    let c = cls(&e, k);
    let mut left = cx.wrapping_sub(c.fx);
    let mut top = cy.wrapping_sub(c.fz);
    let mut right = cx.wrapping_add(sx).wrapping_add(1);
    let mut bottom = cy.wrapping_add(sy).wrapping_add(1);
    if left < 0 {
        left = 0;
    }
    if top < 0 {
        top = 0;
    }
    if (right as u32) > c.width {
        right = c.width as i32;
    }
    if (bottom as u32) > c.height {
        bottom = c.height as i32;
    }
    if left < right && top < bottom && c.cells != 0 {
        let mut y = top;
        while y < bottom {
            let mut x = left;
            while x < right {
                let v = footprint_value(&e, &c, x, y);
                pm_write(&c, x as u32, y as u32, v);
                x += 1;
            }
            y += 1;
        }
    }
}

fn report(k: u32, cx: i32, cy: i32, sx: i32, sy: i32, cause: u32) {
    let c = cls(&env(), k);
    let (w, h) = (c.width as i32, c.height as i32);
    let mut left = cx.wrapping_sub(c.fx);
    if left < 0 {
        left = 0;
    }
    let mut top = cy.wrapping_sub(c.fz);
    if top < 0 {
        top = 0;
    }
    let mut right = cx.wrapping_add(sx);
    if right >= w {
        right = w.wrapping_sub(1);
    }
    let mut bottom = cy.wrapping_add(sy);
    if bottom >= h {
        bottom = h.wrapping_sub(1);
    }
    let p: [u32; 8] = [
        176,
        0,
        left as u32,
        top as u32,
        right as u32,
        bottom as u32,
        1u32.wrapping_shl(k),
        cause,
    ];
    unsafe { svc_emit(p.as_ptr(), 32) };
}

fn defined(k: u32) -> bool {
    r32(pm_addr(k) + 12) != 0
}

fn rebuild_report_all(cx: i32, cy: i32, sx: i32, sy: i32, cause: u32) {
    let mut k = 0;
    while k < 32 {
        if defined(k) {
            rebuild(k, cx, cy, sx, sy);
        }
        k += 1;
    }
    k = 0;
    while k < 32 {
        if defined(k) {
            report(k, cx, cy, sx, sy, cause);
        }
        k += 1;
    }
}

fn unit_rect(u: u32) -> (i32, i32, i32, i32) {
    (ri16(u + U_CELL), ri16(u + U_CELL + 2), ri16(u + U_FOOT), ri16(u + U_FOOT + 2))
}

fn stamp_update(e: &Env, u: u32) {
    let sa = e.stamp_addr(u);
    let (x, y, w, h) = unit_rect(u);
    if sa == 0 {
        rebuild_report_all(x, y, w, h, 2);
        return;
    }
    let s = r32(sa);
    w32(sa, e.ticks());
    let mut k = 0;
    while k < 32 {
        if r32(pm_addr(k) + 28) > s {
            rebuild(k, x, y, w, h);
            report(k, x, y, w, h, 2);
        }
        k += 1;
    }
}

fn cell_heights(x: i32, y: i32, w: i32, h: i32) {
    let p: [u32; 5] = [
        132,
        (x.wrapping_sub(1) as i16) as i32 as u32,
        (y.wrapping_sub(1) as i16) as i32 as u32,
        (w.wrapping_add(2) as i16) as i32 as u32,
        (h.wrapping_add(2) as i16) as i32 as u32,
    ];
    unsafe { svc_emit(p.as_ptr(), 20) };
}

// ---------------------------------------------------------------- cells, claims, buckets

fn yard_byte(e: &Env, u: u32, k: i32) -> u32 {
    let def = e.off(r32(u + U_DEF));
    let y = e.off(r32(def + 334));
    r8(y.wrapping_add(k as u32))
}

// A take of the cell field at address fa by unit t (id tid).
fn take(e: &Env, fa: u32, t: u32, tid: u32) {
    let v = r16(fa);
    if v == 0 {
        w16(fa, tid);
        return;
    }
    let o = e.unit_id(v);
    let pl = e.off(r32(o + U_PLAYER));
    if r32(pl) != 0 && r8(pl + 115) == 3 {
        w32(o + U_FLAGS, r32(o + U_FLAGS) | F_PENDING);
        w32(t + U_FLAGS, r32(t + U_FLAGS) | F_OVERLAP);
        w16(fa, tid);
    } else {
        w32(o + U_FLAGS, r32(o + U_FLAGS) | F_OVERLAP);
        w32(t + U_FLAGS, r32(t + U_FLAGS) | F_PENDING);
    }
}

fn claim(e: &Env, u: u32) {
    let fl = r32(u + U_FLAGS);
    if fl & F_PENDING == 0 {
        return;
    }
    w32(u + U_FLAGS, fl & !F_PENDING);
    let (x, y, w, h) = unit_rect(u);
    let first = unsafe { los_get_map_cell(x, y) };
    let n = (first.wrapping_sub(e.cells) as i32) / (CELL_SIZE as i32);
    let id = r16(u + U_ID);
    let fl = r32(u + U_FLAGS);
    let yard_bit = if r8(u + U_COB) & 4 != 0 { 2 } else { 4 };
    let mut row = 0;
    while row < h {
        let mut col = 0;
        while col < w {
            let k = row * w + col;
            let idx = n.wrapping_add(row.wrapping_mul(e.w)).wrapping_add(col);
            let ca = e.cell(idx);
            if fl & F_STRUCT != 0 {
                if yard_byte(e, u, k) & yard_bit != 0 {
                    take(e, ca, u, id);
                } else if r16(ca) == id {
                    w16(ca, 0);
                }
            } else if fl & 3 == 1 {
                take(e, ca, u, id);
            } else {
                take(e, ca + 2, u, id);
            }
            col += 1;
        }
        row += 1;
    }
}

fn overlaps(o: u32, l: i32, t: i32, r: i32, b: i32) -> bool {
    let (ox, oy, ow, oh) = unit_rect(o);
    l < ox + ow && r > ox && t < oy + oh && b > oy
}

fn reclaim(e: &Env, l: i32, t: i32, w: i32, h: i32) {
    let r = l.wrapping_add(w);
    let b = t.wrapping_add(h);
    let g = e.game;
    let gw = r32(g + G_GRID_W);
    let gh = r32(g + G_GRID_H);
    let grid = e.off(r32(g + G_GRID));
    let bx0 = (l >> 3) - 1;
    let bx1 = (r >> 3) + 1;
    let by0 = (t >> 3) - 1;
    let by1 = (b >> 3) + 1;
    let mut bx = bx0;
    while bx <= bx1 {
        let mut by = by0;
        while by <= by1 {
            if (bx as u32) < gw && (by as u32) < gh {
                let bucket = grid.wrapping_add(10 * (by as u32).wrapping_mul(gw).wrapping_add(bx as u32));
                let mut o = r32(bucket + 6);
                while o != 0 {
                    let oa = e.off(o);
                    if overlaps(oa, l, t, r, b) {
                        claim(e, oa);
                    }
                    let mut c = r32(oa + U_CARGO);
                    while c != 0 {
                        let ca = e.off(c);
                        if overlaps(ca, l, t, r, b) {
                            claim(e, ca);
                        }
                        c = r32(ca + U_CARGO_NEXT);
                    }
                    o = r32(oa + U_CARGO_NEXT);
                }
            }
            by += 1;
        }
        bx += 1;
    }
}

fn in_chain(e: &Env, bucket: u32, uo: u32) -> bool {
    let mut p = r32(bucket + 6);
    while p != 0 {
        if p == uo {
            return true;
        }
        p = r32(e.off(p) + U_CARGO_NEXT);
    }
    false
}

// bucket and uo are block offsets, u the unit's address.
fn unlink(e: &Env, bucket_off: u32, uo: u32) {
    let bucket = e.off(bucket_off);
    let u = e.off(uo);
    let next = r32(u + U_CARGO_NEXT);
    let head = r32(bucket + 6);
    if head == uo {
        w32(bucket + 6, next);
    } else {
        let mut p = head;
        while p != 0 {
            let pa = e.off(p);
            let n = r32(pa + U_CARGO_NEXT);
            if n == uo {
                w32(pa + U_CARGO_NEXT, next);
                break;
            }
            p = n;
        }
    }
}

fn file_unit(e: &Env, uo: u32, bucket_off: u32) {
    let u = e.off(uo);
    let cur = r32(u + U_BUCKET);
    if cur == bucket_off {
        return;
    }
    if r32(u + U_CARRIER) == 0 {
        if cur != 0 {
            unlink(e, cur, uo);
        }
        let b = e.off(bucket_off);
        w32(u + U_CARGO_NEXT, r32(b + 6));
        w32(b + 6, uo);
    }
    w32(u + U_BUCKET, bucket_off);
}

// ---------------------------------------------------------------- Apply

fn do_removal(e: &Env, uo: u32) {
    let u = e.off(uo);
    let (x, y, w, h) = unit_rect(u);
    let id = r16(u + U_ID);
    let fl = r32(u + U_FLAGS);
    if r32(u + U_BUCKET) != r32(e.game + G_OVERFLOW) {
        let mut row = 0;
        while row < h {
            let mut col = 0;
            while col < w {
                let k = row * w + col;
                let (cx, cy) = (x + col, y + row);
                if cx < 0 || cx >= e.w || cy < 0 || cy >= e.h {
                    col += 1;
                    continue;
                }
                let ca = e.cell_xy(cx, cy);
                if fl & F_STRUCT != 0 {
                    if r16(ca) == id {
                        w16(ca, 0);
                    }
                    if yard_byte(e, u, k) & 1 != 0 {
                        w8(ca + 12, r8(ca + 12) & 0xfd);
                    }
                } else if fl & 3 == 1 {
                    if r16(ca) == id {
                        w16(ca, 0);
                    }
                } else if fl & 3 == 2 {
                    if r16(ca + 2) == id {
                        w16(ca + 2, 0);
                    }
                }
                col += 1;
            }
            row += 1;
        }
        if fl & F_STRUCT != 0 {
            cell_heights(x, y, w, h);
        }
    }
    let mut fl = r32(u + U_FLAGS) & !F_PENDING;
    let ov = fl & F_OVERLAP != 0;
    if ov {
        fl &= !F_OVERLAP;
    }
    w32(u + U_FLAGS, fl);
    if ov {
        reclaim(e, x, y, w, h);
    }
    stamp_update(e, u);
}

fn apply_add(e: &Env, uo: u32) {
    let u = e.off(uo);
    let sa = e.stamp_addr(u);
    if sa != 0 {
        w32(sa, e.ticks());
    }
    let (x, y, w, h) = unit_rect(u);
    if x < 0 || y < 0 || x + w >= e.w || y + h >= e.h {
        let ov = r32(e.game + G_OVERFLOW);
        file_unit(e, uo, ov);
        return;
    }
    let gw = r32(e.game + G_GRID_W);
    let bx = ri32(u + U_POS) >> 23;
    let bz = ri32(u + U_POS + 8) >> 23;
    let elem = (bz as u32).wrapping_mul(gw).wrapping_add(bx as u32);
    let bucket = r32(e.game + G_GRID).wrapping_add(elem.wrapping_mul(10));
    file_unit(e, uo, bucket);
    let fl = r32(u + U_FLAGS);
    let id = r16(u + U_ID);
    let yard_bit = if r8(u + U_COB) & 4 != 0 { 2 } else { 4 };
    let mut row = 0;
    while row < h {
        let mut col = 0;
        while col < w {
            let k = row * w + col;
            let ca = e.cell_xy(x + col, y + row);
            if fl & F_STRUCT != 0 {
                let m = yard_byte(e, u, k);
                if m & yard_bit != 0 {
                    take(e, ca, u, id);
                }
                if m & 1 != 0 {
                    w8(ca + 12, r8(ca + 12) | 2);
                }
            } else if fl & 3 == 1 {
                take(e, ca, u, id);
            } else if fl & 3 == 2 {
                take(e, ca + 2, u, id);
            }
            col += 1;
        }
        row += 1;
    }
    if fl & F_STRUCT != 0 {
        cell_heights(x, y, w, h);
        rebuild_report_all(x, y, w, h, 2);
    }
}

fn apply_set_yard(e: &Env, uo: u32, open: i32) -> i32 {
    let u = e.off(uo);
    if ri32(sim() + S_QUIRK_SKIP_YARD) != 0 {
        let def = e.off(r32(u + U_DEF));
        if r32(def + 334) == 0 {
            return 0;
        }
    }
    let (x, y, w, h) = unit_rect(u);
    let xw = (x.wrapping_add(w) as i16) as i32;
    let yh = (y.wrapping_add(h) as i16) as i32;
    if !(x >= 1 && y >= 1 && xw < e.w && yh < e.h) {
        return 0;
    }
    let id = r16(u + U_ID);
    let bit = if open != 0 { 2 } else { 4 };
    let mut row = 0;
    while row < h {
        let mut col = 0;
        while col < w {
            let k = row * w + col;
            if yard_byte(e, u, k) & bit != 0 {
                let c = r16(e.cell_xy(x + col, y + row));
                if c != 0 && c != id {
                    return 0;
                }
            }
            col += 1;
        }
        row += 1;
    }
    let cob = r8(u + U_COB);
    w8(u + U_COB, (cob & !4) | (((open as u32) & 1) << 2));
    w32(u + U_FLAGS, r32(u + U_FLAGS) | F_PENDING);
    claim(e, u);
    rebuild_report_all(x, y, w, h, 2);
    1
}

#[no_mangle]
pub extern "C" fn Apply(ev: u32, size: u32) -> i32 {
    // Keeps the function table import alive.
    if unsafe { read_volatile(&NEVER) } != 0 {
        let f = unsafe { read_volatile(&TABLE[0]) };
        f();
    }
    if size < 8 {
        return 0;
    }
    let kind = r32(ev);
    let need = match kind {
        164 => 12,
        168 => 24,
        169 => 12,
        170 => 12,
        171 => 16,
        176 => 32,
        _ => 8,
    };
    if size < need {
        return 0;
    }
    let e = env();
    match kind {
        168 => {
            let s0 = ri32(ev + 16);
            let s1 = ri32(ev + 20);
            if s0 < 0 || s1 < 0 {
                return 0;
            }
            let cx = ri16(ev + 8);
            let cy = ri16(ev + 12);
            rebuild_report_all(cx, cy, s0 as i16 as i32, s1 as i16 as i32, 1);
            return 1;
        }
        170 => {
            w8(e.game + G_SEA, r32(ev + 8));
            return 1;
        }
        171 => {
            let g = e.game;
            w32(g + G_MAP_W, r32(g + G_BASE_X).wrapping_sub(r32(ev + 8)));
            w32(g + G_MAP_H, r32(g + G_BASE_Y).wrapping_sub(r32(ev + 12)));
            return 1;
        }
        _ => {}
    }
    let uo = r32(ev + 4);
    if uo == 0 {
        return 0;
    }
    let u = e.off(uo);
    match kind {
        160 => {
            apply_add(&e, uo);
            1
        }
        161 => {
            do_removal(&e, uo);
            1
        }
        162 => {
            do_removal(&e, uo);
            let sb = r32(u + U_BUCKET);
            if r32(u + U_CARRIER) == 0 && sb != 0 {
                unlink(&e, sb, uo);
                w32(u + U_CARGO_NEXT, 0);
            }
            w32(u + U_BUCKET, 0);
            1
        }
        163 => {
            w32(u + U_FLAGS, r32(u + U_FLAGS) | F_PENDING);
            let (x, y, w, h) = unit_rect(u);
            reclaim(&e, x, y, w, h);
            rebuild_report_all(x, y, w, h, 2);
            1
        }
        164 => apply_set_yard(&e, uo, ri32(ev + 8)),
        165 => {
            let sb = r32(u + U_BUCKET);
            if sb == 0 {
                return 0;
            }
            if !in_chain(&e, e.off(sb), uo) {
                return 0;
            }
            unlink(&e, sb, uo);
            w32(u + U_CARGO_NEXT, 0);
            1
        }
        166 => {
            let sb = r32(u + U_BUCKET);
            if sb == 0 {
                return 0;
            }
            let b = e.off(sb);
            if in_chain(&e, b, uo) {
                return 0;
            }
            w32(u + U_CARGO_NEXT, r32(b + 6));
            w32(b + 6, uo);
            1
        }
        169 => {
            let sa = e.stamp_addr(u);
            if sa == 0 {
                return 0;
            }
            w32(sa, r32(ev + 8));
            1
        }
        _ => 0,
    }
}

// ---------------------------------------------------------------- queries

#[no_mangle]
pub extern "C" fn UnitsInRange(pos: u32, range: i32, skip: i32, ids: u32, capacity: i32) -> i32 {
    let e = env();
    let gw = r32(e.game + G_GRID_W);
    let gh = r32(e.game + G_GRID_H);
    if gw == 0 || gh == 0 {
        return 0;
    }
    let px = ri32(pos);
    let pz = ri32(pos + 8);
    let clamp = |v: i32, n: u32| -> i32 {
        if (v as u32) < n {
            v
        } else if v < 0 {
            0
        } else {
            (n - 1) as i32
        }
    };
    let x1 = clamp(px.wrapping_sub(range) >> 23, gw);
    let x2 = clamp(px.wrapping_add(range) >> 23, gw);
    let z1 = clamp(pz.wrapping_sub(range) >> 23, gh);
    let z2 = clamp(pz.wrapping_add(range) >> 23, gh);
    let rr = (((range as i64) * (range as i64)) >> 32) as i32;
    let grid = e.off(r32(e.game + G_GRID));
    let mut skip = skip;
    let mut count: i32 = 0;
    let mut z = z1;
    while z <= z2 {
        let mut x = x1;
        while x <= x2 {
            let b = grid.wrapping_add(10 * (z as u32).wrapping_mul(gw).wrapping_add(x as u32));
            let mut o = r32(b + 6);
            while o != 0 {
                let oa = e.off(o);
                let dx = ri32(oa + U_POS).wrapping_sub(px) as i64;
                let dz = ri32(oa + U_POS + 8).wrapping_sub(pz) as i64;
                let d = (((dx * dx) >> 32) as i32).wrapping_add(((dz * dz) >> 32) as i32);
                if d <= rr {
                    if skip > 0 {
                        skip -= 1;
                    } else {
                        if count == capacity {
                            return count;
                        }
                        w16(ids.wrapping_add((count as u32).wrapping_mul(2)), r16(oa + U_ID));
                        count += 1;
                    }
                }
                o = r32(oa + U_CARGO_NEXT);
            }
            x += 1;
        }
        z += 1;
    }
    count
}

#[no_mangle]
pub extern "C" fn GetCellMeanHeight(pos: u32) -> i32 {
    let e = env();
    let cx = (ri32(pos) >> 16) as i16 as i32 / 16;
    let cz = (ri32(pos + 8) >> 16) as i16 as i32 / 16;
    if cx >= 0 && cx < e.w && cz >= 0 && cz < e.h {
        let c = e.cell_xy(cx, cz);
        return ((r8(c + 6) + r8(c + 5)) >> 1) as i32;
    }
    -1
}

#[no_mangle]
pub extern "C" fn GetGroundHeight(pos: u32) -> i32 {
    let e = env();
    let xw = (ri32(pos) >> 16) as i16 as i32;
    let zw = (ri32(pos + 8) >> 16) as i16 as i32;
    let cx = xw >> 4;
    let cz = zw >> 4;
    let u = xw & 15;
    let v = zw & 15;
    if cx < 0 || cx + 1 >= e.w || cz < 0 || cz + 1 >= e.h {
        return -1;
    }
    let a = r8(e.cell_xy(cx, cz) + 4) as i32;
    let b = r8(e.cell_xy(cx + 1, cz) + 4) as i32;
    let c = r8(e.cell_xy(cx, cz + 1) + 4) as i32;
    let d = r8(e.cell_xy(cx + 1, cz + 1) + 4) as i32;
    let north = a + (b - a) * u / 16;
    let south = c + (d - c) * u / 16;
    north + (south - north) * v / 16
}

#[no_mangle]
pub extern "C" fn PassMapGet(k: i32, x: i32, y: i32) -> i32 {
    let p = pm_addr((k as u32) & 31);
    let w = r32(p + 16);
    let wa = base()
        .wrapping_add(r32(p + 24))
        .wrapping_add(w.wrapping_mul((y as u32) >> 4).wrapping_add(x as u32).wrapping_mul(4));
    ((r32(wa) >> (2 * ((y as u32) & 15))) & 3) as i32
}

#[no_mangle]
pub extern "C" fn PassMapInBounds(k: i32, x: i32, y: i32) -> i32 {
    let p = pm_addr((k as u32) & 31);
    ((x as u32) < r32(p + 16) && (y as u32) < r32(p + 20)) as i32
}

// ---------------------------------------------------------------- placement

// Ground level of the footprint at (x, y) of size (fx, fz): (L, H, G).
fn ground_level(e: &Env, def: u32, x: i32, y: i32, fx: i32, fz: i32) -> (i32, i32, i32) {
    let yard = e.off(r32(def + 334));
    let mut l: i32 = 255;
    let mut h: i32 = 0;
    let mut row = 0;
    while row < fz {
        let mut col = 0;
        while col < fx {
            let k = row * fx + col;
            if r8(yard.wrapping_add(k as u32)) & 8 != 0 {
                let c = e.cell_xy(x + col, y + row);
                let lo = r8(c + 6) as i32;
                let hi = r8(c + 5) as i32;
                if lo < l {
                    l = lo;
                }
                if hi > h {
                    h = hi;
                }
            }
            col += 1;
        }
        row += 1;
    }
    let g = if h < l {
        ((e.sea - r8(def + 556) as i32) as u32 & 0xff) as i32
    } else {
        l
    };
    (l, h, g)
}

fn whole(v: i32) -> i32 {
    (v >> 16) as i16 as i32
}

fn build_test(e: &Env, def: u32, cell: u32, tid: u32, player: u32) -> i32 {
    let sim = sim();
    w32(sim + S_PLACE_H, 0);
    w32(sim + S_PLACE_METAL, 0);
    let x = ri16(cell);
    let y = ri16(cell + 2);
    let fx = ri16(def + 330);
    let fz = ri16(def + 332);
    if x < 1 || y < 1 || x.wrapping_add(fx) >= e.w || y.wrapping_add(fz) >= e.h {
        return 0;
    }
    let mut seen = true;
    if player != 0 {
        let hh = r8(e.cell_xy(x, y) + 4) as i32;
        let xx = fx.wrapping_add(x.wrapping_mul(2)).wrapping_mul(1 << 19);
        let zz = fz.wrapping_add(y.wrapping_mul(2)).wrapping_mul(1 << 19);
        let tx = whole(xx) >> 5;
        let ty = whole(zz).wrapping_sub(hh / 2) >> 5;
        let ew = ri32(player + 128);
        let eh = ri32(player + 132);
        if !((tx as u32) < ew as u32 && (ty as u32) < eh as u32) {
            return 0;
        }
        let i = ew.wrapping_mul(ty).wrapping_add(tx);
        let vm = e.off(r32(e.game + G_VIS_MASK));
        let word = r16(vm.wrapping_add((i as u32).wrapping_mul(2)));
        let pi = r8(e.game + G_PLAYER_INDEX);
        if word & 1u32.wrapping_shl(pi) == 0 {
            return 0;
        }
        if r16(e.game + G_MAP_FLAGS) & 2 != 0 {
            let ex = e.off(r32(player + 124));
            seen = r8(ex.wrapping_add(i as u32)) != 0;
        }
    }
    let yard = e.off(r32(def + 334));
    let mut sum: i32 = 0;
    let mut row = 0;
    while row < fz {
        let mut col = 0;
        while col < fx {
            let k = row * fx + col;
            let m = r8(yard.wrapping_add(k as u32));
            let idx = (y + row).wrapping_mul(e.w).wrapping_add(x + col);
            let ca = e.cell(idx);
            sum = sum.wrapping_add(r8(ca + 7) as i32);
            let mut fail = false;
            if m & 1 != 0 && r8(ca + 12) & 2 != 0 && seen {
                fail = true;
            }
            if !fail && m & 6 != 0 {
                let cu = r16(ca);
                if cu != 0 && cu != (tid & 0xffff) && seen {
                    fail = true;
                }
            }
            if !fail && m & 0x20 != 0 && e.blocked(idx) {
                fail = true;
            }
            if !fail && m & 0x40 != 0 {
                let fr = e.feat_rec(idx);
                if fr != 0 && r16(fr + 254) & 0x200 != 0 {
                    fail = true;
                }
            }
            if fail {
                w32(sim + S_PLACE_METAL, sum as u32);
                return 0;
            }
            col += 1;
        }
        row += 1;
    }
    w32(sim + S_PLACE_METAL, sum as u32);
    // site tests
    let (l, h, g) = ground_level(e, def, x, y, fx, fz);
    let mut kk: i32 = 0;
    let mut need_geo = false;
    let mut has_geo = false;
    let mut row = 0;
    while row < fz {
        let mut col = 0;
        while col < fx {
            let k = row * fx + col;
            let m = r8(yard.wrapping_add(k as u32));
            let idx = (y + row).wrapping_mul(e.w).wrapping_add(x + col);
            if m & 0x10 != 0 {
                let hi = r8(e.cell(idx) + 5) as i32;
                if hi > kk {
                    kk = hi;
                }
            }
            if m & 0x80 != 0 {
                need_geo = true;
                let fr = e.feat_rec(idx);
                if fr != 0 && r16(fr + 254) & 0x20 != 0 {
                    has_geo = true;
                }
            }
            col += 1;
        }
        row += 1;
    }
    let maxslope = r8(def + 552) as i32;
    let maxwd = ri16(def + 446);
    let minwd = ri16(def + 448);
    let sea = e.sea;
    let mut fail = false;
    if need_geo && !has_geo {
        fail = true;
    }
    if h >= l && h - l > maxslope {
        fail = true;
    }
    if (kk as u32 & 0xff) > (g as u32 & 0xff) {
        fail = true;
    }
    if l < sea - maxwd {
        fail = true;
    }
    let mx = if h > kk { h } else { kk };
    if mx > sea - minwd {
        fail = true;
    }
    if fail {
        return 0;
    }
    w32(sim + S_PLACE_H, g as u32);
    1
}

#[no_mangle]
pub extern "C" fn CanBuildAt(def: u32, cell: u32, ty: i32, player: u32) -> i32 {
    let e = env();
    build_test(&e, def, cell, ty as u32, player)
}

#[no_mangle]
pub extern "C" fn CanPlaceUnitFootprint(def: u32, other: u32, cell: u32, flags: i32) -> i32 {
    let e = env();
    let x = ri16(cell);
    let y = ri16(cell + 2);
    let fx = ri16(def + 330);
    let fz = ri16(def + 332);
    if x < 0 || y < 0 || fx.wrapping_add(x) >= e.w || fz.wrapping_add(y) >= e.h {
        return if flags == 2 { 1 } else { 0 };
    }
    if r8(def + 559) == 0 {
        return build_test(&e, def, cell, 0, 0);
    }
    if flags != 1 {
        return 1;
    }
    let sea = e.sea;
    let maxwd = ri16(def + 446);
    let minwd = ri16(def + 448);
    let maxs = r8(def + 552) as i32;
    let maxws = r8(def + 553) as i32;
    let other = other & 0xffff;
    let mut row = 0;
    while row < fz {
        let mut col = 0;
        while col < fx {
            let idx = (y + row).wrapping_mul(e.w).wrapping_add(x + col);
            let ca = e.cell(idx);
            let low = r8(ca + 6) as i32;
            let high = r8(ca + 5) as i32;
            let d = high - low;
            let cu = r16(ca);
            if e.blocked(idx)
                || (cu != 0 && cu != other)
                || low < sea - maxwd
                || high > sea - minwd
                || (d > maxs && (low >= sea || d > maxws))
            {
                return 0;
            }
            col += 1;
        }
        row += 1;
    }
    1
}

fn snap(p: i32, f: i32) -> i32 {
    let v = p.wrapping_sub(f.wrapping_mul(1 << 19)).wrapping_add(1 << 19);
    (v >> 20) as i16 as i32
}

#[no_mangle]
pub extern "C" fn SnapWorldPosToFootprint(def: u32, pos: u32) {
    if r8(def + 559) != 0 {
        return;
    }
    let e = env();
    let fx = ri16(def + 330);
    let fz = ri16(def + 332);
    let cx = snap(ri32(pos), fx);
    let cz = snap(ri32(pos + 8), fz);
    let f = if cx < 1 || cz < 1 || cx.wrapping_add(fx) >= e.w || cz.wrapping_add(fz) >= e.h {
        0
    } else {
        ground_level(&e, def, cx, cz, fx, fz).2
    };
    w32(pos, fx.wrapping_add(cx.wrapping_mul(2)).wrapping_mul(1 << 19) as u32);
    w32(pos + 8, fz.wrapping_add(cz.wrapping_mul(2)).wrapping_mul(1 << 19) as u32);
    w32(pos + 4, (f as u32).wrapping_mul(65536));
}

#[no_mangle]
pub extern "C" fn CanPlaceFootprintAt(unit: u32, pos: u32) -> i32 {
    let e = env();
    let ux = ri16(unit + U_FOOT);
    let uz = ri16(unit + U_FOOT + 2);
    let a = snap(ri32(pos), ux);
    let c = snap(ri32(pos + 8), uz);
    if a < 0 || c < 0 || a.wrapping_add(ux) >= e.w || c.wrapping_add(uz) >= e.h {
        return 0;
    }
    let j = ((c >> 1).wrapping_add(ux >> 2))
        .wrapping_mul(e.w >> 1)
        .wrapping_add(a >> 1)
        .wrapping_add(ux >> 2);
    let vm = e.off(r32(e.game + G_VIS_MASK));
    let word = r16(vm.wrapping_add((j as u32).wrapping_mul(2)));
    let pi = r8(unit + U_PLAYER_INDEX);
    if word & 1u32.wrapping_shl(pi) == 0 {
        return 1;
    }
    let def = e.off(r32(unit + U_DEF));
    let sea = e.sea;
    let mut lo = sea - ri16(def + 446);
    let hi = sea - ri16(def + 448);
    let f1 = r32(def + 577);
    if lo < sea && f1 & 0x800 != 0 && f1 & 0x20_0000 == 0 {
        lo = sea;
    }
    let maxs = r8(def + 552) as i32;
    let id = r16(unit + U_ID);
    let mut row = 0;
    while row < uz {
        let mut col = 0;
        while col < ux {
            let idx = (c + row).wrapping_mul(e.w).wrapping_add(a + col);
            let ca = e.cell(idx);
            let low = r8(ca + 6) as i32;
            let high = r8(ca + 5) as i32;
            let u1 = r16(ca);
            let u2 = r16(ca + 2);
            if e.blocked(idx)
                || r8(ca + 12) & 2 != 0
                || (u1 != 0 && u1 != id)
                || (u2 != 0 && u2 != id)
                || low < lo
                || high > hi
                || high - low > maxs
            {
                return 0;
            }
            col += 1;
        }
        row += 1;
    }
    1
}

#[no_mangle]
pub extern "C" fn IsPadSlotFree(pad: u32, id: i32) -> i32 {
    let e = env();
    if r32(pad + U_CARRIER) != 0 {
        return 0;
    }
    let mut c = r32(pad + U_CARGO);
    while c != 0 {
        let ca = e.off(c);
        if (r8(ca + U_TRANSPORT_PIECE) as u8 as i8 as i32) == id {
            return 0;
        }
        c = r32(ca + U_CARGO_NEXT);
    }
    1
}

// ---------------------------------------------------------------- pass map build

#[no_mangle]
pub extern "C" fn BuildAllPassMaps(limits: u32, count: i32) {
    let e0 = env();
    let g = e0.game;
    let mut k = 0;
    while k < 32 {
        w32(pm_addr(k) + 12, 0);
        k += 1;
    }
    let mut i: i32 = 0;
    while i < count {
        let l = limits.wrapping_add((i as u32).wrapping_mul(16));
        let idx = r32(l);
        if idx < 32 {
            let p = pm_addr(idx);
            w16(p, r16(l + 4));
            w16(p + 2, r16(l + 6));
            w16(p + 4, r16(l + 8));
            w16(p + 6, r16(l + 10));
            w8(p + 8, r8(l + 12));
            w8(p + 9, r8(l + 13));
            w8(p + 10, r8(l + 14));
            w8(p + 11, r8(l + 15));
            w32(p + 12, 1);
        }
        i += 1;
    }
    let mut k = 0;
    while k < 32 {
        if defined(k) {
            let p = pm_addr(k);
            let width = r32(g + G_WIDTH);
            let height = r32(g + G_HEIGHT);
            w32(p + 16, width);
            w32(p + 20, height);
            let n = (height.wrapping_add(15) >> 4).wrapping_mul(width);
            let old = r32(p + 24);
            if old != 0 {
                unsafe { svc_free(base().wrapping_add(old)) };
            }
            if n != 0 {
                let a = unsafe { svc_alloc(n.wrapping_mul(4) as i32) };
                w32(p + 24, a.wrapping_sub(base()));
            } else {
                w32(p + 24, 0);
            }
            let fx = ri16(p) as u32;
            let fz = ri16(p + 2) as u32;
            let a1 = width.wrapping_add(fx);
            let a2 = fz.wrapping_add(height);
            let s = if a1 > a2 { a1 } else { a2 };
            let scratch = unsafe { svc_alloc(s.wrapping_add(3) as i32) };
            if n != 0 {
                full_build(k, scratch);
            }
            unsafe { svc_free(scratch) };
        }
        k += 1;
    }
    w8(g + G_PROGRESS, 100);
}

// The full build of class k, with the scratch block (at least width + fx + 3 bytes).
fn full_build(k: u32, scratch: u32) {
    let e = env();
    let c = cls(&e, k);
    let w = c.width as i32;
    let h = c.height as i32;
    let (fx, fz) = (c.fx, c.fz);
    if fx < 1 || fz < 1 {
        return;
    }
    // column x' in -1 ..= w + fx - 1 lives at scratch[x' + 1]
    let mut y = 0;
    while y < h {
        let mut xc = -1;
        while xc < w + fx {
            let mut flags: u32 = 0;
            let mut yy = y - 1;
            while yy <= y + fz {
                let off = xc < 0 || xc >= w || yy < 0 || yy >= h;
                let pv = if off { 0 } else { pass_value(&e, &c, xc, yy) };
                if off || pv != 3 {
                    flags |= 2;
                }
                if yy >= y && yy < y + fz && (off || pv == 0) {
                    flags |= 1;
                }
                yy += 1;
            }
            w8(scratch + (xc + 1) as u32, flags);
            xc += 1;
        }
        let mut x = 0;
        while x < w {
            let mut block_bad = false;
            let mut grown_bad = false;
            let mut xc = x - 1;
            while xc <= x + fx {
                let f = r8(scratch + (xc + 1) as u32);
                if f & 2 != 0 {
                    grown_bad = true;
                }
                if f & 1 != 0 && xc >= x && xc < x + fx {
                    block_bad = true;
                }
                xc += 1;
            }
            let v = if block_bad {
                0
            } else if grown_bad {
                1
            } else {
                3
            };
            pm_write(&c, x as u32, y as u32, v);
            x += 1;
        }
        y += 1;
    }
}

// ---------------------------------------------------------------- refreshes

#[no_mangle]
pub extern "C" fn RefreshMovedUnits(k: i32, unit: u32) {
    let e = env();
    let k = (k as u32) & 31;
    let pm = pm_addr(k);
    let t = e.ticks();
    let o = r32(pm + 28);
    let s = if t > 30 { t - 30 } else { 0 };
    let sa = e.stamp_addr(unit);
    if sa == 0 {
        return;
    }
    let p = r32(sa);
    w32(pm + 28, s);
    w32(sa, t);
    let (x, y, w, h) = unit_rect(unit);
    if p < o {
        rebuild(k, x, y, w, h);
        report(k, x, y, w, h, 2);
    }
    if s != o {
        let end = e.off(r32(e.game + G_UNITS_END));
        let mut m = e.unit_id(1);
        while m <= end {
            if r32(m + U_FLAGS) & F_ALIVE != 0 {
                let ms = e.stamp_addr(m);
                if ms != 0 {
                    let q = r32(ms);
                    if o <= q && q < s {
                        let (mx, my, mw, mh) = unit_rect(m);
                        rebuild(k, mx, my, mw, mh);
                        report(k, mx, my, mw, mh, 2);
                    }
                }
            }
            m = m.wrapping_add(UNIT_SIZE);
        }
    }
    w32(sa, p);
}

#[no_mangle]
pub extern "C" fn RefreshUnitIfStale(k: i32, unit: u32) {
    let e = env();
    let k = (k as u32) & 31;
    let sa = e.stamp_addr(unit);
    if sa == 0 {
        return;
    }
    if r32(sa) < r32(pm_addr(k) + 28) {
        let (x, y, w, h) = unit_rect(unit);
        rebuild(k, x, y, w, h);
        report(k, x, y, w, h, 2);
    }
}

// ---------------------------------------------------------------- reclaim pick

#[no_mangle]
pub extern "C" fn PickRandomReclaimableResourcesInRadius(
    pos: u32,
    range: i32,
    energy: u32,
    energy_amount: u32,
    metal: u32,
    metal_amount: u32,
) -> i32 {
    let e = env();
    let w = range >> 16;
    let cap = (w.wrapping_mul(w) as u32) / 256;
    let metal_list = unsafe { svc_alloc(cap.wrapping_mul(16) as i32) };
    let energy_list = unsafe { svc_alloc(cap.wrapping_mul(16) as i32) };
    let hh = range / 2;
    let px = ri32(pos);
    let py = ri32(pos + 4);
    let pz = ri32(pos + 8);
    let mut ne: u32 = 0;
    let mut nm: u32 = 0;
    let mut z = pz.wrapping_sub(hh);
    let zmax = pz.wrapping_add(hh);
    'rows: while z <= zmax {
        let mut x = px.wrapping_sub(hh);
        let xmax = px.wrapping_add(hh);
        'cols: while x <= xmax {
            let pt: [u32; 3] = [x as u32, py as u32, z as u32];
            let cell = unsafe { los_get_map_cell_at_position(pt.as_ptr() as *const u8) };
            if cell != 0 {
                let kf = (unsafe { feat_get_cell_feature(cell) } as u32) & 0xffff;
                if kf < 0xfffb {
                    let f = e.feat(kf);
                    if r16(f + 254) & 0x180 == 0x180 {
                        let en = f32::from_bits(r32(f + 236));
                        let me = f32::from_bits(r32(f + 240));
                        if en != 0.0 {
                            let a = energy_list.wrapping_add(ne.wrapping_mul(16));
                            w32(a, pt[0]);
                            w32(a + 4, pt[1]);
                            w32(a + 8, pt[2]);
                            w32(a + 12, r32(f + 236));
                            ne += 1;
                        }
                        if me != 0.0 {
                            let a = metal_list.wrapping_add(nm.wrapping_mul(16));
                            w32(a, pt[0]);
                            w32(a + 4, pt[1]);
                            w32(a + 8, pt[2]);
                            w32(a + 12, r32(f + 240));
                            nm += 1;
                        }
                    }
                }
            }
            // a step that wraps past i32::MAX ends the row instead of looping on
            match x.checked_add(0x30_0000) {
                Some(nx) => x = nx,
                None => break 'cols,
            }
        }
        match z.checked_add(0x30_0000) {
            Some(nz) => z = nz,
            None => break 'rows,
        }
    }
    let pick = |list: u32, n: u32| -> u32 {
        let mut idx: u32 = 0;
        let mut val: f32 = 0.0;
        let mut d = 0;
        while d < 3 {
            let i = unsafe { svc_random_int(n as i32) } as u32;
            let v = f32::from_bits(r32(list.wrapping_add(i.wrapping_mul(16)).wrapping_add(12)));
            if v > val {
                idx = i;
                val = v;
            }
            d += 1;
        }
        idx
    };
    let ep = if ne != 0 { pick(energy_list, ne) } else { 0 };
    let mp = if nm != 0 { pick(metal_list, nm) } else { 0 };
    // the picked entries are copied out before the lists are freed
    let mut eo = [0u32; 4];
    let mut mo = [0u32; 4];
    if ne != 0 {
        let a = energy_list.wrapping_add(ep.wrapping_mul(16));
        eo = [r32(a), r32(a + 4), r32(a + 8), r32(a + 12)];
    }
    if nm != 0 {
        let a = metal_list.wrapping_add(mp.wrapping_mul(16));
        mo = [r32(a), r32(a + 4), r32(a + 8), r32(a + 12)];
    }
    unsafe { svc_free(energy_list) };
    unsafe { svc_free(metal_list) };
    if ne != 0 {
        let dst = r32(energy);
        w32(dst, eo[0]);
        w32(dst + 4, eo[1]);
        w32(dst + 8, eo[2]);
        w32(energy_amount, eo[3]);
    } else {
        w32(energy, 0);
        w32(energy_amount, 0);
    }
    if nm != 0 {
        let dst = r32(metal);
        w32(dst, mo[0]);
        w32(dst + 4, mo[1]);
        w32(dst + 8, mo[2]);
        w32(metal_amount, mo[3]);
    } else {
        w32(metal, 0);
        w32(metal_amount, 0);
    }
    (ne != 0 || nm != 0) as i32
}
