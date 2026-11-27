# features

## 1. Role

Features are the things on the map that are not units: trees, rocks, wrecks and the like. Each feature type has a
footprint of cells, and a placed feature fills its cells: the first cell (its origin) holds the type's number and the
others say how far they are from it. A feature with a 3D model, and a sprite feature that burns or plays its dying
animation, also holds a spot: an entry of a pool of 2048 kept on three lists (used, at rest and free). Other modules
publish to features when a unit leaves a wreck, a unit reclaims a feature, a wreck is resurrected and an explosion
hits a feature; the console and the map loader publish too. Each tick, in its phase, features advances the
animation of every animated type, scans one cell of the map, where a feature may seed a copy of itself nearby, and
updates each spot of the used list: a wreck falls or comes to rest, a fire burns, spreads and burns out, and a dying
feature turns into its dead type. Features also answers which feature is on a cell or under a position, and where a
footprint's centre is.

## 2. Interface

### Slots (`interfaces/features.h`, version 5.1)

| Slot | When the engine calls it |
| --- | --- |
| `UpdateFeatures` | the features phase (phase 5), once a tick; see `UpdateFeatures` in section 4 |
| `LoadFeatureFileList`, `NopAfterFeatureEnum`, `ResolveFeatureLinks`, `FreeFeatureFileList`, `FindOrLoadFeatureType` | at match load, outside the tick; the engine's (below) |
| `InitFeatureAnimPool`, `PlaceMissionFeatures`, `StampFeatureMetal` | at match load, outside the tick; the engine's (below) |
| `FreeFeaturePool` | when a match shuts down; the engine's (below) |
| `SaveFeatures`, `LoadFeatures` | when a game is saved or loaded, outside the tick; the engine's (below) |
| `GetCellFeature` | a query, from map, units, orders, weapons, ai and the engine |
| `GetFootprintCentre` | a query, from weapons and orders |
| `Apply` | for each event of features' inbox (PLACE, REMOVE, DAMAGE, RECLAIM, REMOVE_ALL): at once on a synchronous route or outside the tick, otherwise when the engine delivers the inbox |
| `Place` | synchronous, from units (a corpse under the sea); the engine delivers features' inbox first |
| `FeatureTypeCount` | a query, at the start of the features phase |
| `AnimateFeatureType` | in the features phase, once for each feature type in index order |
| `ScanFeatureCell` | in the features phase, once, after the type loop |
| `FirstFeatureSpot` | in the features phase, once, after the scan |
| `UpdateFeatureSpot` | in the features phase, once for each spot of the used list, from `FirstFeatureSpot` along the spot each call returns, until -1 |
| `FindFeatureAtPos` | a query, from orders and players |

`docs/schedule.md`, "Routes", and `data/routes.csv` give the routes: units' PLACE, orders' RECLAIM and REMOVE and
weapons' DAMAGE reach `Apply` at once (synchronous); the engine's PLACE, REMOVE and REMOVE_ALL (the map loader and the
console) are queued, and are published only outside the tick, where every route delivers at once. The engine
delivers features' inbox before the type loop, before the scan and before each spot (`docs/schedule.md`, the passes
`features` and `features.anim`).

**The engine's slots.** Features' entry in the kit's manifest (`wasm/modules.json`, `engine`) keeps eleven slots in
the engine: `LoadFeatureFileList`, `FreeFeatureFileList`, `NopAfterFeatureEnum`, `ResolveFeatureLinks`,
`InitFeatureAnimPool`, `FreeFeaturePool`, `PlaceMissionFeatures`, `StampFeatureMetal`, `FindOrLoadFeatureType` and
`LoadFeatures` (load class) and `SaveFeatures` (a query of a saved game, outside the tick). The module does not export
them, and the host calls the engine's native slot instead, with features' state in the block as the native slot reads
it (ABI section 6, "Engine slots"; section 9.3, "What stays in the engine"). Their subsections in section 4 state what
each leaves in the block that the module's slots then read.

### The manifest entry

The kit's manifest has features' entry, and the module must match it exactly (ABI 4.2 and 9.1):

- `name` and `interface`: `features`.
- `exports`, in this order: `UpdateFeatures`, `GetCellFeature`, `GetFootprintCentre`, `Apply`, `Place`,
  `FeatureTypeCount`, `AnimateFeatureType`, `ScanFeatureCell`, `FirstFeatureSpot`, `UpdateFeatureSpot`,
  `FindFeatureAtPos`. The other eleven slots are the engine's (above).
- `data`: `g_game`, `g_simGlobals`, `g_extTdf`, `g_blockBase` (ABI 8.2). The module reads `g_game` and `g_blockBase`.
- `majors`: `features` 5, `los_radar` 5, `map` 9, `mission` 2, `network` 2, `scripts` 3, `services` 4.
- `data_base` 262144 and `table_base` 112: the module's window of `window` (32768) bytes from 0x40000, and its 32 table
  entries from 112 (ABI 2.2 and 2.3).
- `publishes`: `{"4": 24, "48": 8, "49": 12, "50": 20, "51": 20}`: economy's INCOME (`EconomyIncome`, 24 bytes), which
  the route table carries from features' outbox to economy, and features' own FREE_STATE (`FeaturesFreeState`, 8),
  INIT_STATE (`FeaturesInitState`, 12), PASS_MAPS (`FeaturesPassMaps`, 20) and AREA_DAMAGE (`FeaturesAreaDamage`, 20).
  `Emit` takes any kind a route carries from the calling module's outbox (ABI 9.3).
- Fuel: the module meters itself by rule version 1 (ABI section 11) and exports `__fuel_set` and `__fuel_get`. The
  entry has no `fuel_budget` and no `size_budget` ("Kit gaps", below).
- Slot kinds, as the entry gives them:

| Slot | Kinds |
| --- | --- |
| `UpdateFeatures`, `ScanFeatureCell` | `[] -> void` |
| `GetCellFeature` | `[block] -> int` |
| `GetFootprintCentre` | `[linear, block, linear] -> void` |
| `Apply` | `[linear, int] -> int` |
| `Place` | `[linear] -> int` |
| `FeatureTypeCount`, `FirstFeatureSpot` | `[] -> int` |
| `AnimateFeatureType` | `[int] -> void` |
| `UpdateFeatureSpot` | `[int] -> int` |
| `FindFeatureAtPos` | `[linear, linear, linear] -> int` |

The `unsigned short` results of `GetCellFeature` and `FindFeatureAtPos` are returned zero-extended (ABI 7).

- Imports, as the entry lists them:

