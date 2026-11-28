# map

## 1. Role

Map is the ground as the simulation sees it. It keeps which unit stands on each cell of the map (the
footprints, and the claim of cells when two units overlap), the spatial grid of units (a bucket for each square
of 8 by 8 cells, and a bucket for the units off the map), and the pass map of each movement class: two bits a cell
that say whether a unit of that class can stand there, and at what cost. Other modules publish to map when a unit
is placed, moves, dies, boards or leaves a carrier, or opens its yard, and when the terrain or a feature changes;
map then updates the cells, the grid and the pass maps, and reports each pass map change to pathfinding. Map also
answers the placement tests (can a unit type be built or placed at a cell), the height of the ground at a
position, the units within range of a position, and the pick of a random feature to reclaim. It saves and loads
the metal and player feature plot maps of the cells, and frees what it allocated when a match shuts down.

## 2. Interface

### Slots (`interfaces/map.h`, version 9.2)

| Slot | When the engine calls it |
| --- | --- |
| `Apply` | for each event of map's inbox: before each synchronous slot below, at each phase boundary, and at once for an event published outside the tick or on a synchronous route |
| `UnitsInRange` | a query, whenever orders, the victory conditions or los_radar walk the units near a position, a page of ids at a time |
| `GetCellMeanHeight` | a query, from features, players and weapons |
| `CanBuildAt` | synchronous, from ai and the build interface; map's inbox is delivered first |
| `CanPlaceUnitFootprint` | synchronous, from orders, ai, players and scripts; map's inbox is delivered first |
| `BuildAllPassMaps` | once at match load, after the terrain and the features are loaded, outside the tick |
| `FreePassMaps` | when a match shuts down; the engine's for now (below) |
| `LoadMetalPlotmap`, `LoadPlayerFeaturesPlotmap` | when a saved game is loaded |
| `SaveMetalPlotmap`, `SavePlayerFeaturesPlotmap` | when a game is saved |
| `SnapWorldPosToFootprint`, `CanPlaceFootprintAt`, `IsPadSlotFree` | queries, from orders |
| `PickRandomReclaimableResourcesInRadius` | synchronous, from orders' patrol and repair orders; map's inbox is delivered first |
| `AllocateBuckets` | at match load, when the terrain's derived layers are built, outside the tick; the engine's for now (below) |
| `FreeBuckets` | when a match shuts down; the engine's for now (below) |
| `GetGroundHeight` | a query, from almost every module |
| `PassMapGet`, `PassMapInBounds` | queries, from pathfinding during a search |
| `RefreshMovedUnits` | synchronous, at the start of a path search for a unit; map's inbox is delivered first |
| `RefreshUnitIfStale` | synchronous, at the end of a path search for a unit; map's inbox is delivered first |
| `WriteState` | from the harness, after each phase, to dump map's own state (record 45, `mapState`, dumped and never hashed: `docs/trace-format.md`, section 8.28) |
| `PathLockStamp` | a query, when a saved game is written, outside the tick |

**The engine's slots.** The manifest's entry for map (`wasm/modules.json`, `engine`) keeps nine slots in the engine:
`FreePassMaps`, `AllocateBuckets` and `FreeBuckets` (load class), the four plot map slots `LoadMetalPlotmap`,
`LoadPlayerFeaturesPlotmap`, `SaveMetalPlotmap` and `SavePlayerFeaturesPlotmap` (save class: their buffers are the
engine's, `docs/module-abi.md` section 13, "What stays in the engine"), `WriteState` (a dump query, called from the
harness only; the engine writes map's state from the block, the same section) and `PathLockStamp` (a query of a saved
game). The module doesn't export them, and the host calls the engine's native slot instead, with the module's state
in the block (ABI section 6, "Engine slots"). Moving the load-class ones into the module is ozgb/btm#43. Their
subsections in section 4 state what the engine's slot does with the block, which the module keeps or reads as block
data: the bucket grid, the off-map bucket, the pass maps freed at shutdown, the plot maps, the dumped state and the
stamps.

### The manifest entry

The kit's manifest (`wasm/modules.json`) has map's entry. The module must match it exactly (ABI 9.1):

- `name` and `interface`: `map`.
- `exports`, in this order: `Apply`, `UnitsInRange`, `GetCellMeanHeight`, `CanBuildAt`, `CanPlaceUnitFootprint`,
  `BuildAllPassMaps`, `SnapWorldPosToFootprint`, `CanPlaceFootprintAt`, `IsPadSlotFree`,
  `PickRandomReclaimableResourcesInRadius`, `GetGroundHeight`, `PassMapGet`, `PassMapInBounds`, `RefreshMovedUnits`,
  `RefreshUnitIfStale`. The other nine slots are the engine's (above).
- `data`: `g_game`, `g_simGlobals`, `g_extTdf`, `g_blockBase` (ABI 8.2). The module reads `g_game`,
  `g_simGlobals` and `g_blockBase`.
- `majors`: `map` 9, `los_radar` 5, `features` 5, `services` 4.
- `publishes`: `{"132": 20, "176": 32}`: los_radar's CELL_HEIGHTS (`LosRadarCellHeights`, 20 bytes), which the route
  table carries from map's outbox to los_radar, and CELLS_CHANGED (`MapCellsChanged`, 32 bytes). `Emit` takes any kind
  a route carries from the calling module's outbox (ABI 9.3).
- `data_base` 557056 and `table_base` 400: map's window of `window` (32768) bytes from 0x88000, and its 32 table
  entries from 400 (ABI 2.2).
- `blocked`: the kit's entry is `"blocked": true`, so it has no `fuel_budget` and no `size_budget`, and the kit has no
  map `.wasm` (ABI 4.2).
- Fuel: the module meters itself by rule version 1 (ABI section 11) and exports `__fuel_set` and `__fuel_get`. The
  gate judges a module of a blocked entry without budgets (ABI section 15, "A blocked entry is judged without
  budgets"): its size is not judged, and it runs with the largest fuel budget of ABI 11.3.
- The gate: the entry is blocked, so the oracle takes its `recordings` route (ABI section 15): it replays the module
  on every recording of `tests/replay/map/`, and its result has `scope` `recordings`, `scenarios` `not_judged` and
  `budgets` `not_judged`.
- Slot kinds, as the entry gives them:

| Slot | Kinds |
| --- | --- |
| `Apply` | `[linear, int] -> int` |
| `UnitsInRange` | `[linear, int, int, linear, int] -> int` |
| `GetCellMeanHeight`, `GetGroundHeight` | `[linear] -> int` |
| `CanBuildAt` | `[block, linear, int, block] -> int` |
| `CanPlaceUnitFootprint` | `[block, int, linear, int] -> int` |
| `BuildAllPassMaps` | `[linear, int] -> void` |
| `SnapWorldPosToFootprint` | `[block, linear] -> void` |
| `CanPlaceFootprintAt` | `[block, linear] -> int` |
| `IsPadSlotFree` | `[block, int] -> int` |
| `PickRandomReclaimableResourcesInRadius` | `[linear, int, linear, linear, linear, linear] -> int` |
| `PassMapGet`, `PassMapInBounds` | `[int, int, int] -> int` |
| `RefreshMovedUnits`, `RefreshUnitIfStale` | `[int, linear] -> void` |

The `short` argument `type` of `CanBuildAt` arrives sign-extended, and the `unsigned short` `otherTypeId` of
`CanPlaceUnitFootprint` zero-extended (ABI 7). The unit of the two refreshes is the address of a `Unit` in the block.

- Imports, as the entry lists them:

| Import | Kinds | Used for |
| --- | --- | --- |
| `los_radar.GetMapCell` | `[int, int] -> block` | the first cell of a footprint, in the claim |
| `los_radar.GetMapCellAtPosition` | `[linear] -> block` | the cell under a point, in the reclaim pick |
| `features.GetCellFeature` | `[block] -> int` | the feature on a cell, in the reclaim pick |
| `services.RandomInt` | `[int] -> int` | the reclaim pick's draws |
| `services.SimAlloc` | `[int] -> linear` | the pass map words, the scratch block of `BuildAllPassMaps`, the reclaim lists |
| `services.SimFree` | `[linear] -> void` | the old pass map words, the scratch block, the reclaim lists |
| `services.Emit` | `[linear, int] -> void` | publishing CELLS_CHANGED and CELL_HEIGHTS |

- Hooks: every slot of `interfaces/map.h` says "Hooks: none", and the module raises none.

### Kit gaps

At `interfaces-v69` these are gaps in the kit, not in the module:

- **The budgets.** Map's entry is blocked, so it has no budgets, and the module's fuel and size are judged only once
  it has them (above).
- **The publish payloads.** The recordings carry a publish as a `services.SchedPublish` line, with 7 and the size (the
  publisher, map, and the payload size; the payload's fields aren't recorded), which is `services.Emit` (ABI 9.3,
  "Routes in the recordings"). The replay compares only the size of each publish (20 for CELL_HEIGHTS, 32 for
  CELLS_CHANGED) and its order among the imports. The rectangle, classes and cause of a CELLS_CHANGED appear as the
  arguments of pathfinding's `CellsChanged` calls in `tests/replay/pathfinding/` (`orders-factory.txt`,
  `orders-move-attack.txt`, `skirmish2.txt`); those of a CELL_HEIGHTS are checked by no recording, and the gate judges no scenario of a blocked entry.

### Arguments outside the block

- **An event** given to `Apply` is a payload of `interfaces/map.h` in the caller's memory (`linear`), `size` bytes,
  starting with `MapEventHeader`. Its `unit` is a block offset (0 for none).
- **A position** (`pos` of `UnitsInRange`, `GetCellMeanHeight`, `GetGroundHeight`, `SnapWorldPosToFootprint`,
  `CanPlaceFootprintAt` and the reclaim pick) is the address of a `Vec3`: three 32-bit signed fixed-point numbers,
  `x`, `y`, `z`, 65536 to a world unit, 2^20 to a cell (16 world units). It may lie in the block (an order's
  position) or in the caller's memory. Only `SnapWorldPosToFootprint` writes it.
- **A cell** given to `CanBuildAt` and `CanPlaceUnitFootprint` is the address of a `Point16` (`x`, then `y`, each a
  signed 16-bit integer), read only.
- **`MapMoveLimits`** (`BuildAllPassMaps`): `count` records of 16 bytes in the engine's memory, as
  `interfaces/map.h` lays them out: `index` (int), `footprintX`, `footprintZ`, `maxWaterDepth`, `minWaterDepth`
  (short), `maxSlope`, `badSlope`, `maxWaterSlope`, `badWaterSlope` (unsigned char).
- **The out arguments of the reclaim pick** (`energy`, `metal`: the address of a `Vec3` pointer each;
  `energyAmount`, `metalAmount`: the address of a 32-bit float each) and the `ids` buffer of `UnitsInRange` are the
  caller's memory, written as section 4 says.

### Events

None. No slot of this module records a sim event (kinds are listed in `docs/sim-events.md`).

## 3. State

Every `Ref` field is a block offset (`docs/module-abi.md` section 7.1): the address is `g_blockBase` plus the offset,
and 0 is null (X2). The unit flags this section names are those of `docs/sim-events.md`, Terms.

### Owned

**`Cell`** (the elements of `Game.cells`):

| Field | Type | Written by | Meaning |
| --- | --- | --- | --- |
| `unit` | unsigned short | `Apply` | the id of the unit that holds the cell for ground placement, 0 for none |
| `unit2` | unsigned short | `Apply` | the id of the unit that holds the cell in the air layer, 0 for none |
| `flags`, bit 1 (mask 2) | 1 bit | `Apply` | set while a structure whose yard map byte has bit 0x01 for the cell stands on it |
| `plotFeature4` (bits 3 to 6 of `flags`) | 4 bits | `LoadPlayerFeaturesPlotmap` | the player feature plot map value of the cell |
| `metal` | unsigned char | `LoadMetalPlotmap` | the metal plot map value of the cell |

**`Unit`**, written by `Apply`:

| Field | Type | Meaning |
| --- | --- | --- |
| `flags`, bits 27 and 26 (`f26_27`) | 2 bits | the claim pending and overlapped bits (flag bits, below) |
| `bit2` (bit 2 of `cobStateFlags`, byte 271) | 1 bit | the yard is open (1) or closed (0) |
| `spatialBucket` | `Ref<SpatialBucket>` | the bucket the unit is filed in: a bucket of the grid, the off-map bucket, or null |
| `cargoNext` | `Ref<Unit>` | the next unit of the chain the unit is in |

**`UnitMotion.pathLockStamp`** (int): a unit's stamp. Written by `Apply`; `RefreshMovedUnits` changes it only during
the call (Q27).

**The buckets.** Every bucket, in the grid and the off-map one, is a 10-byte record that `layout.json` lays out twice
over the same bytes: as `SpatialBucket` (`bytes`, bytes 0 to 5, then `head`) and as `Rec` (`a`, `b`, `flags`, then an
int). This spec calls the three first fields of a bucket its *a*, *b* and *flags*, and its last its `head`:

| Bytes | Name here (`layout.json`) | Type | Meaning |
| --- | --- | --- | --- |
| 0 | *a* (`Rec.a`) | unsigned char | the highest `Cell.high` of the bucket's cells, at least `Game.seaLevel` |
| 1 | *b* (`Rec.b`) | unsigned char | the highest *a* of the bucket and its neighbours |
| 2 to 5 | *flags* (`Rec.flags`) | int | the edge bits (flag bits, below); 0x1f in the off-map bucket |
| 6 to 9 | `head` (`SpatialBucket.head`) | `Ref<Unit>` | the first unit of the bucket's chain, 0 for an empty chain |

The recordings show bytes 0 to 5 as `SpatialBucket#N.bytes`.

**The grid record**, `Game.spatialGrid_1429f`, a `Grid2` (24 bytes) that `layout.json` also lists field by field
under other names in `Game`. The two namings are the same bytes:

| `Grid2` field | The same bytes in `Game` | Type | Meaning |
| --- | --- | --- | --- |
| `cells` | `Game.cells_1429f` | `Ref<struct FogCell>` in `layout.json` (`Ref<SpatialBucket>` in `Game`) | the first bucket of the grid; 0 when there is none |
| `width` | `Game.width_142a3` | unsigned int | buckets in a row, *GW* below |
| `height` | `Game.height_142a7` | unsigned int | rows of buckets, *GH* below |
| `count` | `Game.pad_142ab` (`char[4]` in `Game`, the same 4 bytes read as an int) | int | the number of buckets allocated, *N* below |
| `widthFixed` | `Game.field_142af` | int | `Game.baseX` × 65536 |
| `heightFixed` | `Game.field_142b3` | int | `Game.baseY` × 65536 |

`Game.overflowBucket` (`Ref<SpatialBucket>`): the off-map bucket; 0 when there is none.

**The pass maps**, `SimGlobals.g_passMaps`, a `PassMapTable`: `entries`, 32 `PassMap` records of 32 bytes, one per
movement class index 0 to 31. "PassMap k" is entry k, and k is "the class".

| Field | Type | Meaning |
| --- | --- | --- |
| `footprintX`, `footprintZ` | short | the class footprint, in cells: columns, then rows |
| `maxWaterDepth`, `minWaterDepth` | short | the water depth limits, below sea level |
| `maxSlope`, `badSlope` | unsigned char | the slope limits on land |
| `maxWaterSlope`, `badWaterSlope` | unsigned char | the slope limits under water |
| `defined` | int | 1 when the last `BuildAllPassMaps` gave this class, else 0 |
| `width`, `height` | unsigned int | the size of the class's map, in cells |
| `cells` | `Ref<unsigned int>` | the pass map words (section 4, Definitions) |
| `lastTick` | unsigned int | the class's window |

