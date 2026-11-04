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

The engine's manifest (`docs/module-abi.md` section 4) has no `targeting` entry yet, and the ABI's
kinds (section 7) have none for the list of units that `ai.GetVisibleEnemiesInRadius` fills. Until
both are settled, this section gives what the entry must hold apart from that kind, marked `units`
below.

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
| `ai.GetVisibleEnemiesInRadius` | `[int, linear, int, int, units] -> void` | the candidates of a search |
| `services.RandomInt` | `[int] -> int` | the order of a search and its distances |

- Fuel: the module meters itself by rule version 1 (ABI section 11) and exports `__fuel_set` and
  `__fuel_get`. The budget comes with the manifest entry.
- Hooks: the header names the hook `FilterTargets` for `FindWeaponTarget` and
  `FindBestTargetIfFireAtWill`. This module raises no hook.

### Arguments outside the block

- **`pos`** (`SetWeaponTargetPos`, `GetWeaponTargetPos`) is a `Vec3` in the caller's memory: read by
  `SetWeaponTargetPos`, and written by `GetWeaponTargetPos` only when it returns 1. When it returns 0,
  `pos` is left as it was.
- **The positions given to imports** (`units.GetGroundHeight`, `units.GetSweetSpot`,
  `ai.GetVisibleEnemiesInRadius`) are `Vec3`s in the module's own memory or in the block.
  `units.GetGroundHeight` reads only `x` and `z`; `units.GetSweetSpot` writes all three.
- **The candidate list** given to `ai.GetVisibleEnemiesInRadius` starts empty for each search; the
  import appends the candidates to it, in the order the recording's `o out` line lists them.
- **Text** given to imports (`"StartBuilding"`, `"TargetCleared"`) is a NUL-terminated string in the
  module's own memory.

## 3. State

### Owned

**The weapon slots** of a unit: `Unit.weapons` is three slots of 28 bytes, slot *w* at byte 4 + 28*w*
of the `Unit`. `layout.json` gives no record for a slot, so this spec names its fields as the
recordings do, `weapon{w}TargetX` and so on:

| Bytes of the slot | Replay name | Type | Meaning |
| --- | --- | --- | --- |
| 0 | `weapon{w}TargetX` | signed 16-bit | the target unit's id, or the ground target's *x* |
| 2 | `weapon{w}TargetZ` | signed 16-bit | −32768 for a unit target or none, else the ground target's *z* |
| 12 | `weapon{w}Def` | `Ref<WeaponDef>` | the weapon (read only) |
| 27 | `weapon{w}Flags` | unsigned byte | bit 1 (value 2): *ready* (read only) |

A slot holds:

- *no target* when TargetX = 0 and TargetZ = −32768;
- a *unit target* when TargetZ = −32768 and TargetX ≠ 0: the unit at index TargetX (read as unsigned
  16-bit) of the unit pool, that is at `Game.units` plus TargetX times the `Unit` stride;
- a *ground target* when TargetZ ≠ −32768, at (TargetX × 65536, TargetZ × 65536) in position units.

**`Unit.netDirtyFlags`** (16-bit): every target set clears bits 10 to 14 of it (it becomes
`netDirtyFlags` AND 0x83ff).

### Read

- `Game`: `units`, `seaLevel` (unsigned byte), `uiOptionFlags` (bit 10, value 0x400).
- `Unit`: `pos` (`Vec3`: 32-bit *x*, *y*, *z*, 65536 to a map unit), `id`, `def`, `player`,
  `playerIndex`, `unitDefIndex`, `killCount`, `motion`, `activateFlags`, `flags`.
- `UnitMotion`: `velocity` (`Vec3`).
- `Player`: `active` (int), `type` (byte).
- `UnitDef`: `range` (signed 16-bit), `flags1`, `categories`, `weaponCategories` (`Ref<UnitTypeSet>[3]`).
- `WeaponDef`: `flags`, `speed` (int).
- `UnitTypeSet`: `bits`, 64 bytes. Index *i* is *in the set* when bit (*i* AND 7) of byte (*i* >> 3) is
  set.

### Bits

| Field | Mask | Word used here |
| --- | --- | --- |
| `Unit.flags` | 0x10000000 | *targetable*: the unit takes part |
| `Unit.flags` | 0x00004000 | *excluded*: never chosen while set |
| `Unit.flags` | bits 20 and 21 | the *fire order*, 0 to 3; 2 is fire at will |
| `Unit.activateFlags` | 0x10 | *hidden*, for weapons that respect it |
| `UnitDef.flags1` | 0x00008000 | *always attackable* |
| `UnitDef.flags1` | 0x10000000 | *reach not checked*: the shooter skips the reach import |
| `WeaponDef.flags` | 0x00000080 | the weapon skips hidden targets |
| `WeaponDef.flags` | 0x02000000 | the weapon never leads |

A player is *computer-controlled* when `active` ≠ 0 and `type` = 2.

### Between ticks

Nothing outside the fields above.

## 4. Behaviour