| Import | Kinds | Used for |
| --- | --- | --- |
| `los_radar.GetMapCell` | `[int, int] -> block` | the cell at a cell position, or 0 off the map |
| `los_radar.SoundHeardMask` | `[linear] -> int` | argument 4 of the sound of a feature that starts to burn |
| `map.GetCellMeanHeight` | `[linear] -> int` | the ground under a falling spot |
| `map.GetGroundHeight` | `[linear] -> int` | the height of a footprint's centre |
| `mission.GetGameType` | `[] -> int` | whether the match runs the multiplayer rules (game type 3) |
| `network.SendFeatureHit` | `[block, int, int] -> void` | a hit that this machine does not apply (arguments: the weapon definition, x, z) |
| `network.SendFeatureKilled` | `[int, int] -> void` | a feature that a hit killed, in game type 3 (x, z) |
| `network.SendFeatureBurning` | `[int, int] -> void` | a feature that started to burn (x, z) |
| `network.SendFeatureReclaimed` | `[block, int, int] -> void` | a feature that a unit reclaimed, in game type 3 (the unit, x, z) |
| `scripts.ObjectStateSize` | `[linear] -> int` | the size of the piece state of a feature type's model |
| `services.SimAllocTagged` | `[linear, int] -> linear` | the piece state of a placed feature with a model, tag `"Object State"` |
| `services.InitGafSequence` | `[linear, linear, int] -> void` | starting a spot's animation and its shadow on a sequence of the type |
| `services.StepGafSequence` | `[linear] -> int` | advancing a type's or a spot's animation by a tick |
| `services.Emit` | `[linear, int] -> void` | publishing INCOME, FREE_STATE, INIT_STATE, PASS_MAPS and AREA_DAMAGE |

The behaviour of section 4 makes four more imports, which the entry does not list ("Kit gaps"):

| Import | Kinds | Used for |
| --- | --- | --- |
| `services.RandomInt` | `[int] -> int` | the scan's seeding, a fire's burn time and its spread (section 5, "Draws") |
| `services.GameRand` | `[int] -> int` | the two draws of a burning feature's smoke, from stream 0 (section 5, "Draws") |
| `services.RecordSimEvent` | `[int, int, linear, int] -> void` | the three sim event kinds below |
| `services.GetGafFrame` | `[linear, int] -> linear` | the current frame of a burning spot's animation, for its smoke |

- Hooks: every slot of `interfaces/features.h` says "Hooks: none", and the module raises none.

### Kit gaps

At `interfaces-v66` these are gaps in the kit, not in the module:

- **The budgets.** Features' entry has no `fuel_budget` or `size_budget`, which ABI 4.2 requires of an entry that is
  not `blocked`.
- **The missing imports.** The entry's `imports.services` lists `Emit`, `InitGafSequence`, `SimAllocTagged` and
  `StepGafSequence` only. The draws of the scan, of a fire and of its smoke (`RandomInt`, `GameRand`), the three sim
  events (`RecordSimEvent`) and the smoke's frame are not listed. The kit has no import that reads a cursor's current
  frame; `GetGafFrame` of the cursor's `src` and `index` is the same frame for a running sequence, whose index is
  always below its table's count, and the spec names it for that reason. The recordings carry no line for a
  draw (`docs/replay-format.md`, "A random draw inside an import": the replayer advances the stream words), and no
  recording reaches a fire, its smoke or a geothermal placement, so the recordings give no line for the other two.
  Until the entry lists them, a module that imports them is refused by the loader (ABI 9.1).
- **The publish payloads.** The recordings carry a publish as a `services.SchedPublish` line with 5 and the size (the
  publisher, features, and the payload size; the payload's fields are not recorded), which is `services.Emit` (ABI
  9.3, "Routes in the recordings"). The replay compares only the size of each publish and its order among the
  imports; the fields of section 4 are judged by the scenario traces.
- **The phase driver.** No recording holds an `UpdateFeatures` call (the replay schema names it the phase driver,
  "unrecorded"): the recordings hold the calls of the type loop, the scan and the spot loop, each as its own slot.

### Arguments outside the block

- **An event** given to `Apply` is a payload of `interfaces/features.h` in the caller's memory (`linear`), `size`
  bytes, starting with `FeaturesEventHeader`: `FeaturesPlace` (48 bytes), `FeaturesRemove` (16), `FeaturesDamage`
  (16), `FeaturesReclaim` (20), or the header alone (4) for REMOVE_ALL. Every field is a 32-bit integer; the `unit`
  of RECLAIM is a block offset.
- **`Place`** takes a `FeaturesPlace` in the caller's memory, read only.
- **A cell** given to `GetCellFeature` is the address of a `Cell` of `Game.cells`, in the block.
- **`GetFootprintCentre`**: `cell` is the address of a `Point16` (`x`, then `y`, each signed 16 bits), read only;
  `def` is the address of a `Feature` of `Game.features`, in the block; `out` is the address of a `Vec3` that the slot
  writes whole.
- **`FindFeatureAtPos`**: `pos` is the address of a `Vec3`, read only; `cell` and `size` are each the address of a
  `Point16`, or 0. The slot writes them only as section 4 says.
- **A position** is a `Vec3` of three signed 32-bit fixed-point numbers, 65536 to a world unit and 2^20 to a cell
  (16 world units). A cell position (x, z) is in cells, z being the row.

### Events

Features records three kinds of sim event, each through `services.RecordSimEvent` with flags 0; `docs/sim-events.md`
gives their arguments, phases and order:

- `SIM_EVENT_TIMED_SUB_PARTICLES`: when a feature whose type has the geothermal bit is placed (D9). Its position is
  the position the placement was given, or the footprint's centre; its list is 4.
- `SIM_EVENT_WHITE_SMOKE`: when a burning sprite spot is updated on a tick whose number is a multiple of 3 (D16). Its
  list is 5; its position is the footprint's centre moved by two draws scaled by the current frame (section 5).
- `SIM_EVENT_SOUND_AT_NAME`: when a feature starts to burn (D10), with sound name 1, at the cell (x, z): x and z are the
  cell coordinates times 2^20, y is the cell's `Cell.height` times 2^16, and argument 4 is what
  `los_radar.SoundHeardMask` returns for that position.

The terms "spot flag bit 0", "feature number 0xfffb" and "game type 3" are those of `docs/sim-events.md`, Terms.

## 3. State

Features keeps no region of its own. Its state is view state in the block, hashed by the trace where it lies.

### Owned

| Field | Type | Meaning |
| --- | --- | --- |
| `Cell.feature` | u16 | on a feature's origin cell, the feature type's number (an index of `Game.features`); 0xffff for no feature, 0xfffe for a cell that another cell's feature covers, 0xfffc for the marker the map loader places; the values 0xfffb to 0xffff are markers, not numbers (feature number 0xfffb) |
| `Cell.spot` | u16 | on an origin cell whose spot flag (`Cell.flag0`) is set, the spot's index; on an origin cell without it, the damage the feature has taken (D14); on a covered cell, the two bytes `Cell.offsetY` (low byte) and `Cell.offsetX` (high byte), the distance from the origin in rows and columns |
| `Cell.flags` | u8 | bit 0 (`Cell.flag0`): the origin cell holds a spot; bits 3 to 6 (`Cell.owner`): the player that owns the feature, 10 for none. Features writes these bits only; bits 1, 2 and 7 are map's and stay as they are |
| `Game.pool` (`Pool`) | | `entries` (a block offset: the 2048 spots), `usedHead`, `restHead`, `freeHead`: the spot at the head of each list, or -1 |
| `Spot` (48 bytes) | | the spots, below |
| `Game.scanIndex` | int | the cell the scan last visited, as an index of `Game.cells` |
| `Feature.anim`, `Feature.shadowAnim` | `Handle` | the animation cursors of a type that animates; `AnimateFeatureType` steps them |

