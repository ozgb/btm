# los_radar

## 1. Role

Line of sight and radar decide what each player sees of the map and of the other players' units. The
module keeps, for every unit slot, where the unit's sight was last put on the map; it counts, cell by cell,
how many sights cover each cell of a player's explored map, and marks in the visibility mask the cells each
player has ever seen. Sight takes one of two shapes: a mask from the sight mask table, placed around the unit
(the circle type), or a set of lines from the true line of sight tables that stop where the terrain rises
above the eye (the true type). Eyeballs are short-lived sights of the viewing player. Each tick the module
also works out which units radar and sonar show and jamming hides, cloaks units whose enemies come near,
keeps for each player an index of the enemy units it sees, and records two snapshots for the radar picture.
It also answers queries: the map cell under a position, which players hear a sound, whether a unit or a
point is visible, and which seen enemies are near a position.

## 2. Interface

### Slots (`interfaces/los_radar.h`, version 5.1)

| Slot | When the engine calls it |
| --- | --- |
| `Apply` | to deliver each event of the module's inbox (the kinds of `LosRadarEventKind`), before each pass below and at each phase boundary |
| `GetMapCell`, `GetMapCellAtPosition`, `GetOriginCellAtPosition` | from other modules and the engine, whenever they want a map cell |
| `ClampWorldPosToTerrain` | from the engine and other modules, to turn a map position into a world position on the terrain |
| `IsUnitVisibleToPlayer` | from other modules, whenever they ask |
| `SoundHeardMask` | when a sound is played at a position |
| `UpdateUnitLineOfSight` | the players phase's line of sight pass: once a tick for each unit slot of a player that holds a unit |
| `RecordRadarUnits` | after the line of sight pass of the viewing player |
| `UpdateSensorRadarAndCloak` | in the 30-tick update of the viewing player, after its economy |
| `RecordRadarMapped` | right after `UpdateSensorRadarAndCloak` |
| `ExpireEyeballs` | once a tick, after the last phase |
| `GetVisibleEnemiesInRadius`, `HasReadyUnitInRange`, `GetSeenUnitsInRadius` | from other modules (targeting, the AI), whenever they ask |
| `IsPointVisible` | from other modules, whenever they ask |
| `CreateUnitIndex`, `FreeUnitIndex` | when a player's index is made at match start, and freed at match end |
| `RefreshUnitIndex` | once a tick for each player that has an index |
| `RegionSize` | at match load, before `Init`, to size the region |
| `Init` | once per match, before the first tick and before any event, with the region the engine allocated |
| `Shutdown` | at match end; the engine frees the region it returns |

`LoadTntMap`, `InitRadar`, `FreeRadar`, `FreeMapResources`, `SightCache`, `WriteState`, `LoadMappingData`
and `SaveMappingData` are the entry's `engine` slots: the module does not export them, and the host calls the
engine's native slot instead, with the module's state in the block as the native slot reads it (ABI section
6, "Engine slots"). "The engine slots" in section 4 states what each reads of the module's state, so that the
module's region is the one they read.

### The manifest entry

The kit's manifest (`wasm/modules.json`) has the entry `los_radar`, and the signatures
`signatures.los_radar`. The module must have that entry, which is:

- `name` and `interface`: `los_radar`.
- `majors`: `los_radar` 5, `network` 2, `map` 9, `services` 4.
- `data_base` 425984 (0x68000) and `table_base` 272, within the window of `window` (32768) bytes and the
  table range of ABI sections 2.2 and 2.3.
- `exports`, in this order: `Apply`, `GetMapCell`, `GetMapCellAtPosition`, `GetOriginCellAtPosition`,
  `ClampWorldPosToTerrain`, `IsUnitVisibleToPlayer`, `SoundHeardMask`, `UpdateUnitLineOfSight`,
  `RecordRadarUnits`, `UpdateSensorRadarAndCloak`, `RecordRadarMapped`, `ExpireEyeballs`,
  `GetVisibleEnemiesInRadius`, `HasReadyUnitInRange`, `IsPointVisible`, `GetSeenUnitsInRadius`,
  `CreateUnitIndex`, `FreeUnitIndex`, `RefreshUnitIndex`, `RegionSize`, `Init`, `Shutdown`.
- `engine`: `LoadTntMap`, `InitRadar`, `FreeRadar`, `FreeMapResources`, `SightCache`, `WriteState`,
  `LoadMappingData` and `SaveMappingData`, each with the entry's reason.
- `data`: `g_game`, `g_simGlobals`, `g_extTdf`, `g_blockBase`, `g_region` (ABI 8.2: the four every module
  has, then `g_region` for a module whose slots take its region). The module reads `g_blockBase`, `g_game`
  for the `Game` fields of section 3, `g_simGlobals` for `SimGlobals.g_losTables` and
  `SimGlobals.g_losUnitIndex`, and `g_region`.
- `blocked`: the kit's entry is `"blocked": true`, so it has no `fuel_budget` and no `size_budget`, and the
  kit has no los_radar `.wasm` (ABI section 4.2). The gate judges a module of it without budgets (ABI
  section 15, "A blocked entry is judged without budgets"): its size is not judged, and it runs with the
  largest fuel budget of ABI section 11.3.
- Slot kinds (`signatures.los_radar`):

| Slot | Kinds |
| --- | --- |
| `Apply` | `[linear, int] -> int` |
| `GetMapCell` | `[int, int] -> block` |
| `GetMapCellAtPosition`, `GetOriginCellAtPosition` | `[linear] -> block` |
| `ClampWorldPosToTerrain` | `[int, int, linear] -> void` |
| `IsUnitVisibleToPlayer` | `[block, block] -> int` |
| `SoundHeardMask` | `[linear] -> int` |
| `UpdateUnitLineOfSight` | `[block] -> void` |
| `RecordRadarUnits`, `UpdateSensorRadarAndCloak`, `RecordRadarMapped`, `ExpireEyeballs` | `[] -> void` |
| `GetVisibleEnemiesInRadius` | `[int, linear, int, int, linear, int] -> int` |
| `HasReadyUnitInRange` | `[int, linear, int] -> int` |
| `IsPointVisible` | `[block, linear] -> int` |
| `GetSeenUnitsInRadius` | `[int, linear, int, linear, int] -> int` |
| `CreateUnitIndex`, `FreeUnitIndex`, `RefreshUnitIndex` | `[int] -> void` |
| `RegionSize` | `[linear] -> int` |
| `Init` | `[linear, block, int] -> void` |
| `Shutdown` | `[] -> block` |

- Imports, every one listed and nothing else (ABI 9.1: the module imports every listed function, even one
  it never calls), in the manifest's order:

| Import | Kinds | Used for |
| --- | --- | --- |
| `network.BroadcastPlayerInfo` | `[] -> void` | after a share toggle (SHARE_MAPPING, SHARE_RADAR) |
| `map.UnitsInRange` | `[linear, int, int, linear, int] -> int` | the units near a radar, sonar or jammer (`UpdateSensorRadarAndCloak`) |
| `map.GetGroundHeight` | `[linear] -> int` | the terrain height (`ClampWorldPosToTerrain`) |
| `services.SimAlloc` | `[int] -> linear` | the unit index and the storage of its lists |
| `services.SimFree` | `[linear] -> void` | the old storage of a list that grew, and the lists of a freed index |
| `services.RecordSimEvent` | `[int, int, linear, int] -> void` | the events of section 2, "Events" |
| `services.SimCaptureAlloc` | `[int, linear] -> linear` | the radar snapshots |
| `services.GetGafFrame` | `[linear, int] -> linear` | the sight masks (section 3, "The sight masks") |

- What each import gives, as the kit's headers say: `map.UnitsInRange`, given a position, a range, a skip, `ids` and a capacity,
  writes to `ids` (unsigned 16-bit unit slot numbers) up to `capacity` of the units within `range` of `pos`,
  after skipping the first `skip`, and returns how many it wrote (`interfaces/map.h`). `map.GetGroundHeight(pos)`
  returns the terrain height under `pos`, in whole world units; the recordings write its argument as
  `pos=(x,z)`, the two fields it reads. `services.SimCaptureAlloc(bytes, &offset)` returns zeroed bytes of
  the radar arena and writes their offset in the arena to `offset`, or returns null (0) when the arena is
  full. `services.SimAlloc(size)` returns zeroed block bytes (a size the block cannot hold ends the game).
  `services.GetGafFrame(table, i)` returns frame `i` of a frame table, or null outside it.
