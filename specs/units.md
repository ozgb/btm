# units

## 1. Role

Units owns the unit table: every unit's record in the block, from the moment it is created to the moment it
dies. It creates units (finished, as a console or a capture makes them, or as a construction a builder has just
started), gives each its script, its model state, its weapons, its movement and its place on the map through
the other subsystems, and announces it. Once a tick, for each unit of each playing player, it counts the unit,
runs its wind generator, steps its timers, its health percentage, the sea's damage and its self-repair, puts it
back on the ground after it moved, and kills it when it is dying. A death credits the kill, ranks the players,
leaves a wreck and removes the unit from every subsystem. Units also answers the build and repair progress of
builders, says whether one unit can load, reclaim or repair another, keeps the references other subsystems'
orders hold to a unit, keeps each player's index of factories with a pad, finds unit types by name, and applies
the events other subsystems publish to change a unit (its flags, position, angles, stance, health, cargo and
owner).

## 2. Interface

### Slots (`interfaces/units.h`, version 7.2)

| Slot | When the engine calls it |
| --- | --- |
| `Apply` | for each event of units' inbox, in the order it was published (`docs/schedule.md`, "Routes") |
| `UpdateAllUnits` | the units phase (`docs/schedule.md`, the passes `units.players` and `units`) |
| `UpdateUnitHead`, `UpdateUnitBody`, `UpdateUnitTail` | once a tick for each unit of the units pass, in that order with the other subsystems' steps between them (`docs/schedule.md`, the pass `units`) |
| `KillUnit` | when the engine shuts a match down (each unit) |
| `CreateUnit` | when another subsystem makes a unit (a factory's or a builder's order, players' respawn) |
| `FindUnitType`, `FindUnitTypeId` | when another subsystem looks a unit type up by name |
| `AddBuildProgress`, `AddRepairProgress` | from a builder's order, each tick it builds or repairs |
| `CanLoad`, `CanReclaim`, `CanRepair` | queries of orders |
| `UnitRefInit`, `UnitRefSetUnit`, `UnitRefGet`, `UnitRefSetValue`, `UnitRefUnlink` | when orders and movement build, point, read or drop a reference to a unit |
| `CreateFactoryIndex`, `FreeFactoryIndex` | when a player's factory index is made or freed, outside the tick |
| `RefreshFactoryIndex` | once a tick for each player that has an index |
| `GetFactoriesInRadius` | when an aircraft looks for a repair pad |
| the 21 engine slots (below) | by the engine only |

The entry's `engine` slots stay in the engine (ABI section 6, "Engine slots", and section 9.3, "What stays in
the engine"): the module does not export them, and the host calls the engine's native slot, which reads and
writes the module's records in the block as section 3 lays them out. So the module's records must be exactly the
`layout.json` records of section 3, at the offsets it gives. The load-class slots that need engine calls stay
the engine's for now; none of them is in a recording.

### The manifest entry

The kit's manifest (`wasm/modules.json`) has the entry `units`, and the module must match it exactly (ABI 4.2
and 9.1):

- `name` and `interface`: `units`.
- `majors`: `units` 7, `los_radar` 5, `map` 9, `mission` 2, `network` 2, `orders` 6, `scripts` 3, `economy` 5,
  `services` 4.
- `data_base` 491520 (0x78000) and `table_base` 336: the module's window of `window` (32768) bytes and its
  `table_slots` (32) table entries (ABI 2.2 and 2.3).
- `blocked`: `true` in the kit's manifest. A blocked entry has no `fuel_budget` and no `size_budget` (ABI 4.2),
  and the gate judges a module of it on the recordings route, with `budgets: not_judged` (ABI section 15).
- `exports`, in this order: `Apply`, `UpdateAllUnits`, `KillUnit`, `CreateUnit`, `FindUnitType`,
  `FindUnitTypeId`, `AddBuildProgress`, `AddRepairProgress`, `CanLoad`, `CanReclaim`, `CanRepair`,
  `UnitRefUnlink`, `UpdateUnitHead`, `UpdateUnitBody`, `UpdateUnitTail`, `GetFactoriesInRadius`,
  `CreateFactoryIndex`, `FreeFactoryIndex`, `RefreshFactoryIndex`, `UnitRefInit`, `UnitRefSetUnit`,
  `UnitRefGet`, `UnitRefSetValue`.
