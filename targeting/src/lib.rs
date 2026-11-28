#![no_std]

use core::ptr::{addr_of_mut, read_unaligned, read_volatile, write_unaligned};

#[link(wasm_import_module = "map")]
extern "C" {
    #[link_name = "GetGroundHeight"]
    fn get_ground_height(pos: u32) -> i32;
}
#[link(wasm_import_module = "scripts")]
extern "C" {
    #[link_name = "GetSweetSpot"]
    fn get_sweet_spot(unit: u32, out: u32);
}
#[link(wasm_import_module = "weapons")]
extern "C" {
    #[link_name = "GetWeaponRange"]
    fn get_weapon_range(unit: u32, weapon: i32) -> i32;
    #[link_name = "WeaponCanReachUnit"]
    fn weapon_can_reach_unit(unit: u32, target: u32, weapon: i32) -> i32;
}
#[link(wasm_import_module = "los_radar")]
extern "C" {
    #[link_name = "GetVisibleEnemiesInRadius"]
    fn get_visible_enemies(player: i32, pos: u32, radius: i32, flags: i32, out: u32, cap: i32) -> i32;
}
#[link(wasm_import_module = "services")]
extern "C" {
    #[link_name = "RandomInt"]
    fn random_int(range: i32) -> i32;
    #[link_name = "Emit"]
    fn emit(effect: u32, size: i32);
}

