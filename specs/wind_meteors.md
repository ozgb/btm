# wind_meteors

## 1. Role

Wind and meteors are the two pieces of weather in a match. Every few seconds the wind picks a new
speed and direction within the map's limits; wind generators and anything else that reads the wind
use the result. Meteors are showers of a map-chosen weapon falling on a random spot of the map: a
schedule starts a shower, a shower drops one projectile at a fixed interval for a fixed duration,
and the next shower comes a fixed gap after the last one ends. The engine switches showers on and
off, sets their parameters and the wind's limits, and starts a shower at once, through events in the
module's inbox. The meteor state is kept in saved games.

## 2. Interface

### Slots (`interfaces/wind_meteors.h`, version 3.1)

| Slot | When the engine calls it |
| --- | --- |
| `UpdateWind` | every game tick, in the wind phase, with the main stream; and once at match load, with the load stream |
| `UpdateMeteors` | every game tick, in the meteors phase |
| `Apply` | for each event of the module's inbox, in the order it was published (`docs/schedule.md`, "Routes") |
| `InitMeteors` | at match load, after the map's meteor parameters are applied and the weapon types are loaded |
| `EmptyShutdownPreCleanup` | when a match shuts down |
| `LoadMeteorDefaults` | when a map's meteor parameters are missing or incomplete |
| `SaveMeteors`, `LoadMeteors` | when a game is saved or loaded |

### The manifest entry

The kit's manifest (`wasm/modules.json`) has the module's entry, and the module must match it exactly (ABI 9.1). In full:

- `name` and `interface`: `wind_meteors`.
- `exports`, in this order (the header's): `UpdateWind`, `UpdateMeteors`, `Apply`, `InitMeteors`,
  `EmptyShutdownPreCleanup`, `LoadMeteorDefaults`, `SaveMeteors`, `LoadMeteors`.
- `data`: `g_game`, `g_simGlobals`, `g_extTdf`, `g_blockBase` (ABI 8.2).
- `majors`: `wind_meteors` 3, `weapons` 4, `services` 4.
- `publishes`: `{"80": 36}`, the SPAWN effect (`WindMeteorsSpawn`, 36 bytes).
- `data_base` 196608, `table_base` 48, `fuel_budget` 250, `size_budget` 13100 (ABI 2, 11.3 and 16).
- Slot kinds:

| Slot | Kinds |
| --- | --- |
| `UpdateWind` | `[int] -> void` (the stream: 0 main, 1 load) |
| `UpdateMeteors`, `InitMeteors`, `EmptyShutdownPreCleanup` | `[] -> void` |
| `Apply` | `[linear, int] -> void` |
| `LoadMeteorDefaults` | `[linear] -> void` |
| `SaveMeteors`, `LoadMeteors` | `[handle] -> void` |

- Imports, every one listed and nothing else:

| Import | Kinds | Used for |
| --- | --- | --- |
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
| `services.HapiBank_OpenAccount` | `[handle, linear] -> int` | saved games |
| `services.HapiBank_SetIntegerItem` | `[handle, linear, int] -> int` | saved games |
| `services.HapiBank_GetIntegerItem` | `[handle, linear, int] -> int` | saved games |
| `services.Emit` | `[linear, int] -> void` | publishing a meteor (SPAWN) |

- Fuel: the module meters itself by rule version 1 (ABI section 11) and exports `__fuel_set` and
  `__fuel_get`. The budget is 250 units per tick for all of the module's slot calls together (ABI section
  11.3, and the appendix in section 16).

### Arguments outside the block