The eight limit fields are copied from a `MapMoveLimits` (`interfaces/map.h`), field for field.

**The placement record** (`SimGlobals`):

| Field | Type | Meaning |
| --- | --- | --- |
| `g_lastPlaceHeight` | int | the ground level of the last site that fitted (0 to 255), or 0 |
| `g_lastPlaceMetalSum` | int | the sum of `Cell.metal` over the cells of the footprint that the last test reached |

**`Game`**: `seaLevel` (unsigned char), `mapWidth`, `mapHeight` (int, the playable width and height in world units),
written by `Apply`; `progress_38d73` (unsigned char), written by `BuildAllPassMaps`; and the grid record and
`overflowBucket` above.

### Read

- `Game`: `width`, `height` (int, the map in cells), `cells`, `units` (the unit table, `Unit` records of 280 bytes, the
  unit of id *n* at index *n*), `unitsEnd` (the last unit of the table), `ticks` (int), `baseX`, `baseY` (int, the map
  in world units), `seaLevel`, `featureCount` (int), `features` (`Feature` records of 256 bytes), `visibilityMask`
  (one 16-bit word per element), `playerIndex` (unsigned char), `mapFlags` (only the bit of mask 2, map flag 2).
- `Game.poolCount` (unsigned 16-bit), the number of unit slots in the table (`Game.unitsEnd` is the slot at index
  `poolCount` − 1). The recordings read it, and every `Unit#N.motion`, in each call that can walk the units; the behaviour reads only the units section 4 names (for `BuildAllPassMaps`, the
  motion of a unit that holds a cell).
- `Cell`: `unit`, `unit2` (unsigned 16-bit), `height`, `high`, `low`, `metal` (unsigned bytes), `feature` (unsigned 16-bit),
  `offsetY`, `offsetX` (unsigned bytes, at offsets 10 and 11, read only on a cell whose `feature` is 0xfffe), `flags`
  (bit 1, and `plotFeature4`).
- `Unit`: `id` (unsigned 16-bit), `cell` and `footprint` (`Point16`, `x` then `y`, signed 16-bit), `pos` (`Vec3`, its
  `x` and `z`), `def`, `player`, `motion`, `carrier`, `cargo`, `cargoNext`, `playerIndex` (unsigned byte),
  `transportPiece` (read as a signed byte), `cobStateFlags` bit 2, and `flags` (the bits below).
- `UnitDef`: `footprintx`, `footprintz` (signed 16-bit), `yardmap` (`Ref<char>`, read as unsigned bytes), `draft`,
  `maxslope`, `maxwaterslope`, `mobile` (unsigned bytes), `maxwaterdepth`, `minwaterdepth` (signed 16-bit), `flags1`
  (32 bits).
- `Player`: `active` (int), `type` (unsigned byte, offset 115), `explored` (`Ref<unsigned char>`), `exploredWidth`,
  `exploredHeight` (int).
- `Feature`: `flags16` (the bits below), `energy`, `metal` (32-bit floats).
- `SimGlobals.g_quirks.skipEmptyYardmap` (`interfaces/quirks.h`). Every recording runs with it 0, the default, and no
  `r` line shows it.

### Flag bits

**`Unit.flags`**, the 32-bit word at offset 272:

| Bits | Name here (`layout.json`) | Meaning |
| --- | --- | --- |
| 0x20000000 (bit 29) | structure (`Unit.immobile`) | unit flag 0x20000000: the unit's cells follow its yard map |
| 0x10000000 (bit 28) | alive (`Unit.alive`) | unit flag 0x10000000 |
| 0x8000000 (bit 27) | claim pending (`Unit.f26_27`, high bit) | the unit has cells to take again: set when it lost a cell to another unit, and by SET_YARD and RECLAIM_FOOTPRINT; a claim clears it first |
| 0x4000000 (bit 26) | overlapped (`Unit.f26_27`, low bit) | another unit is waiting for cells this unit holds; REMOVE_UNIT clears it and lets the overlapping units claim again |
| 0x3 (bits 0 and 1) | layer (`Unit.mode`) | for a unit without the structure bit: 1 means the unit holds `Cell.unit`, 2 means it holds `Cell.unit2` |

**The yard map bits.** A yard map byte (section 4, "The yard map") is read as an unsigned byte:

| Bit | Meaning |
| --- | --- |
| 0x01 | the cell is marked under the structure (`Cell.flags` bit 1 follows it); refused for building on a cell whose `Cell.flags` has bit 1 |
| 0x02 | the structure holds the cell while its yard is open; refused for building on a cell held by another unit |
| 0x04 | the structure holds the cell while its yard is closed; refused for building on a cell held by another unit |
| 0x08 | a ground cell: its heights make the ground level |
| 0x10 | a cell whose `Cell.high` must not stand above the ground level |
| 0x20 | refused for building on a cell blocked by a feature |
| 0x40 | refused for building on a cell whose feature is indestructible |
| 0x80 | the footprint needs a geothermal feature on one of these cells |

**The feature bits**, of `Feature.flags16` (bytes 254 and 255 of a `Feature`; `layout.json` also gives byte 254 as
`Feature.flags` and byte 255 as `Feature.flagsHigh`): 0x20 `geothermal`, 0x40 `blocking`, 0x80 `reclaimable`, 0x100
`autoreclaimable`, 0x200 `indestructible`.

**Other bits:** `UnitDef.flags1` bit 11 (0x800) and bit 21 (0x200000), read by `CanPlaceFootprintAt`, have no name in
the kit. The edge bits of a bucket's *flags*, not in `layout.json`: 1 the top row of buckets, 2 the bottom row, 4 the
left column, 8 the right column.

### Between ticks

- Every field of the 32 `PassMap` records. A class's `lastTick` is written only by `RefreshMovedUnits`;
  `BuildAllPassMaps` and `FreePassMaps` keep it (Q16).
- The grid and the off-map bucket, allocated at load and freed at shutdown. Only `Apply`'s chain writes (`head`,
  `Unit.cargoNext`, `Unit.spatialBucket`) change them during a match. A unit is in at most one bucket's chain, and
  only when its `Unit.carrier` is null.
- Nothing in the placement record has to hold between ticks: the caller reads it right after the call.
- Nothing outside the block.

## 4. Behaviour

Within one call, only the order of import calls is observable. The block writes are compared at the end of the call,
so the order they are listed in carries no meaning.

### Definitions

**A footprint.** The footprint at (x, y) of size (w, h) is the rectangle of w columns and h rows of cells with left
top (x, y), taken row by row from the top, left to right. Its k-th cell (counting from 0) is the cell (x + col,
y + row) with k = row × w + col, for row 0 to h − 1 and col 0 to w − 1. With w ≤ 0 or h ≤ 0 it has no cells. The
footprint of a unit is the footprint at `Unit.cell` of size `Unit.footprint`. The footprint of a unit type at (x, y)
has the size (*fx*, *fz*) = (`UnitDef.footprintx`, `UnitDef.footprintz`).

**The cell (x, y)** is element y × `Game.width` + x of `Game.cells` (`Cell#N` in the recordings): its index is
y × `Game.width` + x.

**The unit of id n** is element n of the table `Game.units` (`Unit#N`). A unit id on a cell names that unit.

**A unit's stamp** is `UnitMotion.pathLockStamp` of the record `Unit.motion` names. A unit with null motion has no
stamp.

**A class's window** is `lastTick` of its `PassMap`. **A defined class** is a `PassMap` whose `defined` is not 0.

**The feature tests.** Let *v* be the `Cell.feature` of a cell (X18).

- **The origin cell** of a cell whose *v* is 0xfffe is the cell at index n − (`Cell.offsetY` × `Game.width` +
  `Cell.offsetX`), where n is the cell's own index.
- **Blocked by a feature**, with *v2* the `Cell.feature` of the origin cell:

  | *v* | Blocked |
  | --- | --- |
  | 0xffff | no |
  | below 0xfffb, and ≥ `Game.featureCount` | yes |
  | below 0xfffb, and < `Game.featureCount` | the `blocking` bit of element *v* of `Game.features` |
  | 0xfffb, 0xfffc, 0xfffd | yes |
  | 0xfffe, with *v2* ≥ 0xfffb | no (Q1) |
  | 0xfffe, with *v2* < 0xfffb | the `blocking` bit of element *v2* of `Game.features`, with no test against `Game.featureCount` (Q1) |

- **The feature record of a cell:** element *v* of `Game.features` when *v* < 0xfffb and *v* < `Game.featureCount`;
  for *v* = 0xfffe, element *v2* when *v2* < 0xfffb (whatever `Game.featureCount` is), and none otherwise; for any
  other value, none.

**The yard map** of a unit type is the bytes at `UnitDef.yardmap`, *fx* × *fz* of them, one unsigned byte per cell of
its footprint, in footprint order: byte k goes with the k-th cell. Its bits are in section 3, "Flag bits" (X12). **The yard
bit of a unit** is 0x02 when its `Unit.bit2` is 1 and 0x04 when it is 0. A slot that reads the yard map of a unit type
whose `yardmap` is null is undefined, but for the SET_YARD case `skipEmptyYardmap` covers.

**A take of a cell field by a unit.** The field is `Cell.unit` or `Cell.unit2` of one cell, and the taking unit is *T*
(id *t*). Let *v* be the field's value, and *O* the unit of id *v*:

| Case | Writes |
| --- | --- |
| *v* is 0 | the field becomes *t* |
| *v* isn't 0, and *O*'s player (`Unit.player`) has `Player.active` ≠ 0 and `Player.type` = 3 | *O* gets the claim pending bit, *T* gets the overlapped bit, and the field becomes *t* |
| *v* isn't 0, otherwise | *O* gets the overlapped bit, *T* gets the claim pending bit, and the field keeps *v* |

*O* may be *T* itself, when the cell already holds *t*: the same rule applies, so *T* gets both bits (Q2). A take
only ever sets flag bits. A null `Unit.player` of *O* is undefined (X13).

**The claim of a unit.**

