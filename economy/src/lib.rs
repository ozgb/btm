#![no_std]

use core::ptr::{read_unaligned, read_volatile, write_unaligned, write_volatile};

#[link(wasm_import_module = "econworld")]
extern "C" {
    #[link_name = "Tick"]
    fn w_tick() -> i32;
    #[link_name = "Difficulty"]
    fn w_difficulty() -> i32;
    #[link_name = "SharingEnabled"]
    fn w_sharing_enabled() -> i32;
    #[link_name = "Wind"]
    fn w_wind() -> f32;
    #[link_name = "Tide"]
    fn w_tide() -> f32;
    #[link_name = "PlayerInfo"]
    fn w_player_info(p: i32, out: *mut i32);
    #[link_name = "Allied"]
    fn w_allied(a: i32, b: i32) -> i32;
    #[link_name = "FirstUnit"]
    fn w_first_unit(p: i32) -> i32;
    #[link_name = "LastUnit"]
    fn w_last_unit(p: i32) -> i32;
    #[link_name = "NextUnit"]
    fn w_next_unit(u: i32) -> i32;
    #[link_name = "UnitSlot"]
    fn w_unit_slot(u: i32) -> i32;
    #[link_name = "UnitInfo"]
    fn w_unit_info(u: i32, out: *mut i32) -> i32;
    #[link_name = "TypeInfo"]
    fn w_type_info(t: i32, out: *mut i32);
    #[link_name = "CellMetal"]
    fn w_cell_metal(x: i32, y: i32) -> i32;
    #[link_name = "Emit"]
    fn w_emit(p: *const i32, size: i32);
}

#[link(wasm_import_module = "network")]
extern "C" {
    #[link_name = "SendShareMapInfo"]
    fn n_send_share_map_info(a: i32, b: i32);
}

#[no_mangle]
pub static mut g_game: u32 = 0;
#[no_mangle]
pub static mut g_simGlobals: u32 = 0;
#[no_mangle]
pub static mut g_extTdf: [u8; 8] = *b"tdf\0\0\0\0\0";
#[no_mangle]
pub static mut g_blockBase: u32 = 0;
#[no_mangle]
pub static mut g_region: u32 = 0;

fn ident(x: u32) -> u32 {
    x
}
// A function pointer, so that the module imports the table as its manifest entry lists.
static IDENT: fn(u32) -> u32 = ident;

// Set once by Init (before the first tick).
static mut UNIT_SLOTS: i32 = 0;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// ---- memory helpers ----

fn game() -> u32 {
    unsafe { read_volatile(&raw const g_game) }
}
fn base() -> u32 {
    unsafe { read_volatile(&raw const g_blockBase) }
}
fn region() -> u32 {
    unsafe { read_volatile(&raw const g_region) }
}
fn unit_slots() -> i32 {
    unsafe { read_volatile(&raw const UNIT_SLOTS) }
}
fn rd32(a: u32) -> u32 {
    unsafe { read_unaligned(a as *const u32) }
}
fn wr32(a: u32, v: u32) {
    unsafe { write_unaligned(a as *mut u32, v) }
}
fn rf(a: u32) -> f32 {
    f32::from_bits(rd32(a))
}
fn wf(a: u32, v: f32) {
    wr32(a, v.to_bits())
}
fn rd(a: u32) -> f64 {
    f64::from_bits(unsafe { read_unaligned(a as *const u64) })
}
fn wd(a: u32, v: f64) {
    unsafe { write_unaligned(a as *mut u64, v.to_bits()) }
}

// Player record fields.
const P_BASE: u32 = 7011;
const P_SIZE: u32 = 331;
const P_ENERGY: u32 = 140;
const P_ENERGY_INCOME: u32 = 144;
const P_ENERGY_USAGE: u32 = 148;
const P_METAL: u32 = 152;
const P_METAL_INCOME: u32 = 156;
const P_METAL_USAGE: u32 = 160;
const P_ENERGY_CAP: u32 = 164;
const P_METAL_CAP: u32 = 168;
const P_TOT_E_PROD: u32 = 172;
const P_TOT_M_PROD: u32 = 180;
const P_TOT_E_CONS: u32 = 188;
const P_TOT_M_CONS: u32 = 196;
const P_E_WASTED: u32 = 204;
const P_M_WASTED: u32 = 212;
const P_E_BONUS: u32 = 220;
const P_M_BONUS: u32 = 224;
const P_SHARE_METAL: u32 = 228;
const P_SHARE_ENERGY: u32 = 232;
const P_ECON: u32 = 236;
const P_INDEX: u32 = 326;

