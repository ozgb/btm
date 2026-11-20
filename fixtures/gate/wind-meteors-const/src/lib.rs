#![no_std]

#[link(wasm_import_module = "services")]
extern "C" {
    #[link_name = "BuildDataPath"]
    fn imp_BuildDataPath(a: i32, b: i32, c: i32, d: i32) -> i32;
    #[link_name = "CommandArgs_GetIntArg"]
    fn imp_CommandArgs_GetIntArg(a: i32, b: i32, c: i32) -> i32;
    #[link_name = "FatalError"]
    fn imp_FatalError(a: i32);
    #[link_name = "GameRand"]
    fn imp_GameRand(a: i32) -> i32;
    #[link_name = "HapiBank_GetIntegerItem"]
    fn imp_HapiBank_GetIntegerItem(a: i32, b: i32, c: i32) -> i32;
    #[link_name = "HapiBank_OpenAccount"]
    fn imp_HapiBank_OpenAccount(a: i32, b: i32) -> i32;
    #[link_name = "HapiBank_SetIntegerItem"]
    fn imp_HapiBank_SetIntegerItem(a: i32, b: i32, c: i32) -> i32;
    #[link_name = "RandomInt"]
    fn imp_RandomInt(a: i32) -> i32;
    #[link_name = "TdfFile_Construct"]
    fn imp_TdfFile_Construct(a: i32);
    #[link_name = "TdfFile_Destruct"]
    fn imp_TdfFile_Destruct(a: i32);
    #[link_name = "TdfFile_LoadFile"]
    fn imp_TdfFile_LoadFile(a: i32, b: i32) -> i32;
    #[link_name = "TdfFile_SelectRecord"]
    fn imp_TdfFile_SelectRecord(a: i32, b: i32) -> i32;
    #[link_name = "TdfRecord_GetFieldDouble"]
    fn imp_TdfRecord_GetFieldDouble(a: i32, b: i32, c: f64) -> f64;
    #[link_name = "TdfRecord_GetFieldInt"]
    fn imp_TdfRecord_GetFieldInt(a: i32, b: i32, c: i32) -> i32;
    #[link_name = "TdfRecord_GetFieldString"]
    fn imp_TdfRecord_GetFieldString(a: i32, b: i32, c: i32, d: i32, e: i32) -> i32;
}

#[link(wasm_import_module = "weapons")]
extern "C" {
    #[link_name = "FindWeaponByName"]
    fn imp_FindWeaponByName(a: i32) -> i32;
    #[link_name = "SpawnProjectile"]
    fn imp_SpawnProjectile(a: i32, b: i32, c: i32, d: i32) -> i32;
}

#[no_mangle]
pub static mut g_game: u32 = 0;
#[no_mangle]
pub static mut g_simGlobals: u32 = 0;
#[no_mangle]
pub static mut g_extTdf: [u8; 4] = [0; 4];
#[no_mangle]
pub static mut g_blockBase: u32 = 0;

#[used]
static CONSTANTS: [u32; 2] = [1, 2];

/// Never true when the engine calls a slot; keeps every import in the module.
fn keep_imports(stream: i32) {
    if stream != i32::MIN {
        return;
    }
    unsafe {
        imp_BuildDataPath(0, 0, 0, 0);
        imp_CommandArgs_GetIntArg(0, 0, 0);
        imp_FatalError(0);
        imp_GameRand(0);
        imp_HapiBank_GetIntegerItem(0, 0, 0);
        imp_HapiBank_OpenAccount(0, 0);
        imp_HapiBank_SetIntegerItem(0, 0, 0);
        imp_RandomInt(0);
        imp_TdfFile_Construct(0);
        imp_TdfFile_Destruct(0);
        imp_TdfFile_LoadFile(0, 0);
        imp_TdfFile_SelectRecord(0, 0);
        imp_TdfRecord_GetFieldDouble(0, 0, 0.0);
        imp_TdfRecord_GetFieldInt(0, 0, 0);
        imp_TdfRecord_GetFieldString(0, 0, 0, 0, 0);
        imp_FindWeaponByName(0);
        imp_SpawnProjectile(0, 0, 0, 0);
    }
}

#[no_mangle]
pub extern "C" fn UpdateWind() {}

#[no_mangle]
pub extern "C" fn UpdateMeteors() {}

#[no_mangle]
pub extern "C" fn InitMeteors() {}

#[no_mangle]
pub extern "C" fn EmptyShutdownPreCleanup() {}

#[no_mangle]
pub extern "C" fn EnableMeteors() {}

#[no_mangle]
pub extern "C" fn DisableMeteors() {}

#[no_mangle]
pub extern "C" fn SetMeteorParams(_a: i32) {}

#[no_mangle]
pub extern "C" fn LoadMeteorDefaults(_a: i32) {}

#[no_mangle]
pub extern "C" fn SaveMeteors(_a: i32) {}

#[no_mangle]
pub extern "C" fn LoadMeteors(_a: i32) {}

#[no_mangle]
pub extern "C" fn CmdMeteor(_a: i32) {}

#[no_mangle]
pub extern "C" fn UpdateWindOnStream(stream: i32) { keep_imports(stream) }

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