Within one call, only the order of import calls is observable. The block writes are compared at the
end of the call, so the order they are listed in here carries no meaning.

In each slot, *u* is the unit, *w* the weapon (0, 1 or 2) and "the slot" is *u*'s weapon slot *w*.

### `SetWeaponTargetUnit(u, target, w)`

TargetX becomes the low 16 bits of `target.id`, TargetZ becomes −32768, and `u.netDirtyFlags` is cleared
as in section 3. No imports.

### `SetWeaponTargetPos(u, pos, w)`

TargetX becomes *x* = `pos.x` >> 16 and TargetZ becomes *z* = `pos.z` >> 16, each as its low 16 bits
(section 5). When *z* is −32768, TargetZ becomes −32767 instead (Q1). `u.netDirtyFlags` is cleared as in
section 3. No imports.

### `ClearWeaponTarget(u, w)`

| The slot | Imports, in order | Writes |
| --- | --- | --- |
| no target | none | none |
| anything else | `units.FindScript(u.script, "StartBuilding")`, its result ignored (Q2); then `units.StartScriptWithArgs(u.script, "TargetCleared", null, 0, 1, w, 0, 0, 0)`, its result ignored | TargetX = 0, TargetZ = −32768 |

### `ResetWeaponTarget(u, w)`

TargetX becomes 0 and TargetZ −32768. No imports.

### `GetWeaponTargetUnit(u, w)`

The unit target of the slot, or null when the slot holds no unit target. No imports, no writes.

### `GetWeaponTargetPos(u, pos, w)`

| The slot | Imports, in order | Result and writes |
| --- | --- | --- |
| no target | none | 0 |
| a ground target at (*x*, *z*) | `units.GetGroundHeight` of (*x*, *z*), giving *h*₁. If *h*₁ > `Game.seaLevel`: `units.GetGroundHeight` of (*x*, *z*) again, giving *h* (Q3); otherwise *h* = `Game.seaLevel` | 1; `pos` = (*x*, *h* × 65536, *z*) |
| a unit target *t* with `t.unitDefIndex` = 0 | as `ClearWeaponTarget` for the slot: `units.FindScript`, then `units.StartScriptWithArgs`, with the same arguments | 0; TargetX = 0, TargetZ = −32768 |
| a unit target *t* | `units.GetSweetSpot(t, s)` | 1; `pos` = *s*, led when the weapon leads (below) |

The weapon *leads* when all of these hold: the slot is ready, the slot's `WeaponDef` has
0x02000000 clear in `flags` and a `speed` ≠ 0, `t.motion` is not null, and `u.killCount` > 5. Then, with
*d* = `u.pos` − *s* per axis:

- the distance *D* = √(*d*.x² + *d*.y² + *d*.z²), truncated to an integer;
- the time *T* = (*D* × 65536) / `speed`, truncated, then *T*′ = (*T* × 0xcccc) >> 16;
- `pos` = *s* + (`t.motion.velocity` × *T*′) >> 16, per axis.

Section 5 gives the widths.

### `FindWeaponTarget(u, w, useRange)`

The radius *r* is the result of the import `weapons.GetWeaponRange(u, w)` when `useRange` ≠ 0, or
`u.def.range` when it is 0 (with no import). Then the import
`ai.GetVisibleEnemiesInRadius(u.playerIndex, u.pos, r, 0, list)` fills the candidate list.

The search then makes up to 50 *picks*, stopping early when the list is empty. A pick draws
*i* = `services.RandomInt(n)`, with *n* the number of candidates left; candidate *i* is the pick, and the
last candidate of the list takes its place (Q4). The pick *t* is *eligible* when:

1. *t* is targetable and not excluded;
2. `t.def` is always attackable, or `u`'s player is computer-controlled, or bit 10 of
   `Game.uiOptionFlags` is set;
3. `u.def` is reach not checked, or the import `weapons.WeaponCanReachUnit(u, t, w)` returns nonzero;
4. `useRange` ≠ 0, or `t.unitDefIndex` is not in `u.def.categories`;
5. the slot's weapon does not skip hidden targets, or *t* is not hidden.

The reach import is made for a pick exactly when 1 and 2 hold and `u.def` is not reach not checked,
whatever 4 and 5 give. For an eligible pick, *distance* = `services.RandomInt(q)`, with
*q* = hi(*dx*²) + hi(*dz*²), where *dx* and *dz* are `u.pos` − `t.pos` on those axes and hi is defined in
section 5. A pick draws nothing more.

An eligible pick whose `unitDefIndex` is in `u.def.weaponCategories[w]` is a *fallback*; any other is a
*preferred* pick. The result is the preferred pick with the lowest distance, or, when there is none, the
fallback with the lowest distance, or null. Between equal distances the earlier pick wins.

### `FindBestTargetIfFireAtWill(u)`

When `u`'s fire order is 2, the result and every import are those of `FindWeaponTarget(u, 0, 0)`.
Otherwise null, with no imports.

### `ChooseWeapon(u)`

