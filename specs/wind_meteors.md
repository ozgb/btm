# wind_meteors

## 1. Role

Wind and meteors are the two pieces of weather in a match. Every few seconds the wind picks a new
speed and direction within the map's limits; wind generators and anything else that reads the wind
use the result. Meteors are showers of a map-chosen weapon falling on a random spot of the map: a
schedule starts a shower, a shower drops one projectile at a fixed interval for a fixed duration,
and the next shower comes a fixed gap after the last one ends. Showers can be switched on and off by
the map and by the `meteor` console command, and their state is kept in saved games.

## 2. Interface

### Slots (`interfaces/wind_meteors.h`, version 1.2)

| Slot | When the engine calls it |
| --- | --- |
| `UpdateWind` | every game tick, in the wind phase |
| `UpdateMeteors` | every game tick, right after `UpdateWind` |
| `UpdateWindOnStream` | once at match load, with the load stream; `UpdateWind` behaves as this slot with the main stream |
| `InitMeteors` | at match load, after the map's meteor parameters are set |
| `EmptyShutdownPreCleanup` | when a match shuts down |
| `EnableMeteors`, `DisableMeteors` | when a map is loaded, and from the console command |
| `SetMeteorParams` | when a map is loaded |
| `LoadMeteorDefaults` | when a map's meteor parameters are missing or incomplete |
| `SaveMeteors`, `LoadMeteors` | when a game is saved or loaded |
| `CmdMeteor` | the `meteor` console command |

### The manifest entry

The module's entry in the engine's manifest (`docs/module-abi.md` section 4). The kit doesn't carry the
manifest yet, so its entry is given here in full, and it is exact (ABI 9.1).

- `name` and `interface`: `wind_meteors`.
- `exports`, in this order: `UpdateWind`, `UpdateMeteors`, `InitMeteors`, `EmptyShutdownPreCleanup`,
  `EnableMeteors`, `DisableMeteors`, `SetMeteorParams`, `LoadMeteorDefaults`, `SaveMeteors`,
  `LoadMeteors`, `CmdMeteor`, `UpdateWindOnStream`.
- `data`: `g_game`, `g_simGlobals`, `g_extTdf`, `g_blockBase` (ABI 8.2).
- Slot kinds: every slot returns `void`. `SetMeteorParams` and `LoadMeteorDefaults` take `[linear]`,
  `SaveMeteors` and `LoadMeteors` `[handle]`, `CmdMeteor` `[scratch]`, `UpdateWindOnStream` `[int]` (the
  stream: 0 main, 1 load), and the others `[]`.
- Imports, every one listed and nothing else:

| Import | Kinds | Used for |
| --- | --- | --- |
| `weapons.SpawnProjectile` | `[block, linear, linear, int] -> int` | dropping a meteor |
| `weapons.FindWeaponByName` | `[linear] -> block` | finding the meteor weapon |
| `services.GameRand` | `[int] -> int` | every meteor draw, and the first draw of a wind change |
| `services.RandomInt` | `[int] -> int` | the wind's speed and direction |
| `services.FatalError` | `[linear] -> void` | a bad default meteor data file |
| `services.BuildDataPath` | `[linear, linear, linear, linear] -> linear` | the default data file's path |
| `services.TdfFile_Construct` | `[linear] -> void` | the default data file |
| `services.TdfFile_Destruct` | `[linear] -> void` | the default data file |
| `services.TdfFile_LoadFile` | `[linear, linear] -> int` | the default data file |
| `services.TdfFile_SelectRecord` | `[linear, linear] -> int` | the default data file |
| `services.TdfRecord_GetFieldInt` | `[handle, linear, int] -> int` | the default data file |
| `services.TdfRecord_GetFieldDouble` | `[handle, linear, f64] -> f64` | the default data file |
| `services.TdfRecord_GetFieldString` | `[handle, linear, linear, int, linear] -> int` | the default data file |
| `services.CommandArgs_GetIntArg` | `[scratch, int, int] -> int` | the console command's argument |
| `services.HapiBank_OpenAccount` | `[handle, linear] -> int` | saved games |
| `services.HapiBank_SetIntegerItem` | `[handle, linear, int] -> int` | saved games |
| `services.HapiBank_GetIntegerItem` | `[handle, linear, int] -> int` | saved games |