- If the unit's claim pending bit is 0, the claim changes nothing and calls no import.
- Otherwise the bit becomes 0, and the claim calls `los_radar.GetMapCell(x, y)` once, with (x, y) = `Unit.cell` (each
  sign-extended to 32 bits). The result is the address of a cell, at index *n* of `Game.cells`. The k-th cell of the
  claim (row and col as in the unit's footprint) is the cell at index *n* + row × `Game.width` + col. A null result
  is undefined. Then, by `Unit.flags` as it is after the bit is cleared:

| `Unit.flags` | Each cell of the claim (the k-th) |
| --- | --- |
| structure bit set | if yard map byte k has the unit's yard bit: a take of `Cell.unit`. Otherwise, if `Cell.unit` is the unit's id, it becomes 0. `Cell.flags` isn't changed. |
| structure bit clear, layer 1 | a take of `Cell.unit` |
| structure bit clear, layer 0, 2 or 3 | a take of `Cell.unit2` |

The takes run over the cells in footprint order (k from 0), each one seeing the bits and cells the earlier ones wrote.
A take can set the claim pending bit of the unit itself again (Q2). The claim never publishes and never changes a
pass map.

**The re-claim of the units overlapping a rectangle** (left *L*, top *T*, width *W*, height *H*, signed 16-bit values
from `Unit.cell` and `Unit.footprint`). Let *R* = *L* + *W* and *B* = *T* + *H*. A unit *o* overlaps the rectangle
when *L* < `o.cell.x` + `o.footprint.x`, *R* > `o.cell.x`, *T* < `o.cell.y` + `o.footprint.y` and *B* > `o.cell.y`
(X11).

The re-claim visits the buckets of the grid with column *bx* from ⌊*L* / 8⌋ − 1 to ⌊*R* / 8⌋ + 1 and row *by* from
⌊*T* / 8⌋ − 1 to ⌊*B* / 8⌋ + 1, **columns in the outer order and rows in the inner order**: (bx0, by0), (bx0, by0 + 1),
..., then (bx0 + 1, by0), and so on. A bucket whose *bx*, taken as unsigned, is ≥ *GW*, or whose *by*, unsigned, is ≥
*GH*, is skipped (so are negative columns and rows). The bucket visited is element *by* × *GW* + *bx* of the grid. The
off-map bucket is never visited.

In each visited bucket, for each unit *o* of its chain in chain order:

1. if *o* overlaps the rectangle, the claim of *o* is made;
2. then, for each unit *c* of *o*'s cargo chain in order (from `o.cargo` along `Unit.cargoNext` to null), if *c*
   overlaps the rectangle, the claim of *c* is made. The cargo is visited whether or not *o* overlaps.

Each claim sees the writes of the claims before it: a take that sets the claim pending bit of a unit visited later
makes that unit claim when its turn comes, and one that sets it on a unit already visited leaves the bit set. The order
of the `los_radar.GetMapCell` imports is the order of the claims that find their bit set.

**The bucket grid.** *GW* × *GH* buckets, each 2^23 fixed-point units (8 cells) square, over the map, row by row from
the top: bucket (x, z) is element z × *GW* + x of the buckets at `Game.cells_1429f`. A further *N* − *GW* × *GH* buckets
follow them as padding: they are in no row or column. A bucket's bytes start at 10 × its element number.

- **The chain of a bucket** is the list of units from its `head` along `Unit.cargoNext` to null.
- **The bucket column of a coordinate** *v* (a 32-bit fixed-point `x` or `z`) is *v* arithmetic shift right by 23.
- **The bucket of a position** is element *bz* × *GW* + *bx* of the grid, with *bx* and *bz* the bucket columns of
  its `x` and its `z`. It isn't clamped to the grid (Q3, X8).
- **The off-map bucket** is the bucket `Game.overflowBucket` names, outside the grid.

**Filing a unit in a bucket *b***. If *b* is the unit's `Unit.spatialBucket` already, nothing changes. Otherwise:

- if `Unit.carrier` is null: when `Unit.spatialBucket` isn't null, the unit is unlinked from that bucket's chain; then
  `Unit.cargoNext` becomes *b*'s `head`, and *b*'s `head` becomes the unit (the unit goes to the front of *b*'s chain);
- whatever the carrier, `Unit.spatialBucket` becomes *b*.

A carried unit changes no chain. *b* is never null (filing before the buckets are allocated is undefined).

**Unlinking a unit from a bucket's chain.** If the bucket's `head` is the unit, the `head` becomes the unit's
`Unit.cargoNext`. Otherwise the unit before it in the chain gets the unit's `Unit.cargoNext` as its own
`Unit.cargoNext`. Each event that unlinks then sets the unit's own `Unit.cargoNext` to null (filing sets it again at
once). Unlinking a unit that isn't in the chain is undefined, except in BUCKET_UNLINK, which tests it first.

**The pass map words.** The map of class k has `width` × `height` cells, each a 2-bit pass value: 0 (closed), 1
(passable at a cost, or at an edge), 3 (open). 2 is never stored. The words are at `cells`: ⌊(`height` + 15) / 16⌋ ×
`width` unsigned 32-bit words. One word holds 16 rows of one column: the value of (x, y) is in word ⌊y / 16⌋ × `width`
+ x, at bits 2 × (y mod 16) and 2 × (y mod 16) + 1. Writing a value replaces those two bits only (X16).

**The pass value of a cell for class k.** For the cell (x, y), with *L* the limits of PassMap k:

| First condition that holds, in this order | Pass value |
| --- | --- |
| the cell is blocked by a feature | 0 |
| `Cell.unit` ≠ 0, and that unit has a null `Unit.motion` or a stamp below the class's window (unsigned) | 0 |
| `Cell.low` < `Game.seaLevel` − *L*.`maxWaterDepth` | 0 |
| `Cell.high` > `Game.seaLevel` − *L*.`minWaterDepth` | 0 |
| under water (`Cell.low` < `Game.seaLevel`) and the slope > *L*.`badWaterSlope` | 1 when the slope ≤ *L*.`maxWaterSlope`, else 0 |
| on land (`Cell.low` ≥ `Game.seaLevel`) and the slope > *L*.`badSlope` | 1 when the slope ≤ *L*.`maxSlope`, else 0 |
| otherwise | 3 |

The slope is (`Cell.high` − `Cell.low`) mod 256 (X17).

**The full build of class k.** With *W*, *H* the class's `width` and `height`, *fx*, *fz* its `footprintX`,
`footprintZ` (each at least 1 in every call the engine makes), and the pass values of the cells for class k as they
are at the call (its window included), the value written at each (x, y) with 0 ≤ x < *W* and 0 ≤ y < *H* is:

| Condition | Value |
| --- | --- |
| a cell of the block of columns x to x + *fx* − 1 and rows y to y + *fz* − 1 is off the map, or has pass value 0 | 0 |
| otherwise, every cell of the grown block, columns x − 1 to x + *fx* and rows y − 1 to y + *fz*, is on the map and has pass value 3 | 3 |
| otherwise | 1 |

"On the map" is 0 ≤ column < *W* and 0 ≤ row < *H*. So a block whose grown block passes an edge of the map is at most
1, and a block that passes the right or bottom edge itself is 0. The bits of rows `height` and up of the last row of
words are 0 after the build: with the values above, the build writes every bit of the words.

**The footprint value of class k at (x, y).** With *W* = `Game.width`, *H* = `Game.height` (signed), *fx*, *fz* the
class footprint, and the pass values of the cells for class k with its current window:

| First condition that holds, in this order | Value |
| --- | --- |
| x < 0, or y < 0, or x + *fx* ≥ *W*, or y + *fz* ≥ *H* | 0 |
| a cell of the block of columns x to x + *fx* − 1 and rows y to y + *fz* − 1 has pass value 0 | 0 |
| x ≥ 1, y ≥ 1, x + *fx* ≤ *W* − 2, y + *fz* ≤ *H* − 2, and every cell of the grown block (columns x − 1 to x + *fx*, rows y − 1 to y + *fz*) has pass value 3 | 3 |
| otherwise | 1 |

The bounds are signed (X19). This differs from the full build at the right and bottom edges (Q4).

**A rebuild of class k over (cell, size).** cell = (*cx*, *cy*) and size = (*sx*, *sy*) are `Point16` values,
sign-extended; *fx*, *fz* are the class footprint, *W*, *H* its `width` and `height`. The rectangle is the columns from
left = *cx* − *fx* to right = *cx* + *sx* + 1 (not included), and the rows from top = *cy* − *fz* to bottom = *cy* +
*sy* + 1 (not included), clipped: a left or top below 0 becomes 0; right becomes *W* when right > *W*, and bottom
becomes *H* when bottom > *H*, both compared unsigned (X20). When left < right and top < bottom (signed),
every (x, y) of the clipped rectangle gets the footprint value of class k at (x, y) in the class's words. Nothing else
is written and nothing is imported. Each footprint value reads only the cells, so the order of these writes doesn't
matter.

**A report of class k over (cell, size) with cause c.** A publish (`Emit`; `SchedPublish 7 32` in the recordings) of
a 32-byte CELLS_CHANGED effect, the `MapCellsChanged` payload of `interfaces/map.h`, with *cx*, *cy*, *sx*, *sy*, *fx*,
*fz*, *W*, *H* as in the rebuild:

| Field | Value |
| --- | --- |
| `h.kind` | 176, `MAP_EFFECT_CELLS_CHANGED` |
| `h.unit` | 0 |
| `rect[0]`, left | *cx* − *fx*, or 0 when that is below 0 |
| `rect[1]`, top | *cy* − *fz*, or 0 when that is below 0 |
| `rect[2]`, right | *cx* + *sx*, or *W* − 1 when *cx* + *sx* ≥ *W* (signed, *W* as an int) |
| `rect[3]`, bottom | *cy* + *sy*, or *H* − 1 when *cy* + *sy* ≥ *H* (signed) |
| `classes` | 1 << k |
| `cause` | c, a `PathChange` of `interfaces/pathfinding.h`: 1 `PATH_CHANGE_TERRAIN`, 2 `PATH_CHANGE_UNITS` |

right and bottom are included in the rectangle. A report is published even when the rebuild's rectangle was empty or
no value changed (Q5, X21).

**A rebuild and report of every defined class** over (cell, size) with cause c: a rebuild of every defined class over
(cell, size), then a report of every defined class over the same, in increasing class index: one CELLS_CHANGED publish
per defined class. With no defined class, nothing is written and nothing is published.

**The stamp update of a unit** (REMOVE_UNIT and CLEAR_UNIT):

| `Unit.motion` | Writes and imports |
| --- | --- |
| null | a rebuild and report of every defined class over (`Unit.cell`, `Unit.footprint`), cause 2 |
| not null | let *s* be the unit's stamp before the event. The stamp becomes `Game.ticks`. Then, for each class index k from 0 to 31 in increasing order whose window is greater than *s* (unsigned): a rebuild of class k over (`Unit.cell`, `Unit.footprint`) and then a report of class k over the same, cause 2. This test doesn't read `PassMap.defined` (Q6). A class whose window isn't greater than *s* is neither rebuilt nor reported. |

**A CELL_HEIGHTS publish over (cx, cy) and (sw, sh).** One publish (`Emit`; `SchedPublish 7 20` in the recordings) of
a 20-byte `LosRadarCellHeights` (`interfaces/los_radar.h`): `h.kind` = `LOS_RADAR_CELL_HEIGHTS` (132), `cell[0]` =
cx, `cell[1]` = cy, `size[0]` = sw, `size[1]` = sh (X15). For a unit at (x, y) with footprint (w, h)
it is over (x − 1, y − 1) and (w + 2, h + 2).

**The ground level of a footprint** of a unit type: over the cells of the footprint whose yard byte has bit 0x08,
*L* is the smallest `Cell.low` and *H* the largest `Cell.high`, with *L* = 255 and *H* = 0 when there are none. When
*H* < *L*, the ground level *G* is (`Game.seaLevel` − `UnitDef.draft`) mod 256 (X25). Otherwise *G* =
*L*.

**The build test of a unit type at a cell, with a type id and a player.** The unit type is a `UnitDef`, the cell a
`Point16` (x, y), the type id a 16-bit value, and the player a `Player` or null. First `g_lastPlaceHeight` and
`g_lastPlaceMetalSum` (the placement record) become 0. Then the result is the first row of this table that applies, in
order. A result of 0 leaves the record as it is at that point (X24):

| Outcome | Result | `g_lastPlaceMetalSum` | `g_lastPlaceHeight` |
| --- | --- | --- | --- |
| Off the map: x < 1, or y < 1, or x + *fx* ≥ `Game.width`, or y + *fz* ≥ `Game.height` | 0 | 0 | 0 |
| a player is given and the site is outside the player's explored map (the visibility test, below) | 0 | 0 | 0 |
| a player is given and the visibility word of the site lacks the bit 1 << `Game.playerIndex` (Q7) | 0 | 0 | 0 |
| A cell test fails at the k-th cell of the footprint at (x, y) (the cell tests, below) | 0 | the sum of `Cell.metal` of cells 0 to k (Q8) | 0 |
| No cell test fails, and a site test fails (the site tests, below) | 0 | the sum over the whole footprint | 0 |
| Otherwise | 1 | the sum over the whole footprint | the ground level *G* |

*The visibility test* (only when a player is given). Let *h* be `Cell.height` of the cell (x, y).

- *X* = (*fx* + 2x) × 2^19 and *Z* = (*fz* + 2y) × 2^19, each wrapping at 32 bits: the centre of the footprint, in
  fixed point.
- *tx* = whole(*X*) arithmetic shift right by 5, and *ty* = (whole(*Z*) − ⌊*h* / 2⌋) arithmetic shift right by 5
  (X27).
- The site is inside the explored map when *tx* < `Player.exploredWidth` and *ty* < `Player.exploredHeight`, both
  compared as unsigned 32-bit (a negative *tx* or *ty* is outside).
- The site's index is *i* = `Player.exploredWidth` × *ty* + *tx*, and its visibility word is element *i* of
  `Game.visibilityMask` (X29).
- *S*, "the site is seen", is then: with map flag 2 set, 1 when byte *i* of `Player.explored` isn't 0, else 0; with
  map flag 2 clear, 1.

With no player, *S* is 1 and there's no visibility test.

*The cell tests.* For each cell of the footprint at (x, y) in order, with *m* its yard byte: its `Cell.metal` is added
to `g_lastPlaceMetalSum` (X26), and then the cell fails when any of these holds:

1. *m* has bit 0x01, `Cell.flags` has bit 1 (mask 2), and *S* is 1;
2. *m* has a bit of 0x06, `Cell.unit` isn't 0, `Cell.unit` isn't the type id (both as 16-bit patterns), and *S* is 1;
3. *m* has bit 0x20 and the cell is blocked by a feature;
4. *m* has bit 0x40, the cell has a feature record, and that record's `indestructible` bit is set.

*The site tests.* With *L*, *H* and *G* of the footprint at (x, y) (the ground level), *K* the largest `Cell.high`
over the cells whose yard byte has bit 0x10 (0 when there are none), and *sea* = `Game.seaLevel`, the site fails when
any of these holds:

1. some cell's yard byte has bit 0x80, and no cell whose yard byte has bit 0x80 has a feature record with its
   `geothermal` bit set;
2. *H* ≥ *L* and *H* − *L* > `UnitDef.maxslope`;
3. *K* > *G*, compared as unsigned bytes;
4. *L* < *sea* − `UnitDef.maxwaterdepth`;
5. max(*H*, *K*) > *sea* − `UnitDef.minwaterdepth`.

### `Apply(event, size)`

`event` is the address of the event in the module's memory, `size` its size in bytes. The result is the outcome, 1 or
0. Every outcome 0 below writes nothing and calls no import. The checks, in this order, before any kind's effects:

| Condition | Outcome |
| --- | --- |
| `size` < 8 (unsigned) | 0 |
| `size` < the payload size of `h.kind` (unsigned): 12 for `MAP_SET_YARD` (`MapSetYard`), 24 for `MAP_PASS_MAPS` (`MapPassMaps`), 12 for `MAP_LOAD_PATH_LOCK_STAMP` (`MapPathLockStamp`), 12 for `MAP_SET_SEA_LEVEL` (`MapSetSeaLevel`), 16 for `MAP_SET_EDGE` (`MapSetEdge`), 32 for kind 176 (`MapCellsChanged`), 8 for any other kind | 0 |
| the kind is `MAP_PASS_MAPS`, `MAP_SET_SEA_LEVEL` or `MAP_SET_EDGE` | that kind's effects, below (`h.unit` isn't read) |
| `h.unit` is 0 | 0 |
| the kind is one of `MAP_ADD_UNIT`, `MAP_REMOVE_UNIT`, `MAP_CLEAR_UNIT`, `MAP_RECLAIM_FOOTPRINT`, `MAP_SET_YARD`, `MAP_BUCKET_UNLINK`, `MAP_BUCKET_PREPEND`, `MAP_LOAD_PATH_LOCK_STAMP` | that kind's effects, below, for the unit at block offset `h.unit` |
| any other kind (167, 176, any value outside `MapEventKind`) | 0 |

The kinds are the `MapEventKind` values of `interfaces/map.h`. Bytes of the event beyond its payload aren't read
(X4). "The unit" below is the unit `h.unit` names, *u* its id (`Unit.id`), (x, y) its `Unit.cell` and (w, h)
its `Unit.footprint`.

#### `MAP_ADD_UNIT` (160)

Outcome 1.

1. If `Unit.motion` isn't null, the unit's stamp becomes `Game.ticks`. This holds in every case below.
2. **Off the map:** if x < 0, or y < 0, or x + w ≥ `Game.width`, or y + h ≥ `Game.height` (X9), the unit
   is filed in the off-map bucket. Nothing else changes (Q9).
3. **On the map:** otherwise the unit is filed in the bucket of its position (`Unit.pos`). Then, by the unit's flags,
   for each cell of its footprint, read from `Game.cells` directly (the k-th cell):

| `Unit.flags` | Each cell | Imports |
| --- | --- | --- |
| structure bit set | if yard map byte k has the unit's yard bit: a take of the cell's `Cell.unit` by the unit. Whether or not byte k has the yard bit, and whatever the take did: if yard map byte k has bit 0x01, `Cell.flags` gets bit 1 (OR 2). A cell whose byte lacks the yard bit keeps its `Cell.unit` (Q10). | a CELL_HEIGHTS publish over (x − 1, y − 1) and (w + 2, h + 2), then a rebuild and report of every defined class over (`Unit.cell`, `Unit.footprint`), cause 2 |
| structure bit clear, layer 1 | a take of `Cell.unit` | none |
| structure bit clear, layer 2 | a take of `Cell.unit2` | none |
| structure bit clear, layer 0 or 3 | nothing | none |

`MAP_ADD_UNIT` doesn't test or change the claim pending bit, and makes no `los_radar.GetMapCell` import.

#### `MAP_REMOVE_UNIT` (161)

Outcome 1, and the removal of the unit. **The removal of a unit** is these writes and imports:

1. **The cells,** only when `Unit.spatialBucket` differs from `Game.overflowBucket` (the two stored offsets compared:
   a null bucket differs from a non-null off-map bucket). For each cell of the footprint, read from `Game.cells` (the
   k-th):

| `Unit.flags` | Each cell | Imports |
| --- | --- | --- |
| structure bit set | `Cell.unit` becomes 0 if it is *u*; if yard map byte k has bit 0x01, `Cell.flags` loses bit 1 (AND 0xfd), whoever holds the cell | then a CELL_HEIGHTS publish over (x − 1, y − 1) and (w + 2, h + 2) |
| structure bit clear, layer 1 | `Cell.unit` becomes 0 if it is *u* | none |
| structure bit clear, layer 2 | `Cell.unit2` becomes 0 if it is *u* | none |
| structure bit clear, layer 0 or 3 | nothing | none |

   A cell held by another unit keeps its holder. A unit in the off-map bucket changes no cell and publishes no
   CELL_HEIGHTS. A removal whose footprint has a cell outside the map (*x* or *y* below 0, *x* ≥ `Game.width` or
   *y* ≥ `Game.height`) while the unit's bucket isn't the off-map bucket (a null bucket, after a clear or before
   any add) is outside the contract: no recording makes one, and a module may behave in any way for those cells.
2. **The flags:** the unit's claim pending bit becomes 0. If its overlapped bit was set, it becomes 0 too, and then the
   units overlapping (x, y, w, h) re-claim. A clear overlapped bit means no re-claim and no `los_radar.GetMapCell`
   import.
3. **The stamp update** of the unit.

The import order: the CELL_HEIGHTS publish (step 1), then the `los_radar.GetMapCell` imports of the re-claim in its
order (step 2), then the reports of step 3.