0 when weapon 0 is ready, else 1 when weapon 1 is ready, else `weapon2Flags` AND 2, that is 0 or 2
(Q5). No imports, no writes.

## 5. Exactness

- **Random draws:** every draw is `services.RandomInt`, on the main stream, in the order section 4 gives:
  for each pick, its index draw and then, if it is eligible, its distance draw. A search with no
  candidates draws nothing.
- **Target fields:** TargetX and TargetZ are signed 16-bit. `SetWeaponTargetPos` shifts the 32-bit
  `pos.x` and `pos.z` right by 16 with the sign kept, then keeps the low 16 bits. A ground target's
  position is the signed 16-bit field times 65536, in 32 bits.
- **The ground height:** the import's result is a signed 32-bit integer, compared signed with
  `Game.seaLevel` (0 to 255). `pos.y` is *h* × 65536, wrapping at 32 bits.
- **The distance of a lead:** *d*.x, *d*.y and *d*.z are 32-bit differences that wrap. Each is converted
  to a 64-bit float, and the squares and the two sums are 64-bit operations, each rounded, summed as
  ((*d*.x² + *d*.y²) + *d*.z²). The square root is a 64-bit square root, and *D* is it truncated
  toward zero to a 32-bit integer.
- **The time of a lead:** *D* × 65536 is a 64-bit integer, divided by `speed` with the quotient truncated
  toward zero, and *T* is its low 32 bits. *T* × 0xcccc is a 64-bit product, shifted right by 16 with the
  sign kept, and *T*′ is its low 32 bits. Each velocity component times *T*′ is a 64-bit product, shifted
  right by 16 with the sign kept; its low 32 bits are added to *s* with 32-bit wrapping.
- **The search distance:** *dx* and *dz* are 32-bit differences that wrap; hi(*v*) is the high 32 bits of
  the 64-bit product *v* × *v*. *q* is the 32-bit wrapping sum of the two.
- **The best picks:** a pick replaces the best of its kind only when its distance is strictly lower.
- **The radius:** `UnitDef.range` is sign-extended from 16 bits.
- **`useRange`** counts as set when nonzero, and **the weapon index** of `FindWeaponTarget` is its low
  8 bits.

## 6. Quirks

- **Q1:** a ground target whose *z* field would be −32768 is stored as −32767, so that it isn't read as a
  unit target.
- **Q2:** clearing a target looks up the script `"StartBuilding"` and ignores the answer.
- **Q3:** a ground target above sea level asks for the ground height twice and uses the second answer.
- **Q4:** a search picks candidates in random order by removing each pick and moving the last candidate
  into its place, so the order depends on the draws and on the list's order.
- **Q5:** `ChooseWeapon` returns 2 when only weapon 2 is ready, and 0 when none is, the same as when
  weapon 0 is.

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
   `weapon0Flags` is 18 (ready), the weapon `weapondef#16` has `flags` 0x00080a01 and `speed`
   655360, `Unit#775.motion` is `UnitMotion#36`, and `Unit#37.killCount` is 7. `units.GetSweetSpot`
   gives *s* = (180226455, 17441044, 103711884), and `Unit#37.pos` is (172578459, 851968, 97802795):
   - *d* = (−7647996, −16589076, −5909089), *D* = 19199130;
   - *T* = 19199130 × 65536 / 655360 = 1919913, *T*′ = 1919913 × 0xcccc >> 16 = 1535906;
   - the velocity (−414900, 119718, −130073) adds (−9723624, 2805718, −3048400);
   - `pos` = (170502831, 20246762, 100663484), result 1.
4. **`tests/replay/targeting/orders-guard-attacked.txt`, the call at tick 138 of
   `FindWeaponTarget(Unit#251, 0, 1)`.** `weapons.GetWeaponRange` gives 200, and the list is
   [`Unit#2`, `Unit#3`, `Unit#4`]. `Unit#251`'s player is computer-controlled, its def is not reach not
   checked, and its weapon does not skip hidden targets. All three candidates are targetable, not
   excluded, and their `unitDefIndex` 95 is not in `weaponCategories[0]`, so each is preferred.
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

Not covered by any recording:

- `ChooseWeapon` returning 2, and `SetWeaponTargetPos` with a *z* of −32768 (Q1).
- `FindWeaponTarget` with more than 50 candidates, with a fallback pick chosen, or with two equal
  distances.
- The always-attackable flag, bit 10 of `Game.uiOptionFlags`, `UnitDef.categories` with a candidate
  in it, and a weapon that skips hidden targets. The recordings would match even if each of these
  checks were removed.
- A `killCount` of exactly 5 or 6 when every other lead condition holds.
- `SetWeaponTargetUnit` with a null target, and a ground height below or equal to sea level with a
  nonzero sea level.
- The callback argument of `"TargetCleared"`, which the recordings show as `?`: it is null.

## 8. Kit version

The targeting recordings are not in a tagged kit yet: this spec was written against an untagged kit
export, interfaces version `interfaces-v1` (targeting 1.1). The gate pair for targeting is the first
`reference-traces-v*` tag that holds `tests/replay/targeting/`.
