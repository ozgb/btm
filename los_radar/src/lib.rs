#![no_std]

use core::ptr::{read_unaligned, read_volatile, write_unaligned};

#[link(wasm_import_module = "network")]
extern "C" {
    #[link_name = "BroadcastPlayerInfo"]
    fn broadcast_player_info();
}

#[link(wasm_import_module = "map")]
extern "C" {
    #[link_name = "UnitsInRange"]
    fn units_in_range(pos: u32, range: i32, skip: i32, ids: u32, cap: i32) -> i32;
    #[link_name = "GetGroundHeight"]
    fn get_ground_height(pos: u32) -> i32;
}

#[link(wasm_import_module = "services")]
extern "C" {
    #[link_name = "SimAlloc"]
    fn sim_alloc(size: i32) -> u32;
    #[link_name = "SimFree"]
    fn sim_free(p: u32);
    #[link_name = "RecordSimEvent"]
    fn record_sim_event(kind: i32, flags: i32, args: u32, count: i32);
    #[link_name = "SimCaptureAlloc"]
    fn sim_capture_alloc(bytes: i32, offset: u32) -> u32;
    #[link_name = "GetGafFrame"]
    fn get_gaf_frame(table: u32, i: i32) -> u32;
}

#[no_mangle]
pub static mut g_game: u32 = 0;
#[no_mangle]
pub static mut g_simGlobals: u32 = 0;
#[no_mangle]
pub static mut g_extTdf: u32 = 0x0066_6474;
#[no_mangle]
pub static mut g_blockBase: u32 = 0;
#[no_mangle]
pub static mut g_region: u32 = 0;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

// ---- memory access -------------------------------------------------------

