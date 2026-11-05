# targeting

## 1. Role

Each of a unit's three weapons has a target: nothing, an enemy unit, or a point on the ground. The
targeting subsystem stores and clears those targets, says where a weapon should aim (following a
unit's sweet spot, and leading a moving unit when the weapon and the shooter qualify), picks which
weapon a unit fires first, and searches the enemies a unit can see for the best one to attack.

## 2. Interface

### Slots (`interfaces/targeting.h`, version 1.1)

| Slot | When the engine calls it |
| --- | --- |
| `SetWeaponTargetUnit`, `SetWeaponTargetPos` | when an order or a script gives a weapon a target |
| `ClearWeaponTarget` | when a weapon gives up its target and the unit's script must be told |
| `ResetWeaponTarget` | when a weapon's target is dropped silently |
| `GetWeaponTargetUnit`, `GetWeaponTargetPos` | every tick a weapon aims or fires |
| `FindWeaponTarget` | when a weapon with no target looks for one |
| `FindBestTargetIfFireAtWill` | when an idle unit looks for something to attack |
| `ChooseWeapon` | when a unit attacks, to pick the weapon that fires first |

`ChooseWeapon` takes the unit as its only argument, as the header lists.

### The manifest entry

The kit's manifest (`wasm/modules.json`) carries the slot signatures of this module (`signatures.targeting`)
and its interface major, 1 (`majors.targeting`). It has no build entry for it and no budget: a clean-room
module sets its own (see Fuel). This section gives the entry the module must have.

- `name` and `interface`: `targeting`.
- `exports`, in this order: `SetWeaponTargetUnit`, `SetWeaponTargetPos`, `ClearWeaponTarget`,
  `ResetWeaponTarget`, `GetWeaponTargetUnit`, `GetWeaponTargetPos`, `FindWeaponTarget`,
  `FindBestTargetIfFireAtWill`, `ChooseWeapon`.
- `data`: `g_game`, `g_blockBase` (ABI 8.2).
- Slot kinds:

| Slot | Kinds |
| --- | --- |
| `SetWeaponTargetUnit` | `[block, block, int] -> void` |
| `SetWeaponTargetPos` | `[block, linear, int] -> void` |
| `ClearWeaponTarget`, `ResetWeaponTarget` | `[block, int] -> void` |
| `GetWeaponTargetUnit` | `[block, int] -> block` |
| `GetWeaponTargetPos` | `[block, linear, int] -> int` |
| `FindWeaponTarget` | `[block, int, int] -> block` |
| `FindBestTargetIfFireAtWill` | `[block] -> block` |
| `ChooseWeapon` | `[block] -> int` |

- Imports, every one listed and nothing else:

| Import | Kinds | Used for |
| --- | --- | --- |
| `units.FindScript` | `[block, linear] -> int` | clearing a target |
| `units.StartScriptWithArgs` | `[block, linear, linear, int, int, int, int, int, int] -> int` | telling the script a target is cleared |
| `units.GetGroundHeight` | `[linear] -> int` | the height of a ground target |
| `units.GetSweetSpot` | `[block, linear] -> void` | the point to aim at on a target unit |
| `weapons.GetWeaponRange` | `[block, int] -> int` | the search radius |
| `weapons.WeaponCanReachUnit` | `[block, block, int] -> int` | whether a candidate can be hit |
| `ai.GetVisibleEnemiesInRadiusList` | `[int, linear, int, int, linear, int] -> int` | the candidates of a search |
| `services.RandomInt` | `[int] -> int` | the order of a search and its distances |

- Fuel: the module meters itself by rule version 1 (ABI section 11) and exports `__fuel_set` and
  `__fuel_get`. The manifest gives it no budget; how a manifest sets the budget is in the kit's
  `docs/module-abi.md`, section 11.3.
- Hooks: the header names the hook `FilterTargets` for `FindWeaponTarget` and
  `FindBestTargetIfFireAtWill`. This module raises no hook.

### Arguments outside the block

- **`pos`** (`SetWeaponTargetPos`, `GetWeaponTargetPos`) is a `Vec3` in the caller's memory: read by
  `SetWeaponTargetPos`, and written by `GetWeaponTargetPos` only when it returns 1. When it returns 0,
  `pos` is left as it was.
- **The positions given to imports** (`units.GetGroundHeight`, `units.GetSweetSpot`,
  `ai.GetVisibleEnemiesInRadiusList`) are `Vec3`s in the module's own memory, in the block, or (for
  `GetWeaponTargetPos`) the caller's `pos`. `units.GetGroundHeight` reads only `x` and `z`;
  `units.GetSweetSpot` writes all three.
  What `units.GetGroundHeight` is given for *y* is not compared: the replay compares only *x* and *z* of that
  argument (a `pos=(x,z)` form in the recordings, `docs/replay-format.md`, the `vec3` mode with `"axes":
  "xz"`), as in `i units.GetGroundHeight pos=(42467328,17301504) = 1`. The *y* the game itself passes is the
  *y* the caller's `pos` held on entry, which the module may pass unchanged, or any other value.
