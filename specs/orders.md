# orders

## 1. Role

Orders keeps each unit's orders and runs them. A unit has two lists of orders in the block (`Unit.list`, the
first of which is the order it carries out, and `Unit.list2`, the secondary orders that all run each tick). Every
order is an `Order` record of one *kind*, an index into the order type table (`SimGlobals.g_missionOrderTableVec`),
which gives the kind's name, its flags and the behaviour that runs it. Once a tick, for each unit of an active
player of type 1 or 2, the engine asks orders to run the unit's first order, then its secondary orders: moving,
attacking, guarding, patrolling, building, helping to build, repairing, reclaiming, capturing, resurrecting,
loading and unloading, and the air forms of these. Running an order asks movement for targets to steer to,
other modules for answers (ranges, footprints, build and repair progress), and publishes effects that units,
scripts, movement, weapons, targeting, features, economy and players apply. Orders also takes the events that
change a unit's orders (an order added, a command of the player, a build queued, orders stopped or cancelled,
being carried, being attacked or paralysed, a status reported), expands a player's command to a group of units
into one command per unit, and answers which order a click would give.

## 2. Interface

### Slots (`interfaces/orders.h`, version 6.2)

| Slot | When the engine calls it |
| --- | --- |
| `Apply` | for each event of orders' inbox, in the order it was published (`docs/schedule.md`, "Routes") |
| `RunOrders` | once a tick for each unit of an active player of type 1 or 2, in the unit's update, before movement's `UpdateMotion` |
| `RunSecondaryOrders` | right after `RunOrders`, for the same unit |
| `GetOrderFlags` | when another module asks for the flags of a unit's first order (a query) |
| `GetOrderType` | when the engine asks which order a unit gets against a target unit or a position for a relation mode (a query) |
| `MissionTypeFromName` | when an order kind is looked up by its name (a query) |
| `ExpandGroupCommand` | for each GROUP_COMMAND and GROUP_BUILD the engine publishes, instead of `Apply` |
| `RegisterAllOrderTypes`, `ClearOrderTypeTable`, `LoadOrders`, `SaveOrders`, `WriteState` | engine slots (below) |

The five engine slots are the entry's `engine` slots: the module does not export them, and the host calls the
engine's native slot, which reads and writes the module's records in the block as section 3 lays them out (ABI
section 6, "Engine slots", and section 9.3, "What stays in the engine"). `RegisterAllOrderTypes` fills the order
type table before the game starts and `ClearOrderTypeTable` empties it; `LoadOrders` and `SaveOrders` read and
write a unit's two lists for a saved game; `WriteState` writes the lists for the dump. So the module's `Order`
records must be exactly the `layout.json` record of section 3, linked as section 3 says, for those slots to read,
and the module reads the order type table but never writes it.

### The manifest entry

The kit's manifest (`wasm/modules.json`) has the entry `orders`, and the module must match it exactly (ABI 4.2
and 9.1):

- `name` and `interface`: `orders`.
- `majors`: `orders` 6, `movement` 5, `units` 7, `scripts` 3, `weapons` 4, `map` 9, `network` 2, `los_radar` 5,
  `features` 5, `targeting` 4, `mission` 2, `services` 4.
- `data_base` 458752 (0x70000) and `table_base` 304: the module's window of `window` (32768) bytes and its
  `table_slots` table entries (ABI 2.2 and 2.3). Everything the module keeps outside the block (none between
  calls: ABI section 12) and every buffer it passes to an import lies in that window.
- `blocked`: `true` in the kit's manifest. A blocked entry has no `fuel_budget` and no `size_budget` (ABI 4.2),
  and the gate judges a module of it on the recordings route, with `budgets: not_judged` (ABI section 15).
- `exports`, in this order: `Apply`, `RunOrders`, `RunSecondaryOrders`, `GetOrderFlags`, `GetOrderType`,
  `MissionTypeFromName`, `ExpandGroupCommand`.
- `engine`: `RegisterAllOrderTypes`, `ClearOrderTypeTable`, `LoadOrders`, `SaveOrders` and `WriteState`, each with
  the entry's reason (load class, save class, a dump query).
- `data`: `g_game`, `g_simGlobals`, `g_extTdf`, `g_blockBase` (ABI 8.2). The module reads `g_game` (the `Game`
  record), `g_simGlobals` (the `SimGlobals` record, which holds the order type table) and `g_blockBase` (to turn a
  block offset into an address and back).
- `publishes`: `{"2": 24, "4": 24, "33": 16, "35": 20, "96": 16, "97": 24, "98": 12, "192": 48, "256": 24,
  "259": 12, "260": 12, "269": 12, "295": 44, "304": 16, "305": 24, "306": 20, "307": 12, "308": 12, "309": 16,
  "310": 12, "311": 76, "312": 12, "313": 16, "314": 12, "315": 24, "316": 20, "317": 16, "326": 12, "327": 12,
  "328": 12, "329": 16, "330": 16, "331": 16}`: economy's REQUEST and INCOME, features' REMOVE and RECLAIM,
  targeting's SET_UNIT, SET_POS and CLEAR, players' GIVE_UNIT, weapons' DAMAGE, RELEASE, CLAIM and ADD_STOCKPILE,
  orders' own COMMAND (from `ExpandGroupCommand`), orders' fourteen effects (304 to 317), and movement's
  SET_TARGET, DESTROY_TARGET, SET_FLIGHT_MODE, SET_ALTITUDE, SET_APPROACH_RADIUS and SET_HEADING. The route table
  (`data/routes.csv`) carries each from orders' outbox, and says which are delivered at once (`synchronous`: the
  subscriber's writes follow the publish in the recording) and which are queued.
- Fuel: the module meters itself by rule version 1 (ABI section 11) and exports `__fuel_set` and `__fuel_get`.
- Slot kinds:

| Slot | Kinds |
| --- | --- |
| `Apply` | `[linear, int] -> int` |
| `RunOrders`, `RunSecondaryOrders` | `[block] -> void` |
| `GetOrderFlags` | `[block] -> int` |
| `GetOrderType` | `[int, block, block, linear] -> int` |
| `MissionTypeFromName` | `[linear] -> int` |
| `ExpandGroupCommand` | `[linear, int] -> int` |

A unit, an order, a target and a reference are block offsets (0 for none) where an event or an effect payload
carries them, and where a block field holds them (`Unit.list`, `Order.next`, `Order.target`, `Order.attached`)
(`interfaces/orders.h`, `OrdersEventHeader`). Every pointer argument of a slot or an import, and every pointer an import
returns (the `block` and `linear` kinds), is a linear address (`g_blockBase` + the offset; 0 is null); the `int`-kind
order arguments and answers of the `movement.New*` imports and the `int` answer of `movement.UnitTarget` are offsets. The `mode` of `GetOrderType` is the `int` kind
and only its low byte is used.

- Imports, every one listed and nothing else (ABI 9.1: every listed import is imported, even one no exported slot
  calls):

| Import | Kinds | Used for |
| --- | --- | --- |
| `movement.NewPathOrderAt` | `[int, linear] -> int` | "a path target to P" (the air kinds) |
| `movement.NewPathOrderToUnit` | `[int, block] -> int` | `AirStrike` state 4 and `VTOL_Pickup` state 1: a target that follows a unit |
| `movement.NewPathOrderToUnitAt` | `[int, block, linear] -> int` | `AirToGroundHover`'s strafe |
| `movement.NewPathOrderToUnitAtAngle` | `[int, block, int] -> int` | `VTOL_Landing` and `VTOL_Pickup`: a target at a pad slot or a transport piece |
| `movement.NewAirManeuverOrder` | `[int, linear, linear] -> int` | `AirToAir` state 1 |
| `movement.NewApproachRadius` | `[int, int, int, int] -> int` | "the approach goal (P, r)" |
| `movement.NewRingApproach` | `[int, int, int, int, int] -> int` | "the ring goal (P, r1, r2)" |
| `movement.NewPointMarker` | `[int, linear, linear] -> int` | "the footprint marker (cell, size)" |
| `movement.UnitTarget` | `[block] -> int` | "the end of an order": whether the unit's goal steers to the order's target |
| `units.CreateUnit` | `[int, int, linear, int, int, int] -> block` | `BuildingBuild`, `MobileBuild`, `VTOL_MobileBuild` and `Resurrect`: the unit started |
| `units.FindUnitTypeId` | `[linear] -> int` | QUEUE_BUILD's type number; `Resurrect` state 3 |
| `units.AddBuildProgress` | `[block, block, f32] -> int` | "the build step" (`HelpBuild` and `MobileBuild` state 3); `BuildingBuild` state 3, its own row (the target box, no BUSY); "the air build pass" (`VTOL_HelpBuild` and `VTOL_MobileBuild`) |
| `units.AddRepairProgress` | `[block, block, f32] -> int` | `SelfRepair`, `RepairUnit`, `RepairUnitNoMove` |
| `units.CanLoad` | `[block, block] -> int` | `GetOrderType` modes 1, 2 and 6 |
| `units.CanReclaim` | `[block, block] -> int` | `ReclaimUnit` and `VTOL_ReclaimUnit` |
| `units.CanRepair` | `[block, block] -> int` | `GetOrderType` modes 1, 2 and 8; `VTOL_Follow`, `VTOL_RepairPatrol` |
| `units.GetFactoriesInRadius` | `[int, linear, int, linear, int] -> int` | "the pad query" |
| `units.UnitRefInit` | `[block, block, block] -> void` | "a new order": the order's `link` |
| `units.UnitRefSetUnit` | `[block, block] -> void` | "a new order" without a target; the kinds that change an order's target |
| `units.UnitRefGet` | `[block] -> block` | `Capture`, `ReclaimUnit`, `RepairUnitNoMove`: the unit the order's `link` names |
| `units.UnitRefSetValue` | `[block, block] -> void` | "a new order": the `link`'s value, the order itself |
| `scripts.QueryScript` | `[block, linear, linear, linear, linear, linear] -> int` | `BuildingBuild` (QueryBuildInfo), "the landing pad search" (QueryLandingPad), `VTOL_Pickup` (QueryTransport) |
| `scripts.FindScript` | `[block, linear] -> int` | "start the building script", "stop the building script" |
| `scripts.GetPieceOffset` | `[block, int, linear] -> void` | `VTOL_Pickup` state 3 |
| `scripts.GetPiecePosition` | `[block, int, linear] -> void` | `BuildingBuild` state 2: the build piece |
| `scripts.GetNanoPiecePosition` | `[linear, linear] -> void` | "the nano point": where nano particles start |
| `weapons.GetWeaponRange` | `[block, int] -> int` | `Attack_Chase` and `Suppress` |
| `weapons.WeaponCanReachUnit` | `[block, block, int] -> int` | ATTACKED; `Attack_Chase`, `Guard_NoMove`, `Follow_Ground`, `VTOL_Follow`, `AirToGroundHover` |
| `map.UnitsInRange` | `[linear, int, int, linear, int] -> int` | "the units in range (P, r)" |
| `map.CanPlaceUnitFootprint` | `[block, int, linear, int] -> int` | `BuildingBuild`, `MobileBuild`, `VTOL_MobileBuild`, `VTOL_Unload`: whether a footprint fits |
| `map.SnapWorldPosToFootprint` | `[block, linear] -> void` | `MobileBuild`, `VTOL_MobileBuild`: the build site |
| `map.CanPlaceFootprintAt` | `[block, linear] -> int` | `VTOL_LandIfCan` |
| `map.IsPadSlotFree` | `[block, int] -> int` | "the landing pad search", `VTOL_Landing` state 6 |
| `map.GetGroundHeight` | `[linear] -> int` | "the site point", "the feature box", `VTOL_LandIfCan`, `VTOL_Reclaim` |
| `network.SendFeatureReclaimed` | `[block, int, int] -> void` | `Resurrect` state 5, in game type 3 |
| `network.SendScriptCall` | `[block, int, int, int, int, int, int] -> int` | "start the building script", "stop the building script" |
| `network.SendScriptCallByName` | `[block, linear, int, int, int, int, int] -> int` | `VTOL_Pickup` state 3 |
| `los_radar.GetMapCellAtPosition` | `[linear] -> block` | "the feature at P" of `GetOrderType`; `Resurrect` state 5 |
| `los_radar.GetOriginCellAtPosition` | `[linear] -> block` | `Resurrect` state 5 |
| `los_radar.GetVisibleEnemiesInRadius` | `[int, linear, int, int, linear, int] -> int` | "the enemy count (P, r)" (`Wait`, `Guard_NoMove`) |
| `features.GetCellFeature` | `[block] -> int` | `Resurrect` state 5 |
| `features.FindFeatureAtPos` | `[linear, linear, linear] -> int` | "the site cell and size" (`Reclaim`, `Resurrect`, `VTOL_Reclaim`) |
| `targeting.GetWeaponTargetUnit` | `[block, int] -> block` | ATTACKED; `Guard_NoMove`, `Follow_Ground`, `VTOL_Follow` |
| `targeting.FindBestTargetIfFireAtWill` | `[block] -> block` | `Patrol`, `Standby`, `Standby_Mine`, `VTOL_Standby`, `VTOL_Patrol`, `VTOL_SeekAttack` |
| `targeting.ChooseWeapon` | `[block] -> int` | `Attack_Chase` state 0 |
| `mission.GetGameType` | `[] -> int` | `Resurrect` state 5 |
| `mission.GetMissionParams` | `[linear] -> void` | `AirStrike` state 4: the gravity |
| `services.RandomInt` | `[int] -> int` | every draw (section 5) |
| `services.SimAlloc` | `[int] -> linear` | "a new order" (86 bytes) |
| `services.SimFree` | `[linear] -> void` | "the end of an order" |
| `services.RecordSimEvent` | `[int, int, linear, int] -> void` | the sim events of "Events" |
| `services.Emit` | `[linear, int] -> void` | every publish |