#[inline(always)]
fn ru8(a: u32) -> u32 {
    unsafe { read_unaligned(a as *const u8) as u32 }
}
#[inline(always)]
fn ru16(a: u32) -> u32 {
    unsafe { read_unaligned(a as *const u16) as u32 }
}
#[inline(always)]
fn ri16(a: u32) -> i32 {
    unsafe { read_unaligned(a as *const i16) as i32 }
}
#[inline(always)]
fn ri32(a: u32) -> i32 {
    unsafe { read_unaligned(a as *const i32) }
}
#[inline(always)]
fn ru32(a: u32) -> u32 {
    unsafe { read_unaligned(a as *const u32) }
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
#[inline(always)]
fn region() -> u32 {
    unsafe { read_volatile(&raw const g_region) }
}

// ---- layout --------------------------------------------------------------

const G_PLAYERS: u32 = 7011;
const G_NUMPLAYERS: u32 = 10812;
const G_PLAYERINDEX: u32 = 10819;
const G_NETFLAGS: u32 = 10820;
const G_PROJCOUNT: u32 = 82419;
const G_PROJ: u32 = 82423;
const G_BASEX: u32 = 82467;
const G_BASEY: u32 = 82471;
const G_WIDTH: u32 = 82483;
const G_HEIGHT: u32 = 82487;
const G_VISMASK: u32 = 82547;
const G_EYECOUNT: u32 = 82551;
const G_EYES: u32 = 82555;
const G_SEALEVEL: u32 = 82559;
const G_MAPFLAGS: u32 = 82561;
const G_CELLS: u32 = 82567;
const G_GRID1: u32 = 82575;
const G_FLAGS142F1: u32 = 82673;
const G_POOLCOUNT: u32 = 82769;
const G_UNITS: u32 = 82775;
const G_UNITSEND: u32 = 82779;
const G_LOSTABLE: u32 = 84059;
const G_MATCHFLAGS: u32 = 229168;
const G_TICKS: u32 = 232007;

const S_LOSTABLES: u32 = 48152;
const S_UNITINDEX: u32 = 48184;

const PLAYER_SIZE: u32 = 331;
const UNIT_SIZE: u32 = 280;

const U_POS: u32 = 106;
const U_DEF: u32 = 146;
const U_PLAYER: u32 = 150;
const U_DEFINDEX: u32 = 166;
const U_ID: u32 = 168;
const U_WORKTIME: u32 = 176;
const U_DAMAGED: u32 = 250;
const U_PLAYERINDEX: u32 = 255;
const U_BUILDLEFT: u32 = 260;
const U_ACTIVATE: u32 = 270;
const U_FLAGS: u32 = 272;

const D_MINX: u32 = 350;
const D_MINZ: u32 = 358;
const D_MAXY: u32 = 366;
const D_MAXYHIGH: u32 = 368;
const D_SIZEX: u32 = 374;
const D_SIZEY: u32 = 378;
const D_SIZEZ: u32 = 382;
const D_RANGE: u32 = 514;
const D_RADAR: u32 = 516;
const D_SONAR: u32 = 518;
const D_MINCLOAK: u32 = 520;
const D_RADARJAM: u32 = 522;
const D_SONARJAM: u32 = 524;
const D_FLAGS1: u32 = 577;
const D_FLAGS2: u32 = 581;

const P_ACTIVE: u32 = 0;
const P_INFO: u32 = 39;
const P_UNITSBEGIN: u32 = 103;
const P_UNITSEND: u32 = 107;
const P_TYPE: u32 = 115;
const P_EXPLORED: u32 = 124;
const P_EXPW: u32 = 128;
const P_EXPH: u32 = 132;
const P_EXPSIZE: u32 = 136;
const P_ALLIED: u32 = 264;
const P_INDEX: u32 = 326;

const I_COLOR: u32 = 150;
const I_FLAGS97: u32 = 151;
const I_FLAGS9B: u32 = 155;

const E_SIZE: u32 = 36;

const ALIVE: u32 = 0x1000_0000;
const DYING: u32 = 0x4000;

// ---- small helpers -------------------------------------------------------

#[inline(always)]
fn gm(off: u32) -> u32 {
    game() + off
}
#[inline(always)]
fn player_addr(i: u32) -> u32 {
    game() + G_PLAYERS + PLAYER_SIZE * i
}
#[inline(always)]
fn map_flags() -> u32 {
    ru16(gm(G_MAPFLAGS))
}
#[inline(always)]
fn set_map_flags(v: u32) {
    w16(gm(G_MAPFLAGS), v)
}
#[inline(always)]
fn view_index() -> u32 {
    ru8(gm(G_PLAYERINDEX))
}
#[inline(always)]
fn units_addr() -> u32 {
    base().wrapping_add(ru32(gm(G_UNITS)))
}
#[inline(always)]
fn units_end_addr() -> u32 {
    base().wrapping_add(ru32(gm(G_UNITSEND)))
}
#[inline(always)]
fn slot_of(u: u32) -> u32 {
    u.wrapping_sub(units_addr()) / UNIT_SIZE
}
/// Last slot of the table; 0 when `unitsEnd` is before `units` (slot 0 is then still scanned, a harmless
/// edge outside the contract).
fn last_slot() -> u32 {
    let (a, e) = (units_addr(), units_end_addr());
    if e < a {
        0
    } else {
        (e - a) / UNIT_SIZE
    }
}
#[inline(always)]
fn unit_at(slot: u32) -> u32 {
    units_addr().wrapping_add(slot.wrapping_mul(UNIT_SIZE))
}
#[inline(always)]
fn def_of(u: u32) -> u32 {
    base().wrapping_add(ru32(u + U_DEF))
}
#[inline(always)]
fn uplayer_of(u: u32) -> u32 {
    base().wrapping_add(ru32(u + U_PLAYER))
}
#[inline(always)]
fn info_of(p: u32) -> u32 {
    base().wrapping_add(ru32(p + P_INFO))
}
#[inline(always)]
fn ready(u: u32) -> bool {
    let f = ru32(u + U_FLAGS);
    f & ALIVE != 0 && f & DYING == 0
}
#[inline(always)]
fn sea_level() -> i32 {
    ru8(gm(G_SEALEVEL)) as i32
}

fn set_viewer_changed() {
    set_map_flags(map_flags() & !8);
    let a = gm(G_FLAGS142F1);
    w16(a, ru16(a) | 4);
}

// ---- sights --------------------------------------------------------------

#[derive(Clone, Copy)]
struct Sight {
    pl: u32,
    cx: u32,
    cy: u32,
    cf: u32,
    d: i32,
    h: i32,
    px: i32,
    py: i32,
    pz: i32,
}

fn unit_sight(u: u32) -> Sight {
    let def = def_of(u);
    let c = region().wrapping_add(5 * slot_of(u));
    let mut py = ri32(u + U_POS + 4);
    let sea = (sea_level() + 1).wrapping_mul(65536);
    if py < sea {
        py = sea;
    }
    Sight {
        pl: uplayer_of(u),
        cx: c,
        cy: c + 2,
        cf: c + 4,
        d: ri16(def + D_RANGE),
        h: (ru16(def + D_MAXYHIGH) & 0xff) as i32,
        px: ri32(u + U_POS),
        py,
        pz: ri32(u + U_POS + 8),
    }
}

fn eye_sight(e: u32) -> Sight {
    Sight {
        pl: base().wrapping_add(ru32(e)),
        cx: e + 32,
        cy: e + 34,
        cf: e + 11,
        d: ri16(e + 8),
        h: ru8(e + 10) as i32,
        px: ri32(e + 16),
        py: ri32(e + 20),
        pz: ri32(e + 24),
    }
}

fn gaf(i: i32) -> u32 {
    unsafe { get_gaf_frame(ru32(gm(G_LOSTABLE)), i) }
}

fn mask_count() -> i32 {
    ru16(ru32(gm(G_LOSTABLE))) as i32
}

fn mask_index(s: &Sight) -> i32 {
    let mut i = s.d / 32 - 5;
    if i < 0 {
        i = 0;
    }
    let m = mask_count();
    if i >= m - 1 {
        i = m - 1;
    }
    i
}

fn circle_cell(s: &Sight) -> (i32, i32) {
    let f = gaf(mask_index(s));
    let (xo, yo) = if f == 0 { (0, 0) } else { (ri16(f + 4), ri16(f + 6)) };
    let yh = s.py >> 16;
    (s.px / 2097152 - xo, s.pz / 2097152 - yh / 64 - yo)
}

/// Calls `f(dx, dy)` for each true cell of a sight with the given origin and reference.
fn true_cells<F: FnMut(i32, i32)>(s: &Sight, x: i32, y: i32, r: i32, mut f: F) {
    let gl = sim() + S_LOSTABLES;
    let first = ru32(gl + 4);
    let last = ru32(gl + 8);
    let n_actual = (last.wrapping_sub(first) as i32) / 16;
    let n = n_actual as i16 as i32;
    let mut k = s.d / 32;
    if k < 0 {
        k = 0;
    }
    if k >= n - 1 {
        k = n - 1;
    }
    let idx = k - 1;
    if idx < 0 || idx >= n_actual {
        return;
    }
    let g1 = gm(G_GRID1);
    let cells = base() + ru32(g1);
    let w = ru32(g1 + 4);
    let h = ru32(g1 + 8);
    let t = base() + first + 16 * idx as u32;
    let lf = ru32(t + 4);
    let ll = ru32(t + 8);
    let nlines = (ll.wrapping_sub(lf) as i32) / 16;
    let mut li = 0;
    while li < nlines {
        let line = base() + lf + 16 * li as u32;
        let sf = ru32(line + 4);
        let sl = ru32(line + 8);
        let nsteps = (sl.wrapping_sub(sf) as i32) / 4;
        let mut best: i32 = 0;
        let mut gap: i32 = -1;
        let mut j = 1;
        while j <= nsteps {
            let st = base() + sf + 4 * (j - 1) as u32;
            let a = ru16(st) as i32;
            let b = ru16(st + 2) as i32;
            let dx = a.wrapping_add(x) as i16 as i32;
            let dy = b.wrapping_add(y) as i16 as i32;
            if (dx as u32) < w && (dy as u32) < h {
                let c = cells + 2 * ((dy as u32) * w + dx as u32);
                let d0 = ru8(c) as i32 - r;
                let d1 = ru8(c + 1) as i32 - r;
                if d0.wrapping_mul(best) > gap.wrapping_mul(j) {
                    f(dx, dy);
                    if d1.wrapping_mul(best) > gap.wrapping_mul(j) {
                        best = j;
                        gap = d1;
                    }
                }
            }
            j += 1;
        }
        li += 1;
    }
}

fn grid_dims() -> (u32, u32) {
    let g1 = gm(G_GRID1);
    (ru32(g1 + 4), ru32(g1 + 8))
}

/// Add (delta = 1) or take off (delta = -1) a sight, with its cache as it is.
fn add_sight(s: &Sight, delta: i32) {
    let pl = s.pl;
    if ru8(pl + P_INDEX) == view_index() {
        set_viewer_changed();
    }
    if ru32(pl + P_EXPLORED) == 0 {
        return;
    }
    let xa = base() + ru32(pl + P_EXPLORED);
    let xw = ri32(pl + P_EXPW);
    let cx = ri16(s.cx);
    let cy = ri16(s.cy);
    let bump = |i: i32| {
        let a = xa.wrapping_add(i as u32);
        w8(a, ru8(a).wrapping_add(delta as u32));
    };
    if map_flags() & 4 != 0 {
        let (w, h) = grid_dims();
        if (cx as u32) < w && (cy as u32) < h {
            bump(cy.wrapping_mul(xw).wrapping_add(cx));
            let r = ru8(s.cf) as i32;
            true_cells(s, cx, cy, r, |dx, dy| bump(dy.wrapping_mul(xw).wrapping_add(dx)));
        }
    } else {
        let f = gaf(ru8(s.cf) as i32);
        if f == 0 {
            return;
        }
        let hw = ri32(gm(G_WIDTH)) / 2;
        let hh = ri32(gm(G_HEIGHT)) / 2;
        let fw = ru16(f) as i32;
        let fh = ru16(f + 2) as i32;
        let tr = ru8(f + 8);
        let px = ru32(f + 16);
        for i in 0..fh {
            for j in 0..fw {
                if ru8(px + (fw * i + j) as u32) == tr {
                    continue;
                }
                let x = cx + j;
                let y = cy + i;
                if x >= 0 && x < hw && y >= 0 && y < hh {
                    bump(y.wrapping_mul(xw).wrapping_add(x));
                }
            }
        }
    }
}

fn reveal(s: &Sight) {
    let pl = s.pl;
    let bit = 1u32 << ru8(pl + P_INDEX);
    let vm = base() + ru32(gm(G_VISMASK));
    let hw = ri32(gm(G_WIDTH)) / 2;
    let hh = ri32(gm(G_HEIGHT)) / 2;
    let cx = ri16(s.cx);
    let cy = ri16(s.cy);
    let mut changed = false;
    let mut set = |i: i32| {
        let a = vm.wrapping_add((i as u32).wrapping_mul(2));
        let old = ru16(a);
        let new = old | bit;
        if new != old {
            changed = true;
            w16(a, new);
        }
    };
    if map_flags() & 4 != 0 {
        let (w, h) = grid_dims();
        if (cx as u32) < w && (cy as u32) < h {
            set(hw.wrapping_mul(cy).wrapping_add(cx));
            let r = ru8(s.cf) as i32;
            true_cells(s, cx, cy, r, |dx, dy| set(hw.wrapping_mul(dy).wrapping_add(dx)));
        }
    } else {
        let f = gaf(mask_index(s));
        if f != 0 {
            let fw = ru16(f) as i32;
            let fh = ru16(f + 2) as i32;
            let tr = ru8(f + 8);
            let px = ru32(f + 16);
            for i in 0..fh {
                for j in 0..fw {
                    if ru8(px + (fw * i + j) as u32) == tr {
                        continue;
                    }
                    let x = cx + j;
                    let y = cy + i;
                    if x >= 0 && x < hw && y >= 0 && y < hh {
                        set(y.wrapping_mul(hw).wrapping_add(x));
                    }
                }
            }
        }
    }
    if changed && ru8(pl + P_INDEX) == view_index() {
        set_viewer_changed();
    }
}

fn move_sight(s: &Sight) {
    let mf = map_flags();
    if mf & 4 != 0 {
        let x = (s.px >> 16) >> 5;
        let mut v = s.h + (s.py >> 16);
        if v < 0 {
            v = 0;
        }
        if v > 255 {
            v = 255;
        }
        let y = ((s.pz >> 16) - (v >> 1)) >> 5;
        let frame = ru8(s.cf) as i32;
        if ri16(s.cx) == x && ri16(s.cy) == y && (frame - v).abs() <= 5 {
            return;
        }
        if frame != 0 && mf & 2 != 0 {
            add_sight(s, -1);
        }
        w16(s.cx, x as u32);
        w16(s.cy, y as u32);
        let (w, h) = grid_dims();
        if (x as u32) >= w || (y as u32) >= h {
            w8(s.cf, 0);
            return;
        }
        w8(s.cf, v as u32);
        if mf & 2 != 0 {
            add_sight(s, 1);
        }
        if mf & 1 != 0 {
            reveal(s);
        }
    } else {
        let (cx, cy) = circle_cell(s);
        let i = mask_index(s);
        if ri16(s.cx) == cx && ri16(s.cy) == cy && ru8(s.cf) as i32 == i {
            return;
        }
        if mf & 2 != 0 {
            add_sight(s, -1);
        }
        w16(s.cx, cx as u32);
        w16(s.cy, cy as u32);
        w8(s.cf, i as u32);
        if mf & 2 != 0 {
            add_sight(s, 1);
        }
        if mf & 1 != 0 {
            reveal(s);
        }
    }
}

fn place_sight(s: &Sight) {
    if map_flags() & 2 == 0 {
        return;
    }
    w8(s.cf, 0);
    if map_flags() & 4 != 0 {
        move_sight(s);
    } else {
        let (cx, cy) = circle_cell(s);
        w16(s.cx, cx as u32);
        w16(s.cy, cy as u32);
        w8(s.cf, mask_index(s) as u32);
        add_sight(s, 1);
        reveal(s);
    }
}

// ---- visibility tests ----------------------------------------------------

fn grid_entry(l: u32, x: i32, y: i32, z: i32) -> Option<u32> {
    grid_entry_t(l, (x >> 16) >> 5, ((z >> 16) - ((y >> 16) >> 1)) >> 5)
}

fn grid_entry_t(l: u32, tx: i32, ty: i32) -> Option<u32> {
    let w = ru32(l + P_EXPW);
    let h = ru32(l + P_EXPH);
    if (tx as u32) < w && (ty as u32) < h {
        Some(w.wrapping_mul(ty as u32).wrapping_add(tx as u32))
    } else {
        None
    }
}

fn explored(l: u32, x: i32, y: i32, z: i32) -> bool {
    match grid_entry(l, x, y, z) {
        Some(e) => ru8(base() + ru32(l + P_EXPLORED) + e) != 0,
        None => false,
    }
}

fn seen(l: u32, x: i32, y: i32, z: i32) -> bool {
    match grid_entry(l, x, y, z) {
        Some(e) => {
            let a = base() + ru32(gm(G_VISMASK)) + 2 * e;
            ru16(a) & (1 << view_index()) != 0
        }
        None => false,
    }
}

fn visible(l: u32, x: i32, y: i32, z: i32) -> bool {
    if map_flags() & 2 != 0 {
        explored(l, x, y, z)
    } else {
        seen(l, x, y, z)
    }
}

// ---- Apply ---------------------------------------------------------------

fn slot_ok(u: u32) -> bool {
    if u == 0 {
        return false;
    }
    let d = u.wrapping_sub(ru32(gm(G_UNITS)));
    d % UNIT_SIZE == 0 && d / UNIT_SIZE < ru16(gm(G_POOLCOUNT))
}

fn unit_ok(u: u32) -> bool {
    slot_ok(u) && ru32(base() + u + U_DEF) != 0 && ru32(base() + u + U_PLAYER) != 0
}

fn record_share(t: u32, v: u32) {
    let args = [t, v];
    unsafe { record_sim_event(33, 0, args.as_ptr() as u32, 2) };
}

#[no_mangle]
pub extern "C" fn Apply(ev: u32, size: i32) -> i32 {
    if size < 4 {
        return 0;
    }
    let kind = ru32(ev);
    let need = match kind {
        128 | 129 | 130 | 137 | 133 | 134 | 135 => 8,
        131 => 28,
        132 => 20,
        136 | 139 => 12,
        138 => 16,
        _ => return 0,
    };
    if size < need as i32 {
        return 0;
    }
    let g = game();
    match kind {
        128 | 129 | 130 => {
            let u = ru32(ev + 4);
            if !unit_ok(u) {
                return 0;
            }
            let ua = base() + u;
            let s = unit_sight(ua);
            match kind {
                128 => move_sight(&s),
                129 => add_sight(&s, -1),
                _ => place_sight(&s),
            }
            1
        }
        131 => {
            let n = ri32(g + G_EYECOUNT);
            if map_flags() & 2 == 0 || n >= 20 || n < 0 {
                return 0;
            }
            let e = base() + ru32(g + G_EYES) + E_SIZE * n as u32;
            w32(e, (player_addr(view_index()) - base()) as u32);
            w32(e + 4, e + 32 - base());
            w16(e + 8, ru32(ev + 16));
            w8(e + 10, ru32(ev + 20));
            w32(e + 12, e + 11 - base());
            w32(e + 16, ru32(ev + 4));
            let mut y = ri32(ev + 8);
            let sea = (sea_level() + 1).wrapping_mul(65536);
            if y < sea {
                y = sea;
            }
            w32(e + 20, y as u32);
            w32(e + 24, ru32(ev + 12));
            w32(e + 28, ru32(g + G_TICKS).wrapping_add(ru32(ev + 24)));
            let s = eye_sight(e);
            place_sight(&s);
            w32(g + G_EYECOUNT, (n + 1) as u32);
            1
        }
        132 => {
            let width = ri32(g + G_WIDTH);
            let height = ri32(g + G_HEIGHT);
            let mut px = ru16(ev + 4) as i16 as i32;
            let mut py = ru16(ev + 8) as i16 as i32;
            let sx = ru16(ev + 12) as i16 as i32;
            let sy = ru16(ev + 16) as i16 as i32;
            let mut xmax = px + sx;
            let mut ymax = py + sy;
            if px < 0 {
                px = 0;
            }
            if py < 0 {
                py = 0;
            }
            if xmax >= width {
                xmax = width - 1;
            }
            if ymax >= height {
                ymax = height - 1;
            }
            if px >= xmax || py >= ymax {
                return 1;
            }
            let cells = base() + ru32(g + G_CELLS);
            let at = |c: i32, r: i32| cells + 13 * (r * width + c) as u32;
            let mut row = py;
            while row < ymax {
                let mut col = px;
                while col < xmax {
                    let c = at(col, row);
                    let mut lo = ru8(c + 4);
                    let mut hi = lo;
                    let mut see = |a: u32| {
                        let v = ru8(a + 4);
                        if v < lo {
                            lo = v;
                        }
                        if v > hi {
                            hi = v;
                        }
                    };
                    if col < width - 1 {
                        see(at(col + 1, row));
                    }
                    if row < height - 1 {
                        see(at(col, row + 1));
                    }
                    if col < width - 1 && row < height - 1 {
                        see(at(col + 1, row + 1));
                    }
                    w8(c + 6, lo);
                    w8(c + 5, hi);
                    col += 1;
                }
                row += 1;
            }
            1
        }
        133 => {
            recalculate(ru32(ev + 4));
            1
        }
        134 | 135 => {
            let p = ri32(ev + 4);
            if !(0..=9).contains(&p) {
                return 0;
            }
            if ru8(g + G_NETFLAGS) & 1 == 0 {
                return 0;
            }
            if ru32(player_addr(p as u32) + P_INFO) == 0 {
                return 0;
            }
            let info = info_of(player_addr(p as u32));
            let (m, t) = if kind == 134 { (0x20, 2) } else { (0x40, 4) };
            let a = info + I_FLAGS97;
            let nv = ru16(a) ^ m;
            w16(a, nv);
            record_share(t, nv & m);
            unsafe { broadcast_player_info() };
            1
        }
        136 => {
            let from = ri32(ev + 4);
            let to = ri32(ev + 8);
            if !(0..=9).contains(&from) || !(0..=9).contains(&to) {
                return 0;
            }
            let vm = base() + ru32(g + G_VISMASK);
            let n = ri32(g + G_WIDTH).wrapping_mul(ri32(g + G_HEIGHT)) / 4;
            let mut i = 0;
            while i < n {
                let a = vm + 2 * i as u32;
                let v = ru16(a);
                if v & (1 << from) != 0 {
                    w16(a, v | (1 << to));
                }
                i += 1;
            }
            1
        }
        137 | 138 => {
            let u = ru32(ev + 4);
            if !slot_ok(u) {
                return 0;
            }
            let c = region() + 5 * ((u - ru32(g + G_UNITS)) / UNIT_SIZE);
            if kind == 137 {
                w32(c, 0);
                w8(c + 4, 0);
            } else {
                let cell = ru32(ev + 8);
                w16(c, cell);
                w16(c + 2, cell >> 16);
                w8(c + 4, ru32(ev + 12));
            }
            1
        }
        _ => {
            // SET_MODE
            let clear = ru32(ev + 4);
            let set = ru32(ev + 8);
            if (clear | set) >> 3 != 0 {
                return 0;
            }
            set_map_flags((map_flags() & !clear) | set);
            1
        }
    }
}

fn recalculate(reset: u32) {
    let g = game();
    if reset != 0 {
        let n = ri32(g + G_WIDTH).wrapping_mul(ri32(g + G_HEIGHT)).wrapping_mul(2) / 4;
        let v = if map_flags() & 1 != 0 { 0 } else { 0xff };
        let vm = base() + ru32(g + G_VISMASK);
        let mut i = 0;
        while i < n {
            w8(vm + i as u32, v);
            i += 1;
        }
    }
    let fill = if map_flags() & 2 != 0 { 0 } else { 1 };
    for p in 0..10 {
        let pa = player_addr(p);
        let ty = ru8(pa + P_TYPE);
        if ru32(pa + P_ACTIVE) != 0
            && (1..=3).contains(&ty)
            && ru8(pa + P_INDEX) != 10
            && ru32(pa + P_EXPLORED) != 0
        {
            let xa = base() + ru32(pa + P_EXPLORED);
            let n = ri32(pa + P_EXPSIZE);
            let mut i = 0;
            while i < n {
                w8(xa + i as u32, fill);
                i += 1;
            }
        }
    }
    let last = last_slot();
    let mut k = 1;
    while k <= last {
        let u = unit_at(k);
        if ru16(u + U_DEFINDEX) != 0 && ru32(u + U_DEF) != 0 && ru32(u + U_PLAYER) != 0 {
            place_sight(&unit_sight(u));
        }
        k += 1;
    }
    let a = gm(G_FLAGS142F1);
    w16(a, ru16(a) | 4);
    set_map_flags(map_flags() & !8);
    record_radar_mapped();
    record_radar_units();
}

// ---- map cells -----------------------------------------------------------

fn cell_at(x: i32, y: i32) -> u32 {
    let g = game();
    let w = ri32(g + G_WIDTH);
    let h = ri32(g + G_HEIGHT);
    if x >= 0 && x < w && y >= 0 && y < h {
        base() + ru32(g + G_CELLS) + 13 * (y * w + x) as u32
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn GetMapCell(x: i32, y: i32) -> u32 {
    cell_at(x, y)
}

#[no_mangle]
pub extern "C" fn GetMapCellAtPosition(pos: u32) -> u32 {
    cell_at(ri32(pos) >> 20, ri32(pos + 8) >> 20)
}

#[no_mangle]
pub extern "C" fn GetOriginCellAtPosition(pos: u32) -> u32 {
    let mut x = ri32(pos) >> 20;
    let mut z = ri32(pos + 8) >> 20;
    let c = cell_at(x, z);
    if c == 0 {
        return 0;
    }
    if ru16(c + 8) == 0xfffe {
        x -= ru8(c + 11) as i32;
        z -= ru8(c + 10) as i32;
        return cell_at(x, z);
    }
    c
}

fn height_at(p: u32) -> i32 {
    let sea = sea_level();
    let mut h = unsafe { get_ground_height(p) };
    if h > sea {
        h = unsafe { get_ground_height(p) };
        h.wrapping_mul(65536)
    } else {
        sea.wrapping_mul(65536)
    }
}

#[no_mangle]
pub extern "C" fn ClampWorldPosToTerrain(x: i32, y: i32, out: u32) {
    let g = game();
    let bx = ri32(g + G_BASEX);
    let by = ri32(g + G_BASEY);
    let mut x = x;
    let mut y = y;
    if x < 0 {
        x = 0;
    } else if x > bx - 1 {
        x = bx - 1;
    }
    if y < 0 {
        y = 0;
    } else if y > by - 1 {
        y = by - 1;
    }
    let mut p = [0i32; 3];
    let pp = p.as_mut_ptr() as u32;
    let row = |p: &[i32; 3]| (p[2] >> 16) - ((p[1] >> 16) >> 1);
    p[0] = x.wrapping_mul(65536);
    let t = y & !15;
    let mut found = false;
    let mut s1 = 0;
    let mut i = 128;
    while i >= 0 {
        p[2] = (t + i).wrapping_mul(65536);
        p[1] = height_at(pp);
        s1 = row(&p);
        if s1 <= y {
            found = true;
            break;
        }
        i -= 16;
    }
    if found {
        let mut q = p;
        q[2] = p[2].wrapping_add(0x100000);
        q[1] = height_at(q.as_mut_ptr() as u32);
        let s2 = row(&q);
        if (s1 < s2 || y <= s2) && s2 != s1 {
            p[2] = p[2].wrapping_add(((y - s1).wrapping_mul(1 << 20)).wrapping_div(s2 - s1));
            p[1] = height_at(pp);
        }
    }
    w32(out, p[0] as u32);
    w32(out + 4, p[1] as u32);
    w32(out + 8, p[2] as u32);
}

// ---- queries -------------------------------------------------------------

#[no_mangle]
pub extern "C" fn IsUnitVisibleToPlayer(player: u32, u: u32) -> i32 {
    visible_to(player, u) as i32
}

fn visible_to(player: u32, u: u32) -> bool {
    if ru32(u + U_PLAYER) == player.wrapping_sub(base()) {
        return true;
    }
    if ru8(u + U_ACTIVATE) & 4 != 0 {
        return false;
    }
    let d = def_of(u);
    let qx = ri32(d + D_MINX).wrapping_add(ri32(u + U_POS));
    let qy = ri32(d + D_MAXY).wrapping_add(ri32(u + U_POS + 4));
    let qz = ri32(d + D_MINZ).wrapping_add(ri32(u + U_POS + 8));
    if ru32(u + U_FLAGS) & 0x200 == 0 && qy < sea_level().wrapping_mul(65536) {
        return false;
    }
    let sx = ri32(d + D_SIZEX);
    let sy = ri32(d + D_SIZEY);
    let sz = ri32(d + D_SIZEZ);
    visible(player, qx, qy, qz)
        || visible(player, qx.wrapping_add(sx), qy, qz)
        || visible(player, qx.wrapping_add(sx), qy.wrapping_sub(sy), qz.wrapping_add(sz))
        || visible(player, qx, qy.wrapping_sub(sy), qz.wrapping_add(sz))
}

#[no_mangle]
pub extern "C" fn SoundHeardMask(at: u32) -> i32 {
    let x = ri32(at);
    let y = ri32(at + 4);
    let z = ri32(at + 8);
    if cell_at(x / 1048576, z / 1048576) == 0 {
        return 0;
    }
    let mut r = 0;
    for p in 0..10u32 {
        let pa = player_addr(p);
        if let Some(e) = grid_entry(pa, x, y, z) {
            let hit = if map_flags() & 2 != 0 {
                ru8(base() + ru32(pa + P_EXPLORED) + e) != 0
            } else {
                ru16(base() + ru32(gm(G_VISMASK)) + 2 * e) & (1 << p) != 0
            };
            if hit {
                r |= 1 << p;
            }
        }
    }
    r
}

#[no_mangle]
pub extern "C" fn IsPointVisible(player: u32, pos: u32) -> i32 {
    seen(player, ri32(pos), ri32(pos + 4), ri32(pos + 8)) as i32
}

#[no_mangle]
pub extern "C" fn UpdateUnitLineOfSight(u: u32) {
    if slot_of(u) >= ru16(gm(G_POOLCOUNT)) as u32 {
        return;
    }
    move_sight(&unit_sight(u));
}

// ---- radar snapshots -----------------------------------------------------

fn unit_shown(u: u32, enabled: bool) -> bool {
    ru16(u + U_DEFINDEX) != 0
        && (enabled || ru32(u + U_FLAGS) & 0x300 != 0 || ru8(u + U_PLAYERINDEX) == view_index())
}

/// (px, py, kind, colour) of a shown projectile.
fn proj_shown(j: i32, v: u32) -> Option<(i32, i32, u32, u32)> {
    let p = base() + ru32(gm(G_PROJ)) + 107 * j as u32;
    let px = ri16(p + 6);
    let py = ri16(p + 14) - (ri16(p + 10) >> 1);
    let wd = base() + ru32(p);
    let wf = ru32(wd + 273);
    let on_radar = match grid_entry_t(v, px >> 5, py >> 5) {
        Some(e) if map_flags() & 2 != 0 => ru8(base() + ru32(v + P_EXPLORED) + e) != 0,
        Some(e) => ru16(base() + ru32(gm(G_VISMASK)) + 2 * e) & (1 << view_index()) != 0,
        None => false,
    };
    let vi = view_index();
    if wf & 0x6000_0000 == 0 {
        if wf & 0x40 == 0 && (on_radar || ru8(p + 102) == vi) {
            Some((px, py, 1, 0))
        } else {
            None
        }
    } else {
        let att = ru32(p + 82);
        if on_radar || (att != 0 && ru8(base() + att + U_PLAYERINDEX) == vi) {
            let col = ru8(info_of(player_addr(ru8(p + 102))) + I_COLOR);
            Some((px, py, 2, col))
        } else {
            None
        }
    }
}

fn capture(bytes: u32) -> (u32, u32) {
    let mut off: u32 = 0;
    let p = unsafe { sim_capture_alloc(bytes as i32, &mut off as *mut u32 as u32) };
    (p, unsafe { read_volatile(&off) })
}

fn record_radar_units() {
    let g = game();
    let enabled = map_flags() & 3 == 0 || ru8(g + G_MATCHFLAGS) & 2 != 0;
    let last = last_slot();
    let vp = player_addr(view_index());
    let mut nu = 0u32;
    let mut k = 0;
    while k <= last {
        if unit_shown(unit_at(k), enabled) {
            nu += 1;
        }
        k += 1;
    }
    let pc = ri32(g + G_PROJCOUNT);
    let mut nj = 0u32;
    let mut j = 0;
    while j < pc {
        if proj_shown(j, vp).is_some() {
            nj += 1;
        }
        j += 1;
    }
    let (snap, off) = capture(8 + 48 * nu + 12 * nj);
    if snap == 0 {
        return;
    }
    w32(snap, nu);
    w32(snap + 4, nj);
    let mut a = snap + 8;
    k = 0;
    while k <= last {
        let u = unit_at(k);
        if unit_shown(u, enabled) {
            let d = def_of(u);
            w16(a, k);
            w16(a + 2, ru16(u + U_ID));
            w16(a + 4, ri16(u + U_POS + 2) as u32);
            let row = ri16(u + U_POS + 10) - (ri16(u + U_POS + 6) >> 1);
            w32(a + 8, row as u32);
            w8(a + 12, ru8(info_of(player_addr(ru8(u + U_PLAYERINDEX))) + I_COLOR));
            let mut f = 0;
            if ru8(u + U_DAMAGED) != 0 {
                f |= 1;
            }
            if ru8(u + U_ACTIVATE) & 1 != 0 || ru8(d + D_FLAGS2) & 4 == 0 {
                f |= 2;
            }
            let f1 = ru32(d + D_FLAGS1);
            if f1 & 0x2000_0000 != 0 {
                f |= 4;
            }
            w8(a + 13, f);
            w16(a + 14, ru16(d + D_RADAR));
            w16(a + 16, ru16(d + D_SONAR));
            w16(a + 18, ru16(d + D_RADARJAM));
            w16(a + 20, ru16(d + D_SONARJAM));
            if f1 & 0x2000_0000 != 0 {
                for w in 0..3u32 {
                    let wr = ru32(u + 16 + 28 * w);
                    if wr != 0 {
                        let wd = base() + wr;
                        let r = a + 24 + 8 * w;
                        w32(r, ru32(wd + 224));
                        w8(r + 4, (ru32(wd + 273) >> 30) & 1);
                        w8(r + 5, (ru8(u + 30 + 28 * w) != 0) as u32);
                    }
                }
            }
            a += 48;
        }
        k += 1;
    }
    j = 0;
    while j < pc {
        if let Some((px, py, kind, col)) = proj_shown(j, vp) {
            w32(a, px as u32);
            w32(a + 4, py as u32);
            w8(a + 8, kind);
            w8(a + 9, col);
            a += 12;
        }
        j += 1;
    }
    let args = [off];
    unsafe { record_sim_event(34, 0, args.as_ptr() as u32, 1) };
}

fn record_radar_mapped() {
    let g = game();
    let fa = g + G_FLAGS142F1;
    if ru16(fa) & 4 == 0 {
        return;
    }
    w16(fa, ru16(fa) & !4);
    let width = ru32(g + G_WIDTH);
    let height = ru32(g + G_HEIGHT);
    let c = (width as i32 / 2).wrapping_mul(height as i32 / 2) as u32;
    let (snap, off) = capture(12 + c);
    if snap == 0 {
        return;
    }
    w32(snap, width);
    w32(snap + 4, height);
    w32(snap + 8, c);
    let vm = base() + ru32(g + G_VISMASK);
    let vp = player_addr(view_index());
    let xa = base() + ru32(vp + P_EXPLORED);
    let bit = 1u32 << view_index();
    let mut i = 0;
    while i < c {
        let v = if ru16(vm + 2 * i) & bit == 0 {
            0
        } else if ru8(xa + i) != 0 {
            1
        } else {
            2
        };
        w8(snap + 12 + i, v);
        i += 1;
    }
    let args = [off];
    unsafe { record_sim_event(35, 0, args.as_ptr() as u32, 1) };
}

#[no_mangle]
pub extern "C" fn RecordRadarUnits() {
    record_radar_units();
}

#[no_mangle]
pub extern "C" fn RecordRadarMapped() {
    record_radar_mapped();
}

// ---- sensors -------------------------------------------------------------

/// The distance of the spec's section 4, from position `a` (x, z) to unit `u`.
fn distance(ax: i32, az: i32, u: u32) -> i32 {
    let dx = ri32(u + U_POS).wrapping_sub(ax) as i64;
    let dz = ri32(u + U_POS + 8).wrapping_sub(az) as i64;
    let a = ((dx * dx) as u64 >> 32) as u32;
    let b = ((dz * dz) as u64 >> 32) as u32;
    a.wrapping_add(b) as i32
}

fn each_in_range<F: FnMut(u32)>(pos: u32, range: i32, mut f: F) {
    let mut ids = [0u16; 256];
    let ip = ids.as_mut_ptr() as u32;
    let mut skip = 0;
    loop {
        let n = unsafe { units_in_range(pos, range, skip, ip, 256) };
        let m = if n > 256 { 256 } else { n };
        let mut i = 0;
        while i < m {
            let id = unsafe { read_volatile(&ids[i as usize]) } as u32;
            f(unit_at(id));
            i += 1;
        }
        if n >= 256 {
            skip += n;
        } else {
            break;
        }
    }
}

#[no_mangle]
pub extern "C" fn UpdateSensorRadarAndCloak() {
    let g = game();
    if ru16(g + G_NUMPLAYERS) < 2 {
        return;
    }
    let v = view_index();
    let pl = player_addr(v);
    let pidx = ru8(pl + P_INDEX);
    let last = last_slot();
    let sea = sea_level().wrapping_mul(65536);
    // pass 1
    let mut k = 1;
    while k <= last {
        let a = unit_at(k);
        k += 1;
        let f = ru32(a + U_FLAGS);
        if f & ALIVE == 0 {
            continue;
        }
        let mut f = f & !0x1000;
        let ap = ru32(a + U_PLAYER);
        let allied_share = ap != 0 && {
            let apa = base() + ap;
            ru8(apa + P_ALLIED + pidx) != 0 && ru8(info_of(apa) + I_FLAGS97) & 0x40 != 0
        };
        let watch = ru32(pl + P_ACTIVE) != 0 && ru8(info_of(pl) + I_FLAGS9B) & 0x40 != 0;
        if ru8(a + U_PLAYERINDEX) == v || allied_share || watch {
            f |= 0x300;
        } else {
            f &= !0x700;
        }
        w32(a + U_FLAGS, f);
    }
    // pass 2
    let ub = ru32(pl + P_UNITSBEGIN);
    let ue = ru32(pl + P_UNITSEND);
    if ub != 0 && ue != 0 {
        let mut s = base() + ub;
        let se = base() + ue;
        while s <= se {
            let sa = s;
            s += UNIT_SIZE;
            if !ready(sa) || ru8(sa + U_ACTIVATE) & 1 == 0 || ru32(sa + U_DEF) == 0 {
                continue;
            }
            let d = def_of(sa);
            let r = ri16(d + D_RADAR);
            let n = ri16(d + D_SONAR);
            if r == 0 && n == 0 {
                continue;
            }
            let rr = r.wrapping_add(2 * ri16(sa + U_POS + 6)).wrapping_mul(r.wrapping_add(2 * ri16(sa + U_POS + 6)));
            let ss = n.wrapping_mul(n);
            let range = (if r > n { r } else { n }).wrapping_mul(65536);
            let (ax, az) = (ri32(sa + U_POS), ri32(sa + U_POS + 8));
            each_in_range(sa + U_POS, range, |t| {
                if ru16(t + U_DEFINDEX) == 0 || ru8(t + U_PLAYERINDEX) == v || ru32(t + U_DEF) == 0 {
                    return;
                }
                let td = def_of(t);
                if ru32(td + D_FLAGS1) & 0x100 != 0 {
                    return;
                }
                let dist = distance(ax, az, t);
                let ty = ri32(t + U_POS + 4);
                let mut f = ru32(t + U_FLAGS);
                if ty <= sea && dist < ss {
                    f |= 0x200;
                }
                if ri32(td + D_MAXY).wrapping_add(ty) >= sea && dist < rr {
                    f |= 0x100;
                }
                w32(t + U_FLAGS, f);
            });
        }
    }
    // pass 3
    k = 1;
    while k <= last {
        let j = unit_at(k);
        k += 1;
        if ru32(j + U_FLAGS) & ALIVE == 0
            || ru8(j + U_PLAYERINDEX) == pidx
            || ru8(j + U_ACTIVATE) & 1 == 0
            || ru32(j + U_DEF) == 0
        {
            continue;
        }
        let d = def_of(j);
        let rj = ri16(d + D_RADARJAM);
        if rj != 0 {
            each_in_range(j + U_POS, rj.wrapping_mul(65536), |t| {
                let f = ru32(t + U_FLAGS);
                w32(t + U_FLAGS, (f & !0x100) | 0x400);
            });
        }
        let sj = ri16(d + D_SONARJAM);
        if sj != 0 {
            each_in_range(j + U_POS, sj.wrapping_mul(65536), |t| {
                let f = ru32(t + U_FLAGS);
                w32(t + U_FLAGS, (f & !0x200) | 0x400);
            });
        }
    }
    // pass 4
    k = 1;
    while k <= last {
        let c = unit_at(k);
        k += 1;
        if ru32(c + U_FLAGS) & ALIVE == 0 || ru32(c + U_PLAYER) == 0 || ru32(c + U_DEF) == 0 {
            continue;
        }
        let cp = uplayer_of(c);
        let ty = ru8(cp + P_TYPE);
        if ru32(cp + P_ACTIVE) == 0 || !(ty == 1 || ty == 2) {
            continue;
        }
        let d = def_of(c);
        if ru32(d + D_FLAGS2) & 0x2000 == 0 {
            continue;
        }
        if has_ready(ru8(c + U_PLAYERINDEX), c + U_POS, ri16(d + D_MINCLOAK)) {
            w32(c + U_WORKTIME, ru32(g + G_TICKS).wrapping_add(90));
            w32(c + U_FLAGS, ru32(c + U_FLAGS) | 0x1000);
        }
    }
    // pass 5
    k = 1;
    while k <= last {
        let u = unit_at(k);
        k += 1;
        let f = ru32(u + U_FLAGS);
        if f & ALIVE == 0 || f & 0x100 != 0 || ru8(u + U_ACTIVATE) & 4 != 0 {
            continue;
        }
        if visible(pl, ri32(u + U_POS), ri32(u + U_POS + 4), ri32(u + U_POS + 8)) {
            w32(u + U_FLAGS, f | 0x100);
        }
    }
}

// ---- the unit index ------------------------------------------------------

fn index_base() -> u32 {
    let r = ru32(sim() + S_UNITINDEX);
    if r == 0 {
        0
    } else {
        base() + r
    }
}

const L_VISIBLE: u32 = 12;
const L_KNOWN: u32 = 28;

fn list_push(list: u32, u: u32) {
    let first = ru32(list + 4);
    let last = ru32(list + 8);
    let end = ru32(list + 12);
    if last == end {
        let s = last.wrapping_sub(first) / 4;
        let n = s + if s > 1 { s } else { 1 };
        let p = unsafe { sim_alloc(4 * n as i32) };
        let nf = p - base();
        let mut i = 0;
        while i < s {
            w32(p + 4 * i, ru32(base() + first + 4 * i));
            i += 1;
        }
        w32(p + 4 * s, u - base());
        if first != 0 {
            unsafe { sim_free(base() + first) };
        }
        w32(list + 4, nf);
        w32(list + 8, nf + 4 * (s + 1));
        w32(list + 12, nf + 4 * n);
    } else {
        w32(base() + last, u - base());
        w32(list + 8, last + 4);
    }
}

fn list_clear(list: u32) {
    w32(list + 8, ru32(list + 4));
}

#[no_mangle]
pub extern "C" fn CreateUnitIndex(p: u32) {
    if p >= 10 {
        return;
    }
    if ru32(sim() + S_UNITINDEX) == 0 {
        let a = unsafe { sim_alloc(440) };
        w32(sim() + S_UNITINDEX, a - base());
    }
    let e = index_base() + 44 * p;
    list_clear(e + L_VISIBLE);
    list_clear(e + L_KNOWN);
    w32(e + 8, 0);
    w32(e + 4, 0);
    w32(e, 1);
}

#[no_mangle]
pub extern "C" fn FreeUnitIndex(p: u32) {
    let ib = index_base();
    if ib == 0 || p >= 10 {
        return;
    }
    let e = ib + 44 * p;
    for l in [e + L_VISIBLE, e + L_KNOWN] {
        let first = ru32(l + 4);
        if first != 0 {
            unsafe { sim_free(base() + first) };
        }
        w32(l + 4, 0);
        w32(l + 8, 0);
        w32(l + 12, 0);
    }
    w32(e, 0);
}

#[no_mangle]
pub extern "C" fn RefreshUnitIndex(p: u32) {
    let ib = index_base();
    if ib == 0 || p >= 10 {
        return;
    }
    let e = ib + 44 * p;
    let ticks = ru32(gm(G_TICKS));
    if ru32(e) == 0 || ticks < ru32(e + 4).wrapping_add(30) {
        return;
    }
    list_clear(e + L_VISIBLE);
    list_clear(e + L_KNOWN);
    w32(e + 8, 0);
    let o = player_addr(p);
    let last = last_slot();
    let mut k = 1;
    while k <= last {
        let u = unit_at(k);
        k += 1;
        if !ready(u) {
            continue;
        }
        let up = uplayer_of(u);
        let uidx = ru8(up + P_INDEX);
        if ru8(o + P_ALLIED + uidx) == 0 {
            if visible_to(o, u) && ru32(u + U_FLAGS) & 0x8000 == 0 {
                list_push(e + L_VISIBLE, u);
            }
            if ru32(u + U_FLAGS) & 0x100 != 0 {
                list_push(e + L_KNOWN, u);
            }
        } else if uidx == ru8(o + P_INDEX)
            && f32::from_bits(ru32(u + U_BUILDLEFT)) == 0.0
            && ru32(def_of(u) + D_FLAGS1) & 0x400 != 0
            && ru8(u + U_ACTIVATE) & 1 != 0
        {
            w32(e + 8, 1);
        }
    }
    w32(e + 4, ticks);
}

fn scan_list(list: u32, ax: i32, az: i32, r2: i32, need_ready: bool, out: u32, cap: i32, c: &mut i32) {
    let mut a = base() + ru32(list + 4);
    let end = base() + ru32(list + 8);
    while a < end {
        let uo = ru32(a);
        let u = base() + uo;
        a += 4;
        if distance(ax, az, u) <= r2 && (!need_ready || ready(u)) {
            if *c < cap {
                w32(out + 4 * *c as u32, uo);
            }
            *c += 1;
        }
    }
}

#[no_mangle]
pub extern "C" fn GetVisibleEnemiesInRadius(p: u32, pos: u32, radius: i32, _flags: i32, out: u32, cap: i32) -> i32 {
    let ib = index_base();
    if ib == 0 || p >= 10 {
        return 0;
    }
    let e = ib + 44 * p;
    let r2 = radius.wrapping_mul(radius);
    let (ax, az) = (ri32(pos), ri32(pos + 8));
    let mut c = 0;
    scan_list(e + L_VISIBLE, ax, az, r2, true, out, cap, &mut c);
    if ru32(e + 8) != 0 && c == 0 {
        scan_list(e + L_KNOWN, ax, az, r2, true, out, cap, &mut c);
    }
    c
}

#[no_mangle]
pub extern "C" fn GetSeenUnitsInRadius(p: u32, pos: u32, radius: i32, out: u32, cap: i32) -> i32 {
    let ib = index_base();
    if ib == 0 || p >= 10 {
        return 0;
    }
    let e = ib + 44 * p;
    let r2 = radius.wrapping_mul(radius);
    let mut c = 0;
    scan_list(e + L_VISIBLE, ri32(pos), ri32(pos + 8), r2, false, out, cap, &mut c);
    c
}

fn has_ready(p: u32, pos: u32, range: i32) -> bool {
    let ib = index_base();
    if ib == 0 || p >= 10 {
        return false;
    }
    let e = ib + 44 * p;
    let r2 = range.wrapping_mul(range);
    let (ax, az) = (ri32(pos), ri32(pos + 8));
    let mut a = base() + ru32(e + L_VISIBLE + 4);
    let end = base() + ru32(e + L_VISIBLE + 8);
    while a < end {
        let u = base() + ru32(a);
        a += 4;
        if distance(ax, az, u) <= r2 && ready(u) {
            return true;
        }
    }
    false
}

#[no_mangle]
pub extern "C" fn HasReadyUnitInRange(p: u32, pos: u32, range: i32) -> i32 {
    has_ready(p, pos, range) as i32
}

// ---- eyeballs ------------------------------------------------------------

#[no_mangle]
pub extern "C" fn ExpireEyeballs() {
    let g = game();
    let n = ri32(g + G_EYECOUNT);
    if !(0..=20).contains(&n) {
        return;
    }
    let eyes = base() + ru32(g + G_EYES);
    let ticks = ru32(g + G_TICKS);
    let mut i = 0;
    while i < n {
        let e = eyes + E_SIZE * i as u32;
        if ru32(e + 28) < ticks {
            add_sight(&eye_sight(e), -1);
        }
        i += 1;
    }
    let mut kept = 0u32;
    i = 0;
    while i < n {
        let e = eyes + E_SIZE * i as u32;
        if ru32(e + 28) >= ticks {
            let d = eyes + E_SIZE * kept;
            if d != e {
                let mut b = 0;
                while b < E_SIZE {
                    w8(d + b, ru8(e + b));
                    b += 1;
                }
            }
            w32(d + 4, d + 32 - base());
            w32(d + 12, d + 11 - base());
            kept += 1;
        }
        i += 1;
    }
    w32(g + G_EYECOUNT, kept);
}

// ---- region --------------------------------------------------------------

#[no_mangle]
pub extern "C" fn RegionSize(map: u32) -> i32 {
    ri32(map).wrapping_mul(5)
}

#[no_mangle]
pub extern "C" fn Init(_map: u32, _region: u32, _size: i32) {
}

#[no_mangle]
pub extern "C" fn Shutdown() -> u32 {
    let f = unsafe { read_volatile(&raw const NOP) };
    f();
    let r = region();
    r
}

// A function pointer keeps the table import (ABI 9.1).
fn nop() {}
static NOP: fn() = nop;