- Fuel: the module meters itself by rule version 1 (ABI section 11) and exports `__fuel_set` and
  `__fuel_get`.
- Hooks: every slot of the header says "Hooks: none".
- The gate: the entry is blocked, so the oracle takes its `recordings` route (ABI section 15): it replays
  the module on every recording of `tests/replay/los_radar/` through the generated glue
  (`docs/replay-format.md`, "Against the WASM module"), with the entry's `engine` slots the native ones
  there. Its result has `scope` `recordings`, `scenarios` `not_judged` and `budgets` `not_judged`, and
  `verdict` `equal` when every recording is equal. `services.GetGafFrame` is answered as a pure query
  (`docs/replay-format.md`, the paragraph on `--check` at its start): the module may call it in any order and
  any number of times, with the arguments the recordings have answers for.

### Arguments outside the block

All of these are in the caller's memory (`linear`), read or written in place, little-endian.

- **The event** of `Apply`: `size` bytes, a `LosRadarEventHeader` (`kind`, i32) and the payload of its
  kind, each a record of `interfaces/los_radar.h` with 4-byte fields in the order the header gives them:
  `LosRadarUnit` (`kind`, `unit`: 8 bytes), `LosRadarAddEyeball` (`kind`, `pos[3]`, `sightDistance`,
  `eyeHeight`, `lifetime`: 28), `LosRadarCellHeights` (`kind`, `cell[2]`, `size[2]`: 20),
  `LosRadarRecalculate` (`kind`, `reset`: 8), `LosRadarSetMode` (`kind`, `clear`, `set`: 12, both unsigned),
  `LosRadarSharePlayer` (`kind`, `player`: 8), `LosRadarShareMap` (`kind`, `from`, `to`: 12),
  `LosRadarSightLoad` (`kind`, `unit`, `cell`, `frameIdx`: 16). A `unit` is a block offset of a `Unit`
  (0 for none). The recordings write each field as `a event.<field>` (`event.pos0` to `event.pos2` for
  `pos`), and `a size 8`.
- **`LosRadarMapInfo`** (`RegionSize`, `Init`), 4 bytes: `unitSlots` (i32). The recordings write it as
  `a map (2501)`.
- **The region** (`Init`): the address of its first byte, and `size`, the value `RegionSize` gave.
- **A position** (`Vec3`, 12 bytes: `x`, `y`, `z`, i32 each, world units with 16 fraction bits) for
  `GetMapCellAtPosition`, `GetOriginCellAtPosition`, `SoundHeardMask`, `IsPointVisible` and the radius
  queries; `ClampWorldPosToTerrain`'s `out` is one the module writes whole. The recordings write a position as
  `(x,y,z)`.
- **`out`** of `GetVisibleEnemiesInRadius` and `GetSeenUnitsInRadius`: room for `capacity` `Ref<Unit>`
  (4 bytes each, a block offset); the recordings write the elements the call stores, `o out [Unit#252]`.
- **The buffers the module gives imports**: `ids` of `map.UnitsInRange` (256 × u16, written by the import,
  `o ids x:0100fc00`), `offset` of `services.SimCaptureAlloc` (a u32 the import writes), and `args` of
  `services.RecordSimEvent` (the u32 arguments, `[2,32]`).
- **A snapshot** returned by `services.SimCaptureAlloc`, which the module fills: the recordings write its
  bytes as `o snapshot#N x:...`.
- **A frame** returned by `services.GetGafFrame` (section 3, "The sight masks").

### Events

- `SIM_EVENT_MESSAGE`: by SHARE_MAPPING (message text 2) and SHARE_RADAR (text 4) in a network game.
- `SIM_EVENT_RADAR_UNITS`: by `RecordRadarUnits`, and so at the end of RECALCULATE.
- `SIM_EVENT_RADAR_MAPPED`: by `RecordRadarMapped`, and so at the end of RECALCULATE.

The arguments, flags, phases and the snapshot layouts are the kit's `docs/sim-events.md`; section 4 gives the
values the module puts in them.

## 3. State

### Owned

