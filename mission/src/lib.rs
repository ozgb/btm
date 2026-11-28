#![no_std]

use core::ptr::{read_unaligned, read_volatile, write_unaligned};

#[no_mangle]
pub static mut g_game: u32 = 0;
#[no_mangle]
pub static mut g_simGlobals: u32 = 0;
#[no_mangle]
pub static mut g_extTdf: u32 = 0;
#[no_mangle]
pub static mut g_blockBase: u32 = 0;

const G_NET: u32 = 233961;

const M_NAMES: u32 = 260;
const M_MISSION_NAME: u32 = 2580;
const M_INDEX: u32 = 3096;
const M_FIGURES: u32 = 3376; // surfaceMetal .. startEnergy: 31 words
const M_UNITS: u32 = 3500;
const M_UNIT_COUNT: u32 = 3504;
const M_RULES: u32 = 3508;
const M_RULE_COUNT: u32 = 3512;
const M_FEATURES: u32 = 3516;
const M_FEATURE_COUNT: u32 = 3520;

fn rd<T: Copy>(a: u32) -> T {
    unsafe { read_unaligned(a as *const T) }
}
fn wr<T>(a: u32, v: T) {
    unsafe { write_unaligned(a as *mut T, v) }
}
fn base() -> u32 {
    unsafe { read_volatile(&raw const g_blockBase) }
}
fn game() -> u32 {
    unsafe { read_volatile(&raw const g_game) }
}
fn rec() -> u32 {
    base().wrapping_add(rd::<u32>(game().wrapping_add(G_NET)))
}
fn addr(r: u32) -> u32 {
    if r == 0 { 0 } else { base().wrapping_add(r) }
}

#[no_mangle]
pub extern "C" fn GetGameType() -> i32 {
    rd(rec())
}

#[no_mangle]
pub extern "C" fn GetNameSlot(index: i32) -> u32 {
    if (index as u32) > 8 {
        return 0;
    }
    let a = rec() + M_NAMES + index as u32 * 256;
    if rd::<u8>(a) != 0 { a } else { 0 }
}

#[no_mangle]
pub extern "C" fn GetMissionIndex() -> i32 {
    rd(rec() + M_INDEX)
}

#[no_mangle]
pub extern "C" fn GetMissionName() -> u32 {
    rec() + M_MISSION_NAME
}

#[no_mangle]
pub extern "C" fn HasMissionName() -> i32 {
    (rd::<u8>(rec() + M_MISSION_NAME) != 0) as i32
}

#[no_mangle]
pub extern "C" fn CountStartPositions() -> i32 {
    let r = rec();
    let n: i32 = rd(r + M_RULE_COUNT);
    let t = addr(rd(r + M_RULES));
    let mut c = 0;
    let mut i = 0;
    while i < n {
        if rd::<i32>(t + i as u32 * 12) == 1 {
            c += 1;
        }
        i += 1;
    }
    c
}

#[no_mangle]
pub extern "C" fn GetStartPosition(out: u32, id: i32) -> i32 {
    let r = rec();
    let n: i32 = rd(r + M_RULE_COUNT);
    let t = addr(rd(r + M_RULES));
    let mut i = 0;
    while i < n {
        let p = t + i as u32 * 12;
        if rd::<i32>(p) == 1 && rd::<i32>(p + 4) == id {
            let x = rd::<i16>(p + 8) as i32;
            let z = rd::<i16>(p + 10) as i32;
            wr(out, x.wrapping_mul(65536));
            wr(out + 4, 0i32);
            wr(out + 8, z.wrapping_mul(65536));
            return 1;
        }
        i += 1;
    }
    0
}

#[no_mangle]
pub extern "C" fn GetMissionParams(out: u32) {
    let r = rec() + M_FIGURES;
    let mut i = 0;
    while i < 31 {
        wr(out + i * 4, rd::<u32>(r + i * 4));
        i += 1;
    }
}

#[no_mangle]
pub extern "C" fn CountMissionUnits() -> i32 {
    rd(rec() + M_UNIT_COUNT)
}

#[no_mangle]
pub extern "C" fn GetMissionUnit(index: i32, out: u32) -> i32 {
    let r = rec();
    if index < 0 || index >= rd::<i32>(r + M_UNIT_COUNT) {
        return 0;
    }
    let u = addr(rd(r + M_UNITS)) + index as u32 * 36;
    wr(out, addr(rd(u)));
    wr(out + 4, addr(rd(u + 4)));
    wr(out + 8, addr(rd(u + 8)));
    let mut o = 12;
    while o < 36 {
        wr(out + o, rd::<u32>(u + o));
        o += 4;
    }
    1
}

#[no_mangle]
pub extern "C" fn CountMissionFeatures() -> i32 {
    rd(rec() + M_FEATURE_COUNT)
}

#[no_mangle]
pub extern "C" fn GetMissionFeature(index: i32, out: u32) -> i32 {
    let r = rec();
    if index < 0 || index >= rd::<i32>(r + M_FEATURE_COUNT) {
        return 0;
    }
    let f = addr(rd(r + M_FEATURES)) + index as u32 * 136;
    wr(out, f);
    wr(out + 4, rd::<i32>(f + 128));
    wr(out + 8, rd::<i32>(f + 132));
    1
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// A function pointer puts the module's first slot in its table range (496), so that the module imports the table
// and has a data segment, as the loader's checks expect.
#[used]
static TABLE_ANCHOR: [extern "C" fn() -> i32; 1] = [GetGameType];
