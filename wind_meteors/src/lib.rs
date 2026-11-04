#![no_std]

use core::ptr::{read_unaligned, read_volatile, write_unaligned};

#[link(wasm_import_module = "weapons")]
extern "C" {
    #[link_name = "SpawnProjectile"]
    fn spawn_projectile(weapon: u32, pos: *const i32, vel: *const i32, flag: i32) -> i32;
    #[link_name = "FindWeaponByName"]
    fn find_weapon_by_name(name: u32) -> u32;
}

#[link(wasm_import_module = "services")]
extern "C" {
    #[link_name = "GameRand"]
    fn game_rand(stream: i32) -> i32;
    #[link_name = "RandomInt"]
    fn random_int(range: i32) -> i32;
    #[link_name = "FatalError"]
    fn fatal_error(msg: *const u8);
    #[link_name = "BuildDataPath"]
    fn build_data_path(buf: *mut u8, dir: *const u8, name: *const u8, ext: *const u8) -> u32;
    #[link_name = "TdfFile_Construct"]
    fn tdf_construct(file: *mut u32);
    #[link_name = "TdfFile_Destruct"]
    fn tdf_destruct(file: *mut u32);
    #[link_name = "TdfFile_LoadFile"]
    fn tdf_load(file: *mut u32, path: *const u8) -> i32;
    #[link_name = "TdfFile_SelectRecord"]
    fn tdf_select(file: *mut u32, name: *const u8) -> i32;
    #[link_name = "TdfRecord_GetFieldInt"]
    fn get_int(rec: u32, key: *const u8, def: i32) -> i32;
    #[link_name = "TdfRecord_GetFieldDouble"]
    fn get_double(rec: u32, key: *const u8, def: f64) -> f64;
    #[link_name = "TdfRecord_GetFieldString"]
    fn get_string(rec: u32, dst: *mut u8, key: *const u8, size: i32, def: *const u8) -> i32;
    #[link_name = "CommandArgs_GetIntArg"]
    fn get_int_arg(args: u32, index: i32, def: i32) -> i32;
    #[link_name = "HapiBank_OpenAccount"]
    fn bank_open(bank: u32, name: *const u8) -> i32;
    #[link_name = "HapiBank_SetIntegerItem"]
    fn bank_set(bank: u32, name: *const u8, v: i32) -> i32;
    #[link_name = "HapiBank_GetIntegerItem"]
    fn bank_get(bank: u32, name: *const u8, def: i32) -> i32;
}

#[no_mangle]
pub static mut g_game: u32 = 0;
#[no_mangle]
pub static mut g_simGlobals: u32 = 0;
#[no_mangle]
pub static mut g_extTdf: u32 = 0;
#[no_mangle]
pub static mut g_blockBase: u32 = 0;

// Game offsets
const G_WEAPONS: u32 = 11507;
const G_WIDTH: u32 = 82483;
const G_HEIGHT: u32 = 82487;
const G_WMIN: u32 = 82523;
const G_WMAX: u32 = 82527;
const G_WCOUNTER: u32 = 229060;
const G_WDEN: u32 = 229064;
const G_WX: u32 = 229068;
const G_WZ: u32 = 229076;
const G_WDIR: u32 = 229080;
const G_WSPEED: u32 = 229082;
const G_WSTRENGTH: u32 = 229086;
const G_WENABLED: u32 = 229090;
const G_TICKS: u32 = 232007;

// SimGlobals offsets
const S_NEXT_STRIKE: u32 = 568;
const S_NAME: u32 = 576;
const S_MAGNITUDE: u32 = 608;
const S_INTERVAL: u32 = 612;
const S_ACTIVE: u32 = 616;
const S_STRIKE_END: u32 = 620;
const S_ORIGIN: u32 = 624;
const S_DURATION: u32 = 628;
const S_WEAPON: u32 = 632;
const S_ENABLED: u32 = 636;
const S_NEXT_HIT: u32 = 640;
const S_TARGET: u32 = 644;
const S_GAP: u32 = 648;