- **The region**: 5 bytes for each unit slot, `unitSlots` × 5 bytes, compared byte for byte at the end of
  every recorded call (the pseudo-field `region` of `docs/replay-format.md`, "A module's region and world
  view"). The bytes of slot *k* are at 5 *k*: the *sight cache* of the unit in slot *k*, a `LosSightCache`
  of `interfaces/los_radar.h`, packed with no padding:

| Offset | Field | Type | Meaning |
| --- | --- | --- | --- |
| 0 | *cache x* | i16 | the x of the cell the sight was last put at (the low 16 bits of `LosSightCache.cell`) |
| 2 | *cache y* | i16 | its y (the high 16 bits) |
| 4 | *frame* | u8 | the frame of that sight (section 4, "Sight") |

  The unit of slot *k* is the `Unit` at `Game.units` + 280 *k* (`layout.json`: `Unit` is 280 bytes). The
  region is zero when `Init` gets it.
- **`Game.eyes`**: the table of eyeballs, 20 `Eye` records of 36 bytes, the first `Game.count_14277` of them
  in use (`docs/replay-format.md` compares only those). The engine allocates the table; the module writes its
  records and `Game.count_14277` (i32, 0 to 20). An `Eye`'s fields: `player` (`Ref` to the `Player` the eye
  belongs to), `screen` (a `Ref` to its own `screenPos`), `x` (i16, the sight distance), `flagA` (u8, the eye
  height), `flagB` (the eye's *frame*: `char` in `layout.json`, read and written as u8), `flagPtr` (a `Ref` to its own `flagB`), `pos` (`Vec3`), `expires`
  (u32, a tick), `screenPos` (two i16: *cache x*, *cache y*). `screen` and `flagPtr` are `self` fields: never
  compared (`docs/replay-format.md`, "A pointer into the object itself"); the module writes them as the
  block offsets of the eye's own `screenPos` and `flagB` when it fills an eye.
- **`Game.visibilityMask`** (`Ref<unsigned short>`): its `Game.width` × `Game.height` / 4 u16 entries, one per
  cell of the half-size grid (`Game.width` / 2 × `Game.height` / 2). Bit *p* of an entry is set when player
  *p* has seen the cell. The engine allocates it.
- **`Player.explored`** of every player (`Ref<unsigned char>`, `Player.exploredSize` bytes, a grid of
  `exploredWidth` × `exploredHeight`): u8 sight counts, the number of sights of that player that cover the
  cell (or, with line of sight off, 1 where the map is explored). Counts wrap modulo 256.
- **`Game.mapFlags`** (u16): `Apply`'s SET_MODE writes bits 0 to 2; the module clears bit 3 (below).
- **`Game.flags_142f1`** (u16): bit 2 (value 4), the *map changed* flag. The module sets it and
  `RecordRadarMapped` clears it.
- **`Cell.low` and `Cell.high`** (u8) of the map cells, by CELL_HEIGHTS.
- **`Unit.flags`** bits 0x100, 0x200, 0x400 and 0x1000, and **`Unit.workTime`**, by
  `UpdateSensorRadarAndCloak`.
- **`PlayerInfo.flags_97_wide`** bits 0x20 (share mapping) and 0x40 (share radar), by SHARE_MAPPING and
  SHARE_RADAR.
- **`SimGlobals.g_losUnitIndex`** (`Ref<LosUnitIndex>`): the unit index, null until the first
  `CreateUnitIndex`, then a block allocation of 440 bytes (`layout.json` `LosUnitIndex`): 10
  `LosUnitIndexEntry` of 44 bytes, one per player: `live` (i32), `lastTick` (u32), `hasSpecial` (i32),
  `visible` and `known` (each a *list*, below).
- **A list** (`BlockVector<Ref<Unit>>`, 16 bytes; `docs/replay-format.md`, "A vector" names it a field of
  kind `vector`): byte 0 unused and 0, bytes 1 to 3 padding, then *first* (4), *last* (8) and *end* (12),
  each a block offset (`Ref`) into one block allocation of `Ref<Unit>` elements: the elements in use are
  *first* to *last*, the room *first* to *end*. A list with no storage has all three 0. The recordings write
  a list as `[length/capacity]` and its elements.

### The sight masks

`Game.losTable` is the address of a frame table (load data in the shared memory, ABI section 7.1, not a
block offset). The module reads `FrameTable.count` (u16, `layout.json`) there: the number of masks, *M*
(10 in every recording). Mask *i*, for 0 ≤ *i* < *M*, is the frame `services.GetGafFrame(Game.losTable, i)`
returns, a `GafFrame` (`layout.json`) of which the module reads:

| Field | Type | Meaning |
| --- | --- | --- |
| `width`, `height` | u16 | the mask's size in cells, *fw* × *fh* |
| `xOffset`, `yOffset` | i16 | the cell of the mask that is at the sight's centre |
| `transparency` | u8 | the pixel value that is not part of the sight |
| `pixelsOrLayers` | address | *fw* × *fh* bytes, row by row: pixel (*j*, *i*) is byte *fw* *i* + *j* |

A pixel other than `transparency` is a cell the sight covers. The recordings answer `GetGafFrame` with
`o GafFrame#1.width 19` and the other fields, then `o GafFrame#1.pixelsOrLayers[] [361] ...`. The masks come from that import and are
not state of the module: nothing about them may be kept between calls (ABI section 12). Where they are read
from is not final (ozgb/btm#41).

### Read

- `Game`: `width`, `height` (i32), `baseX`, `baseY` (i32, the map's size in world units), `seaLevel` (u8),
  `playerIndex` (u8, the viewing player), `numPlayers` (u16), `netGameFlags` (u8), `matchFlags` (u8),
  `ticks` (i32), `units`, `unitsEnd` (`Ref<Unit>`: the unit table and its last slot), `poolCount` (u16),
  `projectiles` (`Ref<Projectile>`, 107-byte records), `projectileCount` (i32), `losTable`, `cells`
  (`Ref<Cell>`, `Game.width` × `Game.height` cells of 13 bytes), `grid1` (`Grid`: `cells`, `width`,
  `height` (u32), `count`), `eyes`, `players[0..9]` (`Player`, 331 bytes each).
- `Grid` `Game.grid1`: the *height grid* of the half-size map, `grid1.width` × `grid1.height` cells of 2 bytes,
  cell (*x*, *y*) at `grid1.cells` + 2 (*y* `grid1.width` + *x*): byte 0 is the cell's ground level, byte 1 its
  top (`layout.json` `FogCell` `level0`, `level1`).
- `SimGlobals.g_losTables`: the true line of sight tables, a vector (`docs/replay-format.md`, "A vector in the
  sim globals") of *tables*; a table is a vector of *lines*, and a line a vector of *steps*, a `LosStep` of
  two u16, `a` and `b` (the step's x and y offsets from the origin, taken as signed 16-bit:
  `steps[0].b 65535` is −1). Each vector is a list as above, of 16-byte tables, 16-byte lines and 4-byte
  steps; its length is (*last* − *first*) / element size. The recordings write them as
  `r SimGlobals.g_losTables [9/9]`, `r SimGlobals.g_losTables[0].lines [8/8]`, and so on.
- `Unit`: `pos` (`Vec3`), `xWhole`, `yWhole`, `zWhole` (i16, the high 16 bits of `pos.x`, `pos.y`, `pos.z`),
  `def` (`Ref<UnitDef>`), `player` (`Ref<Player>`), `playerIndex` (u8), `unitDefIndex` (u16), `id` (u16),
  `flags` (u32), `activateFlags` (u8), `recentlyDamagedTimer` (u8), `buildLeft` (f32), `weapons[0..2]`
  (`UnitWeaponSlot`: `weapon`, `stockpile`).
- `UnitDef`: `range` (i16, the sight distance), `modelMaxYHigh` (i16; its low byte is the eye height),
  `modelMinX`, `modelMinZ`, `modelMaxY`, `modelSizeX`, `modelSizeY`, `modelSizeZ` (i32), `radardistance`,
  `sonardistance`, `radardistancejam`, `sonardistancejam`, `mincloakdistance` (i16), `flags1`, `flags2`
  (u32), `flags2Byte` (u8, the low byte of `flags2`).
- `WeaponDef`: `flags` (bits `noRadar` 6, `targetable` 29, `interceptor` 30), `coverage` (i32).
- `Projectile`: `type` (`Ref<WeaponDef>`), `positionWords` (`xWhole`, `yWhole`, `zWhole`, as a unit's),
  `attacker` (`Ref<Unit>`), `owner` (u8).
- `Player`: `active` (i32), `info` (`Ref<PlayerInfo>`), `unitsBegin`, `unitsEnd` (`Ref<Unit>`, the player's
  first and last unit slot), `type` (u8), `explored`, `exploredWidth`, `exploredHeight`, `exploredSize` (i32),
  `allied[0..10]` (u8), `index` (u8).
- `PlayerInfo`: `color` (u8), `flags_97` (u8, the low byte of `flags_97_wide`), `flags_97_wide` (u16),
  `flags_9b` (u8, the low byte of `flags`).
- `FrameTable.count` and the masks (above).

### Bits of the flags words

- `Game.mapFlags`: bit 0 *mapping* (map flag 1 of `docs/sim-events.md`, Terms), bit 1 *line of sight on*
  (map flag 2), bit 2 *true type* (set: true line of sight; clear: the circle type), bit 3 a request to
  redraw the fog, which the module clears whenever the viewing player's sight changes.
- `Game.matchFlags` bit 1: match flag 2 (Terms). `matchFlags` is the high byte of the u16
  `Game.uiOptionFlags`, so this is bit 9 of `uiOptionFlags`; the recordings read the u16 (`r Game.uiOptionFlags
  1038`, bit 9 clear).
- `Game.netGameFlags` bit 0: net game flag 1 (Terms).
- `Unit.flags`: 0x10000000 alive and 0x4000 dying (Terms); 0x100 shown on radar, 0x200 shown on sonar, 0x300
  the pair (Terms, "unit flags 0x300"); 0x400 jammed; 0x1000 near an enemy that decloaks it; 0x8000 a unit the
  index leaves out of the seen list. A *ready* unit has 0x10000000 set and 0x4000 clear.
- `Unit.activateFlags`: bit 0 switched on, bit 2 (value 4) cloaked (Terms, activation bits 0 and 2).
- `UnitDef.flags1`: bit 8 (0x100) *stealth* (radar and sonar never show the unit), bit 10 (0x400) a type whose
  finished, switched-on unit lets the index fall back on the `known` list, bit 29 (0x20000000) *has
  interceptors*. `UnitDef.flags2`: bit 2 (0x4) can be switched on and off (Terms), bit 13 (0x2000) *cloaks*.
- `PlayerInfo.flags_97_wide`: bit 5 (0x20) shares mapping, bit 6 (0x40) shares radar.
  `PlayerInfo.flags` bit 6 (0x40, of `flags_9b`): info flag 0x40, watch mode (Terms).

### Between ticks

The region holds each unit slot's sight cache; `Game.eyes` and `Game.count_14277` the live eyeballs; the
explored maps hold the sight counts of the sights the caches and eyes say are on the map, and the visibility
mask what has been seen. The unit index is in the block. The module keeps nothing else.

## 4. Behaviour

Within one call, only the order of import calls is observable. The block writes are compared at the end of
the call, so the order they are listed in carries no meaning.

A byte this section does not say a call writes keeps its value.

### Definitions

**Slot ranges.** A run of unit slots "from *a* to the slot of `unitsEnd`" (of `Game` or of a player) includes
that last slot.

**Whole parts.** For a position *P*, *P*.x̂, *P*.ŷ, *P*.ẑ are the high 16 bits of `x`, `y`, `z` as signed
16-bit numbers (a unit's are `Unit.xWhole`, `yWhole`, `zWhole`).

**The visibility tests**, for a player *L* and a position *P*: *tx* = *P*.x̂ >> 5 and *ty* = (*P*.ẑ −
(*P*.ŷ >> 1)) >> 5 (arithmetic shifts). *P* is *on L's grid* when 0 ≤ *tx* < *L*.`exploredWidth` and 0 ≤
*ty* < *L*.`exploredHeight`, and its entry is *e* = *L*.`exploredWidth` × *ty* + *tx*. Then:

- *explored*(*L*, *P*): on *L*'s grid and *L*.`explored`[*e*] ≠ 0;
- *seen*(*L*, *P*): on *L*'s grid and bit `Game.playerIndex` of `Game.visibilityMask`[*e*] is set (the
  viewing player's bit, whoever *L* is: Q8);
- *visible*(*L*, *P*): *explored*(*L*, *P*) when `Game.mapFlags` bit 1 is set, else *seen*(*L*, *P*).

**A sight** *S* is what the module puts on and takes off the map. Its parts: a player *S*.pl (a `Player`), a
cache (*cache x*, *cache y*, *frame*), a sight distance *S*.d (i16), an eye height *S*.h (u8) and a position
*S*.P.

- *The sight of a unit* *u* in slot *k*: *S*.pl = *u*.`player`, the cache is region slot *k*, *S*.d =
  *u*.`def`.`range`, *S*.h = the low byte of *u*.`def`.`modelMaxYHigh`, *S*.P = *u*.`pos` with `y` raised to
  (`Game.seaLevel` + 1) × 65536 when it is below that.
- *The sight of an eye* *E*: *S*.pl = *E*.`player`, the cache is *E*.`screenPos` (x, y) and *E*.`flagB`,
  *S*.d = *E*.`x`, *S*.h = *E*.`flagA`, *S*.P = *E*.`pos`.

**The mask of a sight**, for the circle type: *i*(*S*) = *S*.d / 32 − 5 (division truncating toward zero),
then 0 when below 0, and *M* − 1 when *M* − 1 or above.

**The circle cell** of a sight, *cell*(*S*) = (*cx*, *cy*) with *F* = mask *i*(*S*):
*cx* = *S*.P.x / 2^21 − *F*.`xOffset`, *cy* = *S*.P.z / 2^21 − *S*.P.ŷ / 64 − *F*.`yOffset`, each division
truncating toward zero.

**The true table** of a sight: *k* = *S*.d / 32 (truncating toward zero), 0 when below 0; *n* = the number
of tables, as a signed 16-bit number; when *k* ≥ *n* − 1, *k* = *n* − 1. The table is
`g_losTables`[*k* − 1] (Q2).

**The true cells** of a sight with origin (*x*, *y*) = (*cache x*, *cache y*) and reference *r* = *frame*,
on the height grid (*W* = `grid1.width`, *H* = `grid1.height`): a list, in this order. For each line of the
true table, in order: *best* = 0 and *gap* = −1; for each step (*a*, *b*) of the line, *j* = 1 for its
first step, 2 for the next and so on (counting every step, skipped ones too): *dx* = the low 16 bits of *a* +
*x* as a signed number, *dy* likewise of *b* + *y*. The step is skipped unless 0 ≤ *dx* < *W* and 0 ≤ *dy* <
*H*. Otherwise, with *c*0 and *c*1 the two bytes of height grid cell (*dx*, *dy*), *d*0 = *c*0 − *r* and
*d*1 = *c*1 − *r*: when *d*0 × *best* > *gap* × *j*, (*dx*, *dy*) is in the list, and then, when also
*d*1 × *best* > *gap* × *j*, *best* = *j* and *gap* = *d*1. A cell can be in the list more than
once (once per line that reaches it).

The *half size* is *hw* = `Game.width` / 2, *hh* = `Game.height` / 2. Let *pl* = *S*.pl, its explored map
*X* with width *XW* = *pl*.`exploredWidth`, and *bit* = 1 << *pl*.`index`.

**Add a sight** (+1) and **take a sight off** (−1), with *δ* the sign:

1. When *pl*.`index` = `Game.playerIndex`: `Game.mapFlags` bit 3 becomes 0 and `Game.flags_142f1` bit 2
   becomes 1. This happens first, whatever follows.
2. True type (`Game.mapFlags` bit 2 set): nothing more unless 0 ≤ *cache x* < *W* and 0 ≤ *cache y* < *H*.
   Then *X*[*cache y* × *XW* + *cache x*] += *δ*, and for each cell (*dx*, *dy*) of the true cells,
   *X*[*dy* × *XW* + *dx*] += *δ*.
3. Circle type: *F* = mask *frame* (the cached frame). For each pixel (*j*, *i*) of *F* that is not
   `transparency` and whose cell (*cache x* + *j*, *cache y* + *i*) has 0 ≤ *cache x* + *j* < *hw* and
   0 ≤ *cache y* + *i* < *hh*: *X*[(*cache y* + *i*) × *XW* + *cache x* + *j*] += *δ*.

Each += wraps modulo 256.

**Reveal around a sight**: *changed* starts false.

1. True type: when 0 ≤ *cache x* < *W* and 0 ≤ *cache y* < *H*, *bit* is set in `Game.visibilityMask`[*hw* ×
   *cache y* + *cache x*] and in `Game.visibilityMask`[*hw* × *dy* + *dx*] for every cell of the true cells.
2. Circle type: *F* = mask *i*(*S*) (from the sight distance, not the cached frame: Q7). For each pixel
   (*j*, *i*) of *F* that is not `transparency` with 0 ≤ *cache x* + *j* < *hw* and 0 ≤ *cache y* + *i* <
   *hh*: *bit* is set in `Game.visibilityMask`[(*cache y* + *i*) × *hw* + *cache x* + *j*].
3. *changed* is true when a bit that was clear was set. When *changed* and *pl*.`index` =
   `Game.playerIndex`: `Game.mapFlags` bit 3 becomes 0 and `Game.flags_142f1` bit 2 becomes 1.

**Move a sight** (a sight goes where its unit is now). Each cache comparison takes *cache x* and *cache y* as
signed 16-bit values against the whole *x*, *y* (or *cx*, *cy*), and *frame* as u8 against *v* (or *i*).

- True type: *x* = *S*.P.x̂ >> 5; *v* = *S*.h + *S*.P.ŷ, clamped to 0..255; *y* = (*S*.P.ẑ − (*v* >> 1))
  >> 5. Nothing happens when *cache x* = *x*, *cache y* = *y* and |*frame* − *v*| ≤ 5. Otherwise, in order:
  1. when *frame* ≠ 0 and `Game.mapFlags` bit 1 is set, the sight is taken off (with its old cache);
  2. *cache x* = *x*, *cache y* = *y* (their low 16 bits);
  3. when *x* or *y* is outside 0 ≤ *x* < *W*, 0 ≤ *y* < *H*: *frame* = 0, and nothing more;
  4. else *frame* = *v*; the sight is added when `Game.mapFlags` bit 1 is set, and revealed when bit 0 is set.
- Circle type: (*cx*, *cy*) = *cell*(*S*) and *i* = *i*(*S*). Nothing happens when *cache x* = *cx*, *cache
  y* = *cy* and *frame* = *i*. Otherwise: with `Game.mapFlags` bit 1 set, the sight is taken off (old cache),
  the cache becomes (*cx*, *cy*, *i*) and the sight is added; with it clear, the cache becomes (*cx*, *cy*,
  *i*) and nothing is added or taken off. Then, when bit 0 is set, the sight is revealed.

**Place a sight** (a sight's first placing): nothing when `Game.mapFlags` bit 1 is clear. Else *frame* = 0;
then, true type: the sight is moved; circle type: the cache becomes (*cx*, *cy*, *i*(*S*)) with (*cx*, *cy*) =
*cell*(*S*), the sight is added and revealed (whatever bit 0 is).

### `Apply(event, size)`

`size` below 4, or below the payload size of its kind (section 2: 8 for UPDATE_UNIT, REMOVE_UNIT,
REVEAL_UNIT, SIGHT_RESET, RECALCULATE, SHARE_MAPPING and SHARE_RADAR; 28 ADD_EYEBALL; 20 CELL_HEIGHTS; 12
SHARE_MAP and SET_MODE; 16 SIGHT_LOAD; 4 for any other kind): outcome 0, nothing else. A kind not in
`LosRadarEventKind`: outcome 0, nothing else.

**The limits** (ozgb/btm#25): an event outside them changes nothing, writes nothing, does not trap and has
outcome 0.

| Kind | Its limits |
| --- | --- |
| UPDATE_UNIT, REMOVE_UNIT, REVEAL_UNIT | `unit` is a unit: not 0, `Game.units` + 280 *k* for a slot 0 ≤ *k* < `unitSlots` (of `Init`'s map), and that unit's `def` and `player` are not 0 |
| SIGHT_RESET, SIGHT_LOAD | `unit` is a unit slot as above (its `def` is not read) |
| SHARE_MAPPING, SHARE_RADAR | 0 ≤ `player` ≤ 9 |
| SHARE_MAP | 0 ≤ `from` ≤ 9 and 0 ≤ `to` ≤ 9 |
| SET_MODE | (`clear` OR `set`) has no bit above bit 2 |
| ADD_EYEBALL, CELL_HEIGHTS, RECALCULATE | none: their numbers are clamped or used as they are |

The recordings hold only events inside the limits; a null `unit` (outcome 0) is the case the reference shows
too.

| Kind | Effects | Outcome |
| --- | --- | --- |
| UPDATE_UNIT (128) | the sight of the unit is moved | 1 |
| REMOVE_UNIT (129) | the sight of the unit is taken off, whatever `Game.mapFlags` is (Q5) | 1 |
| REVEAL_UNIT (130) | the sight of the unit is placed | 1 |
| ADD_EYEBALL (131), `Game.mapFlags` bit 1 set and `Game.count_14277` < 20 | eye *E* = `Game.eyes`[`count_14277`] is filled (below), its sight is placed, and `count_14277` + 1 | 1 |
| ADD_EYEBALL, otherwise | nothing | 0 |
| CELL_HEIGHTS (132) | below | 1 |
| RECALCULATE (133) | below | 1 |
| SHARE_MAPPING (134), `Game.netGameFlags` bit 0 set | below | 1 |
| SHARE_RADAR (135), `Game.netGameFlags` bit 0 set | below | 1 |
| SHARE_MAPPING, SHARE_RADAR, otherwise | nothing | 0 |
| SHARE_MAP (136) | for each entry *i*, 0 ≤ *i* < `Game.width` × `Game.height` / 4: when it has bit `from`, bit `to` is set | 1 |
| SIGHT_RESET (137) | the 5 bytes of the unit's slot become 0 | 1 |
| SIGHT_LOAD (138) | the slot's *cache x*, *cache y* become the low and high 16 bits of `cell`, and *frame* the low 8 bits of `frameIdx` | 1 |
| SET_MODE (139) | `Game.mapFlags` = (`mapFlags` AND NOT `clear`) OR `set`, its low 16 bits | 1 |

**ADD_EYEBALL's eye**: `player` = `Game.players`[`Game.playerIndex`] (Q4), `x` = the low 16 bits of
`sightDistance`, `flagA` = the low 8 bits of `eyeHeight`, `pos` = `pos[0..2]` with `y` raised to
(`seaLevel` + 1) × 65536 when below, `expires` = `Game.ticks` + `lifetime` (unsigned, wrapping), `screen` and
`flagPtr` its own members; `screenPos` keeps what that record held (Q3). Then the eye's sight is placed: the
true type moves it with the old `screenPos` as its cache and *frame* 0; the circle type sets `screenPos` and
`flagB` from it.

**CELL_HEIGHTS**: *px*, *py*, *sx*, *sy* = the low 16 bits of `cell[0]`, `cell[1]`, `size[0]`, `size[1]` as
signed numbers. *xmax* = *px* + *sx*, *ymax* = *py* + *sy*; *px* and *py* become 0 when below 0; *xmax* becomes
`Game.width` − 1 when it is `Game.width` or above, *ymax* likewise with `Game.height`. When *px* ≥ *xmax* or
*py* ≥ *ymax*, nothing. Otherwise, for each cell (*col*, *row*), *px* ≤ *col* < *xmax*, *py* ≤ *row* < *ymax*
(Q6): `low` and `high` become the least and the greatest `Cell.height` of the cell, the cell to its right
when *col* < `width` − 1, the cell below when *row* < `height` − 1, and the cell below and right when both.

**RECALCULATE(reset)**, the imports in this order: those of `RecordRadarMapped`, then those of
`RecordRadarUnits`, with `GetGafFrame` calls anywhere before them.

1. When `reset` ≠ 0, every byte of the `Game.width` × `Game.height` × 2 / 4 bytes of `Game.visibilityMask`
   becomes 0 when `Game.mapFlags` bit 0 is set, else 0xFF.
2. For each player *p*, 0 to 9, with `active` ≠ 0, `type` 1, 2 or 3 and `index` ≠ 10: the `exploredSize`
   bytes of its `explored` become 0 when `mapFlags` bit 1 is set, else 1.
3. For each unit slot from 1 to the slot of `Game.unitsEnd`, whose unit has `unitDefIndex` ≠ 0: its sight is
   placed.
4. `Game.flags_142f1` bit 2 becomes 1 and `Game.mapFlags` bit 3 becomes 0.
5. `RecordRadarMapped`, then `RecordRadarUnits` (their sections).

**SHARE_MAPPING(p)** and **SHARE_RADAR(p)**, with *I* = `Game.players`[*p*].`info`: `I.flags_97_wide` bit *m*
is inverted (*m* = 0x20 for mapping, 0x40 for radar); then `services.RecordSimEvent(33, 0, [t, v], 2)`
with *t* = 2 for mapping, 4 for radar, and *v* = `I.flags_97_wide` AND *m* after the change; then
`network.BroadcastPlayerInfo()`.

### `GetMapCell(x, y)`

The cell (*x*, *y*), `Game.cells` + 13 (*y* × `Game.width` + *x*), when 0 ≤ *x* < `Game.width` and 0 ≤ *y* <
`Game.height`; else null (0). It writes nothing.

### `GetMapCellAtPosition(pos)`

As `GetMapCell` with *x* = `pos.x` >> 20 and *y* = `pos.z` >> 20 (arithmetic shifts). `pos.y` is not read.

### `GetOriginCellAtPosition(pos)`

*x* = `pos.x` >> 20, *z* = `pos.z` >> 20. Outside the map (as `GetMapCell`): null. When the cell's `feature`
is 0xfffe (Terms, feature number 0xfffb): *x* −= the cell's `offsetX`, *z* −= its `offsetY` (u8 each), and
the result is the cell (*x*, *z*), or null when that is outside the map. Otherwise the cell itself.

### `ClampWorldPosToTerrain(x, y, out)`

*x* is clamped to 0..`Game.baseX` − 1 and *y* to 0..`Game.baseY` − 1. A *height*(*Q*) of a position *Q* is
`map.GetGroundHeight(Q)` when that is above `Game.seaLevel`, else `seaLevel` (Q1 for the imports it makes),
times 65536. The *screen row* of *Q* is *Q*.ẑ − (*Q*.ŷ >> 1).

*p*.x = *x* × 65536, *t* = *y* AND NOT 15. For *i* = 128, 112, ..., 16, 0: *p*.z = (*t* + *i*) × 65536,
*p*.y = *height*(*p*), *s*1 = the screen row of *p*; the search stops at the first *i* with *s*1 ≤ *y*.

| Outcome | Then | `out` |
| --- | --- | --- |
| no *i* has *s*1 ≤ *y* | nothing | *p* of *i* = 0 |
| an *i* is found | *q* = *p* with *q*.z = *p*.z + 0x100000, *q*.y = *height*(*q*), *s*2 its screen row; when *s*1 < *s*2 or *y* ≤ *s*2: *p*.z += ((*y* − *s*1) × 2^20) / (*s*2 − *s*1) (truncating toward zero), *p*.y = *height*(*p*); otherwise *p* is left as found | *p* |

`out` gets all 12 bytes. The imports are the *height* calls, in the order above.

### `IsUnitVisibleToPlayer(player, u)`

1 when *u*.`player` is `player`. 0 when *u*.`activateFlags` bit 2 is set. Otherwise, with *D* = *u*.`def`:
*Q* = (*D*.`modelMinX` + *u*.`pos.x`, *D*.`modelMaxY` + *u*.`pos.y`, *D*.`modelMinZ` + *u*.`pos.z`); 0 when
*u*.`flags` has no 0x200 and *Q*.y < `seaLevel` × 65536. Then 1 at the first of these that is *visible*(`player`,
*Q*): *Q*; *Q* with x += *D*.`modelSizeX`; then also z += *D*.`modelSizeZ` and y −= *D*.`modelSizeY`; then x −=
*D*.`modelSizeX`. Else 0.

### `SoundHeardMask(at)`

0 when `GetMapCell`(`at.x` / 2^20, `at.z` / 2^20) is null, the divisions truncating toward zero (Q10).
Otherwise *tx* = `at`.x̂ >> 5, *ty* = (`at`.ẑ − (`at`.ŷ >> 1)) >> 5, and bit *p* (0 to 9) of the result is set
when, for player *p*: 0 ≤ *tx* < `exploredWidth`, 0 ≤ *ty* < `exploredHeight`, and with *e* = `exploredWidth` ×
*ty* + *tx*: `explored`[*e*] ≠ 0 when `mapFlags` bit 1 is set, else `Game.visibilityMask`[*e*] has bit *p*.

### `UpdateUnitLineOfSight(unit)`

The sight of the unit is moved. The caller passes a unit whose `def` and `player` are not 0; other units are
outside the contract.

### `RecordRadarUnits()`

*enabled* = `Game.mapFlags` bits 0 and 1 both clear, or `Game.matchFlags` bit 1 set. A unit is *shown* when
its `unitDefIndex` ≠ 0 and at least one of these holds: *enabled*; its `flags` has a bit of 0x300; its
`playerIndex` = `Game.playerIndex`. The units are those of every slot from 0 (Q12) to the slot of `Game.unitsEnd`, in order.
A projectile *j*, 0 ≤ *j* < `Game.projectileCount`, at (*px*, *py*) = (x̂, ẑ − (ŷ >> 1)) of its
`positionWords`, is *on radar* when *explored*-or-*seen* holds there for the viewing player *V* =
`Game.players`[`playerIndex`] (with *tx* = *px* >> 5, *ty* = *py* >> 5 as in the visibility tests:
*explored*(*V*) when `mapFlags` bit 1 is set, else *seen*(*V*)). It is *shown*:

| Its weapon's `flags` | Shown when | Kind | Colour |
| --- | --- | --- | --- |
| bits 29 and 30 clear | bit 6 clear, and on radar or `owner` = `playerIndex` | 1 | 0 |
| bit 29 or 30 set | on radar, or its `attacker`'s `playerIndex` = `playerIndex` | 2 | `Game.players`[`owner`].`info`.`color` |

The imports, in order: `services.SimCaptureAlloc(8 + 48 U + 12 J, &offset)`, with *U* and *J* the shown
units and projectiles; when it returns null, nothing more. Otherwise the snapshot is written (the layout of
`docs/sim-events.md`, SIM_EVENT_RADAR_UNITS): *U*, *J*; a record per shown unit, in slot order: slot, `id`,
`xWhole`, the screen row `zWhole` − (`yWhole` >> 1), `Game.players`[`playerIndex` of the unit].`info`.`color`, flags
(1 when `recentlyDamagedTimer` ≠ 0; 2 when `activateFlags` bit 0 is set or `def.flags2Byte` bit 2 is clear; 4
when `def.flags1` bit 29 is set), the def's `radardistance`, `sonardistance`, `radardistancejam`,
`sonardistancejam`, and, only when bit 29 is set, for each weapon slot *w* whose `weapon` is not null (any weapon, `weapondef#0` included): coverage =
the weapon's `coverage`, interceptor = its `flags` bit 30, stocked = 1 when the slot's `stockpile` ≠ 0; a
record per shown projectile, in order: *px*, *py*, kind, colour. The reserved bytes and the weapons the record
leaves out stay 0. Then `services.RecordSimEvent(34, 0, [offset], 1)`.

### `UpdateSensorRadarAndCloak()`

Nothing when `Game.numPlayers` < 2. Otherwise, with *v* = `Game.playerIndex` and *pl* = `Game.players`[*v*],
five passes in order. "The units" are slots 1 to the slot of `Game.unitsEnd`; *alive* is flag 0x10000000.

1. Each alive unit *a*: 0x1000 is cleared; then 0x300 is set when *a*.`playerIndex` = *v*, or *a*.`player`.`allied`[*pl*.`index`] ≠ 0 and *a*.`player`.`info`.`flags_97` bit 6 is set, or *pl*.`active` ≠ 0 and *pl*.`info`.`flags_9b` bit 6 is set; otherwise 0x700 is cleared.
2. Each unit *s* from *pl*.`unitsBegin` to *pl*.`unitsEnd` that is ready, switched on (`activateFlags` bit 0),
   and whose def has *R* = `radardistance` or *N* = `sonardistance` ≠ 0: *rr* = (*R* + 2 × *s*.`yWhole`)² and
   *ss* = *N*², 32-bit; the range is max(*R*, *N*) × 65536. The units in range (below) of *s*.`pos` each get,
   unless its `unitDefIndex` is 0, its `playerIndex` is *v*, or its def's `flags1` bit 8 is set, with *d* =
   the distance (below) from *s*: 0x200 when its `pos.y` ≤ `seaLevel` × 65536 and *d* < *ss*; 0x100 when its
   def's `modelMaxY` + its `pos.y` ≥ `seaLevel` × 65536 and *d* < *rr*.
3. Each alive unit *j* of the units with *j*.`playerIndex` ≠ *pl*.`index` and switched on: when its def's
   `radardistancejam` ≠ 0, each unit in range of *j*.`pos` within that × 65536 has 0x100 cleared and 0x400
   set; then, when `sonardistancejam` ≠ 0, each in range within that × 65536 has 0x200 cleared and 0x400 set
   (every unit the import gives, the jammer and its own side's included: Q11).
4. Each alive unit *c* whose player has `active` ≠ 0 and `type` 1 or 2, and whose def's `flags2` bit 13 is
   set: when a ready unit of entry *c*.`playerIndex`'s `visible` has the distance from *c*.`pos` ≤
   def.`mincloakdistance` squared (32-bit), `workTime` = `Game.ticks` + 90 and 0x1000 is set.
5. Each alive unit with no 0x100 and `activateFlags` bit 2 clear: 0x100 is set when *visible*(*pl*, its `pos`).

**The units in range** of a position *P* within *r*: `map.UnitsInRange(P, r, skip, ids, 256)` with *skip* = 0,
then again with *skip* increased by the count while the count is 256 or more. Each id *n* of each answer is
the unit of slot *n*, taken in order.

**The distance** from a position *A* to a unit *u*: *dx* = *u*.`pos.x` − *A*.x, *dz* = *u*.`pos.z` − *A*.z
(32-bit, wrapping), *d* = ⌊*dx*² / 2^32⌋ + ⌊*dz*² / 2^32⌋, each square a 64-bit product, each quotient's low
32 bits, the sum 32-bit.

### `RecordRadarMapped()`

Nothing when `Game.flags_142f1` bit 2 is clear. Otherwise the bit becomes 0 (whether or not a snapshot follows:
Q9), and with *C* = *hw* × *hh*: `services.SimCaptureAlloc(12 + C, &offset)`. Null: nothing more. Otherwise
the snapshot (`docs/sim-events.md`, SIM_EVENT_RADAR_MAPPED): `Game.width`, `Game.height`, *C*, then for each *i*
< *C*: 0 when `Game.visibilityMask`[*i*] has no bit `playerIndex`, else 1 when the viewing player's
`explored`[*i*] ≠ 0, else 2. Then `services.RecordSimEvent(35, 0, [offset], 1)`.

### `ExpireEyeballs()`

Each eye *E* of the first `count_14277`, in order, with `expires` < `Game.ticks` (unsigned) has its sight taken
off (whatever `mapFlags` is: Q5). Then the eyes that are left keep their order and move to the front of the
table, each record copied whole with `screen` and `flagPtr` set to its new place's own fields, and `count_14277` becomes their number. The records past the new count keep
what they held.

### The unit index

An entry of player *p* is `g_losUnitIndex`.`players`[*p*]. *To push* *u* onto a list: when *last* = *end*,
with *s* the length: `services.SimAlloc(4 × n)` with *n* = *s* + max(*s*, 1), the *s* elements are copied to
the new storage, *u* after them, then, when the old *first* is not 0, `services.SimFree(old first)`; *first*,
*last* = *first* + 4(*s* + 1), *end* = *first* + 4 *n*. Otherwise *u* goes at *last* and *last* += 4.
*To clear* a list: *last* = *first* (the storage is kept).

**`CreateUnitIndex(p)`**: when `g_losUnitIndex` is 0, `services.SimAlloc(440)` and `g_losUnitIndex` = its
block offset (every entry then zero). Entry *p*: both lists cleared, `hasSpecial` = 0, `lastTick` = 0, `live`
= 1.

**`FreeUnitIndex(p)`**: nothing when `g_losUnitIndex` is 0. Else, for `visible` then `known` of entry *p*:
`services.SimFree(first)` when *first* is not 0, and the three words become 0; then `live` = 0.

**`RefreshUnitIndex(p)`** (the caller passes 0 ≤ *p* ≤ 9; the radius queries' `player` likewise): nothing when `g_losUnitIndex` is 0, entry *p*'s `live` is 0, or `Game.ticks` <
`lastTick` + 30 (unsigned). Else both lists are cleared and `hasSpecial` = 0; with *o* =
`Game.players`[*p*], for each ready unit *u* of the units (slots 1 to that of `unitsEnd`), in order:

- when *o*.`allied`[*u*.`player`.`index`] = 0: *u* is pushed onto `visible` when
  `IsUnitVisibleToPlayer`(*o*, *u*) is 1 and *u*.`flags` has no 0x8000; then *u* is pushed onto `known` when
  *u*.`flags` has 0x100;
- otherwise, when *u*.`player`.`index` = *o*.`index`, *u*.`buildLeft` = 0.0, *u*.`def`.`flags1` bit 10 is set
  and *u*.`activateFlags` bit 0 is set: `hasSpecial` = 1.

Then `lastTick` = `Game.ticks`.

**`GetVisibleEnemiesInRadius`**, its arguments `player`, `pos`, `radius`, `flags`, `out` and `capacity` in that order: *r*2 = `radius` × `radius`
(32-bit). Count *c* = 0; each unit of entry `player`'s `visible`, in order, with the distance from `pos` ≤ *r*2
and ready: when *c* < `capacity`, `out`[*c*] = its block offset; *c* += 1. Then, when `hasSpecial` ≠ 0 and *c*
= 0, the same over `known`. The result is *c*; `flags` is not read. The rest of `out` is not written.

**`HasReadyUnitInRange(player, pos, range)`**: 1 when a unit of entry `player`'s `visible` has the distance
from `pos` ≤ `range` × `range` (32-bit) and is ready; else 0.

**`GetSeenUnitsInRadius`** (`player`, `pos`, `radius`, `out`, `capacity`): as `GetVisibleEnemiesInRadius` over `visible`
alone, without the ready test and without the `known` fallback.

When `g_losUnitIndex` is 0, the three return 0 and write nothing (no recording reaches it).

### `IsPointVisible(player, pos)`

*seen*(`player`, `pos`) as 1 or 0.

### `RegionSize(map)`, `Init(map, region, size)`, `Shutdown()`

`RegionSize` returns `map.unitSlots` × 5. `Init` keeps the region and writes nothing in it (it is zero).
`Shutdown` returns the region and forgets it.

### The engine slots

The native slots run on the module's state, through the region and the block:

- **`SightCache(unit, out)`** writes the 5 bytes of the unit's slot of the region to `out`.
- **`WriteState(writer)`** reads the region (`unitSlots` = `Game.poolCount` slots), the eyes and the unit
  index, and writes the records of the kit's `docs/trace-format.md`: `losRadarState` (record 44) and a
  `losRadarIndex[i]` (record 24) for each live entry. It writes nothing.
- **`LoadMappingData`** and **`SaveMappingData`** read and write the `Game.width` × `Game.height` × 2 / 4 bytes
  of `Game.visibilityMask` in a saved game.
- **`LoadTntMap`**, **`InitRadar`**, **`FreeRadar`**, **`FreeMapResources`** load and free the map, the height
  grid, the visibility mask, the eye table and the radar picture, outside the tick. None touches the region.

## 5. Exactness

- **Integers.** 32-bit two's complement unless section 3 gives a field's width. Sums and products wrap at 32
  bits. Signed division truncates toward zero; ">>" of a signed number is an arithmetic shift. Narrow fields
  are sign- or zero-extended by their type when read and keep their low bits when stored.
- **Comparisons** against a grid's size (`grid1.width`, `grid1.height`, `exploredWidth`, `exploredHeight`) are
  unsigned: a negative coordinate is outside. Tick comparisons (`expires` < `ticks`, `ticks` < `lastTick` +
  30) are unsigned. The others are signed.
- **The line walk** products *d*0 × *best*, *gap* × *j* are 32-bit signed; *c*0, *c*1 and *r* are 0 to
  255.
- **Sight counts** in `explored` are u8 and wrap.
- **The squares** of the radius queries and of the sensors are 32-bit; the distance is the 64-bit form of
  section 4.
- **`Unit.buildLeft`** is compared with 0.0 as a 32-bit float (−0.0 equals it).
- No random draws.

**Edges.**
- `ClampWorldPosToTerrain` divides by *s*2 − *s*1, which is 0 only when *y* = *s*1 = *s*2; no recording
  reaches it, and the case is outside the contract.
- A circle mask index or a cached *frame* at or above *M* has no mask (`GetGafFrame` gives null); no recording
  reaches it, and the module then adds and takes off nothing.

**Gaps in the kit.**
- The kit's los_radar entry is blocked, so it has no budgets, and the kit has no los_radar `.wasm`; the
  module's fuel and size are judged once the entry has budgets (section 2).
- The sight masks come from an import, not the block (section 3).

## 6. Quirks

- **Q1.** `ClampWorldPosToTerrain`'s *height* calls `map.GetGroundHeight` twice with the same position when
  the first answer is above `Game.seaLevel`, and uses the second answer; once otherwise.
- **Q2.** The true table of a sight is the one before the index its sight distance gives: `g_losTables`[*k* −
  1]. A sight distance below 32 gives *k* = 0, which names no table; no recording reaches it, and the module
  then uses a table with no lines (only the origin cell counts).
- **Q3.** An eyeball of the true type is moved with the `screenPos` the record held before (from an older eye,
  or 0 in a new table) and *frame* 0: when that old cell is the new one and *v* ≤ 5, the eye adds no sight and
  keeps *frame* 0.
- **Q4.** An eyeball belongs to the viewing player, whoever published it.
- **Q5.** REMOVE_UNIT and an expiring eyeball take their sight off even when line of sight is off.
- **Q6.** CELL_HEIGHTS never reaches the last column or the last row of the map: *xmax* and *ymax* are clamped
  to `width` − 1 and `height` − 1 and are exclusive.
- **Q7.** The circle type reveals with the mask of the sight distance, and adds and takes off with the cached
  *frame*; they differ only when a SIGHT_LOAD set another frame.
- **Q8.** *seen* and `IsPointVisible` test the viewing player's bit of the mask on the given player's grid.
  Reveal indexes the mask with *hw*, and the sights index `explored` with `exploredWidth`.
- **Q9.** `RecordRadarMapped` clears the map changed flag even when the arena is full and no snapshot is taken.
- **Q10.** `SoundHeardMask`'s map test divides with truncation toward zero, so a coordinate just below 0 passes
  it; its hearing test then uses arithmetic shifts.
- **Q11.** Jamming acts on every unit in range, the jammer's own and its allies' included.
- **Q12.** `RecordRadarUnits` walks the unit table from slot 0; every other pass starts at slot 1.

## 7. Examples

- `tests/replay/los_radar/console-sight.txt`, tick 0, `GetMapCell`: `x` 33, `y` 19, `Game.width` 224,
  `height` 102: the cell 19 × 224 + 33 = 4289, `o result Cell#4289`.
- `console-sight.txt`, tick 0, `RegionSize`: `map (2501)` gives 2501 × 5 = 12505; `Init` gets `size 12505`.
- `console-units.txt`, tick 1, `ClampWorldPosToTerrain(304, 32)`: `baseX` 3584, `baseY` 1632, `seaLevel` 0, every
  ground height 1. *p*.x = 304 × 65536 = 19922944, *t* = 32. Each *height* is above 0, so it calls the import
  twice (Q1): *i* = 128 calls at z = 160 × 65536 = 10485760, then 144, ..., 32 (2097152): the screen rows are
  160, 144, ..., 48 (1 >> 1 = 0), and *s*1 = 32 ≤ 32 at *i* = 0. *q*.z = 48 × 65536 = 3145728 (two calls), *s*2
  = 48; 32 < 48 and 32 ≥ 32, so *p*.z += (0 × 2^20) / 16 = 0 and *height* is called again (two calls at 2097152).
  22 calls in all; `o result (19922944,65536,2097152)`.
- `console-units.txt`, tick 45, `SoundHeardMask` at (41932177, 835584, 16318543): the map test is (39, 15),
  inside 224 × 102. x̂ = 639, ŷ = 12, ẑ = 249, so *tx* = 19, *ty* = (249 − 6) >> 5 = 7; `o result 3`: players 0
  and 1 have entry 19 + 7 `exploredWidth` explored (bit 1 of `mapFlags` is set).
- `console-units.txt`, tick 140, ADD_EYEBALL: pos (41943040, 65536, 17039360), `sightDistance` 280,
  `eyeHeight` 26, `lifetime` 60, entering with `mapFlags` 15 (true type, line of sight and mapping on, bit 3
  set). The viewing player's sight changes, so the call writes `w Game.mapFlags 7` and `w Game.flags_142f1 5`
  (step 1 of adding a sight). The eye is
  `Eye#0`: `player` `Player#0` (`playerIndex` 0), `x` 280, `flagA` 26, `expires` 140 + 60 = 200. Moved with the
  table's zero `screenPos` and *frame* 0: *x* = 640 >> 5 = 20, *v* = 26 + 1 = 27, *y* = (260 − 13) >> 5 = 7; the
  cache differs, *frame* is 0 so nothing is taken off, then `screenPos` (20,7), `flagB` 27, the sight is added
  (true table: 280 / 32 = 8, *n* = 9, so *k* = 8 and the table is `g_losTables[7]`) and revealed;
  `count_14277` 1, outcome 1. The later eyes: 263, 15 at (41354610, 65536, 16515982) give *v* 16 and (19,7);
  290, 39 at (36700160, 65536, 15728640) give *v* 40 and (17,6).
- `console-units.txt`, tick 201, `ExpireEyeballs`: three eyes, `expires` 200, 211, 230. 200 < 201, so `Eye#0`'s
  sight is taken off and the others move to the front: `count_14277` 2, and the next call reads `Eye#0.x 263`,
  `expires 211`. Ticks 212 and 231 take off the next two. At tick 231 no eye
  is left, and the recording reaches a `Player` only through the eyes in use at the call's end, so it has only
  `w Game.count_14277 0`: the `explored` writes of taking off that sight are made but not recorded or compared,
  and tick 232's `r Player#0.explored` shows them.
- `console-units.txt`, tick 107, `UpdateUnitLineOfSight(Unit#3)`: `posX` 41941916, x̂ = 639, *x* = 19; the
  cache x of slot 3 (region bytes 15 and 16) becomes 19: `w region 15:13`, and the sight is moved.
- `console-sight.txt`, ticks 10 and 20, SET_MODE and RECALCULATE: `mapFlags` 15, `clear` 2, `set` 0: (15 AND NOT
  2) OR 0 = 13; then RECALCULATE(0) clears bit 3: 13 → 5, `w Game.mapFlags 5`. Bit 3 is set again before tick 20,
  so `set` 2 gives 13 OR 2 = 15 (`w Game.mapFlags 15`) and RECALCULATE clears bit 3: `w Game.mapFlags 7`. At
  tick 30, from 15 again, `clear` 4 gives 11 (circle type), and RECALCULATE fetches mask 4: 290 / 32 − 5 = 4
  (`GetGafFrame FrameTable#1 4`, a 19 × 19 mask, offsets 9, transparency 9) and writes `w Game.mapFlags 3`.
- `console-sight.txt`, tick 0, RECALCULATE(1): `RecordRadarMapped` asks for 12 + 112 × 51 = 5724 bytes; the
  snapshot starts `e0000000 66000000 50160000` (224, 102, 5712). `RecordRadarUnits` asks for 8 + 48 = 56 bytes (one
  shown unit, no projectile: `mapFlags` 7, so only the viewing player's units are shown): `Unit#1`, slot 1, id 1,
  x̂ 560, screen row 240, colour 0, flags 2, radar 400, sonar 400, jams 0: `01000000 00000000 0100 0100 3002 0000
  f0000000 00 02 9001 9001 0000 0000 ...`. The events: `RecordSimEvent 35 0 [0] 1` then `34 0 [5728] 1`.
- `console-units.txt`, tick 30, `RefreshUnitIndex(1)`: `Unit#1` and `Unit#2` each go onto `visible` and then
  `known`. Pushes: `visible` empty: `SimAlloc 4`; `known`: `SimAlloc 4`; `visible` (length 1, full):
  `SimAlloc 8`, then `SimFree` of the first; `known` likewise: `[2/2]` each. At tick 60, a third unit: length 2,
  capacity 2, so 2 + 2 = 4 elements, `SimAlloc 16` then `SimFree`, giving `[3/4]`.
- `console-units.txt`, tick 0, `CreateUnitIndex(0)`: `g_losUnitIndex` null: `SimAlloc 440`, then entry 0's `live`
  1 (all else 0).
- `console-units.txt`, tick 42, `GetVisibleEnemiesInRadius(0, (41943040,65536,17039360), 263, 0, out, 64)`:
  `Unit#252` in `visible` stands at the same position (*d* = 0 ≤ 263²) with flags 0x98280121 (ready):
  `o out [Unit#252]`, result 1.
- `sensors-sonar-jam.txt`, tick 30, `UpdateSensorRadarAndCloak`, viewing player 0, `seaLevel` 0. Pass 1 gives
  `Unit#1` (player 0) 0x300 and clears 0x700 of `Unit#252` (player 1). Pass 2: `Unit#1`, def radar 400, sonar 400,
  ŷ 1: *rr* = 402² = 161604, *ss* = 160000, range 400 × 65536 = 26214400:
  `UnitsInRange (36700160,65536,15728640) 26214400 0 ? 256 = 2`, ids 1 and 252. `Unit#1` is the viewing player's;
  `Unit#252` (def 103, `flags1` 0x008480a9, bit 8 clear) is at *dx* = 80 × 65536, *dz* = 60 × 65536 from it:
  *d* = 6400 + 3600 = 10000. Its `pos.y` 65536 is above 0, so no 0x200; 802816 + 65536 ≥ 0 and 10000 < 161604,
  so 0x100. Pass 3: `Unit#252`'s `sonardistancejam` 650: `UnitsInRange (41943040,65536,19660800) 42598400`, ids 1
  and 252, each loses 0x200 and gains 0x400. Ends: `Unit#1.flags 0x90e00521`, `Unit#252.flags 0x10240521`.

**Not covered by any recording**: the slots `GetOriginCellAtPosition`, `IsUnitVisibleToPlayer` (it is reached
through `RefreshUnitIndex`), `HasReadyUnitInRange` (reached through pass 4 of the sensors), `IsPointVisible`,
`GetSeenUnitsInRadius`, `FreeUnitIndex` and `Shutdown`; the circle type's move (only its place, with mask 4, is
recorded); `Game.seaLevel` above 0; the cloak pass (pass 4) setting `workTime`, the radar-jam branch of pass 3, and
the allied-share and watch-mode branches of pass 1; interceptor weapon records and nuclear projectiles in the
snapshot; `hasSpecial` = 1 and the `known` fallback; the `Apply` kinds CELL_HEIGHTS, SHARE_MAPPING,
SHARE_RADAR, SHARE_MAP and SIGHT_LOAD, every outcome 0 of `Apply`, ADD_EYEBALL with 20 eyes or line of sight
off; a null answer of `SimCaptureAlloc`; `map.UnitsInRange` returning 256; `SimFree` of a freed index; the
edges of section 5 and Q2's *k* = 0.

## 8. Kit version

Kit `interfaces-v69` and `reference-traces-v56`, the versions the gate judges against. Interface `los_radar`
5.1.