A spot's fields, by `layout.json`'s `Spot`; several overlay each other, and which ones a spot uses depends on its
type:

| Field | Offset | Meaning |
| --- | --- | --- |
| `prev`, `next` | 0, 2 | the list links: `next` is the spot nearer the head (-1 at the head), `prev` the spot after it, away from the head (-1 at the tail); a list is walked from its head along `prev` |
| `state` | 4 | a model's spot: the piece state, a block offset, 0 for none |
| `pos` | 8 | a model's spot: its position (three words at 8, 12 and 16) |
| `vel` | 20 | a model's spot: its velocity (three words at 20, 24 and 28) |
| `anim`, `shadow` | 4, 16 | a sprite's spot: the cursors of its animation and its shadow (12 bytes each: `index`, `value`, `kind`, three bytes, then `src` at offset 8, the frame table's address or 0) |
| `rot` | 32 | the rotation, three signed 16-bit angles |
| `damage` | 38 | u16: the damage a model's feature has taken |
| `x`, `z` (`cell`) | 40, 42 | the feature's origin cell, signed 16 bits each |
| `feature` | 44 | u16: the feature type |
| `timer` | 46 | u8: a burning spot's ticks until it spreads (the same byte as `burnTime` and `animBits`) |
| `flags` | 47 | u8: bit 0 `used` (the spot burns), bit 1 `reclaimed`, bit 2 `hasShadow`, bit 3 `noSend`, bit 4 `bit4`, bits 5 to 7 `bits5` |

A spot whose type has the noobject bit is a **sprite spot**: it uses `anim`, `shadow` and `timer`. Any other is a
**model spot**: it uses `state`, `pos` and `vel`. The bytes a spot does not use keep what an earlier user of the spot
left there (Q3, Q4, Q5).

### Read

| Field | Meaning |
| --- | --- |
| `Game.cells` | a block offset: the cells, `Game.width` times `Game.height` of 13 bytes, row by row; the cell at (x, z) has index z × width + x |
| `Game.width`, `Game.height` | the map's size in cells |
| `Game.features`, `Game.featureCount` | a block offset: the feature types, 256 bytes each, and their count |
| `Game.weapons` | the 256 weapon definitions, `WeaponDef`, 277 bytes each: `damage` (u16) and `fireStarter` (u8) |
| `Game.ticks` | the tick |
| `Game.uiOptionFlags` | bit 3 (value 8): features can be damaged |
| `Game.playerIndex`, `Game.players` | this machine's player, and `Player.info` (a block offset) of each player: `PlayerInfo.flags_97` bit 0 (`host`) |
| `Game.windX`, `Game.windZ` | the wind, signed |
| `Game.seaLevel` | u8, the sea level in world units |
| `Game.gravity` | int |
| `Cell.unit` | the unit on a cell, 0 for none (map's) |
| `Cell.height` | u8, the cell's height (map's) |
| `Unit.playerIndex` | u8, the reclaiming unit's player |