- **The event** (`Apply`'s two arguments): the address of a `WindMeteorsEventHeader`, and the size in bytes of
  the whole event, payload included (ABI 6.1, `docs/replay-format.md`, "Event arguments"). The payload of each
  kind is the struct the interface header names for it, laid out in declaration order with no padding:
  - `WindMeteorsParams`, 52 bytes: `kind` (0), `weapon` (4, 32 bytes, a NUL-terminated weapon name), `radius`
    (36, int), `density` (40), `duration` (44), `interval` (48), each of the last three a 32-bit float.
  - `WindMeteorsLimits`, 12 bytes: `kind` (0), `min` (4), `max` (8), ints.
  - ENABLE, DISABLE, START and RESET are the header alone, 4 bytes.
- **The effect** (`Emit`'s two arguments): the address of a `WindMeteorsSpawn` in the module's memory and its
  size, 36. Its layout: `kind` (0, 80), `weapon` (4), `pos` (8, three ints: x, y, z), `vel` (20, three ints),
  `announce` (32).
- **`MeteorShowerParams`** (`layout.json` record `MeteorShowerParams`, 48 bytes): `weapon` (32 bytes, a
  NUL-terminated weapon name), `radius` (int), `density`, `duration`, `interval` (each a 32-bit float). It is
  owned by the caller, and `LoadMeteorDefaults` writes it in place.
- **The `TdfFile`** (ABI 8.1) is three 32-bit words in the module's memory: the root record handle, the
  current record handle (byte offset 4), and flags. "The current record" below means the handle at
  byte offset 4.
- **The saved-game bank** (`SaveMeteors`, `LoadMeteors`) is a `handle`, passed back to the
  `HapiBank_*` services only.

### Events

None. No slot of this module records a sim event (kinds are listed in `docs/sim-events.md`).

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

`windCounter` (unsigned int, the tick after which the wind next changes), `windSpeedMin`, `windSpeedMax`
(int, the map's wind limits), `windSpeedDenominator` (int), `windSpeed` (int), `windDirection` (unsigned
short, a 16-bit angle: 0x10000 is a full turn), `windX`, `windZ` (int), `windStrength` (float),
`windEnabled` (int, 1 on a tick when the wind changed, else 0). `windY` is never written.

### Read

`Game.ticks` (int), `Game.width` and `Game.height` (int, the map's size in the same units as the
target), `Game.weapons` (the weapon table, `WeaponDef[256]`, each entry 277 bytes), `WeaponDef.flags`,
and the random streams through the services.

`WeaponDef.flags` bit 5 (mask 0x20, `meteor` in `layout.json`) is set on a weapon that may be a meteor.

Every offset is the one `layout.json` gives for the record and field. Some `Game` fields lie at offsets
that aren't multiples of their size (`windDirection` at 229080, `windSpeed` at 229082): they are read and
written in place at those offsets, each with its own width, so a write of the 2-byte `windDirection`
changes those 2 bytes only.

### Between ticks

Nothing outside the fields above. Every tick comparison is against `Game.ticks` at the time of the call.

## 4. Behaviour

Within one call, only the order of import calls is observable. The block writes are compared at the
end of the call, so the order they are listed in here carries no meaning.

### `UpdateWind(stream)`

Let *t* be `Game.ticks`.

If `windCounter` < *t*, compared as unsigned 32-bit integers, the wind changes:

1. `GameRand(stream)` gives *g*. `windCounter` grows by (⌊*g* × 10 / 32768⌋ + 5) × 30, wrapping at
   32 bits. The wind therefore changes every 150 to 420 ticks.
2. `RandomInt(windSpeedMax − windSpeedMin)` gives *n*. `windSpeed` becomes `windSpeedMin` + *n*.
3. If `windSpeed` isn't 0: `RandomInt(65536)` gives the new `windDirection`. Otherwise
   `windDirection` keeps its value, and this draw isn't made.
4. `windX` becomes −S(`windDirection`, `windSpeed`) × 2 and `windZ` becomes
   −C(`windDirection`, `windSpeed`) × 2 (S and C in section 5).
5. `windStrength` becomes `windSpeed` / `windSpeedDenominator`, computed as a 32-bit float division of
   the two values converted to float, and then 1.0 if the quotient is above 1.0. RESET sets
   `windSpeedDenominator` to 5000 before the first call. A denominator of 0 never occurs, and the spec
   doesn't define it.
6. `windEnabled` becomes 1.

Otherwise, only `windEnabled` becomes 0.

The imports called, in order: `GameRand(stream)`, `RandomInt`, and then `RandomInt` again if the speed
isn't 0. `RandomInt` draws from its own generator (`rng.seed` in the recordings), whatever `stream` is (Q1).

### `Apply(event, size)`

If `size` is below 4, nothing changes and `kind` isn't read. Otherwise, by `kind`:

| `kind` | Needs `size` of at least | Effect |
| --- | --- | --- |
| 64 ENABLE | 4 | `g_meteorsEnabled` becomes 1 |
| 65 DISABLE | 4 | `g_meteorsEnabled` becomes 0 |
| 66 PARAMS | 52 | the shower parameters are set (below) |
| 67 START | 4 | a shower starts (section 4, "A shower starts"), whether or not showers are switched on |
| 68 LIMITS | 12 | `Game.windSpeedMin` becomes `min`, and `Game.windSpeedMax` becomes `max` |
| 69 RESET | 4 | `Game.windSpeedDenominator` becomes 5000, and `Game.windCounter` becomes 0 |
| any other | | nothing |

A PARAMS or LIMITS event whose `size` is below its payload's changes nothing. The only imports any event
makes are START's four draws. START makes no meteor fall: the first one falls in the next `UpdateMeteors`,
on the same tick if it runs at the same `Game.ticks`.

**The shower parameters** (PARAMS), from the payload's fields:

- `g_meteorWeaponName` becomes a copy of `weapon`, up to and including its NUL. The bytes of
  `g_meteorWeaponName` after that NUL keep the values they had. Publishers always give a name that ends
  within its 32 bytes. Any other name is undefined.
- `g_meteorSpawnMagnitude` becomes `radius`.
- `g_meteorSpawnInterval` becomes 30 / `density`.
- `g_meteorShowerDuration` becomes `duration` × 30.
- `g_meteorScheduleGap` becomes `interval` × 30.

The last three are computed in double precision from the float widened to double, then truncated
toward zero to an int (section 5). The parameters are in seconds and the fields in ticks, at 30 ticks a second.

### `InitMeteors()`

- `g_meteorActive` becomes 0.
- `g_meteorNextStrikeTime` becomes `g_meteorSpawnInterval` (Q2).
- `FindWeaponByName(name)`, where *name* is the address of the 32 bytes of
  `g_meteorWeaponName`: the field in the block, or a copy of its 32 bytes in the module's memory. If the result is 0,
  or the weapon's `meteor` bit is clear, `g_meteorWeapon` becomes the first entry of `Game.weapons`.
  Otherwise it becomes the weapon found. Either way it's stored as a block offset: the weapon's address
  minus `g_blockBase`. For the first entry of `Game.weapons`, that's the game record's address (the value the
  data word `g_game` holds, ABI 8.2) plus the field's offset in `layout.json` (11507), minus `g_blockBase`.

### `EmptyShutdownPreCleanup()`

Does nothing.

### `LoadMeteorDefaults(params)`

Reads record `Default` of the data file `gamedata/meteor.<ext>` into `params`, where `<ext>` is the
data word `g_extTdf`. The file is a `TdfFile` in the module's memory, and the path buffer must be large
enough for the path the service writes: here `gamedata\meteor.tdf` and its NUL, 20 bytes (the recordings'
`BuildDataPath` line). Every outcome starts with these imports:

- `TdfFile_Construct(file)`, then `BuildDataPath(buffer, "gamedata", "meteor", g_extTdf)`, then
  `TdfFile_LoadFile(file, buffer)`.

What follows depends on the outcome:

| Outcome | The imports after those | `params` |
| --- | --- | --- |
| The file doesn't load (`TdfFile_LoadFile` gives 0) | `TdfFile_Destruct(file)` | unchanged (Q3) |
| No `Default` record | `TdfFile_SelectRecord(file, "Default")` = 0, `TdfFile_Destruct(file)` | unchanged (Q3) |
| No `MeteorWeapon` field | `TdfFile_SelectRecord` = 1, `TdfRecord_GetFieldString(current, params.weapon, "MeteorWeapon", 32, "")` = 0, `FatalError` | as `TdfRecord_GetFieldString` left `weapon` |
| A field is zero | `TdfFile_SelectRecord`, `TdfRecord_GetFieldString`, then the fields (below), then `FatalError` | written |
| Good | `TdfFile_SelectRecord`, `TdfRecord_GetFieldString`, then the fields (below), then `TdfFile_Destruct(file)` | written |

- **The fields:** `TdfRecord_GetFieldInt(current, "MeteorRadius", 0)` gives `params.radius`. Then
  `TdfRecord_GetFieldDouble(current, key, 0.0)` for `"MeteorDensity"`, `"MeteorDuration"` and
  `"MeteorInterval"`, in that order. Each result is rounded to the nearest float, giving
  `params.density`, `params.duration` and `params.interval`.
- **"A field is zero":** `params.radius` is 0, or any of the three floats is 0.0 after rounding. All four
  are written before `FatalError` is called.
- **`FatalError`** takes the message `Hey, hoser!  The default meteor shower data was bogus!` (two spaces
  after the `!`). It doesn't return: the import returns an error, the call unwinds as a trap does, and the game ends with
  the message (ABI 13). No import follows it, and nothing the slot would have done after it happens.
- **`TdfRecord_GetFieldString`'s arguments** are, in order: the record, the destination (`params.weapon`),
  the key, the size and the default (`interfaces/services.h`).

The block is not written.

### `UpdateMeteors()`

Let *t* be `Game.ticks`. All comparisons with *t* are unsigned 32-bit.

1. If `g_meteorNextStrikeTime` ≤ *t*: a shower starts (below). Then, if `g_meteorsEnabled` is 0,
   `g_meteorActive` becomes 0 again (Q4).
2. If `g_meteorActive` isn't 0:
   1. If `g_meteorNextHitTime` ≤ *t*: `g_meteorNextHitTime` becomes `g_meteorSpawnInterval` + *t*, and
      a meteor falls (below).
   2. Then, if `g_meteorStrikeEndTime` ≤ *t*, `g_meteorActive` becomes 0.

**A shower starts** (here, and on START):

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
- `Emit(effect, 36)`, with the `WindMeteorsSpawn` effect: `kind` 80, `weapon` the meteor weapon's slot in
  `Game.weapons`, `pos` the position, `vel` the velocity, and `announce` 1. The slot is the weapon's address
  (`g_blockBase` plus `g_meteorWeapon`) minus the address of `Game.weapons`, divided by 277; it is −1 when
  `g_meteorWeapon` is 0. The host refuses a SPAWN whose `weapon` is outside the table, as an import error
  (ABI 13), so a meteor falling while `g_meteorWeapon` is 0 ends the game. In a match it doesn't occur:
  `InitMeteors` stores a weapon before the first tick.

The imports called, in order: four `GameRand(main)` if a shower started, then two `GameRand(main)` and
`Emit` if a meteor fell.

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

The results of all ten imports are ignored. In particular, every item is written even if
`HapiBank_OpenAccount` gives 0. What the bank then does with the items is up to the service.

### `LoadMeteors(bank)`

`HapiBank_OpenAccount(bank, "Meteor")`, then `HapiBank_GetIntegerItem(bank, item, 0)` for the same
items in the same order. Each result goes to its field, whatever `HapiBank_OpenAccount` gave: the
service returns the default the module passes (0) for an item it doesn't have. The `Point16` coordinates keep the low 16 bits.
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
- **The weapon's slot.** The difference of the two addresses is a multiple of 277 for every weapon
  `InitMeteors` stores, so the division is exact.
- **Float to int.** The double truncated toward zero to a 64-bit integer, keeping its low 32 bits as a
  signed int (as in `docs/files-gate.md`). A NaN, an infinity
  or a magnitude of 2^63 or more gives 0. No value traps: a `density` of 0.0 gives 30 / 0 = +∞, so
  `g_meteorSpawnInterval` 0, and a `density` of f32:0x3089705f (about 1e-9) gives 30000000848.45…,
  truncated to 30000000848, whose low 32 bits are −64770224.
- **The table sine and cosine, S(*a*, *s*) and C(*a*, *s*).** *a* is a 16-bit angle (only its low 16
  bits count, 0x10000 a full turn) and *s* a 32-bit int.
  - The table: *T*[*i*] = round(sin(*i* × 2π / 512) × 8192) for *i* = 0 to 511, rounded to nearest,
    with sin computed in double precision. No entry is a tie. It is fixed data: *T*[0] to *T*[5] are 0, 101,
    201, 302, 402, 502, *T*[64] is 5793, *T*[128] is 8192, *T*[256 − *i*] = *T*[*i*], *T*[256 + *i*] = −*T*[*i*],
    and *T*[0] + … + *T*[128] is 671631.
  - S(*a*, *s*) = *P*(*T*[⌊(*a* + 32) / 128⌋ mod 512], *s*), with *a* taken as an unsigned 16-bit
    value (0 to 65535). The table entry is the nearest one, a tie going up.
  - C(*a*, *s*) = S(*a* + 0x4000, *s*): the cosine is the sine a quarter turn on.
  - *P*(*e*, *s*) = ⌊(*e* × *s* + 4096) / 8192⌋, with the product in 64 bits, keeping the low 32 bits
    as a signed int.
- **Wind strength.** A float division of two floats, correctly rounded (IEEE 754 single precision).

## 6. Quirks

- **Q1.** `UpdateWind` takes its first draw from the stream it's given, but speed and direction
  always come from `RandomInt`'s own generator (`rng.seed`). A wind change made with the load stream
  therefore draws from both.
- **Q2.** `InitMeteors` schedules the first shower at tick `g_meteorSpawnInterval`, not after `g_meteorScheduleGap`.
- **Q3.** A missing default meteor data file, or one without a `Default` record, leaves the params as
  they were, with no error. Only a present but bad record is fatal.
- **Q4.** When showers are switched off, the schedule keeps running. A due shower still makes its four
  draws and moves all three times forward. It just doesn't stay active.
- **Q5.** The origin's z offset is −15 to −6, while its x offset is −15 to 14.

## 7. Examples

From `tests/replay/wind_meteors/` of the kit.

**Wind**, `meteors-earthquake.txt`, call at tick 1 of `UpdateWind` (`a stream main`). `windCounter` 0 < 1, with
`windSpeedMin` 1000, `windSpeedMax` 2000 and `windSpeedDenominator` 5000.
- `GameRand main` = 4620 gives ⌊4620 × 10 / 32768⌋ = 1, so the counter becomes (1 + 5) × 30 = 180.
- `RandomInt 1000` = 198 gives speed 1198. `RandomInt 65536` = 29704 gives the direction.
- `windX` = −S(29704, 1198) × 2 = −696, and `windZ` = −C(29704, 1198) × 2 = 2292.
- `windStrength` = 1198 / 5000 = f32:0x3e7559b4.

**No wind change**, `meteors-hailstorm.txt`, call at tick 0 of `UpdateWind` (`a stream load`), after RESET: the
counter 0 isn't below 0, so there is no draw and `windEnabled` is 0.

**Params**, `meteors-earthquake.txt`, call at tick 0 of `Apply` with `a event.kind 66` and `a size 52`. `density`
f32:0x3ecccccd (0.4000000059604645) gives 30 / 0.4000000059604645 = 74.9999988…, truncated to 74. A float division
would give 75. `duration` 4.0 gives 120, and `interval` 720.0 gives 21600. In `meteors-hailstorm.txt`,
`density` 400.0 gives 30 / 400 = 0.075, so `g_meteorSpawnInterval` 0.

**Limits and reset**, `meteors-hailstorm.txt`, calls at tick 0 of `Apply`: LIMITS with `min` 2200 and `max` 4000 (size 12)
writes `windSpeedMin` 2200 and `windSpeedMax` 4000; RESET (size 4) writes `windSpeedDenominator` 5000 (`windCounter` was
0 already).

**The weapon**, `console-features.txt`, call at tick 0 of `InitMeteors`. `FindWeaponByName "Meteor"` gives
`weapondef#225`, whose flags 0x00000020 have the `meteor` bit, so `g_meteorWeapon` is that weapon, and
`g_meteorNextStrikeTime` becomes `g_meteorSpawnInterval`, 15. Its meteors are published with `weapon` 225.

**A shower and its first meteor**, `meteors-hailstorm.txt`, call at tick 1 of `UpdateMeteors`.
`Game.width` is 384, `Game.height` 320, and `g_meteorSpawnMagnitude` 2800.
- The draws 25202, 2247, 3521 and 7061 give target z 246, target x 26, offset z −14 and offset x −9.
  The target is (26, 246) and the origin (17, 232).
- The velocity is (9 × 2^20 / 90, −983040, 14 × 2^20 / 90) = (104857, −983040, 163111).
- The draws 19902 and 20102 give r = 1700, so *R* = 111411200 and *a* = 40204.
- The position is (90599392, 88473600, 327630432).
- The effect is `Emit (80,226,90599392,88473600,327630432,104857,-983040,163111,1) 36`.

**START and rounding toward zero**, `console-features.txt`, call at tick 70 of `Apply` with `a event.kind 67`, then
`UpdateMeteors` at tick 70.
- The draws 11777, 5660, 8954 and 31871 give the target (38, 36) and the origin (52, 23).
- `g_meteorStrikeEndTime` is 70 + 150 = 220, and `g_meteorNextStrikeTime` is 220 + 1800 = 2020.
- In `UpdateMeteors`, the meteor's vx is −14 × 2^20 / 90 = −163111.8…, so −163111, and `g_meteorNextHitTime` becomes
  70 + 15 = 85.

**The switch**, `console-features.txt`, calls of `Apply` at tick 100 (ENABLE) and 150 (DISABLE), each with `a size 4`:
`g_meteorsEnabled` becomes 1, then 0.

**Not in any recording:** `SaveMeteors`, `LoadMeteors`, `FatalError`, `EmptyShutdownPreCleanup`, the two
Q3 outcomes, a wind change on the load stream, an event shorter than its payload or of another kind, and a
meteor with no weapon stored (`weapon` −1). Only the scenario traces check these.

## 8. Kit version

Kit `interfaces-v59` and `reference-traces-v53`, the versions the gate judges against. Interface
`wind_meteors` 3.1. The imports are against `weapons` 4.0 and `services` 4.0.