#### `MAP_CLEAR_UNIT` (162)

Outcome 1, the removal of the unit (its three steps, with the same import order), and also:

- if `Unit.carrier` is null and `Unit.spatialBucket` isn't null, the unit is unlinked from that bucket's chain (the
  off-map bucket as well as a bucket of the grid), and its `Unit.cargoNext` becomes null;
- `Unit.spatialBucket` becomes null, whatever the carrier.

A carried unit keeps its `Unit.cargoNext` (Q11). The unlink adds no import.

#### `MAP_RECLAIM_FOOTPRINT` (163)

Outcome 1.

1. The unit's claim pending bit becomes 1.
2. The units overlapping (`Unit.cell`, `Unit.footprint`) re-claim. The unit itself is one of them when it is in a
   bucket the re-claim visits (or carried by a unit in one) and its footprint isn't empty, so it makes its own claim in
   its turn. When it is in no visited bucket, its claim pending bit stays 1.
3. A rebuild and report of every defined class over (`Unit.cell`, `Unit.footprint`), cause 2.

Imports: the `los_radar.GetMapCell` imports of the re-claim in its order, then the reports.

#### `MAP_SET_YARD` (164)

`event` is a `MapSetYard`; let *open* be its `open` (int).

| Condition, in this order | Outcome | Effects |
| --- | --- | --- |
| `SimGlobals.g_quirks.skipEmptyYardmap` isn't 0 and `UnitDef.yardmap` of the unit's type is null | 0 | none |
| the footprint isn't clear for *open* (below) | 0 | none |
| otherwise | 1 | below |

**The footprint is clear for *open*** when all of these hold:

- x ≥ 1, y ≥ 1, x + w < `Game.width` and y + h < `Game.height`, with x + w and y + h wrapped to 16 bits
  (X10, Q9);
- no cell of the footprint whose yard map byte has 0x02 (when *open* ≠ 0) or 0x04 (when *open* = 0) has a `Cell.unit`
  other than 0 and other than *u*.

**Effects of outcome 1:**

1. `Unit.bit2` becomes *open* AND 1 (Q12, X14). The other bits of `Unit.cobStateFlags` keep their values.
2. The unit's claim pending bit becomes 1, and then the claim of the unit is made, with the new `Unit.bit2`: the claim
   always runs.
3. A rebuild and report of every defined class over (`Unit.cell`, `Unit.footprint`), cause 2.

Imports: the one `los_radar.GetMapCell` of the claim, then the reports.

With `skipEmptyYardmap` 0 (3.1c) and a null yard map, an outcome whose bounds test passes reads the yard map through a
null offset. That case isn't defined by this spec (`docs/scenario-format.md`, `quirks_reach`,
`units.skip-empty-yardmap`). The bounds test failing still gives outcome 0.

#### `MAP_BUCKET_UNLINK` (165)

| Condition | Outcome | Effects |
| --- | --- | --- |
| `Unit.spatialBucket` is null | 0 | none |
| the unit isn't in the chain of its bucket | 0 | none |
| otherwise | 1 | the unit is unlinked from its bucket's chain, and its `Unit.cargoNext` becomes null. `Unit.spatialBucket` keeps its value. |

`Unit.carrier` isn't read.

#### `MAP_BUCKET_PREPEND` (166)

| Condition | Outcome | Effects |
| --- | --- | --- |
| `Unit.spatialBucket` is null | 0 | none |
| the unit is already in the chain of its bucket | 0 | none |
| otherwise | 1 | `Unit.cargoNext` becomes the bucket's `head` (null if the chain was empty), and the `head` becomes the unit. `Unit.spatialBucket` keeps its value. |

`Unit.carrier` isn't read.

#### `MAP_PASS_MAPS` (168)

`event` is a `MapPassMaps`.

| Condition | Outcome | Effects |
| --- | --- | --- |
| `size[0]` < 0 or `size[1]` < 0 (signed 32-bit) | 0 | none |
| otherwise | 1 | below |

Let the cell be (`cell[0]`, `cell[1]`) and the size (`size[0]`, `size[1]`), each taken as its low 16 bits, a signed
16-bit value (X5). A rebuild and report of every defined class over (cell, size), cause 1. With no
defined class, the outcome is still 1.

#### `MAP_LOAD_PATH_LOCK_STAMP` (169)

`event` is a `MapPathLockStamp`.

| Condition | Outcome | Effects |
| --- | --- | --- |
| `Unit.motion` is null | 0 | none |
| otherwise | 1 | the unit's stamp becomes `stamp` (all 32 bits). No pass map changes and nothing is published. |

#### `MAP_SET_SEA_LEVEL` (170)

`event` is a `MapSetSeaLevel`. Outcome 1. `Game.seaLevel` becomes the low 8 bits of `level`. Nothing else changes:
no pass map, no publish.

#### `MAP_SET_EDGE` (171)

`event` is a `MapSetEdge`. Outcome 1. `Game.mapWidth` becomes `Game.baseX` − `x`, and `Game.mapHeight` becomes
`Game.baseY` − `z`, each wrapping at 32 bits (X6).

### `UnitsInRange`

The arguments are `pos`, `range`, `skip`, `ids` and `capacity`. A query. Only `pos.x` and `pos.z` are read. `ids` is the address of an array of unsigned 16-bit ids, `capacity` its
length. It makes no import and writes nothing in the block.

1. **The rectangle.** *x₁* is the clamp to *GW* of the bucket column of `pos.x` − `range`, and *x₂* the same with
   `pos.x` + `range`; *z₁* and *z₂* the same with `pos.z` and *GH*. The sums wrap at 32 bits. The clamp of *v* to a
   size *n* is *v* when 0 ≤ *v* < *n*, 0 when *v* < 0, and *n* − 1 when *v* ≥ *n* (X34).
2. **The radius.** *R* = ⌊`range`² / 2^32⌋ (X35).
3. **The walk.** The buckets of rows z from *z₁* to *z₂*, and in each row the columns x from *x₁* to *x₂* (each
   inclusive, ascending): bucket z × *GW* + x. In each bucket, the units of its chain in order. The off-map bucket is
   never visited.
4. **The test.** A unit *o* is in range when D(*o*) ≤ *R*, a signed comparison, with D(*o*) = ⌊*dx*² / 2^32⌋ +
   ⌊*dz*² / 2^32⌋, where *dx* = `o.pos.x` − `pos.x` and *dz* = `o.pos.z` − `pos.z` (X35).
5. **The output.** In the walk's order, the first `skip` units in range are passed over (none when `skip` ≤ 0). Each
   later unit in range is written as its `Unit.id` to the next element of `ids`, until the count written equals
   `capacity`: at the next unit in range after that, the call returns. The result is the number of ids written. The
   elements of `ids` after them keep their values.

With `capacity` 0 the result is 0. A negative `capacity` never equals the count, so every unit in range is written
(Q13). With *x₁* > *x₂* or *z₁* > *z₂* (which a negative `range` can give) no bucket is visited. A grid with *GW*
or *GH* 0 isn't defined.

### `GetCellMeanHeight(pos)`

Let *cx* = trunc(whole(`pos.x`) / 16) and *cz* = trunc(whole(`pos.z`) / 16), rounded toward zero (X30,
Q14). When 0 ≤ *cx* < `Game.width` and 0 ≤ *cz* < `Game.height`, the result is ⌊(`Cell.low` + `Cell.high`) / 2⌋ of
the cell (*cx*, *cz*) (X32). Otherwise it is −1. Nothing is written.

### `CanBuildAt`

The arguments are `unit`, `cell`, `type` and `player`, in that order. `unit` is the address of a `UnitDef`,
`player` of a `Player` or null. The result and the placement record are those of
the build test of `unit` at `cell`, with the type id `type` and the player `player`. Nothing else is written and no
import is made.

### `CanPlaceUnitFootprint`

The arguments are `unit`, `otherTypeId`, `cell` and `flags`, in that order. `unit` is the address of a
`UnitDef`. Let (x, y) be `cell`. The outcome is the first row that applies, in order:

| Outcome | Result | The placement record |
| --- | --- | --- |
| x < 0, or y < 0, or *fx* + x ≥ `Game.width`, or *fz* + y ≥ `Game.height` | 1 when `flags` is 2, else 0 | unchanged |
| `UnitDef.mobile` is 0 | the result of the build test of `unit` at `cell` with the type id 0 and no player | as that build test leaves it |
| `flags` isn't 1 | 1 | unchanged |
| some cell of the footprint at (x, y) fails the move test, below | 0 | unchanged |
| otherwise | 1 | unchanged |

The second row's build test has its own bounds: a cell with x = 0 or y = 0 gives 0 there, with the record set to 0
(Q15).

**The move test** of a cell, with *sea* = `Game.seaLevel` and *d* = `Cell.high` − `Cell.low` (a signed int): the cell
fails when any of these holds:

1. it is blocked by a feature;
2. `Cell.unit` isn't 0 and isn't `otherTypeId` (unsigned 16-bit);
3. `Cell.low` < *sea* − `UnitDef.maxwaterdepth`;
4. `Cell.high` > *sea* − `UnitDef.minwaterdepth`;
5. *d* > `UnitDef.maxslope`, and either `Cell.low` ≥ *sea* or *d* > `UnitDef.maxwaterslope`.

No import is made.

### `BuildAllPassMaps(limits, count)`

`limits` is an array of `count` `MapMoveLimits`, owned by the caller (the `a limits[i].<field>` lines of the
recordings). Every `index` is 0 to 31 in every call the engine makes; other indexes aren't defined here.

Writes:

- `defined` becomes 0 in all 32 `PassMap` records. Then, for each i from 0 to `count` − 1 in order, the record of
  class `limits[i].index` takes the eight limit fields of `limits[i]` and `defined` 1. A later entry with the same
  index replaces an earlier one.
- For each class k from 0 to 31, in order, whose `defined` is now 1:
  - `width` becomes `Game.width` and `height` becomes `Game.height` (as unsigned 32-bit values).
  - Let *n* = ⌊(`height` + 15) / 16⌋ × `width`, the number of words (X22).
  - `cells` becomes the block offset of a new allocation of *n* × 4 bytes when *n* ≠ 0, and 0 when *n* = 0.
  - The words then hold the full build of class k, every bit of them (the full build).
- The other records (`defined` 0) keep `width`, `height`, `cells` and their limits as they were (Q16). `lastTick`
  of every record is unchanged.
- `Game.progress_38d73` becomes 100 (X23).

Imports, for each class k with `defined` 1, in class order, and nothing else:

1. When `cells` wasn't 0 before: the free service with the old words (their address). A null `cells` frees nothing.
2. When *n* ≠ 0: the allocation service with *n* × 4 bytes.
3. The allocation service with the scratch size *S* + 3 bytes (X22), and then the free service with that
   same block. This pair is made for every defined class, *n* = 0 included. What the scratch block holds isn't part of
   this spec: the module may use its *S* + 3 bytes as working memory during the build.

### `FreePassMaps()`

The engine's slot for now (section 2, ozgb/btm#43): the module doesn't export it. This subsection states what the engine's slot does, which the module sees as block data; its "imports" are the engine's calls of the services.

For each class k from 0 to 31, in order: when `cells` isn't 0, the free service with the words (their address). Then,
for all 32 records, `width` and `height` become 0, `cells` 0 and `defined` 0. The limit fields and `lastTick` keep
their values. The imports are those frees only, in class order.

### `LoadMetalPlotmap(file)`

The engine's slot (section 2): the module doesn't export it. This subsection states what the engine's slot does with the block; its "imports" are the engine's calls, and its buffers are the engine's.

Let *S* = `Game.width` × `Game.height` (X42). The buffer is the engine's, at least *S* bytes.

| Outcome | Imports, in order | Written |
| --- | --- | --- |
| No account | `files.BankOpenAccount(file, "Metal")` = 0 | nothing |
| No box | `BankOpenAccount` ≠ 0, `files.BankOpenNamedBox(file, "Plotmap")` = 0 | nothing |
| Wrong size | both ≠ 0, `files.BankGetBoxSize(file)` ≠ *S* | nothing |
| Short read | both ≠ 0, `BankGetBoxSize` = *S*, `files.BankReadBox(file, buffer, S)` = *r* with *r* < *S* as unsigned values | nothing |
| Good | as above, with *r* ≥ *S* as unsigned values | `Cell.metal` of cell *i* becomes byte *i* of the buffer, for *i* from 0 to *S* − 1 |

A negative *r* counts as a full read (Q17).

### `LoadPlayerFeaturesPlotmap(file)`

The engine's slot (section 2): the module doesn't export it. This subsection states what the engine's slot does with the block; its "imports" are the engine's calls, and its buffers are the engine's.

Let *S* = ⌊`Game.width` × `Game.height` / 2⌋, truncating toward zero (X42). The outcomes, imports and
buffer are those of this table, with *r* the result of `files.BankReadBox`:

| Outcome | Imports, in order | Written |
| --- | --- | --- |
| No account | `files.BankOpenAccount(file, "PlayerFeatures")` = 0 | nothing |
| No box | `BankOpenAccount` ≠ 0, `files.BankOpenNamedBox(file, "Plotmap")` = 0 | nothing |
| Wrong size | both ≠ 0, `files.BankGetBoxSize(file)` ≠ *S* | nothing |
| Short read | both ≠ 0, `BankGetBoxSize` = *S*, `files.BankReadBox(file, buffer, S)` = *r* with *r* < *S* as unsigned values | nothing |
| Good | as above, with *r* ≥ *S* as unsigned values (Q17) | below |

In the good outcome, for *i* from 0 to *S* − 1, with *v* byte *i* of the buffer (X41):

- `Cell.plotFeature4` of cell 2*i* becomes ⌊*v* / 16⌋;
- `Cell.plotFeature4` of cell 2*i* + 1 becomes *v* mod 16.

Only those 4 bits of `Cell.flags` change: bits 0 to 2 and bit 7 keep their values.

### `SaveMetalPlotmap(file)`

The engine's slot (section 2): the module doesn't export it. This subsection states what the engine's slot does with the block; its "imports" are the engine's calls, and its buffers are the engine's.

Let *S* = `Game.width` × `Game.height`. The imports, in this order, every result ignored:

1. `files.BankOpenAccount(file, "Metal")`
2. `files.BankOpenNamedBox(file, "Plotmap")`
3. `files.BankWriteBox(file, buffer, S)`, where byte *i* of the buffer is `Cell.metal` of cell *i* of `Game.cells`, for
   *i* from 0 to *S* − 1.

Nothing in the block changes.

### `SavePlayerFeaturesPlotmap(file)`

The engine's slot (section 2): the module doesn't export it. This subsection states what the engine's slot does with the block; its "imports" are the engine's calls, and its buffers are the engine's.

Let *S* = ⌊`Game.width` × `Game.height` / 2⌋, truncating toward zero. Byte *i* of the buffer, for *i* from 0 to
*S* − 1, is (`plotFeature4` of cell 2*i*) × 16 + (`plotFeature4` of cell 2*i* + 1): the even cell's 4-bit field in the
high half, the odd cell's in the low half (X41). The imports, in this order, every result ignored:

1. `files.BankOpenAccount(file, "PlayerFeatures")`
2. `files.BankOpenNamedBox(file, "Plotmap")`
3. `files.BankWriteBox(file, buffer, S)`

Bit 7 of `Cell.flags` isn't saved, and nor is the last cell when `Game.width` × `Game.height` is odd (Q18).
Nothing in the block changes.