The fields of a feature type (`Feature`, by `layout.json`) that the slots read: `footprint` (two signed 16-bit numbers,
x then z), `object` (the address of the type's model, load data, or 0), `seqburn`, `seqburnshad`, `seqdie`,
`seqdieshad`, `seqreclamate`, `seqreclamateshad` (the addresses of frame tables, load data, or 0), `burnweapon` (a
block offset of a `WeaponDef` of `Game.weapons`, or 0), `burnTime`, `damage` (u16), `energy`, `metal` (f32), `dead`,
`burnt`, `reclamate` (u16 type numbers, 0xffff for none), `spreadChance`, `seedChance`, `seedSpread` (u8), and
`flags16`:

| Bit of `flags16` | `layout.json` name | Meaning |
| --- | --- | --- |
| 0 | `noobject` | a sprite type, with no model |
| 1 | `animating` | the type's animation runs |
| 4 | `flamable` | the feature can burn |
| 5 | `geothermal` | a vent: a placement records the plume |
| 9 | `indestructible` | a plain remove (D8) refuses it, and DAMAGE does nothing |

### Between ticks

- Every spot is on exactly one of the three lists, and `Pool.usedHead`, `restHead` and `freeHead` name their heads.
- A cell whose `Cell.flag0` is set is an origin cell whose spot is `Cell.spot`, and that spot's `cell` is that cell.
- At match start (`InitFeatureAnimPool`, the engine's) every spot is free: `freeHead` is 0, spot i has `prev` i + 1
  and `next` i - 1, spot 0 has `next` -1 and spot 2047 has `prev` -1, `usedHead` and `restHead` are -1, every other
  byte of the 2048 spots is 0, and `Game.scanIndex` is 0.

## 4. Behaviour

Within one call, only the order of import calls is observable. The block writes are compared at the end of the call,
so the order they are listed in carries no meaning.

A **draw** is a call of `services.RandomInt`; `services.GameRand` is named where it is meant. Every import named
below is made at the point of its definition, in the order the definitions give, and each publish is one
`services.Emit` of the named payload with its size.

### Definitions

**D1. The cell at (x, z)** is the cell of index z × `Game.width` + x of `Game.cells`, taken without an import. **A
cell from `GetMapCell`** is what `los_radar.GetMapCell(x, z)` returns (0 off the map). A cell's position is its index
modulo `Game.width` (x) and its index divided by `Game.width` (z).

**D2. The origin of a cell** is the cell itself, except for a cell whose `Cell.feature` is 0xfffe: then it is the cell
whose index is the cell's index minus (`Cell.offsetY` × `Game.width` + `Cell.offsetX`), and its position is the
cell's position minus (`offsetX`, `offsetY`).

**D3. The footprint centre** of a cell position (cx, cz) for a type of footprint (fx, fz) is the position with x =
(fx + 2 × cx) × 2^19 and z = (fz + 2 × cz) × 2^19, and y = h × 2^16, where h is what `map.GetGroundHeight` returns
for the address of a `Vec3` that holds that x and z (its y is not read). One `GetGroundHeight` import each time it is
taken.

**D4. Moving spot i to a list** (used, rest or free; "the list's head" is `Pool.usedHead`, `restHead` or
`freeHead`). Spot i leaves the list it is on: the spot that was its `next` (when not -1) gets i's old `prev` as its
`prev`, the spot that was its `prev` (when not -1) gets i's old `next` as its `next`, and when i was the head of its
list that head becomes i's old `prev`. Spot i becomes the head of the target list: its `next` is -1, its `prev` is the
target's old head (read after i left its list, -1 for an empty list), and that spot, when not -1, gets `next` i.

**D5. Taking a spot.** When `freeHead` is -1 the pool is full and nothing is written. Otherwise the spot i at
`freeHead` is moved to the used list (D4) and its bit 0 of `flags` is cleared; its other bytes are left as they are.

**D6. Clearing a cell:** `Cell.feature` becomes 0xffff and bit 0 of `Cell.flags` is cleared. The other bits of
`Cell.flags` and `Cell.spot` stay (Q10).

**D7. A PASS_MAPS publish** for an origin at position (x, z) of a type of footprint (fx, fz): a `FeaturesPassMaps` of
x, z, fx, fz (20 bytes).

**D8. Removing the feature of a cell, plain or forced.** Let o be the origin of the cell (D2) and T the type that
`o.Cell.feature` names.

| Case | Imports, in order | Written | Result |
| --- | --- | --- | --- |
| `o.Cell.feature` is 0xfffb or more | none | nothing | 0 |
| plain, and T has the indestructible bit | none | nothing | 0 |
| otherwise | when `o.Cell.flag0` is set and T is not a sprite type: FREE_STATE (`FeaturesFreeState`, 8 bytes) with the spot's `state`; then PASS_MAPS (D7) for o and T's footprint | when `o.Cell.flag0` is set: for a model type the spot's `state` becomes 0, and the spot `o.Cell.spot` moves to the free list (D4). o is cleared (D6), and so is each cell of T's footprint from o (rows 0 to fz - 1, columns 0 to fx - 1) whose `Cell.feature` is 0xfffe | 1 |

**D9. Placing type t on cell c, with a position or none, a rotation or none, and an owner.** Let (ax, az) be c's
position (D1), and T the type t (taken without checking `Game.featureCount`).

| Case | Imports, in order | Written | Result |
| --- | --- | --- | --- |
| t is 0xffff | none | nothing | none |
| t is 0xfffc | none | `c.Cell.feature` becomes 0xfffc | none |
| ax + fx is more than `Game.width`, or az + fz more than `Game.height` | none | nothing | none |
| a remove of a cell of the footprint gives 0 (a marker, or an indestructible feature) | the imports of the removes up to it (below) | what the removes before it wrote (Q7) | none |
| T is a model type and the pool is full after the removes | the removes' | the removes' | none |
| T is a model type | the removes'; D3 when no position is given; `scripts.ObjectStateSize(T.object)`; `services.SimAllocTagged("Object State", size)`; INIT_STATE; a geothermal type's plume (below); PASS_MAPS (D7) for (ax, az) and T's footprint | below | the spot |
| T is a sprite type | the removes'; a geothermal type's plume; PASS_MAPS | below | none |

- **The removes.** The cells of the footprint from c are visited row by row (rows 0 to fz - 1, then columns 0 to
  fx - 1), and each whose `Cell.feature` is not 0xffff gets a plain remove (D8), with its imports and writes. The first
  remove that gives 0 ends the placement: the cells cleared before it stay cleared.
- **A model type:** a spot is taken (D5), spot i. Its `feature` becomes t, `damage` 0, `cell` (ax, az); `pos` becomes
  the given position, or the footprint centre of (ax, az) for T (D3); `rot` becomes the given rotation, or (0, 0, 0);
  `state` becomes the block offset of the address `SimAllocTagged` returned. Its `vel` and its `flags` but bit 0 are
  left as they are (Q3, Q5). The INIT_STATE is a `FeaturesInitState` (12 bytes) of that offset and t. `c.Cell.feature`
  becomes t, `c.Cell.spot` i, `c.Cell.flag0` 1.
- **A sprite type:** `c.Cell.feature` becomes t, `c.Cell.spot` 0, `c.Cell.flag0` 0.
- **Both:** `c.Cell.owner` becomes the owner's low 4 bits. Every other cell of the footprint, at row r and column k
  from c, gets `Cell.feature` 0xfffe, `Cell.offsetX` k, `Cell.offsetY` r and `Cell.flag0` 0.
- **The plume.** When T has the geothermal bit: `SIM_EVENT_TIMED_SUB_PARTICLES` with the given position, or with the
  footprint centre of (ax, az) for T, taken again (D3, one more `GetGroundHeight` import, after the model's
  INIT_STATE), and the list 4: `RecordSimEvent(10, 0, [x, y, z, 4], 4)`.

**D10. Lighting the cell (x, z).** Let c be the cell from `GetMapCell(x, z)` (one import) and T its type.

| Case | Imports after `GetMapCell`, in order | Written |
| --- | --- | --- |
| c is 0, `c.Cell.feature` is 0xfffb or more, `T.seqburn` is 0, `c.Cell.flag0` is set, or the pool is full | none | nothing |
| otherwise | `InitGafSequence(spot.anim, T.seqburn, 0)`; `InitGafSequence(spot.shadow, T.seqburnshad, 0)` when `seqburnshad` is not 0; the draw of the burn time; `los_radar.SoundHeardMask(position)`; `RecordSimEvent(31, 0, [1, x, y, z, mask], 5)` (`SIM_EVENT_SOUND_AT_NAME`, section 2); `network.SendFeatureBurning(x, z)` | below |

A spot is taken (D5). Its `feature` becomes `c.Cell.feature`; `x` and `z` become x and z; `timer` becomes the burn
time, b + `RandomInt`(b), where b is `T.burnTime` halved (rounded down), kept to its low 8 bits (Q11); in `flags`,
`used` becomes 1, `hasShadow` 1 when `seqburnshad` is not 0 and 0 otherwise, and `noSend` 0, the other bits as D5 left
them. `c.Cell.spot` becomes the spot's index and `c.Cell.flag0` 1. The sound's position is (x × 2^20,
`c.Cell.height` × 2^16, z × 2^20).

**D11. Felling the cell (x, z), by a hit (r = 0) or by a reclaim (r = 1).** c is the cell from `GetMapCell(x, z)`; when
its `Cell.feature` is 0xfffe, x and z become the origin's position (D2) and c the cell from a second `GetMapCell` of
that position. T is c's type. The first cell is never 0 (D11 is reached only for a cell that holds a feature) and
neither is the origin of a covered cell. For a sprite type the animation pair is (`seqdie`, `seqdieshad`) when r is 0
and (`seqreclamate`, `seqreclamateshad`) when r is 1; for a model type there is none.

| Case | Imports after the `GetMapCell`s | Written |
| --- | --- | --- |
| `c.Cell.feature` is 0xfffb or more | none | nothing |
| the pair's animation is not 0, and `c.Cell.flag0` is set or the pool is full | none | nothing |
| the pair's animation is not 0 | `InitGafSequence(spot.anim, animation, 0)`; `InitGafSequence(spot.shadow, shadow, 0)` when the shadow is not 0 | a spot is taken (D5): its `feature` becomes `c.Cell.feature`, `x` and `z` x and z, and in `flags` `used` 0, `reclaimed` r, `hasShadow` 1 or 0 as the shadow is given, `bit4` r; `c.Cell.spot` becomes its index and `c.Cell.flag0` 1 |
| otherwise (a model type, or no animation) | those of D12 for (x, z) and r | those of D12 |

**D12. Replacing the feature at (x, z) by its dead type, with r = 0 or 1.** c is the cell from `GetMapCell(x, z)` and T
its type. The new type v is `T.reclamate` when r is 1, otherwise `T.dead`.

| Case | Imports after `GetMapCell`, in order | Written |
| --- | --- | --- |
| c is 0, or `c.Cell.feature` is 0xfffb or more | none | nothing |
| `c.Cell.flag0` is set | a plain remove of c (D8); a placement (D9) of v on c with the position and rotation below, owner 10 | the remove's and the placement's |
| otherwise | a plain remove of c; a placement of v on c with no position, no rotation, owner 10 | the same |

When `c.Cell.flag0` is set, s is the spot `c.Cell.spot`: when bit 1 (`reclaimed`) of `s.flags` is set, v is
`T.reclamate` whatever r is (Q5). The placement's position is the three words of `s.pos` and its rotation `s.rot`,
read after the remove; for a sprite spot these are the bytes of its cursors (Q4). The placement is made even when the
remove gave 0 (Q6).

**D13. Burning out the spot s.** c is the cell from `GetMapCell(s.x, s.z)`. When c is 0 nothing more happens.
Otherwise a plain remove of c (D8), then, when the burnt type `Game.features[s.feature].burnt` is not 0xffff, a
placement of it on c with no position, no rotation and owner 10 (D9), even when the remove gave 0 (Q6).

**D14. A hit on the cell (x, z) by weapon w** (`Game.weapons`[w]), where c is the cell at (x, z) (D1, no import) and T
its type. The result is 1 when the hit killed the feature.

| Case | Imports, in order | Written | Result |
| --- | --- | --- | --- |
| bit 3 of `Game.uiOptionFlags` is clear | none | nothing | 0 |
| `c.Cell.feature` is 0xfffb or more, or T has the indestructible bit | none | nothing | 0 |
| `GetGameType` is 3 and the `host` bit of this machine's player (`Game.players[Game.playerIndex]`) is clear | `mission.GetGameType`; `network.SendFeatureHit(address of the weapon definition, x, z)` | nothing | 0 |
| T has the flamable bit, the weapon's `fireStarter` is not 0, and `c.Cell.flag0` is clear | `GetGameType`; those of lighting (x, z) (D10) | D10's | 0 |
| `c.Cell.flag0` is set and T is a sprite type | `GetGameType` | nothing | 0 |
| `c.Cell.flag0` is set, and the spot s = `c.Cell.spot` has (`s.x`, `s.z`) other than (x, z) | `GetGameType` | nothing | 0 |
| `c.Cell.flag0` is set: `s.damage` + the weapon's `damage`, kept to 16 bits, is less than `T.damage` | `GetGameType` | `s.damage` becomes that sum | 0 |
| `c.Cell.flag0` is set and that sum is `T.damage` or more | `GetGameType`; those of felling (`s.x`, `s.z`) with r = 0 (D11); in game type 3, `network.SendFeatureKilled(x, z)` | `s.damage` becomes the sum; D11's | 1 |
| `c.Cell.flag0` is clear: the weapon's `damage` + `c.Cell.spot` is less than `T.damage` | `GetGameType` | `c.Cell.spot` becomes that sum, kept to 16 bits | 0 |
| `c.Cell.flag0` is clear and that sum is `T.damage` or more | `GetGameType`; felling (x, z) with r = 0; in game type 3, `SendFeatureKilled(x, z)` | D11's | 1 |

`GetGameType` is made once, in the cases that reach it. A kill whose felling does nothing (a full pool, Q13) still
gives 1.

**D15. A reclaim by unit u at position p.** x and z are `p.x` and `p.z` divided by 2^20, rounded toward zero. c is the
cell from `GetMapCell(x, z)` and a second `GetMapCell(x, z)` gives the cell the type is read from, d: T is the type of
d's origin (D2). There is no type when d is 0, when `d.Cell.feature` is below 0xfffb and not below
`Game.featureCount`, when it is 0xfffb or more and not 0xfffe, or when it is 0xfffe and the origin's `Cell.feature` is
0xfffb or more.

| Case | Imports after the two `GetMapCell`s, in order | Written | Result |
| --- | --- | --- | --- |
| no type | none | nothing | 0 |
| `c.Cell.flag0` is set and T is a sprite type (Q9) | none | nothing | 0 |
| otherwise | INCOME (energy); INCOME (metal); those of felling (x, z) with r = 1 (D11); `mission.GetGameType`; when it is 3, `network.SendFeatureReclaimed(address of u, x, z)` | D11's | 1 |

The two INCOMEs are `EconomyIncome` payloads (24 bytes) of kind 4 (economy's INCOME), player `u.Unit.playerIndex`,
unit u's block offset, and: resources 1, energy `T.energy`, metal 0.0; then resources 2, energy 0.0, metal `T.metal`.
The floats are copied bit for bit.

**D16. Updating spot i.** s is spot i and T = `Game.features[s.feature]`. The result is `s.prev` as it was before the
update.

| Case | Imports, in order | Written |
| --- | --- | --- |
| model spot, `vel` all 0 | none | spot i moves to the rest list (D4) |
| model spot, `vel` not all 0 | `map.GetCellMeanHeight(address of s.pos)`, after `pos` is moved | `pos` becomes `pos` + `vel` (each word wrapping); with g = the height returned × 2^16: when `pos.y` ≤ g, `pos.y` becomes g and `vel` (0, 0, 0); else when `pos.y` < `Game.seaLevel` × 2^16, `vel` becomes (0, -11468, 0); else `vel.y` becomes `vel.y` - `Game.gravity` |
| sprite spot, `used` set (burning) | the smoke when `Game.ticks` (unsigned) is a multiple of 3; `StepGafSequence(s.anim)`; `StepGafSequence(s.shadow)` when `hasShadow` is set; then: when `s.anim.src` is 0, those of burning out (D13); else when `timer` is above 0 and `noSend` is clear, and `timer` - 1 is 0, those of the spread (D17) | when `s.anim.src` is 0: D13's; else when `timer` is above 0 and `noSend` is clear: `timer` drops by 1, and at 0 D17's |
| sprite spot, `used` clear (dying) | `StepGafSequence(s.anim)`; `StepGafSequence(s.shadow)` when `hasShadow` is set; when `s.anim.src` is then 0, those of replacing (`s.x`, `s.z`) with r = 0 (D12) | the cursors; D12's |

**The smoke:** the footprint centre of (`s.x`, `s.z`) for T (D3); `GetGafFrame(s.anim.src, s.anim.index)`, the current
frame f; two `GameRand(0)` draws; then `RecordSimEvent(28, 0, [x, y, z, 5], 4)` (`SIM_EVENT_WHITE_SMOKE`), with x and
y moved as section 5, "The smoke", gives.

**D17. The spread from a burning spot s of type T at (cx, cz).**

1. **Around:** each cell position (x, z) with z from cz - 3 to cz + 3 and, within each z, x from cx - 3 to cx + 3,
   but (cx, cz) itself: `GetMapCell(x, z)`; when the cell is not 0, its `Cell.feature` is below 0xfffb, its
   `Cell.flag0` is clear and its type has the flamable bit: a draw `RandomInt(100)`, and when the draw is below that
   type's `spreadChance`, those of lighting (x, z) (D10).
2. **Downwind:** a point starts at (cx × 2^16, cz × 2^16), and (lx, lz) at (cx, cz). Five times: the point's x grows
   by 2 × `Game.windX` and its z by 2 × `Game.windZ`; the cell position (x, z) is the point's high 16 bits, as signed
   numbers (section 5); when (x, z) is not (lx, lz), the same test as in step 1 is made on (x, z) (`GetMapCell`, and
   the draw and the lighting when the cell passes), and (lx, lz) becomes (x, z).
3. **The fire's damage:** when `T.burnweapon` is not 0: the footprint centre of (cx, cz) for T (D3), then an
   AREA_DAMAGE (`FeaturesAreaDamage`, 20 bytes) of the weapon's number, (`burnweapon` - the block offset of
   `Game.weapons`) / 277, and that centre.

**D18. Animating type k.** When `Game.features[k]` has the animating bit: `StepGafSequence(address of its anim)`,
then `StepGafSequence(address of its shadowAnim)`. Otherwise nothing. The results are ignored.

**D19. The scan step.** `Game.scanIndex` drops by 1. When it is then below 0, it becomes `Game.width` ×
`Game.height` - 1, and nothing else happens. Otherwise, with c the cell of that index and T its type:

| Case | Imports, in order | Written |
| --- | --- | --- |
| `c.Cell.feature` is 0xfffb or more, or `c.Cell.flag0` is set | none | the index only |
| the draw `RandomInt(100)` is `T.seedChance` or more | none (the draw) | the index |
| otherwise | the draw; the x draw; the z draw; `GetMapCell(x, z)`; when that cell t is not 0, `c.Cell.unit` is 0 and `t.Cell.feature` is 0xffff, those of placing `c.Cell.feature` on t with no position, no rotation, owner 10 (D9) | the index; D9's |

x is the index modulo `Game.width` plus `RandomInt(T.seedSpread)` minus `T.seedSpread` / 2 (rounded down); z is the
index divided by `Game.height` (Q1) plus `RandomInt(T.seedSpread)` minus `T.seedSpread` / 2. The x draw comes before
the z draw. The test reads the scanned cell's unit, not the target's (Q2).

### `UpdateFeatures()`

The features phase. Its effects, in order: for k from 0 to `Game.featureCount` - 1 (the count read once, first), D18
for k; then D19; then, from i = `Pool.usedHead` (read once, after the scan) while i is not -1, D16 for spot i, i
becoming D16's result. No recording holds it ("Kit gaps").

### `AnimateFeatureType(type)`

The effects of D18 for `type`.

### `ScanFeatureCell()`

The effects of D19.

### `FirstFeatureSpot()`

Returns `Pool.usedHead`. Writes nothing.

### `UpdateFeatureSpot(spot)`

The effects of D16 for spot `spot`, and returns D16's result.

### `FeatureTypeCount()`

Returns `Game.featureCount`. Writes nothing.

### `GetCellFeature(cell)`

Returns `Cell.feature` when it is below 0xfffb; the `Cell.feature` of the cell's origin (D2), whatever it is, when it
is 0xfffe; 0xffff otherwise. No import, no write.

### `FindFeatureAtPos(pos, cell, size)`

(x, z) is (`pos.x` / 2^20, `pos.z` / 2^20), each shifted right arithmetically and kept to its low 16 bits as a signed
number. p is the cell from `GetMapCell(x, z)`.

| Case | Imports | Written | Result |
| --- | --- | --- | --- |
| p is 0 | `GetMapCell` | nothing | 0xffff |
| `p.Cell.feature` is 0xfffe | `GetMapCell(x, z)`, then `GetMapCell` of (x - `offsetX`, z - `offsetY`), each kept to signed 16 bits; that cell then stands for p in the two rows below (it is never 0: a covered cell's origin is on the map) | | |
| `p.Cell.feature` is 0xfffb or more | as above | nothing | 0xffff |
| otherwise | as above | `cell`, when not 0, becomes the position of p (the shifted position, or the moved one); `size`, when not 0, becomes `Game.features[p.Cell.feature].footprint` | `p.Cell.feature` |

### `GetFootprintCentre(cell, def, out)`

`out` becomes the footprint centre of (`cell.x`, `cell.y`) for `def`'s footprint (D3), with one `GetGroundHeight`
import. Nothing else is written.

### `Apply(event, size)`

A size below 4 gives 0 before `kind` is read. Then by `kind`; an unknown kind gives 0 with no change. A size below the
kind's payload (PLACE 48, REMOVE 16, DAMAGE 16, RECLAIM 20) gives 0 with no change, and so does an event outside the
limits below (ozgb/btm#25: a malformed event changes nothing and does not trap). The recordings hold no event outside
them.

| Kind | Limits | Effects | Outcome |
| --- | --- | --- | --- |
| PLACE 32 | `x` from 0 to `Game.width` - 1, `z` from 0 to `Game.height` - 1; `feature` below `Game.featureCount`, or 0xffff, or 0xfffc; `owner` from 0 to 10 | placing `feature` (its low 16 bits) on the cell at (`x`, `z`) (D1), with the position `pos` when `flags` has `FEATURES_HAS_POS` (1) and none otherwise, the rotation of the low 16 bits of each `rot` word when `flags` has `FEATURES_HAS_ROT` (2) and none otherwise, and `owner` (D9). When a spot was placed and `flags` has `FEATURES_SINK` (4): the spot's `vel.y` becomes -11468 and its `vel.z` 0; `vel.x` stays (Q3) | 1 when a spot was placed, 0 otherwise (Q12) |
| REMOVE 33 | `x`, `z` on the map, as for PLACE | removing the feature of the cell at (`x`, `z`) (D8), forced when `force` is not 0 and plain when it is 0 | D8's result |
| DAMAGE 34 | `x`, `z` on the map; `weapon` from 0 to 255 | the hit on (`x`, `z`) by `weapon` (D14) | D14's result |
| RECLAIM 35 | `unit` the block offset of a unit of the unit table (not 0) | the reclaim by that unit at (`pos[0]`, `pos[1]`, `pos[2]`) (D15) | D15's result |
| REMOVE_ALL 36 | none | for each cell of `Game.cells` in index order whose `Cell.feature` is below 0xfffb or is 0xfffe, as the cell is when it is visited: a forced remove of that cell (D8) | 1 |

### `Place(place)`

The PLACE row of `Apply` for the payload `place`, with its limits, and its outcome as the result.

### The engine's slots

- **`InitFeatureAnimPool(info)`** allocates the 2048 spots and a pseudo-unit (`Pool.featureUnit`) and leaves the pool
  as section 3, "Between ticks", gives; it loads the map's feature types.
- **`LoadFeatureFileList`, `FreeFeatureFileList`, `NopAfterFeatureEnum`, `ResolveFeatureLinks`,
  `FindOrLoadFeatureType`** read the feature definitions and fill `Game.features` and `Game.featureCount`; the module
  reads their result (section 3, "Read").
- **`PlaceMissionFeatures`** places the mission's features as D9 does, with no position, no rotation and owner 10.
- **`StampFeatureMetal`** writes `Cell.metal` of the footprint of each indestructible feature with metal.
- **`FreeFeaturePool`** frees the spots, the piece states (FREE_STATE for each model spot of the rest list, then of
  the used list) and the types, and leaves `Game.features` 0 and `Game.featureCount` 0.
- **`SaveFeatures`** reads the cells and spots; **`LoadFeatures`** places the saved features as D9, D10 and D11 do.

## 5. Exactness

### Draws

- `RandomInt(n)` is `services.RandomInt` (ABI 9.3): for n below 2 it gives 0 and draws nothing (no stream word
  moves). Features draws `RandomInt(100)` in the scan (one per scanned cell that holds a feature with no spot), then
  `RandomInt(seedSpread)` twice, x first, when the seed test passes; `RandomInt(burnTime / 2)` once per lighting
  (D10), after the cursors are started and before the sound; `RandomInt(100)` once per tested cell of a spread (D17),
  in the order of D17, each draw before the lighting it may cause, so a lighting's own draw comes between two of the
  spread's.
- The smoke draws `GameRand(0)` twice (stream 0, `rng.main`), x first. A draw v (0 to 0x7fff) scaled to n is
  floor(v × n / 32768), with n the frame's width (or height) halved, rounded down, as an unsigned 16-bit number. The
  draws are made whatever the frame's size.

### Integers

- Cell positions in D1, D2, D9 and D15 are 32-bit: a cell's position is its index modulo `Game.width` and its index
  divided by `Game.width`, both of nonnegative numbers. D9's (ax, az), the PASS_MAPS fields and a spot's `x`, `z`
  are kept to signed 16 bits.
- D3: fx, fz, cx and cz are signed 16-bit numbers; x = (fx + 2 × cx) shifted left by 19, z likewise, y = the height
  shifted left by 16, each in 32-bit two's complement, wrapping.
- `FindFeatureAtPos`: x is `pos.x` shifted right by 20 (arithmetic), kept to 16 bits signed; z likewise from `pos.z`.
  The moved position subtracts the unsigned bytes `offsetX` and `offsetY` and keeps the result to 16 bits signed.
- D15: x = `p.x` / 2^20 and z = `p.z` / 2^20 in signed division rounded toward zero (-1 gives 0, not -1).
- D14: the weapon's `damage` and `c.Cell.spot` (both u16) are added as 32-bit numbers and compared with `T.damage`
  (u16); the stored sum is its low 16 bits. A spot's `damage` + the weapon's `damage` is kept to 16 bits before it is
  compared.
- D10: b = `T.burnTime` shifted right by 1 (u16); the burn time b + `RandomInt(b)` is stored as its low 8 bits.
- D16: the `timer` test is on the u8 `timer`; "a multiple of 3" is `Game.ticks` taken as unsigned 32 bits, modulo 3.
  `pos` + `vel` and `vel.y` - `Game.gravity` wrap in 32 bits; `Game.seaLevel` (u8) shifted left by 16 is compared
  signed with `pos.y`; g is the returned height shifted left by 16.
- The scan: x = index modulo `Game.width`, z = index / `Game.height` (Q1), both of the nonnegative index;
  `seedSpread` / 2 rounds down; x and z are then the 32-bit sums. The index wraps to `Game.width` × `Game.height` - 1
  when 1 less than it would be below 0.
- D17, downwind: the point's words start at cx × 65536 and cz × 65536, and each of the five steps adds 2 ×
  `Game.windX` (2 × `Game.windZ`) to the 32-bit word, wrapping; the cell position is the word's high 16 bits taken as
  a signed 16-bit number (the word shifted right by 16, arithmetic).
- D17, the weapon's number: the difference of two block offsets divided by 277, exactly.
- D4: a list head is read as a signed 16-bit number when it is stored in a `prev`.
- D9: the footprint tests ax + fx > `Game.width` and az + fz > `Game.height` are 32-bit signed; a footprint of
  0 in x or z visits no cell. The owner is stored as its low 4 bits.
- PLACE: `feature` is taken as its low 16 bits and `rot` as three signed 16-bit numbers, each the low 16 bits of its
  word.

### The smoke

Let (x, y, z) be the footprint centre of (`s.x`, `s.z`) for T, f the frame `GetGafFrame` returns, w its `width` and
h its `height` (`GafFrame`, u16), and dx, dy the two draws scaled to w halved and h halved (above). Each move is on the
high 16 bits of the word as a signed 16-bit number, wrapping in 16 bits, the low 16 bits unchanged:

- the high half of x grows by dx - `xOffset` + (w shifted right by 2), where `xOffset` is read as an unsigned 16-bit
  number;
- the high half of y grows by 2 × (`yOffset` - dy) - 2 × (h shifted right by 2), with `yOffset` read as an unsigned
  16-bit number;
- z is not moved.

### Floats

None is computed: `Feature.energy` and `Feature.metal` go into the INCOME payloads as their 32 bits, and 0.0 is the
bit pattern 0.

## 6. Quirks

- **Q1.** The scan's z is the scanned index divided by `Game.height`, not by `Game.width`.
- **Q2.** The seed lands only when the scanned cell has no unit (`c.Cell.unit` is 0); the target cell's unit is not
  read.
- **Q3.** A model spot's `vel` is not set by a placement: it keeps the three words a previous user of the spot left
  (for a spot that was a sprite spot, its shadow cursor from the `kind` byte on, then the four bytes after the
  cursor), so a placed model whose `vel` is not all 0 falls on its next update. SINK sets `vel.y` and `vel.z` only.
- **Q4.** Replacing a feature that holds a spot by its dead type places the new type at the three words of the spot's
  `pos` and with its `rot`, read after the old feature is removed: for a sprite spot these are its `anim` cursor's
  `kind` byte and the three bytes after it, its `anim.src`, and its `shadow` cursor's `index` and `value`, and a `rot`
  that no sprite writes.
- **Q5.** Taking a spot clears only bit 0 of its `flags`. A model spot's `reclaimed` bit is the one a previous user
  left, and when it is set the dead type is the `reclamate` type even for a hit.
- **Q6.** Replacing by the dead type and burning out make the placement even when the plain remove gave 0 (an
  indestructible feature); the placement then meets the feature still on the cell and is refused, except for the
  types 0xffff and 0xfffc.
- **Q7.** A placement refused part way leaves the cells it cleared cleared, with their imports made.
- **Q8.** DAMAGE reads the cell at (`x`, `z`) without resolving a covered cell (0xfffe), so a hit on a covered cell
  does nothing; a hit on a model compares the spot's cell with (`x`, `z`).
- **Q9.** RECLAIM refuses a sprite with a spot by the spot flag of the cell under the position, not of the origin.
  Through one of the covered cells of a burning or dying sprite, the two INCOMEs are published and the outcome is 1;
  the felling then does nothing when the type has a reclaim animation (the origin has its spot flag), and otherwise
  replaces the feature by its `reclamate` type (D12).
- **Q10.** A remove leaves `Cell.spot` (`offsetX`, `offsetY`, or the damage) and the owner bits of the cells it clears.
- **Q11.** The burn time is kept to 8 bits.
- **Q12.** A PLACE of a sprite type has outcome 0 though the feature stands on its cells.
- **Q13.** A hit that should fell a sprite whose death animation needs a spot, with the pool full, changes nothing and
  still has outcome 1.
- **Q14.** `GetCellFeature` returns the origin's `Cell.feature` for a covered cell even when it is a marker.

## 7. Examples

Every example is from the kit's recordings, on a map of `Game.width` 224 and `Game.height` 102.

**E1.** `tests/replay/features/commander-death.txt`, call at tick 182 of `GetFootprintCentre`: `cell` (50, 1),
`def` `Feature#6`, footprint (1, 1). x = (1 + 100) × 2^19 = 52953088, z = (1 + 2) × 2^19 = 1572864;
`map.GetGroundHeight pos=(52953088,1572864)` returns 1, so y = 65536. `out` is (52953088, 65536, 1572864). The next
call, (54, 4) for the same type, has ground 27: (57147392, 1769472, 4718592).

**E2.** Same file, the first `Apply` at tick 182: DAMAGE (`size` 16), `x` 50, `z` 1, `weapon` 213. `Game.uiOptionFlags`
12 has bit 3. The cell at (50, 1) is `Cell#274` (1 × 224 + 50) with `Cell.feature` 6 and no spot; `Feature#6` is a
sprite type, not flamable, with `damage` 0, and `WeaponHit#213` has `damage` 9999 and `fireStarter` 0. Imports:
`mission.GetGameType` = 2; the felling: `GetMapCell 50 1` = `Cell#274`, and since the death pair is (`FrameTable#9`,
null), `InitGafSequence Spot#0.anim FrameTable#9 0` and no shadow. The free head was 0: spot 0 moves to the used list
(`usedHead` 0, `freeHead` 1, `Spot#0.prev` -1, `Spot#1.next` -1); `Spot#0.cell` (50, 1), `Spot#0.feature` 6, its
`flags` stay 0; `Cell#274.flags` goes from 84 (owner 10) to 85, and its `Cell.spot` is 0, the spot's index
(unchanged). Outcome 1.

**E3.** `tests/replay/features/orders-air-fights.txt`, `Apply` at tick 709: PLACE (`size` 48), `x` 47, `z` 20,
`feature` 321, `flags` 3, `owner` 1, `pos` (50462720, 65536, 22282240), `rot` (0, 0, 0). The type has a model and a
footprint (2, 2) of free cells, so no remove. Imports: `scripts.ObjectStateSize Object3do#304` = 88; `SimAllocTagged
"Object State" 88` = `ModelState#1`; the INIT_STATE (`SchedPublish 5 12`); the PASS_MAPS (`SchedPublish 5 20`). The
free head was 2: `usedHead` 2, `freeHead` 3, `Spot#1.next` 2, `Spot#2.prev` 1, `Spot#3.next` -1; `Spot#2` gets the
state, `pos` (50462720, 65536, 22282240), `cell` (47, 20), `feature` 321. `Cell#4527` (20 × 224 + 47) gets
`feature` 321, `Cell.spot` 2, `Cell.flags` 9 (bit 0, owner 1); `Cell#4528` gets 0xfffe and `Cell.spot` 256
(`offsetX` 1, `offsetY` 0), `Cell#4751` 0xfffe and 1, `Cell#4752` 0xfffe and 257. Outcome 1.

**E4.** Same file, the next `UpdateFeatureSpot` of spot 2 at tick 709: a model spot with `vel` (0, 0, 0). It moves to
the rest list: `usedHead` 1, `restHead` 2, `Spot#1.next` -1, `Spot#2.prev` -1. No import. The result is 1, the
spot's `prev` before the move.

**E5.** `tests/replay/features/orders-reclaim-feature.txt`, `Apply` at tick 245: RECLAIM (`size` 20), `unit` `Unit#1`
(`Unit.playerIndex` 0), `pos` (41943040, 65536, 17039360). x = 41943040 / 2^20 = 40, z = 16. Imports:
`GetMapCell 40 16` twice (`Cell#3624`, which holds type 1, a sprite type with no spot); the two INCOMEs
(`SchedPublish 5 24`, twice); the felling with r = 1: `GetMapCell 40 16`, then `InitGafSequence Spot#0.anim
FrameTable#3 0` (the reclaim pair, no shadow); `mission.GetGameType` = 2, so no network call. Spot 0 is taken
(`usedHead` 0, `freeHead` 1) with `cell` (40, 16), `feature` 1 and `flags` 18 (`reclaimed` and `bit4`);
`Cell#3624.flags` becomes 85. Outcome 1.

**E6.** `tests/replay/features/orders-air-fights.txt`, `UpdateFeatureSpot` calls from tick 709: the dying spot 1 steps
its cursor each tick, `StepGafSequence Spot#1.anim` returning 0, 0, 1, 0, 0 at ticks 709 to 713, and returns 0 (its
`prev`), with no other import while `anim.src` stays `FrameTable#2`.

**E7.** `tests/replay/features/console-features.txt`, `Apply` at tick 90: REMOVE_ALL (`size` 4). Every feature of the
map is removed by a forced remove in cell order; each makes one PASS_MAPS (`SchedPublish 5 20`) and none holds a spot,
so there is no FREE_STATE. Outcome 1.

**E8.** `tests/replay/features/console-features.txt`, `ScanFeatureCell` at tick 90: `Game.scanIndex` goes from 22759
to 22758; the cell of index 22758 holds no feature, so nothing else is written and no stream word moves.

**E9.** `tests/replay/features/commander-death.txt`, an `Apply` at tick 182: DAMAGE at (33, 19), `weapon` 213. The
cell at (33, 19), `Cell#4289`, holds type 0, whose `flags16` 777 has the indestructible bit (bit 9). Outcome 0, with no
import (`mission.GetGameType` is not called) and no write.

**Not recorded.** No recording holds `UpdateFeatures` or `Place`, a scan that draws (every scanned cell holds no
feature or a feature with its spot flag), a PLACE of a sprite type, the removes inside a
placement, a refused or forced plain REMOVE, a DAMAGE that lights a feature, that damages a model or that is sent in
game type 3, a RECLAIM that is refused or that fells a model, a burning spot (the smoke, the burn out, the spread and
the fire's damage), the end of a dying animation (D12), a falling model, a seed that lands, a geothermal placement, a
full pool, and an animated type (`AnimateFeatureType` with the animating bit).

## 8. Kit version

`interfaces-v66` / `reference-traces-v55` (ozgb/byte-tactics-moddable@74bdf5ef6). Features interface 5.1
(`interfaces/features.h`, version 5.1).