- **The candidate list** of a search is what `ai.GetVisibleEnemiesInRadiusList` writes to the buffer the
  module gives it (ABI section 7.3): its arguments are the player, a position, a radius, flags (0), the
  buffer and the buffer's capacity in units. It returns the number of candidates, which may be above the
  capacity, and writes the first of them, up to the capacity, each as a block offset, in the order the
  recordings' `o out` line lists them. A module that gets a count above its capacity calls again with a
  larger buffer. No search in the recordings lists more than 52 candidates, so a buffer of at least 52
  units needs no second call; the kit does not define how a replay judges a second call, and
  ozgb/byte-tactics-moddable#892 will define it. The module
  allocates no heap and uses no scratch (ABI sections 2.1 and 2.2), and its statics are the same at the end
  of each tick as at its start (ABI section 12).
  The recordings show the import as one line, `i ai.GetVisibleEnemiesInRadius`, whose arguments are the
  player index, `pos` written as `pos=(x,y,z)` (the three 32-bit coordinates), the radius and the flags,
  as in `i ai.GetVisibleEnemiesInRadius 1 pos=(221249536,65536,92274688) 200 0`. It has no buffer and no
  capacity. The line `o out [...]` follows it and holds the whole list. After that line come `o Unit#N.*`
  lines (for example `o Unit#2.flags 0x90240321`) and `o UnitDef#N.flags1` lines (8 in the recordings,
  for example `o UnitDef#21.flags1 0x0084008a` in `skirmish4-long` at call 25768), giving the fields of
  the listed units, and of their defs, that changed since they were last written: `flags`, `posX`,
  `posZ`, `def`, `unitDefIndex` and `activateFlags` of a unit. Each search in the recordings has one such
  import.
- **Text** given to imports (`"StartBuilding"`, `"TargetCleared"`) is a NUL-terminated string in the
  module's own memory.

### Events

None. No slot of this module records a sim event (kinds are listed in `docs/sim-events.md`).

## 3. State

### Owned

**The weapon slots** of a unit: `Unit.weapons` is three `UnitWeaponSlot` records of 28 bytes, slot *w* at
byte 4 + 28*w* of the `Unit`. The recordings name the fields of slot *w* `weapon{w}TargetX` and so on:

| Field of `UnitWeaponSlot` | Replay name | Type | Meaning |
| --- | --- | --- | --- |
| `aimTargetXOrUnitId` (byte 0) | `weapon{w}TargetX` | signed 16-bit | the target unit's id, or the ground target's *x* |
| `aimTargetZOrUnitSentinel` (byte 2) | `weapon{w}TargetZ` | signed 16-bit | −32768 for a unit target or none, else the ground target's *z* |
| `weapon` (byte 12) | `weapon{w}Def` | `Ref<WeaponDef>` | the weapon (read only) |
| `flags` (byte 27) | `weapon{w}Flags` | unsigned byte | bit 1 (value 2): *has a weapon* (read only) |

A slot holds:

- *no target* when TargetX = 0 and TargetZ = −32768;
- a *unit target* when TargetZ = −32768 and TargetX ≠ 0: the unit whose index in the unit pool is
  TargetX (read as signed 16-bit). A unit's `id` equals its index in the unit pool, so the unit that
  `SetWeaponTargetUnit` stored by `id` is the one read back;
- a *ground target* when TargetZ ≠ −32768, at (TargetX × 65536, TargetZ × 65536) in position units.

**`Unit.netDirtyFlags`** (16-bit; in `layout.json` its bits are `unknown_ba` (bits 0 and 1), `dirty` (bit 2)
and `short` (bits 3 to 15)): every target set clears bits 10 to 14 of it, which are bits 7 to 11 of `short`
(it becomes `netDirtyFlags` AND 0x83ff). Bits 0 to 9 (`unknown_ba`, `dirty` and bits 0 to 6 of `short`) and bit 15
(bit 12 of `short`) are kept.