### `SnapWorldPosToFootprint(unit, pos)`

`unit` is the address of a `UnitDef`. When `UnitDef.mobile` isn't 0, nothing changes. Otherwise:

- the site is (*cx*, *cz*) = (snap(`pos.x`, *fx*), snap(`pos.z`, *fz*)) (X28);
- `pos.x` becomes (*fx* + 2*cx*) × 2^19 and `pos.z` becomes (*fz* + 2*cz*) × 2^19, each wrapping at 32 bits;
- `pos.y` becomes *F* × 65536, where *F* is 0 when *cx* < 1, or *cz* < 1, or *cx* + *fx* ≥ `Game.width`, or *cz* +
  *fz* ≥ `Game.height` (Q15), and otherwise the ground level *G* of the footprint at (*cx*, *cz*).

Nothing else is written; the placement record is unchanged. No import is made.

### `CanPlaceFootprintAt(unit, pos)`

`unit` is the address of a `Unit`. Here the footprint is `Unit.footprint` (*ux* columns, *uz* rows) and the limits come
from the unit's type, `Unit.def`.

1. The site is (*a*, *c*) = (snap(`pos.x`, *ux*), snap(`pos.z`, *uz*)). `pos.y` isn't read.
2. When *a* < 0, or *c* < 0, or *a* + *ux* ≥ `Game.width`, or *c* + *uz* ≥ `Game.height`: the result is 0.
3. Let *j* = ((*c* asr 1) + (*ux* asr 2)) × (`Game.width` asr 1) + (*a* asr 1) + (*ux* asr 2), where asr is an
   arithmetic shift right (Q19). When element *j* of `Game.visibilityMask` lacks the bit 1 << `Unit.playerIndex`,
   the result is 1 (Q20, X29).
4. Otherwise let *lo* = *sea* − `UnitDef.maxwaterdepth` and *hi* = *sea* − `UnitDef.minwaterdepth`, with *sea* =
   `Game.seaLevel`. When *lo* < *sea*, `UnitDef.flags1` has bit 0x800 and doesn't have bit 0x200000, *lo* becomes
   *sea*. The result is 0 when any cell of the footprint at (*a*, *c*) of size (*ux*, *uz*) (every cell: the yard map
   isn't read) has any of:
   - it is blocked by a feature;
   - `Cell.flags` bit 1 (mask 2) set;
   - `Cell.unit` not 0 and not `Unit.id`;
   - `Cell.unit2` not 0 and not `Unit.id`;
   - `Cell.low` < *lo*, or `Cell.high` > *hi*;
   - `Cell.high` − `Cell.low` > `UnitDef.maxslope` (a signed int compared with the unsigned byte).

   Otherwise the result is 1.

Nothing is written and no import is made.

### `IsPadSlotFree(pad, id)`

`pad` is the address of a `Unit`.

| Outcome | Result |
| --- | --- |
| `pad.carrier` isn't null | 0 |
| some unit of the chain that starts at `pad.cargo` and follows `Unit.cargoNext` to null has `transportPiece`, read as a signed byte (−128 to 127), equal to `id` (a 32-bit compare, X33) | 0 |
| otherwise | 1 |

Nothing is written and no import is made.

### `PickRandomReclaimableResourcesInRadius`

The arguments are `pos`, `range`, `energy`, `energyAmount`, `metal` and `metalAmount`. Synchronous. `pos` is the address of a `Vec3`, `range` a 16.16 fixed-point distance. `energy` and `metal` are each the
address of a 32-bit pointer to a caller's `Vec3`; `energyAmount` and `metalAmount` the addresses of 32-bit floats. All
are linear addresses in the module's memory (ABI 7.1), none a block offset.

Let *w* = `range` arithmetic shift right by 16 (its whole part, signed), the capacity *c* = ⌊*w*² / 256⌋, and *h* =
`range` / 2 truncated toward zero (X36).

**The scan points.** For z = `pos.z` − *h*, then z + 0x300000, and on while z ≤ `pos.z` + *h*; and in each, for x =
`pos.x` − *h*, then x + 0x300000, and on while x ≤ `pos.x` + *h*: the point (x, *y*, z), rows of z outermost. Its *y*
isn't specified (Q21).

**A point's feature.** `los_radar.GetMapCellAtPosition` with the point. When the result is 0 the point is skipped.
Otherwise `features.GetCellFeature` with that cell gives *k*, an unsigned 16-bit value. The point is skipped when *k* ≥
0xfffb. Otherwise the feature is element *k* of `Game.features`. It is skipped unless both its `reclaimable` (0x80) and
`autoreclaimable` (0x100) bits are set. A qualifying feature adds:

- to the energy list, the entry (the point, `Feature.energy`), when `Feature.energy` isn't 0.0;
- to the metal list, the entry (the point, `Feature.metal`), when `Feature.metal` isn't 0.0.

The tests are float comparisons: −0.0 counts as 0.0, and a NaN as not 0.0 (X38). A feature can be in both lists. The lists
keep the scan's order, from entry 0, each entry 16 bytes in its allocation: the point's 12 bytes, then the value
(Q22).

**The pick of a list** with *n* > 0 entries: three draws *i*₁, *i*₂, *i*₃ (X37). Starting from the index 0 and
the value 0.0, each draw in turn replaces them with its index and its entry's value when that value is greater than the
value held, a float comparison (false with a NaN). The pick is the index held after the third: the first draw with the
greatest value above 0.0, or entry 0 when no drawn value is above 0.0 (Q23).

**Outputs.**

| List | Written |
| --- | --- |
| energy list not empty | the 12 bytes of the `Vec3` at the address held in `*energy` become the picked entry's point; `*energyAmount` becomes its value |
| energy list empty | `*energy` (the pointer) becomes 0; `*energyAmount` becomes +0.0 |
| metal list not empty | the `Vec3` at `*metal` and `*metalAmount`, the same way |
| metal list empty | `*metal` becomes 0; `*metalAmount` becomes +0.0 |

The result is 1 when either list isn't empty, else 0. Nothing in the block changes, but for the two allocations
(Q22).

**The imports, in order:**

1. the allocation service with *c* × 16 bytes (the metal list), then the allocation service with *c* × 16 bytes (the
   energy list);
2. for each scan point in order: `los_radar.GetMapCellAtPosition`, then, when it gave a cell,
   `features.GetCellFeature`;
3. when the energy list isn't empty, its three draws; then, when the metal list isn't empty, its three draws;
4. the free service with the energy list, then with the metal list. A list whose allocation gave 0 makes no free.

### `AllocateBuckets()`

The engine's slot for now (section 2, ozgb/btm#43): the module doesn't export it. This subsection states what the engine's slot does, which the module sees as block data; its "imports" are the engine's calls of the services.

Let *GW* = (`Game.baseX` × 65536 + 0x7fffff) arithmetic shift right by 23 and *GH* = (`Game.baseY` × 65536 + 0x7fffff)
arithmetic shift right by 23, and let *N* = *GW* × *GH* rounded up to a multiple of 8 (X39).

The imports, in this order:

1. The allocation service with 10 bytes: the new off-map bucket.
2. The free service with the address of the previous grid, when `Game.cells_1429f` isn't 0 before the call. No import
   is made when it is 0.
3. The allocation service with *N* × 10 bytes, when *N* isn't 0. When *N* is 0 no allocation is made.

The results of the allocations are never tested. A result of 0 isn't defined by this spec. The previous off-map bucket,
if there was one, isn't freed (Q24).

What is written:

- `Game.overflowBucket` becomes the new off-map bucket's block offset. All its 10 bytes are 0, except its *flags*,
  which become 0x1f.
- `Grid2.widthFixed` becomes `Game.baseX` × 65536 and `Grid2.heightFixed` becomes `Game.baseY` × 65536, wrapping at 32
  bits. `Grid2.width` becomes *GW*, `Grid2.height` *GH*, `Grid2.count` *N*.
- `Grid2.cells` becomes the new grid's block offset, or 0 when *N* is 0.
- Every byte of the *N* buckets is written, as follows. Bucket (x, z) of the grid:
  - *flags*: the edge bits, 1 when z = 0, 2 when z = *GH* − 1, 4 when x = 0 and 8 when x = *GW* − 1, or-ed together (a
    bucket in a corner has two of them, and a row's or column's both when *GW* or *GH* is 1);
  - *a*: the greatest of `Game.seaLevel` and `Cell.high` of every cell (*cx*, *cy*) of the map (0 ≤ *cx* <
    `Game.width`, 0 ≤ *cy* < `Game.height`) with ⌊*cy* / 8⌋ × *GW* + ⌊*cx* / 8⌋ equal to the bucket's element number
    (X40);
  - *b*: when *GW* ≥ 2 and *GH* ≥ 2, the greatest *a* of the buckets (x′, z′) of the grid with |x′ − x| ≤ 1 and
    |z′ − z| ≤ 1: the bucket itself and its up to 8 neighbours. When *GW* = 1 or *GH* = 1, every *b* is 0 (Q25);
  - `head`: 0.

  A padding bucket has *a* = `Game.seaLevel`, *b* 0, *flags* 0 and `head` 0.

When *N* is 0 while *GW* or *GH* isn't, the call isn't defined by this spec.

### `FreeBuckets()`

The engine's slot for now (section 2, ozgb/btm#43): the module doesn't export it. This subsection states what the engine's slot does, which the module sees as block data; its "imports" are the engine's calls of the services.

- The free service with the address of the grid, when `Game.cells_1429f` isn't 0; then the free service with the
  address of the off-map bucket, when `Game.overflowBucket` isn't 0. Those are the imports, in that order. A 0 makes
  no import.
- `Grid2.width`, `Grid2.height`, `Grid2.count`, `Grid2.cells` and `Game.overflowBucket` become 0.
- `Grid2.widthFixed` and `Grid2.heightFixed` keep their values (Q26).

### `GetGroundHeight(pos)`

Let *X* = whole(`pos.x`) and *Z* = whole(`pos.z`) (X27). Let *cx* = *X* asr 4, *cz* = *Z* asr 4, and the
fractions *u* = *X* mod 16 and *v* = *Z* mod 16 (X31). `pos.y` isn't read.

- When *cx* < 0, or *cx* + 1 ≥ `Game.width`, or *cz* < 0, or *cz* + 1 ≥ `Game.height`: the result is −1 (Q14).
- Otherwise, with the `Cell.height` values *a* of the cell (*cx*, *cz*), *b* of (*cx* + 1, *cz*), *c* of (*cx*,
  *cz* + 1) and *d* of (*cx* + 1, *cz* + 1):
  - *north* = *a* + trunc((*b* − *a*) × *u* / 16),
  - *south* = *c* + trunc((*d* − *c*) × *u* / 16),
  - the result is *north* + trunc((*south* − *north*) × *v* / 16) (X30).

Nothing is written and no import is made.

### `PassMapGet(moveClass, x, y)`

Returns the pass value at (x, y) of PassMap `moveClass`: with *w* = `width`, word *w* × ⌊y / 16⌋ + x of its words,
shifted right by 2 × (y mod 16), its low 2 bits. A result of 0 to 3. Nothing is written and nothing is imported.
`defined` isn't looked at.

The engine asks only for a class 0 to 31 whose map was built, and only for a cell inside the class's `width` and
`height`. Other calls aren't defined here.

### `PassMapInBounds(moveClass, x, y)`

Returns 1 when x < `width` and y < `height` of PassMap `moveClass`, both compared as unsigned 32-bit values (a negative
x or y is a large value, so it gives 0), and 0 otherwise. `defined` isn't looked at: a class the last
`BuildAllPassMaps` didn't define answers from the `width` and `height` it still holds (Q16), and gives 0 after
`FreePassMaps` or before any build. Nothing is written and nothing is imported. A `moveClass` outside 0 to 31 isn't
defined here.

### `RefreshMovedUnits(moveClass, unit)`

k is `moveClass`, 0 to 31. The unit always has a motion record; any other call isn't defined here. `defined` of class
k isn't looked at.

Let *t* be `Game.ticks` as an unsigned 32-bit value, *o* the class's window before the call, *s* = *t* − 30 when *t* >
30 and 0 when *t* ≤ 30 (the new window), and *p* the unit's stamp before the call, unsigned (X7).

Writes:

- `lastTick` of class k becomes *s*.
- The rebuilds listed below, each with the window *s*.
- The unit's stamp is *p* again at the end of the call. During the call it is `Game.ticks`: every pass value this call
  computes for a cell that holds the unit takes `Game.ticks` as the unit's stamp, and a synchronous subscriber of the reports reads `Game.ticks` in `UnitMotion.pathLockStamp` (Q27).

The rebuilds and reports, in this order, each report right after its own rebuild and before the next rebuild (the route
to pathfinding is synchronous, `docs/schedule.md`):

1. When *p* < *o* (unsigned): a rebuild of class k over (`Unit.cell`, `Unit.footprint`) of the unit, then a report of
   class k over the same with cause 2.
2. Then, when *s* ≠ *o*, for each unit *m* of the table from `Unit#1` to the unit `Game.unitsEnd` names, both included,
   in index order, that is alive (unit flag 0x10000000), has a motion record, and whose stamp *q* satisfies *o* ≤ *q* <
   *s* (unsigned): a rebuild of class k over (`Unit.cell`, `Unit.footprint`) of *m*, then a report of class k over the
   same with cause 2. The unit given as the argument is never one of these, since its stamp is `Game.ticks` ≥ *s*
   during the call.

The imports are the reports' publishes, in that order, and nothing else.

### `RefreshUnitIfStale(moveClass, unit)`

k is `moveClass`, 0 to 31, and the unit has a motion record (other calls aren't defined here). When the unit's stamp is
below the class's window, compared as unsigned 32-bit values: a rebuild of class k over (`Unit.cell`,
`Unit.footprint`) of the unit, with the window as it is, then a report of class k over the same with cause 2. The unit
is then stale in that rebuild: a cell whose `Cell.unit` is the unit's id has pass value 0. Otherwise nothing happens.
Nothing else is written; the stamp and the window keep their values. The import is that one publish, or none.

### `WriteState(writer)`

The engine's slot (section 2): the module doesn't export it. This subsection states what the engine's slot does with the block.

Writes section 45 through the `StateWriter` the engine gives (`interfaces/state_writer.h`). It reads only, and makes
no import. The calls through the writer are in this order, and their order is what a dump shows.

Let *count* be `Grid2.count` read as an unsigned 32-bit value, taken as 0 when `Game.cells_1429f` is 0 or *count* is
above 2^20 (1048576).

| Call | Field name | Index | Size or kind | Value |
| --- | --- | --- | --- | --- |
| `Begin` | section 45, name `"mapState"` | | | |
| `Int` | `"spatial.count"` | 0 | 4 | *count* |
| for each bucket *r* from 0 to *count* − 1, these three: | | | | |
| `Int` | `"spatial[%u].a"` | *r* | 1 | *a* of bucket *r* |
| `Int` | `"spatial[%u].flags"` | *r* | 4 | *flags* of bucket *r* |
| `Handle` | `"spatial[%u].first"` | *r* | `STATE_HANDLE_UNIT` (2) | `head` of bucket *r*, the stored block offset as it is |
| `Int` | `"overflow.present"` | 0 | 1 | 1 when `Game.overflowBucket` isn't 0, else 0 |
| then, only when it is 1: | | | | |
| `Int` | `"overflow.a"` | 0 | 1 | *a* of the off-map bucket |
| `Int` | `"overflow.flags"` | 0 | 4 | *flags* of the off-map bucket |
| `Handle` | `"overflow.first"` | 0 | `STATE_HANDLE_UNIT` (2) | `head` of the off-map bucket |

*b* isn't written (it is in trace section 14, `docs/trace-format.md`, section 8.5). No `Run` and no `Bytes`
call is made.

### `PathLockStamp(unit)`

The engine's slot (section 2): the module doesn't export it. This subsection states what the engine's slot does with the block.

Returns the unit's stamp (as an int), or 0 when `Unit.motion` is null. Nothing is written and nothing is imported.

## 5. Exactness

- **X1, widths.** Fields are 32-bit two's complement unless said otherwise, and arithmetic is signed 32-bit.
  `Cell.height`, `high`, `low`, `metal`, `offsetX`, `offsetY`, `Game.seaLevel`, `UnitDef.draft`, `maxslope`,
  `maxwaterslope`, `Unit.playerIndex` and the yard map bytes are unsigned bytes. `Cell.unit`, `unit2`, `feature` and
  `Unit.id` are unsigned 16-bit, compared as such; the unit of id *v* is element *v* (0 to 65535) of `Game.units`.
  `UnitDef.footprintx`, `footprintz`, `maxwaterdepth`, `minwaterdepth`, the `PassMap` shorts, `Unit.cell`,
  `Unit.footprint` and every `Point16` field are signed 16-bit, sign-extended to 32 bits before any arithmetic.
- **X2, block offsets.** A stored pointer (`Unit.spatialBucket`, `Unit.cargoNext`, a bucket's `head`,
  `Grid2.cells`, `Game.overflowBucket`, `PassMap.cells`) is a block offset: the address minus `g_blockBase`, and 0 for
  null (ABI 7.1). An allocation's result is stored as its address minus `g_blockBase`; a free takes the address,
  `g_blockBase` plus the stored offset. A unit's block offset is `Game.units` plus 280 × its index. `h.unit` is a block
  offset, 0 for null. `WriteState` passes a `head` to `Handle` as the stored offset, unchanged.
- **X3, what an allocation holds.** `services.SimAlloc` gives zeroed bytes (ABI 9.3). The module writes every bit
  of the pass map words (the full build, with 0 in the bits of rows `height` and up). The grid and the off-map bucket
  are written by the engine's `AllocateBuckets` (section 2), and the module reads them as block data; section 4 states
  their bytes. The scratch block of `BuildAllPassMaps` and the two reclaim lists are freed in the call that allocates
  them, and their bytes aren't part of this spec.
- **X4, the event size.** `size` is compared as an unsigned 32-bit integer, first with 8 and then with the
  kind's payload size (12, 16, 24 or 32, else 8). The negative test of `MAP_PASS_MAPS` is on the signed 32-bit
  `size[0]` and `size[1]`, before truncation, so a size of 65536 passes and becomes 0.
- **X5, the `MAP_PASS_MAPS` rectangle.** `cell[0]`, `cell[1]`, `size[0]`, `size[1]` are each cut to their
  low 16 bits and read as signed 16-bit values; the rebuilds and the reports use those values.
- **X6, sea level and edge.** `MAP_SET_SEA_LEVEL` keeps the low 8 bits of `level` (an unsigned byte).
  `MAP_SET_EDGE` subtracts in 32-bit two's complement, wrapping.
- **X7, stamps and windows.** The stamp is the 32-bit `UnitMotion.pathLockStamp`, written as `Game.ticks`
  (the same 32 bits); `MAP_LOAD_PATH_LOCK_STAMP` stores `stamp` unchanged. Stamps and windows are compared as unsigned
  32-bit values everywhere: a class is refreshed when its window is above the stamp. `RefreshMovedUnits` takes
  `Game.ticks` as unsigned: *s* = *t* − 30 for *t* > 30, else 0.
- **X8, the bucket of a position.** The bucket columns are arithmetic shifts right by 23 of the signed
  32-bit coordinates (a negative coordinate gives a negative column). The element number *bz* × *GW* + *bx* is a
  32-bit product and sum, wrapping, and it isn't clamped to the grid. The stored `Unit.spatialBucket` is the block
  offset `Game.cells_1429f` plus 10 × the element number.
- **X9, `MAP_ADD_UNIT` bounds.** x + w and y + h are 32-bit sums of sign-extended values, compared signed
  with `Game.width` and `Game.height`.
- **X10, `MAP_SET_YARD` bounds.** x + w and y + h are wrapped to 16 bits and read as signed 16-bit values
  before the signed compare with `Game.width` and `Game.height`; the lower bound is 1, not 0.
- **X11, the re-claim.** The overlap tests are signed 32-bit compares of the sign-extended fields, with *R*
  and *B* 32-bit sums. ⌊*L* / 8⌋ (and the others) are arithmetic shifts right by 3 of the sign-extended values. The
  skip tests compare *bx* and *by* as unsigned 32-bit values with the unsigned *GW* and *GH*.
- **X12, yard map bytes and `Cell.flags`.** Yard map bytes are tested with the masks of section 3.
  `Cell.flags` is a byte: OR 2 sets bit 1, AND 0xfd clears it, and the other 7 bits keep their values.
- **X13, the player test of a take.** `Player.active` (int) is not 0, and `Player.type` (an unsigned byte)
  equals 3.
- **X14, `Unit.bit2`** is one bit: `MAP_SET_YARD` stores *open* AND 1, while its clear test chooses 0x02 for any
  *open* not 0.
- **X15, CELL_HEIGHTS fields.** x − 1, y − 1, w + 2 and h + 2 are each wrapped to 16 bits (signed), then
  sign-extended to the 32-bit fields. The 20 bytes of the payload are exactly those five ints.
- **X16, pass values** are 0, 1 or 3, two bits a cell, at shift 2 × (y mod 16) of word ⌊y / 16⌋ ×
  `width` + x. A write replaces only those two bits.
- **X17, heights and slope.** The depth limits are signed 16-bit, sign-extended: `Game.seaLevel` −
  `maxWaterDepth` and `Game.seaLevel` − `minWaterDepth` (and the `UnitDef` forms) are signed 32-bit, and the cell bytes
  are compared with them as signed ints. "Under water" compares the two bytes, `Cell.low` < `Game.seaLevel`. The slope
  of a pass value is (`Cell.high` − `Cell.low`) truncated to an unsigned byte: a cell with `high` below `low` has a
  slope of 256 minus the difference, compared unsigned with the unsigned byte limits.
- **X18, features.** `Cell.feature` is unsigned 16-bit and is compared with the signed `Game.featureCount`
  as a value 0 to 65535. The owner cell of 0xfffe is found with the unsigned bytes `offsetY`, `offsetX` and the signed
  `Game.width`. Its feature type is used with no test against `Game.featureCount`: a type at or past the count reads
  whatever that element of `Game.features` holds.
- **X19, bounds of the footprint value** use `Game.width` and `Game.height` as signed ints, with ≥ (x + *fx*
  ≥ *W* gives 0). The full build uses the class's `width` and `height` with "on the map".
- **X20, the rebuild rectangle.** Sums of sign-extended 16-bit values in 32-bit ints. The clip of right
  and bottom is an unsigned comparison with `width` and `height` (a negative right or bottom becomes `width` or
  `height`); left < right and top < bottom are signed.
- **X21, the report rectangle.** Left and top clipped at 0 (signed); right and bottom clipped at `width` −
  1 and `height` − 1 by a signed comparison with `width` and `height` as ints. It isn't the rebuild's rectangle: its
  right and bottom are included, and its clip is signed. `classes` is 1 shifted left by the class index, an unsigned
  32-bit value.
- **X22, word count and scratch size.** *n* = ⌊(`height` + 15) / 16⌋ × `width`, unsigned 32-bit, wrapping;
  the words take *n* × 4 bytes. The scratch size *S* is `width` + *fx* when `width` + *fx* > *fz* + `height`, else
  *fz* + `height`, with *fx*, *fz* sign-extended and the sums and the comparison unsigned 32-bit; the scratch block is
  *S* + 3 bytes.
- **X23, progress.** `Game.progress_38d73` is 100 at the end of the call.
- **X24, placement comparisons.** The bounds tests, *sea* − `maxwaterdepth`, *sea* − `minwaterdepth`,
  *H* − *L*, *d* and `Cell.high` − `Cell.low` are signed 32-bit (a cell with `high` < `low` gives a negative
  difference, which passes a slope test). *K* > *G* is between unsigned bytes. `Cell.unit` against the type id compares
  the low 16 bits of each. The explored-map bounds test is unsigned 32-bit.
- **X25, the ground level's byte.** (`Game.seaLevel` − `UnitDef.draft`) is truncated to 8 bits: a draft
  above the sea level gives 256 + *sea* − *draft*. *G* and `g_lastPlaceHeight` are then 0 to 255.
- **X26, the metal sum** adds unsigned bytes into a 32-bit int, starting from 0 in each build test, one cell
  at a time in footprint order, a cell's metal before its cell tests.
- **X27, whole(*v*)**: bits 16 to 31 of a 32-bit *v*, as a signed 16-bit value. For *v* ≥ 0 it is ⌊*v* /
  65536⌋. "asr" is an arithmetic shift right (it rounds toward minus infinity on negatives). The multiplications by
  2^19 wrap at 32 bits.
- **X28, snap(*p*, *f*)**, for a coordinate *p* (32-bit) and a footprint size *f*: (*p* − *f* × 2^19 + 2^19),
  wrapping at 32 bits, arithmetic shift right by 20, then the low 16 bits as a signed 16-bit value. It is the cell of
  the footprint's left or top edge, rounded to the nearest cell (a half rounds up).
- **X29, visibility words.** *ty* subtracts ⌊`Cell.height` / 2⌋ (never negative) from the whole z before the
  shift. The word of the build test is element *i* of `Game.visibilityMask` with the stride `Player.exploredWidth`;
  that of `CanPlaceFootprintAt` uses the stride `Game.width` asr 1 and the index *j*. The masks 1 << `Game.playerIndex`
  and 1 << `Unit.playerIndex` are 32-bit with the shift count taken mod 32, tested against the 16-bit word: an index
  from 16 to 31 never matches, and an index of 32 or more tests bit (index mod 32).
- **X30, trunc** in `GetGroundHeight` and `GetCellMeanHeight` is signed division rounding toward zero: (*b* −
  *a*) × *u* / 16 with *b* − *a* = −7 and *u* = 5 gives −2, not −3. In `GetCellMeanHeight` a whole part from −15 to −1
  gives cell 0.
- **X31, the fractions** *u* and *v* are the low 4 bits of the whole parts, 0 to 15, also for negative
  whole parts.
- **X32, the mean** is (`low` + `high`) shift right by 1, computed in int (0 to 255).
- **X33, `transportPiece`** is read as a signed byte: 255 is −1, and it equals an `id` of −1, never 255.
- **X34, the clamp.** The bucket column's shift is arithmetic (a negative sum shifts toward −∞: −10485760
  gives −2). The test 0 ≤ *v* < *n* is the unsigned comparison of *v* with *n*.