#[inline(always)]
fn gp(off: u32) -> u32 {
    unsafe { read_volatile(&raw const g_game) + off }
}
#[inline(always)]
fn sp(off: u32) -> u32 {
    unsafe { read_volatile(&raw const g_simGlobals) + off }
}
#[inline(always)]
fn r32(a: u32) -> i32 {
    unsafe { read_unaligned(a as *const i32) }
}
#[inline(always)]
fn w32(a: u32, v: i32) {
    unsafe { write_unaligned(a as *mut i32, v) }
}
#[inline(always)]
fn r16(a: u32) -> i16 {
    unsafe { read_unaligned(a as *const i16) }
}
#[inline(always)]
fn w16(a: u32, v: i16) {
    unsafe { write_unaligned(a as *mut i16, v) }
}

#[inline(always)]
fn scale(d: i32, k: i32) -> i32 {
    ((d as i64 * k as i64) / 32768) as i32
}

#[inline(always)]
fn sine(a: i32, s: i32) -> i32 {
    let a = (a as u32 & 0xffff) as usize;
    let e = unsafe { *T.get_unchecked(((a + 32) / 128) & 511) } as i64;
    ((e * s as i64 + 4096) >> 13) as i32
}
#[inline(always)]
fn cosine(a: i32, s: i32) -> i32 {
    sine(a.wrapping_add(0x4000), s)
}

#[inline(always)]
fn d2i(x: f64) -> i32 {
    if x != x || x >= 2147483648.0 || x <= -2147483649.0 {
        i32::MIN
    } else {
        x as i32
    }
}

#[inline(always)]
fn wind(stream: i32) {
    let t = r32(gp(G_TICKS)) as u32;
    if (r32(gp(G_WCOUNTER)) as u32) < t {
        let g = unsafe { game_rand(stream) };
        w32(gp(G_WCOUNTER), r32(gp(G_WCOUNTER)).wrapping_add((scale(g, 10) + 5).wrapping_mul(30)));
        let min = r32(gp(G_WMIN));
        let n = unsafe { random_int(r32(gp(G_WMAX)).wrapping_sub(min)) };
        let speed = min.wrapping_add(n);
        w32(gp(G_WSPEED), speed);
        if speed != 0 {
            let d = unsafe { random_int(65536) };
            w16(gp(G_WDIR), d as i16);
        }
        let dir = r16(gp(G_WDIR)) as i32;
        w32(gp(G_WX), sine(dir, speed).wrapping_mul(2).wrapping_neg());
        w32(gp(G_WZ), cosine(dir, speed).wrapping_mul(2).wrapping_neg());
        let mut q = speed as f32 / r32(gp(G_WDEN)) as f32;
        if q > 1.0 {
            q = 1.0;
        }
        w32(gp(G_WSTRENGTH), q.to_bits() as i32);
        w32(gp(G_WENABLED), 1);
    } else {
        w32(gp(G_WENABLED), 0);
    }
}

#[no_mangle]
pub extern "C" fn UpdateWind() {
    wind(0);
}

#[no_mangle]
pub extern "C" fn UpdateWindOnStream(stream: i32) {
    wind(stream);
}

#[inline(always)]
fn start_shower(t: i32) {
    w32(sp(S_ACTIVE), 1);
    let end = r32(sp(S_DURATION)).wrapping_add(t);
    w32(sp(S_STRIKE_END), end);
    w32(sp(S_NEXT_STRIKE), r32(sp(S_GAP)).wrapping_add(end));
    w32(sp(S_NEXT_HIT), t);
    let d1 = unsafe { game_rand(0) };
    let d2 = unsafe { game_rand(0) };
    let d3 = unsafe { game_rand(0) };
    let d4 = unsafe { game_rand(0) };
    let tz = scale(d1, r32(gp(G_HEIGHT)));
    let tx = scale(d2, r32(gp(G_WIDTH)));
    let oz = scale(d3, 10) - 15;
    let ox = scale(d4, 30) - 15;
    w16(sp(S_TARGET), tx as i16);
    w16(sp(S_TARGET) + 2, tz as i16);
    w16(sp(S_ORIGIN), tx.wrapping_add(ox) as i16);
    w16(sp(S_ORIGIN) + 2, tz.wrapping_add(oz) as i16);
}