### Read

- `Game`: `units`, `seaLevel` (unsigned byte), `uiOptionFlags` (bit 10, value 0x400).
- `Unit`: `pos` (`Vec3`: 32-bit *x*, *y*, *z*, 65536 to a map unit), `id`, `def`, `player`,
  `playerIndex`, `unitDefIndex`, `killCount`, `motion`, `activateFlags`, `flags`, `script`
  (the layout's `Ref<UnitScript>`; the recordings show the object as `CobScript#N`).
- `UnitMotion`: `velocity` (`Vec3`).
- `Player`: `active` (int), `type` (byte).
- `UnitDef`: `range` (signed 16-bit), `flags1`, `categories`, `weaponCategories` (`Ref<UnitTypeSet>[3]`).
- `WeaponDef`: `flags`, `speed` (int).
- `UnitTypeSet`: `bits`, 64 bytes. Index *i* is *in the set* when bit (*i* AND 7) of byte (*i* >> 3) is
  set.

### Bits

| Field | Mask | Name |
| --- | --- | --- |
| `Unit.flags` | 0x10000000 | `alive` (bit 28) |
| `Unit.flags` | 0x00004000 | `killPending` (bit 14) |
| `Unit.flags` | bits 20 and 21 | `fireOrder`, 0 to 3; 2 is fire at will |
| `Unit.activateFlags` | 0x10 | *hidden*, for weapons that respect it |
| `UnitDef.flags1` | 0x00008000 | bit 15 of `flags1`, unnamed in the kit: *always attackable* (a target with it passes check 2 of the search whatever its attacker's player) |
| `UnitDef.flags1` | 0x10000000 | bit 28 of `flags1`, unnamed in the kit: *reach not checked* (the shooter skips the reach import) |
| `WeaponDef.flags` | 0x00000080 | `paralyzer` (bit 7): a weapon with it set does not pick a hidden target |
| `WeaponDef.flags` | 0x02000000 | `cruise` (bit 25): a weapon with it set does not lead |

A player is *computer-controlled* when `active` ≠ 0 and `type` = 2.

### Between ticks

Nothing outside the fields above. The `rng.*` lines of the recordings are the random stream's, which
`services.RandomInt` reads and writes (the replay's stub of that import advances them, as the game's
draw does); this module reads and writes none of them.

## 4. Behaviour

Within one call, only the order of import calls is observable. The block writes are compared at the
end of the call, so the order they are listed in here carries no meaning.

In each slot, *u* is the unit, *w* the weapon (0, 1 or 2) and "the slot" is *u*'s weapon slot *w*. The
engine never calls a slot with a *w* above 2; this spec defines nothing for a larger *w*.

### `SetWeaponTargetUnit(u, target, w)`

TargetX becomes the low 16 bits of `target.id`, TargetZ becomes −32768, and `u.netDirtyFlags` is cleared
as in section 3. No imports. A null `target` is outside the contract: the engine never passes one, and the
module may trap.

### `SetWeaponTargetPos(u, pos, w)`

TargetX becomes *x* = `pos.x` >> 16 and TargetZ becomes *z* = `pos.z` >> 16, each as its low 16 bits
(section 5). When *z* is −32768, TargetZ becomes −32767 instead (Q1). `u.netDirtyFlags` is cleared as in
section 3. No imports.

### `ClearWeaponTarget(u, w)`

| The slot | Imports, in order | Writes |
| --- | --- | --- |
| no target | none | none |
| anything else | `units.FindScript(u.script, "StartBuilding")`, its result ignored (Q2); then `units.StartScriptWithArgs(u.script, "TargetCleared", null, 0, 1, w, 0, 0, 0)`, its result ignored | TargetX = 0, TargetZ = −32768 |

Both imports are always made, with `u.script` passed as it is, here and in the clear of
`GetWeaponTargetPos`; a null `u.script` is outside the contract: the reference faults on it (a unit whose def has no script
data, or a killed unit, has one), and the module may trap.

### `ResetWeaponTarget(u, w)`

TargetX becomes 0 and TargetZ −32768. No imports.

### `GetWeaponTargetUnit(u, w)`

The unit target of the slot, or null when the slot holds no unit target. No imports, no writes.

### `GetWeaponTargetPos(u, pos, w)`

| The slot | Imports, in order | Result and writes |
| --- | --- | --- |
| no target | none | 0 |
| a ground target at (*x*, *z*) = (TargetX × 65536, TargetZ × 65536) | `units.GetGroundHeight` of (*x*, *z*), giving *h*₁. If *h*₁ > `Game.seaLevel`: `units.GetGroundHeight` of (*x*, *z*) again, giving *h* (Q3); otherwise *h* = `Game.seaLevel` | 1; `pos` = (*x*, *h* × 65536, *z*) |
| a unit target *t* with `t.unitDefIndex` = 0 | `units.FindScript(u.script, "StartBuilding")`, its result ignored (Q2); then `units.StartScriptWithArgs(u.script, "TargetCleared", null, 0, 1, w, 0, 0, 0)`, its result ignored | 0; TargetX = 0, TargetZ = −32768 |
| a unit target *t* | `units.GetSweetSpot(t, s)` | 1; `pos` = *s*, led when the weapon leads (below) |

The weapon *leads* when all of these hold: the slot has a weapon (bit 1 of its flags), the slot's `WeaponDef` has
`cruise` clear in `flags` and a `speed` ≠ 0, `t.motion` is not null, and `u.killCount` (unsigned 16-bit) > 5. Then, with
*d* = `u.pos` − *s* per axis:

- the distance *D* = √(*d*.x² + *d*.y² + *d*.z²), truncated to an integer;
- the time *T* = (*D* × 65536) / `speed`, truncated, then *T*′ = (*T* × 0xcccc) >> 16;
- `pos` = *s* + (`t.motion.velocity` × *T*′) >> 16, per axis.

Section 5 gives the widths.

### `FindWeaponTarget(u, w, useRange)`

The result and the imports are those of *the search* for (*u*, *w*, `useRange`), defined here and used
again by `FindBestTargetIfFireAtWill`.

The radius *r* is the result of the import `weapons.GetWeaponRange(u, w)` when `useRange` ≠ 0, or
`u.def.range` when it is 0 (with no import). Then the import
`ai.GetVisibleEnemiesInRadiusList(u.playerIndex, u.pos, r, 0, buffer, capacity)` fills the candidate list.

The search then makes up to 50 *picks*, stopping early when the list is empty. Pick *k* (from 1) draws
*i* = `services.RandomInt(n)`, with *n* the list's length less *k* − 1, and its candidate is the one at
position *i* (from 0) among the candidates not yet picked, which stand in this order (Q4): the list's
order at first, and after a pick at position *p* of *m* candidates, the *m* − 1 left stand as before at
every position but *p*, and when *p* is not the last position, the candidate that stood last now stands at
*p*. The pick *t* is *eligible* when:

1. *t* has `alive` (bit 28 of `t.flags`) set and `killPending` (bit 14) clear;
2. `t.def` is always attackable, or `u`'s player is computer-controlled, or bit 10 of
   `Game.uiOptionFlags` is set;
3. `u.def` is reach not checked, or the import `weapons.WeaponCanReachUnit(u, t, w)` returns nonzero;
4. `useRange` ≠ 0, or `t.unitDefIndex` is not in `u.def.categories`;
5. the slot's weapon has `paralyzer` clear, or *t* is not hidden.

The reach import is made for a pick exactly when 1 and 2 hold and `u.def` is not reach not checked,
whatever 4 and 5 give. For an eligible pick, *distance* = `services.RandomInt(q)`, with
*q* = hi(*dx*²) + hi(*dz*²), where *dx* and *dz* are `u.pos` − `t.pos` on those axes and hi is defined in
section 5. A pick draws nothing more.

An eligible pick whose `unitDefIndex` is in `u.def.weaponCategories[w]` is a *fallback*; any other is a
*preferred* pick. The result is the preferred pick with the lowest distance, or, when there is none, the
fallback with the lowest distance, or null. Between equal distances the earlier pick wins.

### `FindBestTargetIfFireAtWill(u)`

When `u`'s fire order is 2, the result and every import are those of the search for (*u*, 0, 0),
defined under `FindWeaponTarget`: the radius is `u.def.range`, with no range import. Otherwise null, with
no imports.

### `ChooseWeapon(u)`

0 when weapon 0 has a weapon, else 1 when weapon 1 has one, else `weapon2Flags` AND 2, that is 0 or 2
(Q5). No imports, no writes.

## 5. Exactness

- **Random draws:** every draw is `services.RandomInt`, on the main stream, in the order section 4 gives:
  for each pick, its index draw and then, if it is eligible, its distance draw. A search with no
  candidates draws nothing. The module passes each bound to `services.RandomInt` as it is, without a
  guard: a bound of 0 (a *q* of 0) is a call of `services.RandomInt(0)`, and its result is the import's.
- **Target fields:** TargetX and TargetZ are signed 16-bit. `SetWeaponTargetPos` shifts the 32-bit
  `pos.x` and `pos.z` right by 16 with the sign kept, then keeps the low 16 bits. A ground target's
  position is the signed 16-bit field times 65536, in 32 bits.
- **The ground height:** the import's result is a signed 32-bit integer, compared signed with
  `Game.seaLevel` (0 to 255). `pos.y` is *h* × 65536, wrapping at 32 bits.
- **The distance of a lead:** *d*.x, *d*.y and *d*.z are 32-bit differences that wrap. *D* is the
  square root of *d*.x² + *d*.y² + *d*.z², in double precision (64-bit IEEE): each square is rounded to
  double, the sum is rounded as (x² + y²) + z² in that order, and the root is truncated toward zero to a
  signed 32-bit integer. A root of 2^31 or more gives *D* = −2^31; nothing in the engine bounds it, and no
  recording reaches it (they stay below 2^25).
- **The time of a lead:** *T* is signed. It is the quotient of *D* × 65536 by `speed` (both signed, the
  product exact in 64 bits) truncated toward zero, then reduced to its low 32 bits as a signed value, so a
  quotient beyond 32 bits wraps. A negative `speed` is not rejected: it gives *T* the opposite sign to
  *D*, and *T*′ then moves `pos` away from the target's velocity. *T*′ is *T* × 0xcccc divided by 65536
  and rounded toward negative infinity (it always fits 32 bits). Each added offset is the product of a
  velocity component and *T*′ divided by 65536 and rounded toward negative infinity, in exact 64-bit
  arithmetic, reduced to its low 32 bits and added to *s* with 32-bit wrapping.
- **The search distance:** *dx* and *dz* are 32-bit differences that wrap; hi(*v*) is *v* × *v* divided
  by 2^32 and rounded down (the square is exact, at most 2^62). *q* is hi(*dx*) + hi(*dz*) reduced to 32
  bits as a signed value.
- **The best picks:** a pick replaces the best of its kind only when its distance is strictly lower.
- **The radius:** `UnitDef.range` is sign-extended from 16 bits.
- **`useRange`** counts as set when nonzero, and **the weapon index** of `FindWeaponTarget` is its low
  8 bits.

## 6. Quirks

- **Q1:** a ground target whose *z* field would be −32768 is stored as −32767.
- **Q2:** clearing a target makes the import `units.FindScript(u.script, "StartBuilding")` before the
  `"TargetCleared"` call, and nothing depends on its result.
- **Q3:** a ground target above sea level asks for the ground height twice and uses the second answer.
- **Q4:** a draw of a search picks by position among the candidates not yet picked, and a pick moves the
  candidate that stood last to the picked position (section 4), so the candidate a draw yields depends on
  the earlier draws and on the list's order, not only on the draw.
- **Q5:** `ChooseWeapon` returns 0 when no slot has a weapon, and 2 when slot 2 is the only one that has.

## 7. Examples

1. **`tests/replay/targeting/orders-guard-attacked.txt`, call at tick 20 of
   `SetWeaponTargetUnit(Unit#2, Unit#251, 0)`.** `Unit#251.id` is 251, so `Unit#2.weapon0TargetX`
   becomes 251. TargetZ is already −32768 and `netDirtyFlags` is 0, so neither changes.
2. **`tests/replay/targeting/orders-tier2-spider.txt`, calls at ticks 20 and 21 for `Unit#3`, weapon 0.**
   `SetWeaponTargetPos` with `pos` = (42467328, 65536, 17301504) stores TargetX = 42467328 >> 16 = 648
   and TargetZ = 17301504 >> 16 = 264. At tick 21, `GetWeaponTargetPos` asks for the ground height at
   (42467328, 17301504) and gets 1, which is above `Game.seaLevel` (0), so it asks again and gets 1:
   `pos` = (42467328, 65536, 17301504), result 1.
3. **`tests/replay/targeting/skirmish4-long.txt`, the call at tick 25755 of
   `GetWeaponTargetPos(Unit#37, pos, 0)`: a lead.** The slot holds the unit target `Unit#775`.
   `weapon0Flags` is 18 (bit 1 set), the weapon `weapondef#16` has `flags` 0x00080a01 and `speed`
   655360, `Unit#775.motion` is `UnitMotion#36`, and `Unit#37.killCount` is 7. `units.GetSweetSpot`
   gives *s* = (180226455, 17441044, 103711884), and `Unit#37.pos` is (172578459, 851968, 97802795):
   - *d* = (−7647996, −16589076, −5909089), *D* = 19199130;
   - *T* = 19199130 × 65536 / 655360 = 1919913, *T*′ = 1919913 × 0xcccc >> 16 = 1535906;
   - the velocity (−414900, 119718, −130073) adds (−9723624, 2805718, −3048400);
   - `pos` = (170502831, 20246762, 100663484), result 1.
4. **`tests/replay/targeting/orders-guard-attacked.txt`, the call at tick 138 of
   `FindWeaponTarget(Unit#251, 0, 1)`.** `weapons.GetWeaponRange` gives 200, and the list is
   [`Unit#2`, `Unit#3`, `Unit#4`]. `Unit#251`'s player is computer-controlled, its def is not reach not
   checked, and its weapon has `paralyzer` clear. All three candidates have `alive` set and
   `killPending` clear, and their `unitDefIndex` 95 is not in `weaponCategories[0]`, so each is preferred.
   `Unit#251.pos` has *x* 221249536 and *z* 92274688:

   | Pick | Draw | Candidate | *dx*, *dz* | *q* | Distance |
   | --- | --- | --- | --- | --- | --- |
   | 1 | `RandomInt 3` = 2 | `Unit#4` | 8912897, 8912897 | 36992 | 24209 |
   | 2 | `RandomInt 2` = 1 | `Unit#3` | 7492839, 7500034 | 26167 | 1502 |
   | 3 | `RandomInt 1` = 0 | `Unit#2` | 8912896, 5111808 | 24580 | 10558 |

   Each pick's reach import returns 1. The result is `Unit#3`.
5. **`tests/replay/targeting/orders-tier2-spider.txt`, the call at tick 120 of
   `ClearWeaponTarget(Unit#3, 1)`.** The slot holds a target, so the imports are
   `units.FindScript(CobScript#3, "StartBuilding")` (−1, ignored) and then
   `units.StartScriptWithArgs(CobScript#3, "TargetCleared", null, 0, 1, 1, 0, 0, 0)`. TargetX becomes 0
   and TargetZ −32768.

Not recorded in the kit's recordings (`docs/replay-format.md`, "Not recorded"), each with the kit's reason:

- `ChooseWeapon` returning 1 or 2 (every recorded call returns 0): no shipped unit has weapon slot 0
  empty and slot 1 or 2 defined.
- `SetWeaponTargetPos` with a *z* of −32768 (Q1): it needs a world *z* of 32768, and scenarios and the
  harness reject positions off the map.
- A candidate with `alive` clear or `killPending` set: the candidate list holds only live
  units, so neither check can fail.
- Bit 10 of `Game.uiOptionFlags` set when a pick's shooter is not computer-controlled and its def is not
  always attackable: the bit is set in 16 recorded searches (`console-units`, from call 73), but no
  recording holds it deciding a pick. The kit calls it "the menu option bit 4 of the match flags": the
  match flags are the high byte of `Game.uiOptionFlags`, and its value 4 is bit 10 of the 16-bit word.
- The lead with the slot's *has a weapon* bit clear on its own: its only caller skips a slot without that
  bit.
- The lead with a weapon that has `cruise` set or whose `speed` is 0, for a unit with more than five
  kills and the other lead conditions met: not reached by the scenario suite; it would need a veteran
  missile silo or bomber.

Not recorded, and the kit gives no reason:

- `FindBestTargetIfFireAtWill` with a fire order of 1 or 3 (fire order 0 is recorded).
- A `UnitDef.range` that is negative as a signed 16-bit value.
- A lead with `killCount` of exactly 5 or 6 when every other lead condition holds.
- `GetWeaponTargetPos` over ground whose height equals `Game.seaLevel`.
- A *q* that wraps below 0.
- Two eligible picks of one kind with the same distance, a fallback among them.
- A null `weapon{w}Def` when the lead or the paralyzer test reads it.
- A unit target with a negative TargetX.
- A null `u.script` in a clear.
- A null target in `SetWeaponTargetUnit`.
- A weapon index above 2 in any slot.

The callback argument of `"TargetCleared"`, which the recordings show as `?`: it is null.

## 8. Kit version

Kit `interfaces-v3` and `reference-traces-v23`, the versions the gate judges against. Interface
`targeting` 1.1.
