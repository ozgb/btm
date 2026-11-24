# targeting

## 1. Role

Each of a unit's three weapons has a target: nothing, an enemy unit, or a point on the ground. The
targeting subsystem applies the events other modules publish to set, clear, reset and load those targets,
publishes an effect when it clears a weapon's target, says where a weapon should aim (following a
unit's sweet spot, and leading a moving unit when the weapon and the shooter qualify), picks which
weapon a unit fires first, and searches the enemies los_radar lists for a unit, for the best one to attack.

## 2. Interface

### Slots (`interfaces/targeting.h`, version 4.0)

| Slot | When the engine calls it |
| --- | --- |
| `Apply` | once for each event delivered from targeting's inbox, in the order the events were published |
| `GetWeaponTargetUnit` | when another module asks which unit a weapon aims at |
| `GetWeaponTargetPos` | every tick a weapon aims or fires |
| `FindWeaponTarget` | when a weapon with no target looks for one |
| `FindBestTargetIfFireAtWill` | when an idle unit looks for something to attack |
| `ChooseWeapon` | when a unit attacks, to pick the weapon that fires first |
| `SavedTargetPair` | when a game is saved, to read a weapon's target pair; outside the tick |

The events are those of `TargetingEventKind` in the header: `TARGETING_SET_UNIT` (96), `TARGETING_SET_POS`
(97), `TARGETING_CLEAR` (98), `TARGETING_RESET` (99) and `TARGETING_LOAD` (100), called SET_UNIT, SET_POS,
CLEAR, RESET and LOAD below.
Other modules publish them; the engine holds the inbox and delivers it, through `Apply`, before each call of
`GetWeaponTargetUnit`, `GetWeaponTargetPos`, `FindWeaponTarget` and `FindBestTargetIfFireAtWill`, and at
each phase boundary (the header; `docs/schedule.md` lists `targeting.Apply` once in every phase and in the
work after the phases). An
event may also be delivered as soon as it is published, inside its publisher's call (`interfaces/services.h`,
`SchedPublish`). Each `Apply` takes effect at once: an event's changes are all made in its `Apply`.

`ChooseWeapon` takes the unit as its only argument, as the header lists.

### The manifest entry