#[inline(always)]
fn meteor_falls(t: i32) {
    let tx = r16(sp(S_TARGET)) as i32;
    let tz = r16(sp(S_TARGET) + 2) as i32;
    let ox = r16(sp(S_ORIGIN)) as i32;
    let oz = r16(sp(S_ORIGIN) + 2) as i32;
    let vel = [
        tx.wrapping_sub(ox).wrapping_shl(20) / 90,
        -983040,
        tz.wrapping_sub(oz).wrapping_shl(20) / 90,
    ];
    let d5 = unsafe { game_rand(0) };
    let d6 = unsafe { game_rand(0) };
    let big_r = scale(d5, r32(sp(S_MAGNITUDE))).wrapping_shl(16);
    let a = d6.wrapping_mul(2);
    let pos = [
        ox.wrapping_shl(20).wrapping_sub(sine(a, big_r)),
        88473600,
        oz.wrapping_shl(20).wrapping_sub(cosine(a, big_r)),
    ];
    let off = r32(sp(S_WEAPON)) as u32;
    let wp = if off == 0 { 0 } else { unsafe { read_volatile(&raw const g_blockBase) }.wrapping_add(off) };
    let _ = t;
    unsafe { spawn_projectile(wp, pos.as_ptr(), vel.as_ptr(), 1) };
}

#[no_mangle]
pub extern "C" fn UpdateMeteors() {
    let t = r32(gp(G_TICKS));
    if (r32(sp(S_NEXT_STRIKE)) as u32) <= t as u32 {
        start_shower(t);
        if r32(sp(S_ENABLED)) == 0 {
            w32(sp(S_ACTIVE), 0);
        }
    }
    if r32(sp(S_ACTIVE)) != 0 {
        if (r32(sp(S_NEXT_HIT)) as u32) <= t as u32 {
            w32(sp(S_NEXT_HIT), r32(sp(S_INTERVAL)).wrapping_add(t));
            meteor_falls(t);
        }
        if (r32(sp(S_STRIKE_END)) as u32) <= t as u32 {
            w32(sp(S_ACTIVE), 0);
        }
    }
}

static KEEP: [extern "C" fn(); 1] = [EmptyShutdownPreCleanup];

#[no_mangle]
pub extern "C" fn EmptyShutdownPreCleanup() {
    unsafe { read_volatile(&raw const KEEP) };
}

#[no_mangle]
pub extern "C" fn InitMeteors() {
    w32(sp(S_ACTIVE), 0);
    w32(sp(S_NEXT_STRIKE), r32(sp(S_INTERVAL)));
    let w = unsafe { find_weapon_by_name(sp(S_NAME)) };
    let first = gp(G_WEAPONS);
    let chosen = if w == 0 || (unsafe { *((w + 273) as *const u8) } & 0x20) == 0 { first } else { w };
    w32(sp(S_WEAPON), chosen.wrapping_sub(unsafe { read_volatile(&raw const g_blockBase) }) as i32);
}

#[no_mangle]
pub extern "C" fn EnableMeteors() {
    w32(sp(S_ENABLED), 1);
}

#[no_mangle]
pub extern "C" fn DisableMeteors() {
    w32(sp(S_ENABLED), 0);
}

#[no_mangle]
pub extern "C" fn SetMeteorParams(params: u32) {
    let mut i = 0;
    while i < 32 {
        let c = unsafe { *((params + i) as *const u8) };
        unsafe { *((sp(S_NAME) + i) as *mut u8) = c };
        if c == 0 {
            break;
        }
        i += 1;
    }
    w32(sp(S_MAGNITUDE), r32(params + 32));
    let f = |o: u32| f32::from_bits(r32(params + o) as u32) as f64;
    w32(sp(S_INTERVAL), d2i(30.0 / f(36)));
    w32(sp(S_DURATION), d2i(f(40) * 30.0));
    w32(sp(S_GAP), d2i(f(44) * 30.0));
}

#[no_mangle]
pub extern "C" fn LoadMeteorDefaults(params: u32) {
    let mut file = [0u32; 3];
    let f = file.as_mut_ptr();
    let mut buf = [0u8; 256];
    unsafe {
        tdf_construct(f);
        let ext = &raw const g_extTdf as *const u8;
        build_data_path(buf.as_mut_ptr(), b"gamedata\0".as_ptr(), b"meteor\0".as_ptr(), ext);
        if tdf_load(f, buf.as_ptr()) == 0 {
            tdf_destruct(f);
            return;
        }
        if tdf_select(f, b"Default\0".as_ptr()) == 0 {
            tdf_destruct(f);
            return;
        }
        let rec = read_volatile(f.add(1));
        if get_string(rec, params as *mut u8, b"MeteorWeapon\0".as_ptr(), 32, b"\0".as_ptr()) == 0 {
            fatal_error(BAD.as_ptr());
            return;
        }
        let radius = get_int(rec, b"MeteorRadius\0".as_ptr(), 0);
        w32(params + 32, radius);
        let dens = get_double(rec, b"MeteorDensity\0".as_ptr(), 0.0) as f32;
        let dur = get_double(rec, b"MeteorDuration\0".as_ptr(), 0.0) as f32;
        let intv = get_double(rec, b"MeteorInterval\0".as_ptr(), 0.0) as f32;
        w32(params + 36, dens.to_bits() as i32);
        w32(params + 40, dur.to_bits() as i32);
        w32(params + 44, intv.to_bits() as i32);
        if radius == 0 || dens == 0.0 || dur == 0.0 || intv == 0.0 {
            fatal_error(BAD.as_ptr());
            return;
        }
        tdf_destruct(f);
    }
}