- **X35, the distance.** *dx* and *dz* are wrapping 32-bit differences, squared as signed 64-bit products,
  each shifted right by 32 (the product is never negative), then added at 32 bits, wrapping. *R* is the 64-bit square
  of `range` shifted right by 32, kept as a signed 32-bit int. `pos.y` and `Unit.pos.y` are never read.
- **X36, the reclaim scan.** *w* is the signed high 16 bits of `range`; *w*² is a 32-bit product; ⌊*w*² / 256⌋
  is exact (*w*² ≥ 0). *h* is the signed division of `range` by 2, rounding toward zero. Each z and x is computed at
  32 bits, and the loop tests z ≤ `pos.z` + *h* and x ≤ `pos.x` + *h* as signed 32-bit comparisons, with the bounds
  computed at 32 bits. A negative `range` gives no scan point. The allocation size *c* × 16 is unsigned. Each
  step of 0x300000 wraps at 32 bits too: when the last z of the scan (or the last x of a row) is above
  0x7fffffff − 0x300000, the step past it wraps to a negative value, which is still within the bound, and the
  scan goes on past the wrap. Such a position is outside the contract: no recording calls the pick at such a
  position, and a module may behave in any way for it.
- **X37, the reclaim draws.** Each draw of the pick is one `services.RandomInt(n)`, with *n* the length of the
  list, giving an index 0 to *n* − 1 (0 for *n* below 2), used unscaled. A list of length 1 still makes its three
  calls. The draws are three for the energy list, then three for the metal list, each list's only when it isn't empty:
  0, 3 or 6 in all. `RandomInt` advances its own generator (`rng.seed` in the recordings), not a game logic stream:
  the pick makes no draw from `rng.main` or any other stream. `rng.seed` is the Park-Miller state that the trace's
  phase row calls `rng` (`docs/trace-format.md`) and `docs/schedule.md` calls the Park-Miller generator.
- **X38, the floats.** The amounts are copied as 32-bit floats, bit for bit. The "not 0.0" tests and the
  pick's "greater" comparisons are IEEE 754 single-precision comparisons. The empty-list amount is +0.0 (0x00000000).
- **X39, the grid size.** `Game.baseX` × 65536 and `Game.baseY` × 65536 wrap at 32 bits. *GW* and *GH* are those
  plus 0x7fffff, wrapping at 32 bits, then an arithmetic shift right by 23, stored as unsigned. *N* = (*GW* × *GH* +
  7) with its low three bits cleared, at 32 bits. The buckets are packed, 10 bytes each, with no padding between them.
- **X40, *a* and *b*.** All comparisons are of unsigned bytes. The map's cells are read in their order in
  `Game.cells`. The bucket number ⌊*cy* / 8⌋ × *GW* + ⌊*cx* / 8⌋ isn't clipped: a map wider than 8 × *GW* cells would
  reach the next row's buckets.
- **X41, the feature plot byte.** The byte saved is ((`plotFeature` of the even cell) × 16 + (`plotFeature`
  of the odd cell mod 16)) mod 256, which is the same as the formula with `plotFeature4`. A load writes ⌊*v* / 16⌋ and
  *v* mod 16 of the unsigned byte *v*.
- **X42, the plot map sizes.** *S* is a 32-bit product (the division by 2 is signed and truncates toward
  zero). The box size test is a signed 32-bit equality. The read test compares the result of `files.BankReadBox` with
  *S* as unsigned 32-bit values.

## 6. Quirks

- **Q1.** For a cell whose feature is 0xfffe, the origin's feature isn't tested against `Game.featureCount`, and
  an origin value from 0xfffb to 0xfffd counts as no feature (not blocked), while the same value on the cell itself
  blocks.
- **Q2.** A take of a cell that already holds the taking unit's own id treats the unit as the other unit. A unit
  whose player isn't of type 3 then gets both its overlapped and its claim pending bits, and keeps the cell. A
  `MAP_SET_YARD` on a structure of a local player therefore leaves the structure with 0x4000000 and 0x8000000 set.
- **Q3.** The bucket of a position isn't clamped: a unit whose footprint is on the map but whose position lies
  outside the grid is filed at whatever element number the formula gives.
- **Q4.** A rebuild and the full build give different values near the right and bottom edges. The full build
  allows a block that ends on the last column or row (value at most 1 there) and gives 3 to a block whose grown block
  ends on the last column or row. The footprint value of a rebuild is 0 for a block that ends on the last column or row
  (x + *fx* ≥ `Game.width`), and at most 1 for one whose grown block ends there. A map rebuilt near those edges
  therefore changes there even when nothing on its cells did.