The recordings carry `services.Emit` as `services.SchedPublish 3 <size>` (3 is orders' outbox),
`targeting.FindBestTargetIfFireAtWill` as `services.ScheduleTargetingFireAtWill`, `targeting.GetWeaponTargetUnit`
as `services.ScheduleTargetingTargetUnit` and `map.CanPlaceUnitFootprint` as
`services.ScheduleMapCanPlaceUnitFootprint` (ABI 9.3, "Routes in the recordings").

**Two calls that are not in the entry.** Section 4 names two more calls: "the reclaim pick"
(`map.PickRandomReclaimableResourcesInRadius`, whose signature the manifest has under `signatures.map`, and which
`docs/schedule.md` lists among the imports of `orders.RunOrders`), in `RepairPatrol` and `VTOL_RepairPatrol`, and
`economy.RequestGranted`, in `BuildWeapon` state 1. Neither is an import of the entry above, which the module must
match exactly, so the entry stays as the kit has it. Until the entry lists them, a module of it can't import them
(ABI 9.1: the module imports the entry's imports and nothing else), so it can't make these calls and can't produce
the rows of section 4 that follow them. The host runs the engine's native orders for a `blocked` entry (ABI 4.2),
and the gate judges the module on the recordings alone, none of which reaches either call (section 7), so those rows
are judged by no check until the kit lists the imports ("Gaps in the kit").

- Hooks: `interfaces/orders.h` names AllowBuild, AircraftSeekRepair, GuardResponse and IdleBuilderAction on
  `RunOrders`, and AllowCommand on `GetOrderType` and on the event kind COMMAND; every other slot says "Hooks:
  none". The lines only fix where each hook runs (ABI section 6, "Hook points"); in this kit version the module
  calls none and behaves as section 4 states.

### Arguments outside the block

All in the caller's memory (`linear`), little-endian, read in place:

- **The event of `Apply`**: an `OrdersEventHeader` (`kind`, `unit`, 8 bytes) and its kind's payload (`interfaces/orders.h`),
  `size` bytes in all. The payloads and their sizes: ADD and COMMAND `OrdersAdd` (44), QUEUE_BUILD `OrdersQueueBuild`
  (44), STOP `OrdersStop` (12), BE_CARRIED the header alone (8), STATUS_FLAGS `OrdersStatusFlags` (16), ATTACKED
  `OrdersAttacked` (12), PARALYZE `OrdersParalyze` (12), CANCEL `OrdersCancel` (12). The recordings write the fields
  as `a event.kind`, `a event.unit`, `a event.type`, `a event.queue`, `a event.target`, `a event.hasPos`,
  `a event.pos0` to `pos2`, `a event.params0`, `a event.params1`, `a event.order`, `a event.flags`, and the size as
  `a size`.
- **The command of `ExpandGroupCommand`**: an `OrdersGroupCommand` (112 bytes: the header, `mode`, `type`, `queue`,
  `target`, `hasPos`, `pos[3]`, `params[2]` and the bit set `units[16]`), `size` bytes. The recordings write
  `a command.kind` to `a command.units15`.
- **`pos` of `GetOrderType`**: a `Vec3` (three signed 32-bit words), or 0 for none. **`name` of
  `MissionTypeFromName`**: a zero-terminated string.
- **What the module passes to imports**, in its own window: a `Vec3` for each position an import takes by address
  (`movement.NewPathOrderAt`, `NewPathOrderToUnitAt`, `NewAirManeuverOrder`'s two, `map.UnitsInRange`,
  `map.CanPlaceFootprintAt`, `map.GetGroundHeight`, `map.SnapWorldPosToFootprint`, `los_radar.GetMapCellAtPosition`, `los_radar.GetOriginCellAtPosition`,
  `los_radar.GetVisibleEnemiesInRadius`, `units.GetFactoriesInRadius`, `units.CreateUnit`, `features.FindFeatureAtPos`'s
  first), or the address of the order's own `Order.pos` or the unit's own `Unit.pos` in the block where section 4 says
  so; a `Point16` for each cell or size (`movement.NewPointMarker`'s two, `map.CanPlaceUnitFootprint`'s cell, the two
  outputs of `features.FindFeatureAtPos`); a `Vec3` that `scripts.GetPiecePosition`, `scripts.GetPieceOffset` and
  `scripts.GetNanoPiecePosition` write; four signed 32-bit words for `scripts.QueryScript` (each set to −1 before the
  call, or the address of `Order.ticks` where section 4 says so); zero-terminated names (`StartBuilding`,
  `StopBuilding`, `QueryBuildInfo`, `QueryLandingPad`, `QueryTransport`, `BeginTransport`, and the QUEUE_BUILD and
  `Resurrect` type names) for `scripts.FindScript`, `scripts.QueryScript`, `network.SendScriptCallByName` and
  `units.FindUnitTypeId`; a buffer of 256 unsigned 16-bit unit ids for `map.UnitsInRange`; a buffer of 64 unit block
  offsets (or the larger buffer of section 4) for `units.GetFactoriesInRadius` and
  `los_radar.GetVisibleEnemiesInRadius`; a `MissionParams` (`interfaces/mission.h`) for `mission.GetMissionParams`;
  the unit reference imports (`units.UnitRefInit`, `UnitRefSetValue`, `UnitRefSetUnit`, `UnitRefGet`) take the
  address of the order's own `Order.link` in the block (the order's address + 18); every effect payload of `services.Emit`; the argument array of `services.RecordSimEvent`.
- **Effect payloads**: each publish is the `interfaces/*.h` payload of its kind, every byte written (a field section 4
  does not name is 0); a `name` of START_SCRIPT is the text, at most 31 bytes, zero-padded to 32 bytes.

A recorded line shows the arguments the recording keeps: a cell or size passed by address is written by value
(`(x,y)`), a position by address as its three words (`(x,y,z)`), and a buffer the import writes as `?`
(`docs/replay-format.md`). Two names of the recordings stand for addresses of a new order:

- `alloc#n` is the address `services.SimAlloc` answered (`docs/replay-format.md`, "Services", the `alloc` result:
  `= alloc#1`; `alloc#?` names a block no earlier allocation of the file returned, as in `i services.SimFree alloc#?`
  for an order the file did not see made). Once a later line names the block as an order, it is `Order#n`.
- `PathOrderAttach#n` is the address of the new order's `Order.link` (the order + 18), an embedded `PathOrderAttach`
  record, named on its own while the recording has not yet named the order (`i units.UnitRefInit PathOrderAttach#1
  Unit#1 null`, then `i units.UnitRefSetValue PathOrderAttach#1 Order#5`). Once the order is named, its link is
  `Order#n.link` (`docs/replay-format.md`, "An embedded polymorphic object").

### Effects of other modules that reach `Apply`

`data/routes.csv` delivers to orders, besides orders' own event kinds, six effect kinds of other modules, all
synchronous. The engine turns each into the orders event below before `Apply` is called, so `Apply` only ever sees
kinds 288 to 296 (and `ExpandGroupCommand` 297 and 298). The kit states what each effect asks (`interfaces/ai.h`,
`interfaces/weapons.h`, `interfaces/movement.h`) but not how its fields become the event's; the mapping as `Apply`
receives it:

| Effect (publisher, kind) | Event `Apply` receives | Fields |
| --- | --- | --- |
| ai's ADD_ORDER 240 (`AiAddOrder`, 44) | ADD 288 (`OrdersAdd`, 44) | `unit`, `type`, `queue`, `target`, `hasPos`, `pos`, `params` copied |
| ai's QUEUE_BUILD 241 (`AiQueueBuild`, 16) | QUEUE_BUILD 289 (`OrdersQueueBuild`, 44) | `unit`, `count` copied; `name` the first 31 bytes of the `UnitDef.unitname` of entry `type` of `Game.defs`, the rest of the 32 bytes 0 |
| ai's STOP 243 (`AiStop`, 12) | STOP 290 (`OrdersStop`, 12) | `unit`, `all` copied |
| weapons' ATTACKED 272 (`WeaponsAttacked`, 16) | ATTACKED 293 (`OrdersAttacked`, 12) | `unit`, `attacker` copied (`amount` dropped) |
| weapons' PARALYZE 277 (`WeaponsParalyze`, 12) | PARALYZE 294 (`OrdersParalyze`, 12) | `unit`, `ticks` copied |
| movement's ORDER_FLAGS 336 (`MovementOrderFlags`, 16) | STATUS_FLAGS 292 (`OrdersStatusFlags`, 16) | `unit`, `order`, `flags` copied |

An effect shorter than its payload reaches no `Apply` call. Each event is delivered at once (the route is
synchronous), inside the publisher's call: a STATUS_FLAGS from movement's `New*` imports reaches `Apply` while the
import runs (section 4, "Imports that change the order").

### Events

The slots record these kinds of sim event (`docs/sim-events.md` gives each kind's arguments, flags and phases; every
record here has flags 0):

- `SIM_EVENT_UNIT_SPEECH`: "speech (k, t)" and "announce (t)", when a run of an order speaks (`RunOrders`, and
  `RunSecondaryOrders` for a kind of the second list).
- `SIM_EVENT_NANO_PARTICLES`: "the nano spray", from the build and repair kinds while they work.
- `SIM_EVENT_REVERSE_NANO_PARTICLES`: "the reverse spray", from the capture and reclaim kinds while they work.
- `SIM_EVENT_TELEPORT_PARTICLES`: `Teleport`, for each unit it moves.
- `SIM_EVENT_SELECTION_ORDERS_DIRTY`: "the orders dirty mark", when a build or help order starts, ends or is cut short,
  and when a unit under construction is finished.
- `SIM_EVENT_BUILD_MENU_UPDATE`: "the build menu update", when a factory's build count or a stockpile count changes.

`Apply`, `ExpandGroupCommand` and the queries record none of their own; `Apply` records what the runs of code 2 it
reaches record.

## 3. State

The terms of `docs/sim-events.md`, Terms (the call flags, the order flags, the unit flags, the states, the damage and
game types) are used here and in section 4 as that page defines them.

### Owned

**The `Order` record** (`layout.json` `Order`, 86 bytes). An order is an 86-byte allocation of the block
(`services.SimAlloc(86)`, zeroed), freed by `services.SimFree`. The orders of a unit form two singly linked lists
through `Order.next`: the *first list* starts at `Unit.list`, the *second list* at `Unit.list2`. An order is in the
second list exactly when its `flags_42` has `ORDER_FLAG_SECOND_LIST` (0x40000); "its list" always means the list that
bit names. The unit's *first order* is the order at `Unit.list` (0 when the list is empty).

| Offset | Size | Field (recordings' name) | Meaning common to every kind |
|---|---|---|---|
| 0 | 4 | the class word | names the record's class; written when the order is made. The kit gives no value for it ("Gaps in the kit"). |
| 4 | 1 | `kind` | the order's kind: its index in the order type table. |
| 5 | 1 | `state` | the kind's step counter, unsigned 8-bit; the frame sets it to 0 or adds 1 by result, and some runs write it. |
| 6 | 4 | `flags` | the wake mask: the conditions the order waits on; 0 means the order runs at every pass of its list. |
| 10 | 4 | `wakeFrame` | the tick at which the order wakes, unsigned; 0xffffffff for none. |
| 14 | 4 | `unit` | the unit the order belongs to, set when it is put in a list; the run is given this unit. |
| 18 | 16 | `link` | the order's reference to its target unit, owned by units' reference imports; `link.owner` (offset 22, also `Order.target`) is the target unit, `link.next` the next reference to that unit, `link.value` the order itself. The module writes these words only through those imports. |
| 34 | 12 | `posX`, `posY`, `posZ` | the order's position (`Order.pos`), world coordinates with 16 fractional bits. |
| 46 | 4 | `start` | two signed 16-bit words (`start.x` at 46, `start.y` at 48, recorded as `(x,y)`); (0,0) when made. |
| 50 | 4 | `cached` | two signed 16-bit words; (0,0) when made, never written by a run. |
| 54 | 4 | `ticks` | first argument word (signed 32-bit), the *a* of a new order; its meaning is the kind's. |
| 58 | 4 | `radius` | second argument word, the *b* of a new order; the kind's. |
| 62 | 4 | `progress` | third argument word, the *c* of a new order; the kind's. |
| 66 | 4 | `flags_42` | the order's flag word: its kind's table flags when made, with the bits below. |
| 70 | 4 | `created` | the tick the order was made, unsigned. |
| 74 | 4 | `next` | the next order of the same list, or 0. |
| 78 | 4 | `subFlags` | the pending flags: conditions that have happened and that the order has not yet been run for. |
| 82 | 4 | `attached` | the movement target the order has given its unit (a block offset that movement's `New*` imports answer), or 0. |

Section 4 calls `Order.target` *the target* of the order. A kind that gives a union word a meaning says so: "`Order.ticks`
(here: the countdown)".

**`Order.flags` and `Order.subFlags`.** A condition is *pending* when it is in `flags` and in `subFlags` or in the
unit's `Unit.netDirtyFlags` (16 bits, zero-extended to 32). The conditions are bits of one 32-bit space; the code a run
is given is the set of pending bits, and the kinds test its bits (the call flags of `docs/sim-events.md`, Terms):

| Bit | Meaning to `RunOrders` |
|---|---|
| 0x1 | the wake: `subFlags` gains it when the tick reaches `wakeFrame`. "A wake delay of n" sets it in `flags`. |
| 0x2 | set in `flags` at the moment the order is deleted, it makes the deletion run the order with code 2 (call flag 2). |
| 0x8, 0x20 | call flag 8 and call flag 0x20. |
| 0x3e0 | "attach target T" clears these bits from `subFlags` when it attaches a target. |
| 0x10000 | when pending, the frame clears the unit's three weapons' targets (targeting's CLEAR) before the run. It can be pending only from `subFlags`. |

`subFlags` gains bits from STATUS_FLAGS events and from the wake. Before every run, the frame of `RunOrders` removes the
pending bits from `subFlags` and sets `flags` to 0; the run then sets `flags` to what it next waits on.

**`Order.flags_42`.** It starts as the table flags of the order's kind ("a new order"). The bits that matter:

| Bit | Meaning |
|---|---|
| 0x4 | the order stays: "deleting the unit's orders (all 0)", BE_CARRIED and "adding an order" without the queue keep it. |
| 0x20 | (table) "adding an order" puts an order with 0x20 or 0x40000 at the front of its list. |
| 0x40 | (table) "adding an order" without the queue keeps the unit's orders. |
| 0x80 | `ORDER_FLAG_NO_UNDERATTACK_LINE` (order flag 0x80). |
| 0x100 | (table) a factory's build queue entry: `ticks` is the unit type number. |
| 0x200 | `ORDER_FLAG_TARGET`: the order has a target unit. Cleared when made with no target. |
| 0x400 | `ORDER_FLAG_POSITION`: the order has a position. Cleared when made with no position. |
| 0x1000 | the marked order of "inserting O after the marked order". |
| 0x2000 | `ORDER_FLAG_ANNOUNCE` (order status flag 0x2000): "announce (t)" records the line once and clears it. |
| 0x4000 | the order was inserted as a command ("inserting a command of kind K"); an order inserted before an order with the bit gains it. |
| 0x8000 | the order has added its patrol return order. |
| 0x10000 | set on an order deleted while it is not the order the operation measures against (the list operations say which): its end publishes no weapons RELEASE. |
| 0x20000 | `ORDER_FLAG_IDLE`. |
| 0x40000 | `ORDER_FLAG_SECOND_LIST`. |
| 0x100000 | (table) a build kind; `Follow_Ground` and `VTOL_Follow` test it on another unit's first order. |
| bit 22 (4194304) | the order has started its unit's building script: "stop the building script" and the end of the order stop it. |
| 0x800000 | set by result 9 of `RunOrders`. Movement reads it in the block. |
| 0x1000000 | (table) the engine slots number the saved kinds without it. |

**The order type table** (`SimGlobals.g_missionOrderTableVec`): a block vector of `OrderTableEntry` records
(`layout.json` `OrderTableEntry`, 25 bytes): `value` (0, the kind's status text), `notify` (4, the kind's run, a code
pointer), `draw` (8), `drawMask` (12), `markerAnim` (16, 1 byte), `flags` (17, the kind's table flags), `name` (21,
the kind's name). The engine fills it (`RegisterAllOrderTypes`) and empties it (`ClearOrderTypeTable`); the module
only reads it. It holds 68 entries, sorted by `name` compared without case, entry 0 being `Ready` (name "", flags 0).
The recordings carry it whole at the first call (`r SimGlobals.g_missionOrderTableVec [68/68]` and a line per field).

**Which behaviour runs.** The run of an order of kind K is the behaviour of the code table entry `orderNotify[j]` that
entry K's `notify` names (`r SimGlobals.g_missionOrderTableVec[K].notify orderNotify[j]`); `data/replay_schema.json`
(`annotations.code_tables.orderNotify.entries`) names entry j by the order type it stands for. Section 4, "Order
kinds", gives the behaviour each index j runs, under the kind whose entry names it: `orderNotify[64]` is the behaviour
of both `QMove` and `QPatrol`, and `orderNotify[54]` of both `SelfDestruct` and `SelfDestructFG`. The run is given (the
order's `Order.unit`, the order, the code) and answers a *result*.

The table as the recordings hold it (flags in hex; `notify` as the `orderNotify` index):

| K | `name` | `value` | `flags` | `notify` |
|---|---|---|---|---|
| 0 | "" (Ready) | "Ready" | 0x0 | [0] |
| 1 | "Activate" | "Activate" | 0x10060 | [46] |
| 2 | "AirStrike" | "Airstrike" | 0x600 | [29] |
| 3 | "AirToAir" | "Engaging target" | 0x200 | [30] |
| 4 | "AirToGround" | "Engaging target" | 0x200 | [31] |
| 5 | "AirToGroundHover" | "Engaging target" | 0x200 | [32] |
| 6 | "Attack_Chase" | "Attacking" | 0x280 | [6] |
| 7 | "Attack_Kamikaze" | "Attacking" | 0x600 | [7] |
| 8 | "Attack_NoMove" | "Attacking" | 0x280 | [45] |
| 9 | "AttackSpecial" | "Annihilating" | 0x680 | [8] |
| 10 | "AttackUType" | "Attacking" | 0x4 | [61] |
| 11 | "BeCarried" | "Being transported" | 0x24 | [57] |
| 12 | "BuildingBuild" | "Nanolathing" | 0x10010c | [52] |
| 13 | "BuildWeapon" | "Nanolathing" | 0xc0140 | [53] |
| 14 | "Capture" | "Capturing" | 0x200 | [17] |
| 15 | "Cloak_Off" | "Decloaking" | 0x10060 | [49] |
| 16 | "Cloak_On" | "Cloaking" | 0x10060 | [48] |
| 17 | "Deactivate" | "Deactivate" | 0x10060 | [47] |
| 18 | "Follow_Ground" | "Guarding" | 0x200 | [4] |
| 19 | "GetBuilt" | "Under construction" | 0x224 | [56] |
| 20 | "Ground_Pickup" | "Loading" | 0x200 | [65] |
| 21 | "Ground_Unload" | "Unloading" | 0x400 | [11] |
| 22 | "Guard_NoMove" | "Ready" | 0x20 | [62] |
| 23 | "HelpBuild" | "Nanolathing" | 0x100208 | [14] |
| 24 | "MakeSelectable" | "Unit is available" | 0x4 | [58] |
| 25 | "MobileBuild" | "Nanolathing" | 0x100508 | [13] |
| 26 | "Move_Ground" | "Moving" | 0x402 | [3] |
| 27 | "Paralyze" | "Paralyzed" | 0x24 | [55] |
| 28 | "Park" | "Parking" | 0x0 | [9] |
| 29 | "Patrol" | "Patrolling" | 0x412 | [10] |
| 30 | "QMove" | "Ready with orders" | 0x400 | [64] |
| 31 | "QPatrol" | "Ready with orders" | 0x400 | [64] |
| 32 | "Reclaim" | "Reclaiming" | 0x100800 | [19] |
| 33 | "ReclaimUnit" | "Reclaiming" | 0x100200 | [20] |
| 34 | "RepairPatrol" | "Repair patrol" | 0x412 | [15] |
| 35 | "RepairUnit" | "Repairing" | 0x100200 | [16] |
| 36 | "RepairUnitNoMove" | "Repairing" | 0x200 | [21] |
| 37 | "Resurrect" | "Resurrecting" | 0x200 | [18] |
| 38 | "SelfDestruct" | "SELF DESTRUCT ENGAGED" | 0x40040 | [54] |
| 39 | "SelfDestructFG" | "SELF DESTRUCT ENGAGED" | 0x0 | [54] |
| 40 | "SelfRepair" | "Repairing" | 0x1000204 | [63] |
| 41 | "Standby" | "Standby" | 0x20000 | [1] |
| 42 | "Standby_Mine" | "Standby" | 0x1020000 | [2] |
| 43 | "Standing_FireOrder" | "Acknowledged" | 0x10060 | [51] |
| 44 | "Standing_MoveOrder" | "Acknowledged" | 0x10060 | [50] |
| 45 | "Stop" | "Stopping" | 0x0 | [44] |
| 46 | "Suppress" | "Suppressing fire" | 0x410 | [5] |
| 47 | "Teleport" | "Teleporting" | 0x600 | [12] |
| 48 | "VTOL_Evade" | "Evading" | 0x0 | [39] |
| 49 | "VTOL_Follow" | "Guarding" | 0x200 | [27] |
| 50 | "VTOL_GetRepaired" | "Under repair" | 0x200 | [42] |
| 51 | "VTOL_HelpBuild" | "Nanolathing" | 0x100208 | [34] |
| 52 | "VTOL_LandIfCan" | "Seeking to land" | 0x400 | [43] |
| 53 | "VTOL_Landing" | "Landing" | 0x600 | [24] |
| 54 | "VTOL_MobileBuild" | "Nanolathing" | 0x100508 | [33] |
| 55 | "VTOL_Move" | "Moving" | 0x402 | [23] |
| 56 | "VTOL_Patrol" | "Patrolling" | 0x412 | [28] |
| 57 | "VTOL_Pickup" | "Loading" | 0x200 | [25] |
| 58 | "VTOL_Reclaim" | "Reclaiming" | 0x100800 | [37] |
| 59 | "VTOL_ReclaimUnit" | "Reclaiming" | 0x100200 | [38] |
| 60 | "VTOL_RepairPatrol" | "Repair patrol" | 0x412 | [35] |
| 61 | "VTOL_RepairUnit" | "Repairing" | 0x100200 | [36] |
| 62 | "VTOL_SeekAttack" | "Seeking to attack" | 0x600 | [40] |
| 63 | "VTOL_SeekGuard" | "Seeking to guard" | 0x600 | [41] |
| 64 | "VTOL_Standby" | "Standby" | 0x20000 | [22] |
| 65 | "VTOL_Unload" | "Unloading" | 0x400 | [26] |
| 66 | "Wait" | "Waiting" | 0x4 | [59] |
| 67 | "WaitForAttack" | "Waiting for attack" | 0x204 | [60] |

**`Unit` fields.** The module writes two `Unit` fields directly: `Unit.list` and `Unit.list2`, as orders are put in,
moved and taken out. Every other change to a unit, a player, a feature or a weapon is an effect it publishes, or an
import's own write.

### Read

| Field | Owner | Use |
| --- | --- | --- |
| `Game.ticks` (the *tick*), `Game.units`, `Game.unitsEnd` (the last unit record), `Game.defs`, `Game.features`, `Game.featureCount`, `Game.cells`, `Game.spots`, `Game.width`, `Game.players`, `Game.seaLevel` (unsigned byte), `Game.overflowBucket`, `Game.mapWidth`, `Game.mapHeight`, `Game.visibilityMask`, `Game.playerIndex`, `Game.localPlayer`, `Game.interfaceType` | game | the tick; the unit array (280-byte records), the unit types, the features (256-byte records), the cells (13-byte records), the spots and the map; the queries' visibility tests |
| `Game.width_142a3`, `Game.height_142a7` | game | the columns and rows of the table of spatial buckets that `Unit.spatialBucket` and `Game.overflowBucket` point into; the recordings read them to size that table and name the buckets (`r Game.width_142a3 28`). The module only compares the two offsets ("the off-map return") and need not read them |
| `SimGlobals.g_missionOrderTableVec` | engine | the order type table |
| `Unit.list`, `list2`, `netDirtyFlags`, `motion` (whether it is 0, and its `UnitMotion.flags` low two bits, the flight mode), `def`, `player`, `playerIndex`, `unitDefIndex`, `id`, `flags`, `activateFlags`, `cobStateFlags`, `pos` (`posX`, `posY`, `posZ`; `xWhole`, `zWhole` the high halves), `heading`, `cell`, `footprint`, `spatialBucket`, `carrier`, `cargo`, `script`, `buildLeft` (f32), `health` (signed 16-bit), `killCount` (unsigned 16-bit), `attacker`, `lastAttackerSlot`, `lastDamageType`, `postTransferHoldoff`, `moveOrder`, `fireOrder`, `group`, `weapons[i]` (`weapon<i>Flags`, `weapon<i>Def`, `weapon<i>Stockpile`) | units | the running unit, its target and the units the kinds look at |
| `UnitDef.flags1`, `flags2` (and `flags2Byte`), `categories`, `weaponCategories` (`weaponCategories0` to `2`, `UnitTypeSet` records), `weapons` (`weapons0`), `defaultmissiontype`, `ids`, `range`, `buildRange` (unsigned 16-bit), `buildRate` (unsigned 16-bit), `buildtime`, `maxHealth` (unsigned 32-bit), `metalCost`, `energyCost` (f32), `maneuverleashlength` (unsigned 16-bit), `kamikazedistance`, `attackrunlength`, `altitude` (signed 16-bit), `maxvelocity`, `capacity`, `height180`, `radius184`, `minwaterdepth`, `footprintx`, `footprintz`, `modelMinX` to `modelMaxZ`, `modelMaxYHigh`, `selfdestructcountdown` | units | the unit type of a unit, or entry `ticks` of `Game.defs` |
| `UnitMotion.flags`, `UnitMotion.speed`, `UnitMotion.velocityX`, `velocityY`, `velocityZ` | movement | the flight mode; `AirStrike` and `AirToAir` |
| `Player.active` (32-bit), `type` (8-bit), `index`, `allied`, `energy`, `energyCapacity`, `metal`, `metalCapacity` (f32), `exploredWidth`, `exploredHeight` | players | the default order, the relation tests, the repair patrols |
| `WeaponDef.flags`, `range`, `reloadTime`, `metalPerShot`, `energyPerShot` | weapons | the attack kinds, `BuildWeapon` |
| `Feature.flags`, `flags16`, `energy`, `metal` (f32), `height`, `footprintX`, `footprintZ`, `name` | features | the reclaim and resurrect kinds; `GetOrderType`'s feature at P |
| `Cell.feature` (16-bit), `offsetX`, `offsetY`, `spot` | los_radar | the feature at P; `Resurrect` |
| `FeatureSpot.rot` | features | `Resurrect`'s angles |

A recording's read set may list fields the module need not read: `r` lines are the call's inputs, and a read is
never compared.

### Between ticks

Each order's 86 bytes, the two lists and the table are block state; the module keeps nothing else. The engine slots
read and write the module's records in the block, so they must be laid out exactly as `layout.json` says:

- `RegisterAllOrderTypes` writes `SimGlobals.g_missionOrderTableVec`, the 68 entries above; `ClearOrderTypeTable`
  empties it.
- `LoadOrders` makes the unit's saved orders (each an 86-byte `services.SimAlloc` with its `link` set through units'
  reference imports), writes every `Order` field from the bank, links them into `Unit.list` and `Unit.list2` (by
  `ORDER_FLAG_SECOND_LIST`) in saved order, and, when the first order of `Unit.list` has a non-zero `attached`,
  publishes movement's SET_TARGET (12): `unit` the order's `Order.unit`, `target` its `attached`.
- `SaveOrders` reads `Unit.list` then `Unit.list2` and every field of each order (and the kind's `name` in the
  table); it writes nothing of the module's.
- `WriteState` reads, for every unit with a unit type and at least one order, each order of both lists (`kind`,
  `state`, `flags`, `wakeFrame`, `start`, `ticks`, `radius`, `progress`, `created`, `subFlags`, `attached`).

## 4. Behaviour

Within one call, only the order of import calls (and so of `services.RandomInt` draws and of publishes) is
observable. The block writes are compared at the end of the call, so the order they are listed in carries no meaning.
Two rules of the replay bound that comparison:

- **An order freed in the call.** An object that `services.SimFree` frees is not walked again until
  `services.SimAlloc` hands its address out again (`docs/replay-format.md`, "An object field that an import
  changes"), and the end-of-call walk reaches only the objects that the call's arguments and their pointer fields lead
  to ("Objects"). So the writes a run makes
  to an order that is then deleted in the same call (its `state`, `flags`, `wakeFrame`, ...) have no `w` lines and are
  not compared; only the imports and publishes of its end are. A new order allocated at a freed order's address is a
  new object (`Order#5`, not the name of the order freed there).
- **Imports that change the order.** An import can change the order's own fields before it returns. Movement's `New*`
  imports deliver movement's ORDER_FLAGS at once, which reaches `Apply` as STATUS_FLAGS (section 2) and ORs its `flags`
  into `Order.subFlags`; the recording shows it as an `o` line after the import (`i movement.NewApproachRadius Order#4
  45878325 19689825 16 = ApproachRadius#1`, then `o Order#4.subFlags 128`; `docs/replay-format.md`, "An object field
  that an import changes"). The module reads such a field from the block after the import, never from a copy taken
  before it: the end-of-call `w` line compares the value the block holds at the end, the import's bits included, and
  the next pass of `RunOrders` sees them (a condition the run then waits on may already be pending).

Every publish is `services.Emit(payload, size)`, recorded as `services.SchedPublish 3 <size>`, and named here KIND
(size) with its payload. Whether a publish is delivered at once (its subscriber's writes follow it in the recording) or
queued is the route's (`data/routes.csv`). A unit, an order or a target in a payload is its block offset (0 for none).
*U* is the unit whose order runs (the order's `Order.unit`), *O* the order, *c* the code it is given, *D* `U`'s
`UnitDef`, *T* the order's target (`Order.target`) unless a row says otherwise, and "the tick" is `Game.ticks`. A
condition "*c* has *m*" means *c* AND *m* is not 0.

### Terms

Each term is defined once; section 4 uses it by its name, in quotes where the name could be read as ordinary words.

#### Offsets, arithmetic and names

- **Following a block offset.** A unit, an order or a target named in a payload or held in a block field is a block
  offset (`docs/module-abi.md` 7.1): 0 is none, any other value the record at the block base plus the offset. A
  `block` answer of an import (and `services.SimAlloc`'s answer) is a linear address: it is turned into an offset (its
  address minus the block base) before it is stored in a block field, and offsets are turned into addresses (plus the
  block base) before they are passed as `block` or `linear` arguments.
- **The unit array.** The unit records sit one after another, 280 bytes apart, from `Game.units` to `Game.unitsEnd`
  (the last record). Unit *i* is the record at `Game.units` + 280 × *i*; "the unit *i* records after *u*" is the
  record at *u*'s offset + 280 × *i*.
- **Order type names.** A rule that names an order type by a string, such as `MOBILEBUILD`, means the index that
  `MissionTypeFromName` gives that string at the moment the rule uses it (0, `Ready`, when the table lacks it). The
  indices of the recordings' table are given in brackets: `MOBILEBUILD` (`MobileBuild`, 25). The rule is the name.
- **A kind byte** is a 32-bit payload field truncated to its low 8 bits, as an index in the order type table.
- **whole(v)**: the high 16 bits of a 32-bit word, as a signed 16-bit value (`Unit.xWhole` is whole(`Unit.posX`),
  `Unit.zWhole` whole(`Unit.posZ`)).
- **S(a, s), C(a, s)**: the scaled sine and cosine of the kit's runtime (`runtime/vectors/README.md`, "Scaled sine and
  cosine") of the low 16 bits of an angle *a* and a signed 32-bit scale *s*.
- **Step(a, s)** = (−S(a, s), 0, −C(a, s)), each negation wrapping. P + Step(a, s) and P − Step(a, s) add or subtract
  the three words (signed 32-bit, wrapping), so the y word stays P's.
- **A(x, z)**: the angle of a vector of the kit's runtime ("Angle of a vector") of two signed 32-bit values, kept as
  its low 16 bits, signed. **Head(P, Q)** = A(P.x − Q.x, P.z − Q.z), the differences signed 32-bit, wrapping: the
  heading from P towards Q.
- **hyp(x, y)**: the kit's `hypot` of two ints converted to double. **ftol(v)**: the kit's `ftol` (truncation toward
  zero, the low 32 bits; 0 for a NaN or an infinity).
- **Cell(P, f)**, the cell of a position P for a footprint *f* (two signed 16-bit words *f.x*, *f.y*): ((P.x − *f.x* ×
  0x80000 + 0x80000) >> 20, (P.z − *f.y* × 0x80000 + 0x80000) >> 20), signed 32-bit, wrapping, the shift arithmetic,
  each result kept as its low 16 bits, signed. **Centre(c, f)**: x = (*f.x* + 2 × *c.x*) << 19 and z = (*f.y* + 2 ×
  *c.y*) << 19, signed 32-bit, wrapping. **Snapping P to f**: P's x and z become Centre(Cell(P, f), f); its y stays.
- **The build heading to P**: the low 16 bits, unsigned (0 to 65535), of Head(`U.pos`, P) − `U.heading`.

#### The list operations

- **Inserting O before B** (B an order of O's list, or 0 for the end of that list): O takes B's place in its list, so
  the order (or the list start) that pointed to B now points to O; `O.unit` becomes the unit and `O.next` becomes B;
  when B is not 0, `O.flags_42` gains B's bit 0x4000 (`O.flags_42` OR (`B.flags_42` AND 0x4000)).
- **Inserting O at the front**: inserting O before the order that starts its list (before 0 when the list is empty),
  so O becomes the first order of its list. Section 4 says "at the front" for it.
- **Adding O to the tail**: O goes at the end of its list: the last order's `next` (or the list start, for an empty
  list) becomes O, `O.unit` becomes the unit and `O.next` 0. `O.flags_42` is unchanged. Section 4 says "at the
  tail".
- **Inserting O after the marked order**: always in the first list. `O.unit` becomes the unit and `O.flags_42` gains
  0x1000. When an order of the first list has `flags_42` bit 0x1000 (O is not yet in the list), the earliest such
  order M loses that bit, `O.next` becomes `M.next` and `M.next` becomes O. When none has it, O is added at the end of
  the first list (`O.next` 0).
- **Inserting a command of kind K**: a new order (kind K, no target, no position, 0, 0, 0) whose `flags_42` gains
  0x4000, inserted at the front.
- **Moving O to the back**: O, which is in the first list, is taken out of it and put at its end: the order (or list
  start) that pointed to O now points to `O.next`, the last order's `next` (or the list start, when O was alone)
  becomes O, and `O.next` becomes 0. No flag changes.
- **Removing O** (measured against an order F that the operation names): O is looked for in its list from the start.
  When it is found, the order (or list start) that pointed to it now points to `O.next`; when O is not F, `O.flags_42`
  gains 0x10000; then "the end of an order" for O. When O is not in its list, nothing happens. The 0x10000 bit is set
  before the end's effects, so an order removed while it is not F publishes no weapons RELEASE (Q14).
- **Deleting an order** O is removing O measured against the unit's first order at that moment.
- **Deleting the unit's orders (all *a*)**: let F be the unit's first order at the start. Each order of the first list
  in turn, from the start: when *a* is 0 and its `flags_42` has bit 0x4, it stays; otherwise it is unlinked (the list
  skips it), gains `flags_42` 0x10000 when it is not F, and its end follows. Then, when *a* is not 0, the order that
  starts the second list is removed (measured against `Unit.list`, which is 0 by then, so every one gains 0x10000)
  until the second list is empty. Each end completes, its imports and effects in order, before the next order is
  looked at.
- **Adding an order (kind K, queue *q*, target T, position P, *a*, *b*)**, in this order:
  1. a new order O (kind K, target T, position P, *a*, *b*, 0) is made (its allocation and `units` imports come
     first);
  2. when *q* is 0 and `O.flags_42` lacks bit 0x40: every order of the first list without `flags_42` bit 0x4 is
     unlinked and ended, from the start, each gaining 0x10000 when it is not the first order the unit had when this
     step began;
  3. when `O.flags_42` lacks 0x40000: while the unit's first order has `flags_42` bit 0x4000, it is removed (measured
     against itself, so it gains no 0x10000);
  4. the result: `O.flags_42` has gained 0x1, and 0x2000 (`ORDER_FLAG_ANNOUNCE`) when *q* is 0; O is then inserted at
     the front (of its list: the second list when it has 0x40000) when its `flags_42` has any of 0x40020, else inserted
     after the marked order (always in the first list).

  *q* is compared with 0 only. The bits looked at are `O.flags_42`'s: the table flags with 0x200 cleared when T is 0
  and 0x400 cleared when P is none.

#### An order's life

- **The run of an order with a code**: the behaviour of the order's kind ("Which behaviour runs", section 3), given
  `Order.unit`, the order and a code: the pending bits from `RunOrders`, 0 from `RunSecondaryOrders`, 2 at the end of
  an order whose `flags` has 0x2. It answers a *result*, 0 to 9; `RunOrders` and `RunSecondaryOrders` say what each
  result does.
- **A new order (kind K, target T, position P, *a*, *b*, *c*)**. Imports, in order:
  1. `services.SimAlloc(86)`: the order's address (zeroed);
  2. `units.UnitRefInit(link, T, 0)`, link being the address of the order's `Order.link` (the order + 18), T the target
     unit or 0;
  3. `units.UnitRefSetValue(link, order)`;
  4. only when the order's `flags_42` (below) has no `ORDER_FLAG_TARGET`: `units.UnitRefSetUnit(link, 0)`.

  Fields: the class word; `kind` K (8-bit); `state` 0; `flags` 0; `subFlags` 0; `wakeFrame` 0xffffffff;
  `posX`/`posY`/`posZ` the 12 bytes at P, or 0, 0, 0 when there is no P; `start` (0,0); `cached` (0,0); `ticks` *a*;
  `radius` *b*; `progress` *c*; `created` the tick; `unit` 0; `next` 0; `attached` 0; `flags_42` the `flags` of table
  entry K, less `ORDER_FLAG_TARGET` (0x200) when T is 0, less `ORDER_FLAG_POSITION` (0x400) when there is no P. The
  order is in no list until a list operation puts it in one. A new order of a kind named N takes K from the name
  ("Order type names").
- **The end of an order** (what removing and deleting do to each order they drop), in this order, with *u* the order's
  `Order.unit`:
  1. when `flags` has 0x2: the run of the order with code 2 (the order is already out of its list); its result is
     ignored;
  2. when `flags_42` has bit 22 (4194304): "stop the building script";
  3. when `Unit.motion` of *u* is not 0: `movement.UnitTarget(u)`; when the answer is not 0 and equals `attached`,
     movement's SET_TARGET (12) (`unit` *u*, `target` 0) then DESTROY_TARGET (12) (`unit` *u*, `target` `attached`);
     otherwise, when `attached` is not 0, DESTROY_TARGET (12) alone (Q15). `attached` becomes 0. When `Unit.motion` is 0,
     nothing of this;
  4. when `flags_42` has no 0x10000: weapons' RELEASE (12): `unit` *u*, `weapon` 3;
  5. UNLINK (12): `unit` the order's `link.owner` (0 for none), `ref` the block offset of the order's `link`;
  6. `services.SimFree(order)`.
- **A wake delay of n**: `flags` gains 0x1 and `wakeFrame` becomes the tick + n (unsigned 32-bit, wrapping).
- **Taking kind K's table flags**: `kind` becomes K, and `flags_42` becomes the `flags` of table entry K, except bits
  0x200 and 0x400, which keep their values.
- **The patrol return order** (for order O of *U*): when no order of `Unit.list` has `flags_42` bit 0x8000, a new order
  (kind `O.kind`, no target, position `U.pos`, 0, 0, 0) at the tail (with no 0x4000 from any order). Then, in every
  case, `O.flags_42` gains 0x8000.

#### Orders a run issues

- **The attack order (V, forced)** answers 1 when it added orders, 0 otherwise:
  1. 0 (no import) when V is *U*; when forced is 0 and `U.flags` AND 0xc0000 (`moveOrder`) is 0; or when forced is 0 and `U.flags`
     AND 0x300000 (`fireOrder`) is 0;
  2. K = `GetOrderType(3, U, V, none)` (this module's own rule, run inside the call; mode 3 makes no import); 0 when K
     is 0;
  3. when forced is 0 and `U.flags` AND 0xc0000 (`moveOrder`) is 0x40000: R = `GetOrderType(2, U, none, U.pos)` (its imports here); a
     new order (kind R, no target, position `U.pos`, 0, 0, 0) at the front; then a new order (kind K, target V, no
     position, 0, 0, `D.maneuverleashlength`) whose `start` becomes (`U.xWhole`, `U.zWhole`), at the front. So the
     attack is first and the move second. R is not checked: an R of 0 makes an order of kind 0 (Q11);
  4. otherwise a new order (kind K, target V, no position, 0, 0, 0) at the front;
  5. 1.
- **The repair order (V)** answers 1 when it added orders, 0 otherwise. K = `GetOrderType(8, U, V, none)` (its imports
  here); 0 when K is 0. Then by m = `U.flags` AND 0xc0000 (`moveOrder`):

  | m | Effects | Answer |
  |---|---|---|
  | 0 | R = `GetOrderType(2, U, none, U.pos)`; a new order (kind R, no target, position `U.pos`, 0, 0, 0) at the front; then a new order (kind K, target V, no position, 0, 0, `D.range`) whose `start` becomes (`U.xWhole`, `U.zWhole`), at the front | 1 |
  | 0x40000 | the same with `progress` `D.maneuverleashlength` | 1 |
  | 0x80000 | a new order (kind K, target V, no position, 0, 0, 0) at the front | 1 |
  | 0xc0000 | nothing | 0 |

#### Targets for movement

- **Release the target**: when `Unit.motion` of *U* is not 0 and `attached` is not 0: movement's SET_TARGET (12)
  (`unit` *U*, `target` 0), then DESTROY_TARGET (12) (`unit` *U*, `target` `attached`); `attached` becomes 0.
  Otherwise nothing.
- **Attach target t**: when `Unit.motion` of *U* is not 0: release the target; then when *t* is not 0, `subFlags` loses
  0x3e0, movement's SET_TARGET (12) (`unit` *U*, `target` *t*) and `attached` becomes *t*. When `Unit.motion` is 0:
  nothing (*t* is not kept). "Attach target 0" is a release.
- **The approach goal (P, r)**: when `D.flags1` has no 0x800: `movement.NewApproachRadius(O, P.x, P.z, r)`, then attach
  target (its answer). Otherwise: release the target, and no import.
- **The ring goal (P, r1, r2)**: the same with `movement.NewRingApproach(O, P.x, P.z, r1, r2)`.
- **The footprint marker (cell, size)**: the same with `movement.NewPointMarker(O, cell, size)`, cell and size each a
  `Point16` passed by address (recorded as `(x,y)`).

  The `New*` import of these three goals is called even when `Unit.motion` is 0, and its answer is then neither
  attached nor destroyed (Q16).
- **A path target to P**: `movement.NewPathOrderAt(O, P)`, P a `Vec3` passed by address; its answer is a target, *t*.
- **The move goal (P, r)**: a path target to P; movement's SET_APPROACH_RADIUS (16) (`unit` *U*, `target` *t*, `value`
  the low 16 bits of r, signed); attach target *t*.
- **SET_ALTITUDE (t, v)**, **SET_APPROACH_RADIUS (t, v)**, **SET_HEADING (t, v)**: movement's events of that name, 16
  bytes, `unit` *U*, `target` *t*, `value` *v* (signed 32-bit); **SET_FLIGHT_MODE (m)**: movement's, 12 bytes, `unit`
  *U*, `mode` *m*.
- **The climb preparation**, in order: CLAIM (3); when `U.carrier` is not 0, ATTACH_TO_PIECE (*U*, 0, −1, 2);
  SET_STATE_BITS (1, 1); then, only when `UnitMotion.flags` of `U.motion` (as the block holds it after those
  synchronous effects) has its low two bits (`UnitMotion.mode`) equal to 1: SET_FLIGHT_MODE (2), a path target to `U.pos`, SET_ALTITUDE
  (*t*, `D.altitude` / 2, truncated toward zero), attach target *t*, and `flags` gains 0xe0.
- **The off-map return** (a test and its effects): when `U.spatialBucket` equals `Game.overflowBucket`: with the centre
  M = ((`Game.mapWidth` / 2) << 16, 0, (`Game.mapHeight` / 2) << 16) (divisions truncated toward zero), the move goal
  (`U.pos` + Step(Head(`U.pos`, M), 0x3200000), 0x80); `flags` gains 0xe0; result 2.

#### Speech, particles and scripts

- **Speech (k, t)**: when `U.flags` has unit flag 0x10000000 (`alive`) and lacks unit flag 0x4000,
  `services.RecordSimEvent(1, 0, [U.id, k, t], 3)`: `SIM_EVENT_UNIT_SPEECH`, voice slot *k*, speech text number *t*
  (`interfaces/sim_events.h`; 0 is the kind's text), flags 0. Otherwise no import.
- **Announce (t)**: when `flags_42` has `ORDER_FLAG_ANNOUNCE` (0x2000): the bit is cleared and speech (5, t).
  Otherwise nothing.
- **The orders dirty mark**: `services.RecordSimEvent(15, 0, [U.id], 1)`, `SIM_EVENT_SELECTION_ORDERS_DIRTY`.
- **The build menu update**: `services.RecordSimEvent(16, 0, [U.id], 1)`, `SIM_EVENT_BUILD_MENU_UPDATE`.
- **The nano point of N**: `scripts.GetNanoPiecePosition(N, out)`, *out* a `Vec3` of the module's; the three words it
  writes are the point.
- **The nano spray (o, lo, hi)**: `services.RecordSimEvent(3, 0, [o.x, o.y, o.z, lo.x, lo.y, lo.z, hi.x, hi.y, hi.z,
  6], 10)`, `SIM_EVENT_NANO_PARTICLES`. **The reverse spray (lo, hi, o)**: `services.RecordSimEvent(4, 0, [lo.x, lo.y,
  lo.z, hi.x, hi.y, hi.z, o.x, o.y, o.z, 6], 10)`, `SIM_EVENT_REVERSE_NANO_PARTICLES`.
- **The target box of V**: lo = (`V.posX` + E.`modelMinX`, `V.posY`, `V.posZ` + E.`modelMinZ`) and hi = (`V.posX` +
  E.`modelMaxX`, `V.posY` + E.`modelMaxY`, `V.posZ` + E.`modelMaxZ`), E being V's `UnitDef`, each sum signed 32-bit,
  wrapping. **The model box of V** is the same with lo.y = `V.posY` + E.`modelMinY`.
- **Start the building script (arg)**, *arg* an unsigned 16-bit value: `scripts.FindScript(U.script, "StartBuilding")`
  gives an index; START_SCRIPT (76): `unit` *U*, `form` 2 (`ORDERS_SCRIPT_BY_INDEX_WITH_ARGS`), `index` the index,
  `name` all zero, `callback` 0, `update` 0, `count` 1, `args` (*arg*, 0, 0, 0); `network.SendScriptCall(U, index, 1,
  arg, 0, 0, 0)` (answer ignored); `flags_42` gains bit 22 (4194304). The index is used as answered.
- **Stop the building script**: when `flags_42` has bit 22 (4194304): `scripts.FindScript(U.script, "StopBuilding")` gives an
  index; START_SCRIPT (76) as above with `count` 0 and `args` (0, 0, 0, 0); `network.SendScriptCall(U, index, 0, 0, 0,
  0, 0)`; `flags_42` loses bit 22 (4194304). Otherwise nothing.
- **START_SCRIPT "name" (form, update, count, a0, a1)**: START_SCRIPT (76): `unit` *U*, `form`, `index` 0, `name` the
  text, `callback` 0, `update`, `count`, `args` (a0, a1, 0, 0), a1 being 0 when it is not given.
- **The build stance wait (code)**: when `U.cobStateFlags` has no 0x1: `flags` becomes code OR 0x4 and the answer is 2.
  Otherwise the answer is 1 and nothing is written. A row that "takes the build stance wait (code)" has that answer as
  its result.
- **The script busy wait (code)**: when `U.cobStateFlags` has 0x2: `flags` becomes code OR 0x4 and the answer is 2.
  Otherwise the answer is 1.

#### Publishes

| Name here | Kind (size) | Payload |
| --- | --- | --- |
| CLEAR (w) | targeting's CLEAR 98 (12) | `unit` *U*, `weapon` w |
| SET_UNIT (w, V) | targeting's SET_UNIT 96 (16) | `unit` *U*, `weapon` w, `target` V |
| SET_POS (w) | targeting's SET_POS 97 (24) | `unit` *U*, `weapon` w, `pos` `O.pos` |
| CLAIM (w) | weapons' CLAIM 260 (12) | `unit` *U*, `weapon` w |
| RELEASE (w) | weapons' RELEASE 259 (12) | `unit` *U*, `weapon` w |
| DAMAGE (V, amount, kind) | weapons' DAMAGE 256 (24) | `unit` V, `source` *U*, `amount`, `damageKind` kind, `direction` 0 |
| ADD_STOCKPILE (w) | weapons' ADD_STOCKPILE 269 (12) | `unit` *U*, `weapon` w |
| SET_STATE_BITS (m, s) | orders' SET_STATE_BITS 304 (16) | `unit` *U*, `mask` m, `set` s |
| SET_POSITION (V, P, mode) | orders' SET_POSITION 305 (24) | `unit` V, `pos` P, `mode` |
| ATTACH_TO_PIECE (V, carrier, piece, mode) | orders' ATTACH_TO_PIECE 306 (20) | `unit` V, `target` carrier, `piece`, `mode` |
| FINISH_CONSTRUCTION (V) | orders' FINISH_CONSTRUCTION 307 (12) | `unit` V, `builder` *U* |
| BUILD_DECAY (n) | orders' BUILD_DECAY 308 (12) | `unit` *U*, `ticks` n |
| REPAIR (V, amount) | orders' REPAIR 309 (16) | `unit` V, `builder` *U*, `amount` (f32) |
| BUSY (n) | orders' BUSY 312 (12) | `unit` *U*, `until` the tick + n (signed 32-bit) |
| SET_FLAGS (cl, st) | orders' SET_FLAGS 313 (16) | `unit` *U*, `clear` cl, `set` st |
| SET_STANCE (V, f, mv, fi, g) | orders' SET_STANCE 315 (24) | `unit` V, `fields` f, `moveOrder` mv, `fireOrder` fi, `group` g |

The others (economy's REQUEST and INCOME, features' REMOVE and RECLAIM, players' GIVE_UNIT, orders' SET_ANGLES and
SET_BUILD_STATE, and orders' COMMAND, CLEAR_NET_DIRTY, UNLINK) are written out where they are used.

#### Tests and lists

- **The leash test** holds when `O.progress` (here: the leash) is not 0 and ftol(hyp(`U.xWhole` − `O.start.x`,
  `U.zWhole` − `O.start.y`)) ≥ `O.progress` (signed): the unit is past its leash. The differences are of the signed
  16-bit words, in 32 bits.
- **Health below** a limit: `health` (signed 16-bit) sign-extended to 32 bits and compared unsigned with the limit
  (so a negative health is never below, Q42). **Damaged (V)**: V's health is below (`maxHealth` of V's `UnitDef` >> 2) × 3
  (unsigned 32-bit). **Repaired (V)**: V's health is not below its `maxHealth`.
- **Repairable (V)**, for *U* with player P: V is not *U*; `P.allied[` V's `Player.index` `]` is not 0; `V.flags` AND 3
  is 1; not (V is repaired and `V.buildLeft` is 0.0, either sign); and not (`V.lastAttackerSlot` equals `P.index` and
  `V.lastDamageType` is 5). It reads only the block.
- **The units in range (P, r)**: `map.UnitsInRange(P, r, skip, ids, 256)` with *skip* 0, then again with *skip*
  increased by the count while a call answers 256; *ids* is a buffer of 256 unsigned 16-bit unit numbers, and each
  number *n* of a call's answer names unit *n* of the unit array, in order.
- **The repairable list (r)**: the units in range (`U.pos`, r) that are repairable, in order.
- **The ground allies (r)**: the units in range (`U.pos`, r) V for which V's player's `Player.allied[` *U*'s
  `Player.index` `]` is not 0, V's `UnitDef.flags1` lacks 0x800 (`flying`), and V is not *U*, in order.
- **The enemy count (P, r)**: `los_radar.GetVisibleEnemiesInRadius(U.playerIndex, P, r, 0, buf, 64)`, *buf* a buffer
  of 64 unit words; the answer is the count. **The enemy list (P, r)**: the enemy count gives *n*; when *n* > 64, a
  second call with a buffer of *n* words and capacity *n* gives *n2* (else there is no second call and *n2* = *n*). The
  list is the first min(*n2*, capacity of the last call) words of the last buffer.
- **The pad query**: `units.GetFactoriesInRadius(p, U.pos, 0xf00, buf, 64)`, *p* the `Player.index` of `U.player`,
  *buf* 64 unit words; when its answer *n* is above 64, a second call with the same arguments and a buffer of *n* words
  (capacity *n*) answers *n2*. The pads are the first min(last answer, last capacity) words.
- **The pad diversion**: when *U* is damaged: the pad query; when it gives at least one pad: release the target;
  `services.RandomInt(count)` = *k*; a new order (`VTOL_LANDING` (`VTOL_Landing`, 53), target pad *k*, no position, 0,
  0, 0) at the front; `flags` becomes 0; the diversion *happens*. When *U* is not damaged nothing is called; when there
  is no pad, only the query is.
- **The landing pad search (H, p)**, an answer: when *p* is not −1 and `map.IsPadSlotFree(H, p)` is not 0: *p*.
  Otherwise four words each set to −1, `scripts.QueryScript(H.script, "QueryLandingPad", &q0, &q1, &q2, &q3)` (answer
  ignored; it writes them); then for *i* = 0 to 3 in order, skipping a *qi* of −1 with no import, the first *qi* for
  which `map.IsPadSlotFree(H, qi)` is not 0. None: −1.
- **The build gap (P, f)** of *U* to a point P for a footprint *f*: g = whole(ftol(hyp(`U.posX` − P.x, `U.posZ` − P.z)))
  − ftol(hyp(`U.footprint.x`, `U.footprint.y`) × 8.0) + ftol(hyp(*f.x*, *f.y*) × −8.0), the differences signed 32-bit
  and wrapping before the conversion, the products in double, the sum signed 32-bit. P is *out of build range* when
  g > `D.buildRange` (signed).

#### Builds and features

- **The build step** (with V = the target), the state 3 of `HelpBuild` and `MobileBuild`: `units.AddBuildProgress(U,
  V, a)`, *a* the float of (`D.buildRate` / 30, integer division); when it answers not 0: the nano point of *U*, then
  the nano spray (that point, the model box of V), V read after the import; then BUSY (300); then when `V.buildLeft`
  (read after the import) is not 0.0: a wake delay of 1, `flags` gains 0xa, result 2; when it is 0.0: result 1.
  `BuildingBuild` state 3 is not the build step: its own row has the target box and no BUSY.
- **The air build pass (d)** (with T the target), `VTOL_HelpBuild` state 3 and `VTOL_MobileBuild` states 3 and 4. Imports,
  in order: when the tick (unsigned) mod 150 is 0: with *a* = Head(`U.pos`, T.pos) + 0xdb6e (32-bit), a path target to
  T.pos − Step(*a*, `D.buildRange` << 16), SET_HEADING (*t*, the low 16 bits of *a*, signed), attach target *t*; then
  `units.AddBuildProgress(U, T, a2)`, *a2* the float of (`D.buildRate` / 30, integer division); when it answers not 0:
  the nano point of *U*, then the nano spray (that point, the model box of T), T read as the block now holds it. Then:

  | `T.buildLeft` after the imports | Writes | Result |
  | --- | --- | --- |
  | not 0.0 | a wake delay of 1; `flags` gains 0xa | 2 |
  | 0.0 (either sign) | none | *d* |
- **The site cell and size**: `features.FindFeatureAtPos(O.pos, cell, size)`, two `Point16` outputs; its answer is a
  feature number (0xffff for none, `docs/sim-events.md`, Terms, "feature number 0xfffb"). F is then the `Feature`
  record of that number in `Game.features`.
- **The site point**: x = (size.x + 2 × cell.x) << 19, z = (size.y + 2 × cell.y) << 19 (signed 32-bit); then
  `services.RandomInt(F.height)` = *g*, then `map.GetGroundHeight` of that point (its y word not set) = *h*, and y =
  (*g* + *h*) << 16. The draw comes before the height import.
- **The feature box**: lo = (cell.x << 20, *h* << 16, cell.y << 20) with *h* = `map.GetGroundHeight` of lo (its y word
  not set before the call); hi = lo + (`F.footprintX` << 20, `F.height` << 16, `F.footprintZ` << 20), wrapping.
- **The reclaim pick (P, r)** answers four things: an energy point and amount and a metal point and amount. It is a
  call to map, `map.PickRandomReclaimableResourcesInRadius(P, r, &E, &eAmount, &M, &mAmount)`: P a `Vec3` passed by
  address, E and M two words of the module that hold, before the call, the linear addresses of two `Vec3` records of the
  module (the import writes a chosen point into the record a word points at, or sets the word to 0 for a kind with
  none), eAmount and mAmount two floats the import sets (an import the entry does not list yet: "Gaps in the
  kit"). The choice and its `services.RandomInt` draws are map's, not the module's: the module makes no draw for it. The module reads back the
  answer (1 when either kind has a resource, else 0) and, when it is 1, E and M (each either 0, none, or the linear
  address of a `Vec3` holding the point) and the two amounts (0 for a kind with none).
- **The reclaim pulse (a, b, n)**: a number, at least 1, for unit *a* working on unit *b* over *n* ticks:
  m = `UnitDef.metalCost` of *b* when it is greater than 10.0, else 10.0 (also when it is not a number); p =
  `UnitDef.buildRate` of *a* × ((`Unit.killCount` of *a* + 5) / 5, integer division) × `UnitDef.maxHealth` of *b* × *n*,
  each product signed 32-bit, wrapping; q = p read as an unsigned 32-bit number, as a double; d = q / (m × 300) in
  double (the product m × 300 is not rounded to a float); r = ftol(d); the pulse is r when r > 1, else 1. No import, no write.

### `Apply(event, size)`

The checks, in order, each refusing with outcome 0, no change and no import:

1. `size` below 8 (before `kind` is read);
2. `kind` outside 288 to 296 (so GROUP_COMMAND 297 and GROUP_BUILD 298 are refused here);
3. `size` below the kind's payload (section 2): ADD and COMMAND 44, QUEUE_BUILD 44, STOP 12, BE_CARRIED 8,
   STATUS_FLAGS 16, ATTACKED 12, PARALYZE 12, CANCEL 12;
4. the event's limits, which a malformed event fails, without a trap:
   - a *unit* is valid when it is not 0, at least `Game.units`, at most `Game.unitsEnd`, and a multiple of 280 bytes
     (the `Unit` record) from `Game.units`;
   - an *order* is valid when it is not 0 and its 86 bytes lie inside the block (its class word can't be checked:
     "Gaps in the kit");
   - a *type* is valid when its kind byte is below the count of entries of the order type table;
   - for STATUS_FLAGS, `order` must be valid, and `unit` is not looked at; for every other kind, `unit` must be valid;
   - ADD and COMMAND: `type` must be valid, and `target` 0 or a valid unit; ATTACKED: `attacker` 0 or a valid unit;
     CANCEL: `type` must be valid. QUEUE_BUILD, STOP, BE_CARRIED and PARALYZE have no limit beyond `unit`.

The recordings hold no event outside these limits and no outcome 0. Then:

| Kind | Effects | Outcome |
| --- | --- | --- |
| ADD 288 | adding an order (the kind byte of `type`, queue `queue`, target the unit at `target` (none for 0), position (`pos[0]`, `pos[1]`, `pos[2]`) when `hasPos` is not 0 and none otherwise, `params[0]`, `params[1]`) | 1 |
| QUEUE_BUILD 289 | changing a build count (below), with the first 31 bytes of `name` followed by a 0 (byte 31 is replaced by 0) | 1, whatever the count change did |
| STOP 290 | deleting the unit's orders (all `all`; any nonzero value is all) | 1 |
| BE_CARRIED 291 | when `Unit.carrier` is 0: nothing. Otherwise every order of the first list without `flags_42` bit 0x4 is unlinked and ended, from the start, each gaining 0x10000 when it is not the first order at the start (as "deleting the unit's orders (all 0)"); then a new order (kind `BECARRIED` (`BeCarried`, 11), target `Unit.carrier`, no position, 0, 0, 0) at the front | 1 |
| STATUS_FLAGS 292 | the order at `order` gets `Order.subFlags` OR'ed with `flags` | 1 |
| ATTACKED 293 | the attack response (below), with the unit at `attacker` | 1 |
| PARALYZE 294 | let P be the kind named `paralyze` (`Paralyze`, 27). When the unit's first order is not 0 and its `kind` is P: its `Order.ticks` (here: the paralysis ticks left) += `ticks`, signed 32-bit, wrapping. Otherwise a new order (kind P, no target, no position, `ticks`, 0, 0) at the front (Q1) | 1 |
| COMMAND 295 | the command rule (below) | 1 |
| CANCEL 296 | let K be the kind byte of `type`. In the list K's table entry names (the second list when the entry's `flags` has 0x40000, the first otherwise), the earliest order whose `kind` is K is removed (measured against the first order at that moment). None: no change | 1 when one was removed, 0 when none was found |

No event kind publishes anything of its own beyond what the list operations, "a new order" and "the end of an order"
publish, except ATTACKED (targeting's SET_UNIT) and the orders it adds. Hooks: none for `Apply`, but COMMAND names
AllowCommand (section 2).

**The command rule (COMMAND).** Let K be the kind byte of `type`, T the unit at `target` (0 for none), P the position
(`pos[0..2]`) when `hasPos` is not 0 and none otherwise. When `queue` is not 0, the first list is searched from the
start for the earliest order O that *matches*: `O.kind` is K; T is 0, or T is `O.target`; and P is none, or both
P.x − `O.posX` and P.z − `O.posZ` (each signed 32-bit, wrapping) lie in −0x100000 to 0x100000 inclusive (16 world
units; the y coordinates are not compared).

| When | Effects |
| --- | --- |
| `queue` not 0 and an order matches | that order is removed (measured against the first order at the start of the search); nothing is added (Q2) |
| `queue` not 0 and none matches, or `queue` 0 | adding an order (K, queue `queue`, target T, position P, `params[0]`, `params[1]`) |

Only the first list is searched, even when K's table entry names the second list (Q3).

**Changing a build count (QUEUE_BUILD).** Let N be the copied name and C `count`.

| When | The kind K and the type number *id* |
| --- | --- |
| N contains `MAKENUKE` or `MAKEANTI` anywhere (exact bytes, case-sensitive) | K is `BUILDWEAPON` (`BuildWeapon`, 13), *id* 0; no import |
| otherwise | `units.FindUnitTypeId(N)`; its answer's low 16 bits, unsigned, are *id*. *id* 0: nothing more happens. Else K is `MOBILEBUILD` (`MobileBuild`, 25) when `Unit.motion` is not 0, `BUILDINGBUILD` (`BuildingBuild`, 12) when it is 0 |

Then the count change, in the list L that K's table entry names (the second list when the entry's `flags` has
0x40000). A *build order of id* is an order of L whose `kind` is K and whose `Order.ticks` (here: the unit type number)
equals *id* (32-bit compare):

- **C above 0:** let Z be the last order of L. When Z exists, `Z.kind` is K and `Z.ticks` is *id*: `Z.radius` (here:
  the count queued) += C, signed 32-bit, wrapping, and nothing else. Otherwise: adding an order (K, queue 1, no target,
  no position, *id*, C), which places it as "adding an order" says: a `BuildWeapon` order (table flags 0xc0140, with
  0x40000) at the front of the second list; a `MobileBuild` (0x100508) or `BuildingBuild` (0x10010c) order, with
  neither 0x20 nor 0x40000, after the marked order of the first list (at its end when no order is marked) (Q4).
- **C 0 or below:** repeat: let B be the *last* build order of *id* in L. None: stop. When `B.radius` > −C (signed
  32-bit, −C wrapping): `B.radius` += C, and stop. Otherwise C += `B.radius` (wrapping), B is removed (measured against
  the unit's first order at that moment), and repeat with the new C. So with C 0, a last build order of *id* whose
  count is above 0 is left as it is, and one whose count is 0 or below is removed (Q5).

**The attack response (ATTACKED).** Let *u* be the unit, A the unit at `attacker` (none for 0, and none when
`A.unitDefIndex` is 0), *D* *u*'s `UnitDef` and Q *u*'s `Player`. A *type set* is a `UnitTypeSet` record (64 bytes,
`bits` 16 words); `UnitDef.categories` and `UnitDef.weaponCategories0` to `2` hold the block offset of one each. A set
at offset *s* *contains* type number *t* (an unsigned 16-bit `unitDefIndex`, 0 to 65535) when bit (*t* AND 31) of the
32-bit word at block offset *s* + 4 × (*t* >> 5) is set. The word is read with no bound: for *t* below 512 it is
`bits[t >> 5]`; for *t* of 512 or more it lies past the record (up to 8188 bytes past its start), and that word of
the block is tested as it is. With *s* 0 (no set) the word is read at linear address 4 × (*t* >> 5), outside the
block, where the reference build faults; no recording and no scenario has a unit type with no set.

Nothing happens (no imports) unless all of these hold: A is not none; `Q.active` is not 0; `Q.type` is 1 or 2;
`D.flags1` AND 0x10010000 is not 0; `Unit.buildLeft` equals 0.0 (either sign); and `Q.allied[` A's `Player.index` `]`
is 0. Then, with *u* as the running unit *U*:

1. **The attack order.** When *u*'s first order is 0 or has `flags_42` bit 0x20000 (`ORDER_FLAG_IDLE`), and
   `D.categories` does not contain `A.unitDefIndex`, and `D.weaponCategories0` does not contain it:
   `weapons.WeaponCanReachUnit(u, A, 0)`; when it is not 0, the attack order (A, 0). Its answer is *ordered* (0 when
   it was not asked).
2. **Free weapons.** When *ordered* is 0 and `Unit.flags` AND 0x300000 (`fireOrder`) is not 0, for each weapon *i* = 0, 1, 2 in turn:
   when `Unit.weapon{i}Flags` has bits 0x2 and 0x10: `weapons.WeaponCanReachUnit(u, A, i)`; when that is not 0 and the
   `WeaponDef` at `Unit.weapon{i}Def` lacks `flags` bit 0x4000000 (`commandFire`): `targeting.GetWeaponTargetUnit(u,
   i)` (recorded as `services.ScheduleTargetingTargetUnit`) gives W; when W is 0, or `weapons.WeaponCanReachUnit(u, W,
   i)` (asked only when W is not 0) is 0, or `D.weaponCategories{i}` contains `W.unitDefIndex` (looked at only when that
   answer was not 0): SET_UNIT (*i*, A).

### `ExpandGroupCommand(command, size)`

| When | Result |
| --- | --- |
| `size` below 112 | 0, nothing published |
| `unit` (the header's, the first unit of the range) is not a valid unit (as `Apply`'s limits say) | 0 |
| `kind` 297 (GROUP_COMMAND), with `target` 0 or a valid unit, and, when `mode` is 0, `type` valid | the group order (below) |
| `kind` 298 (GROUP_BUILD) | the group build (below) |
| any other case | 0 |

The recordings hold no command outside these limits.

**The range.** Let F be the unit at `unit`. The range has *n* units, *n* = (`Game.unitsEnd` − F's offset) / 280 (signed,
truncated toward 0) + 1, at most 512. Unit *i* (0 ≤ *i* < *n*) of the range is the unit *i* records after F; it is
*in the group* when bit (*i* AND 31) of `units[i >> 5]` is set. Units are visited in increasing *i*.

Every COMMAND published is an `OrdersAdd` of 44 bytes (recorded as `services.SchedPublish 3 44`): `kind` 295, `unit`
the unit, `type` the kind, `queue` the command's `queue`, `target` as stated, `hasPos` 1 and `pos` the position when
there is one, `hasPos` 0 and `pos` (0, 0, 0) when there is none, `params` the command's `params[0]`, `params[1]`. The
result is the count of COMMANDs published.

**The group order (GROUP_COMMAND).**

1. *Takes a target*: when `mode` is not 0, true unless the low byte of `mode` is 5, 10 or 14; when `mode` is 0, true
   when the table entry of the kind byte of `type` has `flags` bit 0x200.
2. X, the excluded unit: the unit at `target` (0 for none) when it takes a target, else 0. X is also the `target` of
   every COMMAND, and the target passed to `GetOrderType`.
3. The members: the units in the group that are not X. *count* = the number of members; *sx*, *sz* = the sums over the
   members of `Unit.posX` >> 16 and `Unit.posZ` >> 16 (arithmetic shifts), as signed 32-bit sums. When *count* is 0,
   the result is 0 and nothing more happens.
4. The centre: *ax* = (*sx* / *count*) × 65536 and *az* = (*sz* / *count*) × 65536 (the division truncates toward 0;
   the product wraps at 32 bits). *range* = *count* × 3000.
5. C, the clicked position: (`pos[0]`, `pos[1]`, `pos[2]`). The command's position is C when `hasPos` is not 0, and
   none otherwise.
6. For each member *u*, in increasing *i*:
   - its kind K: the kind byte of `type` when `mode` is 0; otherwise the low byte of `GetOrderType(mode, u, X, C)`,
     with C passed even when `hasPos` is 0 (Q6), so any imports `GetOrderType` makes happen here, member by member;
   - skipped (nothing published) when K is 0; when K is `STANDING_FIREORDER` (`Standing_FireOrder`, 43) and *u*'s
     `UnitDef.flags2Byte` lacks bit 0x2; when K is `STANDING_MOVEORDER` (`Standing_MoveOrder`, 44) and
     `UnitDef.flags2Byte` lacks bit 0x1;
   - its position: none when `hasPos` is 0. When `hasPos` is not 0 and K's table entry has `flags` bit 0x2: let *dx* =
     `Unit.posX` − *ax* and *dz* = `Unit.posZ` − *az* (signed 32-bit, wrapping); *e* = ((*dx* × *dx*) >> 32) + ((*dz* ×
     *dz*) >> 32), each product exact in signed 64 bits and shifted arithmetically, each kept as its low 32 bits, the
     sum signed 32-bit; when *e* ≤ *range* (signed), the position is (C.x + `Unit.posX` − *ax*, C.y, C.z + `Unit.posZ` −
     *az*) (wrapping); otherwise C. When K's entry lacks bit 0x2: C;
   - a COMMAND for *u* with type K, target X and that position.

**The group build (GROUP_BUILD).** `mode`, `type` and `target` are not read. The position is (`pos[0]`, `pos[1]`,
`pos[2]`) when `hasPos` is not 0, none otherwise. For each unit in the group, in increasing *i* (no unit is excluded),
with *D* its `UnitDef`:

| When | Published |
| --- | --- |
| `D.flags1` lacks 0x40 (`canbuild`) | nothing |
| `D.flags1` has 0x40 (`canbuild`), lacks 0x800 | COMMAND: type `MOBILEBUILD` (25), target 0, the position |
| `D.flags1` has 0x40 (`canbuild`) and 0x800 | COMMAND: type `VTOL_MOBILEBUILD` (`VTOL_MobileBuild`, 54), target 0, the position |

(`flying` in `layout.json` is the bit 0x800 of `flags1`.)

### `GetOrderType`

The arguments are (`mode`, `unit`, `target`, `pos`).

`mode` is taken as its low byte. *u* is `unit`, T is `target` (0 for none), P is `pos` (0 for none; when not 0, a
`Vec3`). *D* is *u*'s `UnitDef`, and the answer is an index in the order type table, 0 for none; an answer given as a
name is that name's index. The same rule runs inside the module wherever section 4 names `GetOrderType(...)`, with the
same imports at that point. Hooks: AllowCommand names this slot (section 2).

**Picking.** "Pick(*a*, *b*)" is *a* when `D.flags1` has 0x800 (`flying`) and *b* otherwise.

**Relation.** When T is not 0: when `T.flags` lacks unit flag 0x10000000 (`alive`) the answer is 0 at once (no imports).
Otherwise T is *friendly* when *u*'s `Player.allied[` T's `Player.index` `]` is not 0, and *enemy* when it is 0. With T
0, neither holds.

**Feature at P.** Defined only where it is used:

- *visible(P)*: let *x* = whole(P.x) >> 5 and *y* = (whole(P.z) − (whole(P.y) >> 1)) >> 5, in 32-bit arithmetic with
  arithmetic shifts. P is visible when *x* (as unsigned) is below *u*'s `Player.exploredWidth`, *y* (as unsigned) is
  below `Player.exploredHeight`, and the 16-bit word `Game.visibilityMask[x + y × exploredWidth]` has bit (1 <<
  `Game.playerIndex`) (Q13). No import.
- *the feature at P*: `los_radar.GetMapCellAtPosition(P)` gives a `Cell` (0: no feature). Let *f* = `Cell.feature`
  (16-bit). *f* below 0xfffb: no feature when *f* ≥ `Game.featureCount`, else the `Feature` record *f* of
  `Game.features` (256 bytes each). *f* = 0xfffe: the owner cell is the cell (`Cell.offsetY` × `Game.width` +
  `Cell.offsetX`) records before this one (13 bytes each); with *g* its `feature`, no feature when *g* ≥ 0xfffb, else
  `Feature` record *g* (no count check). Any other *f*: no feature. A recorded answer with no `r` lines of its
  fields (`= Cell#4072` in `ui-right-click.txt`) is a cell the replayer makes zeroed (`docs/replay-format.md`, "An
  object returned as its offset"), so its `feature` is 0.
- *marked*: there is a feature and its `Feature.flags` byte has bit 0x80.
- *the feature check (bit m)*: `D.flags2` has *m*, P is not 0, P is visible, and the feature at P is marked, tested in
  that order; the import is made only when the first three hold.

Each mode, its tests in order; the first that answers wins, and reaching the end answers 0. Imports are made only where
named, and only when the tests before them did not answer.

| Mode | Rule |
| --- | --- |
| 1 | when `Game.interfaceType` is 1: (a) `D.flags2` has 0x10 and enemy: the mode-3 answer (same arguments); (b) `D.flags2` has 0x400 and enemy: Pick(`VTOL_RECLAIMUNIT` 59, `RECLAIMUNIT` 33); (c) friendly and `units.CanRepair(u, T)` not 0 and `T.buildLeft` ≠ 0.0: Pick(`VTOL_HELPBUILD` 51, `HELPBUILD` 23); (d) friendly and `units.CanRepair(u, T)` not 0 (asked again): Pick(`VTOL_REPAIRUNIT` 61, `REPAIRUNIT` 35); (e) `D.flags1` has 0x800 (`flying`), friendly, and T's `UnitDef.flags1` has 0x200: `VTOL_LANDING` (53); (f) T not 0 and `units.CanLoad(u, T)` not 0: Pick(`VTOL_PICKUP` 57, `GROUND_PICKUP` 20); (g) `D.flags2` has 0x20 and friendly: Pick(`VTOL_FOLLOW` 49, `FOLLOW_GROUND` 18); (h) the feature check (0x800): `RESURRECT` (37); (i) the feature check (0x400): Pick(`VTOL_RECLAIM` 58, `RECLAIM` 32); (j) `D.flags2` lacks 0x80 or `Unit.motion` is 0: 0; (k) Pick(`VTOL_MOVE` 55, `MOVE_GROUND` 26). Otherwise (`interfaceType` not 1): (a) `D.flags2` has 0x10 and enemy: the mode-3 answer; (b) `D.flags2` has 0x400 and enemy: the mode-12 answer; (c) T not 0 and `units.CanRepair(u, T)` not 0 and `T.buildLeft` ≠ 0.0: the mode-8 answer (which asks `CanRepair` again); (d) T not 0, `T.playerIndex` equals `Game.localPlayer`, `T.flags` has 0x20 (`selectable`), `T.buildLeft` = 0.0, `T.postTransferHoldoff` is 0, and either `T.carrier` is 0 or the carrier's `Unit.flags` has 0x40000000 (`cargoSelectable`): 0; (e) the feature check (0x800): `RESURRECT`; (f) the feature check (0x400): Pick(`VTOL_RECLAIM`, `RECLAIM`); (g) `D.flags2` lacks 0x80 or `Unit.motion` is 0: 0; (h) Pick(`VTOL_MOVE`, `MOVE_GROUND`) |
| 2 | `D.flags2` lacks 0x80: 0; `Unit.motion` 0: `QMOVE` (30); T 0: Pick(`VTOL_MOVE`, `MOVE_GROUND`); `D.flags2` has 0x1000 (`capture`) and enemy: `CAPTURE` (14); `D.flags2` has 0x400 and enemy: Pick(`VTOL_RECLAIMUNIT`, `RECLAIMUNIT`); friendly and `units.CanRepair(u, T)` not 0 and `T.buildLeft` ≠ 0.0: Pick(`VTOL_HELPBUILD`, `HELPBUILD`); friendly and `units.CanRepair(u, T)` not 0 (asked again) and T's health below T's `UnitDef.maxHealth`: Pick(`VTOL_REPAIRUNIT`, `REPAIRUNIT`); `D.flags1` has 0x800 (`flying`), friendly, and T's `UnitDef.flags1` has 0x200: `VTOL_LANDING`; `units.CanLoad(u, T)` not 0: Pick(`VTOL_PICKUP`, `GROUND_PICKUP`); `D.flags2` has 0x20 and friendly: Pick(`VTOL_FOLLOW`, `FOLLOW_GROUND`); else Pick(`VTOL_MOVE`, `MOVE_GROUND`) |
| 3 | the mode-3 rule (below) |
| 4 | `D.flags2` has 0x4000: `ATTACKSPECIAL` (9); else 0 |
| 5 | `D.flags2` has 0x100, `D.flags1` has 0x800 (`flying`), T not 0, and T's `UnitDef.flags1` has 0x200: `VTOL_LANDING`; else `D.flags2` has 0x100: Pick(`VTOL_UNLOAD` 65, `GROUND_UNLOAD` 21); else 0 |
| 6 | T 0: 0; `units.CanLoad(u, T)` 0: 0; else Pick(`VTOL_PICKUP`, `GROUND_PICKUP`) |
| 7 | `D.flags2` has 0x20 and friendly: Pick(`VTOL_FOLLOW`, `FOLLOW_GROUND`); else 0 |
| 8 | `units.CanRepair(u, T)` (asked even when T is 0) is 0: 0; `T.buildLeft` ≠ 0.0: Pick(`VTOL_HELPBUILD`, `HELPBUILD`); else Pick(`VTOL_REPAIRUNIT`, `REPAIRUNIT`) |
| 9 | `D.flags2` lacks 0x40: 0; `Unit.motion` 0: `QPATROL` (31); `D.flags2` has 0x200: Pick(`VTOL_REPAIRPATROL` 60, `REPAIRPATROL` 34); else Pick(`VTOL_PATROL` 56, `PATROL` 29) |
| 10 | `STOP` (45) |
| 11 | `TELEPORT` (47) |
| 12 | `D.flags2` lacks 0x400: 0. Otherwise the feature at P is looked up first (`los_radar.GetMapCellAtPosition(P)` is asked even when P is 0, with the address 0, Q7). Then: P not 0, `D.flags2` has 0x800, P visible and the feature marked: `RESURRECT`; P not 0, P visible and the feature marked: Pick(`VTOL_RECLAIM`, `RECLAIM`); T 0: 0; else Pick(`VTOL_RECLAIMUNIT`, `RECLAIMUNIT`) |
| 13 | `D.flags2` has 0x1000 (`capture`), T not 0, and `T.player` is not *u*'s `Unit.player` (offsets compared): `CAPTURE`; else 0 |
| 14 | `UnitDef.ids` is 0 or `Unit.motion` is 0: 0; else Pick(`VTOL_MOBILEBUILD` 54, `MOBILEBUILD` 25) |
| 0, 15 to 255 | 0 |

Friendly and enemy both need T not 0, so a test "friendly and `CanRepair`" makes no import when T is 0 or an enemy.
Modes 1 and 2 can ask `units.CanRepair` twice, and mode 8 asks it even with no target (Q12).
`T.buildLeft` is an f32 compared with 0.0 (−0.0 equals 0.0).

**The mode-3 rule.** W0 is the `WeaponDef` at `Unit.weapon0Def`, W1 the one at `Unit.weapon1Def`, and Dw the
`WeaponDef` at `D.weapons0`. "Below water" means T's `UnitDef.modelMaxYHigh` + whole(`T.posY`) (signed 16-bit values,
summed in 32 bits) is below `Game.seaLevel` (an unsigned byte). No import.

The answer is the first row that holds, from the top. The rows marked *armed* hold only when `Unit.flags` has
0x80000000 (`armed`); *air* is Dw's `flags` bit 0x100 and *tv* is T's `UnitDef.flags1` bit 0x800 (`flying`).

| Armed | Condition | Answer |
| --- | --- | --- |
| | `D.flags2` lacks 0x10 | 0 |
| *armed* | T is not an enemy (T 0 or friendly) and W0's `flags` has 0x20000 (`toAirWeapon`) | 0 |
| *armed* | T is not an enemy, `D.flags1` lacks 0x800 (`flying`) | `SUPPRESS` (46) |
| *armed* | T is not an enemy, Dw's `flags` has 0x100 (`dropped`) | `AIRSTRIKE` (2) |
| *armed* | T is not an enemy | `AIRTOGROUND` (4) |
| *armed* | T is an enemy, `T.flags` AND 3 is not 2, W0's `flags` has 0x20000 | 0 |
| *armed* | below water, W0's `flags` lacks 0x10000 (`waterWeapon`), and (`Unit.weapon1Flags` lacks 0x2 or W1's `flags` lacks 0x10000) | 0 |
| *armed* | not below water, `D.flags1` has 0x1000, W0's `flags` has 0x10000 | 0 |
| *armed* | not below water, `D.flags1` has 0x1000, `Unit.weapon1Flags` has 0x2 and W1's `flags` has 0x10000 | 0 |
| *armed* | `D.flags1` has 0x800 (`flying`), *air* and not *tv* | `AIRSTRIKE` |
| *armed* | `D.flags1` has 0x800 (`flying`), not *air* and *tv* | `AIRTOAIR` (3) |
| *armed* | `D.flags1` has 0x800 (`flying`), not *tv*, `D.flags1` lacks 0x8000000 | `AIRTOGROUND` |
| *armed* | `D.flags1` has 0x800 (`flying`), not *tv*, `D.flags1` has 0x8000000 | `AIRTOGROUNDHOVER` (5) |
| *armed* | `D.flags1` has 0x800 (`flying`) (*air* and *tv*) | 0 |
| *armed* | `Unit.motion` not 0 | `ATTACK_CHASE` (6) |
| *armed* | `Unit.flags` has 0x20000000 (`immobile`) | `ATTACK_NOMOVE` (8) |
| | `D.flags1` has 0x10000000 | `ATTACK_KAMIKAZE` (7) |
| | otherwise | 0 |

**Failure answers.** `units.CanRepair` and `units.CanLoad` answer 0 for "cannot", which moves on to the next test.
`los_radar.GetMapCellAtPosition` answering 0 means no feature.

### `GetOrderFlags(unit)`

0 when `unit` is 0 or `Unit.list` is 0; otherwise the first order's `Order.flags_42`, as a 32-bit value. No import, no
write.

### `MissionTypeFromName(name)`

`name` is a zero-terminated string. Let the table be the *N* entries of `SimGlobals.g_missionOrderTableVec`, compared
by their `name` strings with the kit's case-insensitive comparison (`runtime/vectors/README.md`, `strcmpi`: each byte
`A` to `Z` compared as its lower case letter, every other byte as itself, unsigned, the shorter string's 0 sorting
first).

The answer is the index (AND 0xff) of the first entry, in table order, whose name does not compare below `name`, when that
entry exists and its name compares equal to `name`; otherwise 0. The table is sorted by that comparison. An empty table answers 0. Entry 0 of the recordings' table has the empty name, so "" answers 0 either way. No
import, no write. "Order type names" (above) is this rule.

### `RunOrders(unit)`

The input is the unit. Hooks: AllowBuild, AircraftSeekRepair, GuardResponse and IdleBuilderAction name this slot
(section 2).

The call is a series of *passes*. Each pass looks at O, the order at `Unit.list` at the start of that pass. When O is
not 0 and the tick, as an unsigned 32-bit number, is at least `O.wakeFrame`, the pass first wakes O: `O.wakeFrame` becomes
0xffffffff and `O.subFlags` gains 0x1. Then, with pending = (`O.subFlags` OR `Unit.netDirtyFlags`) AND `O.flags`:

| When | Imports, in order | Writes | Then |
|---|---|---|---|
| `Unit.list` is 0 | the default order (below) | its writes | the call ends |
| `O.flags` is not 0 and pending is 0 | none | the wake's, if any | the call ends; no default order |
| otherwise | CLEAR_NET_DIRTY (12): `unit` the unit, `mask` pending (also when pending is 0, Q8); when pending has 0x10000: CLEAR (0), CLEAR (1), CLEAR (2); the run of O with code pending; then the result's imports (below) | before the run: `O.subFlags` loses the pending bits and `O.flags` becomes 0; after it: the result's writes | as the result says |

The result applies to O whatever the run did to the lists meanwhile:

| Result | Imports and writes on O | Then |
|---|---|---|
| 0 | `state` becomes 0 | the next pass |
| 1 | `state` gains 1 (8-bit, wrapping) | the next pass |
| 2, 4 | nothing | the next pass |
| 3 | `services.RandomInt(15)` = *r*; `flags` gains 0x1 and `wakeFrame` becomes the tick + 30 + *r* | the next pass |
| 5, 8 | deleting the order | the next pass |
| 6 | moving O to the back | the next pass |
| 7 | deleting the unit's orders (all 1) | the call ends; no default order |
| 9, `O.next` not 0 | `flags_42` gains 0x800000; deleting the order | the next pass |
| 9, `O.next` 0 | `flags_42` gains 0x800000; `state` becomes 0; `services.RandomInt(30)` = *r*; `flags` gains 0x1 and `wakeFrame` becomes the tick + 30 + *r* | the next pass |
| any other | as 7 | the call ends; no default order |

So the next pass runs the order that is now first, at once, when its `flags` is 0 or one of its conditions is pending:
a run that ends with result 0, 1, 2 or 4 and leaves `flags` 0 runs again in the same call (its next `state` for result
1), and one that has set `flags` (a wake delay, or a condition to wait on) ends the call unless that condition is
already pending (Q9).

**The default order** (`Unit.list` 0 at a pass, whether it was empty or became empty):

| When | Imports and writes |
|---|---|
| `Player.active` of `Unit.player` is not 0, its `Player.type` is 1 or 2, and `UnitDef.defaultmissiontype` of `Unit.def` is not 0 | inserting a command of kind `defaultmissiontype` (its byte, unsigned); the new order is not run in this call |
| otherwise | none |

### `RunSecondaryOrders(unit)`

The input is the unit. The call walks `Unit.list2` from its first order. For each order O met:

- when `O.flags` is 0, or the tick (unsigned) is at least `O.wakeFrame`: `O.flags` becomes 0, the run of O with code
  0, and the result by the table below; after a result that does not end the call, the walk starts again at the first
  order of `Unit.list2`;
- otherwise the walk goes on with `O.next`.

| Result | Effect | Then |
|---|---|---|
| 0 | `state` becomes 0 | again from the first |
| 1 | `state` gains 1 | again from the first |
| 2, 4 | nothing | again from the first |
| 3 | `flags` gains 0x1 and `wakeFrame` becomes the tick + 30 + `services.RandomInt(15)` | again from the first |
| 5, 8, 9, any other | deleting the order | again from the first |
| 6, 7 | deleting the order | the call ends |

The call ends when the walk reaches the end. No CLEAR_NET_DIRTY is published, `wakeFrame` is not reset and `subFlags`
is not read or written (Q10).

### Order kinds

One subsection per entry of the order type table, in index order, except that two pairs of kinds with one behaviour
share a subsection (`QMove` and `QPatrol`, `SelfDestruct` and `SelfDestructFG`). Each gives the table entry's flags (the
bits that matter) and what the order's words mean for the kind; the fields its run reads beyond section 3's "Read"
table; then its outcomes, a table whose rows are tested from the top (the first that holds is the outcome) unless it
says otherwise. A test that asks an import makes it when its row is tested, whether or not the row holds, so the
outcomes of later rows come after that import. "Result r" is the run's result. "Recorded" names the calls of `tests/replay/orders/` in which an order of the
kind is the first order run, or says "unrecorded". The runs of `RunOrders` start with `flags` 0 (the frame clears it),
so "`flags` becomes X" and "`flags` gains X" differ only after an earlier write of the same run.

#### `Ready` (kind 0)

Table entry: flags 0 (`orderNotify[0]`). The run changes nothing, calls no import and has result 5 for any code.
Recorded: unrecorded (no recorded run of a kind 0 order).

#### `Activate` (kind 1)

Table entry: flags 0x10060. When `D.flags2` has bit 0x4:
SET_STATE_BITS (1, 1). Result 5. Recorded: unrecorded.

#### `AirStrike` (kind 2)

Table entry: flags 0x600 (target and position). `Order.pos` (here: the strike point, refreshed from the target),
`Order.start` and `Order.progress` (here: the leash).

Reads: `UnitDef.attackrunlength`, `altitude`, `maxHealth`; `UnitMotion.speed`; `Unit.heading`, `health`.

Before the state, in this order:

| When | Effects | Result |
| --- | --- | --- |
| *c* has any of 0x1000a | when `O.next` is 0 and `U.flags` has any of 0x300000 (`fireOrder`): a new order (`VTOL_SEEKATTACK` (`VTOL_SeekAttack`, 62), target T, position `O.pos`, 0, 0, 0) at the tail | 5 |
| T is 0 and `flags_42` has 0x200 | when `O.next` is 0: a new order (`VTOL_SEEKATTACK`, no target, position `U.pos`, 0, 0, 0) at the tail | 5 |
| T is not 0 | `O.pos` becomes T's `pos` (all three words); go on | |
| the leash test holds | none further | 5 |

Then, with *d* = ftol(hyp(`O.posX` − `U.posX`, `O.posZ` − `U.posZ`)) and *a* = Head(`U.pos`, `O.pos`):

| State | When | Imports and writes | Result |
| --- | --- | --- | --- |
| 0 | `U.motion` not 0 and `D.flags1` has 0x800 (`flying`) | announce (1); the climb preparation | 1 |
| 0 | otherwise | none | 7 |
| 1 | *d* < 0x1e00000 | RELEASE (3); CLAIM (0); the move goal (`U.pos` + Step(*a*, 0x8c00000), 0x3c0); `flags` gains 0xe2 | 1 |
| 1 | otherwise | RELEASE (3); CLAIM (0) | 1 |
| 2 | | `services.RandomInt(0x4000)` = *q*; the move goal (`U.pos` + Step(*q* + *a* − 0x2000, *d* / 2), 0x1e0) (*d* / 2 truncated toward zero); `flags` becomes 0x100e8 | 1 |
| 3 | | none | 1 |
| 4 | *c* has any of 0xe0 | none | 1 |
| 4 | otherwise | `mission.GetMissionParams(p)`, *p* a `MissionParams`; *g* = its `gravity`. *g* 0: nothing more, result 7. Else *t* = ftol(sqrt((2 × `D.altitude`) / *g*) × 30.0 × *m*) + 1 + `D.attackrunlength`, *m* = whole(`UnitMotion.speed`) of `U.motion`; then, when T is not 0, `movement.NewPathOrderToUnit(O, T)`, else a path target to `O.pos`, either giving *t'*; SET_APPROACH_RADIUS (*t'*, the low 16 bits of *t*, signed); attach target *t'*; a wake delay of 1; `flags` gains 0x100e8 | 2 (7 when *g* is 0) |
| 5 | | CLAIM (0); SET_POS (0); the move goal (`U.pos` + Step(*a*, (`D.attackrunlength` + 0x3c0) << 16), 0x3c0); `flags` becomes 0xe2 | 1 |
| 6 | *U* not damaged | CLEAR (0); the move goal (`U.pos` + Step(`U.heading`, 0x5a00000), 0x80); `flags` becomes 0xe2; `state` becomes 3 | 2 |
| 6 | *U* damaged | CLEAR (0); the move goal as above; `flags` becomes 0xe2; then the pad query; with at least one pad: release the target, `services.RandomInt(count)`, a new order (`VTOL_LANDING`, the pad drawn, no position, 0, 0, 0) at the front, `flags` becomes 0. No pad: nothing more (the goal stays attached, `flags` 0xe2) (Q17) | 0 |
| other | | none | 7 |

Failure answers: the path imports' answers are used as they come (0 attaches nothing). The pad query giving none: no
draw, result 0 in state 6. `GetMissionParams` gravity 0: result 7 with no move. Recorded: unrecorded.

#### `AirToAir` (kind 3)

Table entry: flags 0x200. `Order.ticks` (here: the pursuit counter); `Order.start`, `Order.progress` (here: the leash).

Reads: `UnitDef.maxvelocity` and `altitude` of *U*; T's `pos`, `heading`, `UnitDef.maxvelocity`, and its
`UnitMotion.velocityX`, `velocityY` and `velocityZ`.

Before the state, in this order: *c* has any of 0x10008: when `O.next` is 0 and `U.flags` has any of 0x300000 (`fireOrder`), a new
order (`VTOL_SEEKATTACK`, target T, position `O.pos`, 0, 0, 0) at the tail; result 5. T is 0 and `flags_42` has 0x200:
when `O.next` is 0, a new order (`VTOL_SEEKATTACK`, no target, position `U.pos`, 0, 0, 0) at the tail; result 5. The
off-map return (result 2). The leash test: result 5.

*Ahead*: with *u* = Step(Head(`U.pos`, T.pos), 0x140000) and *f* = Step(`U.heading`, 0x140000), the sum whole(*u*.x) ×
whole(*f*.x) + whole(*u*.z) × whole(*f*.z) (32-bit), kept as its low 16 bits signed, is above 0.

| State | When | Imports and writes | Result |
| --- | --- | --- | --- |
| 0 | `U.motion` not 0 and `D.flags1` has 0x800 (`flying`) | announce (1); the climb preparation; a wake delay of 1; `ticks` becomes 0 | 1 |
| 0 | otherwise | none | 7 |
| 1 | (always first) | RELEASE (3); CLAIM (0); SET_UNIT (0, T); then the rows below | |
| 1 | *c* has any of 0xe0 and ahead | `movement.NewAirManeuverOrder(O, U.pos + Step(U.heading, D.maxvelocity × 30), Step(U.heading, D.maxvelocity))` = *t*; SET_ALTITUDE (*t*, `D.altitude`); attach target *t*; `services.RandomInt(30)` = *r*; a wake delay of *r* + 60; `ticks` becomes 0 | 2 |
| 1 | *c* has none of 0xe0 and `ticks` < 0x5a (signed) | `ticks` becomes 0 when ahead, else `ticks` + 0x2d; with *D2* = ftol(hyp(`U.posX` − T.posX, `U.posZ` − T.posZ)): when whole(*D2*) > 0xa0: P = T.pos with x + 45 × `velocityX` and z + 45 × `velocityZ` of T's motion (wrapping); V = (T's `velocityX`, `velocityY`, `velocityZ`) + Step(T.heading, T's `maxvelocity` / 2) (the half truncated toward zero), each word wrapping; `movement.NewAirManeuverOrder(O, P, V)` = *t*; attach target *t*. Then a wake delay of 0x2d; `flags` gains 0x100e8 | 2 |
| 1 | otherwise | release the target; a new order (`VTOL_EVADE` (`VTOL_Evade`, 48), target T, no position, 0, 0, 0) at the front; `ticks` becomes 0; `flags` becomes 0 | 0 |
| other | | none | 7 |

Failure answers: a maneuver answer of 0 attaches nothing; state 1 reads T's motion without checking it. Recorded:
unrecorded.

#### `AirToGround` (kind 4)

Table entry: flags 0x200. `Order.start`, `Order.progress` (here: the leash). Let *w* be the `WeaponDef.range` at
`Unit.weapon0Def`, read at the start of every run.

Before the state, in this order: *c* has any of 0x1000a: when `O.next` is 0 and `U.flags` has any of 0x300000 (`fireOrder`), a new
order (`VTOL_SEEKATTACK`, target T, position `O.pos`, 0, 0, 0) at the tail; result 5. T is 0 and `flags_42` has 0x200:
when `O.next` is 0, a new order (`VTOL_SEEKATTACK`, no target, position `U.pos`, 0, 0, 0) at the tail; result 5. T not
0: `O.pos` becomes T's `pos`. When `U.spatialBucket` equals `Game.overflowBucket`: a wake delay of 30 and `state`
becomes 2 (the state below is then 2). The leash test: result 5.

With *d* and *a* as in `AirStrike`:

| State | Imports and writes | Result |
| --- | --- | --- |
| 0, `U.motion` not 0 and `D.flags1` has 0x800 (`flying`) | announce (1); the climb preparation | 1 |
| 0, otherwise | none | 7 |
| 1 | RELEASE (3); `services.RandomInt(0x4000)` = *q*; the move goal (`U.pos` + Step(*q* + *a* − 0x2000, *d* / 2), 0x80); `flags` becomes 0x100e8 | 1 |
| 2 | CLAIM (0); when T is not 0 SET_UNIT (0, T), else SET_POS (0); the move goal (`O.pos`, *w*); `flags` becomes 0x100e8 | 1 |
| 3 | a path target to `O.pos` + Step(*a*, *w* × 0x30000); `services.RandomInt(0x80)` = *r*; SET_APPROACH_RADIUS (*t*, *r* + 0x80); attach target *t*; `flags` becomes 0x100ea | 1 |
| 4 | the pad diversion (result 0 when it happens); else `services.RandomInt(2)` = *b*; *h* = `U.heading` + 0x4000 when *b* is not 0, else `U.heading` − 0x4000; the move goal (`U.pos` + Step(*h*, *w* << 16), 0x80); `flags` becomes 0x100ea | 1 |
| 5 | `state` becomes 2 | 2 |
| other | none | 7 |

Recorded: unrecorded.

#### `AirToGroundHover` (kind 5)

Table entry: flags 0x200. `Order.ticks` (here: the strafing side, 0 or 1), `Order.radius` (here: the miss count);
`Order.start`, `Order.progress` (here: the leash). *W* is the `WeaponDef.range` at `Unit.weapon0Def`.

Checked first, in this order:

| When | Effects | Result |
| --- | --- | --- |
| *c* has any of 0x10008 | when `O.next` is 0 and `U.flags` has any of 0x300000 (`fireOrder`): a new order (`VTOL_SEEKATTACK`, target T, position `O.pos`, 0, 0, 0) at the tail | 5 |
| T is 0 and `flags_42` has 0x200 | when `O.next` is 0: a new order (`VTOL_SEEKATTACK`, no target, position `U.pos`, 0, 0, 0) at the tail | 5 |
| the off-map return | as there | 2 |
| the leash test holds | none | 5 |

Then by state, with *Dt* = ftol(hyp(T.posX − `U.posX`, T.posZ − `U.posZ`)):

| When | Imports and writes | Result |
| --- | --- | --- |
| state 0, `U.motion` 0 or `D.flags1` lacks 0x800 (`flying`) | none | 7 |
| state 0 | announce (1); the climb preparation | 1 |
| state 1 | RELEASE (3); `services.RandomInt(0x4000)` = *r*; the move goal (`U.pos` + Step(*r* + Head(`U.pos`, T.pos) − 0x2000, *Dt* / 2), 0x80) (truncated toward zero); `flags` becomes 0x100e8 | 1 |
| state 2 | CLAIM (0); SET_UNIT (0, T); the move goal (T.pos, *W*); `flags` becomes 0x100e8; `ticks` and `radius` become 0 | 1 |
| state 3 | `weapons.WeaponCanReachUnit(U, T, 0)`; then the strafe (below) | 0 or 2 |
| any other state | none | 7 |

The strafe: when the reach answer is 0, `radius` + 1. Then:

- when `radius` ≥ 2 (signed): `radius` becomes 0; `services.RandomInt(0x10000)` = *r*; a path target to T.pos +
  Step(*r*, *W* << 16); SET_APPROACH_RADIUS (*t*, 0x80); the target is not attached (Q18); `flags` gains 0x110e8;
  result 2;
- otherwise: *a* = Head(`U.pos`, T.pos); when `ticks` is not 0 the angle is *a* + 0x2000 and `ticks` becomes 0, else
  *a* − 0x2000 and `ticks` becomes 1; `movement.NewPathOrderToUnitAt(O, T, P)` = *t* with P = T.pos − Step(angle, ((*W*
  × 2) / 3) << 16) (truncated toward zero); SET_APPROACH_RADIUS (*t*, 0x10); SET_ALTITUDE (*t*, `D.altitude`); attach
  target *t*; `flags` becomes 0x100e8; then the pad diversion: result 0 when it happens (`flags` 0 then), else result 2.

Recorded: unrecorded.

#### `Attack_Chase` (kind 6)

Table entry: flags 0x280 (target; order flag 0x80). `Order.ticks` (here: the weapon that aims, 0 to 2), `Order.radius`
(here: the chase step, 0 to 8), `Order.progress` (here: the leash, 0 for none) with `Order.start` its centre, and
`Order.pos` (here: the unit's position when the chase started).

Let *w* be `ticks` as the run starts (Q19). Before the table: the
result is 5, with nothing else done, when *c* has 0x800, when T is 0, when *c* has any of 0x10008, or when the leash
test holds. "The reach import" is `weapons.WeaponCanReachUnit(U, T, w)`.

| When | Imports | Writes | Result |
| --- | --- | --- | --- |
| state 0, `U.motion` 0, or `D.flags1` has 0x800 (`flying`), or `U.flags` lacks 0x80000000 (`armed`) | none | none | 7 |
| state 0, otherwise | announce (0); when *w* is 0, `targeting.ChooseWeapon(U)` | `O.pos` becomes `U.pos`; `radius` 0; when *w* is 0, `ticks` becomes the answer | 1 |
| state 1, *c* has any of 0x3000 | release the target | none | 1 |
| state 1, the reach import answers 0 | release the target; the reach import | none | 1 |
| state 1, otherwise | release the target; the reach import; CLAIM (0); CLAIM (2); SET_UNIT (*w*, T) | `flags` becomes 0x13808 | 2 |
| state 2 | `weapons.GetWeaponRange(U, w)` = R, then by the step (below) | below | below |
| state 3, *c* has any of 0x40e0 | none | `state` becomes 1 | 4 |
| state 3, the reach import answers not 0 | the reach import; CLAIM (0); CLAIM (2); SET_UNIT (*w*, T) | `flags` becomes 0x148e8; a wake delay of 30 | 2 |
| state 3, otherwise | the reach import; RELEASE (3) | `flags` becomes 0x100e8; a wake delay of 30 | 2 |
| any other state | none | none | 7 |

State 2, by the step *s* (`radius`); TP is T's position; divisions are signed, truncated toward zero:

| Step | Imports after `GetWeaponRange` | Writes | Result |
| --- | --- | --- | --- |
| 0 | the approach goal (TP, R) | `radius` becomes 1 | 1 |
| 1 to 4, \|`U.posY` − T.posY\| > 0x80000 | the approach goal (TP, R / 2) | `radius` becomes 6 | 1 |
| 1 to 4, otherwise | `services.RandomInt(0x8000)` = *r*; the approach goal (G, R / 4) | none | 1 |
| 5 | the approach goal (TP, R / 2) | `radius` becomes 6 | 1 |
| 6 | the approach goal (TP, 0) | `radius` becomes 7 | 1 |
| 7 | the ring goal (TP, R, R / 2) | `radius` becomes 8 | 1 |
| 8 | the ring goal (TP, R × 2, R) | `radius` becomes 0 | 1 |
| any other | none | none | 7 |

For steps 1 to 4 without the height difference: *a* = Head(TP, `U.pos`) + *r* − 0x4000 (signed 32-bit), *d* = R << 16
(wrapping), and G = (T.posX − S(*a*, *d*), T.posY, T.posZ − C(*a*, *d*)) (wrapping). The height difference is the
signed 32-bit difference, its absolute value compared signed (Q20).

Failure answers: `GetWeaponRange` answering 0 or less gives goals of that radius; `ChooseWeapon` answering 0 stores 0.
Recorded: `orders-move-attack.txt` tick 100 (`Unit#1`, `Order#3`: states 0, 1, 2 step 0 and 3 in one call);
`orders-guard-attacked.txt` tick 20 (`Unit#2`, `Order#7`: states 0 and 1, the reach answer 1).

#### `Attack_Kamikaze` (kind 7)

Table entry: flags 0x600. `Order.pos` (here: the point to reach, refreshed from the target each run).

Before the table: when *c* has any of 0x10008 the result is 5 and nothing else happens. Otherwise, when T is not 0,
`O.pos` becomes T's `posX`, `posY`, `posZ`.

| When | Imports | Writes | Result |
| --- | --- | --- | --- |
| state 0, `U.carrier` is not 0 | none | (the position only) | 7 |
| state 0, otherwise | announce (0); the approach goal (`O.pos`, *r*), *r* = `D.kamikazedistance` read as unsigned 16-bit, or 16 when that is below 16 | a wake delay of 60; `flags` gains 0xe0 | 1 |
| state 1, *c* has 0x20 | speech (6, 0); a new order (`SELFDESTRUCT` (`SelfDestruct`, 38), no target, no position, 1, 0, 0) at the front | none | 5 |
| state 1, *c* has 0x40 | none | none | 8 |
| state 1, otherwise | none | `state` becomes 0 | 2 |
| any other state | none | none | 7 |

A `SelfDestruct` order has `flags` 0x40040, so it goes to the front of the second list. Recorded: unrecorded.

#### `Attack_NoMove` (kind 8)

Table entry: flags 0x280 (target; order flag 0x80).

| When | Imports | Writes | Result |
| --- | --- | --- | --- |
| T is 0, or *c* has any of 0x10808 | none | none | 5 |
| state 0 | announce (0) | announce's | 1 |
| state 1 | CLAIM (0); SET_UNIT (0, T) | `flags` becomes 0x11808 | 1 |
| state 2 | RELEASE (3) | none | 9 |
| other | none | none | 7 |

Recorded: unrecorded.

#### `AttackSpecial` (kind 9)

Table entry: flags 0x680 (target, position; order flag 0x80). The run turns the order into an ordinary attack on its
target.

| When | Imports | Writes | Result |
| --- | --- | --- | --- |
| always | those of `GetOrderType(3, U, T, none)` (none), giving K | taking kind K's table flags (K as its low byte; a K of 0 is stored like any other, Q21); `ticks` becomes 2 | 2 |

The code and `state` are not read. Recorded: unrecorded.

#### `AttackUType` (kind 10)

Table entry: flags 0x4. `Order.ticks` (here: the unit type number hunted).

| When | Imports, in order | Writes | Result |
| --- | --- | --- | --- |
| state 0, `D.flags2` lacks 0x10 | none | none | 7 |
| state 0 | `services.RandomInt(90)` = *n* | a wake delay of *n* + 1 | 1 |
| state 1, some unit qualifies | one draw per qualifying unit (below); then `GetOrderType(3, U, best, none)` = K; then the new order's imports | a new order (K, target best, no position, 0, 0, 0) at the front | 0 |
| state 1, no unit qualifies | none | none | 5 |
| any other state | none | none | 7 |

The scan: every unit record from unit 1 to `Game.unitsEnd` inclusive, in array order (Q22). A unit V qualifies when
its `unitDefIndex` equals `ticks` and `U`'s player's `Player.allied[` V's `Player.index` `]` is 0. Its score: *dx* =
`V.posX` − `U.posX`, *dz* = `V.posZ` − `U.posZ` (signed 32-bit, wrapping); *d* = hi(*dx*²) + hi(*dz*²) (hi: the high
32 bits of the exact signed 64-bit square), summed signed 32-bit; then *d* − `services.RandomInt(d >> 1)` (the draw is
made even when *d* >> 1 is below 2). The best starts with score 0x7fffffff and no unit; V becomes the best when its score is at
most the best score (so the last of equal scores wins). A K of 0 still makes an order (Q21). The code is not read.
Recorded: unrecorded.

#### `BeCarried` (kind 11)

Table entry: flags 0x24.

| When | Imports | Writes | Result |
| --- | --- | --- | --- |
| `U.carrier` is 0 | none | none | 5 |
| state 0 | CLAIM (3) | none | 1 |
| state 1 | none | a wake delay of 10 | 2 |
| other | none | none | 7 |

Recorded: `units-transport.txt` ticks 4408 to 4416, `Unit#1` with `Order#1` (carrier `Unit#3`); only tick 4414 runs it.

#### `BuildingBuild` (kind 12)

Table entry: flags 0x10010c (build queue entry). `Order.ticks` (here: the unit type number to build, an entry of
`Game.defs`, whose `UnitDef` is *E*), `Order.radius` (here: how many more to build), `Order.pos` (here: the build
piece's position), T (here: the unit being built). A factory's build order.

| When | Imports in order | Writes | Result |
| --- | --- | --- | --- |
| *c* has 0x2 (the order is deleted) | when T = V is not 0: economy's INCOME 4 (24): `player` `U.playerIndex`, `unit` *U*, `resources` 2, `energy` 0.0, `metal` F; FINISH_CONSTRUCTION (V); DAMAGE (V, 30000, 9). Then always: SET_STATE_BITS (9, 0); the build menu update | none | 5 |
| *c* has 0x8 | speech (7, 6); the build menu update | `radius` − 1 | 0 |
| state 0, `U.flags` has 0x20000000 (`immobile`), `radius` ≤ 0 | `units.UnitRefSetUnit(link, 0)`; SET_STATE_BITS (1, 0) | T 0 | 5 |
| state 0, `U.flags` has 0x20000000 (`immobile`), `radius` > 0 | `units.UnitRefSetUnit(link, 0)`; SET_STATE_BITS (1, 1) | T 0 | 1 |
| state 0, `U.flags` lacks 0x20000000 (`immobile`) | `units.UnitRefSetUnit(link, 0)` | T 0 | 7 |
| state 1 | takes the build stance wait (2) | its writes | its answer |
| state 2, the footprint refused | `scripts.QueryScript(U.script, "QueryBuildInfo", &p, 0, 0, 0)` with *p* = −1 before the call; `scripts.GetPiecePosition(U, p, out)`; `map.CanPlaceUnitFootprint(E, 0, cell, U.flags AND 3)` (recorded as `services.ScheduleMapCanPlaceUnitFootprint`) answers 0 | `O.pos` becomes *out*; a wake delay of 15; `flags` gains 0x2 | 2 |
| state 2, placed, no unit made | as above, the footprint answer not 0; `units.CreateUnit(U.playerIndex, ticks, &O.pos, 0, 1, 0)` = N; `units.UnitRefSetUnit(link, N)`; then T is 0: speech (7, 36) | `O.pos` as above; a wake delay of 300; `flags` gains 0x2 | 2 |
| state 2, unit made | as above with T = V not 0 after the set: speech (9, 30); ATTACH_TO_PIECE (V, *U*, *p*, 1); SET_STANCE (V, 3, `U.moveOrder`, `U.fireOrder`, 0); adding an order to V (`GETBUILT` (`GetBuilt`, 19), queue 1, target *U*, no position, 0, 0); SET_STATE_BITS (8, 1); the build menu update | `O.pos` as above; the added order's | 1 |
| state 3, T = V is not 0 | `units.AddBuildProgress(U, V, B)`; when it answers not 0: the nano point of *U*, the nano spray (that point, the target box of V) | `V.buildLeft` (read after the import) ≠ 0.0: a wake delay of 1, `flags` gains 0xa, result 2; = 0.0: none, result 1 | 2 or 1 |
| state 3, T is 0 | none | none | 7 |
| state 4 | speech (8, 0); SET_STATE_BITS (8, 0); FINISH_CONSTRUCTION (T); `units.UnitRefSetUnit(link, 0)`; the build menu update | T 0; `radius` − 1 | 0 |
| other | none | none | 7 |

"link" is the address of `Order.link`; "T 0" is the import's write. *cell* is Cell(`O.pos` (the new value), (*E*.`footprintx`,
*E*.`footprintz`)). *p* is the piece `scripts.QueryScript` wrote (−1 when it wrote nothing). B = the float of
(`D.buildRate` / 30, integer division). F = ftol((1.0 − `V.buildLeft`) × V's `UnitDef.metalCost`), taken as unsigned
32-bit and converted to a float (Q23). `ticks` is passed to `units.CreateUnit` as its low 16 bits, unsigned.

Failure answers: `map.CanPlaceUnitFootprint` 0 retries in 15 ticks; `units.CreateUnit` 0 (seen as T 0 after the set)
retries in 300 ticks; `units.AddBuildProgress` 0 means no spray, and the result still follows `buildLeft`. Recorded:
`orders-air.txt` ticks 4090 to 4102, `Unit#2` with `Order#3` (target `Unit#6`), every run in state 3.

#### `BuildWeapon` (kind 13)

Table entry: flags 0xc0140 (second list: it runs from `RunSecondaryOrders` with code 0; state 2 adds the stockpile).
`Order.ticks` (here: the weapon slot *w*, 0 to 2), `Order.radius` (here: how many more to stockpile), `Order.progress`
(here: the reload ticks paid). The weapon is the `WeaponDef` at `Unit.weapon<w>Def`; the stockpile is the unsigned byte
`weapon<w>Stockpile`.

| When | Imports in order | Writes | Result |
| --- | --- | --- | --- |
| state 0, `radius` ≤ 0 | none | none | 5 |
| state 0, the stockpile ≥ 200 | none | a wake delay of 300 | 2 |
| state 0, otherwise | none | `progress` becomes 0 | 1 |
| state 1 | `economy.RequestGranted(U, 3)` (recorded as `services.ScheduleEconomyRequestGranted`) = *g*; economy's REQUEST 2 (24): `player` `U.playerIndex`, `unit` *U*, `resources` 3, `energy` En, `metal` Me (floats) | *g* not 0: `progress` becomes N; N ≥ `reloadTime` → result 1, else a wake delay of 5 → result 2. *g* 0: a wake delay of 10 → result 2 | 1 or 2 |
| state 2 | ADD_STOCKPILE (*w*); the build menu update | `radius` − 1 | 0 |
| other | none | none | 7 |

With P = `progress` and Tr = `reloadTime`: N = min(P + 5, Tr); Me = ftol(N × `metalPerShot` / Tr) − ftol(P ×
`metalPerShot` / Tr), En the same with `energyPerShot`, each converted to a float for the payload. The REQUEST is
published whatever *g* is (Q24). `economy.RequestGranted` is not an import of the entry today ("Gaps in the kit").
Failure answers: *g* 0 retries in 10 ticks with `progress` unchanged. Recorded: unrecorded.

#### `Capture` (kind 14)

Table entry: flags 0x200. The `link` holds the unit captured; `Order.radius` (here: the capture time, in ticks),
`Order.ticks` (here: the time spent so far).

Every run starts with `units.UnitRefGet(link)` = V. When V is 0 or *c* has any of 0x10008: speech (7, 4), result 8.

| When | Imports (after `UnitRefGet`) | Writes | Result |
| --- | --- | --- | --- |
| state 0, `U.motion` is 0 | none | none | 7 |
| state 0, `D.flags2` lacks 0x1000 (`capture`) | none | none | 7 |
| state 0, V's `UnitDef.flags2` has 0x1000 (`capture`) | speech (7, 32) | none | 8 |
| state 0, `V.buildLeft` is not 0.0 (either sign) | speech (7, 34) | none | 8 |
| state 0, otherwise | announce (5); CLAIM (3); the footprint marker (`V.cell`, `V.footprint`) | `radius` becomes the capture time (below); `flags` becomes 0x100e8 | 1 |
| state 1, *c* has 0x40 | none | none | 8 |
| state 1, `V.pos` out of build range (the build gap (`V.pos`, `V.footprint`)) | none | none | 0 |
| state 1, otherwise | start the building script (the build heading to `V.pos`) | its writes | 1 |
| state 2 | takes the build stance wait (0x10008) | its writes | its answer |
| state 3 | speech (11, 0) | none | 1 |
| state 4, `V.motion` is not 0 and `V.flags` has any of 0xc | stop the building script | a wake delay of 30 | 0 |
| state 4, `ticks` ≥ `radius` (signed) | none | none | 1 |
| state 4, otherwise | the nano point of *U*; the reverse spray (the target box of V, that point); BUSY (900) | `ticks` + 2; a wake delay of 2 | 2 |
| state 5 | players' GIVE_UNIT 192 (48): `unit` V, `player` the index of `U.player` among `Game.players` (0 to 9), the rest 0; speech (16, 0) | none | 5 |
| any other state | none | none | 7 |

The capture time, from V and its `UnitDef` E:

1. t0 = ftol(*e* / 2000 + *m* / 140 + 150), where *e* = E.`energyCost` × 30 and *m* = E.`metalCost` × 30, each rounded
   to a float, and the quotients and sums computed in double;
2. t1 = t0 when t0 < 1800, else 1800 (signed);
3. t2 = ((`V.health` sign-extended + E.`maxHealth`) × t1) / (E.`maxHealth` × 2), unsigned 32-bit, wrapping, the
   division unsigned;
4. t3 = ((`V.killCount` / 5 + 10) × t2 × 10) / 100, signed 32-bit, wrapping, divisions truncated toward zero. `radius`
   becomes t3.

`Capture` at state 1 out of build range goes back to state 0, which claims and places the marker again (Q25).
Recorded: unrecorded.

#### `Cloak_Off` (kind 15)

Table entry: flags 0x10060. When `D.flags2` has bit 0x2000: SET_FLAGS (0x800, 0). Result 5. Recorded: unrecorded.

#### `Cloak_On` (kind 16)

Table entry: flags 0x10060. When `D.flags2` has bit 0x2000: SET_FLAGS (0, 0x800). Result 5. Recorded: unrecorded.

#### `Deactivate` (kind 17)

Table entry: flags 0x10060. When `D.flags2` has bit 0x4: SET_STATE_BITS (1, 0). Result 5. Recorded: unrecorded.

#### `Follow_Ground` (kind 18)

Table entry: flags 0x200 (target). T (here: the guarded unit G); `Order.ticks` (here: the guard radius); `Order.pos`
(here: an offset from G, not a position).

Checked first, in every state: T 0: result 5; `U.carrier` not 0: result 7; G's `UnitDef.flags1` has 0x800 (`flying`): result 8.

State 0: announce (9); RELEASE (3); `ticks` becomes (*u* + *g* + 2) << 4 with *u*, *g* the first
words of `U.footprint` and `G.footprint`; `services.RandomInt(65536)` = *a*, taken as signed 16-bit; `O.pos` becomes
Step(*a*, `ticks` << 16). Result 1.

State 1, the first row that applies (A = `G.attacker`):

| When | Imports, in order | Writes | Result |
| --- | --- | --- | --- |
| A is not 0, A's player's `Player.allied[` *U*'s `Player.index` `]` is 0, *c* has 0x10, `D.categories` does not contain `A.unitDefIndex`, and the attack order (A, 1) answers 1 | the attack order's imports | its orders; `flags` becomes 0 | 3 |
| the same test on A, the attack order answering 0 | the attack order's imports; then, when `U.flags` has any of 0x300000 (`fireOrder`), the free weapons of ATTACKED (with A as the attacker, and no first `WeaponCanReachUnit(U, A, i)`: a weapon whose `weapon{i}Flags` has 0x2 and 0x10 and whose `WeaponDef.flags` lacks 0x4000000 goes straight to `targeting.GetWeaponTargetUnit`); then the rows below | as below | as below |
| G is not repaired, `D.flags1` has 0x40 (`canbuild`), and `GetOrderType(8, U, G, none)` = K is not 0 | its imports | release the target; a new order (K, target G, no position, 0, 0, 0) at the front; `flags` becomes 0 | 3 |
| the copy test (below), L's kind is neither `MobileBuild` (25) nor `BuildingBuild` (12), and L is actionable | the release of the target, then the new order's imports | release the target; a new order (L's `kind`, L's target, position L's `pos`, 0, 0, 0) at the front; `flags` becomes 0 | 3 |
| the copy test, L's kind is `MobileBuild` or `BuildingBuild`, and L's target is not 0 | the release of the target, then the new order's imports | release the target; a new order (`HELPBUILD` (`HelpBuild`, 23), L's target, position L's `pos`, 0, 0, 0) at the front; `flags` becomes 0 | 3 |
| otherwise | the approach goal (G's `pos` + `O.pos`, `ticks` / 2) | a wake delay of 30; `flags` gains 0x18 | 2 |

The copy test, with L = G's first order: L is not 0, L's `kind` is not 0, both `D.flags1` and G's `UnitDef.flags1` have
bit 0x40, L's `flags_42` has bit 0x100000, and L's target is not *U*. L is actionable when its `flags_42` has
`ORDER_FLAG_TARGET` and its target is not 0, or it has `ORDER_FLAG_POSITION`. The goal point is G's `pos` plus `O.pos`,
each word wrapping; `ticks` / 2 truncates toward zero.

Failure answers: `GetWeaponTargetUnit` 0 and `WeaponCanReachUnit` 0 lead to SET_UNIT as the free weapons say; a goal
answer of 0 attaches nothing. Recorded: `orders-guard-attacked.txt`, `Unit#3` `Order#3` tick 22 and `Unit#4` `Order#8`
tick 24 (state 0, then the last state 1 row, in the same call).

#### `GetBuilt` (kind 19)

Table entry: flags 0x224 (target). T (here: the builder). The order of a unit under construction. "Waits" means `flags`
gains bit 0x8000 (`Order.waiting`).

| When | Imports | Writes | Result |
| --- | --- | --- | --- |
| `U.buildLeft` = 0.0, `U.motion` = 0 | the orders dirty mark | none | 5 |
| `U.buildLeft` = 0.0, `U.motion` ≠ 0 | the orders dirty mark; then the finish rule (below) | its writes | 5 |
| state 0 | CLAIM (3) | a wake delay of 300; waits | 1 |
| state 1 | none | a wake delay of 30; waits | 1 |
| state 2, *c* has 0x8000 | none | a wake delay of 30; waits | 2 |
| state 2, *c* lacks 0x8000, has 0x1 | BUILD_DECAY (11) | a wake delay of 11; waits | 2 |
| state 2, neither | none | waits | 2 |
| other | none | none | 7 |

The finish rule, with B = T. First, when B is not 0, for each order *q* of B's first list in order (from `B.list`
along `next`; the orders added are *U*'s, so B's list does not change):

| *q*'s `kind` | Imports | Effect |
| --- | --- | --- |
| `QMove` (30) | `GetOrderType(2, U, none, q.pos)` (its imports) = K | when K is not 0: adding an order (K, queue 1, no target, position `q.pos`, 0, 0) to *U* |
| `QPatrol` (31) | `GetOrderType(9, U, none, q.pos)` (its imports) = K | the same |
| any other | none | none |

Then the stance, with *live* (V) = `V.flags` has unit flag 0x10000000 (`alive`) and lacks unit flag 0x4000
(`killPending`):

| When | Publishes, in order |
| --- | --- |
| B is not 0, *live* (*U*) and *live* (B), and `U.player`'s `active` is not 0 and its `type` is 1 | SET_STANCE (*U*, 3, `B.moveOrder`, `B.fireOrder`, 0); SET_STANCE (*U*, 4, 0, 0, `B.group`) |
| B is not 0, *live* (*U*) and *live* (B) | SET_STANCE (*U*, 3, `B.moveOrder`, `B.fireOrder`, 0) |
| otherwise | none |

Last:

| When | Effect |
| --- | --- |
| the walk over B's list added no order (B = 0 included) | adding an order (`PARK` (`Park`, 28), queue 1, no target, no position, 0, 0) to *U* |
| it added at least one | none |

Recorded: `orders-air.txt` ticks 4090 to 4102 (`Unit#6` with `Order#4` and `Unit#7` with `Order#2`, state 2, the code
with 0x8000); `orders-build-patrol.txt` ticks 148 to 162 (`Unit#2` with `Order#2`); `orders-factory.txt` ticks 19 to 22
(`Unit#2` with `Order#3`: tick 19 in state 0).

#### `Ground_Pickup` (kind 20)

Table entry: flags 0x200. T (here: the unit to load), `Order.ticks` (here: the pickup attempts). `docs/sim-events.md`,
Terms, "state 2 of the pickup order", describes the states.

Only the low byte of *c* is read (Q26). Before the table: when T is 0 or *c* has 8: speech (7, 35), result 8.

| When | Imports | Writes | Result |
| --- | --- | --- | --- |
| state 0, `U.motion` is 0, or `D.flags2` lacks 0x100 | none | none | 7 |
| state 0, `T.footprint.x` > `D.capacity` (signed 16-bit compare) | speech (7, 39) | none | 8 |
| state 0, otherwise | announce (18) | none | 1 |
| state 1 | takes the script busy wait (8) | its writes | its answer |
| state 2 | START_SCRIPT "TransportPickup" (1, 1, 1, `T.id`); speech (12, 0) | `ticks` + 1; a wake delay of 15 | 1 |
| state 3 | takes the script busy wait (8) | its writes | its answer |
| state 4, `T.carrier` is not 0 | none | none | 5 |
| state 4, `ticks` ≥ 3 | none | none | 9 |
| state 4, otherwise | the approach goal (T's `pos`, 0) | `flags` becomes 0xe8 | 1 |
| state 5 | release the target | none | 0 |
| any other state | none | none | 7 |

Recorded: unrecorded.

#### `Ground_Unload` (kind 21)

Table entry: flags 0x400. `Order.pos` (here: the drop point); the `link` is set to the cargo; `Order.ticks` (here: the
drop attempts).

Before the table: when *c* has 8: speech (7, 42), result 8.

| When | Imports | Writes | Result |
| --- | --- | --- | --- |
| state 0, `U.motion` is 0, or `D.flags2` lacks 0x100 | none | none | 7 |
| state 0, after `units.UnitRefSetUnit(link, U.cargo)` T (as the block now holds it) is 0 | that import | its writes | 5 |
| state 0, otherwise | that import; announce (41); START_SCRIPT "TransportDrop" (1, 1, 1, `T.id`, (`O.posX` AND 0xffff0000) + (`O.posZ` >> 16)) | `ticks` + 1; a wake delay of 15 | 1 |
| state 1 | takes the script busy wait (8) | its writes | its answer |
| state 2, T's `carrier` is not *U* (T is not checked: a T of 0 reads the record at offset 0) | speech (13, 0) | none | 5 |
| state 2, `ticks` ≥ 3 | none | none | 9 |
| state 2, `D.flags1` has 0x1000 | the approach goal (`O.pos`, ftol(`D.height180` × 1.5)) | `flags` becomes 0xe8 | 1 |
| state 2, otherwise | the approach goal (`O.pos`, 0) | `flags` becomes 0xe8 | 1 |
| state 3 | none | none | 0 |
| any other state | none | none | 7 |

`O.posZ` >> 16 is arithmetic; the sum wraps. Recorded: `units-transport.txt` tick 4415, `Unit#3` with `Order#3`
(state 1, the script busy wait, run twice in the call; section 7).

#### `Guard_NoMove` (kind 22)

Table entry: flags 0x20. `Order.pos` (here: the guarded spot), `Order.ticks` (here: the miss count), `Order.radius`
(here: the miss limit).

| When | Imports | Writes | Result |
| --- | --- | --- | --- |
| *c* has any of 0x10008 (any state) | none | `state` becomes 3 | 2 |
| state 0 | RELEASE (3) | a wake delay of 30 | 1 |
| state 1, the aim answer V is a unit with unit flag 0x10000000 | `targeting.GetWeaponTargetUnit(U, 0)` (recorded as `services.ScheduleTargetingTargetUnit`) = V; `units.UnitRefSetUnit(link, V)`; CLAIM (0); SET_UNIT (0, T); `services.RandomInt(3)` = *g* | T is V (the import's write); `O.pos` becomes V's `pos`; `ticks` 0; `radius` *g* + 3 | 1 |
| state 1, V is 0 or lacks that flag | `targeting.GetWeaponTargetUnit(U, 0)`; `units.UnitRefSetUnit(link, V)` | a wake delay of 30 | 2 |
| state 2 | *m* = 0 when *c* has 0x4000, else `ticks` + 1; when *m* ≤ `radius` (signed): `weapons.WeaponCanReachUnit(U, T, 0)`; when that is 0 or not asked: `services.RandomInt(100)` = *g* | `ticks` becomes *m*; reachable: `flags` gains 0x7008, result 2; else *g* < 80: `ticks` becomes 0, result 1; *g* ≥ 80: result 0 | 2, 1 or 0 |
| state 3, the enemy list (`O.pos`, 0x280) has *n* > 0 units | the enemy list's calls; `services.RandomInt(n)` = *g*; `units.UnitRefSetUnit(link, list[g])`; SET_UNIT (0, T) | T is list[*g*]; `state` becomes 1 | 2 |
| state 3, the list is empty | the enemy count's call | none | 0 |
| other | none | none | 7 |

Recorded: unrecorded.

#### `HelpBuild` (kind 23)

Table entry: flags 0x100208 (target, build kind). T (here: the unit under construction).

| When | Imports, in order | Writes | Result |
| --- | --- | --- | --- |
| *c* has 0x2 | the orders dirty mark | none | 5 |
| T is 0 | speech (7, 7) | none | 8 |
| state 0, `U.motion` 0 or `D.flags1` lacks 0x40 (`canbuild`) | none | none | 7 |
| state 0, otherwise | the ring goal (T's `pos`, `D.buildRange` + *r*, *r*) | `flags` becomes 0xe8 | 1 |
| state 1, *c* has 0x40 | speech (7, 10) | none | 8 |
| state 1, `T.buildLeft` is 0.0 | none | none | 5 |
| state 1, otherwise | CLAIM (3); start the building script (the build heading to T's `pos`); the orders dirty mark | their writes | 1 |
| state 2 | takes the build stance wait (10) | its writes | its answer |
| state 3 | the build step | its writes | 2 or 1 |
| state 4 | speech (8, 3) | `flags` gains 0x2 | 5 |
| any other state | none | none | 7 |

*r* = ftol(sqrt(*fx* × *fx* + *fz* + *fz*) × 16.0) / 2 (integer division), *fx*, *fz* T's `UnitDef.footprintx` and
`footprintz` as doubles, the square root correctly rounded (Q27). Failure answers: a goal answer of 0 attaches
nothing; `AddBuildProgress` 0 skips the particles only. Recorded: `orders-build-patrol.txt`, `Unit#1`, `Order#7`, ticks
150 (state 0, after `RepairPatrol` made the order in the same call), 151 (state 1), 152 and 153 (state 2, waiting), 154
(state 2 passes, state 3 runs), 155 to 162 (state 3).

#### `MakeSelectable` (kind 24)

Table entry: flags 0x4. Always SET_FLAGS (0x8000, 0x20); result 5. Recorded: unrecorded.

#### `MobileBuild` (kind 25)

Table entry: flags 0x100508 (position, build kind, build queue entry). `Order.ticks` (here: the unit type number to
build), `Order.pos` (here: the build site), `Order.progress` (here: the blocked tries), T (here: the unit being built).
*E* = entry `ticks` of `Game.defs`, *f* = (*E*.`footprintx`, *E*.`footprintz`), and the cell C = Cell(`O.pos`, *f*).

| When | Imports, in order | Writes | Result |
| --- | --- | --- | --- |
| *c* has 0x8 | speech (7, 7); the orders dirty mark | none | 8 |
| *c* has 0x2 | the orders dirty mark | none | 5 |
| state 0 | the footprint marker (C, *f*) | `O.pos` x and z become Centre(C, *f*) (C from the old position; y kept); `progress` 0; `flags` becomes 0xe0 | 1 |
| state 1, *c* has 0x40 and `O.pos` is out of build range (the build gap (`O.pos`, *f*)) | speech (7, 11) | none | 8 |
| state 1, `map.CanPlaceUnitFootprint(E, 0, C, 1)` answers 0, `progress` 0 | the placement import; speech (7, 43) | `progress` + 1; a wake delay of 30 | 2 |
| the same, `progress` not 0 and at most 10 (signed) | the placement import | `progress` + 1; a wake delay of 30 | 2 |
| the same, `progress` above 10 | the placement import; speech (7, 31) | none | 8 |
| state 1, placed, `units.CreateUnit` answers 0 | the placement import; CLAIM (3); `map.SnapWorldPosToFootprint(E, q)`; `units.CreateUnit(U.playerIndex, ticks, &O.pos, 0, 1, 0)`; `units.UnitRefSetUnit(link, 0)`; speech (7, 36) | `O.pos` becomes *q*; a wake delay of 300 | 2 |
| state 1, the new unit N is made | the same first four imports; `units.UnitRefSetUnit(link, N)`; speech (9, 30); the orders dirty mark; adding an order to N (`GETBUILT` (19), queue 1, target *U*, no position, 0, 0); start the building script (the build heading to N's `pos`) | `O.pos` becomes *q*; their writes | 1 |
| state 2 | takes the build stance wait (10) | its writes | its answer |
| state 3 | the build step | its writes | 2 or 1 |
| state 4 | speech (8, 3) | none | 5 |
| any other state | none | none | 7 |

The placement import is recorded as `services.ScheduleMapCanPlaceUnitFootprint`, with C by value. *q* is a copy of
`O.pos` that the snap import may rewrite; all three words are written back, and `O.pos` is *q* when `CreateUnit` reads
it. `ticks` is passed to `CreateUnit` as its low 16 bits, unsigned. The build gap is computed only when *c* has 0x40.

Failure answers: placement 0 is the waiting rows; `CreateUnit` 0 is the "answers 0" row (the CLAIM, the snap and the
position are not undone, and the state stays 1, Q28); `AddBuildProgress` 0 skips the particles only. Recorded:
`orders-factory.txt` `Unit#1` `Order#1`, ticks 15 to 18 (state 1, waiting), 19 (state 1, built), 20 to 22 (state 2);
`units-transport.txt` `Unit#251` `Order#5`, tick 4411 (state 0); `orders-build-patrol.txt` `Unit#1` `Order#1`, ticks
148 and 149 (state 3, progress answer 1); `orders-air.txt` `Unit#1` `Order#1`, ticks 4090 to 4102 (state 3, progress
answer 0).

#### `Move_Ground` (kind 26)

Table entry: flags 0x402 (position). `Order.pos` (here: the destination), `Order.ticks` (here: the extra radius).

| When | Imports | Writes | Result |
| --- | --- | --- | --- |
| state 0, `U.carrier` is not 0 | none | none | 7 |
| state 0, otherwise | announce (0); the approach goal (`O.pos`, `ticks` + 4) | `flags` becomes 0xe0 | 1 |
| state 1, *c* has 0x20 | speech (6, 0) | none | 5 |
| state 1, otherwise | none | none | 9 |
| any other state | none | none | 7 |

`ticks` + 4 wraps. Recorded: `ui-right-click.txt` tick 22 (`Unit#1`, `Order#4`, state 0).

#### `Paralyze` (kind 27)

Table entry: flags 0x24. `Order.ticks` (here: the ticks of paralysis still to start).

| When | Imports in order | Writes | Result |
| --- | --- | --- | --- |
| `ticks` = 0 | SET_STATE_BITS (0x10, 0) | none | 5 |
| `ticks` ≠ 0 | CLAIM (3); CLEAR (0), CLEAR (1), CLEAR (2); release the target; SET_STATE_BITS (0x10, 1) | a wake delay of *m*, *m* = min(`ticks`, 1800) (signed); `ticks` becomes 0 | 1 |

Recorded: unrecorded.

#### `Park` (kind 28)

Table entry: flags 0. The unit clears a space around itself.

| When | Imports | Writes | Result |
| --- | --- | --- | --- |
| state 0, `U.motion` is 0 | none | none | 7 |
| state 0, `D.flags1` has 0x800 (`flying`) | none | `O.pos` becomes `U.pos`; taking kind `VTOL_MOVE` (`VTOL_Move`, 55)'s table flags | 0 |
| state 0, otherwise | the footprint marker (P, E), below | `flags` becomes 0xe0 | 1 |
| state 1, *c* has 0x20 | none | none | 5 |
| state 1, `O.next` is not 0 | none | none | 5 |
| state 1, otherwise | none | a wake delay of 30 | 0 |
| any other state | none | none | 7 |

The marker: *n* = `U.footprint.x`, plus 3 when `D.minwaterdepth` ≥ 0 (signed); the unit's cell is (`U.posX` >> 20,
`U.posZ` >> 20), the words shifted as unsigned; P = (cell x − *n* × 4, cell z − *n* × 3) and E = (*n* × 8, *n* × 6), each
kept to its low 16 bits. In the flying row the order's `kind` becomes `VTOL_MOVE` (55) with that entry's table flags (the
row's "taking"); with result 0 its `state` is 0 and its `flags` 0, so the next pass of `RunOrders` runs it, in the same
call, as an order of that kind in state 0. Recorded: unrecorded.

#### `Patrol` (kind 29)

Table entry: flags 0x412 (position). `Order.pos` (here: the patrol point).

| When | Imports | Writes | Result |
| --- | --- | --- | --- |
| state 0, `U.motion` is 0 | none | none | 7 |
| state 0, otherwise | announce (0); the patrol return order | a wake delay of 1 | 1 |
| state 1 | RELEASE (3); the approach goal (`O.pos`, 0) | a wake delay of 15; `flags` gains 0xe0 | 1 |
| state 2, *c* has any of 0xe0 | none | `state` becomes 1 | 6 |
| state 2, `targeting.FindBestTargetIfFireAtWill(U)` (recorded as `services.ScheduleTargetingFireAtWill`) answers V ≠ 0, and the attack order (V, 0) answers 1 | the targeting import; the attack order's imports | `flags` becomes 0; `state` becomes 1 | 3 |
| state 2, otherwise | the targeting import (and the attack order's, when V ≠ 0); `services.RandomInt(30)` = *r* | a wake delay of *r* + 30; `state` becomes 1 | 4 |
| any other state | none | none | 7 |

Recorded: unrecorded.

#### `QMove` (kind 30) and `QPatrol` (kind 31)

Table entries: flags 0x400 (position) for both; both name `orderNotify[64]`. For any code and state: a wake delay of
60; result 6. No imports. Recorded: unrecorded.

#### `Reclaim` (kind 32)

Table entry: flags 0x100800 (build kind). `Order.pos` (here: a point on the feature), `Order.ticks` (here: the reclaim
time left).

Every run starts with the site cell and size (one `features.FindFeatureAtPos`), in every state. Then:

| When | Imports after the find, in order | Writes | Result |
| --- | --- | --- | --- |
| the find answers 0xffff | speech (7, 22) | none | 8 |
| `F.flags` lacks 0x80 | none | none | 8 |
| state 0, `U.motion` not 0 and `D.flags2` has 0x400 | the footprint marker (cell, size) | `flags` becomes 0xe0 | 1 |
| state 0, otherwise | none | none | 7 |
| state 1, *c* has 0x40 | none | none | 8 |
| state 1, otherwise | the site point (one draw, one `GetGroundHeight`); start the building script (the build heading to the site point) | `ticks` becomes ftol(15 + (`F.energy` + `F.metal`) × 0.5) | 1 |
| state 2 | takes the build stance wait (0) | its writes | its answer |
| state 3 or 4, `ticks` − 2 is 0 or less | state 3 only: speech (11, 0) | a wake delay of 2; `ticks` − 2 | 1 |
| state 3 or 4, `ticks` − 2 in 1 to 15 | state 3 only: speech (11, 0); then BUSY (300) | a wake delay of 2; `ticks` − 2 | 2 |
| state 3 or 4, `ticks` − 2 above 15 | state 3 only: speech (11, 0); then BUSY (300); the nano point of *U*; the feature box (one `GetGroundHeight`); the reverse spray (the feature box, that point), recorded twice | a wake delay of 2; `ticks` − 2 | 2 |
| state 5 | features' RECLAIM 35 (20): `unit` *U*, `pos` `O.pos` | none | 5 |
| any other state | none | none | 7 |

The time: `F.energy` + `F.metal` summed in double, times 0.5, plus 15, then ftol. Recorded: unrecorded.

#### `ReclaimUnit` (kind 33)

Table entry: flags 0x100200 (target, build kind). The `link` holds the unit reclaimed; `Order.ticks` (here: the
reclaim pulse, the damage of one hit), `Order.radius` (here: the ticks since the last hit).

Every run starts with `units.UnitRefGet(link)` = V. When V is 0 or *c* has any of 0x10008: result 5, nothing else.

| When | Imports (after `UnitRefGet`) | Writes | Result |
| --- | --- | --- | --- |
| state 0, `U.motion` not 0 and `D.flags2` has 0x400, `units.CanReclaim(U, V)` answers 0 | the `CanReclaim` import; speech (7, 33); speech (7, 22) | none | 8 |
| state 0, `U.motion` not 0 and `D.flags2` has 0x400, otherwise | the `CanReclaim` import; announce (21); CLAIM (3) | none | 1 |
| state 0, otherwise | speech (7, 22) | none | 7 |
| state 1, *c* has 0x20 | none | none | 1 |
| state 1, otherwise | the footprint marker (`V.cell`, `V.footprint`) | `flags` gains 0x100e8; a wake delay of 15; `ticks` becomes the reclaim pulse (*U*, V, 15); `radius` 0 | 2 |
| state 2, *c* has 0x40 | none | none | 9 |
| state 2, otherwise | start the building script (the build heading to `V.pos`) | its writes | 1 |
| state 3 | takes the build stance wait (0x10008) | its writes | its answer |
| state 4 | speech (11, 0) | none | 1 |
| state 5, in reach and `units.CanReclaim(U, V)` answers not 0 | the `CanReclaim` import; when `radius` ≥ 15: DAMAGE (V, `ticks`, 5); BUSY (900); the nano point of *U*; `units.UnitRefGet(link)` = V2; the reverse spray (the target box of V2, that point) | when the damage was published `radius` 0; then a wake delay of 2 and `radius` + 2 | 2 |
| state 5, otherwise | the `CanReclaim` import only when in reach; then stop the building script | a wake delay of 15 | 0 |
| any other state | none | none | 7 |

In reach: with *dx* = `U.posX` − `V.posX` and *dz* = `U.posZ` − `V.posZ` (wrapping), *q* = hi(*dx*²) + hi(*dz*²), signed
32-bit; L = `D.buildRange` + V's `UnitDef.radius184` (unsigned 16-bit plus signed 16-bit); in reach when *q* ≤ L × L
(signed 32-bit). The second `UnitRefGet` of state 5 comes after the DAMAGE, which weapons applies at once, and its
answer is used without a check (Q29). Recorded: unrecorded.

#### `RepairPatrol` (kind 34)

Table entry: flags 0x412 (position). `Order.pos` (here: the patrol point); T (here: an optional unit whose position
becomes the point). P is `U.player`.

| When | Imports, in order | Writes | Result |
| --- | --- | --- | --- |
| state 0 | the patrol return order | `O.pos` becomes T's `pos` when T is not 0 (before the return order) | 1 |
| state 1, *c* has any of 0xe0 | none | none | 6 |
| state 1 | the state 1 run (below) | below | 6, 3 or 2 |
| any other state | none | none | 7 |

The state 1 run. It always starts with the approach goal (`O.pos`, 16), a wake delay of 60 and `flags` gaining 0xe0.
Then, with *low* = `P.energy` < `P.energyCapacity` × 0.2 or `P.metal` < `P.metalCapacity` × 0.2 (in double):

- *the repair test*: when `P.energy` ≥ `P.energyCapacity` × 0.2 (in double): the repairable list (`D.range` << 16);
  when it is not empty, `services.RandomInt(n)` = *i* (*n* its length, drawn even when *n* is 1) and V = entry *i*;
  when `P.allied[` V's `Player.index` `]` is not 0 (Q30): `GetOrderType(8, U, V, none)` (its imports) = K. The test
  *finds* V when K is not 0; it makes the imports it reaches either way;
- *the reclaim rows*, given the pick's E, *e*, M, *m*, the first that holds gives the point X: (1) M not none and
  `P.metal` < `P.metalCapacity` × 0.2; (2) E not none and `P.energy` < `P.energyCapacity` × 0.2; (3) M not none and
  `P.metal` + *m* ≤ `P.metalCapacity` (the sum in double); (4) E not none and `P.energy` + *e* ≤ `P.energyCapacity`.
  X is M for rows 1 and 3, E for rows 2 and 4.

| When | Imports after the goal, in order | Writes | Result |
| --- | --- | --- | --- |
| the repair test finds V, and the repair order (V) answers 1 | the repair test's; the repair order's (it asks `GetOrderType(8, ...)` again) | its orders | 6 |
| the repair test finds V, and the repair order answers 0 | the same | none | 3 |
| it finds none, *low*, the reclaim pick (`U.pos`, `D.range` << 16) answers 1, and reclaim row 1 holds | the repair test's; the pick; release the target; a new order (`RECLAIM` (`Reclaim`, 32), no target, position M, 0, 0, 0) at the front; release the target again (it publishes nothing) | `flags` becomes 0 | 3 |
| the same, the pick answering 1 and row 2, 3 or 4 giving X | the repair test's; the pick; release the target; a new order (`RECLAIM`, no target, position X, 0, 0, 0) at the front | `flags` becomes 0 | 3 |
| otherwise | the repair test's; when *low*: the pick | none | 2 |

Failure answers: `map.UnitsInRange` 0 gives an empty list (no draw); a goal answer of 0 attaches nothing; the pick
answering 0, or none of its four rows holding, is result 2. Recorded: `orders-build-patrol.txt`, `Unit#1`, `Order#4`,
tick 150: state 0 then state 1 in the same call, the repair row with result 6.

#### `RepairUnit` (kind 35)

Table entry: flags 0x100200 (target, build kind). T (here: the unit repaired), `Order.progress` (here: the leash, 0 for
none) with `Order.start` its centre.

Before the table, in this order: T is 0: speech (7, 26), result 5; the leash test holds: result 5; `T.flags` AND 3 is
not 1: speech (7, 26), result 5.

| When | Imports | Writes | Result |
| --- | --- | --- | --- |
| state 0, `U.motion` is not 0, `D.flags1` has 0x40 (`canbuild`) and `T.buildLeft` is 0.0 (either sign) | announce (24) | none | 1 |
| state 0, otherwise | none | none | 7 |
| state 1, *c* has 0x40 | none | none | 8 |
| state 1, `T.pos` out of build range (the build gap (`T.pos`, `T.footprint`)) | the footprint marker (`T.cell`, `T.footprint`); `services.RandomInt(30)` = *r* | a wake delay of *r* + 30; `flags` gains 0xe8 | 2 |
| state 1, otherwise | CLAIM (3); start the building script (the build heading to `T.pos`) | its writes | 1 |
| state 2 | takes the build stance wait (8) | its writes | its answer |
| state 3, T is repaired | none | none | 1 |
| state 3, `T.flags` has any of 0xc | stop the building script | a wake delay of 15 | 0 |
| state 3, otherwise | BUSY (150); `units.AddRepairProgress(U, T, f)`, *f* the float of (`D.buildRate` / 30, integer division); when it answers not 0: the nano point of *U*, then the nano spray (that point, the target box of T) | a wake delay of 1; `flags` gains 0x8 | 2 |
| state 4 | speech (10, 40) | none | 5 |
| any other state | none | none | 7 |

Failure answers: `AddRepairProgress` 0 only leaves out the particles. Recorded: unrecorded.

#### `RepairUnitNoMove` (kind 36)

Table entry: flags 0x200 (target). The `link` holds the unit repaired.

Every run starts with `units.UnitRefGet(link)` = V. When V is 0: speech (7, 26), result 5.

| When | Imports (after `UnitRefGet`) | Writes | Result |
| --- | --- | --- | --- |
| state 0, `D.flags1` lacks 0x40 (`canbuild`) | none | none | 7 |
| state 0, `V.buildLeft` is 0.0 (either sign) and `U.activateFlags` has activation bit 0 | CLAIM (3) | none | 1 |
| state 0, otherwise | none | none | 8 |
| state 1, V is repaired | none | none | 1 |
| state 1, `V.flags` has any of 0xc | none | none | 1 |
| state 1, otherwise | BUSY (150); `units.AddRepairProgress(U, V, f)`, *f* as for `RepairUnit`; when it answers not 0: the nano point of *U*, `units.UnitRefGet(link)` = V2, the nano spray (that point, the target box of V2) | a wake delay of 1; `flags` gains 8 | 2 |
| state 2 | speech (10, 40) | none | 5 |
| any other state | none | none | 7 |

Recorded: unrecorded.

#### `Resurrect` (kind 37)

Table entry: flags 0x200 (target). `Order.pos` (here: a point on the wreck), `Order.ticks` (here: the unit type number
to bring back), `Order.radius` (here: the countdown), T (here: the new unit, set in state 5).

When `state` is 5 or less, the run starts with the site cell and size; its answer 0xffff is speech (7, 29) and result
8, and `F.flags` lacking bit 0x80 is result 8. States 6 and above make no find.

| When | Imports after the find, in order | Writes | Result |
| --- | --- | --- | --- |
| state 0, `U.motion` not 0 and `D.flags2` has 0x800 | the footprint marker (cell, size) | `flags` becomes 0xe0 | 1 |
| state 0, otherwise | none | none | 7 |
| state 1, *c* has 0x40 | none | none | 8 |
| state 1, otherwise | the site point; start the building script (the build heading to the site point) | its writes | 1 |
| state 2 | takes the build stance wait (0) | its writes | its answer |
| state 3, `units.FindUnitTypeId(name)` answers 0 | the type import; speech (7, 27) | `ticks` 0 | 8 |
| state 3, it answers a type number *t* | the type import; speech (11, 0) | `ticks` *t*; `radius` becomes ftol(entry *t* of `Game.defs`'s `buildtime` × 0.3 / *b*), *b* the double of (`D.buildRate` / 30, integer division) | 1 |
| state 4, `radius` not 0 | the nano point of *U*; the feature box (one `GetGroundHeight`); the nano spray (that point, the feature box); BUSY (300) | `radius` − 1; a wake delay of 1 | 2 |
| state 4, `radius` 0 | none | `radius` becomes −1 | 1 |
| state 5, `units.CreateUnit` answers 0 | `units.CreateUnit(U.playerIndex, ticks, &O.pos, 0, 1, 0)`; `units.UnitRefSetUnit(link, 0)`; speech (7, 36) | a wake delay of 300 | 2 |
| state 5, new unit N, `features.GetCellFeature(c)` 0xfffb or more | `CreateUnit`; `units.UnitRefSetUnit(link, N)`; `los_radar.GetOriginCellAtPosition(&O.pos)` = *c*; `features.GetCellFeature(c)` | none | 8 |
| state 5, N made, the cell's feature below 0xfffb | as above; then orders' SET_ANGLES 316 (20): `unit` N, `bank`, `heading`, `pitch` the three signed 16-bit words of the `FeatureSpot.rot` of entry `c.spot` of `Game.spots`; `los_radar.GetMapCellAtPosition(&O.pos)` = *mc*; features' REMOVE 33 (20): `x` = *k* mod `Game.width`, `z` = *k* div `Game.width`, `force` 0, *k* the index of *mc* in `Game.cells`; `mission.GetGameType()`; when it answers 3 (game type 3), `network.SendFeatureReclaimed(U, x, z)` with *x*, *z* the column and row of *c*'s index in `Game.cells` (each kept as 16 bits); orders' SET_BUILD_STATE 317 (16): `unit` N, `buildLeft` 0.0, `health` 1; the orders dirty mark | none | 1 |
| state 6 | speech (8, 28); `GetOrderType(8, U, T, none)` = K | when K is not 0, a new order (K, target T, no position, 0, 0, 0) at the front | 5 |
| any other state | none | none | 7 |

The name: `F.name` up to its first zero byte or 63 bytes, whichever comes first, then cut at its first `_`. A cell's
index is its offset from `Game.cells` divided by 13; "mod" and "div" truncate. `ticks` is passed to `CreateUnit` as its
low 16 bits, unsigned. With `D.buildRate` below 30 the divisor is 0.0, the quotient an infinity or a NaN, and ftol gives 0: `radius` is 0. The SET_ANGLES uses
the spot of the cell the import answers without testing it, and a cell feature of 0xfffb or more ends the order with N
already made (Q32). Failure answers: `CreateUnit` 0 keeps the state at 5 (the next run retries); `GetGameType` is only
compared with 3. Recorded: unrecorded.

#### `SelfDestruct` (kind 38) and `SelfDestructFG` (kind 39)

Table entries: `SelfDestruct` flags 0x40040 (second list, so it runs from `RunSecondaryOrders` with code 0);
`SelfDestructFG` flags 0 (first list, so it runs from `RunOrders`). Both name `orderNotify[54]`, and both run as below. `Order.radius` (here: the countdown word: bits 28 to 31 all set once it is armed, the count in bits
0 to 27), `Order.ticks` (here: the done mark). *n* = `D.selfdestructcountdown` (3 bits, 0 to 7).

First, always: when `radius` AND 0xf0000000 is 0, `radius` becomes *n* OR 0xf0000000. Then, with *k* = `radius` AND
0x0fffffff read after that:

| When | Imports in order | Writes | Result |
| --- | --- | --- | --- |
| `ticks` ≠ 0, or *n* = 0 | DAMAGE (*U*, 30000, 3) | none | 5 |
| `ticks` = 0, *n* > 0, *c* has 0x2, `U.flags` lacks unit flag 0x4000 (`killPending`) | speech (23, 0) | none | 5 |
| `ticks` = 0, *n* > 0, *c* has 0x2, `U.flags` has 0x4000 (`killPending`) | none | none | 5 |
| `ticks` = 0, *n* > 0, *c* lacks 0x2, *k* > 0 | speech (L(*k*), 0) | `radius` becomes (*k* − 1) OR 0xf0000000; a wake delay of 30; `flags` gains 0x2 | 1 |
| `ticks` = 0, *n* > 0, *c* lacks 0x2, *k* = 0 | speech (L(0), 0); `services.RandomInt(15)` = *g* | `ticks` becomes 1; a wake delay of *g*; `flags` gains 0x2 | 1 |

L(*k*) is 22 − *k* for *k* from 0 to 5. For *k* 6 and 7 the slot is not defined ("Gaps in the kit"). Recorded: unrecorded.

#### `SelfRepair` (kind 40)

Table entry: flags 0x1000204 (target). T (here: the unit that repairs *U*, a repair pad).

| When | Imports in order | Writes | Result |
| --- | --- | --- | --- |
| T = 0 | speech (7, 23) | none | 8 |
| state 0, T's `UnitDef.flags1` lacks 0x40 (`canbuild`) | none | none | 7 |
| state 0, `T.buildLeft` = 0.0 and `U.activateFlags` has activation bit 0 | CLAIM (3) | none | 1 |
| state 0, otherwise | none | none | 8 |
| state 1, *U* is repaired | none | none | 1 |
| state 1, otherwise | BUSY (150); `units.AddRepairProgress(T, U, R)`; when it answers not 0: the nano point of T, then the nano spray (that point, the target box of *U*) | a wake delay of 1; `flags` gains 0x8 | 2 |
| state 2 | speech (10, 40) | none | 5 |
| other | none | none | 7 |

R = the float of (T's `UnitDef.buildRate` / 30, integer division). A negative health counts as repaired ("health
below"). Failure answers: `AddRepairProgress` 0 means no spray; the wake delay and result are the same. Recorded:
`orders-air-repair.txt`, ticks 848 to 852, `Unit#4` with `Order#2` (target `Unit#2`): 848 to 851 in state 1, 852 ends the
order (state 1 with *U* repaired: result 1, then state 2 in the same call).

#### `Standby` (kind 41)

Table entry: flags 0x20000 (`ORDER_FLAG_IDLE`). The unit's idle order: it watches for a target.

| When | Imports | Writes | Result |
| --- | --- | --- | --- |
| state 0, `U.motion` is 0 | none | none | 7 |
| state 0, otherwise | RELEASE (3) | `flags` gains 0x10000; a wake delay of 1 | 1 |
| state 1, `targeting.FindBestTargetIfFireAtWill(U)` (recorded as `services.ScheduleTargetingFireAtWill`) answers V ≠ 0 and the attack order (V, 0) answers 1 | the targeting import; the attack order's imports | none | 5 |
| state 1, otherwise | the targeting import (and the attack order's, when V ≠ 0); `services.RandomInt(30)` = *r* | `flags` gains 0x10000; a wake delay of *r* + 30 | 2 |
| any other state | none | none | 7 |

Recorded: state 1 in every file. Runs that do something: `orders-air-repair.txt` tick 848 (`Unit#251`, `Order#4`).

#### `Standby_Mine` (kind 42)

Table entry: flags 0x1020000 (idle). The idle order of a mine: it self-destructs when an enemy comes near.

| When | Imports | Writes | Result |
| --- | --- | --- | --- |
| state 0, `U.flags` lacks 0x20000000 (`immobile`) | none | none | 7 |
| state 0, otherwise | RELEASE (3) | `flags` gains 0x10000; a wake delay of 1 | 1 |
| state 1, `targeting.FindBestTargetIfFireAtWill(U)` answers V ≠ 0, `V.flags` AND 3 is 1 and `U.flags` has any of 0x300000 (`fireOrder`) | the targeting import; a new order (`SELFDESTRUCT` (`SelfDestruct`, 38), no target, no position, 1, 0, 0) at the front | none | 5 |
| state 1, otherwise | the targeting import; `services.RandomInt(30)` = *r* | `flags` gains 0x10000; a wake delay of *r* + 30 | 2 |
| any other state | none | none | 7 |

Recorded: unrecorded.

#### `Standing_FireOrder` (kind 43)

Table entry: flags 0x10060. `Order.ticks` (here: the stance). SET_STANCE (*U*, 2, 0, `ticks`, 0); then, when `ticks`
is 0 or 1, for *w* = 0, 1, 2 in order: CLEAR (*w*) when `Unit.weapon<w>Flags` has bit 0x10. Result 5. Recorded:
unrecorded.

#### `Standing_MoveOrder` (kind 44)

Table entry: flags 0x10060. `Order.ticks` (here: the stance). Always SET_STANCE (*U*, 1, `ticks`, 0, 0); result 5.
Recorded: unrecorded.

#### `Stop` (kind 45)

Table entry: flags 0. The run, always: announce (0); CLEAR (0), CLEAR (1), CLEAR (2); then, when `U.flags` AND 3 is 2
and `D.flags1` has bit 0x800 (`flying`): a new order (`VTOL_LANDIFCAN` (`VTOL_LandIfCan`, 52), no target, position `U.pos`, 0,
0, 0) at the front. Result 5. Recorded: unrecorded.

#### `Suppress` (kind 46)

Table entry: flags 0x410 (position). `Order.pos` (here: the point fired at), `Order.ticks` (here: the weapon; 2 means
the third weapon alone), `Order.radius` (here: the approach radius, shrinking).

Before the table: when *c* has 0x800 the result is 5 and nothing else happens. Let *wb* be the low byte of `ticks`.

| When | Imports | Writes | Result |
| --- | --- | --- | --- |
| state 0, `D.flags1` has 0x800 (`flying`) | none | none | 8 |
| state 0, otherwise | announce (0); `weapons.GetWeaponRange(U, wb)` | `radius` becomes the answer | 1 |
| state 1, `ticks` is 2 | CLAIM (3); SET_POS (2) | `flags` becomes 0x1c00 | 1 |
| state 1, otherwise | CLAIM (0); CLAIM (1); SET_POS (0); SET_POS (1) | `flags` becomes 0x1c00 | 1 |
| state 2, *c* has 0x400 | RELEASE (3) | `state` becomes 1 | 6 |
| state 2, `U.motion` is 0 | RELEASE (3) | none | 9 |
| state 2, `radius` ≤ 0 | RELEASE (3) | none | 9 |
| state 2, otherwise | RELEASE (3); the approach goal (`O.pos`, `radius`); `weapons.GetWeaponRange(U, wb)` = R; `services.RandomInt(R / 3)` = *r* | `flags` becomes 0xe0; `radius` − *r*; `state` becomes 1 | 4 |
| any other state | none | none | 7 |

R / 3 truncates toward zero; the subtraction wraps. Recorded: unrecorded.

#### `Teleport` (kind 47)

Table entry: flags 0x600. `Order.pos` (here: the destination).

Every run, any state, the code not read: for each unit record V of the unit array, from unit 0 to `Game.unitsEnd`
inclusive, in order, other than *U*, whose `pos` lies in *U*'s model box (lo ≤ `V.pos` ≤ hi on x, y and z, signed,
Q22): with *dest* = `O.pos` + `V.pos` − `U.pos` (wrapping), `services.RecordSimEvent(5, 0, [V.posX, V.posY, V.posZ,
dest.x, dest.y, dest.z, 30, 5], 8)` (`SIM_EVENT_TELEPORT_PARTICLES`), then SET_POSITION (V, *dest*, 1). No writes.
Result 5. Recorded: unrecorded.

#### `VTOL_Evade` (kind 48)

Table entry: flags 0. `Order.ticks` (here: the side, 0 or 1). *W* is the `WeaponDef.range` at `Unit.weapon0Def`. The
side's heading is `U.heading` − 0x4000 when `ticks` is not 0, else `U.heading` + 0x4000.

| When | Imports, in order | Writes | Result |
| --- | --- | --- | --- |
| T is 0, or *c* has any of 0x10008 | none | none | 5 |
| state 0, `U.motion` 0 or `D.flags1` lacks 0x800 (`flying`) | none | none | 7 |
| state 0 | `services.RandomInt(2)` = *s*; the move goal (`U.pos` + Step(the side's heading, *W* << 16), 128) | `ticks` becomes *s* (before the heading is chosen); `flags` becomes 0x100e8 | 1 |
| state 1 | the move goal (`U.pos` + Step(the side's heading, *W* << 17), 128) | `flags` becomes 0x100e8 | 1 |
| state 2 | none | none | 5 |
| any other state | none | none | 7 |

Recorded: unrecorded.

#### `VTOL_Follow` (kind 49)

Table entry: flags 0x200. T (here: the unit guarded); `Order.ticks` (here: the circling angle), `Order.radius` (here:
its parity); `Order.pos` follows the target. *W* is the `WeaponDef.range` at `Unit.weapon0Def`.

| When | Imports, in order | Writes | Result |
| --- | --- | --- | --- |
| T is 0, or *c* has any of 0x48 | when `O.next` is 0: a new order (`VTOL_SEEKGUARD` (`VTOL_SeekGuard`, 63), target T, position `O.pos`, 0, 0, 0) at the tail | its writes | 5 |
| the off-map return | as there | as there | 2 |

Otherwise `O.pos` becomes T's `pos`, and:

| When | Imports, in order | Writes | Result |
| --- | --- | --- | --- |
| state 0, `U.motion` 0 or `D.flags1` lacks 0x800 (`flying`) | none | none | 7 |
| state 0 | announce (9); the climb preparation; `services.RandomInt(0x10000)` = *a* | `ticks` *a*; `radius` *a* AND 1 | 1 |
| state 1 | RELEASE (3) | none | 1 |
| state 2 | the guard pass (below) | as below | 3 or 2 |
| any other state | none | none | 7 |

The guard pass (state 2), with X = `T.attacker` and L = T's first order:

- *the defence* applies when X is not 0, X's player's `Player.allied[` *U*'s `Player.index` `]` is 0, *c* has 0x10, and
  `D.categories` does not contain `X.unitDefIndex`; it asks the attack order (X, 1). When that answers 0 and `U.flags`
  has any of 0x300000 (`fireOrder`), *the free weapons* follow: for each weapon *i* = 0, 1, 2 in turn whose
  `Unit.weapon{i}Flags` has bits 0x2 and 0x10 and whose `WeaponDef.flags` (at `Unit.weapon{i}Def`) lacks 0x4000000
  (`commandFire`): `targeting.GetWeaponTargetUnit(U, i)` (recorded as `services.ScheduleTargetingTargetUnit`) = V; when
  V is not 0, `weapons.WeaponCanReachUnit(U, V, i)`; when V is 0, or that answers 0, or `D.weaponCategories{i}`
  contains `V.unitDefIndex`: SET_UNIT (*i*, X);
- *the repair test*: `units.CanRepair(U, T)`; when it answers not 0, K = `GetOrderType(8, U, T, none)` (its imports);
- *the help test*, its parts asked in this order, each only when the ones before held: `T.list` is not 0; L's `kind` is
  not 0; `D.flags1` has 0x40 (`canbuild`); `units.CanRepair(U, L's target)` is not 0 (L's target may be 0, Q33); T's
  `UnitDef.flags1` has 0x40 (`canbuild`); L's `flags_42` has 0x100000; *U* is not L's target. With all held: *building* = L's kind is
  `MobileBuild` (25), `BuildingBuild` (12) or `VTOL_MobileBuild` (54); *actionable* = (L's `flags_42` has 0x200 and L's
  target is not 0) or L's `flags_42` has 0x400.

| When | Imports, in order | Writes | Result |
| --- | --- | --- | --- |
| the defence applies and the attack order answers 1 | the attack order's | its orders; `flags` becomes 0 | 3 |
| K is not 0 | the defence's (when it applies: the attack order's, then the free weapons'); the repair test's; release the target; a new order (K, target T, no position, 0, 0, 0) at the front | `flags` becomes 0 | 3 |
| the help test holds, not *building*, *actionable* | the defence's; the repair test's; the help test's; release the target; a new order (K2, L's target, position L's `pos`, 0, 0, 0) at the front, K2 being L's kind with `RepairUnit` (35) changed to `VTOL_RepairUnit` (61), `Reclaim` (32) to `VTOL_Reclaim` (58), `ReclaimUnit` (33) to `VTOL_ReclaimUnit` (59) and `HelpBuild` (23) to `VTOL_HelpBuild` (51) | `flags` becomes 0 | 3 |
| the help test holds, *building*, L's target not 0 | the same tests' imports; release the target; a new order (`VTOL_HELPBUILD` (`VTOL_HelpBuild`, 51), L's target, position L's `pos`, 0, 0, 0) at the front | `flags` becomes 0 | 3 |
| otherwise (the circle) | the tests' imports; when *c* has any of 0xe0: `services.RandomInt(0x2000)` = *r*; the move goal (T.pos + Step(`ticks`, distance), 128) with `ticks` as now written | when *c* has any of 0xe0, `ticks` becomes `ticks` − *r* − 0x4000 (wrapping); a wake delay of 30; `flags` gains 0xf8 (so 0xf9) | 2 |

The distance is (*W* + 160) << 16 when `U.flags` has bit 0x80000000 (`armed`), else 0x1400000.

Recorded: `orders-air-repair.txt`, tick 852, `Unit#4` (`Order#5`, made in the same call).

#### `VTOL_GetRepaired` (kind 50)

Table entry: flags 0x200. T (here: the repairing unit, a pad).

| When | Imports, in order | Writes | Result |
| --- | --- | --- | --- |
| T is 0 | speech (7, 23) | none | 8 |
| state 0, *U* is repaired | none | none | 1 |
| state 0, otherwise | none | a wake delay of 0x1e; `flags` gains 0x8 (so 0x9) | 2 |
| state 1 | speech (10, 40) | none | 5 |
| any other state | none | none | 7 |

Recorded: unrecorded.

#### `VTOL_HelpBuild` (kind 51)

Table entry: flags 0x100208 (target, build kind). T (here: the unit under construction); `Order.progress` (here: a retry
count, set to 0).

| When | Imports and writes | Result |
| --- | --- | --- |
| T is 0, or *c* has 8 | speech (7, 8); the orders dirty mark | 8 |
| *c* has 2 | the orders dirty mark | 5 |
| state 0, `U.motion` not 0, `D.flags1` has 0x800 (`flying`), `D.ids` 0 | none | 7 |
| state 0, `U.motion` not 0, `D.flags1` has 0x800 (`flying`) | announce (2); the climb preparation | 1 |
| state 0, otherwise | none | 7 |
| state 1 | `progress` becomes 0; the move goal (T's `pos`, `D.buildRange`); `flags` becomes 0xe0 | 1 |
| state 2, *c* has 0x40 | none | 8 |
| state 2, `T.buildLeft` is 0.0 | none | 5 |
| state 2 | start the building script (the low 16 bits of Head(`U.pos`, T.pos), with no heading subtracted, Q34); the orders dirty mark | 1 |
| state 3 | the air build pass (5) | 2 or 5 |
| other states | none | 7 |

Failure answers: `AddBuildProgress` 0: no particles; the result still follows `buildLeft`. Recorded: unrecorded.

#### `VTOL_LandIfCan` (kind 52)

Table entry: flags 0x400 (position). `Order.ticks` (here: the circling angle), `Order.radius` (here: its parity),
`Order.pos` (here: the place to land near; (0, 0, 0) means here). *S* is `Game.seaLevel`.

| When | Imports, in order | Writes | Result |
| --- | --- | --- | --- |
| `O.next` is not 0 | none | none | 5 |
| *c* has 0x40 | none | none | 5 |
| the off-map return | as there | as there | 2 |
| state 0, `U.motion` 0 or `D.flags1` lacks 0x800 (`flying`) | none | none | 7 |
| state 0 | `services.RandomInt(0x10000)` = *a*; the climb preparation | when `O.pos` is (0, 0, 0), `O.pos` becomes `U.pos`; `ticks` *a*; `radius` *a* AND 1 | 1 |
| state 1, `map.CanPlaceFootprintAt(U, &U.pos)` not 0 | that import; START_SCRIPT "EndTransport" (0, 1, 0, 0); a path target to `U.pos`; the ground height (below) = *v*; SET_ALTITUDE (*t*, *v*); attach target *t*; SET_STATE_BITS (1, 0) | `flags` becomes 0xe0 | 1 |
| state 1, that answer 0 | that import; then the spot search (below) | as below | 2 |
| state 2, *c* has 0x20 | SET_FLIGHT_MODE (1) | none | 5 |
| state 2, otherwise | none | none | 8 |
| any other state | none | none | 7 |

The ground height: *g1* = `map.GetGroundHeight(&U.pos)`. When *g1* > *S* (signed): *g2* = a second call; *v* = 0 when
*g2* > *S*, else a third call *g3* and *v* = *g3* − *S*. When *g1* ≤ *S*: a second call *g2*, and *v* = *g2* − *S* (Q35).

The spot search: for each try *r* = 0x40, 0x50, ..., 0xf0 in that order (at most 12), until a try fits: P = `U.pos`;
P.x gains (`services.RandomInt(2r + 1)` − *r*) << 16, then P.z gains (`services.RandomInt(2r + 1)` − *r*) << 16 (the x
draw first); P is snapped to `U.footprint`; `map.CanPlaceFootprintAt(U, P)`; the try *fits* when that answers not 0.

| When | Imports after the tries, in order | Writes | Result |
| --- | --- | --- | --- |
| a try fits (no later try is made) | a path target to that try's P; attach target *t* (no SET_ event) | `flags` becomes 0xe0 | 2 |
| no try fits, the low byte of *c* has any of 0xe0 | the move goal (`O.pos` + Step(`ticks`, 0xa00000), 0x40), with `ticks` as now written | `ticks` becomes `ticks` − 0x5555 (wrapping); `flags` gains 0xe0 | 2 |
| no try fits, otherwise | the move goal (`O.pos` + Step(`ticks`, 0xa00000), 0x40) | `flags` gains 0xe0 | 2 |

Recorded: unrecorded.

#### `VTOL_Landing` (kind 53)

Table entry: flags 0x600. T (here: the pad unit H); `Order.ticks` (here: first the circling angle, then the pad slot).
*w* is the `WeaponDef.range` at `Unit.weapon0Def`.

| When | Imports and writes | Result |
| --- | --- | --- |
| T 0 | speech (7, 13) | 8 |
| state 0, `U.motion` not 0 and `D.flags1` has 0x800 (`flying`) | announce (12); the climb preparation; `services.RandomInt(0x10000)` = `ticks` | 1 |
| state 0, otherwise | none | 7 |
| state 1, *p* = the landing pad search (H, the landing pad search (H, −1)) is not −1 | the two searches, the inner first (Q36); `state` becomes 2 | 2 |
| state 1, *p* is −1 | the two searches; a path target to H.pos + Step(`ticks`, *w* << 16); `ticks` + 0x4000 (wrapping); SET_APPROACH_RADIUS (*t*, 0x80); attach target *t*; `flags` becomes 0xe8; `state` becomes 1 | 2 |
| state 2 | `movement.NewPathOrderToUnitAtAngle(O, H, −1)` = *t*; SET_APPROACH_RADIUS (*t*, 0xa0); attach target *t*; `flags` becomes 0xe8 | 1 |
| state 3 | *p* = the landing pad search (H, −1); `ticks` *p*; *p* −1: speech (7, 16), result 0. Else `movement.NewPathOrderToUnitAtAngle(O, H, p)` = *t*; SET_APPROACH_RADIUS (*t*, 0x30); attach target *t*; `flags` becomes 0xe8 | 1 (0 on −1) |
| state 4 | none | 1 |
| state 5, *c* has 0x20 | none | 1 |
| state 5 | *p* = the landing pad search (H, `ticks`); `ticks` *p*; *p* −1: speech (7, 14), result 0. Else `movement.NewPathOrderToUnitAtAngle(O, H, p)` = *t*; SET_ALTITUDE (*t*, the `UnitDef.modelMaxYHigh` of `U.cargo` when it is not 0, else 0); START_SCRIPT "EndTransport" (0, 1, 0, 0); attach target *t*; a wake delay of 15; `state` becomes 5; `flags` gains 0xe8 | 2 (0 on −1) |
| state 6, *c* has 0x40 | none | 8 |
| state 6, `map.IsPadSlotFree(H, ticks)` is 0 | speech (7, 15) | 0 |
| state 6, `U.cargo` not 0 | `IsPadSlotFree`; START_SCRIPT "EndTransport" (0, 0, 0, 0); ATTACH_TO_PIECE (`U.cargo`, H, `ticks`, 0) | 5 |
| state 6, no cargo | `IsPadSlotFree`; ATTACH_TO_PIECE (*U*, H, `ticks`, 0); then when *U*'s health is below `D.maxHealth`, H's `UnitDef.flags1` has 0x200 and 0x40, and `H.buildLeft` is 0.0: release the target; a new order (`SELFREPAIR` (`SelfRepair`, 40), target H, no position, 0, 0, 0) at the front | 5 |
| other states | none | 7 |

Failure answers: `QueryScript`'s answer is ignored; the four words keep −1 where the script writes nothing. Recorded:
unrecorded.

#### `VTOL_MobileBuild` (kind 54)

Table entry: flags 0x100508. `Order.ticks` (here: the unit type number to build, entry *B* of `Game.defs`),
`Order.progress` (here: the blocked-site retries), `Order.pos` (here: the site); T becomes the unit being built. *f* =
(*B*.`footprintx`, *B*.`footprintz`).

| When | Imports, in order | Writes | Result |
| --- | --- | --- | --- |
| *c* has 0x2 | the orders dirty mark | none | 5 |
| *c* has 0x8 | speech (7, 7); the orders dirty mark | none | 8 |
| state 0, `U.motion` 0 or `D.flags1` lacks 0x800 (`flying`) | none | none | 7 |
| state 0 | announce (2); the climb preparation | as there | 1 |
| state 1 | the move goal (`O.pos` as rewritten, `D.buildRange`) | `progress` 0; `O.pos` x and z become Centre(Cell(`O.pos`, *f*), *f*); `flags` becomes 0xe0 | 1 |
| state 2, *c* has 0x40 | none | none | 8 |
| state 2, `map.CanPlaceUnitFootprint(B, 0, Cell(O.pos, f), 1)` (recorded as `services.ScheduleMapCanPlaceUnitFootprint`) 0, `progress` 0 | that import; speech (7, 43) | `progress` + 1; a wake delay of 30 | 2 |
| the same, `progress` not 0 and at most 10 (signed) | that import | `progress` + 1; a wake delay of 30 | 2 |
| the same, `progress` > 10 (signed) | that import; speech (7, 31) | none | 8 |
| state 2, that answer not 0 | that import; `map.SnapWorldPosToFootprint(B, P)` on a copy P of `O.pos` (it rewrites P); `units.CreateUnit(U.playerIndex, the low 16 bits of ticks (unsigned), &O.pos, 0, 1, 0)` = N (`O.pos` already P); `units.UnitRefSetUnit(link, N)`; then, when T (as the block now holds it) is 0: speech (7, 36); otherwise: speech (9, 30), adding an order to T (`GETBUILT` (19), queue 1, target *U*, no position, 0, 0), start the building script (the build heading to T's `pos`), the orders dirty mark | `O.pos` becomes P | 8 when T is 0; else 1 |
| state 3, `U.cobStateFlags` lacks 0x1 | the air build pass (1) | `flags` becomes 0xe (10 OR 0x4), and the pass's writes are added to it (Q37) | the pass's |
| state 3, otherwise | the air build pass (1) | the pass's | the pass's |
| state 4 | the air build pass (1) | the pass's | 2 or 1 |
| state 5 | speech (8, 3) | none | 5 |
| any other state | none | none | 7 |

Recorded: unrecorded.

#### `VTOL_Move` (kind 55)

Table entry: flags 0x402 (position). `Order.pos` (here: the destination).

| When | Imports, in order | Writes | Result |
| --- | --- | --- | --- |
| state 0, `U.motion` 0 or `D.flags1` lacks 0x800 (`flying`) | none | none | 7 |
| state 0 | the climb preparation | as there | 1 |
| state 1 | announce (0); RELEASE (3); a path target to `O.pos` (after the snap); attach target *t* | `O.pos` snapped to `U.footprint`; `flags` becomes 0xe0 | 1 |
| state 2 | when `O.next` is 0: speech (6, 0) | none | 5 |
| any other state | none | none | 7 |

Recorded: unrecorded.

#### `VTOL_Patrol` (kind 56)

Table entry: flags 0x412 (position). `Order.pos` (here: the patrol point).

| State | When | Imports and writes | Result |
| --- | --- | --- | --- |
| 0 | `U.motion` not 0 and `D.flags1` has 0x800 (`flying`) | the patrol return order; announce (19); the climb preparation; RELEASE (3) | 1 |
| 0 | otherwise | none | 7 |
| 1 | | `subFlags` loses 0xe0 | 1 |
| 2 | *c* has any of 0xe0 | none | 6 |
| 2 | otherwise | the move goal (`O.pos` + Step(Head(`U.pos`, `O.pos`), 0x1400000), 0x150); `flags` gains 0xe0; then the pad diversion (result 0 when it happens: its release drops the goal just attached, Q38); then `targeting.FindBestTargetIfFireAtWill(U)` (recorded as `services.ScheduleTargetingFireAtWill`) = V; V not 0 and the attack order (V, 0) answering 1: `flags` becomes 0, result 3; otherwise a wake delay of 0x1e, result 2 | 2, 3 or 0 |
| other | | none | 7 |

Recorded: `orders-air.txt`, tick 4100 (`Unit#5`, `Order#11`, states 0, 1 and 2 in one call).

#### `VTOL_Pickup` (kind 57)

Table entry: flags 0x200. T (here: the unit to load); `Order.ticks` (here: the carrier's transport piece). *S* is
`Game.seaLevel`.

Checked first, in this order:

| When | Imports, in order | Writes | Result |
| --- | --- | --- | --- |
| T is 0, or *c* has any of 0x10048 | speech (7, 35) | none | 8 |
| T's `UnitDef.modelMaxY` + `T.posY` ≤ *S* << 16 (signed 32-bit) | speech (7, 35) | none | 8 |
| `U.cargo` is not 0 | none | none | 8 |

Then by state:

| When | Imports, in order | Writes | Result |
| --- | --- | --- | --- |
| state 0, `U.motion` 0 or `D.flags1` lacks 0x800 (`flying`) | none | none | 7 |
| state 0, `T.footprint.x` (signed) > `D.capacity` | speech (7, 38) | none | 8 |
| state 0, otherwise | announce (17); the climb preparation | as there | 1 |
| state 1 | `movement.NewPathOrderToUnit(O, T)` = *t*; SET_ALTITUDE (*t*, `D.altitude`); SET_APPROACH_RADIUS (*t*, 0x30); attach target *t* | `flags` becomes 0x100e8 | 1 |
| state 2 | announce (20); `scripts.QueryScript(U.script, "QueryTransport", &O.ticks, 0, 0, 0)` (it writes `ticks`) | `ticks` −1 before the import; `flags` becomes 0x100e8 | 1 |
| state 3 | START_SCRIPT "BeginTransport" (1, 1, 1, *h*); `network.SendScriptCallByName(U, "BeginTransport", 1, h, 0, 0, 0)`; `scripts.GetPieceOffset(U, ticks, o)`; `movement.NewPathOrderToUnitAtAngle(O, T, −1)` = *t*; SET_ALTITUDE (*t*, −whole(*o*.y)); attach target *t* | `flags` becomes 0x100ea | 1 |
| state 4, *c* has any of 0x42 | START_SCRIPT "EndTransport" (0, 0, 0, 0) | none | 8 |
| state 4, otherwise | ATTACH_TO_PIECE (T, *U*, `ticks`, 0); speech (12, 0); a path target to `U.pos`; SET_ALTITUDE (*t*, `D.altitude`); the target is not attached (Q18) | `flags` gains 0xe0 | 1 |
| state 5 | none | none | 5 |
| any other state | none | none | 7 |

*h* is T's `UnitDef.modelMaxY` (a 32-bit int). Recorded: `orders-air.txt`, tick 4095, `Unit#3` (`Order#8`, state 1).

#### `VTOL_Reclaim` (kind 58)

Table entry: flags 0x100800. `Order.pos` (here: the feature's position), `Order.ticks` (here: the reclaim time left).

Every run first makes the site cell and size (the size is not used). An answer 0xffff: speech (7, 22), result 8.
Otherwise, when `F.flags` lacks 0x80: result 8.

| State | When | Imports and writes | Result |
| --- | --- | --- | --- |
| 0 | `U.motion` not 0, `D.flags1` has 0x800 (`flying`), `D.flags2` has 0x400 | announce (21); the climb preparation | 1 |
| 0 | otherwise | none | 7 |
| 1 | | a path target to `O.pos`; attach target *t*; `flags` becomes 0xe0 | 1 |
| 2 | *c* has 0x40 | none | 8 |
| 2 | | `ticks` becomes ftol(30 + (`F.metal` + `F.energy`) × 0.5), in double; speech (11, 0) | 1 |
| 3 | | a wake delay of 2; `ticks` − 2; when that is 0 or less: result 1. Else BUSY (300); when `ticks` > 30: the nano point of *U*; the feature box (one `GetGroundHeight`); the reverse spray (the feature box, that point), recorded twice | 2 (1 when done) |
| 4 | | features' RECLAIM 35 (20): `unit` *U*, `pos` `O.pos` | 5 |
| other | | none | 7 |

Recorded: unrecorded.

#### `VTOL_ReclaimUnit` (kind 59)

Table entry: flags 0x100200. T (here: the unit reclaimed); `Order.ticks` (here: the damage of one pulse),
`Order.radius` (here: the ticks since the last pulse).

| When | Imports and writes | Result |
| --- | --- | --- |
| T 0, or *c* has any of 0x10008 | none | 5 |
| state 0, `U.motion` 0 or `D.flags1` lacks 0x800 (`flying`) | none | 7 |
| state 0, `D.flags2` lacks 0x400 | speech (7, 22) | 7 |
| state 0, `units.CanReclaim(U, T)` 0 | that import; speech (7, 33) | 8 |
| state 0 | that import; announce (21); the climb preparation | 1 |
| state 1 | `ticks` becomes the reclaim pulse (*U*, T, 15); `radius` 0; `O.pos` snapped to `U.footprint`; a path target to T's `pos`; attach target *t*; `flags` gains 0x100e8; speech (11, 0) | 1 |
| state 2, *c* has 0x40 | none | 9 |
| state 2 | `flags` gains 0x10008; with *dx*, *dz* = `U.pos` − T.pos (wrapping), *q* = hi(*dx*²) + hi(*dz*²) and *r* = `D.buildRange`: when *q* ≤ *r* × *r* (signed 32-bit) and `units.CanReclaim(U, T)` is not 0: when `radius` ≥ 15, DAMAGE (T, `ticks`, 5) and `radius` 0; the nano point of *U*; the reverse spray (the target box of T, that point); a wake delay of 2; `radius` + 2; result 2. Otherwise (`CanReclaim` asked only when *q* ≤ *r* × *r*): a wake delay of 30, result 0 | 2 or 0 |
| other states | none | 7 |

Recorded: unrecorded.

#### `VTOL_RepairPatrol` (kind 60)

Table entry: flags 0x412 (position). P is `U.player`.

| When | Imports and writes | Result |
| --- | --- | --- |
| *c* has any of 0x48 | a wake delay of 0x1e | 0 |
| state 0, `U.motion` not 0, `D.flags1` has 0x800 (`flying`), `D.flags2` has 0x200 | when T is not 0, `O.pos` becomes T's `pos`; the patrol return order; announce (19); the climb preparation | 1 |
| state 0, otherwise | none | 7 |
| state 1, *c* has any of 0xe0 | none | 6 |
| state 1 | the state 1 run (below) | 0, 2, 3 or 6 |
| other | none | 7 |

The state 1 run. It always starts with a path target to `O.pos`, SET_ALTITUDE (*t*, `D.altitude`), attach target
*t*, a wake delay of 0x2d and `flags` gaining 0xe0. Then:

- *the repair test*: when `P.energy` ≥ `P.energyCapacity` × 0.2 (in double): the repairable list (`D.range` << 16);
  when it is not empty, `services.RandomInt(n)` = *i* and V = entry *i*; then `units.CanRepair(U, V)` = *a1*; when
  *a1* is 0, or *a1* is not 0 and `V.buildLeft` is not 0.0, a second `units.CanRepair(U, V)` = *a2* (Q39);
- *the reclaim rows*, given the pick's E, *e*, M, *m*, the first that holds gives the point X: (1) `P.metal` <
  `P.metalCapacity` × 0.2 and M not none: M; (2) `P.energy` < `P.energyCapacity` × 0.2 and E not none: E; (3) M not
  none and `P.metal` + *m* ≤ `P.metalCapacity` (in double): M; (4) E not none and `P.energy` + *e* ≤
  `P.energyCapacity`: E.

| When | Imports after the start, in order | Writes | Result |
| --- | --- | --- | --- |
| the pad diversion happens | the diversion's | its writes | 0 |
| *a1* not 0, `V.buildLeft` 0.0, and the repair order (V) answers 1 | the diversion's test; the repair test's; the repair order's | its orders | 6 |
| *a1* not 0, `V.buildLeft` 0.0, and the repair order answers 0 | the same | none | 3 |
| *a2* not 0 and `V.buildLeft` not 0.0 | the diversion's test; the repair test's (both `CanRepair`); release the target; a new order (`VTOL_HELPBUILD` (`VTOL_HelpBuild`, 51), target V, no position, 0, 0, 0) at the front | `flags` becomes 0 | 3 |
| none of the above, the reclaim pick (`U.pos`, 0xf00000) answers 1 and a reclaim row gives X | the diversion's test; the repair test's; the pick; release the target; a new order (`VTOL_RECLAIM` (`VTOL_Reclaim`, 58), no target, position X, 0, 0, 0) at the front | `flags` becomes 0 | 3 |
| otherwise | the diversion's test; the repair test's; the pick | none | 2 |

The pick is asked whatever the resources are. Recorded: unrecorded.

#### `VTOL_RepairUnit` (kind 61)

Table entry: flags 0x100200. T (here: the unit to repair); `Order.start` and `Order.progress` (here: the leash);
`Order.pos` follows the target.

Checked first, in this order: T is 0: speech (7, 26), result 8; the leash test holds: result 5; `T.flags` AND 3 is not
1: speech (7, 26), result 5. Then `O.pos` becomes T's `pos`, and:

| When | Imports, in order | Writes | Result |
| --- | --- | --- | --- |
| state 0, `U.motion` 0 or `D.flags1` lacks 0x800 (`flying`) | none | none | 7 |
| state 0, `units.CanRepair(U, T)` 0 | that import; speech (7, 25) | none | 8 |
| state 0, otherwise | that import; announce (24); the climb preparation | as there | 1 |
| state 1 | a path target to `O.pos`; SET_ALTITUDE (*t*, `D.altitude`); attach target *t* | `flags` becomes 0xe8 | 1 |
| state 2, *c* has 0x40 | none | none | 8 |
| state 2, `T.flags` AND 3 not 1 | none | none | 8 |
| state 2, `T.flags` has any of 0xc | none | a wake delay of 15 | 0 |
| state 2, T is repaired | none | none | 1 |
| state 2, otherwise | REPAIR (T, the float of (`D.buildRate` / 30, integer division)); the nano point of `O.unit`; the nano spray (that point, the target box of T) | a wake delay of 1; `flags` gains 0x8 (so 0x9) | 2 |
| state 3 | speech (10, 40) | none | 5 |
| any other state | none | none | 7 |

T is read as the block holds it after REPAIR. Recorded: unrecorded.

#### `VTOL_SeekAttack` (kind 62)

Table entry: flags 0x600. `Order.ticks` (here: the circling angle), `Order.radius` (here: its parity). *w* is the
`WeaponDef.range` at `Unit.weapon0Def`.

| When | Imports and writes | Result |
| --- | --- | --- |
| *c* has 0x40 | none | 5 |
| the off-map return | as there | 2 |
| state 0, `U.motion` not 0, `D.flags1` has 0x800 (`flying`), T not 0, the attack order (T, 0) answers 1 | the attack order; `flags` becomes 0 | 0 |
| state 0, the same, T not 0, the attack order answers 0 | no import (Q40) | 1 |
| state 0, the same, T 0 | when `O.posX`, `posY`, `posZ` are all 0, `O.pos` becomes `U.pos`; `services.RandomInt(0x10000)` = `ticks`; `radius` `ticks` AND 1; the climb preparation | 1 |
| state 0, otherwise | none | 7 |
| state 1 | RELEASE (3); the pad diversion (result 0 when it happens); `targeting.FindBestTargetIfFireAtWill(U)` = V; V not 0 and the attack order (V, 0) answering 1: result 5. Else when *c* has any of 0xe0: `services.RandomInt(0x2000)` = *q*, `ticks` += −*q* − 0x5555; the move goal (`O.pos` + Step(`ticks`, (*w* + 160) << 16), 128); `services.RandomInt(30)` = *r*; a wake delay of *r* + 30; `flags` gains 0xe0 | 2 |
| other | none | 7 |

Recorded: unrecorded.

#### `VTOL_SeekGuard` (kind 63)

Table entry: flags 0x600. `Order.ticks`, `Order.radius` as in `VTOL_SeekAttack`; *w* likewise.

| When | Imports and writes | Result |
| --- | --- | --- |
| *c* has 0x40 | none | 5 |
| the off-map return | as there | 2 |
| state 0, `U.motion` not 0 and `D.flags1` has 0x800 (`flying`) | when the three `O.pos` words are 0, `O.pos` becomes `U.pos`; `services.RandomInt(0x10000)` = `ticks`; `radius` `ticks` AND 1 | 1 |
| state 0, otherwise | none | 7 |
| state 1 | the guard sweep (below) | 0, 2 or 3 |
| other | none | 7 |

The guard sweep, in order:

1. The pad diversion (result 0 when it happens).
2. The ground allies (`D.range` << 16). When there is at least one, with A the first: release the target; K =
   `GetOrderType(7, U, A, none)`; a new order (K, target A, no position, 0, 0, 0) at the front (K is not checked,
   Q21); `flags` becomes 0; result 3.
3. Otherwise: when *c* has any of 0xe0, `services.RandomInt(0x2000)` = *q* and `ticks` += −*q* − 0x4000. The move goal
   (`O.pos` + Step(`ticks`, (*w* + 160) << 16), 128); a wake delay of 30; `flags` gains 0xf8; result 2.

Recorded: `orders-air-repair.txt`, tick 852 (`Unit#4`, `Order#3`, reached after the orders before it end; states 0 and
1 in one call).

#### `VTOL_Standby` (kind 64)

Table entry: flags 0x20000 (`ORDER_FLAG_IDLE`). `Order.start` (here: the home point, in whole units).

| When | Imports, in order | Writes | Result |
| --- | --- | --- | --- |
| state 0, `U.motion` 0 or `D.flags1` lacks 0x800 (`flying`) | none | none | 7 |
| state 0 | CLAIM (3) | `flags` 0x10000, then a wake delay of 1 (so 0x10001); `start` becomes (`U.xWhole`, `U.zWhole`) | 1 |
| state 1 | `targeting.FindBestTargetIfFireAtWill(U)` (recorded as `services.ScheduleTargetingFireAtWill`) = V; when V is not 0: the attack order (V, 0) | when that answers 1: `flags` 0, `state` 0 | 3 when it answers 1; else 1 |
| state 2, `D.flags1` has 0x800 (`flying`), `U.flags` AND 3 is 2, `U.cargo` not 0 | `services.RandomInt(0x10000)` = *a*; `services.RandomInt(0x20)` = *b*; a path target to (`start.x` << 16, 0, `start.y` << 16) + Step(*a*, (*b* + 8) << 16); SET_ALTITUDE (*t*, `D.altitude`); attach target *t*; `services.RandomInt(0xf)` = *c2* | a wake delay of *c2* + 0x1e; `state` 1 | 2 |
| state 2, `D.flags1` has 0x800 (`flying`), `U.flags` AND 3 is 2, `U.cargo` 0 | the new order's imports | a new order (`VTOL_LANDIFCAN` (`VTOL_LandIfCan`, 52), no target, position `O.pos`, 0, 0, 0) at the front | 5 |
| state 2, otherwise | `services.RandomInt(0x1e)` = *c2* | `flags` gains 0x10000; a wake delay of *c2* + 0x1e; `state` 1 | 2 |
| any other state | none | none | 7 |

The home words are signed 16-bit, shifted in 32 bits. Recorded: `orders-guard-attacked.txt`, ticks 11 and 12, `Unit#5`
(`Order#6`).

#### `VTOL_Unload` (kind 65)

Table entry: flags 0x400. T becomes the cargo; `Order.pos` (here: the drop point). *fits* is
`map.CanPlaceUnitFootprint(E, 0, Cell(O.pos, F), 1)` (recorded as `services.ScheduleMapCanPlaceUnitFootprint`), E and F
the `UnitDef` and `Unit.footprint` of T.

| When | Imports, in order | Writes | Result |
| --- | --- | --- | --- |
| `U.cargo` is 0 (checked first) | none | none | 5 |
| state 0, `U.motion` 0 or `D.flags1` lacks 0x800 (`flying`) | none | none | 7 |
| state 0 | announce (41); `units.UnitRefSetUnit(link, U.cargo)` (it writes T); a path target to `O.pos`; SET_ALTITUDE (*t*, `D.altitude`); SET_APPROACH_RADIUS (*t*, 0x140); attach target *t* | `flags` becomes 0xe8 | 1 |
| state 1, *fits* not 0 | *fits*; a path target to `O.pos`; SET_ALTITUDE (*t*, `modelMaxYHigh` of `U.cargo`'s `UnitDef`); attach target *t* | `flags` becomes 0xe8 | 1 |
| state 1, *fits* 0 | *fits*; speech (7, 37) | none | 9 |
| state 2, *c* has 0x40 | none | none | 9 |
| state 2, *fits* 0 | *fits*; speech (7, 37) | none | 9 |
| state 2, *fits* not 0 | *fits*; START_SCRIPT "EndTransport" (0, 0, 0, 0); ATTACH_TO_PIECE (`U.cargo`, 0, −1, 1); a path target to `U.pos`; SET_ALTITUDE (*t*, `D.altitude`); attach target *t* | `flags` becomes 0xe0 | 1 |
| state 3 | speech (13, 0) | none | 5 |
| any other state | none | none | 7 |

`U.cargo` in state 2 is read before ATTACH_TO_PIECE, `U.pos` as the block holds it after that effect. The footprint is
T's but the altitude of state 1 is `U.cargo`'s (Q41). Recorded: unrecorded.

#### `Wait` (kind 66)

Table entry: flags 0x4. `Order.ticks` (here: the ticks left to wait), `Order.radius` (here: the watch radius).

| When | Imports in order | Writes | Result |
| --- | --- | --- | --- |
| `radius` ≠ 0, the enemy count (`U.pos`, `radius`) ≠ 0 | the enemy count | none | 5 |
| `radius` ≠ 0, the count 0, `ticks` ≤ 0 (signed) | the enemy count | none | 5 |
| `radius` ≠ 0, the count 0, `ticks` > 0 | the enemy count; `services.RandomInt(30)` = *g* | *w* = *g* + 150: `ticks` − *w*; a wake delay of *w* | 2 |
| `radius` = 0, state 0 | none | a wake delay of `ticks` | 1 |
| `radius` = 0, state 1 | none | none | 5 |
| `radius` = 0, any other state | none | none | 7 |

Recorded: unrecorded.

#### `WaitForAttack` (kind 67)

Table entry: flags 0x204 (target). No imports.

| When | Writes | Result |
| --- | --- | --- |
| T = 0 | none | 5 |
| state 0 | `flags` becomes 0x18 | 1 |
| state 1 | none | 5 |
| other | none | 7 |

Recorded: unrecorded.

## 5. Exactness

**Random draws.** Every draw is `services.RandomInt(n)`, an import call, so its place among the imports is part of the
behaviour; a draw with *n* below 2 is still made (it answers 0). The draws, per run, in the order of section 4:

- the frame: result 3 draws one `RandomInt(15)` (a delay of 30 to 44 ticks) and result 9 of `RunOrders` with `next` 0
  one `RandomInt(30)` (30 to 59), after the run's own draws; result 3 of `RunSecondaryOrders` draws the same;
- `Patrol` state 2, `Standby` state 1, `Standby_Mine` state 1, `RepairUnit` state 1 out of range: one `RandomInt(30)`,
  the delay *r* + 30; `Wait`: one `RandomInt(30)`, plus 150; `SelfDestruct`: one `RandomInt(15)` after the speech;
  `Guard_NoMove`: state 1 one `RandomInt(3)` (plus 3) after SET_UNIT, state 2 one `RandomInt(100)` compared < 80 only
  when the reach test is not made or answers 0, state 3 one `RandomInt(n)` over the enemy list;
- `Attack_Chase` steps 1 to 4: one `RandomInt(0x8000)` per run; `Suppress`: `RandomInt(R / 3)` after the second
  `GetWeaponRange`; `AttackUType`: `RandomInt(90)` in state 0, then one `RandomInt(d >> 1)` per qualifying unit in array
  order; `Follow_Ground`: one `RandomInt(65536)`, taken as signed 16-bit; `RepairPatrol` and `VTOL_RepairPatrol`: one
  `RandomInt(n)` over the repairable list; "the site point": `RandomInt(F.height)` before `map.GetGroundHeight`;
- the air kinds: `VTOL_Evade` `RandomInt(2)`; `VTOL_LandIfCan` state 0 `RandomInt(0x10000)`, its spot search two
  `RandomInt(2r + 1)` per try, x before z, for *r* = 0x40 to 0xf0; `VTOL_Standby` state 2 `RandomInt(0x10000)`,
  `RandomInt(0x20)`, then `RandomInt(0xf)`, or `RandomInt(0x1e)` alone; `AirToGroundHover` state 1 `RandomInt(0x4000)`,
  state 3 `RandomInt(0x10000)` after two misses; `AirStrike` state 2 and `AirToGround` state 1 `RandomInt(0x4000)`;
  `AirToGround` state 3 `RandomInt(0x80)` after the path import, state 4 `RandomInt(2)`; `AirToAir` state 1
  `RandomInt(30)` after the attach; `VTOL_Follow` `RandomInt(0x10000)` in state 0 and `RandomInt(0x2000)` in the circle
  when *c* has 0xe0; `VTOL_SeekAttack` and `VTOL_SeekGuard` `RandomInt(0x10000)` in state 0 and `RandomInt(0x2000)` when
  *c* has 0xe0, and `VTOL_SeekAttack` `RandomInt(30)` after the attach; `VTOL_Landing` state 0 `RandomInt(0x10000)`;
  the pad diversion's `RandomInt(count)` after its query and its release.

**Integer arithmetic.** 32-bit two's complement; every sum, difference, negation and shift left wraps. `>>` of a signed
value is arithmetic; `/` of two ints truncates toward zero. hi(*x*²) is the high 32 bits of the exact signed 64-bit
square. A value "kept as 16 bits" is its low 16 bits, sign-extended unless the text says unsigned.

- The wake test compares the tick and `wakeFrame` as unsigned 32-bit numbers; 0xffffffff never wakes. `wakeFrame` =
  the tick + n wraps.
- `state` is unsigned 8-bit: result 1 takes 255 to 0.
- `Unit.netDirtyFlags` is zero-extended from 16 bits before the OR with `subFlags`; the CLEAR_NET_DIRTY `mask` is the
  whole 32-bit pending value. The three CLEARs of a pending 0x10000 are in weapon order, after CLEAR_NET_DIRTY and
  before the run.
- A new order's `kind` and table entry are K as an unsigned byte; kind bytes and `mode` are low 8 bits, not checked
  against the table beyond `Apply`'s limits.
- `Order.ticks` += `ticks` (PARALYZE) and `Order.radius` += C (a build count) wrap; the build count compare `B.radius` >
  −C is signed with −C wrapping. `units.FindUnitTypeId`'s answer is taken as its low 16 bits, unsigned.
- The COMMAND match accepts *d* when (*d* + 0x100000), as unsigned, is at most 0x200000.
- Type set membership: the word at the set's offset + 4 × (*t* >> 5), unbounded (past the 16 words for *t* ≥ 512),
  bit *t* AND 31, *t* the unsigned 16-bit `unitDefIndex`.
- The group range *n* = (`Game.unitsEnd` − F) / 280 + 1, capped at 512; the group sums are of the signed whole parts,
  signed 32-bit; the centre (sum / count) × 65536 wraps; the distance squares are hi(·) of exact 64-bit products, summed
  in 32 bits and compared signed with *count* × 3000.
- `GetOrderType`'s visibility: *x* and *y* from the sign-extended whole parts, arithmetic shifts, compared unsigned;
  the index *x* + *y* × `exploredWidth` into 16-bit words. The water test sums two signed 16-bit values in 32 bits and
  compares them with the unsigned byte `Game.seaLevel`.
- "Health below" sign-extends the 16-bit `health` and compares it unsigned (32-bit) with the limit; damaged is
  (`maxHealth` >> 2) × 3, unsigned.
- The cell of a footprint: (P − *f* × 0x80000 + 0x80000) >> 20, kept as 16 bits; the centre (*f* + 2*c*) << 19. The
  `Park` marker shifts `posX` and `posZ` right 20 as unsigned, every value kept as 16 bits.
- The attack and repair orders' `start` is (whole(`posX`), whole(`posZ`)); their `progress` is `maneuverleashlength`
  (unsigned 16-bit) or `range` (signed 16-bit), widened.
- Amounts of build and repair: the float of (`buildRate`, unsigned 16-bit, integer divided by 30): exact.
- Multiples of angles (`heading` ± 0x4000, Head + 0xdb6e, *q* + *a* − 0x2000, ± 0x2000, `ticks` − *q* − 0x4000, −*q* −
  0x5555, `ticks` + 0x4000) are 32-bit sums; `Order.ticks` keeps the whole word and S and C use its low 16 bits. The
  scales (*W* << 16, *W* << 17, (*W* + 160) << 16, ((*W* × 2) / 3) << 16, *w* × 0x30000, `buildRange` << 16, (*b* + 8) <<
  16, (`attackrunlength` + 0x3c0) << 16, `range` << 16) wrap. The values of SET_APPROACH_RADIUS and SET_HEADING given as
  16-bit (`buildRange`, *W*, *t*, the heading) are their low 16 bits, sign-extended.
- `mapWidth` / 2, `mapHeight` / 2, `altitude` / 2, `maxvelocity` / 2, *d* / 2 truncate toward zero.
- The capture time steps 2 to 4: the min with 1800 signed; the health scaling unsigned 32-bit with the unsigned
  division; the experience factor signed, divisions toward zero, `killCount` unsigned 16-bit.
- `BUSY`'s `until` is the tick + n, signed 32-bit. The particle boxes are signed 32-bit sums that wrap.
- `VTOL_ReclaimUnit`'s *r* × *r* is a 32-bit product (it wraps for `buildRange` ≥ 46341); `ReclaimUnit`'s L × L likewise.
- The `Ground_Unload` argument is (`posX` AND 0xffff0000) + (`posZ` >> 16), wrapping; the `VTOL_Pickup` altitude is
  −whole(*o*.y).

**Floating point.** Doubles are IEEE binary64 and floats binary32, rounding to nearest. Where a formula mixes floats,
every operation on them is made in double precision (the float values widened exactly), with no rounding to float
between, unless the text says a value is rounded to a float.

- ftol (the kit's): truncation toward zero to 64 bits, the low 32 bits; 0 for a NaN or an infinity.
- `buildLeft` tests compare the f32 with 0.0, so −0.0 equals 0.0.
- The build gap: hyp of the int differences as doubles, ftol, then whole(·); the footprint terms ftol(hyp × 8.0) and
  ftol(hyp × −8.0) in double.
- The leash test: hyp of the 16-bit differences, ftol, compared signed with `progress`.
- `HelpBuild`'s ring: sqrt in double (correctly rounded) of *fx*² + 2*fz*, times 16.0, ftol, then integer division by 2.
- The reclaim pulse: the product read as unsigned 32-bit into a double; *m* the metal cost when it is greater than 10.0
  (a NaN gives 10.0); *m* × 300 and the quotient in double, the product not rounded to a float; ftol; at least 1.
- `Reclaim`'s time: (`energy` + `metal`) in double, × 0.5, + 15, ftol. `VTOL_Reclaim`'s: (`metal` + `energy`) in
  double, × 0.5, + 30, ftol.
- `RepairPatrol` and `VTOL_RepairPatrol`: each threshold is the float capacity times the double 0.2, compared in
  double with the float resource widened; the sums `metal` + *m* and `energy` + *e* are made in double (not rounded to
  float) and compared with the capacity widened.
- `AirStrike` state 4: (2 × `altitude`) / *g* in double from the ints, sqrt, × 30.0, × *m* (an int), ftol; then + 1 +
  `attackrunlength` in int.
- `Resurrect`'s countdown: `buildtime` × 0.3 / (the double of `buildRate` / 30, integer division), ftol; a divisor of
  0.0 gives an infinity or a NaN, so 0.
- The capture time step 1: *e* and *m* rounded to floats, then *e* / 2000 + *m* / 140 + 150 in double, ftol.
- `BuildingBuild`'s refund F: (1.0 − `buildLeft`) × `metalCost` in double, ftol, the result read as unsigned 32-bit and
  rounded to a float.
- `BuildWeapon`'s charges: N, P and Tr converted to floats; N × `metalPerShot` / Tr and P × `metalPerShot` / Tr in
  double, each through ftol; their difference signed 32-bit, rounded to a float for the payload. Tr = 0 gives 0 / 0, a
  NaN, so ftol 0 for both and a charge of 0.
- `Ground_Unload`'s radius ftol(`height180` × 1.5), `height180` signed 16-bit.

**Tables and functions.** S, C, A, hyp and ftol are the kit runtime's (`runtime/vectors/README.md`, "Rules"), with
vectors under `runtime/vectors/`. The order type table is the block's (section 3). `MissionTypeFromName` uses the
runtime's `strcmpi`. The `SelfDestruct` voice slot L(*k*) is 22 − *k* for *k* from 0 to 5.

## 6. Quirks

- **Q1**: PARALYZE puts its new paralysis order at the front of the unit's orders (it becomes the first order), though
  `interfaces/orders.h` says it is added after them.
- **Q2**: COMMAND with `queue` 1 and no target removes the first order of that kind whatever its target.
- **Q3**: COMMAND looks for a match only in the first list, even for a kind of the second list.
- **Q4**: raising a build count looks only at the last order of the kind's list, but adds a new order elsewhere: a
  `BuildWeapon` order at the front of the second list, a `MobileBuild` or `BuildingBuild` order after the marked order
  of the first list. So a raise while the new order is not last (a `BuildWeapon` order with others behind it, or a
  build order with a later order after it) adds another order instead of raising the count.
- **Q5**: changing a build count by 0 removes a last build order of that type whose count is 0 or below.
- **Q6**: GROUP_COMMAND passes the clicked position to `GetOrderType` even when `hasPos` is 0.
- **Q7**: `GetOrderType` mode 12 asks `los_radar.GetMapCellAtPosition` with the address 0 when there is no position.
- **Q8**: `RunOrders` publishes CLEAR_NET_DIRTY with `mask` 0 before every run of an order whose `flags` is 0.
- **Q9**: a run that leaves `flags` 0 and has result 0, 1, 2 or 4 runs again at once in the same `RunOrders` call.
- **Q10**: `RunSecondaryOrders` never resets `wakeFrame`: once the tick has reached it, the order runs at every call
  whatever its `flags`, and, as the walk starts again at the first order after each run, a run that neither moves
  `wakeFrame` past the tick nor ends the order runs again without end. It deletes the order on results 6, 7 and 9
  (whatever `next` is).
- **Q11**: the attack order (and the repair order) makes its return move order even when `GetOrderType` mode 2 gives
  0, an order of kind 0.
- **Q12**: `GetOrderType` modes 1 and 2 ask `units.CanRepair` twice; mode 8 asks it with no target.
- **Q13**: `GetOrderType`'s visibility test uses the explored size of the unit's player but the bit of
  `Game.playerIndex`.
- **Q14**: an order deleted while it is not the unit's first order (every order of the second list among them)
  publishes no weapons RELEASE at its end.
- **Q15**: the end of an order publishes SET_TARGET 0 only when movement's current target of the unit is the order's
  `attached`; otherwise it destroys `attached` and leaves the unit's target.
- **Q16**: with `Unit.motion` 0, the approach, ring and footprint goals still call movement's `New*` import, and the
  target it answers is neither attached nor destroyed.
- **Q17**: `AirStrike` state 6: a damaged unit with no pad ends with result 0, keeping the turn-away goal and `flags`
  0xe2.
- **Q18**: `VTOL_Pickup` state 4 and `AirToGroundHover`'s re-aim after two misses make a movement target (and publish
  its SET_ events) but never attach it: `attached` keeps the earlier target.
- **Q19**: `Attack_Chase` asks `targeting.ChooseWeapon` only when `Order.ticks` is 0, so an order made with a weapon
  of 1 or 2 keeps it, and only one made with 0 takes the chosen weapon.
- **Q20**: `Attack_Chase` state 2 at step 1 leaves `Order.radius` (the step) at 1 unless the height difference exceeds
  0x80000, which sets it to 6: each such run draws a new random approach point, and the step runs 0, 1, 1, ..., 6, 7,
  8, 0, never 2 to 5 unless `radius` holds one already.
- **Q21**: `AttackSpecial` stores, `AttackUType` makes, and `VTOL_SeekGuard`'s guard sweep makes an order of whatever kind
  `GetOrderType` answers, 0 included.
- **Q22**: `AttackUType` and `Teleport` look at every unit record of the array, without testing that the slot holds a
  live unit.
- **Q23**: `BuildingBuild`'s refund with a `buildLeft` above 1.0 gives a negative ftol, refunded as an amount near 2³².
- **Q24**: `BuildWeapon` publishes its REQUEST even when `economy.RequestGranted` answered 0.
- **Q25**: `Capture` at state 1 out of build range goes back to state 0, which claims and places its marker again,
  where `RepairUnit` waits 30 to 59 ticks in state 1.
- **Q26**: `Ground_Pickup` reads only the low byte of its code, so code bits above 0xff (0x10000 among them) are not
  seen.
- **Q27**: `HelpBuild`'s ring radius adds the footprint's z twice instead of squaring it.
- **Q28**: `MobileBuild` keeps its CLAIM and the snapped position when `units.CreateUnit` fails and retries every 300
  ticks; its blocked count is never reset after a success.
- **Q29**: `ReclaimUnit` state 5 asks `units.UnitRefGet` a second time after the DAMAGE (which weapons applies at once)
  and draws the reverse spray at the unit it answers, without testing for 0.
- **Q30**: `RepairPatrol` tests the alliance of the drawn unit again after the list required it, and its first reclaim
  row releases the target twice.
- **Q32**: `Resurrect` state 5 uses the spot of the cell `los_radar.GetOriginCellAtPosition` answers without testing
  that the cell has one, and a cell feature of 0xfffb or more ends the order with the new unit made.
- **Q33**: `VTOL_Follow` asks `units.CanRepair(U, 0)` when the guarded unit's first order has no target.
- **Q34**: `VTOL_HelpBuild` starts the building script with the heading to its target and no unit heading subtracted.
- **Q35**: `VTOL_LandIfCan` asks the ground height twice for one value (three times when the answers fall on both
  sides of the sea level).
- **Q36**: `VTOL_Landing` state 1 makes two landing pad searches, the second given the first's answer as its slot:
  a free slot is asked `map.IsPadSlotFree` a second time, and when the first finds none, `QueryLandingPad` is asked
  again.
- **Q37**: `VTOL_MobileBuild` state 3 runs the air build pass whether or not the unit is in its build stance; out of
  stance its `flags` also has 0x4 (0xe, then 0xf with the pass's wake).
- **Q38**: `VTOL_Patrol` state 2 attaches a new move goal and then, when the pad diversion happens, releases it at
  once (SET_TARGET, SET_TARGET 0, DESTROY_TARGET).
- **Q39**: `VTOL_RepairPatrol` state 1 asks `units.CanRepair(U, V)` a second time when the first answer is 0, or is
  not 0 with `V.buildLeft` not 0.0.
- **Q40**: `VTOL_SeekAttack` state 0 with a target whose attack order fails has result 1 with no climb and no draw.
- **Q41**: `VTOL_Unload` tests the footprint of `Order.target` but takes the altitude of state 1 from `Unit.cargo`'s
  definition.
- **Q42**: the health tests sign-extend a negative health and compare it unsigned, so a unit with negative health
  counts as repaired, and never as damaged.

## 7. Examples

From the kit's recordings. Each was recomputed from this spec's definitions, and every recorded value it states was checked.

- **`Apply`, STATUS_FLAGS**: `tests/replay/orders/orders-move-attack.txt`, call at tick 100 of `Apply`, `Order#3`, `flags`
  256: `Order#3.subFlags` 0 becomes 256, outcome 1. `orders-build-patrol.txt`, tick 150, two calls on `Order#7` with 32
  and 128: `subFlags` 160.
- **`Apply`, ADD**: `units-transport.txt`, call at tick 4410 of `Apply`, `Unit#251`: `type` 25 (`MobileBuild`), `queue` 0,
  no target, `pos` (209190912, 0, 48758784), `params` 246, 1. Imports: `services.SimAlloc 86`, `units.UnitRefInit`,
  `units.UnitRefSetValue`, `units.UnitRefSetUnit Order#5.link null` (no target), then the end of the unit's old first
  order (`movement.UnitTarget Unit#251 = null`, RELEASE and UNLINK, two `SchedPublish 3 12`, `SimFree`; no
  DESTROY_TARGET as its `attached` is 0). `Order#5`: `ticks` 246, `radius` 1, `created` 4410, `flags_42` 0x100508 + 0x1
  + 0x2000 (queue 0) + 0x1000 (inserted after the marked order: none was marked, so at the end of the emptied list) =
  1062153; outcome 1.
- **`GetOrderType`**: `units-transport.txt`, call at tick 4410, mode 14, `Unit#251`, no target, P (209190912, 0,
  48758784): `UnitDef.ids` not 0, `Unit.motion` not 0, `flags1` 0x800 clear: 25 (`MobileBuild`); no import.
- **`ExpandGroupCommand`, a patrol**: `orders-build-patrol.txt`, call at tick 150, mode 9, `units0` 1 (`Unit#1`), no
  target, C (45875200, 65536, 19660800): one member, *count* 1, *range* 3000; the member's whole parts are the centre,
  so *dx*, *dz* are its fractional parts (3125, 29025) and *e* = 0; `RepairPatrol`'s entry (0x412) has 0x2, so the
  COMMAND (44) has the position C + (3125, 0, 29025): the next `RunOrders` reads `Order#4` (kind 34) at (45878325,
  65536, 19689825). Result 1.
- **`ExpandGroupCommand`, an attack**: `orders-move-attack.txt`, call at tick 100, mode 3, `Unit#1` against `Unit#251`
  (mode 3 takes a target), `hasPos` 1: one COMMAND; `Attack_Chase`'s entry (0x280) lacks 0x2, so the position is C
  (221249536, 65536, 92274688); the order (`Order#3`) has `flags_42` 0x280 + 0x1 + 0x2000 + 0x1000 = 12929.
- **`RunOrders`, waiting**: `orders-move-attack.txt`, call at tick 98, `Unit#251`: `Order#2` (`Standby`), `flags` 65537,
  `wakeFrame` 117, `subFlags` 0: the tick is below the wake and nothing is pending: no import, no write.
- **`RunOrders`, the default order**: `orders-guard-attacked.txt`, call at tick 10, `Unit#2`: `Unit.list` 0, `Player#0`
  `active` 1 and `type` 1, `defaultmissiontype` 41: inserting a command of kind 41: `SimAlloc 86` and the three
  reference imports; `Order#3` with `kind` 41, `flags_42` 0x20000 + 0x4000 = 147456, `wakeFrame` 4294967295. It is not
  run.
- **`RunOrders`, results 5, 3 and a new first order**: `orders-air-repair.txt`, call at tick 852, `Unit#4`: `Order#2`
  (`SelfRepair`, state 1, `flags` 9, `wakeFrame` 852) wakes; *U* is repaired (health 280, `maxHealth` 280): result 1, and
  the order runs again in the same call (state 2: `RecordSimEvent 1 0 [4,10,40] 3`, result 5), its end
  (`movement.UnitTarget Unit#4 = null`, RELEASE, UNLINK, `SimFree`). `Order#3` (`VTOL_SeekGuard`, `flags` 0) runs state
  0 (`RandomInt 65536 = 9431`: `ticks` 9431, `radius` 1) and state 1: the ground allies (280 << 16 = 18350080) give
  `Unit#1`; `GetOrderType` mode 7 gives 49; `Order#5` (`VTOL_Follow`, target `Unit#1`) at the front (`Unit#4.list`
  `Order#5`, `Order#5.next` `Order#3`); result 3: `RandomInt 15 = 9`, `Order#3.flags` 1, `wakeFrame` 852 + 30 + 9 = 891.
  Then `Order#5` runs states 0, 1 and 2 (below).
- **`VTOL_Follow`**: the same call: state 0 (no announce, `flags_42` 512): CLAIM, ATTACH_TO_PIECE (the carrier
  `Unit#2`), SET_STATE_BITS; `UnitMotion#2.flags` is 2, so no flight branch; `RandomInt 65536 = 10037`. State 1:
  RELEASE. State 2: no attacker; `units.CanRepair Unit#4 Unit#1 = 0`; the help test reaches `units.CanRepair Unit#4
  null = 0`; the code is 0, no draw; `Unit.flags` lacks 0x80000000 (`armed`), so the distance is 0x1400000. S(10037, 0x1400000) =
  17146880 (entry 78, 6698) and C = 12075520 (entry 206, 4717), so the move goal is at (36700160 − 17146880, 65536,
  15728640 − 12075520) = `movement.NewPathOrderAt Order#5 (19553280,65536,3653120)`; SET_APPROACH_RADIUS (16); SET_TARGET
  (12). `Order#5`: `flags` 0xf9 = 249, `wakeFrame` 882, `state` 2.
- **`SelfRepair`**: `orders-air-repair.txt`, call at tick 848, `Unit#4`, `Order#2` (state 1, target `Unit#2`): health 276
  below 280; `UnitDef#18.buildRate` 200, R = 200 / 30 = 6.0 (f32:0x40c00000). After CLEAR_NET_DIRTY: BUSY (12),
  `units.AddRepairProgress Unit#2 Unit#4 f32:0x40c00000 = 1`, `scripts.GetNanoPiecePosition Unit#2` (32070011, 1585971,
  22010062), then `RecordSimEvent 3 0 [32070011,1585971,22010062,29360128,1376256,20971520,31457280,2397846,23068672,6]
  10`: the target box of `Unit#4` at (30408704, 1376256, 22020096) with `modelMinX`/`Z` −1048576, `modelMaxX`/`Z` 1048576,
  `modelMaxY` 1021590. `wakeFrame` 849.
- **`Standby`**: `orders-air-repair.txt`, call at tick 848, `Unit#251`, `Order#4` (state 1, woken): CLEAR_NET_DIRTY,
  `services.ScheduleTargetingFireAtWill Unit#251 = null`, `services.RandomInt 30 = 3`: `wakeFrame` 848 + 3 + 30 = 881.
- **`BuildingBuild`**: `orders-air.txt`, call at tick 4090, `Unit#2`, `Order#3` (state 3, target `Unit#6`):
  `UnitDef#14.buildRate` 100, B = 3.0 (f32:0x40400000); `units.AddBuildProgress Unit#2 Unit#6 f32:0x40400000 = 0` (no
  spray); `Unit#6.buildLeft` not 0: a wake delay of 1, `wakeFrame` 4091, result 2.
- **`GetBuilt`**: `orders-air.txt`, call at tick 4090, `Unit#6`, `Order#4` (state 2, the code with 0x8000): `wakeFrame`
  4090 + 30 = 4120, no import of its own. `orders-factory.txt`, call at tick 19, `Unit#2`, `Order#3` (state 0): CLAIM,
  `wakeFrame` 319, `flags` 0x8001, `state` 1.
- **`BeCarried`**: `units-transport.txt`, call at tick 4414, `Unit#1`, `Order#1` (state 1): `wakeFrame` 4424.
- **`MobileBuild`, state 0**: `units-transport.txt`, call at tick 4411, `Unit#251`, `Order#5` (`pos` (209190912, 0,
  48758784), footprint (5,5)): C = ((209190912 − 2621440 + 524288) >> 20, (48758784 − 2621440 + 524288) >> 20) = (197,
  44): `movement.NewPointMarker Order#5 (197,44) (5,5)`, SET_TARGET; the centre (5 + 394) << 19 = 209190912 is the old x;
  `flags` 224.
- **`MobileBuild`, state 1**: `orders-factory.txt`, call at tick 19, `Unit#1` (pos x 37291933, z 15245286, heading
  −13572), `Order#1` (`ticks` 68, `pos` (41943040, 65536, 16777216), `UnitDef#68` footprint (6,6)): placement
  `services.ScheduleMapCanPlaceUnitFootprint UnitDef#68 0 (37,13) 1 = 1`, CLAIM, the snap, `units.CreateUnit 0 68 ... 0
  1 0 = Unit#2`, `UnitRefSetUnit Order#1.link Unit#2`, speech `[1,9,30]`, the orders dirty mark `[1]`, the `GetBuilt`
  order for `Unit#2` (`Order#3`, `flags_42` 0x224 + 0x1 = 549, at the front of `Unit#2`'s list), `scripts.FindScript
  "StartBuilding" = 17`, START_SCRIPT (76), `network.SendScriptCall Unit#1 17 1 59405 0 0 0`: the build heading to
  (41943040, 16777216) is the low 16 bits of Head − (−13572), 59405. The order runs again: state 2, the build stance
  wait (10), `flags` 14.
- **`HelpBuild`, `RepairPatrol`**: `orders-build-patrol.txt`, call at tick 150, `Unit#1`, `Order#4` (`RepairPatrol`,
  state 0, no target): the patrol return order (`Order#5`, kind 34); state 1: `NewApproachRadius Order#4 45878325
  19689825 16`, `wakeFrame` 210; the repairable list `map.UnitsInRange (38341685,65536,16019809) 19005440 0 ? 256 = 2`
  (290 << 16) keeps `Unit#2`; `RandomInt 1 = 0`; `CanRepair` twice (mode 8, then the repair order's mode 8); the
  repair order with m 0: the move order `Order#6` (kind 26) and the help order `Order#7` (kind 23, `progress` 290,
  `start` (585, 244)) at the front; result 6, and `Order#7` runs: state 0, `movement.NewRingApproach Order#7 42467328
  17301504 107 47`: ftol(sqrt(25 + 5 + 5) × 16.0) = 94, / 2 = 47, outer 60 + 47. Tick 151, state 1: CLAIM,
  `FindScript` = 17, START_SCRIPT, `SendScriptCall Unit#1 17 1 7032 ...` (the build heading from (38341685, 16019809),
  heading −26558, to (42467328, 17301504)). Tick 148: the build step `units.AddBuildProgress Unit#1 Unit#2
  f32:0x41200000 = 1` (300 / 30 = 10.0).
- **`Move_Ground`**: `ui-right-click.txt`, call at tick 22, `Unit#1`, `Order#4` (state 0, `ticks` 0, `flags_42` 13315,
  `pos` (41943040, 65536, 19660800)): after CLEAR_NET_DIRTY, `RecordSimEvent 1 0 [1,5,0] 3`; `movement.NewApproachRadius
  Order#4 41943040 19660800 4`; SET_TARGET. `flags_42` 5123 (0x2000 cleared), `flags` 224, `state` 1.
- **`Attack_Chase`**: `orders-move-attack.txt`, call at tick 100, `Unit#1`, `Order#3` (target `Unit#251`, `ticks` 0):
  four runs, each after a CLEAR_NET_DIRTY: state 0 (`[1,5,0]`, `targeting.ChooseWeapon` = 0, `pos` the unit's, `radius`
  0); state 1 (nothing attached, so the release publishes nothing; `WeaponCanReachUnit Unit#1 Unit#251 0 = 0`: result 1);
  state 2 (`GetWeaponRange` = 200; step 0: `NewApproachRadius Order#3 221249536 92274688 200`, SET_TARGET; `radius` 1);
  state 3 (the reach answer 0: RELEASE; `flags` 0x100e8 + 0x1 = 65769, `wakeFrame` 130). `orders-guard-attacked.txt`,
  tick 20, `Unit#2`, `Order#7`: state 1 with the reach answer 1: CLAIM (0), CLAIM (2), SET_UNIT; `flags` 0x13808.
- **`Follow_Ground`**: `orders-guard-attacked.txt`, call at tick 22, `Unit#3` (footprint (2,2)), G `Unit#2` (footprint
  (2,2), pos (212336640, ·, 87162880)): speech `[3,5,9]`, RELEASE, `RandomInt 65536 = 48511` (signed −17025); `ticks`
  (2 + 2 + 2) << 4 = 96; `O.pos` = Step(−17025, 96 << 16) = (6279936, 0, 385536); state 1 takes the last row:
  `NewApproachRadius Order#3 218616576 87548416 48`; `wakeFrame` 52, `flags` 0x19.
- **`VTOL_Patrol`**: `orders-air.txt`, call at tick 4100, `Unit#5` (pos (64257579, ·, 25607439)), `Order#11` (state 0,
  `flags_42` 13331, `pos` (45907499, 65536, 26262799)): the patrol return order (`Order#12`, kind 56, `flags_42` 0x412,
  at the tail), `RecordSimEvent 1 0 [5,5,19] 3`, the climb preparation as CLAIM and SET_STATE_BITS (the flight mode is
  not 1), RELEASE. State 1: no import. State 2: Head(`U.pos`, `O.pos`) = A(18350080, −655360) = 16756; S(16756,
  0x1400000) = 20956160, C = −773120, so `movement.NewPathOrderAt Order#11 (24951339,65536,27035919)`;
  SET_APPROACH_RADIUS, SET_TARGET; not damaged; `services.ScheduleTargetingFireAtWill Unit#5 = null`; a wake delay of
  30. `flags` 0xe1, `wakeFrame` 4130, `flags_42` 37907 (0x2000 cleared, 0x8000 set), `next` `Order#12`.
- **`VTOL_Pickup`**: `orders-air.txt`, call at tick 4095, `Unit#3`, `Order#8` (state 1, target `Unit#1`, `attached`
  `PathOrder#1`): `movement.NewPathOrderToUnit Order#8 Unit#1 = PathOrder#4`, SET_ALTITUDE and SET_APPROACH_RADIUS (16
  each), then the release and the attach (SET_TARGET 0, DESTROY_TARGET, SET_TARGET: 12 each); `flags` 0x100e8, `state`
  2, `attached` `PathOrder#4`.
- **`VTOL_Standby`**: `orders-guard-attacked.txt`, call at tick 11, `Unit#5`, `Order#6` (state 0): CLAIM; `flags`
  0x10001, `wakeFrame` 12, `start` (3000,1300) (the unit's whole parts). Tick 12: the wake; state 1:
  `services.ScheduleTargetingFireAtWill Unit#5 = null`, result 1; state 2 in the same call takes the last row
  (`RandomInt 30 = 19`): `wakeFrame` 12 + 19 + 30 = 61, `state` back to 1.
- **`Ground_Unload`, two passes**: `units-transport.txt`, call at tick 4415, `Unit#3` (`netDirtyFlags` 4,
  `cobStateFlags` 2), `Order#3` (state 1, `flags` 1, `wakeFrame` 4415, `subFlags` 0). The first pass wakes it
  (`subFlags` 1): pending = (1 OR 4) AND 1 = 1, CLEAR_NET_DIRTY (12) with `mask` 1; the script busy wait (8) sets
  `flags` 8 OR 4 = 12, result 2. The second pass: pending = (0 OR 4) AND 12 = 4, so CLEAR_NET_DIRTY (12) with `mask` 4,
  whose subscriber clears the unit's bit at once (`o Unit#3.netDirtyFlags 0`); the run again sets `flags` 12, result
  2. The third pass finds `flags` 12 and nothing pending, and the call ends: `w Order#3.flags 12`, `w
  Order#3.wakeFrame 4294967295` (`subFlags` is back to 0, so it has no line).
- **`RunSecondaryOrders`**: every call of every file: `Unit.list2` is 0, no import, no write.

No recording covers: `Apply` of COMMAND, QUEUE_BUILD, STOP, BE_CARRIED, ATTACKED, PARALYZE or CANCEL, or a refused
event (outcome 0); GROUP_BUILD; `GetOrderFlags`; `MissionTypeFromName`; `GetOrderType` modes other than 14 as a call
of its own (modes 1, 3, 7 and 9 run inside `ExpandGroupCommand`, and 2, 7 and 8 inside runs); the results 4, 7, 8 and 9
of `RunOrders` (results 0, 1, 2, 3, 5 and 6 are recorded, 6 at `orders-build-patrol.txt` tick 150, `RepairPatrol`); a pending 0x10000; any order in `Unit.list2`; the end of an order with code 2 or with the building
script started; the runs of `Ready`, `Activate`, `AirStrike`, `AirToAir`, `AirToGround`, `AirToGroundHover`,
`Attack_Kamikaze`, `Attack_NoMove`, `AttackSpecial`, `AttackUType`, `BuildWeapon`, `Capture`, `Cloak_Off`, `Cloak_On`,
`Deactivate`, `Ground_Pickup`, `Guard_NoMove`, `MakeSelectable`, `Paralyze`, `Park`, `Patrol`, `QMove`,
`QPatrol`, `Reclaim`, `ReclaimUnit`, `RepairUnit`, `RepairUnitNoMove`, `Resurrect`, `SelfDestruct`, `SelfDestructFG`,
`Standby_Mine`, `Standing_FireOrder`, `Standing_MoveOrder`, `Stop`, `Suppress`, `Teleport`, `VTOL_Evade`,
`VTOL_GetRepaired`, `VTOL_HelpBuild`, `VTOL_LandIfCan`, `VTOL_Landing`, `VTOL_MobileBuild`, `VTOL_Move`, `VTOL_Reclaim`,
`VTOL_ReclaimUnit`, `VTOL_RepairPatrol`, `VTOL_RepairUnit`, `VTOL_SeekAttack`, `VTOL_Unload`, `Wait` and
`WaitForAttack`; and of the recorded kinds, the `BuildingBuild` states other than 3, the `GetBuilt` finish rule,
`SelfRepair` state 0, `BeCarried` state 0, the `MobileBuild` placement refusals and `CreateUnit` failure, the
`RepairPatrol` reclaim, `VTOL_Patrol` with a pad diversion or a target, `VTOL_SeekGuard`'s circle and the `Follow_Ground`
rows other than the last. `Ground_Unload` is recorded in state 1 only.

No recording calls 23 of the entry's 52 imports: `movement.NewPathOrderToUnitAt`, `movement.NewPathOrderToUnitAtAngle`,
`movement.NewAirManeuverOrder`, `units.FindUnitTypeId`, `units.CanLoad`, `units.CanReclaim`,
`units.GetFactoriesInRadius`, `units.UnitRefGet`, `scripts.QueryScript`, `scripts.GetPieceOffset`,
`scripts.GetPiecePosition`, `map.CanPlaceFootprintAt`, `map.IsPadSlotFree`, `map.GetGroundHeight`,
`network.SendFeatureReclaimed`, `network.SendScriptCallByName`, `los_radar.GetOriginCellAtPosition`,
`los_radar.GetVisibleEnemiesInRadius`, `features.GetCellFeature`, `features.FindFeatureAtPos`,
`targeting.GetWeaponTargetUnit` (`services.ScheduleTargetingTargetUnit`), `mission.GetGameType` and
`mission.GetMissionParams`. The recordings give no line form for their arguments or answers, so the recordings route
can't judge a call of them; only the scenarios would, and those are not played while the entry is blocked
("Gaps in the kit").

### Gaps in the kit

- **Class words.** The `Order` record and the movement targets start with a class word (`docs/replay-format.md`,
  "Polymorphic objects"), but no file of the kit gives the value a module writes there (the same gap the players and
  movement specs name). A module built from this spec can't make an `Order` that the replay or an engine slot
  recognises until the kit gives it, and STATUS_FLAGS's order limit can't test it.
- **Payloads by size.** The recordings carry each publish as `services.SchedPublish 3 <size>`, without its fields, so the
  gate compares an effect by its size and by what its subscribers write (the `o` lines after it), not by its payload.
- **The reclaim pick.** `RepairPatrol` and `VTOL_RepairPatrol` choose resources to reclaim through
  `map.PickRandomReclaimableResourcesInRadius`, which the manifest has under `signatures.map` but the entry does not
  import; no recording carries it, and no row of ABI 9.3's table maps a recorded call to it. The pick's choice and draws
  are map's; ozgb/byte-tactics-moddable#1208 adds the import to the entry, and until then a module can't make the
  call (section 2, "Two calls that are not in the entry").
- **`economy.RequestGranted`.** `BuildWeapon` state 1 asks it (recorded as `services.ScheduleEconomyRequestGranted`), but
  the entry has no `economy` import group, so a module can't make the call until the entry lists it.
- **A blocked entry.** `blocked` is `true` in the kit's manifest, so the entry has no `fuel_budget` and no `size_budget`
  until ozgb/byte-tactics-moddable#1208 sets them; the gate judges a module of it on the recordings route (ABI 4.2, section 15), with `scenarios: not_judged`.
- **Unrecorded imports.** The 23 imports of section 7 that no recording calls have no recorded argument or answer form,
  so no check judges them while the entry is blocked.
- **Fields no recording carries.** No recording has a line of a `UnitTypeSet` (`bits`), `Cell` (`feature`, `offsetX`,
  `offsetY`, `spot`), `Feature` or `FeatureSpot` field. An object an import answers starts zeroed in the replay
  (`docs/replay-format.md`, "An object returned as its offset"), so in `ui-right-click.txt` the `Cell#4072` that
  `los_radar.GetMapCellAtPosition` answers has `feature` 0 and every other field 0; the type set tests of ATTACKED and
  the guard kinds, and the feature tests of section 4, are judged by no recorded value.
- **The self-destruct slots of 6 and 7.** A `SelfDestruct` order of a unit type with a `selfdestructcountdown` of 6 or
  7 speaks with a voice slot the kit does not give.
- **Unset position words.** "The site point" and "the feature box" pass `map.GetGroundHeight` a position whose y word
  was never written; a replay that compared that word could not be matched from this spec.
- **The float precision of unrecorded paths.** No recording runs `BuildingBuild`'s refund, `BuildWeapon`'s charges,
  the capture time, `Reclaim`'s time, `AirStrike`'s state 4, the repair patrols' thresholds or `Resurrect`'s countdown, so
  the precision section 5 states for them (double, no rounding to float) is settled by no recording and is judged only by scenarios.

## 8. Kit version

Kit `interfaces-v68` and `reference-traces-v55`, the versions the gate judges against. Interface `orders` 6.2. The
imports are against `movement` 5, `units` 7, `scripts` 3, `weapons` 4, `map` 9, `network` 2, `los_radar` 5,
`features` 5, `targeting` 4, `mission` 2 and `services` 4.