static BAD: [u8; 56] = *b"Hey, hoser!  The default meteor shower data was bogus!\0\0";

// (item name, SimGlobals offset, is point16)
static ITEMS: [(&[u8; 17], u8, u8); 9] = [
    (b"Enabled\0\0\0\0\0\0\0\0\0\0", 0, 0),
    (b"Active\0\0\0\0\0\0\0\0\0\0\0", 1, 0),
    (b"Next Strike Time\0", 2, 0),
    (b"Time Strike Ends\0", 3, 0),
    (b"Next Hit Time\0\0\0\0", 4, 0),
    (b"Origin X\0\0\0\0\0\0\0\0\0", 5, 1),
    (b"Origin Z\0\0\0\0\0\0\0\0\0", 6, 1),
    (b"Target X\0\0\0\0\0\0\0\0\0", 7, 1),
    (b"Target Z\0\0\0\0\0\0\0\0\0", 8, 1),
];

#[inline(always)]
fn item_addr(k: u8) -> u32 {
    match k {
        0 => sp(S_ENABLED),
        1 => sp(S_ACTIVE),
        2 => sp(S_NEXT_STRIKE),
        3 => sp(S_STRIKE_END),
        4 => sp(S_NEXT_HIT),
        5 => sp(S_ORIGIN),
        6 => sp(S_ORIGIN) + 2,
        7 => sp(S_TARGET),
        _ => sp(S_TARGET) + 2,
    }
}

#[no_mangle]
pub extern "C" fn SaveMeteors(bank: u32) {
    unsafe { bank_open(bank, b"Meteor\0".as_ptr()) };
    let mut i = 0;
    while i < 9 {
        let it = &ITEMS[i];
        let a = item_addr(it.1);
        let v = if it.2 != 0 { r16(a) as i32 } else { r32(a) };
        unsafe { bank_set(bank, it.0.as_ptr(), v) };
        i += 1;
    }
}

#[no_mangle]
pub extern "C" fn LoadMeteors(bank: u32) {
    unsafe { bank_open(bank, b"Meteor\0".as_ptr()) };
    let mut i = 0;
    while i < 9 {
        let it = &ITEMS[i];
        let a = item_addr(it.1);
        let v = unsafe { bank_get(bank, it.0.as_ptr(), 0) };
        if it.2 != 0 {
            w16(a, v as i16);
        } else {
            w32(a, v);
        }
        i += 1;
    }
}