The kit's manifest (`wasm/modules.json`) carries this module's interface major, 4 (`majors.targeting`), and
the majors of its import groups. It has no build entry for it, and its `signatures` list only the names the
manifest's modules use (ABI section 10.1), so they hold none of this module's slots and, of its imports, only
`services.RandomInt`: the kinds below, in the terms of ABI section 7, are the ones to build against. A
module's build sets its `data_base` and `table_base` (within the windows and table ranges of ABI sections 2.2
and 2.3), its `slots`, and its fuel and size budgets (ABI sections 11.1 and 11.3: a budget belongs to its
build, set from the build's own gate runs). This section gives the rest of the entry the module must have.

- `name` and `interface`: `targeting`.
- `exports`, in this order: `Apply`, `GetWeaponTargetUnit`, `GetWeaponTargetPos`, `FindWeaponTarget`,
  `FindBestTargetIfFireAtWill`, `ChooseWeapon`, `SavedTargetPair`.
- `data`: `g_game`, `g_blockBase` (ABI 8.2).
- `majors`: the majors the module is built against (ABI 10.1, the values of the manifest's `majors`):
  `targeting` 4, and for its import groups `map` 9, `scripts` 3, `weapons` 4, `los_radar` 5, `services` 3.
- Slot kinds:

| Slot | Kinds |
| --- | --- |
| `Apply` | `[linear, int] -> int` |
| `GetWeaponTargetUnit` | `[block, int] -> block` |
| `GetWeaponTargetPos` | `[block, linear, int] -> int` |
| `FindWeaponTarget` | `[block, int, int] -> block` |
| `FindBestTargetIfFireAtWill` | `[block] -> block` |
| `ChooseWeapon` | `[block] -> int` |
| `SavedTargetPair` | `[block, int] -> int` |

- Imports, every one listed and nothing else:

| Import | Kinds | Used for |
| --- | --- | --- |
| `map.GetGroundHeight` | `[linear] -> int` | the height of a ground target (`interfaces/map.h`, version 9.0) |
| `scripts.GetSweetSpot` | `[block, linear] -> void` | the point to aim at on a target unit |
| `weapons.GetWeaponRange` | `[block, int] -> int` | the search radius |
| `weapons.WeaponCanReachUnit` | `[block, block, int] -> int` | whether a candidate can be hit |
| `los_radar.GetVisibleEnemiesInRadius` | `[int, linear, int, int, linear, int] -> int` | the candidates of a search |
| `services.RandomInt` | `[int] -> int` | the order of a search and its distances |
| `services.HarnessTargetingEvent` | `[linear, int, int] -> void` | recording the effect for the dump's targeting events |
| `services.SchedPublish` | `[int, linear, int] -> void` | publishing the effect `TARGETING_EFFECT_CLEARED` |

  The kinds of the services imports are those of ABI section 9.3.
- Fuel: the module meters itself by rule version 1 (ABI section 11) and exports `__fuel_set` and
  `__fuel_get`. Its fuel budget and `size_budget` are its build's
  (ABI sections 11.1 and 11.3), not the kit's. Its stack is the manifest's `stack_size`,
  4096 bytes (ABI sections 2.2 and 4.1). How a manifest sets the budget is in the kit's
  `docs/module-abi.md`, section 11.3 (the game manifest's budget is ten times the most fuel any tick of the
  conformity gate's scenarios burns).
- Hooks: the header names the hook `FilterTargets` for `FindWeaponTarget` and
  `FindBestTargetIfFireAtWill`, and `VeterancyLevel` for `GetWeaponTargetPos`; every other slot has
  `Hooks: none`. ABI section 6 ("Hook points") says such a line fixes only where a hook runs, and that
  Milestone 3 decides how a module calls one; this module raises no hook, and the gate judges these slots
  as it does any other (section 7).

### Arguments outside the block

- **The event** of `Apply` is at a `linear` address the engine gives (ABI section 7), and `size`, the second
  argument, is its size in bytes, an `int` kind read as unsigned 32-bit. The module reads the event and does
  not write it. Every event starts with the header `TargetingEventHeader` (`interfaces/targeting.h`), 12
  bytes, and the payload of its kind (ABI section 6.1, the struct the header names for the kind) adds the
  rest:

| Bytes | Field | Kinds | Type | Replay line |
| --- | --- | --- | --- | --- |
| 0 to 3 | `kind` | all | signed 32-bit | `a event.kind 98` |
| 4 to 7 | `unit` | all | unsigned 32-bit: a block offset, 0 for null | `a event.unit Unit#3` |
| 8 to 11 | `weapon` | all | signed 32-bit | `a event.weapon 1` |
| 12 to 15 | `target` | SET_UNIT (`TargetingSetUnit`) | unsigned 32-bit: a block offset, 0 for null | `a event.target Unit#251` |
| 12 to 23 | `pos` | SET_POS (`TargetingSetPos`) | three signed 32-bit words, *x*, *y*, *z* | `a event.pos0 42467328` (and `pos1`, `pos2`) |
| 12 to 15 | `xOrUnit` | LOAD (`TargetingLoad`) | signed 32-bit | `a event.xOrUnit 3140` |
| 16 to 19 | `zOrSentinel` | LOAD (`TargetingLoad`) | signed 32-bit | `a event.zOrSentinel 1180` |

  A kind's *payload size* is 16 for SET_UNIT, 24 for SET_POS and 20 for LOAD; CLEAR and RESET are the
  header alone, 12. The recordings write `size` after the fields, as `a size 16`, and every recorded event
  has exactly its kind's payload size. `unit` and `target` are stored pointers, so they are offsets from the
  block's base (ABI section 7.1; the recordings write them as `Unit#N` or `null`, the `ref` field kind of
  `docs/replay-format.md`). The unit is at `g_blockBase` plus the offset. The module reads no byte of the
  event at or past `size`.
- **The effect** the module publishes is a `TargetingEventHeader` alone, 12 bytes, in the module's own
  memory (its stack or its statics), given to `services.SchedPublish` as `SchedPublish(9, effect, 12)`: 9 is
  `SCHED_TARGETING`, the module's outbox (`interfaces/services.h`), and 12 the payload's size (ABI section
  6.1). The CLEARED effect of a unit *u* and a weapon *w* has `kind` 112 (`TARGETING_EFFECT_CLEARED`), `unit`
  the block offset of *u* (its address less `g_blockBase`) and `weapon` *w*. The recordings show the import
  as `i services.SchedPublish 9 12`: the outbox and the size are compared, and the record is not recorded
  (the `skip` mode of `docs/replay-format.md`, "Services"). The header says what the subscribers do with it:
  the unit's script runs its TargetCleared function with the weapon's number.
- **The recording of the effect**: just before each `SchedPublish` of an effect, the module calls
  `services.HarnessTargetingEvent(effect, 12, 1)` with the same record and its size (ABI section 9.3, "Who
  records the dump's events"), which records it for the dump's targeting events (`docs/trace-format.md`,
  section 8.26); the engine records the events it delivers to `Apply`, so the module records nothing for
  them. A replay does not record the `HarnessTargetingEvent` call and no trace row hashes the dump's events,
  so neither the recordings nor the traces show it (ABI section 9.3).
- **`pos`** (`GetWeaponTargetPos`) is a `Vec3` in the caller's memory, written only when the slot returns 1.
  When it returns 0, `pos` is left as it was.
- **The positions given to imports** (`map.GetGroundHeight`, `scripts.GetSweetSpot`,
  `los_radar.GetVisibleEnemiesInRadius`) are `Vec3`s in the module's own memory, in the block, or (for
  `GetWeaponTargetPos`) the caller's `pos`. `map.GetGroundHeight` reads only `x` and `z`;
  `scripts.GetSweetSpot` writes all three.
  What `map.GetGroundHeight` is given for *y* is not compared: the replay compares only *x* and *z* of that
  argument (a `pos=(x,z)` form in the recordings, `docs/replay-format.md`, the `vec3` mode with `"axes":
  "xz"`), as in `i map.GetGroundHeight pos=(42467328,17301504) = 1`. The module may pass any *y*.
- **The candidate list** of a search is what `los_radar.GetVisibleEnemiesInRadius` writes to the buffer
  the module gives it (ABI section 7.3): its arguments are the player, a position, a radius, flags (0), the
  buffer and the buffer's capacity in units. It returns the number of candidates, which may be above the
  capacity, and writes the first of them, up to the capacity, each as a block offset, in the order the
  recordings' `o out` line lists them. The list holds each unit once, and ABI section 7.3 says a caller
  sizes the buffer for the most the list can hold: a buffer of one entry per slot of the unit pool never
  gets a count above its capacity. How the replay answers the import is informative here, not part of the
  module's behaviour (`docs/replay-format.md`, "Not recorded", the entry for this import): it answers a
  call from the next recorded line with the same player, position, radius and flags, writes the first
  `capacity` units of the recorded list, in order, and returns the recorded count. What binds the
  module: after a count above the capacity it may repeat the call with the same arguments and a larger
  capacity, which the replay answers from the same line as no further import, and any other extra call of
  the import is a divergence. A search makes the import once in the recordings, then draws, and the draws
  use the full count. Holding the whole list, and the capacity and place of the buffer, are the build's
  choice. No search in the recordings lists more than 52 candidates, and the kit bounds the list only by
  the unit pool (section 7, gaps).
  The constraints the ABI sets on that memory: the module allocates no heap and uses no scratch (ABI
  sections 2.1 and 2.2), and the buffer, and any list the module derives from it, is on its stack or in its
  statics. The stack is 4096 bytes with no guard below it (ABI section 2.2), room for at most 1024 offsets
  less the slot's own use of it. Statics are hashed after every tick of a test-mode run, and a hash that
  differs from the first is fatal (ABI section 12): a buffer in statics must hold the same bytes at the end
  of every tick as at the end of the first. The window is all zero at instantiation (ABI section 2.2).
  The recordings show the import as one line, `i los_radar.GetVisibleEnemiesInRadius`, whose arguments are
  the player index, `pos` written as `pos=(x,y,z)` (the three 32-bit coordinates), the radius and the flags,
  as in `i los_radar.GetVisibleEnemiesInRadius 1 pos=(221249536,65536,92274688) 200 0`. It has no buffer and
  no capacity (the `units-buffer` mode of `docs/replay-format.md`). The line `o out [...]` follows it and
  holds the whole list; its length is the import's count (the line has no `= n`). After that line come
  `o Unit#N.*` lines (for example `o Unit#2.flags 0x90240321`) and `o UnitDef#N.flags1` lines (10 in the
  recordings, for example `o UnitDef#208.flags1 0x0084008a` in `skirmish4-long` at tick 25768), giving the fields of
  the listed units, and of their defs, that changed since they were last written: `flags`, `posX`,
  `posZ`, `def`, `unitDefIndex` and `activateFlags` of a unit. Each search in the recordings has one such
  import.

### Events

None. No slot of this module records a sim event (kinds are listed in `docs/sim-events.md`). The CLEARED
effect is published to the module's outbox through `services.SchedPublish`, not recorded as a sim event.

## 3. State

### Owned

**The weapon slots** of a unit: `Unit.weapons` is three `UnitWeaponSlot` records of 28 bytes, slot *w* at
byte 4 + 28*w* of the `Unit`. The recordings name the fields of slot *w* `weapon{w}TargetX` and so on:

| Field of `UnitWeaponSlot` | Replay name | Type | Meaning |
| --- | --- | --- | --- |
| `aimTargetXOrUnitId` (byte 0) | `weapon{w}TargetX` | signed 16-bit | the target unit's id, or the ground target's *x* |
| `aimTargetZOrUnitSentinel` (byte 2) | `weapon{w}TargetZ` | signed 16-bit | −32768 for a unit target or none, else the ground target's *z* |
| `weapon` (byte 12) | `weapon{w}Def` | `Ref<WeaponDef>` | the weapon (read only) |
| `flags` (byte 27) | `weapon{w}Flags` | unsigned byte | value 2 (bit 1): *has a weapon* (read only) |

A slot holds:

- *no target* when TargetX = 0 and TargetZ = −32768;
- a *unit target* when TargetZ = −32768 and TargetX ≠ 0: the unit at index
  TargetX (read as signed 16-bit) of `Game.units`: its address is `g_blockBase` plus the block offset
  `Game.units` (below) plus the index times 280, the size of `Unit` in `layout.json` (the unit stride
  0x118 of `docs/replay-format.md`). A unit's `id` equals its index, and the unit that a SET_UNIT stored
  by `id` is the one read back;
- a *ground target* when TargetZ ≠ −32768, at (TargetX × 65536, TargetZ × 65536) in position units. A slot
  whose TargetX and TargetZ are both 0 is a ground target at (0, 0), not *no target*.

**`Unit.netDirtyFlags`** (16-bit; in `layout.json` its bits are `unknown_ba` (bits 0 and 1), `dirty` (bit 2)
and `short` (bits 3 to 15)): every SET_UNIT and SET_POS applied clears bits 10 to 14 of it, which are bits 7
to 11 of `short` (it becomes `netDirtyFlags` AND 0x83ff). Bits 0 to 9 (`unknown_ba`, `dirty` and bits 0 to 6
of `short`) and bit 15 (bit 12 of `short`) are kept. A LOAD leaves it as it was.

**Pointers in the block.** Every pointer field the module reads from the block is a `Ref` in `layout.json`,
and so a block offset (ABI section 7.1: 32 bits, 0 for null): `Game.units`, `Unit.def`, `Unit.motion`,
`Unit.player`, the slot's `weapon` (`weapon{w}Def`), `UnitDef.categories` and `UnitDef.weaponCategories`
(three offsets). The record it names is at `g_blockBase` plus the offset, and a *null* field in this spec
(a null `motion`, a null `weapon{w}Def`) is one whose offset is 0. The recordings write these fields as
object names (`r Unit#1.motion UnitMotion#1`, `null` for 0), as they do the event's. The module writes
no pointer field.

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

| Field | Mask | Name |
| --- | --- | --- |
| `Unit.flags` | 0x10000000 | `alive` (bit 28) |
| `Unit.flags` | 0x00004000 | `killPending` (bit 14) |
| `Unit.flags` | bits 20 and 21 | `fireOrder`, 0 to 3; 2 is fire at will |
| `Unit.activateFlags` | 0x10 | *hidden*, which a weapon with `paralyzer` set respects |
| `UnitDef.flags1` | 0x00008000 | bit 15 of `flags1`, unnamed in the kit: a target with it set passes check 2 of the search whether or not the shooter's player is computer-controlled |
| `UnitDef.flags1` | 0x10000000 | bit 28 of `flags1`, unnamed in the kit: a shooter with it set skips the reach import |
| `WeaponDef.flags` | 0x00000080 | `paralyzer` (bit 7): a weapon with it set does not pick a hidden target |
| `WeaponDef.flags` | 0x02000000 | `cruise` (bit 25): a weapon with it set does not lead |

A player is *computer-controlled* when `active` ≠ 0 and `type` = 2.

### Between ticks

Nothing outside the fields above. The module keeps no event between calls: the inbox is the engine's, and
each `Apply` has applied its event when it returns. The recordings show `r rng.main`, `r rng.load` and
`w rng.seed` around the search slots: `docs/replay-format.md` says `rng.main`, `rng.load` and `rng.seed` are
the generator words of the block (in the kit's words, "the two generator streams of the block head and
`g_randomSeed`") and are in the sets of the slots that draw; the nine recordings hold 409 `r rng.seed`
lines. `rng.seed` is the generator of `services.RandomInt` (ABI section 9.3, not `rng.main`), which that
import, answered by the host, advances: a draw returns the recorded result and the stream words advance as
the game's do, and `rng.*` is checked like any field (`docs/replay-format.md`), so `w rng.seed` is compared.
This module reads and writes none of the three itself, and never draws except through
`services.RandomInt`.

## 4. Behaviour

Within one call, only the order of import calls is observable. The block writes are compared at the
end of the call, so the order they are listed in here carries no meaning.

In each slot, *u* is the unit, *w* the weapon (0, 1 or 2) and "the slot" is *u*'s weapon slot *w*. The
engine never calls `GetWeaponTargetUnit`, `GetWeaponTargetPos` or `FindWeaponTarget` with a *w* (for
`FindWeaponTarget`, its low 8 bits) above 2. An input *outside the contract* is one this spec sets no
result for: such a *w*, and the inputs section 7 lists as outside the contract. A trap fails a scenario
that reaches it (ABI section 13); the kit does not say whether the gate can reach any of these inputs
(section 7, gaps).

*Publishing CLEARED* for *u* and *w* is the import `services.HarnessTargetingEvent(effect, 12, 1)` and then
the import `services.SchedPublish(9, effect, 12)`, both with the CLEARED effect of *u* and *w* (section 2);
neither returns anything, and the recordings show only the second.

### `Apply(event, size)`

*u* is the unit at the block offset `event.unit`, null when it is 0; *w* is `event.weapon`, a signed
32-bit value; *t* is the unit at the block offset `event.target` of a SET_UNIT, null when it is 0. The
result is the event's *outcome*.

| The event | Imports, in order | Writes | Outcome |
| --- | --- | --- | --- |
| `size` below 12 | none | none | 0 |
| any kind, with *u* null or *w* outside 0 to 2 | none | none | 0 |
| SET_UNIT, SET_POS or LOAD with `size` below its kind's payload size (section 2) | none | none | 0 |
| SET_UNIT with *t* null | none | none | 0 |
| SET_UNIT | none | TargetX = the low 16 bits of `t.id`, TargetZ = −32768; `u.netDirtyFlags` cleared as in section 3 | 1 |
| SET_POS | none | TargetX = `pos[0]` >> 16 and TargetZ = `pos[2]` >> 16, each as its low 16 bits (section 5), except that a TargetZ of −32768 is stored as −32767 (Q1); `u.netDirtyFlags` cleared as in section 3 | 1 |
| CLEAR, the slot with no target | none | none | 0 |
| CLEAR, the slot with a unit or a ground target | publishing CLEARED for *u* and *w* | TargetX = 0, TargetZ = −32768 | 1 |
| RESET, the slot with no target | none | none | 0 |
| RESET, the slot with a unit or a ground target | none | TargetX = 0, TargetZ = −32768 | 1 |
| LOAD | none | TargetX = the low 16 bits of `xOrUnit`, TargetZ = the low 16 bits of `zOrSentinel` (section 5), whatever the slot held | 1 |
| any other kind | none | none | 0 |

`pos[1]` is not read. A LOAD writes the two fields and nothing else: `u.netDirtyFlags` is kept, a
TargetZ of −32768 is stored as it is (Q1 is SET_POS's alone), and no CLEARED is published. A SET_UNIT
whose `t.id` is 0 leaves the slot with no target (section 3), with outcome 1. The `event` record is not
written. A nonzero `event.unit` or `event.target` that is not the offset of a unit is outside the contract.
A SET_UNIT reads `t.id` and nothing else of *t*: the target's own `netDirtyFlags` appears among the recorded
reads (for example `r Unit#3.netDirtyFlags 1024` at tick 138 of `orders-guard-attacked`), is not changed,
and no result depends on it.

### `GetWeaponTargetUnit(u, w)`

The unit target of the slot, or null when the slot holds no unit target. The unit is returned whether or not
it is alive. The result is a `block` kind: the address of the unit (the base of the block, `g_blockBase`, plus
its offset, ABI section 7.1), which the recordings show as `Unit#N`. No imports, no writes.

### `GetWeaponTargetPos(u, pos, w)`

| The slot | Imports, in order | Result and writes |
| --- | --- | --- |
| no target | none | 0 |
| a ground target at (*x*, *z*) = (TargetX × 65536, TargetZ × 65536) | `map.GetGroundHeight` of (*x*, *z*), giving *h*₁. If *h*₁ > `Game.seaLevel`: `map.GetGroundHeight` of (*x*, *z*) again, giving *h* (Q2); otherwise *h* = `Game.seaLevel` | 1; `pos` = (*x*, *h* × 65536, *z*) |
| a unit target *t* with `t.unitDefIndex` = 0 | publishing CLEARED for *u* and *w* | 0; TargetX = 0, TargetZ = −32768 |
| a unit target *t* | `scripts.GetSweetSpot(t, s)` | 1; `pos` = *s*, led when the weapon leads (below) |

The weapon *leads* when all of these hold: the slot has a weapon (value 2 of its flags), the slot's
`WeaponDef` has `cruise` clear in `flags` and a `speed` ≠ 0, `t.motion` is not null,
and `u.killCount` (unsigned 16-bit) > 5. Then, with *d* = `u.pos` − *s* per axis:

- the distance *D* = √(*d*.x² + *d*.y² + *d*.z²), truncated to an integer;
- the time *T* = (*D* × 65536) / `speed`, truncated, then *T*′ = (*T* × 0xcccc) >> 16;
- `pos` = *s* + (`t.motion.velocity` × *T*′) >> 16, per axis.

The factor is 0xcccc / 65536, about 0.8.

When `t.motion` is null there is no lead and `pos` = *s*, result 1: in `edges-economy-targeting`, from tick
126 on, `Unit#1` aims weapon 0 at `Unit#502`, whose `motion` is null, and the call returns the sweet spot
`scripts.GetSweetSpot` gave (`o pos.x 45605925`, `o result 1`), recorded 66 times.

Section 5 gives the widths.

### `FindWeaponTarget(u, w, useRange)`

The result and the imports are those of *the search* for (*u*, *w*, `useRange`), defined here and used
again by `FindBestTargetIfFireAtWill`.

The radius *r* is the result of the import `weapons.GetWeaponRange(u, w)` when `useRange` ≠ 0, or
`u.def.range` when it is 0 (with no import). Then the import
`los_radar.GetVisibleEnemiesInRadius(u.playerIndex, u.pos, r, 0, buffer, capacity)` fills the candidate
list, whose length is the count the import returns, the whole list even when it is above the capacity.

The search makes at most 50 *picks* (13 recorded searches have more than 50 candidates), and ends sooner
when no candidate is left. Pick *k* (from 1) draws *i* = `services.RandomInt(n)`, with *n* the list's
length less *k* − 1, and its candidate is the one at position *i* (from 0) of the candidates not yet
picked. Those stand in this order (Q3): the list's order for pick 1; after a
pick the last candidate moves into the picked position (unless it was the last) and the list shortens by
one. The pick *t* is *eligible* when:

1. *t* has `alive` (bit 28 of `t.flags`) set and `killPending` (bit 14) clear;
2. `t.def` has `flags1` bit 15 set, or `u`'s player is computer-controlled, or bit 10 of
   `Game.uiOptionFlags` is set;
3. `u.def` has `flags1` bit 28 set, or the import `weapons.WeaponCanReachUnit(u, t, w)` returns nonzero;
4. `useRange` ≠ 0, or `t.unitDefIndex` is not in `u.def.categories`;
5. the slot's weapon has `paralyzer` clear, or *t* is not hidden.

The reach import is made for a pick exactly when 1 and 2 hold and `u.def` has `flags1` bit 28 clear,
whatever 4 and 5 give. For an eligible pick, *distance* = `services.RandomInt(q)`, with
*q* = hi(*dx*) + hi(*dz*), where *dx* and *dz* are `u.pos` − `t.pos` on those axes and hi is defined in
section 5. A pick draws nothing more.

An eligible pick whose `unitDefIndex` is in `u.def.weaponCategories[w]` is a *fallback*; any other is a
*preferred* pick. The result is the preferred pick with the lowest distance, or, when there is none, the
fallback with the lowest distance, or null. Between equal distances the earlier pick wins. No writes.

### `FindBestTargetIfFireAtWill(u)`

When `u`'s fire order is 2, the result and every import are those of the search for (*u*, 0, 0),
defined under `FindWeaponTarget`: the radius is `u.def.range`, with no range import. Otherwise null, with
no imports. No writes.

### `ChooseWeapon(u)`

The index of the first slot (0, 1, 2) that has a weapon, or 0 when none has (Q4). A slot has a weapon
when value 2 of its `flags` is set, and nothing else of the slot is read: the kit's "empty" slot and
"a weapon of ID 0" (`docs/replay-format.md`, "Not recorded") are the data that leaves that bit clear. No
imports, no writes.

### `SavedTargetPair(u, w)`

The slot's target as a saved game keeps it, one 32-bit word: TargetX's 16 bits in bits 0 to 15 and
TargetZ's 16 bits in bits 16 to 31. A LOAD with `xOrUnit` = TargetX and `zOrSentinel` = TargetZ stores the
same pair back. A slot with no target gives 0x80000000 (−2147483648 as a signed 32-bit result). No
imports, no writes. The engine calls it outside the tick, while a game is saved; `save-targets` records
12 calls, weapons 0 to 2 of four units (example 8). The gate also judges the saved game
(`docs/reference-traces.md`: the reference's saves of `save-midgame` and `save-targets` are stored, a save
is compared with the reference's by decoded contents, and `load-midgame` and `load-targets` play on from
them).

## 5. Exactness

- **Random draws:** every draw is `services.RandomInt`, on the `rng.seed` generator of ABI section 9.3, in
  the order section 4 gives: for each pick, its index draw and then, if it is eligible, its distance draw.
  A search with no candidates draws nothing. `Apply` and `GetWeaponTargetPos` never draw. Each bound is
  drawn as it stands: a *q* of 0 still makes the call `services.RandomInt(0)`, whose result is the import's
  (ABI section 9.3: a range below 2 gives 0 and leaves the generator alone). A *q* that wraps to a negative
  value is drawn as that negative bound, so its distance is 0 and the generator is left alone.
- **Target fields:** TargetX and TargetZ are signed 16-bit. A SET_POS shifts the 32-bit `pos[0]` and
  `pos[2]` right by 16 with the sign kept, then keeps the low 16 bits. A LOAD keeps the low 16 bits of
  the 32-bit `xOrUnit` and `zOrSentinel`, with no shift. The header gives each as a 16-bit value, and
  `zOrSentinel` sign-extended to 32 bits (−32768 as 0xffff8000), so the low 16 bits are that value; a
  field outside the 16-bit range keeps its low 16 bits all the same. A ground target's position is the
  signed 16-bit field times 65536, in 32 bits.
- **The event's size:** `size` is compared as an unsigned 32-bit value with 12 and with the kind's
  payload size; a larger `size` than the payload's is accepted, and the bytes past the payload are not
  read.
- **The event's weapon:** `event.weapon` is compared as a signed 32-bit value: a negative one is outside
  0 to 2.
- **The ground height:** the import's result is a signed 32-bit integer, compared signed with
  `Game.seaLevel` (0 to 255): a height strictly above `seaLevel` asks again and is used, and a height at or
  below it gives *h* = `seaLevel` after the one call. The kit's "ground under the sea
  (`map.GetGroundHeight` returns 1)" is the second case, with a `seaLevel` of 44 (11 calls in
  `edges-economy-targeting`, each with one import call). A height equal to `seaLevel` is the second case
  too: in `edges-targeting-lead`, with `seaLevel` 1, a height of 1 gives one import call and `o pos.y 65536`
  (16 calls, from tick 11). With `seaLevel` 0 a height of 1 is the first case, as in example 2. `pos.y`
  is *h* × 65536, wrapping at 32 bits.
- **The distance of a lead:** *d*.x, *d*.y and *d*.z are 32-bit differences that wrap. *D* is the
  square root of *d*.x² + *d*.y² + *d*.z², in double precision (64-bit IEEE): each square is rounded to
  double, the sum is rounded as (x² + y²) + z² in that order, and the root is truncated toward zero to a
  signed 32-bit integer. A root of 2^31 or more gives *D* = −2^31; nothing in the engine bounds it, and no
  recording reaches it (they stay below 2^25). The order of the rounding is part of the result: the replay
  compares `pos` exactly.
- **The time of a lead:** *T* is signed. It is the quotient of *D* × 65536 by `speed` (both signed, the
  product exact in 64 bits) truncated toward zero, then reduced to its low 32 bits as a signed value, so a
  quotient beyond 32 bits wraps. A negative `speed` is not rejected: it gives *T* the opposite sign to
  *D*, and *T*′ then moves `pos` away from the target's velocity. *T*′ is *T* × 0xcccc divided by 65536
  and rounded toward negative infinity (it always fits 32 bits). Each added offset is the product of a
  velocity component and *T*′ divided by 65536 and rounded toward negative infinity, in exact 64-bit
  arithmetic, reduced to its low 32 bits and added to *s* with 32-bit wrapping.
- **The search distance:** *dx* and *dz* are 32-bit differences that wrap; hi(*d*) is *d* × *d* divided
  by 2^32 and rounded down (the square is exact, at most 2^62). *q* is hi(*dx*) + hi(*dz*) reduced to 32
  bits as a signed value.
- **The best picks:** a pick replaces the best of its kind only when its distance is strictly lower.
- **The radius:** `UnitDef.range` is sign-extended from 16 bits, and a negative value is passed to the
  candidate import as that negative radius.
- **`useRange`** counts as set when nonzero, and **the weapon index** of `FindWeaponTarget` is its low
  8 bits, and that masked value is the one every import and slot field uses.

## 6. Quirks

- **Q1:** a ground target whose *z* field would be −32768 is stored as −32767.
- **Q2:** a ground target above sea level asks for the ground height twice and uses the second answer.
- **Q3:** the candidate a draw yields depends on the earlier draws and on the list's order, not only on
  the draw: the candidates not yet picked stand in the order section 4 gives, which is not the list's order
  with the picked ones left out.
- **Q4:** `ChooseWeapon` returns 0 when no slot has a weapon, and 2 when slot 2 is the only one that has.

## 7. Examples

1. **`tests/replay/targeting/orders-guard-attacked.txt`, the second `Apply` at tick 20: a SET_UNIT.**
   `event.kind` 96, `event.unit` `Unit#2`, `event.weapon` 0, `event.target` `Unit#251`, `size` 16.
   `Unit#251.id` is 251, so `Unit#2.weapon0TargetX` becomes 251. TargetZ is already −32768 and
   `netDirtyFlags` is 0, so neither changes. No imports, `o result 1`.
2. **`tests/replay/targeting/orders-tier2-spider.txt`, the SET_POS at tick 20 and the call at tick 21 for
   `Unit#3`, weapon 0.** The `Apply` with `event.kind` 97, `pos` = (42467328, 65536, 17301504) and `size`
   24 stores TargetX = 42467328 >> 16 = 648 and TargetZ = 17301504 >> 16 = 264, outcome 1. At tick 21,
   `GetWeaponTargetPos` asks for the ground height at (42467328, 17301504) and gets 1, which is above
   `Game.seaLevel` (0), so it asks again and gets 1: `pos` = (42467328, 65536, 17301504), result 1.
3. **`tests/replay/targeting/skirmish4-long.txt`, the call at tick 25755 of
   `GetWeaponTargetPos(Unit#37, pos, 0)`: a lead.** The slot holds the unit target `Unit#775`.
   `weapon0Flags` is 18 (value 2 set), the weapon `weapondef#16` has `flags` 0x00080a01 and `speed`
   655360, `Unit#775.motion` is `UnitMotion#36`, and `Unit#37.killCount` is 7. `scripts.GetSweetSpot`
   gives *s* = (180226455, 17441044, 103711884), and `Unit#37.pos` is (172578459, 851968, 97802795):
   - *d* = (−7647996, −16589076, −5909089), *D* = 19199130;
   - *T* = 19199130 × 65536 / 655360 = 1919913, *T*′ = 1919913 × 0xcccc >> 16 = 1535906;
   - the velocity (−414900, 119718, −130073) adds (−9723624, 2805718, −3048400);
   - `pos` = (170502831, 20246762, 100663484), result 1.
4. **`tests/replay/targeting/orders-guard-attacked.txt`, the second `FindWeaponTarget` call of
   tick 138, `FindWeaponTarget(Unit#251, 0, 1)`.** `weapons.GetWeaponRange` gives 200, and the list is
   [`Unit#2`, `Unit#3`, `Unit#4`]. `Unit#251`'s player is computer-controlled, its def has `flags1` bit 28
   clear (so the reach import is made), and its weapon has `paralyzer` clear. All three candidates have
   `alive` set and `killPending` clear, and their `unitDefIndex` 95 is not in `weaponCategories[0]`, so each
   is preferred. `Unit#251.pos` has *x* 221249536 and *z* 92274688:

   | Pick | Draw | Candidate | *dx*, *dz* | *q* | Distance |
   | --- | --- | --- | --- | --- | --- |
   | 1 | `RandomInt 3` = 2 | `Unit#4` | 8912897, 8912897 | 36992 | 24209 |
   | 2 | `RandomInt 2` = 1 | `Unit#3` | 7492839, 7500034 | 26167 | 1502 |
   | 3 | `RandomInt 1` = 0 | `Unit#2` | 8912896, 5111808 | 24580 | 10558 |

   Each pick's reach import returns 1. The result is `Unit#3`, and the SET_UNIT that follows at the same
   tick stores 3 in `Unit#251.weapon0TargetX`.
5. **`tests/replay/targeting/orders-tier2-spider.txt`, the four `Apply` calls of tick 120 for `Unit#3`: CLEAR.**
   The first clears weapon 0, which holds a target: the recorded import `i services.SchedPublish 9 12`
   (publishing CLEARED for `Unit#3` and weapon 0, after the unrecorded `HarnessTargetingEvent`), TargetX
   becomes 0 and TargetZ −32768, `o result 1`. The second clears weapon 0 again, which now holds no target: no
   import, no write, `o result 0`. The third clears weapon 1, which holds a target: the same import, TargetX
   0, TargetZ −32768, `o result 1`. The fourth clears weapon 2, which holds no target: no import, no write,
   `o result 0`.
6. **`tests/replay/targeting/orders-tier2-spider.txt`, the first `Apply` at tick 0: a RESET of a new unit.**
   `Unit#1.weapon0TargetX` and `weapon0TargetZ` are both 0, a ground target at (0, 0), so the slot holds a
   target: TargetZ becomes −32768 (TargetX is already 0) with no import, `o result 1`. A RESET of a slot with
   no target has outcome 0, as at tick 160 of `console-units` (`Unit#251`, weapon 0).
7. **`tests/replay/targeting/console-units.txt`, the call at tick 140 of
   `GetWeaponTargetPos(Unit#252, pos, 0)`.** The slot holds the unit target `Unit#2`, whose `unitDefIndex`
   is 0: the recorded import `i services.SchedPublish 9 12` (publishing CLEARED for `Unit#252` and weapon 0,
   after the unrecorded `HarnessTargetingEvent`), TargetX becomes 0 (TargetZ is already −32768), `o result 0`,
   and `pos` is not written.
8. **`tests/replay/targeting/save-targets.txt` and `load-targets.txt`, tick 150: a saved and a loaded
   target pair.** In `save-targets`, `Unit#3`'s weapon 0 holds the ground target TargetX = 3140, TargetZ =
   1180, so `SavedTargetPair(Unit#3, 0)` is 1180 × 65536 + 3140 = 0x049c0c44, `o result 77335620`; weapon 1
   holds the same pair and gives the same word. `Unit#2`'s weapon 0 holds the unit target `Unit#251`
   (TargetX = 251, TargetZ = −32768): 0x8000 × 65536 + 251 = 0x800000fb, `o result -2147483397`. Every
   other weapon of the four units has no target and gives `o result -2147483648`. In `load-targets`, the
   LOAD of `Unit#3`, weapon 0, with `xOrUnit` 3140, `zOrSentinel` 1180 and `size` 20 writes TargetX = 3140
   and TargetZ = 1180 over the RESET slot (TargetX 0, TargetZ −32768), keeps `Unit#3.netDirtyFlags` 8192,
   and has `o result 1`.

Recorded, with counts over the nine recordings:

- `Apply`: 792 SET_UNIT, all with outcome 1; 16 SET_POS, all with outcome 1; 1712 CLEAR, 419 of them of a
  slot with a target (one `services.SchedPublish` each, outcome 1) and 1293 of a slot with none (outcome 0);
  351 RESET, 348 with outcome 1 and 3 with outcome 0; 12 LOAD, all in `load-targets`, all with outcome 1
  (one of a unit target, two of a ground target and nine of no target). Each has its kind's payload size:
  16, 24, 12, 12 and 20.
- `SavedTargetPair`: 12 calls, all in `save-targets` at tick 150, one for each weapon of four units: one of
  a unit target, two of a ground target and nine of no target.
- `GetWeaponTargetPos` of a unit target whose `unitDefIndex` is 0: 7 calls (3 in `console-units`, 4 in
  `edges-targeting-lead`), each publishing CLEARED.

The outcomes of the next two lists are defined, where at all, by this spec alone: no recording checks them.

Not recorded in the kit's recordings, and reached by no scenario (`docs/replay-format.md`, "Not recorded"),
each with the kit's reason:

- `ChooseWeapon` with no slot that has a weapon (it returns 0), and with slot 1 or 2 deciding the result:
  all 347 recorded calls have slot 0 with a weapon. The kit's reason for returning 1 or 2: no shipped unit
  has weapon slot 0 empty and slot 1 or 2 defined. Its reason for no weapon at all: the only caller is the
  attack order, which runs only for a unit with the armed bit, and such a unit has a slot with a weapon.
- An `Apply` of a SET_POS with a *z* of −32768 (Q1): it needs a world *z* of 32768, and scenarios and the
  harness reject positions off the map.
- An `Apply` of an event it rejects (a null unit, a weapon outside 0 to 2, or a SET_UNIT with a null
  target): every publisher passes a live unit and its own weapon number.
- A candidate with `alive` clear or `killPending` set: the candidate list holds only live
  units, so neither check can fail.
- `FindBestTargetIfFireAtWill` with a fire order of 3. The kit's reason: a unit's fire order comes from its
  unit type's standing fire order (0 or 2 in the shipped units, 2 when it has none), from an order whose
  modes are 0, 1 and 2, or from the unit that built it.
- A negative `UnitDef.range`. The kit's reason: the range is the unit type's `sightdistance`, which lies
  between 55 and 490 in the shipped units. Section 5 gives the result.
- The lead with the slot's *has a weapon* bit clear on its own: its only caller skips a slot without that
  bit.
- The lead with a weapon that has `cruise` set or whose `speed` is 0, for a unit with more than five
  kills and the other lead conditions met: not reached by the scenario suite; it would need a veteran
  missile silo or bomber.

Not recorded, and the kit gives no reason and does not say whether a scenario reaches them:

- An `Apply` of a kind other than the five (outcome 0, no change).
- An `Apply` with `size` below 12, or below its kind's payload size (outcome 0, no change).
- Negative `speed` in a lead (section 5 gives the result).
- A *q* that wraps below 0 (section 5 gives the result).
- Two eligible fallback picks with the same distance. Preferred picks that tie are recorded, at distance 0
  (`RandomInt 0 = 0`): the earlier pick wins, as at tick 73 of `console-units`.
- A null `weapon{w}Def` when the lead or the paralyzer test reads it: outside the contract.
- A unit target with a negative TargetX, or with a TargetX outside the unit pool: outside the contract.
- A weapon index above 2 in `GetWeaponTargetUnit`, `GetWeaponTargetPos` or `SavedTargetPair`, and in
  `FindWeaponTarget` a weapon index whose low 8 bits (section 5) are above 2: outside the contract.
- A nonzero `event.unit` or `event.target` that is not the offset of a unit: outside the contract.
- A ground target with a negative coordinate (section 5 gives the shift).
- A `unitDefIndex` of 512 or more (a `UnitTypeSet` holds 512 bits) given to a set test, and a null
  `categories`, `weaponCategories`, `u.def`, `t.def` or `u.player`: outside the contract.
- `netDirtyFlags` bits 0, 1, 3 to 9, 11 and 15 before a SET_UNIT or SET_POS: the recordings set only bits 2,
  10, 12, 13 and 14, so the kept and cleared rule for the others is checked by no recording.
- `GetWeaponTargetUnit` on a dead or recycled target.

Recorded, in `orders-immediate` and `edges-targeting-lead` (the kit's `docs/replay-format.md` lists both):

- `FindBestTargetIfFireAtWill` with a fire order of 1: the call at tick 752 of `orders-immediate`
  (`Unit#1.flags` 0x90d00321), which returns null with no import. Fire order 0 is recorded in 8 calls, and
  fire order 2 in the rest.
- `GetWeaponTargetPos` over ground whose height equals `Game.seaLevel` (section 5, the ground height).
- Bit 10 of `Game.uiOptionFlags` deciding a pick: the bit is set in 174 recorded searches (16 in
  `console-units`, from the `FindBestTargetIfFireAtWill` call at tick 73, and 158 in `edges-targeting-lead`).
  In 9 searches of `edges-targeting-lead` it is what makes a pick eligible: `Unit#1`, whose player is not
  computer-controlled, picks `Unit#502`, whose def has `flags1` bit 15 clear, first in the
  `FindWeaponTarget` call at tick 83 (`o out [Unit#502]`, `o result Unit#502`).
- The lead's kill count. The unit-target calls of `GetWeaponTargetPos` with every other lead condition met
  have a `killCount` of 0 (12058 calls), 1 (279), 2 (267), 3 (133), 4 (140), 5 (133), 6 (129), 7 (121) or
  8 (69). With 5 or fewer, `pos` is the sweet spot; with 6 or more the lead applies, and it moves `pos` in
  79 calls: one with 6 kills (`Unit#2` at tick 1072 of `edges-targeting-lead`, with the velocity of
  `UnitMotion#8`) and 78 with 7 (62 in `edges-targeting-lead`, and the 16 of `skirmish4-long`, among them
  example 3). In the others the velocity adds nothing. The kit says the target moves in the calls with 4
  and 5 kills (`docs/replay-format.md`, the list of `edges-targeting-lead`).

Recorded once, so a single recorded call or search checks it:

- `FindWeaponTarget` with weapon 1: one recorded call.
- A search whose result is a fallback pick: one recorded search.

Gaps in the kit, which this spec does not fill:

- The inputs outside the contract (section 4): the kit does not say whether the gate can reach any of
  them, and so whether a module's result or trap there can be judged.
- The candidate list's maximum: ABI section 7.3 bounds it by the slots of the unit pool, but the kit gives
  no fixed number of them that a module must hold, against the 4096-byte stack of ABI section 2.2
  (section 2).
- The update rule of the `rng.seed` generator: `docs/replay-format.md` says a draw returns the recorded
  result and the stream words advance as the game's do, and ABI section 9.3 gives the range-below-2 case,
  but neither states how the generator is updated. The `w rng.seed` lines are the recorded result.
- The CLEARED record: the recordings carry only the outbox and the size of `services.SchedPublish`, not the
  record's fields, so no recording checks its `kind`, `unit` or `weapon`.

The `FilterTargets` and `VeterancyLevel` hooks are not gate rules: no recording, schema, glue or import of
the kit names them, and `docs/module-abi.md` (section 6, "Hook points") fixes only where a hook runs, leaving
how a module calls one to Milestone 3 (this repository's issue #1). So a module raises none, and the gate
judges `FindWeaponTarget`, `FindBestTargetIfFireAtWill` and `GetWeaponTargetPos` as it does any other slot.

No rule of the candidate tests names a kamikaze unit: the one that `docs/replay-format.md` mentions among
the recorded candidates is judged like any other candidate.

The layout has unaligned fields, such as `Unit.pos` at byte 106 and `UnitDef.flags1` at byte 577
(`layout.json`): the module reads and writes each at its offset, and `docs/module-abi.md` says nothing
else of alignment.

## 8. Kit version

Kit `interfaces-v32` and `reference-traces-v33`, the versions the gate judges against. Interface
`targeting` 4.0.