fn player_addr(p: u32) -> u32 {
    game().wrapping_add(P_BASE).wrapping_add(P_SIZE.wrapping_mul(p))
}

// Account layout: energy ledger at 0, metal ledger at 24; a ledger is make, use, usePaid, stall, makePrev, usePrev.
const L_MAKE: u32 = 0;
const L_USE: u32 = 4;
const L_PAID: u32 = 8;
const L_STALL: u32 = 12;
const L_MAKEPREV: u32 = 16;
const L_USEPREV: u32 = 20;
const METAL: u32 = 24;

// ---- arithmetic ----

fn allow(stall: f32) -> bool {
    stall <= 0.0 || stall.is_nan()
}

// Income: f32(f64(F + v)), with an optional handicap on v.
fn income(f: f32, v: f64, h: Option<f64>) -> f32 {
    match h {
        Some(h) => (f as f64 + v * h) as f32,
        None => (f as f64 + v) as f32,
    }
}

fn ftol_cost(x: f32) -> f32 {
    let d = x as f64;
    let t: i64 = if d.is_nan() || d.is_infinite() || d >= 9223372036854775808.0 || d <= -9223372036854775808.0 {
        0
    } else {
        d as i64
    };
    (t as i32) as f32
}

// ---- account addressing ----

// Region account address for a slot, or None when outside the range.
fn slot_addr(slot: i32) -> Option<u32> {
    if slot >= 0 && slot < unit_slots() {
        Some(region().wrapping_add((slot as u32).wrapping_mul(48)))
    } else {
        None
    }
}

fn player_econ(p: u32) -> u32 {
    let e = rd32(player_addr(p).wrapping_add(P_ECON));
    if e == 0 {
        0
    } else {
        base().wrapping_add(e)
    }
}

// ---- slots ----

#[no_mangle]
pub extern "C" fn RegionSize(map: u32) -> u32 {
    rd32(map).wrapping_mul(48)
}

#[no_mangle]
pub extern "C" fn Init(map: u32, _world: u32, _region: u32, _size: u32) {
    unsafe { write_volatile(&raw mut UNIT_SLOTS, rd32(map) as i32) }
}

#[no_mangle]
pub extern "C" fn Shutdown() -> u32 {
    let f = unsafe { read_volatile(&raw const IDENT) };
    f(region())
}

// Common refusal checks and account lookup. Returns the account address, or None for a refusal.
fn event_account(player: u32, unit: u32) -> Option<u32> {
    if player > 9 {
        return None;
    }
    if unit == 0 {
        let a = player_econ(player);
        if a == 0 {
            return None;
        }
        Some(a)
    } else {
        let slot = unsafe { w_unit_slot(unit as i32) };
        slot_addr(slot)
    }
}

#[no_mangle]
pub extern "C" fn Apply(ev: u32, size: u32) -> i32 {
    if (size as i32) < 12 {
        return 0;
    }
    let kind = rd32(ev);
    let player = rd32(ev + 4);
    let unit = rd32(ev + 8);
    match kind {
        0 => {}
        1 => {
            if (size as i32) < 60 {
                return 0;
            }
        }
        2 | 4 => {
            if (size as i32) < 24 {
                return 0;
            }
        }
        _ => return 0,
    }
    if player > 9 {
        return 0;
    }
    if kind == 1 && unit == 0 {
        return 0;
    }
    let a = match event_account(player, unit) {
        Some(a) => a,
        None => return 0,
    };
    match kind {
        0 => {
            if unit != 0 {
                for i in 0..12 {
                    wr32(a + 4 * i, 0);
                }
            } else {
                for i in 0..12 {
                    wr32(a + 4 * i, 0);
                }
                let idx = unsafe { *((player_addr(player) + P_INDEX) as *const u8) } as u32;
                let off = game().wrapping_sub(base()).wrapping_add(P_BASE).wrapping_add(P_SIZE * idx);
                wr32(a + 48, off);
            }
            1
        }
        1 => {
            for i in 0..12 {
                wr32(a + 4 * i, rd32(ev + 12 + 4 * i));
            }
            1
        }
        2 => request(a, rd32(ev + 12), rf(ev + 16), rf(ev + 20)),
        _ => {
            let res = rd32(ev + 12);
            let h = handicap(player as i32);
            if res & 1 != 0 {
                let v = income(rf(a + L_MAKE), rf(ev + 16) as f64, h);
                wf(a + L_MAKE, v);
            }
            if res & 2 != 0 {
                let v = income(rf(a + METAL + L_MAKE), rf(ev + 20) as f64, h);
                wf(a + METAL + L_MAKE, v);
            }
            1
        }
    }
}