#[no_mangle]
pub extern "C" fn CmdMeteor(args: u32) {
    if r32(args + 0xd0) > 1 {
        let v = unsafe { get_int_arg(args, 1, 0) };
        w32(sp(S_ENABLED), (v != 0) as i32);
    } else {
        start_shower(r32(gp(G_TICKS)));
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

static T: [i16; 512] = [
    0, 101, 201, 302, 402, 502, 603, 703, 803, 903, 1003, 1102, 1202, 1301, 1401, 1499,
    1598, 1697, 1795, 1893, 1990, 2088, 2185, 2282, 2378, 2474, 2570, 2665, 2760, 2854, 2948, 3042,
    3135, 3228, 3320, 3411, 3503, 3593, 3683, 3773, 3862, 3950, 4038, 4125, 4212, 4297, 4383, 4467,
    4551, 4634, 4717, 4799, 4880, 4960, 5040, 5119, 5197, 5274, 5351, 5427, 5501, 5575, 5649, 5721,
    5793, 5863, 5933, 6002, 6070, 6137, 6203, 6268, 6333, 6396, 6458, 6519, 6580, 6639, 6698, 6755,
    6811, 6867, 6921, 6974, 7027, 7078, 7128, 7177, 7225, 7272, 7317, 7362, 7405, 7448, 7489, 7529,
    7568, 7606, 7643, 7679, 7713, 7746, 7779, 7809, 7839, 7868, 7895, 7921, 7946, 7970, 7993, 8014,
    8035, 8054, 8071, 8088, 8103, 8117, 8130, 8142, 8153, 8162, 8170, 8177, 8182, 8186, 8190, 8191,
    8192, 8191, 8190, 8186, 8182, 8177, 8170, 8162, 8153, 8142, 8130, 8117, 8103, 8088, 8071, 8054,
    8035, 8014, 7993, 7970, 7946, 7921, 7895, 7868, 7839, 7809, 7779, 7746, 7713, 7679, 7643, 7606,
    7568, 7529, 7489, 7448, 7405, 7362, 7317, 7272, 7225, 7177, 7128, 7078, 7027, 6974, 6921, 6867,
    6811, 6755, 6698, 6639, 6580, 6519, 6458, 6396, 6333, 6268, 6203, 6137, 6070, 6002, 5933, 5863,
    5793, 5721, 5649, 5575, 5501, 5427, 5351, 5274, 5197, 5119, 5040, 4960, 4880, 4799, 4717, 4634,
    4551, 4467, 4383, 4297, 4212, 4125, 4038, 3950, 3862, 3773, 3683, 3593, 3503, 3411, 3320, 3228,
    3135, 3042, 2948, 2854, 2760, 2665, 2570, 2474, 2378, 2282, 2185, 2088, 1990, 1893, 1795, 1697,
    1598, 1499, 1401, 1301, 1202, 1102, 1003, 903, 803, 703, 603, 502, 402, 302, 201, 101,
    0, -101, -201, -302, -402, -502, -603, -703, -803, -903, -1003, -1102, -1202, -1301, -1401, -1499,
    -1598, -1697, -1795, -1893, -1990, -2088, -2185, -2282, -2378, -2474, -2570, -2665, -2760, -2854, -2948, -3042,
    -3135, -3228, -3320, -3411, -3503, -3593, -3683, -3773, -3862, -3950, -4038, -4125, -4212, -4297, -4383, -4467,
    -4551, -4634, -4717, -4799, -4880, -4960, -5040, -5119, -5197, -5274, -5351, -5427, -5501, -5575, -5649, -5721,
    -5793, -5863, -5933, -6002, -6070, -6137, -6203, -6268, -6333, -6396, -6458, -6519, -6580, -6639, -6698, -6755,
    -6811, -6867, -6921, -6974, -7027, -7078, -7128, -7177, -7225, -7272, -7317, -7362, -7405, -7448, -7489, -7529,
    -7568, -7606, -7643, -7679, -7713, -7746, -7779, -7809, -7839, -7868, -7895, -7921, -7946, -7970, -7993, -8014,
    -8035, -8054, -8071, -8088, -8103, -8117, -8130, -8142, -8153, -8162, -8170, -8177, -8182, -8186, -8190, -8191,
    -8192, -8191, -8190, -8186, -8182, -8177, -8170, -8162, -8153, -8142, -8130, -8117, -8103, -8088, -8071, -8054,
    -8035, -8014, -7993, -7970, -7946, -7921, -7895, -7868, -7839, -7809, -7779, -7746, -7713, -7679, -7643, -7606,
    -7568, -7529, -7489, -7448, -7405, -7362, -7317, -7272, -7225, -7177, -7128, -7078, -7027, -6974, -6921, -6867,
    -6811, -6755, -6698, -6639, -6580, -6519, -6458, -6396, -6333, -6268, -6203, -6137, -6070, -6002, -5933, -5863,
    -5793, -5721, -5649, -5575, -5501, -5427, -5351, -5274, -5197, -5119, -5040, -4960, -4880, -4799, -4717, -4634,
    -4551, -4467, -4383, -4297, -4212, -4125, -4038, -3950, -3862, -3773, -3683, -3593, -3503, -3411, -3320, -3228,
    -3135, -3042, -2948, -2854, -2760, -2665, -2570, -2474, -2378, -2282, -2185, -2088, -1990, -1893, -1795, -1697,
    -1598, -1499, -1401, -1301, -1202, -1102, -1003, -903, -803, -703, -603, -502, -402, -302, -201, -101,
];