- **Q5.** A report is published for every rebuild, even when the rectangle is empty or no value changed, and its
  rectangle is clipped differently from the rebuild's.
- **Q6.** The stamp update of a unit with motion refreshes and reports every class index 0 to 31 whose window is
  above its stamp, whether the class is defined or not.
- **Q7.** The build test with a player tests the visibility word for the bit of `Game.playerIndex` (the player
  this machine views), not of the player passed. The explored-map lookup does use the passed player's `explored`,
  `exploredWidth` and `exploredHeight`.
- **Q8.** A failed build test leaves in `g_lastPlaceMetalSum` the metal of the cells it reached, up to and
  including the failing cell, or the whole sum when a site test failed, or 0 for a bounds or visibility failure.
- **Q9.** `MAP_ADD_UNIT` counts a footprint that reaches the last column (x + w = `Game.width`) or the last row as
  off the map, and files the unit in the off-map bucket without touching the cells. The clear test of `MAP_SET_YARD`
  also rejects a footprint in column 0 or row 0, which `MAP_ADD_UNIT` accepts.
- **Q10.** `MAP_ADD_UNIT` of a structure writes the cells whether or not the claim pending bit is set, and never
  clears a cell whose yard map byte lacks the yard bit, while the claim clears such a cell when it holds the unit's id.
- **Q11.** `MAP_CLEAR_UNIT` on a carried unit sets `Unit.spatialBucket` to null but leaves every chain as it was.
- **Q12.** `MAP_SET_YARD` keeps only the low bit of `open` in `Unit.bit2` but tests the footprint with 0x02 for any
  `open` not 0. For an `open` of 2, the test reads 0x02 and the claim 0x04.
- **Q13.** `UnitsInRange` stops when the count written equals `capacity`, so a negative `capacity` writes every
  unit in range.
- **Q14.** `GetGroundHeight` is −1 over the last column and the last row of cells, since it needs the cell beyond.
  `GetCellMeanHeight` rounds toward zero, so a position up to 15 whole units left of or above the map reads cell 0 of
  that row or column.
- **Q15.** The bounds differ: the build test and the level of `SnapWorldPosToFootprint` refuse x = 0 and y = 0,
  while `CanPlaceUnitFootprint`'s own test and `CanPlaceFootprintAt` accept them. For a type that doesn't move,
  `CanPlaceUnitFootprint` at x = 0 or y = 0 passes its own test and then answers 0 from the build test, with the record
  set to 0, even with `flags` 2.
- **Q16.** `BuildAllPassMaps` clears only `defined` of the classes it isn't given: their `width`, `height`, `cells`
  and limits stay as an earlier build left them, and their words are neither freed nor rebuilt. `PassMapInBounds` and
  `PassMapGet` still answer from them. No slot but `RefreshMovedUnits` writes `lastTick`, so a build uses the window
  the class last had, even from an earlier match: a cell with a unit whose stamp is below it is 0.
- **Q17.** A plot map load counts a negative `files.BankReadBox` result as a full read.
- **Q18.** The player feature plot map holds only bits 3 to 6 of `Cell.flags`, and drops the last cell when
  `Game.width` × `Game.height` is odd.
- **Q19.** The visibility word of `CanPlaceFootprintAt` adds ⌊*ux* / 4⌋ to the row as well as to the column: the
  row term uses the footprint's x size, not its z size.
- **Q20.** `CanPlaceFootprintAt` answers 1, without looking at any cell, when the site's visibility word lacks the
  unit's player bit.
- **Q21.** The *y* of a reclaim scan point, which `los_radar.GetMapCellAtPosition` is given and the picked
  position's `Vec3` receives, isn't specified: no recording calls the pick, and the trace leaves a reclaim order's
  `pos.y` out (`docs/view.md`, "Orders"). An implementation may use any value.