The saved-game imports are `services` (ABI 9.3), as listed. Where `docs/replay-format.md` calls them
slots of the `files` seam, it describes a different module.

- Fuel: the module meters itself by rule version 1 (ABI section 11) and exports `__fuel_set` and
  `__fuel_get`. The sine table of section 5 belongs in the module as a constant: building it at run time
  would cost fuel on every call.

### Arguments outside the block

- **`MeteorParams`** (`layout.json` record `MeteorParams`, 48 bytes): `name` (32 bytes, a NUL-terminated
  weapon name), `radius` (int), `density`, `duration`, `interval` (each a 32-bit float). It is owned by
  the caller. `LoadMeteorDefaults` writes it in place, and `SetMeteorParams` reads it.
- **The console command** (`CmdMeteor`'s `scratch` argument) is the `CommandArgs` class of
  `interfaces/services.h`. The module reads argument 1 with `CommandArgs_GetIntArg`, and the argument
  count from the `count` field: the 32-bit integer at byte offset 0xd0 of the copy. The count includes
  the command's own name, so `meteor 1` has a count of 2. No import gives the count, so this read is an
  exception to ABI 7.2 ("its raw bytes are for the engine"). The exception stands until the ABI settles it.
- **The `TdfFile`** (ABI 8.1) is three 32-bit words in the module's memory: the root record handle, the
  current record handle (byte offset 4), and flags. "The current record" below means the handle at
  byte offset 4.
- **The saved-game bank** (`SaveMeteors`, `LoadMeteors`) is a `handle`, passed back to the
  `HapiBank_*` services only.

## 3. State

### Owned (`SimGlobals`)

| Field | Type | Meaning |
| --- | --- | --- |
| `g_meteorsEnabled` | int | showers are switched on (1) or off (0) |
| `g_meteorActive` | int | a shower is in progress |
| `g_meteorWeaponName` | char[32] | the meteor weapon's name, from the map |
| `g_meteorWeapon` | `Ref<WeaponDef>` | the meteor weapon, a block offset (`docs/module-abi.md` section 7.1) |
| `g_meteorSpawnMagnitude` | int | the spread of a shower's meteors around its origin |
| `g_meteorSpawnInterval` | int | ticks between two meteors of a shower |
| `g_meteorShowerDuration` | int | ticks a shower lasts |
| `g_meteorScheduleGap` | int | ticks from the end of one shower to the start of the next |
| `g_meteorNextStrikeTime` | int | the tick the next shower starts |
| `g_meteorStrikeEndTime` | int | the tick the current shower ends |
| `g_meteorNextHitTime` | int | the tick the next meteor of the current shower falls |
| `g_meteorTarget` | `Point16` | where the shower's meteors fly towards, in map units (`x`, then `y` holding z) |
| `g_meteorOrigin` | `Point16` | the centre the shower's meteors start from (`x`, then `y` holding z) |

### Owned (`Game`)

`windCounter` (unsigned int, the tick after which the wind next changes), `windSpeed` (int),
`windDirection` (unsigned short, a 16-bit angle: 0x10000 is a full turn), `windX`, `windZ` (int),
`windStrength` (float), `windEnabled` (int, 1 on a tick when the wind changed, else 0).
`windY` is never written.

### Read

`Game.ticks` (int), `Game.width` and `Game.height` (int, the map's size in the same units as the
target), `Game.windSpeedMin`, `Game.windSpeedMax`, `Game.windSpeedDenominator` (int), `Game.weapons`
(the weapon table, `WeaponDef[256]`), `WeaponDef.flags`, and the random streams through the services.

`WeaponDef.flags` is a 32-bit word at offset 273 of a `WeaponDef`. Its bit 5 (mask 0x20) is the
`meteor` bit, set on a weapon that may be a meteor.

### Between ticks

Nothing outside the fields above. Every tick comparison is against `Game.ticks` at the time of the call.

## 4. Behaviour

### `UpdateWindOnStream(stream)` and `UpdateWind()`

`UpdateWind()` has exactly the behaviour of `UpdateWindOnStream(main)`. Let *t* be `Game.ticks`.

If `windCounter` < *t*, compared as unsigned 32-bit integers, the wind changes:

1. `GameRand(stream)` gives *g*. `windCounter` grows by (⌊*g* × 10 / 32768⌋ + 5) × 30, wrapping at
   32 bits. The wind therefore changes every 150 to 420 ticks.
2. `RandomInt(windSpeedMax − windSpeedMin)` gives *n*. `windSpeed` becomes `windSpeedMin` + *n*.
3. If `windSpeed` isn't 0: `RandomInt(65536)` gives the new `windDirection`. Otherwise
   `windDirection` keeps its value, and this draw isn't made.
4. `windX` becomes −S(`windDirection`, `windSpeed`) × 2 and `windZ` becomes
   −C(`windDirection`, `windSpeed`) × 2 (S and C in section 5).
5. `windStrength` becomes `windSpeed` / `windSpeedDenominator`, computed as a 32-bit float division of
   the two values converted to float, and then 1.0 if the quotient is above 1.0. The engine sets
   `windSpeedDenominator` to 5000 at match load. A denominator of 0 never occurs, and the spec doesn't
   define it.
6. `windEnabled` becomes 1.

Otherwise, only `windEnabled` becomes 0.

The imports called, in order: `GameRand(stream)`, `RandomInt`, and then `RandomInt` again if the speed
isn't 0. `RandomInt` draws from its own generator (`rng.seed` in the recordings), whatever `stream` is (Q1).

### `InitMeteors()`

- `g_meteorActive` becomes 0.
- `g_meteorNextStrikeTime` becomes `g_meteorSpawnInterval` (Q2).
- `FindWeaponByName(g_meteorWeaponName)` (the address of the name in the block). If the result is 0,
  or the weapon's `meteor` bit is clear, `g_meteorWeapon` becomes the first entry of `Game.weapons`.
  Otherwise it becomes the weapon found.

### `EmptyShutdownPreCleanup()`

Does nothing.

### `EnableMeteors()`, `DisableMeteors()`

`g_meteorsEnabled` becomes 1, or 0.

### `SetMeteorParams(params)`

- `g_meteorWeaponName` becomes a copy of `params.name`, up to and including its NUL. Callers always
  pass a name that ends within its 32 bytes. Any other name is undefined.
- `g_meteorSpawnMagnitude` becomes `params.radius`.
- `g_meteorSpawnInterval` becomes 30 / `params.density`.
- `g_meteorShowerDuration` becomes `params.duration` × 30.
- `g_meteorScheduleGap` becomes `params.interval` × 30.

The last three are computed in double precision from the float widened to double, then truncated
toward zero to an int (section 5). The parameters are in seconds and the fields in ticks, at 30 ticks a second.

### `LoadMeteorDefaults(params)`

Reads record `Default` of the data file `gamedata/meteor.<ext>` into `params`, where `<ext>` is the
data word `g_extTdf`. The file is a `TdfFile` in the module's memory, and the path is a buffer of at
least 256 bytes. Every outcome starts with these imports:

- `TdfFile_Construct(file)`, then `BuildDataPath(buffer, "gamedata", "meteor", g_extTdf)`, then
  `TdfFile_LoadFile(file, buffer)`.

What follows depends on the outcome:

| Outcome | The imports after those | `params` |
| --- | --- | --- |
| The file doesn't load (`LoadFile` gives 0) | `TdfFile_Destruct(file)` | unchanged (Q3) |
| No `Default` record | `TdfFile_SelectRecord(file, "Default")` = 0, `TdfFile_Destruct(file)` | unchanged (Q3) |
| No `MeteorWeapon` field | `SelectRecord` = 1, `TdfRecord_GetFieldString(current, params.name, "MeteorWeapon", 32, "")` = 0, `FatalError` | as `GetFieldString` left `name` |
| A field is zero | `SelectRecord`, `GetFieldString`, then the fields (below), then `FatalError` | written |
| Good | `SelectRecord`, `GetFieldString`, then the fields (below), then `TdfFile_Destruct(file)` | written |

- **The fields:** `TdfRecord_GetFieldInt(current, "MeteorRadius", 0)` gives `params.radius`. Then
  `TdfRecord_GetFieldDouble(current, key, 0.0)` for `"MeteorDensity"`, `"MeteorDuration"` and
  `"MeteorInterval"`, in that order. Each result is rounded to the nearest float, giving
  `params.density`, `params.duration` and `params.interval`.
- **"A field is zero":** `params.radius` is 0, or any of the three floats is 0.0 after rounding. All four
  are written before `FatalError` is called.
- **`FatalError`** takes the message `Hey, hoser!  The default meteor shower data was bogus!` (two spaces
  after the `!`). It doesn't return, so no `TdfFile_Destruct` follows it.

### `UpdateMeteors()`

Let *t* be `Game.ticks`. All comparisons with *t* are unsigned 32-bit.

1. If `g_meteorNextStrikeTime` ≤ *t*: a shower starts (below). Then, if `g_meteorsEnabled` is 0,
   `g_meteorActive` becomes 0 again (Q4).
2. If `g_meteorActive` isn't 0:
   1. If `g_meteorNextHitTime` ≤ *t*: `g_meteorNextHitTime` becomes `g_meteorSpawnInterval` + *t*, and
      a meteor falls (below).
   2. Then, if `g_meteorStrikeEndTime` ≤ *t*, `g_meteorActive` becomes 0.

**A shower starts:**

- `g_meteorActive` becomes 1.
- `g_meteorStrikeEndTime` becomes `g_meteorShowerDuration` + *t*.
- `g_meteorNextStrikeTime` becomes `g_meteorScheduleGap` + `g_meteorStrikeEndTime`.
- `g_meteorNextHitTime` becomes *t*, so the first meteor falls on the same tick.
- Four draws from the main stream, *d1* to *d4*, in this order:
  - target z = ⌊*d1* × `Game.height` / 32768⌋
  - target x = ⌊*d2* × `Game.width` / 32768⌋
  - offset z = ⌊*d3* × 10 / 32768⌋ − 15, so −15 to −6 (Q5)
  - offset x = ⌊*d4* × 30 / 32768⌋ − 15, so −15 to 14
- `g_meteorTarget` becomes (target x, target z), and `g_meteorOrigin` becomes
  (target x + offset x, target z + offset z). Each is stored as 16-bit values, wrapping.

**A meteor falls:**

- The velocity is (vx, −983040, vz), where vx = (`g_meteorTarget.x` − `g_meteorOrigin.x`) × 2^20 / 90
  and vz is the same with the `y` fields. The multiplication wraps at 32 bits, and the division
  truncates toward zero.
- Two draws from the main stream, *d5* and *d6*, in this order:
  - r = ⌊*d5* × `g_meteorSpawnMagnitude` / 32768⌋, and the spread *R* = r × 65536, wrapping at 32 bits;
  - the angle *a* = *d6* × 2, as a 16-bit angle.
- The position is (`g_meteorOrigin.x` × 2^20 − S(*a*, *R*), 88473600, `g_meteorOrigin.y` × 2^20 − C(*a*, *R*)),
  each wrapping at 32 bits.
- `SpawnProjectile(g_meteorWeapon, position, velocity, 1)`: the weapon as its address (block base plus
  the stored offset, or 0 when the stored offset is 0, as `interfaces/ref.h` converts it), and the position and velocity as `Vec3` records (`x`, `y`, `z`, each a 32-bit
  int) in the module's memory. The result is ignored.

The imports called, in order: four `GameRand(main)` if a shower started, then two `GameRand(main)` and
`SpawnProjectile` if a meteor fell.

### `CmdMeteor(args)`

- If the argument count is above 1: `CommandArgs_GetIntArg(args, 1, 0)`. If the result isn't 0, this
  behaves as `EnableMeteors()`. Otherwise it behaves as `DisableMeteors()`.
- Otherwise a shower starts at once, exactly as in `UpdateMeteors` (with the same four draws), whether
  or not showers are switched on. No meteor falls in this call: the first one falls in the next
  `UpdateMeteors`, on the same tick if it runs at the same `Game.ticks`.

### `SaveMeteors(bank)`

`HapiBank_OpenAccount(bank, "Meteor")`, then `HapiBank_SetIntegerItem` with each of these items, in
this order:

| Item | Value |
| --- | --- |
| `Enabled` | `g_meteorsEnabled` |
| `Active` | `g_meteorActive` |
| `Next Strike Time` | `g_meteorNextStrikeTime` |
| `Time Strike Ends` | `g_meteorStrikeEndTime` |
| `Next Hit Time` | `g_meteorNextHitTime` |
| `Origin X` | `g_meteorOrigin.x`, sign-extended |
| `Origin Z` | `g_meteorOrigin.y`, sign-extended |
| `Target X` | `g_meteorTarget.x`, sign-extended |
| `Target Z` | `g_meteorTarget.y`, sign-extended |

The results are ignored.

### `LoadMeteors(bank)`

`HapiBank_OpenAccount(bank, "Meteor")`, then `HapiBank_GetIntegerItem(bank, item, 0)` for the same
items in the same order. Each result goes to its field: the `Point16` coordinates keep the low 16 bits.
The weapon, its name and the timing parameters aren't saved. They come from the map, as at any match load.

## 5. Exactness

- **Random draws.** `GameRand` returns 0 to 32767. Every scaling written ⌊*d* × *k* / 32768⌋ in this
  spec is the 64-bit product *d* × *k* (no overflow), divided by 32768 and truncated toward zero. That is
  the floor for *k* ≥ 0, and rounds toward zero for a negative *k* (a negative radius or map size).
  `RandomInt` is called with whatever range results. The service itself returns 0 for a range below 2.
  `GameRand(main)` and `RandomInt` advance two different generators (`rng.main` and `rng.seed` in the
  recordings).
- **Integers.** Fields are 32-bit two's complement unless the table says otherwise. Sums and products
  wrap at 32 bits, and multiplying by 2^20 or 65536 is the same as a left shift that wraps. Signed
  division truncates toward zero. Tick comparisons are unsigned. `Point16` coordinates are signed 16-bit:
  in every computation they're sign-extended to 32 bits first, and storing one wraps to 16 bits.
- **Float to int.** A double truncated toward zero to an int. A value outside the int range, or a NaN,
  becomes −2147483648.
- **The table sine and cosine, S(*a*, *s*) and C(*a*, *s*).** *a* is a 16-bit angle (only its low 16
  bits count, 0x10000 a full turn) and *s* a 32-bit int.
  - The table: *T*[*i*] = round(sin(*i* × 2π / 512) × 8192) for *i* = 0 to 511, rounded to nearest,
    with sin computed in double precision.
  - S(*a*, *s*) = *P*(*T*[((*a* + 0x20) >> 7) mod 512], *s*), and C(*a*, *s*) = *P*(*T*[((*a* + 0x4020) >> 7) mod 512], *s*),
    with *a* taken as an unsigned 16-bit value before the addition.
  - *P*(*e*, *s*) = ⌊(*e* × *s* + 4096) / 8192⌋, with the product in 64 bits, keeping the low 32 bits
    as a signed int.
- **Wind strength.** A float division of two floats, correctly rounded (IEEE 754 single precision).

## 6. Quirks

- **Q1.** `UpdateWindOnStream` takes its first draw from the stream it's given, but speed and direction
  always come from `RandomInt`'s own generator (`rng.seed`). A wind change made with the load stream
  therefore draws from both.
- **Q2.** `InitMeteors` schedules the first shower at tick `g_meteorSpawnInterval` (a few ticks into
  the match), not after `g_meteorScheduleGap`.
- **Q3.** A missing default meteor data file, or one without a `Default` record, leaves the params as
  they were, with no error. Only a present but bad record is fatal.
- **Q4.** When showers are switched off, the schedule keeps running. A due shower still makes its four
  draws and moves all three times forward. It just doesn't stay active.
- **Q5.** The origin's z offset is −15 to −6, while its x offset is −15 to 14.

## 7. Examples

From `tests/replay/wind_meteors/` of the kit.

**Wind**, `meteors-earthquake.txt`, call at tick 1 of `UpdateWind`. `windCounter` 0 < 1, with
`windSpeedMin` 1000, `windSpeedMax` 2000 and `windSpeedDenominator` 5000.
- `GameRand main` = 4620 gives ⌊4620 × 10 / 32768⌋ = 1, so the counter becomes (1 + 5) × 30 = 180.
- `RandomInt 1000` = 198 gives speed 1198. `RandomInt 65536` = 29704 gives the direction.
- `windX` = −S(29704, 1198) × 2 = −696, and `windZ` = −C(29704, 1198) × 2 = 2292.
- `windStrength` = 1198 / 5000 = f32:0x3e7559b4.

**Params**, `meteors-earthquake.txt`, call at tick 0 of `SetMeteorParams`. `density` f32:0x3ecccccd
(0.4000000059604645) gives 30 / 0.4000000059604645 = 74.9999988…, truncated to 74. A float division
would give 75: this is why the division is in double. `duration` 4.0 gives 120, and `interval` 720.0
gives 21600.

**A shower and its first meteor**, `meteors-hailstorm.txt`, call at tick 1 of `UpdateMeteors`.
`Game.width` is 384, `Game.height` 320, and `g_meteorSpawnMagnitude` 2800.
- The draws 25202, 2247, 3521 and 7061 give target z 246, target x 26, offset z −14 and offset x −9.
  The target is (26, 246) and the origin (17, 232).
- The velocity is (9 × 2^20 / 90, −983040, 14 × 2^20 / 90) = (104857, −983040, 163111).
- The draws 19902 and 20102 give r = 1700, so *R* = 111411200 and *a* = 40204.
- The position is (90599392, 88473600, 327630432).

**Rounding toward zero**, `console-features.txt`, call at tick 70 of `CmdMeteor` (count 1), then
`UpdateMeteors` at tick 70.
- The draws 11777, 5660, 8954 and 31871 give the target (38, 36) and the origin (52, 23).
- `g_meteorStrikeEndTime` is 70 + 150 = 220, and `g_meteorNextStrikeTime` is 220 + 1800 = 2020.
- The meteor's vx is −14 × 2^20 / 90 = −163111.8…, so −163111.

**The console switch**, `console-features.txt`, call at tick 100 of `CmdMeteor`, with `meteor 1` (count 2).
`CommandArgs_GetIntArg 1 0` = 1 gives `g_meteorsEnabled` 1.

**Not in any recording:** `SaveMeteors`, `LoadMeteors`, `FatalError`, `EmptyShutdownPreCleanup`, the two
Q3 outcomes, and a wind change on the load stream. Only the scenario traces check these.

## 8. Kit version

Written against a trial kit (`"version": null`) cut from commit `ca72eecf3`, interface `wind_meteors`
1.2. To be pinned to the first tagged kit (`interfaces-v*`, `reference-traces-v*`) before the
spec merges.