#[no_mangle]
pub static mut g_game: u32 = 0;
#[no_mangle]
pub static mut g_simGlobals: u32 = 0;
#[no_mangle]
pub static mut g_extTdf: u32 = 0x0066_6474;
#[no_mangle]
pub static mut g_blockBase: u32 = 0;

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
fn ri32(a: u32) -> i32 {
    unsafe { read_unaligned(a as *const i32) }
}
#[inline(always)]
fn ru32(a: u32) -> u32 {
    unsafe { read_unaligned(a as *const u32) }
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
// The address of the record a block offset names, 0 for null.
#[inline(always)]
fn blk(off: u32) -> u32 {
    if off == 0 { 0 } else { base().wrapping_add(off) }
}

// ---- layout (layout.json) ------------------------------------------------

const G_UNITS: u32 = 82775;
const G_POOL: u32 = 82769;
const G_SEA: u32 = 82559;
const G_UIOPT: u32 = 229167;
const UNIT_SIZE: u32 = 280;
const U_MOTION: u32 = 0;
const U_POS: u32 = 106;
const U_DEF: u32 = 146;
const U_PLAYER: u32 = 150;
const U_DEFIDX: u32 = 166;
const U_ID: u32 = 168;
const U_KILLS: u32 = 184;
const U_DIRTY: u32 = 186;
const U_PIDX: u32 = 255;
const U_ACT: u32 = 270;
const U_FLAGS: u32 = 272;
const SLOT: u32 = 4;
const SLOT_SIZE: u32 = 28;

#[inline(always)]
fn slot(u: u32, w: u32) -> u32 {
    u + SLOT + SLOT_SIZE * w
}

fn units_off() -> u32 {
    ru32(game() + G_UNITS)
}

// The address of the unit a block offset names, or 0 when it is not a unit of the pool.
fn unit_at(off: u32) -> u32 {
    if off == 0 {
        return 0;
    }
    let first = units_off();
    let d = off.wrapping_sub(first);
    if off < first || d % UNIT_SIZE != 0 || d / UNIT_SIZE >= ru16(game() + G_POOL) {
        return 0;
    }
    base().wrapping_add(off)
}

fn has_target(s: u32) -> bool {
    !(ru16(s) == 0 && ru16(s + 2) == 0x8000)
}

fn clear_slot(s: u32) {
    w16(s, 0);
    w16(s + 2, 0x8000);
}

fn publish_cleared(u: u32, w: u32) {
    let mut e = [112u32, u.wrapping_sub(base()), w];
    unsafe { emit(addr_of_mut!(e) as u32, 12) };
}

fn in_set(set_off: u32, i: u32) -> bool {
    let s = blk(set_off);
    if s == 0 || i >= 512 {
        return false;
    }
    (ru8(s + (i >> 3)) >> (i & 7)) & 1 != 0
}

// Keeps the table import and element segment the ABI requires; not observable.
static KEEP: [extern "C" fn(u32, u32) -> u32; 1] = [keep_fn];

extern "C" fn keep_fn(_u: u32, _w: u32) -> u32 {
    0
}

// ---- Apply ---------------------------------------------------------------

#[no_mangle]
pub extern "C" fn Apply(ev: u32, size: u32) -> i32 {
    unsafe { read_volatile(&raw const KEEP) };
    if size < 12 {
        return 0;
    }
    let kind = ri32(ev);
    let payload = match kind {
        96 => 16,
        97 => 24,
        98 | 99 => 12,
        100 => 20,
        _ => return 0,
    };
    if size < payload {
        return 0;
    }
    let u = unit_at(ru32(ev + 4));
    let w = ri32(ev + 8);
    if u == 0 || !(0..=2).contains(&w) {
        return 0;
    }
    let w = w as u32;
    let s = slot(u, w);
    match kind {
        96 => {
            let t = unit_at(ru32(ev + 12));
            if t == 0 {
                return 0;
            }
            w16(s, ru16(t + U_ID));
            w16(s + 2, 0x8000);
            w16(u + U_DIRTY, ru16(u + U_DIRTY) & 0x83ff);
            1
        }
        97 => {
            let x = ri32(ev + 12) >> 16;
            let mut z = ri32(ev + 20) >> 16;
            if z as i16 == i16::MIN {
                z = -32767;
            }
            w16(s, x as u32);
            w16(s + 2, z as u32);
            w16(u + U_DIRTY, ru16(u + U_DIRTY) & 0x83ff);
            1
        }
        98 => {
            if !has_target(s) {
                return 0;
            }
            clear_slot(s);
            publish_cleared(u, w);
            1
        }
        99 => {
            if !has_target(s) {
                return 0;
            }
            clear_slot(s);
            1
        }
        _ => {
            w16(s, ru32(ev + 12));
            w16(s + 2, ru32(ev + 16));
            1
        }
    }
}

// ---- queries -------------------------------------------------------------

#[no_mangle]
pub extern "C" fn GetWeaponTargetUnit(u: u32, w: i32) -> u32 {
    let s = slot(u, w as u32);
    let x = ru16(s);
    if ru16(s + 2) != 0x8000 || x == 0 {
        return 0;
    }
    base() + units_off() + (x as u16 as i16 as i32 as u32).wrapping_mul(UNIT_SIZE)
}

// floor(sqrt(s)) of a double, rounded to nearest as a double and truncated, for an integer-valued s.
fn trunc_sqrt(s: f64) -> u64 {
    let n = s as u64;
    // integer square root
    let mut q = 0u64;
    let mut bit = 1u64 << 62;
    let mut rem = n;
    while bit > rem {
        bit >>= 2;
    }
    while bit != 0 {
        if rem >= q + bit {
            rem -= q + bit;
            q = (q >> 1) + bit;
        } else {
            q >>= 1;
        }
        bit >>= 2;
    }
    if q * q == n {
        return q;
    }
    // sqrt(n) is in (q, q+1): does it round up to q+1 as a double?
    let k = q + 1;
    let e = 63 - k.leading_zeros();
    let j = if k & (k - 1) == 0 { 53 - e as i32 } else { 52 - e as i32 };
    let d = (k * k - n) as u128;
    if j >= 0 && (d << j) < k as u128 {
        k
    } else {
        q
    }
}

#[no_mangle]
pub extern "C" fn GetWeaponTargetPos(u: u32, pos: u32, w: i32) -> i32 {
    let s = slot(u, w as u32);
    if !has_target(s) {
        return 0;
    }
    let zf = ru16(s + 2);
    let mut v = [0i32; 3];
    let vp = addr_of_mut!(v) as u32;
    if zf != 0x8000 {
        let x = ((ru16(s) as u16 as i16) as i32) << 16;
        let z = ((zf as u16 as i16) as i32) << 16;
        v[0] = x;
        v[2] = z;
        let sea = ru8(game() + G_SEA) as i32;
        let mut h = unsafe { get_ground_height(vp) };
        if h > sea {
            h = unsafe { get_ground_height(vp) };
        } else {
            h = sea;
        }
        w32(pos, x as u32);
        w32(pos + 4, (h as u32) << 16);
        w32(pos + 8, z as u32);
        return 1;
    }
    let t = base() + units_off() + (ru16(s) as u16 as i16 as i32 as u32).wrapping_mul(UNIT_SIZE);
    if ru16(t + U_DEFIDX) == 0 {
        clear_slot(s);
        publish_cleared(u, w as u32);
        return 0;
    }
    unsafe { get_sweet_spot(t, vp) };
    let motion = blk(ru32(t + U_MOTION));
    if ru8(s + 27) & 2 != 0 && motion != 0 && ru16(u + U_KILLS) > 5 {
        let wd = blk(ru32(s + 12));
        let speed = ri32(wd + 104);
        if ru32(wd + 273) & 0x0200_0000 == 0 && speed != 0 {
            let up = u + U_POS;
            let dx = ri32(up).wrapping_sub(v[0]) as f64;
            let dy = ri32(up + 4).wrapping_sub(v[1]) as f64;
            let dz = ri32(up + 8).wrapping_sub(v[2]) as f64;
            let sum = (dx * dx + dy * dy) + dz * dz;
            let dist = trunc_sqrt(sum) as u32 as i32;
            let tt = ((dist as i64) * 65536 / (speed as i64)) as i32;
            let t2 = ((tt as i64) * 0xcccc) >> 16;
            for a in 0..3u32 {
                let vel = ri32(motion + 8 + 4 * a) as i64;
                let off = (vel * t2) >> 16;
                v[a as usize] = v[a as usize].wrapping_add(off as i32);
            }
        }
    }
    w32(pos, v[0] as u32);
    w32(pos + 4, v[1] as u32);
    w32(pos + 8, v[2] as u32);
    1
}

// ---- the search ----------------------------------------------------------

const CAP: usize = 6144;
static mut LIST: [u32; CAP] = [0; CAP];

fn hi(a: i32) -> i32 {
    (((a as i64) * (a as i64)) >> 32) as i32
}

fn search(u: u32, w: u32, use_range: i32) -> u32 {
    let def = blk(ru32(u + U_DEF));
    let r = if use_range != 0 {
        unsafe { get_weapon_range(u, w as i32) }
    } else {
        ru16(def + 514) as u16 as i16 as i32
    };
    let list = addr_of_mut!(LIST) as *mut u32;
    let count = unsafe {
        get_visible_enemies(ru8(u + U_PIDX) as i32, u + U_POS, r, 0, list as u32, CAP as i32)
    };
    let mut rem = if count < 0 { 0 } else { (count as usize).min(CAP) };
    let total = rem;

    let player = blk(ru32(u + U_PLAYER));
    let computer = player != 0 && ri32(player) != 0 && ru8(player + 115) == 2;
    let ui = ru16(game() + G_UIOPT) & 0x400 != 0;
    let skip_reach = ru32(def + 577) & 0x1000_0000 != 0;
    let s = slot(u, w);
    let ucat = ru32(def + 573);
    let wcat = ru32(def + 561 + 4 * w);
    let paralyzer = {
        let wd = blk(ru32(s + 12));
        wd != 0 && ru32(wd + 273) & 0x80 != 0
    };
    let mut best_pref = 0u32;
    let mut best_pref_d = 0i32;
    let mut best_fall = 0u32;
    let mut best_fall_d = 0i32;

    let mut k = 0;
    while k < 50 && rem > 0 {
        k += 1;
        let i = unsafe { random_int(rem as i32) } as usize;
        let cand = unsafe { *list.add(i) };
        unsafe { *list.add(i) = *list.add(rem - 1) };
        rem -= 1;
        let t = blk(cand);
        let f = ru32(t + U_FLAGS);
        if f & 0x1000_0000 == 0 || f & 0x4000 != 0 {
            continue;
        }
        let tdef = blk(ru32(t + U_DEF));
        if !(computer || ui || (tdef != 0 && ru32(tdef + 577) & 0x8000 != 0)) {
            continue;
        }
        if !skip_reach && unsafe { weapon_can_reach_unit(u, t, w as i32) } == 0 {
            continue;
        }
        let idx = ru16(t + U_DEFIDX);
        if use_range == 0 && in_set(ucat, idx) {
            continue;
        }
        if paralyzer && ru8(t + U_ACT) & 0x10 != 0 {
            continue;
        }
        let up = u + U_POS;
        let tp = t + U_POS;
        let q = hi(ri32(up).wrapping_sub(ri32(tp))).wrapping_add(hi(ri32(up + 8).wrapping_sub(ri32(tp + 8))));
        let d = unsafe { random_int(q) };
        if in_set(wcat, idx) {
            if best_fall == 0 || d < best_fall_d {
                best_fall = cand;
                best_fall_d = d;
            }
        } else if best_pref == 0 || d < best_pref_d {
            best_pref = cand;
            best_pref_d = d;
        }
    }
    // the list is a static: leave it as it was at instantiation
    for i in 0..total {
        unsafe { *list.add(i) = 0 };
    }
    let best = if best_pref != 0 { best_pref } else { best_fall };
    blk(best)
}

#[no_mangle]
pub extern "C" fn FindWeaponTarget(u: u32, w: i32, use_range: i32) -> u32 {
    search(u, (w & 0xff) as u32, use_range)
}

#[no_mangle]
pub extern "C" fn FindBestTargetIfFireAtWill(u: u32) -> u32 {
    if (ru32(u + U_FLAGS) >> 20) & 3 == 2 {
        search(u, 0, 0)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn ChooseWeapon(u: u32) -> i32 {
    for w in 0..3 {
        if ru8(slot(u, w) + 27) & 2 != 0 {
            return w as i32;
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn SavedTargetPair(u: u32, w: i32) -> i32 {
    let s = slot(u, w as u32);
    (ru16(s) | (ru16(s + 2) << 16)) as i32
}