- **Q22.** The reclaim lists hold ⌊*w*² / 256⌋ entries, which is 0 when `range` is below 16 × 65536 (an allocation of
  0 bytes, which `services.SimAlloc` gives as a new, non-null, smallest allocation, ABI 9.3; the kit doesn't give that
  allocation's size). A scan with
  more qualifying features than that writes 16-byte entries past the end of its allocation. The lists still behave as
  if they had no limit: the pick reads the entries written. What the block bytes after a list's allocation hold
  afterwards isn't specified by this spec.
- **Q23.** The pick starts from entry 0 and the value 0.0. When none of the three drawn entries has a value above
  0.0 (every drawn value negative), the pick is entry 0, which may not have been drawn.
- **Q24.** `AllocateBuckets` allocates a new off-map bucket every call and doesn't free the previous one. It does
  free the previous grid.
- **Q25.** When the grid is one bucket wide or one bucket tall, every bucket's *b* is 0, whatever the heights.
- **Q26.** `FreeBuckets` leaves `Grid2.widthFixed` and `Grid2.heightFixed` as they were.
- **Q27.** `RefreshMovedUnits` doesn't keep the unit's stamp at `Game.ticks`: the stamp is `Game.ticks` only while
  the call runs (the pass values of the cells the unit holds use `Game.ticks` as its stamp), and the call ends with
  the stamp it started with.

## 7. Examples

From the kit's `tests/replay/`. Each example is recomputed from this spec's definitions.

### `Apply`

**`MAP_ADD_UNIT` of a structure**, `tests/replay/map/ui-build-place.txt`, call at tick 138 of `Apply`, kind 160,
`Unit#2`, size 8. `Unit#2`: cell (38, 22), footprint (4, 4), flags 0x30280221 (structure bit set), `cobStateFlags` 0
(`bit2` 0, so the yard bit is 0x04), motion null, `spatialBucket` null, carrier null, `pos.x` 41943040, `pos.z`
25165824, type `UnitDef#135` with a yard map of 16 bytes of 0x2f. `Game.width` 224, *GW* 28; the 16 cells have
`Cell.unit` 0 and `Cell.flags` 80. Classes 0 to 14 are defined.
- No stamp (motion null). Bounds: 38 + 4 = 42 < 224 and 22 + 4 = 26 < 102, on the map.
- The bucket: 41943040 asr 23 = 5, 25165824 asr 23 = 3, element 5 + 3 × 28 = 89. `Unit#2.spatialBucket` becomes
  `SpatialBucket#89`, `Unit#2.cargoNext` stays null (the old head), `SpatialBucket#89.head` becomes `Unit#2`.
- 0x2f has 0x04 and 0x01: each of cells 22 × 224 + 38 = 4966 to 4969, 5190 to 5193, 5414 to 5417 and 5638 to 5641
  gets `Cell.unit` 2 (a take of a 0 field) and `Cell.flags` 80 OR 2 = 82.
- Imports: `SchedPublish 7 20` (CELL_HEIGHTS over (37, 21) and (6, 6)), then 15 × `SchedPublish 7 32` (reports of
  classes 0 to 14, cause 2). Outcome 1.

**`MAP_REMOVE_UNIT` of a unit with motion, then `MAP_ADD_UNIT`**, `tests/replay/map/orders-air.txt`, calls at tick
1045 of `Apply`, kind 161 then kind 160, `Unit#1`. `Unit#1`: cell (36, 19), footprint (2, 2), flags 0x90e10325
(structure bit clear, layer 1, claim pending and overlapped bits clear), motion `UnitMotion#1` with stamp 944, in a
bucket of the grid. The windows: class 2 is 1000, every other class 0.
- REMOVE: cells 19 × 224 + 36 = 4292, 4293, 4516, 4517 held 1 and become 0. No re-claim (overlapped bit clear). The
  stamp: *s* = 944, the stamp becomes 1045; only class 2 has a window above 944 (1000 > 944, while 0 > 944 fails), so
  one rebuild and one report: one `SchedPublish 7 32`. Outcome 1.
- ADD at the same tick, with the cell now (37, 19): the stamp becomes 1045 (no change), the bucket is the same (no
  chain write), and cells 4293, 4294, 4517, 4518 take `Cell.unit` 1. No import. Outcome 1.

**The rebuild and report of that `MAP_REMOVE_UNIT`**, the same call: a rebuild of class 2 over ((36, 19), (2, 2))
after the unit leaves its cells. Class 2: footprint 2 × 2, `maxWaterDepth` 100, window 1000.
- The rectangle is columns 34 to 38 (right 36 + 2 + 1 = 39 not included), rows 17 to 21.
- Columns 32 to 39 of rows 16 to 22 have pass value 3; column 40 of rows 16 to 21 holds `Unit#2` (`Cell.unit` 2),
  whose motion is null, so 0. Columns 34 to 37 get 3; column 38 gets 1 (its grown block reaches column 40).
- Words 258 to 261 (0xfffff557, 0xfffff407, 0xfffff407, 0xfffff407) become 0xffffffff; word 262 keeps 0xffffd555.
  This is the recorded `w PassMap#2.cells 258:ffff...`.
- The report: rect (34, 17, 38, 21), `classes` 4, `cause` 2, recorded as a `SchedPublish 7 32` import line.

**`MAP_CLEAR_UNIT` of a structure**, `tests/replay/map/orders-air-repair.txt`, call at tick 382 of `Apply`, kind 162,
`Unit#3`. `Unit#3`: cell (38, 24), footprint (2, 2), flags 0xb0084121 (structure bit set, claim pending and overlapped
clear), motion null, carrier null, `spatialBucket` `SpatialBucket#88`, `cargoNext` null, a yard map of 4 bytes of
0x2f. The chain of `SpatialBucket#88` is `Unit#252`, then `Unit#3`. The off-map bucket is `OverflowBucket#1`.
- Cells 24 × 224 + 38 = 5414, 5415, 5638, 5639: `Cell.unit` 3 becomes 0, and `Cell.flags` 82 AND 0xfd = 80.
- Imports: `SchedPublish 7 20` (CELL_HEIGHTS over (37, 23) and (4, 4)); then, motion null, the rebuilds and the
  reports of classes 0 to 14: 15 × `SchedPublish 7 32`.
- The chain: the head is `Unit#252`, not `Unit#3`, so `Unit#252.cargoNext` becomes `Unit#3.cargoNext`, null.
  `Unit#3.cargoNext` stays null. `Unit#3.spatialBucket` becomes null. Outcome 1.

**`MAP_SET_YARD`, the claim, and Q2**, `tests/replay/map/orders-air.txt`, call at tick 1024 of `Apply`, kind 164,
`Unit#2`, `open` 1, size 12. `Unit#2`: cell (40, 16), footprint (7, 6), flags 0x30e42321 (structure), `cobStateFlags`
0, player `Player#0` (active 1, type 1), a yard map of 42 bytes (7 per row): rows 0 and 1 of 0x2f × 7, rows 2 to 5 of
0x2f, 0x2d × 5, 0x2f. Every footprint cell holds 2.
- Clear for open 1: 40 ≥ 1, 16 ≥ 1, 47 < 224, 22 < 102, and every cell with 0x02 holds 0 or 2. Outcome 1.
- `Unit.bit2` becomes 1: `cobStateFlags` 4. The flags get the claim pending bit, then the claim clears it and calls
  `los_radar.GetMapCell 40 16` = `Cell#3624` (16 × 224 + 40).
- Bytes 0x2f have 0x02: a take of a cell holding 2 by unit 2, whose player has type 1, so the unit gets 0x4000000 and
  0x8000000 and the cell keeps 2. Bytes 0x2d lack 0x02: the cell held 2, so it becomes 0. Those are cells 3624 + row ×
  224 + col for rows 2 to 5, cols 1 to 5: 4073 to 4077, 4297 to 4301, 4521 to 4525, 4745 to 4749.
- `Unit.flags` becomes 0x30e42321 OR 0x0c000000 = 0x3ce42321.
- Imports: `los_radar.GetMapCell 40 16`, then 15 × `SchedPublish 7 32`.

**`MAP_ADD_UNIT` of a mobile unit at the front of a chain, then `MAP_BUCKET_UNLINK`**,
`tests/replay/map/orders-air.txt`, calls at tick 1025 of `Apply`, kinds 160 and 165, `Unit#3`: cell (42, 19),
footprint (3, 3), flags 0x10240221 (layer 1), motion `UnitMotion#3` (stamp 0), `pos.x` 45681211, `pos.z` 21015042,
`spatialBucket` null. The head of `SpatialBucket#61` is `Unit#2`.
- ADD: the stamp becomes 1025. Element (45681211 asr 23) + (21015042 asr 23) × 28 = 5 + 2 × 28 = 61:
  `Unit#3.cargoNext` becomes `Unit#2`, the head becomes `Unit#3`, `Unit#3.spatialBucket` becomes `SpatialBucket#61`.
  Cells 4298 to 4300, 4522 to 4524 and 4746 to 4748 take `Cell.unit` 3 (each held 0). No import.
- BUCKET_UNLINK: `Unit#3` is the head, so the head becomes `Unit#2` and `Unit#3.cargoNext` becomes null;
  `Unit#3.spatialBucket` stays `SpatialBucket#61`. Outcome 1.

**`MAP_SET_SEA_LEVEL` and `MAP_SET_EDGE`**, `tests/replay/map/console-features.txt`, tick 160, kind 170, level 20,
size 12: `Game.seaLevel` 0 becomes 20. `tests/replay/map/console-units.txt`, tick 110, kind 171, x 10, z 10, size 16,
with `Game.baseX` 3584 and `Game.baseY` 1632: `Game.mapWidth` 3552 becomes 3574 and `Game.mapHeight` 1504 becomes
1622.

**`MAP_PASS_MAPS` with no defined class**, `tests/replay/map/ui-build-place.txt`, the first call at tick 0, kind 168,
cell (50, 1), size (1, 1), size 24: every `PassMap.defined` is 0, so nothing is written and nothing is published;
outcome 1.

Every one of the 639 `Apply` calls of the five map recordings is reproduced from this text (outcome, import lines and
the writes of section 3), one `MAP_BUCKET_PREPEND` (`orders-air.txt`, tick 1501) and the layer 2 `MAP_ADD_UNIT`
included.

### `UnitsInRange`

**One unit**, `tests/replay/los_radar/console-units.txt`, the import at tick 0 of `UpdateSensorRadarAndCloak`:
`map.UnitsInRange (36700160,65536,15728640) 26214400 0 ? 256 = 1`, `o ids x:0100`. The only unit is unit 1, at the
query's position, in bucket 1 × 28 + 4 = 32. *z₁* = (15728640 − 26214400) asr 23 = −2, clamped to 0. The rectangle is
x 1 to 7, z 0 to 5, with *R* = 160000. D = 0, so the ids are [1], result 1.

**The walk's order**, `tests/replay/orders/orders-air-repair.txt`, the import at tick 852 of `RunOrders`:
`map.UnitsInRange (30408704,1376256,22020096) 18350080 0 ? 256 = 2`, `o ids x:01000200`. The rectangle is x 1 to 5,
z 0 to 4, with *R* = 78400. Unit 1 is at (36700160, 15728640) in bucket 32, with D = 9216 + 9216 = 18432. Unit 2 is at
the query's position, in bucket 59. Row 1 comes before row 2, so the ids are [1, 2], result 2. The positions and
buckets are those the same recording and `tests/replay/map/orders-air-repair.txt` read; that no other unit's chain
reaches the rectangle at tick 852 isn't shown by any line, and the recorded result implies it. Not from a recording:
worked from the definitions, with `skip` 1 the ids would be [2], and with `capacity` 1, [1].

### `GetCellMeanHeight`

**Mean height**, `tests/replay/map/orders-air-repair.txt`, call at tick 382 of `GetCellMeanHeight`, pos (40627783, ·,
25302316). whole gives (619, 386), and trunc(619 / 16) = 38, trunc(386 / 16) = 24. The cell (38, 24) has `low` 1 and
`high` 1, so the result is ⌊2 / 2⌋ = 1.

### `CanBuildAt`

**A site that fits**, `tests/replay/map/orders-air.txt`, call at tick 1350 of `CanBuildAt` (`UnitDef#208`, cell (188,
29), type 0, player null). The footprint is 3 × 3 and `Game.width` 224, so the cells are `Cell#6684` to 6686, 6908 to
6910 and 7132 to 7134. Every yard byte is 0x2f, and every cell has `low` 1, `high` 1, `metal` 223, `unit` 0 and `flags`
0x50 (bit 1 clear). `Cell#6684` has feature 0, and the others 0xfffe with origin `Cell#6684`, so each has feature
record 0, whose `flags16` is 777 (0x0309: `blocking` 0, `geothermal` 0, `indestructible` 1). No cell test fails (bit
0x40 isn't in the yard bytes). *L* = *H* = 1, *K* = 0 and *G* = 1. With *sea* 0, `maxslope` 10, `maxwaterdepth` 0 and
`minwaterdepth` −10000, no site test fails (*H* − *L* = 0 ≤ 10, *K* = 0 ≤ 1, *L* = 1 isn't below 0, max(1, 0) = 1 ≤
10000), so the result is 1, `g_lastPlaceHeight` 1 and `g_lastPlaceMetalSum` 9 × 223 = 2007.

**A site that fails at its first cell**, `tests/replay/map/orders-air.txt`, call at tick 1170 of `CanBuildAt`
(`UnitDef#268`, cell (185, 78), type 0, player null). `Cell#17657`, the first cell, has yard byte 0x2f, `flags` 0x52
and `metal` 223. Its metal is added, and then cell test 1 fails (0x01, `flags` bit 1, *S* 1). The result is 0,
`g_lastPlaceMetalSum` 223 (one cell of nine) and `g_lastPlaceHeight` 0.

### `CanPlaceUnitFootprint`

**A site test fails, with the ground level from the draft**, `tests/replay/map/ui-build-place.txt`, call at tick 180
of `CanPlaceUnitFootprint` (`UnitDef#186`, `mobile` 0, footprint 3 × 3, cell (216, 74), flags 1), so the build test
with type id 0 and no player. Every yard byte is 0x37 (bits 0x01, 0x02, 0x04, 0x10, 0x20: no 0x08). The nine cells
(`Cell#16792` to 16794, 17016 to 17018, 17240 to 17242) have `low` = `high` = 1, `metal` 3, `unit` 0, `flags` 0x50 and
feature 0xffff, so no cell test fails and the metal sum is 27. No cell is a ground cell, so *L* = 255, *H* = 0 and *G*
= (0 − 22) mod 256 = 234 (`draft` 22, *sea* 0). *K* = 1 ≤ 234, and *L* = 255 isn't below 0 − 0, but max(0, 1) = 1 > 0 −
11 (`minwaterdepth` 11): site test 5 fails. The result is 0, `g_lastPlaceMetalSum` 27 and `g_lastPlaceHeight` 0.

**A feature blocks the eighth cell**, the same file and tick, cell (205, 74). Cells 0 to 6 pass. Cell 7, `Cell#17230`,
has feature 1, below `Game.featureCount` 399, and element 1 of `Game.features` has `flags16` 473 (0x01d9, `blocking`
set): cell test 3 fails. The result is 0 and `g_lastPlaceMetalSum` is 8 × 3 = 24.

**A type that doesn't move**, `tests/replay/map/ui-build-place.txt`, call at tick 138 (`UnitDef#135`, `mobile` 0, cell
(38, 22), flags 1). The result is 1 and the record becomes height 1, metal sum 48, the build test's answer.

**A moving type that doesn't fit**, `tests/replay/map/orders-air.txt`, call at tick 1501 of `CanPlaceUnitFootprint`
(`UnitDef#29`, `mobile` 1, footprint 2 × 2, otherTypeId 0, cell (43, 19), flags 1). `Cell#4299` has `unit` 3, which is
neither 0 nor `otherTypeId`, so move test 2 fails and the result is 0. The record isn't written.

### `BuildAllPassMaps`

**The full build**, `tests/replay/map/ui-build-place.txt`, call at tick 0 of `BuildAllPassMaps`, `count` 15 (classes 0
to 14). `Game.width` 224, `Game.height` 102, `Game.seaLevel` 0, `Game.featureCount` 399, every unit's motion null.
- *n* = ⌊117 / 16⌋ × 224 = 7 × 224 = 1568 words: `SimAlloc 6272` for each class. The scratch for a 2 × 2 class is
  max(224 + 2, 2 + 102) + 3 = 229, for a 3 × 3 class 230, for 6 × 6 233. The import lines are, per class, `SimAlloc
  6272`, `SimAlloc <S + 3>`, `SimFree` of the scratch, and no free of the null `cells`.
- All 1568 words of each of the 15 classes are reproduced (rows 102 to 111 of the last row of words are 0), and
  `Game.progress_38d73` ends at 100.
- By hand, class 0 (footprint 2 × 2, `maxWaterDepth` 12, `minWaterDepth` −10000, `maxSlope` 32, `badSlope` 16): the
  cells of columns 0 to 3, rows 0 to 5 have `high` = `low` = 1 (land, slope 0) and no feature, so pass value 3, except
  (1, 2), whose feature 2 has `flags16` 473 = 0x1d9 (blocking bit set): 0. Column 0: row 0 has a full block, but its
  grown block has column −1, so 1; rows 1 and 2 have (1, 2) in their block, so 0; row 3 and on are 1. Word 0 is
  0x55555541 (bits 01, 00, 00, 01, then 01), recorded as `41555555` at the start of `w PassMap#0.cells [1568] 0:`.

### `SnapWorldPosToFootprint`

**Snap**, `tests/replay/map/ui-build-place.txt`, call at tick 138 of `SnapWorldPosToFootprint` (`UnitDef#135`,
`mobile` 0, footprint 4 × 4), pos (41943040, 65536, 25165824). *cx* = (41943040 − 2097152 + 524288) asr 20 = 40370176
asr 20 = 38, and *cz* = (25165824 − 2097152 + 524288) asr 20 = 23592960 asr 20 = 22. `pos.x` becomes (4 + 76) × 2^19
= 41943040 and `pos.z` (4 + 44) × 2^19 = 25165824. The 16 cells have yard byte 0x2f and `low` = `high` = 1, so *F* = 1
and `pos.y` becomes 65536: the position was already on the grid.

### `CanPlaceFootprintAt`

**A fitting position on a seen site**, `tests/replay/map/orders-air.txt`, the fourth call at tick 1582 of
`CanPlaceFootprintAt` (`Unit#3`, footprint 3 × 3, `id` 3, `playerIndex` 0), pos (51904512, 5373952, 25690112). *a* =
(51904512 − 1572864 + 524288) asr 20 = 48 and *c* = 23. *j* = (11 + 0) × 112 + 24 + 0 = 1256, and that word is 1,
which has bit 1 << 0, so the cells are tested. All nine have `unit` 0, `unit2` 0, `flags` 0x50, `low` = `high` = 1 and
no feature: the result is 1. The first call at that tick, at pos (45616248, ·, 21473275), gives the site (42, 19),
whose first cell has `flags` 0x52, so the result is 0.

### `IsPadSlotFree`

**A free pad slot**, `tests/replay/map/orders-air-repair.txt`, call at tick 382 of `IsPadSlotFree` (`Unit#2`, id 1).
`carrier` and `cargo` are null, so the result is 1.

### `PickRandomReclaimableResourcesInRadius`

**The capacity, the scan and the pick.** Not from a recording: worked from the definitions. `range` 200 × 65536: *w* =
200, *c* = ⌊40000 / 256⌋ = 156, two allocations of 2496 bytes, *h* = 6553600, and 5 × 5 scan points (offsets
−6553600, −3407872, −262144, 2883584, 6029312 on each axis). A list with values 5.0, 9.0, 9.0, 2.0 and draws 2, 1, 3
picks index 2 (the first 9.0), amount 9.0. A list with values −1.0, −2.0, 4.0 and draws 1, 1, 1 picks index 0, amount
−1.0 (Q23).

### `AllocateBuckets`

**The bucket grid of a recorded map.** Not from a recording of the call: worked from the definitions, and checked
against recorded bucket bytes. The map of `tests/replay/map/orders-air-repair.txt` has `Game.baseX` 3584, `Game.baseY`
1632, `Game.width` 224, `Game.height` 102 and `Game.seaLevel` 0, and the call at tick 382 reads every `Cell.high`.
- `widthFixed` = 3584 × 65536 = 234881024, so *GW* = (234881024 + 0x7fffff) asr 23 = 28. `heightFixed` = 106954752,
  so *GH* = 115343359 asr 23 = 13. Both match the recorded `Game.width_142a3` 28 and `Game.height_142a7` 13.
- *N* = 28 × 13 = 364, rounded up to 368. The four padding buckets 364 to 367 have *a* 0, *b* 0, *flags* 0.
- Bucket 0: *flags* 1 + 4 = 5; bucket 27: 1 + 8 = 9; bucket 363 (x 27, z 12): 2 + 8 = 10.
- Bucket 61 (x 5, z 2): *a* 1, and *b* 42, from a neighbour. Bucket 63 (x 7, z 2): *a* 211, *b* 231. Bucket 91 (x 7,
  z 3): *a* 231, *b* 231.
- The movement recordings of the same map read 25 bucket lines (18 distinct buckets) as
  `SpatialBucket#N.bytes` (bytes 0 to 5: *a*, *b*, *flags*). All 25 lines match: for example
  `tests/replay/movement/orders-air.txt`, `SpatialBucket#63.bytes x:d3e700000000` (*a* 0xd3 = 211, *b* 0xe7 = 231, *flags* 0), and `SpatialBucket#61.bytes x:012a00000000`.

**A 3 × 2 grid.** Not from a recording: worked from the definitions. `Game.baseX` 320 and `Game.baseY` 200 give *GW* =
3, *GH* = 2, *N* = 8. With `Game.seaLevel` 3 and cell heights whose bucket maxima are 49, 49, 200 / 48, 49, 49, the *a*
are 49 49 200 / 48 49 49, then 3 3 for the padding. The *b* are 49 200 200 / 49 200 200, then 0 0: bucket 0 doesn't
reach bucket 2. The *flags* are 5 1 9 / 6 2 10, then 0 0.

**One bucket wide.** Not from a recording: worked from the definitions. `Game.baseX` 128 and `Game.baseY` 320 give
*GW* = 1, *GH* = 3. Every *b* is 0 (Q25), with every *a* 100. The *flags* are 13, 12, 14.

### `GetGroundHeight`

**Ground height with two fractions**, `tests/replay/map/orders-air.txt`, call at tick 1030 of `GetGroundHeight`, pos
(39194597, ·, 20558683). whole gives (598, 313), so the cell is (37, 19) with fractions *u* = 6, *v* = 9. The four
heights are 1, so *north* = *south* = 1 and the result is 1.

### `PassMapGet`

`tests/replay/map/orders-air.txt`, call at tick 1030 of `PassMapGet(2, 38, 19)`. `width` 224: word 224 × 1 + 38 =
262, which is 0xffffd555; shift 2 × 3 = 6; (0xffffd555 shifted right by 6) mod 4 = 1. The result is 1. And at tick
1351, `PassMapGet(2, 189, 88)`: word 224 × 5 + 189 = 1309, 0x00007fff, shift 16: 0. All 457 calls of the file and the
24 of `ui-build-place.txt` reproduce.

### `PassMapInBounds`

`tests/replay/map/orders-air.txt`, call at tick 1030 of `PassMapInBounds(2, 36, 19)`: 36 < 224 and 19 < 102, so 1. All
608 calls of the file and the 24 of `ui-build-place.txt` reproduce (every one gives 1).

### Not in any recording

No recording checks these, and the gate judges no scenario of a blocked entry (section 2), so no gate checks them
at this kit.

- **Slots never called:** of the module's, `PickRandomReclaimableResourcesInRadius` (its draws, Q22, Q23, Q21),
  `RefreshMovedUnits` and `RefreshUnitIfStale` (the refreshes aren't recorded yet, `docs/replay-format.md`). Of the
  engine's (section 2), every one: `FreePassMaps`, `AllocateBuckets` and `FreeBuckets` (only their results in the
  state, Q25, Q24, Q26), the four plot map slots (every outcome of the load tables, Q18, Q17), `WriteState` and
  `PathLockStamp`.
- **`Apply`:** `MAP_RECLAIM_FOOTPRINT`, `MAP_LOAD_PATH_LOCK_STAMP`, the refusal of `MAP_BUCKET_PREPEND`, every outcome
  0 (each size check, the null unit, the unknown kinds, a negative `MAP_PASS_MAPS` size, the two refusals of
  `MAP_SET_YARD` and of `MAP_BUCKET_UNLINK`, `MAP_LOAD_PATH_LOCK_STAMP` without motion), `MAP_ADD_UNIT` off the map
  (the off-map bucket), a re-claim, a take of a cell held by another unit (either player case), the payload values of
  a CELL_HEIGHTS publish and of a report (the recorder writes only the publisher and the size), a `MAP_PASS_MAPS` with
  defined classes (every recorded one is at tick 0, before the pass maps exist), and `skipEmptyYardmap` on.
- **The pass maps:** a second `BuildAllPassMaps` (the free of old words, Q16); a `PassMapInBounds` that gives 0; a
  stale stamp in a pass value (every unit met by a recorded rebuild or build has a null motion, and no cell holds a
  unit at the build); the right and bottom edge rule of the footprint value (Q4), which no recorded rebuild
  reaches.
- **The features:** the values 0xfffb to 0xfffd, a feature number at or past `Game.featureCount`, and a 0xfffe cell
  whose origin is 0xfffb or above (Q1).
- **`CanBuildAt` and the build test:** a player (the visibility test, map flag 2, Q7); yard bits 0x40 and 0x80
  (cell test 4 and site test 1); site tests 2, 3 and 4 failing; cell test 2 failing alone; a ground level from the draft
  in a call that returns 1, so the byte truncation is never seen in `g_lastPlaceHeight`.
- **`CanPlaceUnitFootprint`:** off the map (both `flags` cases), `flags` other than 1 and 2, move tests 1, 3, 4 and 5
  failing, and Q15.
- **`SnapWorldPosToFootprint`** for a moving type, or at a site whose level is 0 from its bounds.
- **`CanPlaceFootprintAt`** off the map, with an unseen word (Q20), and the `flags1` rule.
- **`IsPadSlotFree`** with a carrier or a non-empty cargo chain.
- **`UnitsInRange`:** the `skip` and `capacity` limits (Q13).
- **The heights:** `GetGroundHeight` on uneven ground (every recorded call reads four equal heights) and its −1
  (Q14); `GetCellMeanHeight`'s −1 and negative positions.

## 8. Kit version

Kit `interfaces-v69` and `reference-traces-v56` (both at commit 7311500c1330), the versions the gate
judges against. Interface `map` 9.2. The imports are against `los_radar` 5.1, `features` 5.1 and `services` 4.3.