fn request(a: u32, res: u32, energy: f32, metal: f32) -> i32 {
    match res & 3 {
        3 => {
            wf(a + L_USE, rf(a + L_USE) + energy);
            wf(a + METAL + L_USE, rf(a + METAL + L_USE) + metal);
            if allow(rf(a + L_STALL)) && allow(rf(a + METAL + L_STALL)) {
                wf(a + L_PAID, rf(a + L_PAID) + energy);
                wf(a + METAL + L_PAID, rf(a + METAL + L_PAID) + metal);
                1
            } else {
                0
            }
        }
        1 => {
            wf(a + L_USE, rf(a + L_USE) + energy);
            if allow(rf(a + L_STALL)) {
                wf(a + L_PAID, rf(a + L_PAID) + energy);
                1
            } else {
                0
            }
        }
        _ => {
            wf(a + METAL + L_USE, rf(a + METAL + L_USE) + metal);
            if allow(rf(a + METAL + L_STALL)) {
                wf(a + METAL + L_PAID, rf(a + METAL + L_PAID) + metal);
                1
            } else {
                0
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn Spend(sp: u32) -> i32 {
    let player = rd32(sp + 4);
    let unit = rd32(sp + 8);
    if player > 9 {
        return 0;
    }
    let a = match event_account(player, unit) {
        Some(a) => a,
        None => return 0,
    };
    let o = player_addr(player);
    let res = rd32(sp + 12);
    let energy = rf(sp + 16);
    let metal = rf(sp + 20);
    match res & 3 {
        3 => {
            if rf(o + P_ENERGY) >= energy && rf(o + P_METAL) >= metal {
                wf(o + P_ENERGY, rf(o + P_ENERGY) - energy);
                wf(o + P_METAL, rf(o + P_METAL) - metal);
                wf(a + L_USE, rf(a + L_USE) + energy);
                wf(a + METAL + L_USE, rf(a + METAL + L_USE) + metal);
                1
            } else {
                0
            }
        }
        1 => {
            if rf(o + P_ENERGY) >= energy {
                wf(o + P_ENERGY, rf(o + P_ENERGY) - energy);
                wf(a + L_USE, rf(a + L_USE) + energy);
                1
            } else {
                0
            }
        }
        _ => {
            if rf(o + P_METAL) >= metal {
                wf(o + P_METAL, rf(o + P_METAL) - metal);
                wf(a + METAL + L_USE, rf(a + METAL + L_USE) + metal);
                1
            } else {
                0
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn RequestGranted(unit: i32, res: u32) -> i32 {
    let slot = unsafe { w_unit_slot(unit) };
    let a = match slot_addr(slot) {
        Some(a) => a,
        None => return 0,
    };
    let e = allow(rf(a + L_STALL));
    let m = allow(rf(a + METAL + L_STALL));
    (match res & 3 {
        3 => e && m,
        1 => e,
        _ => m,
    }) as i32
}

#[no_mangle]
pub extern "C" fn UnitAccount(unit: i32, out: u32) {
    let slot = unsafe { w_unit_slot(unit) };
    if let Some(a) = slot_addr(slot) {
        for i in 0..12 {
            wr32(out + 4 * i, rd32(a + 4 * i));
        }
    }
}

#[no_mangle]
pub extern "C" fn GetEnergyUse(ty: i32) -> f64 {
    let mut t = [0i32; 11];
    unsafe { w_type_info(ty, t.as_mut_ptr()) };
    let f = |i: usize| f32::from_bits(t[i] as u32);
    let eu = f(1);
    if eu != 0.0 {
        return eu as f64;
    }
    if f(4) > 0.0 {
        let w = unsafe { w_wind() };
        return -(w as f64 * f(4) as f64);
    }
    if f(5) > 0.0 {
        let d = unsafe { w_tide() };
        return -(d as f64 * f(5) as f64);
    }
    0.0
}

#[no_mangle]
pub extern "C" fn UpdateMetalExtraction(unit: i32) {
    let mut u = [0i32; 15];
    let mut t = [0i32; 11];
    unsafe {
        w_unit_info(unit, u.as_mut_ptr());
        w_type_info(u[1], t.as_mut_ptr());
    }
    let extracts = f32::from_bits(t[3] as u32);
    if !(extracts > 0.0) {
        return;
    }
    let (cx, cy, fx, fy) = (u[10], u[11], u[12], u[13]);
    let mut w: i16 = 0;
    let mut dy = 0i32;
    while dy < fy {
        let y = cy.wrapping_add(dy);
        let mut dx = 0i32;
        while dx < fx {
            let m = unsafe { w_cell_metal(cx.wrapping_add(dx), y) };
            if m >= 0 {
                w = w.wrapping_add(m.wrapping_add(1) as i16);
            }
            dx += 1;
        }
        dy += 1;
    }
    if unit != 0 {
        wf(base().wrapping_add(unit as u32).wrapping_add(88), (extracts as f64 * w as f64) as f32);
    }
    if u[14] != 0 {
        let fx = [17, u[0], unit, w as i32];
        unsafe { w_emit(fx.as_ptr(), 16) };
    }
}

// The handicap of player q: imports PlayerInfo, and Difficulty when it is a computer player.
fn handicap(q: i32) -> Option<f64> {
    let mut pi = [0i32; 9];
    unsafe { w_player_info(q, pi.as_mut_ptr()) };
    if pi[0] != 0 && pi[1] == 2 {
        match unsafe { w_difficulty() } {
            0 => Some(0.5),
            1 => Some(0.7),
            _ => None,
        }
    } else {
        None
    }
}

#[no_mangle]
pub extern "C" fn UpdatePlayerEconomy(player: i32) {
    let pl = player as u32;
    let p = player_addr(pl);
    let mut dummy = [0u32; 12];
    let e = match player_econ(pl) {
        0 => dummy.as_mut_ptr() as u32,
        a => a,
    };
    let mut pinfo = [0i32; 9];
    unsafe { w_player_info(player, pinfo.as_mut_ptr()) };
    let last = unsafe { w_last_unit(player) } as u32;
    let mut u = unsafe { w_first_unit(player) };

    // sums: [energy, metal] x make, use, paid, stall
    let mut sum = [[0f32; 4]; 2];
    let mut ce = 0f32;
    let mut cm = 0f32;

    while (u as u32) <= last {
        let mut ui = [0i32; 15];
        let mut ti = [0i32; 11];
        if unsafe { w_unit_info(u, ui.as_mut_ptr()) } != 0 {
            unsafe { w_type_info(ui[1], ti.as_mut_ptr()) };
            let slot = unsafe { w_unit_slot(u) };
            let mut dummy2 = [0u32; 12];
            let a = match slot_addr(slot) {
                Some(a) => a,
                None => dummy2.as_mut_ptr() as u32,
            };
            let h = handicap(ui[0]);
            let tf = |i: usize| f32::from_bits(ti[i] as u32);
            let (structure, active, moving) = (ui[2] != 0, ui[3] != 0, ui[6] != 0);
            let v = tf(1);
            let charge = |a: u32| -> bool {
                if v >= 0.0 {
                    wf(a + L_USE, rf(a + L_USE) + v);
                    let funded = allow(rf(a + L_STALL));
                    if funded {
                        wf(a + L_PAID, rf(a + L_PAID) + v);
                    }
                    funded
                } else {
                    wf(a + L_MAKE, income(rf(a + L_MAKE), -v as f64, h));
                    false
                }
            };
            if structure && active {
                let funded = charge(a);
                if tf(3) > 0.0 {
                    if funded {
                        let ex = f32::from_bits(ui[9] as u32);
                        wf(a + METAL + L_MAKE, income(rf(a + METAL + L_MAKE), ex as f64, h));
                    }
                } else if ti[10] != 0 {
                    if funded {
                        wf(a + METAL + L_MAKE, income(rf(a + METAL + L_MAKE), (ti[10] as f32) as f64, h));
                    }
                } else if tf(4) > 0.0 {
                    let w = unsafe { w_wind() };
                    wf(a + L_MAKE, income(rf(a + L_MAKE), w as f64 * tf(4) as f64, h));
                } else if tf(5) > 0.0 {
                    let d = unsafe { w_tide() };
                    wf(a + L_MAKE, income(rf(a + L_MAKE), d as f64 * tf(5) as f64, h));
                }
            } else if !structure && (active || moving) {
                charge(a);
            }

            let bl = f32::from_bits(ui[8] as u32);
            if bl == 0.0 || bl.is_nan() {
                wf(a + L_MAKE, income(rf(a + L_MAKE), tf(0) as f64, h));
                wf(a + METAL + L_MAKE, income(rf(a + METAL + L_MAKE), tf(2) as f64, h));
                cm += tf(9);
                ce += tf(8);
            }

            if !(pinfo[0] != 0 && pinfo[1] == 3) {
                let mut keep = 0i32;
                if ui[4] != 0 && ui[5] == 0 {
                    let t = unsafe { w_tick() } as u32;
                    if !((ui[7] as u32) > t) {
                        let c = if moving { ftol_cost(tf(7)) } else { ftol_cost(tf(6)) };
                        let up = ui[0] as u32;
                        if up <= 9 {
                            let o = player_addr(up) + P_ENERGY;
                            let oe = rf(o);
                            if c <= oe || c.is_nan() || oe.is_nan() {
                                wf(o, oe - c);
                                wf(a + L_USE, rf(a + L_USE) + c);
                                keep = 1;
                            }
                        }
                    }
                }
                let fx = [16, ui[0], u, keep];
                unsafe { w_emit(fx.as_ptr(), 16) };
            }

            for r in 0..2 {
                let b = a + 24 * r as u32;
                for k in 0..4 {
                    sum[r][k] += rf(b + 4 * k as u32);
                }
            }
        }
        u = unsafe { w_next_unit(u) };
    }

    // The player part.
    let mut mstar = [0f64; 2];
    for r in 0..2 {
        let b = e + 24 * r as u32;
        mstar[r] = sum[r][0] as f64 + rf(b + L_MAKE) as f64;
        sum[r][0] = mstar[r] as f32;
        for k in 1..4 {
            sum[r][k] += rf(b + 4 * k as u32);
        }
    }
    if pinfo[8] != 0 {
        ce += rf(p + P_E_BONUS);
        cm += rf(p + P_M_BONUS);
    }
    let mut ratios = [[1f32; 2]; 2];
    for r in 0..2usize {
        let (cap, inc, usage, stock, tprod, tcons, wasted) = if r == 0 {
            (P_ENERGY_CAP, P_ENERGY_INCOME, P_ENERGY_USAGE, P_ENERGY, P_TOT_E_PROD, P_TOT_E_CONS, P_E_WASTED)
        } else {
            (P_METAL_CAP, P_METAL_INCOME, P_METAL_USAGE, P_METAL, P_TOT_M_PROD, P_TOT_M_CONS, P_M_WASTED)
        };
        let c = if r == 0 { ce } else { cm };
        let [m, us, pd, s] = sum[r];
        wf(p + cap, c);
        wf(p + inc, m);
        wf(p + usage, us);
        let produced = if r == 0 { mstar[r] } else { m as f64 };
        wd(p + tprod, rd(p + tprod) + produced);
        wd(p + tcons, rd(p + tcons) + us as f64);
        let amt = (mstar[r] + rf(p + stock) as f64) as f32;
        let (rs, l): (f32, f64) = if s <= amt || s.is_nan() || amt.is_nan() {
            (1.0, amt as f64 - s as f64)
        } else {
            (amt / s, 0.0)
        };
        let (rp, bb): (f32, f32) = if (pd as f64) <= l || pd.is_nan() || l.is_nan() {
            (1.0, (l - pd as f64) as f32)
        } else {
            ((l / pd as f64) as f32, 0.0)
        };
        if bb > c {
            wf(p + stock, c);
            wd(p + wasted, rd(p + wasted) + (bb as f64 - c as f64));
        } else {
            wf(p + stock, bb);
        }
        ratios[r] = [rs, rp];
    }

    // The second walk and the settlement.
    let settle = |a: u32| {
        for r in 0..2u32 {
            let b = a + 24 * r;
            let [rs, rp] = ratios[r as usize];
            wf(b + L_MAKEPREV, rf(b + L_MAKE));
            wf(b + L_USEPREV, rf(b + L_USE));
            let s = rf(b + L_STALL) as f64;
            let q = rf(b + L_PAID) as f64;
            let x = s - rs as f64 * s;
            let y = q - rp as f64 * q;
            wf(b + L_STALL, (x + y) as f32);
            wf(b + L_MAKE, 0.0);
            wf(b + L_USE, 0.0);
            wf(b + L_PAID, 0.0);
        }
    };
    let mut u = unsafe { w_first_unit(player) };
    while (u as u32) <= last {
        let mut ui = [0i32; 15];
        if unsafe { w_unit_info(u, ui.as_mut_ptr()) } != 0 {
            let slot = unsafe { w_unit_slot(u) };
            if let Some(a) = slot_addr(slot) {
                settle(a);
            }
        }
        u = unsafe { w_next_unit(u) };
    }
    settle(e);
}

// The partner test of i: PlayerInfo, and Allied when it is a partner. Returns (passes, info).
fn partner(player: i32, i: i32) -> (bool, [i32; 9]) {
    let mut pi = [0i32; 9];
    unsafe { w_player_info(i, pi.as_mut_ptr()) };
    let is = pi[0] != 0 && pi[1] == 3 && pi[2] != 10 && pi[7] != 0 && pi[3] != 0;
    if is && unsafe { w_allied(player, i) } != 0 {
        (true, pi)
    } else {
        (false, pi)
    }
}

fn share_amount(x: f32, keep: f32, k: f64, cap: f32, y: f32) -> f32 {
    let d = (x as f64 - keep as f64) * k;
    let r = cap as f64 - y as f64;
    if d < r || d.is_nan() || r.is_nan() {
        d as f32
    } else {
        r as f32
    }
}

fn share(player: i32, me: &[i32; 9], metal: bool) {
    let s = player_addr(player as u32);
    let (flag, stock, keep, cap, k, res) = if metal {
        (me[4], P_METAL, P_SHARE_METAL, P_METAL_CAP, f32::from_bits(0x3eaaaaab) as f64, 2)
    } else {
        (me[5], P_ENERGY, P_SHARE_ENERGY, P_ENERGY_CAP, 0.5f64, 1)
    };
    if !(flag != 0 && rf(s + stock) > rf(s + keep)) {
        return;
    }
    let mut rec: Option<(i32, i32)> = None;
    for i in 0..10 {
        let (ok, pi) = partner(player, i);
        let y = rf(player_addr(i as u32) + stock);
        let x = rf(s + stock);
        if ok && (y < x || y.is_nan() || x.is_nan()) {
            rec = Some((i, pi[2]));
        }
    }
    if let Some((r, idx)) = rec {
        if r != player {
            let ra = player_addr(r as u32);
            let amt = share_amount(rf(s + stock), rf(s + keep), k, rf(ra + cap), rf(ra + stock));
            let (en, mt) = if metal { (0f32, amt) } else { (amt, 0f32) };
            let fx = [18, me[2], 0, res, en.to_bits() as i32, mt.to_bits() as i32, idx];
            unsafe { w_emit(fx.as_ptr(), 28) };
        }
    }
}

#[no_mangle]
pub extern "C" fn UpdateResourceSharing(player: i32) {
    if unsafe { w_sharing_enabled() } == 0 {
        return;
    }
    let mut me = [0i32; 9];
    unsafe { w_player_info(player, me.as_mut_ptr()) };
    let t = unsafe { w_tick() } as u32;
    if t % 60 == 0 {
        share(player, &me, true);
        share(player, &me, false);
    }
    let t2 = unsafe { w_tick() } as u32;
    if t2 % 450 == 0 && me[6] != 0 {
        for i in 0..10 {
            let (ok, pi) = partner(player, i);
            if ok {
                let a = me[2] & 0xff;
                let b = pi[2] & 0xff;
                if a != 10 && b != 10 {
                    unsafe { n_send_share_map_info(a, b) };
                }
            }
        }
    }
}