- `engine`, each with the entry's reason: `UnitTypeSetAddTypeOrCategory`, `AllocateUnitMemory`,
  `FreeUnitMemory`, `FreeUnitInfo`, `RefreshUnitInfo`, `LoadUnitTypes`, `FreeUnitTypes`, `LoadDownloadMenus`,
  `FreeDownloadMenus`, `FreeMovementClasses`, `InitUnitCategories`, `FreeUnitCategories`, `KillAllUnits`,
  `KillUnitsOfType`, `ReloadUnitType`, `LoadUnits` and `LoadUnit` ("load class"); `SaveUnits` ("a query of a
  saved game, outside the tick"); `FindMissionUnit` ("only units itself calls it, through the seam, while it
  places the units of a mission at the mission's load, which no recording holds"); `WriteState` ("a dump query,
  called from the harness only"); `GetMovementLimits` ("a query of the game data, outside the tick").
- `data`: `g_game`, `g_simGlobals`, `g_extTdf`, `g_blockBase` (ABI 8.2). The module reads `g_game` (the `Game`
  record), `g_simGlobals` (the `SimGlobals` record, for `g_factoryIndex`) and `g_blockBase` (to turn a block
  offset into an address and back).
- `publishes`: `{"0": 12, "1": 60, "2": 24, "4": 24, "32": 48, "99": 12, "100": 20, "128": 8, "129": 8, "130":
  8, "131": 28, "137": 8, "138": 16, "160": 8, "161": 8, "162": 8, "163": 8, "164": 12, "165": 8, "166": 8,
  "169": 12, "193": 12, "194": 8, "195": 8, "226": 12, "256": 24, "258": 12, "259": 12, "261": 8, "262": 8,
  "267": 20, "288": 44, "290": 12, "291": 8, "292": 16, "320": 8, "321": 8, "322": 48, "323": 12, "324": 12,
  "416": 76}`. The route table (`data/routes.csv`, the rows with publisher `units`) carries each from units'
  outbox. The exported slots publish the kinds of "Effects" below; ECONOMY_LOAD (1), TARGETING_LOAD (100),
  LOS_RADAR_SIGHT_LOAD (138), MAP_RECLAIM_FOOTPRINT (163), MAP_SET_YARD (164), MAP_LOAD_PATH_LOCK_STAMP (169),
  ORDERS_ADD (288), MOVEMENT_LOAD (322) and MOVEMENT_SET_SPEED (324) are published only by engine slots.
- Fuel: the module meters itself by rule version 1 (ABI section 11) and exports `__fuel_set` and `__fuel_get`.
- Slot kinds:

| Slot | Kinds |
| --- | --- |
| `Apply` | `[linear, int] -> int` |
| `UpdateAllUnits` | `[] -> void` |
| `KillUnit`, `UpdateUnitTail` | `[block, int] -> void` |
| `CreateUnit` | `[int, int, linear, int, int, int] -> block` |
| `FindUnitType` | `[linear] -> block` |
| `FindUnitTypeId` | `[linear] -> int` |
| `AddBuildProgress`, `AddRepairProgress` | `[block, block, f32] -> int` |
| `CanLoad`, `CanReclaim`, `CanRepair` | `[block, block] -> int` |
| `UnitRefUnlink`, `UpdateUnitHead` | `[block] -> void` |
| `UpdateUnitBody` | `[block] -> int` |
| `GetFactoriesInRadius` | `[int, linear, int, linear, int] -> int` |
| `CreateFactoryIndex`, `FreeFactoryIndex`, `RefreshFactoryIndex` | `[int] -> void` |
| `UnitRefInit` | `[block, block, block] -> void` |
| `UnitRefSetUnit`, `UnitRefSetValue` | `[block, block] -> void` |
| `UnitRefGet` | `[block] -> block` |

A `block` argument or result is an address in the block (0 for null), as ABI 7 gives it. The `unsigned char`
`player` of `CreateUnit` is its `int` argument's low 8 bits, and its `unsigned short` `typeId` and `int` `id`
are taken as their low 16 bits, unsigned.

- Imports, every one listed and nothing else:

| Import | Kinds | Used for |
| --- | --- | --- |
| `los_radar.GetMapCell` | `[int, int] -> block` | the cell under a dying unit, for its wreck |
| `map.GetGroundHeight` | `[linear] -> int` | the ground under a unit, given the address of its `pos` (Height, Align's callers, Wreck) |
| `mission.GetGameType` | `[] -> int` | the rank pass and the last unit of a player, in a death |
| `mission.GetMissionParams` | `[linear] -> void` | the sea's damage, in `UpdateUnitBody` |
| `network.SendNewUnit` | `[block] -> void` | every created unit |
| `network.SendBuilderLink` | `[block, block] -> void` | a structure created finished, and a finished construction |
| `network.SendUnitKilled` | `[block, int, int, int] -> void` | the death of a simulated player's unit |
| `network.SendAttach` | `[block, block, int, int] -> void` | an attach or a detach |
| `network.SendOwnershipTransfer` | `[block, block] -> void` | SEND |
| `network.SendActivateFlags` | `[block] -> void` | a change of a simulated player's unit's activation bits |
| `orders.GetOrderFlags` | `[block] -> int` | ATTACKED_SPEECH |
| `scripts.QueryScript` | `[block, linear, linear, linear, linear, linear] -> int` | the `"Killed"` script of a dying unit |
| `scripts.CreateObjectState` | `[block] -> block` | the model state of a unit whose type has no script |
| `scripts.FreeObjectState` | `[block] -> void` | a dead unit's model state |
| `scripts.NewUnitScript` | `[block] -> void` | a new unit's script |
| `scripts.SetCob` | `[block, block] -> void` | a new unit's script |
| `scripts.CreatePlayerObjectState` | `[block, block, block] -> block` | a new unit's model state |
| `scripts.SetObjectState` | `[block, block] -> void` | a new unit's script |
| `scripts.DeleteScript` | `[linear] -> void` | a dead unit's script |
| `economy.UpdateMetalExtraction` | `[int] -> void` | every created unit |
| `economy.RequestGranted` | `[int, int] -> int` | build and repair progress |
| `services.RandomInt` | `[int] -> int` | a new unit's heading and hover phase |
| `services.SimAlloc` | `[int] -> linear` | the factory index |
| `services.RecordSimEvent` | `[int, int, linear, int] -> void` | the sim events of "Events" |
| `services.GetSlotDpid` | `[int] -> int` | a death: the network id of the killer's slot |
| `services.FindSlotByDpid` | `[int] -> int` | a death: the killer's slot again |
| `services.Emit` | `[linear, int] -> void` | every effect of "Effects" |

The recordings carry `Emit` as `services.SchedPublish 2 <size>` (2 is units' outbox), `economy.RequestGranted`
as `services.ScheduleEconomyRequestGranted Unit#N <resources>` and `economy.UpdateMetalExtraction` as
`services.ScheduleMetalExtraction Unit#N` (ABI 9.3, "Routes in the recordings"); each passes the unit's block
offset, which the recordings name as the unit. `RecordSimEvent` is recorded as `services.RecordSimEvent <kind>
<flags> [<arguments>] <count>`, `QueryScript` as `scripts.QueryScript <script> "<name>" (<arg0>) (<arg1>) null
null = <result>` followed by `o arg0` and `o arg1`, the two words the import wrote, and `GetGroundHeight` as
`map.GetGroundHeight pos=(<x>,<z>) = <height>`.

### Hooks

`Apply`, `FindUnitType`, `FindUnitTypeId`, `CanLoad`, `CanReclaim`, `CanRepair`, the `UnitRef*` slots,
`UpdateUnitHead`, the factory index slots and `GetFactoriesInRadius` say "Hooks: none". The rest name a hook
(ABI 6, "Hook points"): `UpdateAllUnits` GameFrame; `KillUnit` and `UpdateUnitTail` UnitDestroyed; `CreateUnit`
UnitCreated; `AddBuildProgress` UnitFinished; `AddRepairProgress` RepairAmount; `UpdateUnitBody` SelfHeal; and
the inbox kinds FINISH_CONSTRUCTION (UnitFinished), REPAIR (RepairAmount), SFX_OCCUPY (UnitWaterState), KILL
(UnitDestroyed) and CAPTURE (UnitCreated). Milestone 3 decides how a module raises a hook; until then the
module keeps each of these as its own call or event, and raises none.

### Events

The module records these kinds through `services.RecordSimEvent` (`docs/sim-events.md` has their arguments,
flags and phases; its Terms section, `docs/sim-events.md`, Terms, defines the unit flags, activation bits, kill
types, damage kinds, player types and game types this spec uses):

- `SIM_EVENT_UNIT_SPEECH`: a unit that is alive and not dying says a line: when it is switched on (slot 3) or off
  (slot 4), cloaked (slot 14) or uncloaked (slot 15), by Bits; and when ATTACKED_SPEECH finds it was attacked
  (slot 2, with flag 0x0001).
- `SIM_EVENT_SELECTION_ORDERS_DIRTY`: a unit's activation bits change (Bits).
- `SIM_EVENT_DESELECT_IF_INELIGIBLE`: a unit is attached or detached (Attach).
- `SIM_EVENT_BUILD_MENU_UPDATE`: a builder finishes a construction (Finish).
- `SIM_EVENT_GUI_CHANGED_IF_SCREEN`: a structure of a simulated player is finished (Finish), with screen 0.
- `SIM_EVENT_REMOVE_UNIT_SPEECH`: a unit dies (Death).
- `SIM_EVENT_CD_ACTIVITY`: a credited kill whose killer slot is `Game.localPlayer` (Death).
- `SIM_EVENT_SCORE_FLASH`: a credited kill with a killer slot (Death).
- `SIM_EVENT_PLAYER_TOOK_LEAD`: the rank pass puts the killer first (Death).
- `SIM_EVENT_PLAYER_LEFT` and `SIM_EVENT_FORCES_DESTROYED`: the last unit of a player dies, in game type 3 and 2
  (Death).
- `SIM_EVENT_SMOKE`: a wreck is made with its smoke plume (Wreck).
- `SIM_EVENT_POP_TO_GAME_LAYOUT`: a simulated player's commander dies under the commander-death rule (Kill).

The names in parentheses are the definitions of section 4.

### Arguments outside the block

All in the caller's memory (`linear`), little-endian, read in place:

- **The event of `Apply`**: a `UnitsEventHeader` (`kind`, then `unit`, a block offset) and its kind's payload,
  `size` bytes. The payloads (`interfaces/units.h`) and their sizes: SET_STATE_BITS, SET_FLAGS, SET_NET_DIRTY,
  REPAIR, SET_BUILD_STATE, SCRIPT_VALUE, DAMAGE_RECORD and SET_HEALTH 16; SET_WORK_TIME, FINISH_CONSTRUCTION,
  BUILD_DECAY, UNLINK, TURN, SET_MOVED, NOTIFY_REFS, KILL, SEND and RELOAD_TYPE 12; SET_POSITION 24; CREATE
  28; ATTACH_TO_PIECE 20; MOVE 20; SET_STANCE and SET_ANGLES 24; CHANGE_CELL 32; CAPTURE 48; SFX_OCCUPY,
  ATTACKED_SPEECH, MARK_DAMAGED, KILL_ALL and FORGET_ATTACKER the header alone (8). The recordings write the
  fields as `a event.<field>` and the size as `a size`.
- **`pos`** of `CreateUnit` and of `GetFactoriesInRadius`: a `Vec3` (three signed 32-bit words, x, y, z).
- **`out`** of `GetFactoriesInRadius`: an array of `capacity` 32-bit words, each written as a unit's block
  offset (a `Ref<Unit>`).
- **`name`** of `FindUnitType` and `FindUnitTypeId`: a NUL-terminated string.
- **What the module passes to imports**, in its own memory: a `MissionParams` (`interfaces/mission.h`, 124
  bytes) that `mission.GetMissionParams` fills, of which the module reads `waterDoesDamage` and `waterDamage`;
  the name `"Killed"` (NUL-terminated) and two 32-bit words for `scripts.QueryScript`, its `arg0` and `arg1`, the
  last two pointers 0; the argument array of `RecordSimEvent`; every effect payload of `Emit`.
  `map.GetGroundHeight` is given the address of the unit's own `pos` in the block.

### Effects

Each effect is a payload of `interfaces/<module>.h`, every byte written: a field not named is 0, a unit is its
block offset (0 for none), and a name is NUL-padded. Section 4 names them as below.

| Name in section 4 | Kind (value) | Payload, size | Fields |
| --- | --- | --- | --- |
| ECON_RESET(u) | ECONOMY_RESET (0) | `EconomyEventHeader`, 12 | `player` `u.playerIndex`, `unit` *u* |
| ECON_REQUEST(u, r, e, m) | ECONOMY_REQUEST (2) | `EconomyRequest`, 24 | `player` `u.playerIndex`, `unit` *u*, `resources` *r*, `energy` *e*, `metal` *m* (floats) |
| ECON_INCOME(u, m) | ECONOMY_INCOME (4) | `EconomyIncome`, 24 | `player` `u.playerIndex`, `unit` *u*, `resources` 2 (`ECON_METAL`), `energy` 0, `metal` *m* |
| PLACE(…) | FEATURES_PLACE (32) | `FeaturesPlace`, 48 | as Wreck gives them |
| TGT_RESET(u, w) | TARGETING_RESET (99) | `TargetingEventHeader`, 12 | `unit` *u*, `weapon` *w* |
| LOS_UPDATE(u), LOS_REMOVE(u), LOS_REVEAL(u), SIGHT_RESET(u) | LOS_RADAR_UPDATE_UNIT (128), LOS_RADAR_REMOVE_UNIT (129), LOS_RADAR_REVEAL_UNIT (130), LOS_RADAR_SIGHT_RESET (137) | `LosRadarUnit`, 8 | `unit` *u* |
| EYEBALL(u) | LOS_RADAR_ADD_EYEBALL (131) | `LosRadarAddEyeball`, 28 | `pos` *u*'s `pos`, `sightDistance` *d*.`range`, `eyeHeight` *d*.`modelMaxYHigh` (both sign-extended), `lifetime` 60 |
| MAP_ADD(u), MAP_REMOVE(u), MAP_CLEAR(u), UNLINK_BUCKET(u), PREPEND_BUCKET(u) | MAP_ADD_UNIT (160), MAP_REMOVE_UNIT (161), MAP_CLEAR_UNIT (162), MAP_BUCKET_UNLINK (165), MAP_BUCKET_PREPEND (166) | `MapEventHeader`, 8 | `unit` *u* |
| KILL_UNITS(p) | PLAYERS_KILL_UNITS (193) | `PlayersKillUnits`, 12 | `unit` 0, `player` *p* |
| CREATED(u), DIED(u) | PLAYERS_UNIT_CREATED (194), PLAYERS_UNIT_DIED (195) | `PlayersEventHeader`, 8 | `unit` *u* |
| SQUAD(u, s) | AI_SET_SQUAD (226) | `AiSetSquad`, 12 | `unit` *u*, `squad` *s* |
| DAMAGE(src, u, a, k) | WEAPONS_DAMAGE (256) | `WeaponsDamage`, 24 | `unit` *u*, `source` *src*, `amount` *a*, `damageKind` *k*, `direction` 0 |
| DETONATE(u, s) | WEAPONS_DETONATE (258) | `WeaponsDetonate`, 12 | `unit` *u*, `selfDestruct` *s* |
| RELEASE(u, w) | WEAPONS_RELEASE (259) | `WeaponsRelease`, 12 | `unit` *u*, `weapon` *w* |
| INIT_SLOTS(u), REMOVE_SHOTS(u) | WEAPONS_INIT_SLOTS (261), WEAPONS_REMOVE_PROJECTILES (262) | `WeaponsEventHeader`, 8 | `unit` *u* |
| STOCKPILES(u, a, b, c) | WEAPONS_SET_STOCKPILES (267) | `WeaponsSetStockpiles`, 20 | `unit` *u*, `stockpiles` *a*, *b*, *c* |
| STOP(u) | ORDERS_STOP (290) | `OrdersStop`, 12 | `unit` *u*, `all` 1 |
| BE_CARRIED(u) | ORDERS_BE_CARRIED (291) | `OrdersEventHeader`, 8 | `unit` *u* |
| STATUS(u, o, f) | ORDERS_STATUS_FLAGS (292) | `OrdersStatusFlags`, 16 | `unit` *u*, `order` *o*, `flags` *f* |
| MOVE_NEW(u), MOVE_DESTROY(u) | MOVEMENT_NEW (320), MOVEMENT_DESTROY (321) | `MovementEventHeader`, 8 | `unit` *u* |
| MOVE_MODE(u, m) | MOVEMENT_SET_MODE (323) | `MovementSetMode`, 12 | `unit` *u*, `mode` *m* |
| START(u, n, up) | SCRIPTS_START (416) | `ScriptsStart`, 76 | `unit` *u*, `form` 0 (`SCRIPTS_START_BY_NAME`), `name` *n*, `update` *up*; `index`, `callback`, `count` and `args` 0 |
| START_ARG(u, n, up, a) | SCRIPTS_START (416) | `ScriptsStart`, 76 | `unit` *u*, `form` 1 (`SCRIPTS_START_BY_NAME_WITH_ARGS`), `name` *n*, `update` *up*, `count` 1, `args[0]` *a* |

Writes the module makes before an `Emit` are in the block when the effect is delivered: a synchronous route
(`data/routes.csv`, `delivery`) delivers the effect before `Emit` returns, and its subscriber's writes are in the
block when the module reads it again. The recordings carry a subscriber's writes that the module then reads as
`o` lines after the `Emit` (for example `o Unit#251.motion null` after MOVE_DESTROY).

### Kit gaps

What this spec needs and the kit doesn't give; the maintainer routes them:

- **K1, the whole pass.** `UpdateAllUnits` is exported, but the units pass calls other subsystems' slots
  (`docs/schedule.md`, the passes `units.players` and `units`, and the list of `units.UpdateAllUnits`: weapons,
  scripts, orders, movement), and the entry lists none of them as an import. No recording holds the slot. The
  same gap as players' `UpdatePlayers`.
- **K2, a wreck under the sea.** A wreck whose cell is at or below the sea level is placed by a synchronous call
  of `features.Place`, whose answer decides the smoke plume (Wreck). The entry has no import for it, the ABI's
  table of recorded calls has none, and no recording holds the case (the kit's next interfaces tag adds it).
- **K3, `services.SimFree`.** The factory index frees a vector's storage when the vector grows and when
  `FreeFactoryIndex` frees it, which needs `services.SimFree`; the entry doesn't list it. No recording holds
  either: the recorded vectors never grow and `FreeFactoryIndex` is not recorded.
- **K4, the class word of a reference.** A `PathOrderAttach` (section 3) has a class word at bytes 0 to 3
  (`vptr` true in `layout.json`), which the reference writes when `UnitRefInit` builds the reference and for
  which the kit gives no value. The recordings name the object and never the word. The module leaves the word
  as the caller left it. The same family as the class words that players' and movement's specs found.
- **K5, the effects' fields.** The recordings carry an `Emit` by its size only (`SchedPublish 2 <size>`), so a
  replay checks neither the kind nor the fields of section 2, "Effects"; only the full scenarios judge them, and
  the order of two effects of the same size is checked by size alone. The same gap as other specs found.
- **K6, the vector's words.** `layout.json` gives `FactoryIndexEntry.factories` as 16 bytes of type
  `BlockVector<struct Ref<struct Unit> >` with no fields, and `docs/replay-format.md`, "Vectors", names the
  first, last and end words without their offsets. Section 3 states the layout the engine slots read.
- **K7, FORGET_ATTACKER.** The header says that the kind flags the unit's panel to refresh; the bit it sets is
  bit 14 of `Unit.flags`, the dying flag (Q6).
- **K8, the reload of a type.** RELOAD_TYPE reloads the type's definition through the load-class engine slot
  `ReloadUnitType`, which no import reaches; the module does the kills and not the reload. The case is
  unrecorded and load class.
- **K9, raising a hook.** The kit gives no way for a module to raise a hook (section 2, "Hooks").

## 3. State

Field names are `layout.json`'s; a name in `code` with no record is a field of the unit *u*, and *d* is
`u.def`, the unit's `UnitDef`. The recordings use some other names for the same bytes: `posX`, `posY`, `posZ`
are `pos.x`, `pos.y`, `pos.z`; `bank`, `heading` and `pitch` are the angles; `sfxOccupy` is the 32-bit word at
`unknown_10a` (bytes 266 to 269); `weapon0Stockpile` to `weapon2Stockpile` are `weapons[i].stockpile`;
`FeatureHeap#N.dead` is `Feature.dead` of `Game.features[N]`; `UnitModel#N.model` is `Game.types[N]`.

### Owned

**The unit table**: `Game.units`, `Game.poolCount` records of `Unit` (280 bytes), from `Game.units` to
`Game.unitsEnd` (the last record). Record *k* is the unit of id *k*, and its `id` holds *k*. Each player has the
slice `Player.unitsBegin` to `Player.unitsEnd` (both records of the slice, so the slice is closed); a unit's
`player` points at the player of its slice and `playerIndex` holds that player's index. The engine's load-class
slots lay the table out; the module never changes `id`, `player`, `playerIndex` or the slices. A record whose
`unitDefIndex` is 0 holds no unit.

The fields of a unit the module writes: `pos`, `angles`, `cell`, `footprint`, `carrier`, `cargo`, `cargoNext`,
`def`, `script`, `state`, `unitDefIndex`, `hoverBobPhase`, `group`, `workTime`, `killCount`, `netDirtyFlags`,
`owner`, `attacker`, `lastAttackerSlot`, `lastDamageType`, `healthPercent`, `prevHealthPercent`,
`transportPiece`, `recentlyDamagedTimer`, `postTransferHoldoff`, `unfinishedInitZero`, `buildLeft`, `health`,
the `sfxOccupy` word, `activateFlags`, `cobStateFlags`, `flags` and `zBufferFlag`. The bits of the words:

| Word | Bit (value) | `layout.json` name | Meaning |
| --- | --- | --- | --- |
| `flags` | 0 and 1 (3) | `mode` | the movement mode: 1 on the ground, 2 in the air |
| | 2 and 3 | `unknown_110_2` | cleared at creation |
| | 4 (0x10) | `selected` | the unit is selected |
| | 5 (0x20) | `selectable` | the unit can be selected |
| | 8 (0x100), 9 (0x200) | `unknown_110_8`, `ownerIsLocal` | unit flags 0x300 of `docs/sim-events.md`, Terms |
| | 10 (0x400) | `unknown_110_10` | cleared at creation |
| | 11 (0x800) | `cloakOn` | the cloak order |
| | 13 (0x2000) | `buildLeftChanged` | the build progress changed |
| | 14 (0x4000) | `killPending` | unit flag 0x4000, the unit is dying (Terms) |
| | 16 (0x10000) | `moved` | the unit moved: its height is set again (Height) |
| | 17 (0x20000) | `carriedNoPiece` | carried, at no piece |
| | 18 and 19 | `moveOrder` | the movement stance |
| | 20 and 21 | `fireOrder` | the fire stance |
| | 22 and 23, 24 and 25, 26 and 27 | `f22_23`, `f24_25`, `f26_27` | build menu state |
| | 28 (0x10000000) | `alive` | unit flag 0x10000000, the unit is alive (Terms) |
| | 29 (0x20000000) | `immobile` | unit flag 0x20000000, a structure (Terms) |
| | 30 | `cargoSelectable` | cargo of this unit can be selected |
| | 31 | `armed` | the unit is armed |
| `activateFlags` | 0 (1) | `activate_bit0` | activation bit 0, switched on (Terms) |
| | 2 (4) | `activate_bit2` | activation bit 2, cloaked (Terms) |
| | 3 (8) | `activate_bit3` | building |
| `cobStateFlags` | 0, 1, 2, 3 | `bit0` to `bit3` | script flags |
| `netDirtyFlags` | 2 (4) | `dirty` | the unit is dirty for the network |
| `zBufferFlag` | 0 | `zBufferBit0` | from the type |

**The players' counters**: `Player.unitCount` (unsigned 16-bit), `unitsCreated` (32-bit), `kills`, `losses`,
`commanderKills`, `commanderLosses` (signed 16-bit), `rank` (8-bit) and bit 0 of `flags`.

**`Game.f14353`**, the count of unit heads (32-bit), and bit 4 (0x10) of `Game.flags_37ebe`.

**The factory index**: `SimGlobals.g_factoryIndex`, a block offset to a `FactoryIndex` (240 bytes, 0 until the
first `CreateFactoryIndex`), whose `players[p]` (`FactoryIndexEntry`, 24 bytes) is player *p*'s: `live` (1 while
the player has an index), `lastTick` (the tick of its last refill, unsigned) and `factories`, a vector of unit
block offsets. The vector is 16 bytes: bytes 0 to 3 are 0, then three block offsets, *first* (the storage, 0 for
none), *last* (one past the last element) and *end* (one past the room), each 0 when there is no storage (K6).
Its storage is a `services.SimAlloc` allocation of 4 bytes per element of room. The recordings write a vector as
`[<length>/<room>] Unit#a,Unit#b`.

**References** (`PathOrderAttach`, 16 bytes, in memory the caller owns): `owner` (the unit it points at, or 0),
`next` (the next reference of that unit's list, or 0) and `value` (the order that listens, or 0). A unit's `head`
starts its list: `head`, then each reference's `next`, until 0. Bytes 0 to 3 are the class word (K4).

**A unit's model state** (`ModelState`, made by scripts' imports): the module writes its `owner` (a unit
without a script) and its `bitmap` (made 0), and stores the state in `u.state`.

### Read

`Game`: `ticks`, `width`, `height`, `seaLevel` (unsigned 8-bit), `mapFlags`, `playerIndex`, `localPlayer`,
`commanderDeath`, `windDirection`, `windSpeed`, `windEnabled`, `unitDefCount`, `defs`, `types`, `cells`,
`features`, `players_1b63` (the 11 `Player` records; `Game.players` is the first 10), `sideDefs`. `Player`:
`active`, `type`, `index`, `id`, `info`, `allied`, `shareVision`, the slice. `PlayerInfo`: `side`, `bit6`.
`SideDef`: `commander`. `UnitDef` (the type *d*): `unitname`, `id`, `hp`, `maxHealth`, `buildtime`,
`energyCost`, `metalCost`, `healtimeUnsigned`, `buildangleUnsigned`, `origin`, `footprintx`, `capacity`,
`transportcapacity`, `draft`, `range`, `modelMaxY`, `modelMaxYHigh`, `maxwaterdepth`, `minwaterdepth`,
`maxvelocity`, `mobile`, `maxThisUnit`, `corpseUnsigned`, `buildMenuPageCount`, `windgenerator`, `data` (the
unit type's script, load data), `ids`, `flags1`, `flags2`. `Cell.height`; `Feature.dead`; `UnitMotion.speed`
and `UnitMotion.lastMoveTick` of `u.motion` (movement's record); `Game.types[t]`, the unit type's model (an
`Object3do`, load data: `firstFace`, `vertices`, `faces`, and of the face `PieceFrame.indices`).

The bits of the type's words that the module reads:

| Word | Value | Meaning here |
| --- | --- | --- |
| `flags1` | 0x3 (bits 0 and 1), 0xc (bits 2 and 3) | the type's movement and fire stances |
| | 0x10 | `initCloaked` |
| | 0x40, 0x200 | `canbuild` and bit 9: together, a factory with a pad |
| | 0x80, 0x200, 0x10000 | read into `zBufferBit0`, `cargoSelectable` and `armed` |
| | 0x800 | `flying` |
| | 0x1000 | bit 12: the unit floats (and, with its movement, hovers) |
| | 0x40000 | bit 18: switched on when finished |
| | 0x80000, 0x100000 | bits 19 and 20: the unit moves on the water, over the water |
| | 0x200000 | bit 21: can repair under the sea |
| | 0x800000 | bit 23: the type can be created |
| | 0x1000000 | bit 24: isfeature (Terms); and a wreck under the sea does not sink |
| `flags2` | 0x100 | can transport |
| | 0x200 | can repair |
| | 0x400 | can reclaim |
| | 0x1000 | cannot be reclaimed |
| | 0x80000 | cannot be loaded |

### Between ticks

Everything is in the block: the unit table, the players' counters, the factory index and its vectors, the
references. The module keeps nothing in its window between calls. `rng.seed`, the state of
`services.RandomInt`, is the engine's.

## 4. Behaviour

Within one call, only the order of import calls is observable. The block writes are compared at the end of the
call, so the order they are listed in carries no meaning. Every `Emit` of this section is an import call and
keeps its place in the order.

### Definitions

- *u*, *t*, *b*: units; *d* is `u.def`; *P* is `u.player`. *P* is **simulated** when `P.active` ≠ 0 and `P.type`
  is 1 or 2 (player types 1 and 2, Terms). A unit is **alive** when `flags` has 0x10000000 and **dying** when it
  has 0x4000.
- **A unit offset is valid** when it is 0 or the block offset of a record of the unit table.
- **Cell(p, f)**, the cell of a position *p* for a footprint *f*: (`(p.x − f.x × 0x80000 + 0x80000) >> 20`,
  `(p.z − f.y × 0x80000 + 0x80000) >> 20`), each kept to its low 16 bits, signed; `f.x` and `f.y` are
  sign-extended and the arithmetic is 32-bit, wrapping, with arithmetic shifts.
- **Commander(u)**: `Game.sideDefs[P.info.side].commander` equals *d*.`unitname` with `_strcmpi` (case-insensitive,
  the kit's runtime `strcmpi`).
- **Speak(u, k)**, **Speak(u, k, f)**: when *u* is alive and not dying, `RecordSimEvent` of
  `SIM_EVENT_UNIT_SPEECH` with flags *f* (0 when not given) and the three arguments `u.id`, *k*, 0.
- **Event(kind, args)**: `RecordSimEvent` of the kind with flags 0 and the arguments.
- **Simulated(u)** below means *u*'s *P* is simulated.

**Bits(u, m, s)**, the activation bits under a mask: *o* = `activateFlags`; *n* = *o* with the bits of *m*'s low
byte set when *s* ≠ 0, cleared when *s* is 0; `activateFlags` becomes *n*. When *n* = *o* nothing else happens.
Otherwise, with *g* the bits gained and *l* the bits lost, in order:

1. *g* has 1: START(u, `"Activate"`, 0), then Speak(u, 3);
2. *l* has 1: START(u, `"Deactivate"`, 0), then Speak(u, 4);
3. *g* has 8: START(u, `"StartBuilding"`, 0);
4. *l* has 8: START(u, `"StopBuilding"`, 0);
5. *g* has 4: Speak(u, 14), then STATUS(u, r.`value`, 0x10000) for each reference *r* of *u*'s list whose `value`
   is not 0, in list order (each `next` read after the effect before it);
6. *l* has 4: Speak(u, 15);
7. Event(`SIM_EVENT_SELECTION_ORDERS_DIRTY`, [`u.id`]);
8. when Simulated(u): `network.SendActivateFlags(u)`.

**Height(u)**: when `flags` has 0x10000 or *d*.`flags1` has 0x1000: 0x10000 is cleared from `flags`; then, only
when `u.motion` is not 0 and `flags & 3` is 1:

| *d*.`flags1` | `pos.y` becomes |
| --- | --- |
| has 0x100000, and 0x1000 | max(*h*, `seaLevel` − *d*.`draft`) × 65536, *h* = `map.GetGroundHeight(&u.pos)` |
| has 0x100000, not 0x1000 | *h* × 65536, *h* = `map.GetGroundHeight(&u.pos)` |
| has 0x80000, not 0x100000 | (`seaLevel` − *d*.`draft`) × 65536 (no import) |
| neither | Align(u) (no import) |

**Align(u)**, putting a unit on the ground from its model: *M* is `Game.types[u.unitDefIndex]`; when
*M*.`firstFace` < 0, nothing. Otherwise *F* is the face `M.faces[M.firstFace]` (32-byte `PieceFrame` records),
and for each corner *k* = 0 to 3, *v* is the `Vec3` `M.vertices[F.indices[k]]` (`indices` 16-bit unsigned):

- (*rx*, *rz*) = Rot(`heading`, *v.x*, *v.z*);
- *wx* = (*rx* + `pos.x`) >> 16 and *wz* = (`pos.z` − *rz*) >> 16, each kept to signed 16 bits and then
  sign-extended; *gx* = *wx* >> 4 and *gz* = *wz* >> 4 as unsigned 32-bit numbers, *fx* = *wx* & 15, *fz* = *wz* & 15;
- when *gx* ≥ `Game.width` − 1 or *gz* ≥ `Game.height` − 1 (unsigned), Align stops at once and writes nothing;
- with *H*(x, y) the `Cell.height` of the cell `y × width + x`: *A* = *H*(gx, gz) + ((*H*(gx+1, gz) − *H*(gx, gz))
  × *fx*) / 16 and *B* = *H*(gx, gz+1) + ((*H*(gx+1, gz+1) − *H*(gx, gz+1)) × *fx*) / 16; *h_k* = *A* + ((*B* − *A*)
  × *fz*) / 16;
- when *d*.`flags1` has 0x1000 and *u* is alive and not dying, *h_k* becomes max(*h_k*, `seaLevel`) + Bob(u, k).

Then, with *a* = (*h_0* + *h_1*) / 2 and *c* = (*h_2* + *h_3*) / 2: `pos.y`'s high 16 bits (`yWhole`) become (*a* +
*c*) / 2 (its low 16 bits stay), `pitch` becomes A(*c* − *a*, |*z_0* − *z_3*| >> 16) and `bank` A(*h_0* − *h_1*,
|*x_0* − *x_1*| >> 16), where *x_k*, *z_k* are corner *k*'s *v.x*, *v.z* before the rotation and the shifted value
is kept to signed 16 bits.

**Bob(u, k)**, the hover's bob: *p* = ((`Game.ticks` & 31) + 8*k*) × 2048 + `hoverBobPhase`, kept to 16 bits; *s* =
*d*.`maxvelocity` / 2; *q* = min(`u.motion`'s `speed`, *s*); *w* = (*q* × 65536) / *s*; *m* = 2 − ((2*w*) >> 16); *n*
= min(`Game.ticks` − `u.motion`'s `lastMoveTick`, 60), unsigned; *m* becomes *m* − (*m* × *n*) / 60, the product and
quotient unsigned; Bob is SC(*p*, *m*).

**Place(u, pos, mode)**: *c* = Cell(pos, `footprint`). When *c* = `cell` and *mode* = `flags & 3`: `pos` becomes
*pos*. Otherwise, in order: MAP_REMOVE(u); then `pos` becomes *pos*, `cell` *c*, and `flags`' bits 0 and 1 *mode*
& 3; then MAP_ADD(u), LOS_UPDATE(u). Either way `flags` then gains 0x10000, after the imports.

**Attach(u, t, piece, mode)**, *t* a unit or 0, *piece* and *mode* signed 8-bit: nothing unless *u* is alive,
`flags` lacks 0x20000000, `cargo` is 0, and *t* is 0 or (*t* alive, *t* ≠ *u*, `t.carrier` 0). Then, in order:

1. `network.SendAttach(u, t, piece, mode)`, both sign-extended;
2. when `carrier` is not 0, *u* leaves its carrier's list: the link that held *u* (the carrier's `cargo`, or the
   `cargoNext` of the unit before it) takes *u*'s `cargoNext`; when `carrier` is 0, UNLINK_BUCKET(u);
3. `transportPiece` becomes *piece*'s byte; with *t*: `carrier` becomes *t*, `cargoNext` *t*'s `cargo`, `t.cargo`
   *u*, and `flags` bit 17 is set when *piece* is −1 and cleared otherwise; without *t*: `carrier` and
   `cargoNext` become 0, bit 17 is cleared, and PREPEND_BUCKET(u);
4. MOVE_MODE(u, *mode*'s byte, 0 to 255);
5. when Simulated(u) and *t* is not 0 and `t.def.flags1` lacks 0x200: BE_CARRIED(u);
6. Event(`SIM_EVENT_DESELECT_IF_INELIGIBLE`, [`u.id`]).

**Finish(b, t)**, builder *b* (or 0) finishes *t*: nothing unless *b* is not 0 and alive, `b.def.ids` is not 0,
and *t* is alive. Then `t.state`'s `bitmap` becomes 0, `t.buildLeft` 0.0, `t.flags` gains 0x2000; and in order:

1. when Simulated(t): Event(`SIM_EVENT_GUI_CHANGED_IF_SCREEN`, [0]) when `t.flags` has 0x20000000; otherwise,
   when `t.carrier` is not 0, Attach(t, 0, −1, 1);
2. when `t.def.flags1` has 0x40000: Bits(t, 1, 1);
3. Event(`SIM_EVENT_BUILD_MENU_UPDATE`, [`b.id`]);
4. when `t.def.flags1` has 0x1000000: `t.lastDamageType` becomes 7 and `t.flags` gains 0x4000;
5. when Simulated(t): `network.SendBuilderLink(b, t)`.

Then, when `b.flags` or `t.flags` has 0x10, `Game.flags_37ebe` gains 0x10.

**Build(b, u, x)**, *x* a float, the result 0 or 1: with *L* = `buildLeft` (a float), *T* = *d*.`buildtime`, *D* =
*d*.`maxHealth`:

- *L* = 0.0: the result is 0, nothing written, no import.
- otherwise, when *x* ≥ 0, `netDirtyFlags` gains 0x8000; then *x* = 0.0 gives 0 and nothing else. Otherwise (the
  arithmetic is in section 5): *N* = the new build progress left, *S* = *L* − *N*, *E* = *d*.`energyCost` × *S*,
  *Mc* = *d*.`metalCost` × *S*, *hp* = Tr(*L* × *D*) − Tr(*N* × *D*).

| Case | Imports in order | Writes | Result |
| --- | --- | --- | --- |
| *x* < 0 | ECON_INCOME(u, −*Mc*); DAMAGE(u, u, 30000, 9) when *N* ≥ 1.0; then Finish(b, u) when *N* = 0.0 | `health` max(*hp* + `health`, 0), `buildLeft` *N*, `flags` gains 0x2000 | 0 |
| otherwise (*x* > 0, or a NaN), `economy.RequestGranted(b, 3)` gives non-zero | that call; ECON_REQUEST(b, 3, *E*, *Mc*); Finish(b, u) when *N* = 0.0 | `health` min(*hp* + `health`, *D*) (Q2), `buildLeft` *N*, `flags` gains 0x2000 | 1 |
| otherwise, it gives 0 | that call; ECON_REQUEST(b, 3, *E*, *Mc*); Finish(b, u) when `buildLeft` is 0.0 | nothing | 0 |

`netDirtyFlags` is written before every import; a row's other writes come after its first effect (ECON_INCOME,
or ECON_REQUEST) and before the rest. `health` keeps the low 16 bits of the value. ECON_REQUEST's `player` is
*b*'s `playerIndex`.

**Repair(b, u, x)**, the result 0 or 1, with *D* = *d*.`maxHealth` read as a signed 32-bit number and *T* =
*d*.`buildtime`: when `health` ≥ *D* (a signed comparison) the result is 0, with no import. Otherwise *n1* =
min(Tr(((*D* × *x*) − 1) / *T* + 1), 1) and
*n2* = min(Tr(((*d*.`energyCost` × *x*) − 1) / *T* + 1), 1) (section 5), and in order:
`economy.RequestGranted(b, 1)`; ECON_REQUEST(b, 1, float(*n2*), 0); when the request gave non-zero,
DAMAGE(b, u, *n1*, 10) and the result is 1; otherwise the result is 0. The module writes nothing. The damage
is of kind 10 (Terms), and the recordings show its subscriber's write as an `o Unit#N.health` line after it.

**Create(p, ty, pos, fin, mode, id)**, *p* a player index (0 to 255), *ty* a type index (0 to 65535), *id* a
unit id or 0, the result a unit or 0. *Pl* is `Game.players_1b63[p]` and *d* = `Game.defs[ty]`. The result is 0,
with nothing written and no import, when:

- *ty* is 0, or *d*.`flags1` lacks 0x800000;
- *d*.`maxThisUnit` is not −1 and the units of *Pl*'s slice whose `unitDefIndex` is *ty* number at least
  `maxThisUnit`;
- *id* is 0 and every record of *Pl*'s slice holds a unit;
- *id* is not 0 and record *id* is outside *Pl*'s slice, or holds a unit.

Otherwise *u* is the record *id*, or for *id* 0 the first record of *Pl*'s slice that holds no unit. *f* is
*d*.`origin`, *S* the unit type's script *d*.`data` and *M* the model `Game.types[ty]`. The stages below come in
order; in each, the writes are made before its imports, so a synchronous subscriber of an effect sees the writes
of its stage and of the stages before it. A bit of `flags` not named keeps its value.

1. `unitDefIndex` *ty*, `def` `Game.defs[ty]`, `footprint` *f*, `pos` *pos*, `cell` Cell(*pos*, *f*);
   `buildLeft` 0.0 and `health` *d*.`hp` (the low 16 bits of `maxHealth`, which `layout.json` gives at the same
   offset) when *fin* ≠ 0, else `buildLeft` 1.0, `health` 0 and
   `unfinishedInitZero` 0; `healthPercent`, `prevHealthPercent`, `activateFlags`, `workTime` and `pitch` 0;
   `cobStateFlags` bits 0 to 3 cleared; `zBufferFlag` bit 0 *d*.`flags1` bit 7; `flags`: bits 0 and 1 become 1,
   bit 28 set, bit 29 set when *d*.`mobile` is 0 and cleared otherwise, bit 14 cleared, bit 31 *d*.`flags1` bit 16,
   bit 30 *d*.`flags1` bit 9, bits 2, 3, 4, 10, 11 and 17 cleared, bits 5 and 16 set. Then
   `RandomInt(d.buildangleUnsigned)` gives *r1*.
2. `heading` *r1* + 0x8000 − *d*.`buildangleUnsigned` / 2 (low 16 bits), `bank` 0. SIGHT_RESET(u).
3. `recentlyDamagedTimer` and `postTransferHoldoff` 0; `flags` bit 9 set when `u.player.index` is
   `Game.playerIndex` and cleared otherwise, bit 8 cleared. For *w* = 0, 1, 2: TGT_RESET(u, w), RELEASE(u, w).
4. `netDirtyFlags`, `killCount` and `attacker` 0, `lastAttackerSlot` 10; `flags` bits 18 and 19 *d*.`flags1` bits 0
   and 1, bits 20 and 21 *d*.`flags1` bits 2 and 3, bit 11 *d*.`flags1` bit 4, bits 26 and 27 0, bits 22 and 23 3
   when *d*.`buildMenuPageCount` > 1 and 0 otherwise, bits 24 and 25 0; `owner` `Game.players[u.playerIndex]`.
   ECON_RESET(u).
5. `transportPiece` 255. `RandomInt(65536)` gives *r2*; `hoverBobPhase` *r2*. SQUAD(u, 0).
6. When *S* is not 0: `scripts.NewUnitScript(u)` (which writes `u.script`), `scripts.SetCob(u.script, S)`,
   `scripts.CreatePlayerObjectState(M, S, u)` (its result becomes `u.state`), `scripts.SetObjectState(u.script,
   u.state)`, START(u, `"Create"`, 1). When *S* is 0: `u.script` 0, `scripts.CreateObjectState(M)` (its result
   becomes `u.state`), and that state's `owner` becomes *u*. Either way the state's `bitmap` becomes 0 after these
   imports.
7. INIT_SLOTS(u), `economy.UpdateMetalExtraction(u)`; when *d*.`mobile` is 1, MOVE_NEW(u) (which gives the unit
   its `motion`), and then `heading` *d*.`buildangleUnsigned`.
8. `flags` bits 0 and 1 *mode* & 3. Height(u), with its import.
9. MAP_ADD(u), `network.SendNewUnit(u)`.
10. When *fin* ≠ 0, in order: `network.SendBuilderLink(u, u)` when *d*.`mobile` is 0; Bits(u, 1, 1) when
    *d*.`flags1` has 0x40000; `lastDamageType` 7 and `flags` gains 0x4000 when *d*.`flags1` has 0x1000000.
11. LOS_REVEAL(u); *Pl*'s `unitCount` and `unitsCreated` each add 1; CREATED(u).

The result is *u*.

**Death(u, k)**, unit *u* dies with kill type *k* (Terms), with the severity *v*, the wreck level *w* and the
network id *e* that Kill gives it. *A* is `u.attacker`, and *C* is Commander(u). In order:

1. EYEBALL(u) when `P.index` = `Game.playerIndex`;
2. `services.FindSlotByDpid(e)` gives *s*; `lastAttackerSlot` becomes *s* (its low byte);
3. DIED(u), STOP(u), Event(`SIM_EVENT_REMOVE_UNIT_SPEECH`, [`u.id`]), SQUAD(u, −1), REMOVE_SHOTS(u);
4. when `carrier` is not 0: Attach(u, 0, −1, 1);
5. while `cargo` is not 0: DAMAGE(*A*, `cargo`, 30000, 3 when *k* is 3, else 6), then Attach(`cargo`, 0, −1, 1);
6. MAP_CLEAR(u); LOS_REMOVE(u) when `Game.mapFlags` has 2;
7. the credit (below), with its writes and imports;
8. when *k* is 5 and *A* is not 0: ECON_INCOME(A, *Rm*) with *Rm* the metal of the finished part (section 5);
9. when *v* > 0 and `buildLeft` is 0.0: DETONATE(u, 1 when *k* is 3, else 0);
10. when *w* > 0: Wreck(u, w, *k* ≠ 7);
11. the references: while `head` is not 0, for the reference *r* at `head`: STATUS(u, r.`value`, 8) when
    `r.value` is not 0; then, when `r.owner` is still *u*, *r* is taken out of *u*'s list and its `owner` and
    `next` become 0;
12. when `script` is not 0: `scripts.DeleteScript(script)`, then `script` becomes 0; when `state` is not 0:
    `scripts.FreeObjectState(state)`, then `state` becomes 0;
13. when `motion` is not 0: MOVE_DESTROY(u);
14. `unitDefIndex` becomes 0, `def` `Game.defs[0]` (its block offset), `flags` loses 0x10000000 and 0x30, and
    `P.unitCount` drops by 1; when it is now 0: `mission.GetGameType()`, then Event(`SIM_EVENT_PLAYER_LEFT`,
    [`P.id`]) when it gave 3; then `mission.GetGameType()` again, then Event(`SIM_EVENT_FORCES_DESTROYED`,
    [`P.index`]) when it gave 2.

`attacker` keeps *A*.

**The credit**, with *s* the slot above and *L* = `Game.players[s]` when *s* ≠ 10:

| *k* | Credited when | Writes |
| --- | --- | --- |
| 1, 6, and 5 when *s* ≠ 10 and *s* ≠ `u.playerIndex` | always | `P.losses` + 1; when *s* ≠ 10, `buildLeft` is 0.0 and `u.playerIndex` ≠ *s*: `L.kills` + 1; when *C*: `L.commanderKills` + 1 (when *s* ≠ 10) and `P.commanderLosses` + 1; when *A* is not 0, `buildLeft` is 0.0 and `u.playerIndex` ≠ *s*: `A.killCount` + 1 |
| 3 | `Game.players[Game.localPlayer].shareVision[P.index]` is 0 | `P.losses` + 1; when *C*: `P.commanderLosses` + 1 |
| any other (5 otherwise) | never | none |

The writes are made before the imports. Imports of a credited kill, in order: Event(`SIM_EVENT_CD_ACTIVITY`, [5, *s*, *s*]) when *k* is not 3 and *s* =
`Game.localPlayer`; then, when *s* ≠ 10: the rank pass, then Event(`SIM_EVENT_SCORE_FLASH`, [*s*, `P.index`]).

**The rank pass** runs when *L*.`active` ≠ 0, *L*.`type` is 1, 2 or 3, *L*.`index` ≠ 10, the game type is 3 or 2,
and *L*.`rank` > 0. The game type is `mission.GetGameType()`, called once when it gives 3 and a second time
otherwise; when *L* fails a test before it, there is no call (*L*'s tests are made in the order given and the
game type comes after `index`). With *K*(x) = x.`commanderKills` when `Game.commanderDeath` is 2 and x.`kills`
otherwise, *best* is the least of *L*.`rank` and the `rank` of each player 0 to 9 whose `type` ≠ 0, whose `info.bit6` is 0
and whose *K* is below *K*(L). When *best* < *L*.`rank`: each player 0 to 9 whose
`rank` is at least *best* and below *L*.`rank` (as it was) has `rank` + 1; *L*.`rank` becomes *best*; and when
*best* is 0, Event(`SIM_EVENT_PLAYER_TOOK_LEAD`, [*L*.`index`, *K*(L)]).

**Wreck(u, w, smoke)**: *f* = *d*.`corpseUnsigned`; *w* − 1 times, *f* becomes `Game.features[f].dead`, and when *f*
≥ 0xfffb before any step, nothing happens. When the final *f* ≥ 0xfffb, nothing happens. Otherwise *c* =
`los_radar.GetMapCell(cell.x, cell.y)` (sign-extended); when *c* is 0, nothing more. Otherwise
`map.GetGroundHeight(&u.pos)` gives *h*, and the placement *Q* is a `FeaturesPlace` with `x` and `z` the cell's
column and row (*i* mod `Game.width` and *i* / `Game.width`, *i* the index of *c* in `Game.cells`, 13 bytes a
cell), `feature` *f*, `flags` 3, `owner` `playerIndex`, `pos` `u.pos`, `rot` `bank`, `heading`, `pitch` (bank
and pitch sign-extended, heading zero-extended):

| *h* | Imports | Smoke |
| --- | --- | --- |
| ≤ `seaLevel` | *Q* with `flags` gaining 4 (`FEATURES_SINK`) unless *d*.`flags1` has 0x1000000, placed by `features.Place` (K2) | when *smoke* and `Place` gave 0 |
| > `seaLevel` | PLACE(*Q*) | when *smoke* |

The smoke is Event(`SIM_EVENT_SMOKE`, [`pos.x`, `pos.y`, `pos.z`, 15, 900, 9]), after the placement.

**Kill(u, k)**, the death of *u* with kill type *k*: nothing unless *u* is alive. *C* = Commander(u); when *C*,
`P.flags` loses bit 0. The severity *v* and the wreck level *w*:

| Case | *v*, *w* |
| --- | --- |
| *k* is 7 | 0 and 1 |
| *k* is 4, 5 or 9, or `health` > 0 | 0 and 0 |
| otherwise | *v* = clamp((Uq(−100 × `health`, *d*.`maxHealth`) + `prevHealthPercent`) / 2, 1, 100); then `scripts.QueryScript(script, "Killed", &v, &w, 0, 0)` with *w* 0, which may change *v* and *w*; its result is ignored; the call is made with `script` as it is, 0 included |

Uq(*a*, *b*) is the unsigned 32-bit quotient of *a* and *b*, read back as a signed number. Then *w* becomes 0 when
`buildLeft` is not 0.0. In order:

1. the `QueryScript` of the table, when it runs;
2. `services.GetSlotDpid(lastAttackerSlot)`, giving *e*;
3. when Simulated(u): `network.SendUnitKilled(u, v, w, k)`;
4. Death(u, k);
5. when *C*, `Game.commanderDeath` ≠ 0 and Simulated(u): Event(`SIM_EVENT_POP_TO_GAME_LAYOUT`, [1]), then
   KILL_UNITS(`playerIndex`).

### `Apply(event, size)`

The result is 0, with nothing written and no import, when *size* < 8; when `kind` is none of 352 to 367 and 384
to 398; when *size* is smaller than the kind's payload (section 2); for UNLINK, when `ref` is 0; for every kind
but UNLINK, CREATE, KILL_ALL and RELOAD_TYPE, when `unit` is 0; for SEND and CAPTURE, when `player` is 10 or more
(read unsigned). The limits of ozgb/btm#25 (the contract of `docs/module-abi.md`, 9.3: a malformed argument
changes nothing and does not trap): the reference
has no defined result for the cases below, and the module's answer to each is outcome 0 with nothing written and
no import:

- a `unit`, `target`, `builder` or `attacker` that is not a valid unit offset, or a `ref` that is not the block
  offset of a reference in its `owner`'s list (when its `owner` is not 0);
- CREATE with a `player` (its low byte) above 10, or a `type` (its low 16 bits) at or above `Game.unitDefCount`;
- RELOAD_TYPE with a `type` at or above `Game.unitDefCount`;
- REPAIR with `builder` 0 when the unit's `health` is below its `maxHealth` (signed, as Repair compares them);
- any kind whose unit is not alive where the table below reads its `state` or its script (FINISH_CONSTRUCTION
  with a `state` of 0).

*u* is the event's `unit`. Otherwise, by kind:

| Kind | What happens | Outcome |
| --- | --- | --- |
| SET_STATE_BITS | Bits(u, `mask`, `set`) | 1 |
| SET_FLAGS | `flags` becomes (`flags` & ~`clear`) \| `set` | 1 |
| SET_NET_DIRTY | `netDirtyFlags` becomes (`netDirtyFlags` & ~`clear`) \| `set`, low 16 bits | 1 |
| SET_WORK_TIME | `workTime` becomes `until` | 1 |
| SET_POSITION | Place(u, `pos`, `mode`) | 1 |
| ATTACH_TO_PIECE | Attach(u, `target` or 0, `piece`'s low byte, `mode`'s low byte), both read signed | 1 |
| FINISH_CONSTRUCTION | Finish(`builder` or 0, u) | 1 |
| BUILD_DECAY | Build(u, u, *x*) with *x* the decay of section 5 | 1 |
| REPAIR | Repair(`builder`, u, `amount`) | 1 |
| UNLINK | the reference *r* at `ref`: when `r.owner` is not 0, *r* is taken out of that unit's list (the link that held it takes `r.next`), and `r.owner` and `r.next` become 0 | 1 |
| SET_STANCE | for each bit of `fields`: 1, `flags` bits 18 and 19 become `moveOrder` & 3; 2, bits 20 and 21 `fireOrder` & 3; 4, `group` becomes `group` | 1 |
| SET_ANGLES | for each bit of `fields`: 1, `bank` becomes `bank`'s low 16 bits; 2, `heading` `heading`'s; 4, `pitch` `pitch`'s | 1 |
| SET_BUILD_STATE | `buildLeft` becomes `buildLeft`; `health` `health`'s low 16 bits | 1 |
| TURN | `heading` becomes `heading` + `delta`, low 16 bits; `flags` gains 0x10000 | 1 |
| MOVE | `pos` becomes `pos`; `flags` gains 0x10000 | 1 |
| CHANGE_CELL | `pos` becomes `pos`; `cell` (`cell[0]`, `cell[1]`), each the low 16 bits; `flags` bits 0 and 1 `mode` & 3 | 1 |
| SET_MOVED | `flags` bit 16 becomes `moved` & 1 | 1 |
| SFX_OCCUPY | Occupy(u) (below) | 1 |
| SCRIPT_VALUE | by `which`: 1, Bits(u, 1, `value`); 5, 6 and 19, `cobStateFlags` bit 0, 1 and 3 become `value` & 1; 20, Bits(u, 2, `value`); any other, nothing; then, for every `which`, `netDirtyFlags` gains 4 | 1 |
| NOTIFY_REFS | STATUS(u, r.`value`, `event`) for each reference *r* of *u*'s list whose `value` is not 0, in list order (each `next` read after the effect before it) | 1 |
| ATTACKED_SPEECH | `orders.GetOrderFlags(u)` gives *o*; when *o* lacks 0x80 (order flag 0x80, Terms) and (`lastAttackerSlot` ≠ `playerIndex` or `lastDamageType` is 1): Speak(u, 2, 1) | 1 |
| MARK_DAMAGED | `recentlyDamagedTimer` becomes 240 | 1 |
| DAMAGE_RECORD | `lastDamageType` becomes `damageKind`'s low byte; when `attacker` is not 0, `lastAttackerSlot` becomes the attacker's `playerIndex` and `attacker` the attacker | 1 |
| SET_HEALTH | `health` becomes `health`'s low 16 bits; when `dying` is 1, `flags` gains 0x4000 | 1 |
| KILL | `selfDestruct` 0: DAMAGE(u, u, 30000, 3). Otherwise: DETONATE(u, 1), `flags` gains 0x4000, Kill(u, 3) | 1 |
| SEND | *T* = `Game.players[player]`; `flags` loses 0x10 and `postTransferHoldoff` becomes 150, then `network.SendOwnershipTransfer(u, T)` and DAMAGE(0, u, 30000, 4) | 1 |
| CAPTURE | Capture (below) | 1, or 0 |
| CREATE | Create(`player`'s low byte, `type`'s low 16 bits, `pos`, 1, 1, 0) | 1 when it gave a unit, else 0 |
| KILL_ALL | Kill(*x*, 8) for each record *x* from `Game.units` to `Game.unitsEnd`, in table order, whose `unitDefIndex` is not 0 when it is reached | 1 |
| RELOAD_TYPE | with *ty* = `type`'s low 16 bits read signed, when *ty* ≠ 0: Kill(*x*, 8) for each record *x* from `Game.units` to `Game.unitsEnd` whose `unitDefIndex` read signed is *ty*; then the type's reload, which is the engine's (K8) | 1 |
| FORGET_ATTACKER | `lastAttackerSlot` becomes 10, `attacker` 0, and `flags` gains 0x4000 (Q6) | 1 |

**Occupy(u)**, the water state: *y* = `pos.y`'s high 16 bits (signed), *s* = `seaLevel`, *o* = the `sfxOccupy`
word. When `flags & 3` is 1 or 2: when *y* > *s* the new state *n* is 4; otherwise *n* starts as *o*, becomes 1
when *y* − *s* > −5, then 2 when *d*.`draft` + *y* = *s*, then 3 when *d*.`modelMaxYHigh` + *y* < *s* (each later
test wins). When `flags & 3` is neither, *n* is 0. When *n* ≠ *o*: START_ARG(u, `"setSFXoccupy"`, 1, *n*) and the
word becomes *n*.

**Capture**: *T* = `Game.players[player]`. Create(`T.index`, `unitDefIndex`, `pos`, 1, `flags & 3`, 0) gives *n*,
with all its imports; when it gives 0 the outcome is 0 and nothing more happens. Otherwise *n*'s `flags` lose
bits 18 to 21, and:

| `present` | Writes to *n* | Imports after Create |
| --- | --- | --- |
| not 0 | `health` `hitPoints`' low 16 bits; `buildLeft` float(`buildPercent`); `bank`, `heading`, `pitch` the low 16 bits of `angles[0]`, `[1]`, `[2]` | STOCKPILES(n, the low bytes of `stockpiles[0]`, `[1]`, `[2]`) |
| 0 | `health`, the 32-bit word of `buildLeft`, and the three angles, copied from *u* | STOCKPILES(n, *u*'s `weapons[0]`, `[1]`, `[2]` `stockpile`), DAMAGE(0, u, 30000, 4) |

The writes come before those imports. Then Bits(n, *a*, 1) and Bits(n, ~*a*, 0), *a* being *u*'s
`activateFlags`. The outcome is 1.

### `UpdateAllUnits()`

The units pass of `docs/schedule.md` (the passes `units.players` and `units`): for each player 0 to 9 that is
active, of type 1, 2 or 3 and whose index is not 10, for each record of its slice that holds a unit, the head,
the other subsystems' steps and the body, then (when the body gave 1) orders and movement, then the tail with
*moved* the answer of movement. The steps of this module are `UpdateUnitHead`, `UpdateUnitBody` and
`UpdateUnitTail`; the others need imports the entry doesn't list (K1). No recording holds the slot.

### `UpdateUnitHead(u)`

`Game.f14353` adds 1 (32-bit, wrapping). When *d*.`windgenerator` > 0.0 and `Game.windEnabled` ≠ 0, in order:
START_ARG(u, `"SetDirection"`, 0, `Game.windDirection`), START_ARG(u, `"SetSpeed"`, 0, `Game.windSpeed` << 4).

### `UpdateUnitBody(u)`

Writes: `recentlyDamagedTimer` and `postTransferHoldoff` each drop by 1 when not 0. When `flags` has 0x10 and
(`flags` lacks 0x20, or `buildLeft` ≠ 0.0, or `postTransferHoldoff` (as just written) ≠ 0, or `carrier` is not 0
and `carrier.flags` lacks bit 30), `flags` loses 0x10. When `Game.ticks` mod 30 is 0 (unsigned):
`prevHealthPercent` becomes `healthPercent`, and `healthPercent` becomes clamp(Uq(100 × `health`,
*d*.`maxHealth`), 0, 100) (Q1).

Then, when Simulated(u), in order:

1. `mission.GetMissionParams(&mp)`;
2. when *mp*.`waterDoesDamage` ≠ 0, *mp*.`waterDamage` ≠ 0, `Game.ticks` mod 30 is 0, `pos.y`'s high 16 bits
   (signed) ≤ `seaLevel` and *d*.`flags1` lacks 0x1000: DAMAGE(0, u, *mp*.`waterDamage`, 11);
3. when *d*.`healtimeUnsigned` ≠ 0, `health` < *d*.`maxHealth` (compared unsigned, Q3) and `Game.ticks` & 7 is 0:
   Repair(u, u, float((*d*.`healtimeUnsigned` × 8) / 30)), the division of integers truncating;

and the result is 1. Otherwise there is no import and the result is 0.

### `UpdateUnitTail(u, moved)`

When *moved* ≠ 0: Height(u). Then, when `flags` has 0x4000: Kill(u, `lastDamageType`).

### `KillUnit(unit, cause)`

Kill(`unit`, *cause*).

### `CreateUnit(player, typeId, pos, finished, mode, id)`

Create(*player*, *typeId*, `pos`, *finished*, *mode*, *id*), with the arguments' widths of section 2. The result
is the unit's block address, or 0. A *player* above 10 or a *typeId* at or above `Game.unitDefCount` has no
defined result in the reference; the module returns 0 with nothing written (ozgb/btm#25).

### `FindUnitType(name)`, `FindUnitTypeId(name)`

The search runs over the types 1 to `Game.unitDefCount` − 1 of `Game.defs`, by `unitname`, with `_strcmpi`
(section 5, "The search"). `FindUnitType` returns the block address of the type found, or 0; `FindUnitTypeId`
returns its `id` (unsigned 16-bit), or 0. Neither writes nor imports.

### `AddBuildProgress(builder, unit, amount)`, `AddRepairProgress(builder, unit, amount)`

Build(`builder`, `unit`, *amount*) and Repair(`builder`, `unit`, *amount*), with their results.

### `CanLoad(unit, target)`, `CanReclaim(unit, target)`, `CanRepair(unit, target)`

Queries, with no writes and no import; *u* is `unit` (its type *d*), *t* is `target` (its type *e*). The answer is 1
when every condition of its row holds, else 0:

| Slot | Conditions |
| --- | --- |
| `CanLoad` | *e*.`flags2` lacks 0x80000; *d*.`flags2` has 0x100; the units of *u*'s cargo list (from `cargo` along `cargoNext`) whose `carrier` is *u* number fewer than *d*.`transportcapacity` (a `char`, read as an unsigned byte); `t.motion` is not 0; *e*.`footprintx` ≤ *d*.`capacity`; `t.flags & 3` ≠ 2; *d*.`flags1` has 0x800 or *e*.`minwaterdepth` < 0; `t.pos.y` + *e*.`modelMaxY` > `seaLevel` × 65536 (32-bit, wrapping); `t.buildLeft` is 0.0 |
| `CanReclaim` | *d*.`flags2` has 0x400; `t.flags & 3` ≠ 2; *e*.`flags2` lacks 0x1000 |
| `CanRepair` | *t* is not 0; *d*.`flags2` has 0x200; `t.health` ≠ *e*.`maxHealth` (compared unsigned); `t.flags & 3` ≠ 2; when *d*.`flags1` has 0x800: *d*.`flags1` has 0x200000 or *Y* + *e*.`modelMaxYHigh` ≥ `seaLevel`; when it lacks 0x800: *Y* + *e*.`modelMaxYHigh` ≥ `seaLevel` − *d*.`maxwaterdepth` |

*Y* is `t.pos.y`'s high 16 bits, signed. `CanLoad` and `CanReclaim` read *t* without testing it for null; a
null `target` has no defined result in the reference, and the module answers 0 (ozgb/btm#25).

### The references

*r* is the reference `ref`. Linking *r* to a unit *x* means: when *x* is not 0 and its `unitDefIndex` is not 0,
`r.owner` becomes *x*, `r.next` *x*'s `head`, and *x*'s `head` *r*; otherwise `r.owner` and `r.next` become 0.
Unlinking *r* means: when `r.owner` is not 0, *r* is taken out of its owner's list (the link that held it takes
`r.next`) and `r.owner` and `r.next` become 0. None of these slots imports.

| Slot | Effect |
| --- | --- |
| `UnitRefInit(ref, unit, value)` | `r.value` becomes `value`; then *r* is linked to `unit` (whatever `r.owner` and `r.next` held before) |
| `UnitRefSetUnit(ref, unit)` | *r* is unlinked, then linked to `unit` |
| `UnitRefGet(ref)` | the result is `r.owner` as an address (0 for none) |
| `UnitRefSetValue(ref, value)` | `r.value` becomes `value` |
| `UnitRefUnlink(ref)` | *r* is unlinked |

### The factory index

*I* is the `FactoryIndex` at `SimGlobals.g_factoryIndex`, and *E* its `players[player]`.

- **`CreateFactoryIndex(player)`**: when `g_factoryIndex` is 0, `services.SimAlloc(240)` gives *I* (zeroed) and
  `g_factoryIndex` becomes its block offset. Then *E*'s vector is emptied (*last* becomes *first*; its storage
  and room stay), `lastTick` becomes 0 and `live` 1.
- **`FreeFactoryIndex(player)`**: when `g_factoryIndex` is 0, nothing. Otherwise *E*'s vector's storage is freed
  (K3; the free is made even when the storage is 0) and its 16 bytes become 0, and `live` becomes 0; `lastTick` stays.
- **`RefreshFactoryIndex(player)`**: nothing when `g_factoryIndex` is 0, when `live` is 0, or when `Game.ticks` <
  `lastTick` + 30 (unsigned 32-bit). Otherwise the vector is emptied and refilled, in table order, with each record
  *x* from `Game.units` + 1 to `Game.unitsEnd` that is alive and not dying, whose `x.player.index` is the index of
  *O* = `Game.players[player]` and for which *O*.`allied[x.player.index]` ≠ 0, whose `buildLeft` is 0.0, whose
  type's `flags1` has 0x40 and 0x200, and whose `activateFlags` has 1; and `lastTick` becomes `Game.ticks`. Each
  element added when the vector is full grows it (Grow, section 5).
- **`GetFactoriesInRadius(player, pos, radius, out, capacity)`**: over *E*'s vector in order, each unit *x* whose
  type's `flags1` has 0x40 and 0x200 and whose `activateFlags` has 1, and whose distance *D* (section 5) to `pos`
  is at most `radius`², counts; the *i*-th that counts is written to `out[i]` (its block offset) when *i* <
  `capacity`. The result is the count. The entries of `out` from the count (or from `capacity`) on are not
  written. A `g_factoryIndex` of 0 has no defined result in the reference; the module answers 0 (ozgb/btm#25).

## 5. Exactness

**Notation.** f32(*x*) rounds a real *x* to the nearest 32-bit float and f64(*x*) to the nearest 64-bit float,
ties to even. Tr(*x*) is the kit runtime's `ftol` of a double: truncated toward zero to 64 bits, of which it is
the low 32 bits. Comparisons are of the exact values. Integer arithmetic is 32-bit and wraps, with `/` truncating
toward zero and `>>` arithmetic, unless a line says unsigned. A narrow field is sign-extended when signed and
zero-extended when unsigned (section 3 gives each type), and a write keeps the field's low bits.

- **Random draws:** `services.RandomInt` only (the generator of `rng.seed`, not a game logic stream), two per
  created unit: `RandomInt(d.buildangleUnsigned)` first, then `RandomInt(65536)`, as Create orders them. A range
  below 2 gives 0. No other slot draws.
- **The build step** (Build): *N* = f32(min(max(f64(*L* − f64(*x* / *T*)), 0), 1)), *T* converted exactly; *S* =
  f32(*L* − *N*); *E* = f32(*d*.`energyCost` × *S*) and *Mc* = f32(*d*.`metalCost` × *S*); *L* × *D* and *N* × *D*
  are f64 products (*D* converted exactly) given to Tr. −*Mc* is *Mc* with its sign flipped. max(*hp* + `health`, 0)
  is signed; min(*hp* + `health`, *D*) compares the sum, read unsigned, with *D* (Q2).
- **The decay** of BUILD_DECAY: *x* = −f32(f64(*T* × `ticks`) / *d*.`energyCost`), the product of integers being
  32-bit and wrapping.
- **The repair** (Repair): *D* × *x* and *d*.`energyCost` × *x* are f64 products; − 1, / *T* and + 1 are f64
  operations in that order; Tr gives *n1* and *n2*, each then made at most 1 (signed). float(*n2*) is exact.
- **The self-repair amount**: (`healtimeUnsigned` × 8) / 30 is an integer, converted exactly to a float.
- **The health percentage** (Q1) and Kill's severity: Uq(*a*, *b*) converts *a* to unsigned 32 bits and divides
  by *b* (unsigned 32-bit), truncating; the quotient is read back as a signed number before the clamp. `health`
  is sign-extended before × 100 and × −100.
- **The reclaimed metal** *Rm* of Death: f32(f64((1 − `buildLeft`) × *d*.`metalCost`)), the difference being
  exact.
- **Rot(a, x, z)**: the kit runtime's `rotate_point` (`runtime/vectors/README.md`, "Rotating a point") of the
  angle *a* (16 bits) and the point.
- **A(x, z)**: the kit runtime's `angle_of_vector` ("Angle of a vector"), kept to its low 16 bits.
- **SC(a, s)**: the kit runtime's `scaled_cosine` ("Scaled sine and cosine") of the 16-bit angle *a* and the scale
  *s*.
- **Align's numbers**: the four averages and every `(… × fx) / 16` divide truncating toward zero; |·| of a 32-bit
  difference wraps (|−2³¹| is −2³¹); `yWhole`, `pitch` and `bank` keep the low 16 bits. In Bob, *w* = (*q* × 65536) /
  *s* is a 64-bit signed quotient kept to 32 bits, (2*w*) >> 16 is a 64-bit shift kept to 32 bits, and *m* × *n* /
  60 is unsigned; *s* of 0 (a `maxvelocity` below 2) has no defined result in the reference, and the module takes
  Bob as 0 (ozgb/btm#25).
- **Height's products**: × 65536 is a 32-bit shift left by 16, wrapping; (`seaLevel` − `draft`) are integers.
- **The search** (`FindUnitType`, `FindUnitTypeId`): with *lo* = 1 and *n* = `Game.unitDefCount` − 1 (signed),
  while *n* > 0: *h* = *n* / 2; when `_strcmpi(defs[lo + h].unitname, name)` < 0, *lo* becomes *lo* + *h* + 1 and *n*
  becomes *n* − *h* − 1; otherwise *n* becomes *h*. The type found is `defs[lo]` when *lo* < `unitDefCount` and
  `_strcmpi(name, defs[lo].unitname)` is 0, else none. On a table sorted by `_strcmpi` this is the first type
  whose name equals *name*. The search reads names only, so the order of its reads is not compared; its answer
  on a table that is not sorted is the one this bisection gives.
- **D**, the distance of `GetFactoriesInRadius`: hi(*dx* × *dx*) + hi(*dz* × *dz*), *dx* = `pos.x` − `x.pos.x`, *dz* =
  `pos.z` − `x.pos.z`, hi being the high 32 bits of the signed 64-bit product (the runtime's `mul_hi`); `radius`² is
  32-bit and wraps; the comparison is signed.
- **Grow**: a full vector of length *n* gets room for 1 when *n* is 0, 2 when *n* is 1 and 2*n* otherwise: in order,
  `services.SimAlloc(4 × room)` for the new storage, the elements copied, the old storage freed (K3: the free
  is made even when the old storage is 0, the first growth); *first*,
  *last* and *end* then point into the new storage.
- **Cell(p, f)**, Place and Wreck's indices: as section 4 states them.

## 6. Quirks

- **Q1**: the health percentage divides 100 × `health` by `maxHealth` unsigned; a negative `health` gives 100.
- **Q2**: Build's cap compares `health` plus the gain with `maxHealth` unsigned; a negative sum gives
  `maxHealth`.
- **Q3**: the self-repair's test `health` < `maxHealth` is unsigned; a unit with negative health does not repair
  itself.
- **Q4**: Repair adds at most 1 point of health and asks at most 1 unit of energy per call, whatever the amount.
- **Q5**: Kill clears bit 0 of the player's `flags` when the unit is the commander, whether or not the player is
  simulated and whatever the commander-death rule.
- **Q6**: FORGET_ATTACKER sets bit 14 of `Unit.flags`, the dying flag; the next unit pass kills the unit with its
  `lastDamageType` (K7).
- **Q7**: a SEND sends the unit and then damages it with 30000 of damage kind 4; a CAPTURE without `present`
  damages the captured unit the same way after its copy is made.
- **Q8**: KILL with `selfDestruct` 0 is a damage of 30000 of kind 3 from the unit to itself; with `selfDestruct` 1
  the unit detonates and dies at once.

## 7. Examples

Each is a call of the kit's recordings; the inputs are the values the recording gives at the call (its `r`
lines, or the last value an earlier line gave).

1. **Build progress.** `tests/replay/units/orders-build-patrol.txt`, tick 148, `AddBuildProgress(Unit#1,
   Unit#2, 10.0)`. `Unit#2` is a `UnitDef#112` (`buildtime` *T* 2495, `maxHealth` *D* 326, `energyCost` 760.0,
   `metalCost` 145.0) with `buildLeft` *L* 0x3f2ff7e6 (0.68737638) and `health` 102. *x* / *T* = 0.0040080160...;
   *N* = f32(0.68336836...) = 0x3f2ef13b; *S* = f32(*L* − *N*) = 0x3b835580 (0.0040079951); *E* = 0x4042f2ea
   (3.0460763), *Mc* = 0x3f14c6db (0.58115929); *hp* = Tr(224.08470) − Tr(222.77809) = 224 − 222 = 2. Imports:
   `ScheduleEconomyRequestGranted Unit#1 3 = 1`, then `SchedPublish 2 24` (ECON_REQUEST). Writes:
   `netDirtyFlags` −32768 (0x8000), `health` 104, `buildLeft` 0x3f2ef13b; result 1.
2. **Repair.** `tests/replay/units/orders-air-repair.txt`, tick 848, `AddRepairProgress(Unit#2, Unit#4, 6.0)`.
   `Unit#4` is a `UnitDef#29` (`maxHealth` 280, `buildtime` 8844, `energyCost` 4320.0) with `health` 276. *n1* =
   Tr((1680 − 1) / 8844 + 1) = Tr(1.18985...) = 1; *n2* = Tr((25920 − 1) / 8844 + 1) = Tr(3.93068...) = 3, made 1.
   Imports: `ScheduleEconomyRequestGranted Unit#2 1 = 1`, `SchedPublish 2 24` (ECON_REQUEST with energy 1.0),
   `SchedPublish 2 24` (DAMAGE of 1, kind 10), after which the recording gives `o Unit#4.health 277`; result 1.
3. **Creation.** `tests/replay/units/orders-factory.txt`, tick 19, `CreateUnit(0, 68, (41943040, 65536,
   16777216), 0, 1, 0)`. `UnitDef#68` has `origin` (6, 6), `buildangle` 4096, `mobile` 0, `flags1` 0x008000c9 and a
   script. The first free record of `Player#0`'s slice is `Unit#2`. Cell: (41943040 − 3145728 + 524288) >> 20 = 37
   and (16777216 − 3145728 + 524288) >> 20 = 13. `RandomInt 4096 = 3957` gives the heading 3957 + 32768 − 2048 = 34677;
   `RandomInt 65536 = 39223` the `hoverBobPhase`. The imports are those of Create, in its order: the draw,
   SIGHT_RESET (8), three pairs of TGT_RESET and RELEASE (12 each), ECON_RESET (12), the draw, SQUAD (12),
   `NewUnitScript`, `SetCob`, `CreatePlayerObjectState`, `SetObjectState`, START `"Create"` (76), INIT_SLOTS (8),
   `ScheduleMetalExtraction`, MAP_ADD (8), `SendNewUnit`, LOS_REVEAL (8), CREATED (8); no MOVE_NEW (`mobile` 0) and
   no Height import (`motion` 0). `flags` becomes 0x30e40221: mode 1, bits 5, 9, 18, 21, 22, 23, 28 and 29 (bit 16
   was set and Height cleared it); `buildLeft` 1.0; `Player#0.unitCount` and `unitsCreated` become 2.
4. **A death.** `tests/replay/units/console-units.txt`, tick 150, `UpdateUnitTail(Unit#251, 1)`. `Unit#251` is
   `Player#1`'s commander (`CORCOM`, `UnitDef#167`, `maxHealth` 3000) after a self-destruct: `flags` 0x90e44021
   (dying), `health` −27000, `lastDamageType` 3, `lastAttackerSlot` 1, `attacker` itself, `prevHealthPercent` 100.
   Height does nothing: `flags` lacks 0x10000 and the type's `flags1` lacks 0x1000. Kill(u, 3): *v* = clamp((Uq(2700000, 3000) + 100) / 2, 1, 100) =
   clamp(500, …) = 100; `QueryScript … "Killed" (100) (0)` leaves 100 and 0. `GetSlotDpid 1 = 1`; `Player#1` is
   active of type 2, so `SendUnitKilled Unit#251 100 0 3`; `FindSlotByDpid 1 = 1`. Death: no EYEBALL
   (`playerIndex` is 0); DIED (8), STOP (12), REMOVE_UNIT_SPEECH `[251]`, SQUAD (12), REMOVE_SHOTS (8), MAP_CLEAR
   (8), LOS_REMOVE (8, `mapFlags` 15). The credit of kill type 3: `Player#0.shareVision[1]` is 0, so `losses` and
   `commanderLosses` of `Player#1` become 1; the rank pass (`Player#1` active, type 2, rank 1): `GetGameType = 2`
   twice, no player below, no change; SCORE_FLASH `[1,1]`. DETONATE (12, severity 100, `buildLeft` 0). No wreck
   (*w* 0). `DeleteScript`, `FreeObjectState`, MOVE_DESTROY (8, then `o Unit#251.motion null`). `unitCount` 2 → 1.
   The commander rule (`commanderDeath` 1): POP_TO_GAME_LAYOUT `[1]`, KILL_UNITS (12). Writes: `script`, `state`
   null, `unitDefIndex` 0, `def` `UnitDef#0`, `flags` 0x80e44001, `Player#1.flags` 0.
5. **Repair query.** `tests/replay/units/orders-air-repair.txt`, tick 852, `CanRepair(Unit#4, Unit#1)` is 0:
   `Unit#1`'s `health` 3000 is its `maxHealth`. `CanRepair(Unit#4, null)` is 0.

6. **Reclaim and load queries.** `tests/replay/units/orders-guard-reclaim.txt`, tick 250, `CanReclaim(Unit#1,
   Unit#2)` is 1: `UnitDef#34.flags2` 0x0056f6fb has 0x400, `Unit#2.flags` 0x30282321 has mode 1, and
   `UnitDef#112.flags2` 0x00500004 lacks 0x1000. `tests/replay/units/units-transport.txt`, tick 4100,
   `CanLoad(Unit#3, Unit#1)` is 1: the target's `UnitDef#34.flags2` lacks 0x80000; the transport's
   `UnitDef#122.flags2` 0x005001e8 has 0x100; `Unit#3.cargo` is null, so 0 units against `transportcapacity` 5;
   `Unit#1.motion` is set; `footprintx` 2 ≤ `capacity` 3; `Unit#1`'s mode is 1; `UnitDef#122.flags1` 0x008090a9
   lacks 0x800 but the target's `UnitDef#34.minwaterdepth` is −10000; `posY` 65536 plus the model height is above 0 (`seaLevel` 0);
   `buildLeft` 0.0.
7. **A detach.** `tests/replay/units/orders-air-repair.txt`, tick 852, `Apply` of ATTACH_TO_PIECE (size 20) with
   `unit` `Unit#4`, `target` 0, `piece` −1, `mode` 2. `Unit#4` (`flags` 0x10e40020: alive, not a structure, no
   cargo) is carried by `Unit#2`, whose `cargo` is `Unit#4`. Imports: `network.SendAttach Unit#4 null -1 2`; the
   unit leaves its carrier's list (`Unit#2.cargo` takes `Unit#4.cargoNext`, null); `transportPiece` 255, `carrier`
   and `cargoNext` 0, then PREPEND_BUCKET (8), after which the recording gives `o Unit#4.cargoNext Unit#2` (map's
   write, which stands); MOVE_MODE (12) with mode 2; no BE_CARRIED (no target); DESELECT_IF_INELIGIBLE `[4]`.
   Writes: `Unit#4.transportPiece` 255, `Unit#4.carrier` null, `Unit#2.cargo` null; outcome 1.

Not covered by any recording: `UpdateAllUnits` (K1), `KillUnit`, `FreeFactoryIndex`, `GetFactoriesInRadius`,
`UnitRefUnlink`; `Apply` of FINISH_CONSTRUCTION, BUILD_DECAY, REPAIR,
SET_STANCE, SET_BUILD_STATE, SEND, CAPTURE, FORGET_ATTACKER, KILL_ALL and RELOAD_TYPE; the water
damage and the selection reset of `UpdateUnitBody`; the wind of `UpdateUnitHead`; `FindUnitType` and
`FindUnitTypeId` with a name that is no type; the self-repair of `UpdateUnitBody`; a wreck (both rows of Wreck);
the credit of kill types 5 and 6, and of kill type 1 with a killer slot (the one recorded has slot 10); a rank
change and PLAYER_TOOK_LEAD; the last unit of a player; a death with cargo or a carrier; Kill with a `script` of 0;
a vector that grows; Build with a negative amount and a refused request; a refused Repair; Create's refusals;
Attach with a target; the activation bits 2 and 3 and the loss of bit 1 (Bits); Occupy's states 1 and 2;
SCRIPT_VALUE with `which` 1, 19 or 20; KILL with `selfDestruct` 1; NOTIFY_REFS with references. Some writes are in
no `w` line, so a replay does not compare them: Create's `heading`, `UnitRefInit`'s `value` and a death's reset of
its references.

## 8. Kit version

The gate judges against `interfaces-v68` and `reference-traces-v55` (commit 906e69ae4, the kit's
`MANIFEST.json`). The units interface is version 7.2 (`UNITS_VERSION_MAJOR` 7, `UNITS_VERSION_MINOR` 2).
