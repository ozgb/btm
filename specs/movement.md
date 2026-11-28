# movement

## 1. Role

Movement moves units. Each unit that can move has a motion record (its velocity, speed, turn, flight mode and a
goal), and the goal is what the motion steers to: a short route of waypoints for a ground unit, a point and a
heading for an aircraft. Orders give a goal a target (a point, a unit to follow, a disc, a ring or a footprint
outline), which movement builds for them and frees when they ask. Once a tick, for each unit with a motion,
movement steps the goal towards its target, steers (turning, accelerating, braking, climbing), moves the unit,
places it on the map when it changes cell, and starts the scripts that show it moving. It also takes pathfinding's
routes for its goals and reports an order's progress back to orders.

## 2. Interface

### Slots (`interfaces/movement.h`, version 5.0)

| Slot | When the engine calls it |
| --- | --- |
| `Apply` | for each event of movement's inbox, in the order it was published (`docs/schedule.md`, "Routes") |
| `UpdateMotion` | once a tick for each unit that has a motion, in the unit's update, after its orders |
| `ApplyPathEvent` | in the players phase, for each event pathfinding's `Poll` gives, with the waypoints it copied |
| `HasNetState` | when the network sends unit states (a query) |
| `NewPathOrderAt`, `NewPathOrderToUnit`, `NewPathOrderToUnitAt`, `NewPathOrderToUnitAtAngle`, `NewAirManeuverOrder`, `NewApproachRadius`, `NewRingApproach`, `NewPointMarker` | when an order needs a target; the answer is the new target |
| `TargetType` | when orders asks what kind a target is (a query) |
| `UnitTarget` | when orders asks what a unit's goal steers to (a query) |
| `GetMotionState`, `ReadNetState`, `WriteNetState`, `LoadTarget`, `SaveTarget`, `WriteState` | engine slots (below) |

The six engine slots are the entry's `engine` slots: the module does not export them, and the host calls the
engine's native slot, which reads and writes the module's records in the block as section 3 lays them out (ABI
section 6, "Engine slots", and section 9.3, "What stays in the engine"). So the module's records must be exactly
the `layout.json` records of section 3, at the offsets it gives, for those slots to read.

### The manifest entry

The kit's manifest (`wasm/modules.json`) has the entry `movement`, and the module must match it exactly (ABI 4.2
and 9.1):

- `name` and `interface`: `movement`.
- `majors`: `movement` 5, `units` 7, `map` 9, `scripts` 3, `pathfinding` 4, `services` 4.
- `data_base` 622592 (0x98000) and `table_base` 464: the module's window of `window` (32768) bytes and its
  `table_slots` (32) table entries (ABI 2.2 and 2.3).
- `blocked`: `true` in the kit's manifest. A blocked entry has no `fuel_budget` and no `size_budget` (ABI 4.2),
  and the gate judges a module of it on the recordings route, with `budgets: not_judged` (ABI section 15).
- `exports`, in this order: `Apply`, `UpdateMotion`, `ApplyPathEvent`, `HasNetState`, `NewPathOrderAt`,
  `NewPathOrderToUnit`, `NewPathOrderToUnitAt`, `NewPathOrderToUnitAtAngle`, `NewAirManeuverOrder`,
  `NewApproachRadius`, `NewRingApproach`, `NewPointMarker`, `TargetType`, `UnitTarget`.
- `engine`: `GetMotionState`, `ReadNetState`, `WriteNetState`, `LoadTarget`, `SaveTarget` and `WriteState`, each
  with the entry's reason.
- `data`: `g_game`, `g_simGlobals`, `g_extTdf`, `g_blockBase` (ABI 8.2). The module reads `g_game` (the `Game`
  record) and `g_blockBase` (to turn a block offset into an address and back).
- `publishes`: `{"128": 8, "160": 8, "161": 8, "336": 16, "337": 12, "338": 16, "339": 20, "340": 20, "341": 12,
  "342": 32, "343": 12, "344": 24, "345": 16, "346": 8, "347": 12}`: los_radar's UPDATE_UNIT (`LosRadarUnit`), map's
  ADD_UNIT and REMOVE_UNIT (a `MapEventHeader` alone), and movement's own twelve effects (section 4, "Effects").
  The route table (`data/routes.csv`) carries each from movement's outbox.
- Fuel: the module meters itself by rule version 1 (ABI section 11) and exports `__fuel_set` and `__fuel_get`.
- Slot kinds:

| Slot | Kinds |
| --- | --- |
| `Apply` | `[linear, int] -> int` |
| `UpdateMotion` | `[block] -> void` |
| `HasNetState`, `UnitTarget` | `[block] -> int` |
| `ApplyPathEvent` | `[linear, int, linear] -> void` |
| `NewPathOrderAt` | `[int, linear] -> int` |
| `NewPathOrderToUnit` | `[int, block] -> int` |
| `NewPathOrderToUnitAt` | `[int, block, linear] -> int` |
| `NewPathOrderToUnitAtAngle` | `[int, block, int] -> int` |
| `NewAirManeuverOrder`, `NewPointMarker` | `[int, linear, linear] -> int` |
| `NewApproachRadius` | `[int, int, int, int] -> int` |
| `NewRingApproach` | `[int, int, int, int, int] -> int` |
| `TargetType` | `[int] -> int` |

An order and a target are `int`: block offsets (0 for none) that the module stores and returns. A `short angle`
is the `int` kind, sign-extended from its low 16 bits (ABI 7).

- Imports, every one listed and nothing else:

| Import | Kinds | Used for |
| --- | --- | --- |
| `units.UnitRefUnlink` | `[block] -> void` | unlinking a move target's unit reference before the target is freed |
| `units.UnitRefInit` | `[block, block, block] -> void` | building a move target's unit reference |
| `units.UnitRefSetUnit` | `[block, block] -> void` | never called by an exported slot |
| `map.CanPlaceUnitFootprint` | `[block, int, linear, int] -> int` | whether a unit fits at the cell it moves into |
| `map.GetGroundHeight` | `[linear] -> int` | the ground under a move target that takes an altitude |
| `scripts.GetPiecePosition` | `[block, int, linear] -> void` | where a carried unit sits, and where a followed unit is |
| `scripts.GetPieceAngles` | `[block, int, linear] -> void` | the angles of a carried unit |
| `pathfinding.Submit` | `[linear] -> void` | asking for a route for an AI search goal |
| `pathfinding.Cancel` | `[int] -> void` | dropping an AI search goal's request |
| `services.SimAlloc` | `[int] -> linear` | a motion, a goal or a target |
| `services.SimFree` | `[linear] -> void` | freeing them |
| `services.Emit` | `[linear, int] -> void` | every publish of section 4, "Effects" |

The recordings carry `map.CanPlaceUnitFootprint` as `services.ScheduleMapCanPlaceUnitFootprint` and `Emit` as
`services.SchedPublish 14 <size>` (ABI 9.3, "Routes in the recordings"; 14 is movement's outbox).
`units.UnitRefSetUnit` is in the entry because the engine slot `LoadTarget` uses it; the module imports it (ABI
9.1: every listed import is imported) and never calls it.

- Hooks: every slot of `interfaces/movement.h` says "Hooks: none", and the module raises none.

### Arguments outside the block

All in the caller's memory (`linear`), little-endian, read in place:

- **The event of `Apply`**: a `MovementEventHeader` (`kind`, `unit`) and its kind's payload, `size` bytes. The
  payloads and their sizes: NEW and DESTROY the header alone (8), LOAD `MovementLoadState` (48), SET_MODE and
  SET_FLIGHT_MODE `MovementSetMode` (12), SET_SPEED `MovementSetSpeed` (12), SET_TARGET and DESTROY_TARGET
  `MovementTarget` (12), SET_ALTITUDE, SET_APPROACH_RADIUS and SET_HEADING `MovementTargetValue` (16). The recordings
  write the fields as `a event.kind`, `a event.unit`, `a event.target`, `a event.mode`, `a event.value`, and the size
  as `a size`.
- **The event of `ApplyPathEvent`**: a `PathfindingEventHeader` (20 bytes: `kind`, `unit`, `requester`, `target`,
  `tick`) and its payload (`PathfindingStarted`, 24 bytes; `PathfindingFinished`, 28 bytes), `size` bytes, and
  `waypoints`, an array of `PathPoint` (4 bytes each, `x` then `y`, signed 16-bit) of at least the finished event's
  `count` entries; the module reads at most the first 20.
- **`pos`** of `NewPathOrderAt` and `NewPathOrderToUnitAt`, and **`from`**, **`to`** of `NewAirManeuverOrder`: a
  `Vec3` (three signed 32-bit words). **`pos`** and **`size`** of `NewPointMarker`: a `Point16` each (`x`, `y`,
  signed 16-bit).
- **What the module passes to imports**, in its own memory: a 56-byte `PathRequest` for `pathfinding.Submit`
  (section 4, "Asking for a route"), every byte written; a `Point16` cell for `map.CanPlaceUnitFootprint`; a `Vec3`
  that `scripts.GetPiecePosition` writes and a 6-byte `Angles16` (`bank`, `heading`, `pitch`) that
  `scripts.GetPieceAngles` writes; every effect payload of `Emit`. `map.GetGroundHeight` is given the address of the
  move target's own `pos` in the block, and reads its `x` and `z` (the recordings write only those, `pos=(x,z)`).

### Events

None. No slot of this module records a sim event (kinds are listed in `docs/sim-events.md`). What the effects
start in other modules (a script, map's and los_radar's updates) records its own.

## 3. State

### Owned

Movement owns three families of records, all allocated with `services.SimAlloc` (zeroed) of exactly the record's
`layout.json` size and freed with `services.SimFree`. Each `Ref<...>` field is a block offset (0 for none). The
replay names are the recordings' field names.

**`UnitMotion`** (47 bytes), one per unit that moves, its offset in `Unit.motion`:

| Field | Bytes | Replay name | Meaning |
| --- | --- | --- | --- |
| `obj` | 0, `Ref` | `obj` | the goal |
| `movementClass` | 4, `Ref` | `movementClass` | the unit type's `UnitDef.movementclass`, set at NEW |
| `velocity` | 8, `Vec3` | `velocityX`, `velocityY`, `velocityZ` | per tick, world units with 16 fractional bits |
| `p2` | 20, `Vec3` | `p2X`, `p2Y`, `p2Z` | the smoothed acceleration that tilts an aircraft |
| `speed` | 32, int | `speed` | |
| `turn` | 36, signed 16-bit | `turn` | the turn of the last tick |
| `pathLockStamp` | 38, int | | map's field: the module never writes it |
| `lastMoveTick` | 42, int | `lastMoveTick` | the last tick the unit moved |
| `flags` | 46, byte | `flags` | bits 0-1 `mode`, the flight mode (0 to 3; 1 grounded, 2 flying); bit 2 `flag`, *blocked*: the unit's last cell change did not fit; bits 3-7 stay 0 |

**The goals** (base `PathGoal`): word 0 is the goal's class word (see "Class words", below), then `target` (4,
`Ref` to a target) and `owner` (8, `Ref<Unit>`, the unit). A unit's goal class is fixed at NEW:

| Goal | Bytes | Fields after the base |
| --- | --- | --- |
| `AiSearchGoal` | 101 | `points` (12, 20 `Point16`, whole world units: 16 per cell, so `x` is a position's `x >> 16`), `count` (92, int), `repathClaimTick` (96, unsigned), `flags` (100, byte: bit 0 `active`, *has a route*; bit 1 `flag_1`, *wants a search*; bit 2 `flag_2`, the blocked bit last sent over the network; bit 3 `flag_3`, *changed since sent*) |
| `PatrolGoal` | 28 | `points` (12, 3 `Point16`), `count` (24, int) |
| `LiteGoal` | 40 | `pos` (12, `Vec3`), `vel` (24, `Vec3`), `heading` (36, signed 16-bit), `field_26` (38, stays 0), `netDirtyFlags` (39, byte: bit 0 `dirty`, bits 1-2 `mode`) |
| `PackedPosGoal` | 39 | `pos`, `vel`, `heading`, `field_26` as `LiteGoal` (the layout's `PackedGoal`) |

The replay names `points`, `count`, `repathClaimTick`, `flags`, `posX` to `posZ`, `velX` to `velZ`, `heading`,
`netDirtyFlags`; `points` is written as its bytes (`x:...`).

**The targets** (base `OrderFx`): word 0 is the class word, then `source` (4, `Ref<Order>`, the order the target
was built for). The kind is the class: 2 move target, 3 air maneuver target, 4 disc, 5 ring, 6 footprint marker
(the numbers `TargetType` gives; 1, a plain target, is never built by this module).

| Target | Bytes | Fields after the base |
| --- | --- | --- |
| `PathOrder` (2) | 54 | `flags` (8, unsigned 16-bit), `approachRadius` (10), `altitude` (12), `heading` (14), `piece` (16), each signed 16-bit; `unit` (18, `Ref<Unit>`, the unit the order moves); `ref` (22, a 16-byte `PathOrderAttach`, units' unit reference: its `owner` at 26 is the followed unit); `pos` (38, `Vec3`); `followOffset` (50, int) |
| `AirManeuverOrder` (3) | 44 | `flags` (8, unsigned 16-bit, bit 0 `headingSet`), `target` (10, `Vec3`), `other` (22, `Vec3`), `saveOnly22` (34), `heading` (36, unsigned 16-bit), `value_26` (38), `self` (40, `Ref<Unit>`) |
| `ApproachRadius` (4) | 20 | `pos` (8, `Point16`, a cell), `radius` (12, int), `radiusSq` (16, int) |
| `RingApproach` (5) | 28 | `pos` (8, `Point16`), `inner` (12), `outer` (16), `innerSq` (20), `outerSq` (24), ints |
| `PointMarker` (6) | 24 | `x1` (8), `x2` (12), `y1` (16), `y2` (20), ints: the first and last column and row of a rectangle of cells |

The bits of a move target's `flags`: 0x1 *follows* `ref.owner`; 0x2 *keeps an offset* from it; 0x4 *matches its
heading*; 0x8 *altitude set*; 0x10 *approach radius set* (`radiusFlag`); 0x20 *fixed position*; 0x40 *heading
set* (`headingFlag`); 0x80 *stopped following*. The replay names the reference's owner `ref.owner`, and the
position `posX`, `posY`, `posZ`.

**`Unit.motion`** is written by movement only (NEW and DESTROY).

**Class words.** Word 0 of a goal and of a target says its class, and is what the recordings name an object by
(`LiteGoal#1`, `ApproachRadius#2`: `docs/replay-format.md`, "Polymorphic objects") and what the engine slots read
to know the class. The module writes each class's word when it builds the object. The kit does not give the value of
any class word ("Kit gaps", section 7). The embedded `PathOrderAttach` has a class word of its own, which
`units.UnitRefInit` writes.

### Read

| Field | Owner | Use |
| --- | --- | --- |
| `Game.ticks`, `Game.seaLevel` (unsigned byte), `Game.gravity`, `Game.overflowBucket` | game | the tick; the water line; the tilt; whether a unit is off the map |
| `Game.units`, `Game.unitsEnd` | units | the unit table, units of 280 bytes, for the limits of section 4 |
| `Unit.pos` (`posX`, `posY`, `posZ`; `xWhole`, `yWhole`, `zWhole` are the high halves), `heading`, `pitch`, `cell`, `footprint` (`Point16`), `flags`, `carrier`, `transportPiece` (byte), `def`, `player`, `list`, `spatialBucket`, `id`, `weapons[0].weapon` | units | as section 4 says; the unit's mode is `Unit.flags` bits 0-1 and its move rate bits 2-3 |
| `UnitDef.maxvelocity`, `brakerate`, `acceleration`, `bankscale`, `pitchscale`, `moverate1`, `moverate2`, `movementclass`, `turnrate` (unsigned 16-bit), `altitude` (signed 16-bit), `draft` (byte), `flags1` | units | bit 11 (`flying`): an aircraft; bit 19 (`b19`): stays at or above the water line when carried; bit 22 (`b22`): a sea unit; bits 12 and 19 (mask 0x81000): no slowdown under water |
| `WeaponDef.range` | weapons | a follow offset |
| `Player.active`, `Player.type` | players | type 1 or 2 is a player whose units check the map; type 3 gets the remote goals |
| `Order.unit`, `Order.flags_42` (bit 0x800000) | orders | the unit an order's target reports to; whether a goal may take a straight route |
| `SpatialBucket.bytes` (byte 1: the bucket's ground height in whole units) | map | the cruise height |
| a carrier's `UnitMotion.speed` and `velocity` | movement | a carried unit takes them |

### Between ticks

Nothing outside the block. Every motion, goal and target lives in the block from its allocation to its `SimFree`.
A goal keeps the target offset it was given until it is given another; freeing a target does not clear it.

## 4. Behaviour

Within one call, only the order of import calls is observable. The block writes are compared at the end of the
call, so the order they are listed in carries no meaning. Several effects are delivered before `Emit` returns
(`data/routes.csv`, synchronous): a read listed after such an effect sees what its subscriber wrote. Where it
matters below, a read says "as the block now holds it".

### Definitions

The arithmetic is in section 5. For a unit *u*, *d* is `u.def`, and *M* is `u.motion`.

- **S(a, s), C(a, s)**: the scaled sine and cosine of the kit's runtime (`runtime/vectors/README.md`, "Scaled sine
  and cosine"), a 16-bit angle *a* and a scale *s*. **Step(a, s)** = (−S(a, s), 0, −C(a, s)): a step of length *s*
  along heading *a*.
- **A(x, z)**: the angle of a vector (README, "Angle of a vector"), kept as its low 16 bits, signed.
  **Head(p, q)** = A(p.x − q.x, p.z − q.z): the heading from position *p* towards *q*.
- **Rot(a, x, z)**: rotating a point (README, "Rotating a point").
- **hyp(x, y)**: the kit's `hypot` of the two values as doubles. **ftol(v)**: the kit's `ftol`.
- **Cell(p, fp)**: the cell of a position for a footprint *fp*: (`(p.x − fp.x × 0x80000 + 0x80000) >> 20`,
  `(p.z − fp.y × 0x80000 + 0x80000) >> 20`), each kept to signed 16 bits. **Centre(c, fp)** = (`(c.x × 2 + fp.x)
  << 19`, `(c.y × 2 + fp.y) << 19`), the *x* and *z* of the centre of cell *c*.
- **Cruise(u)**: `(d.altitude + Game.seaLevel) << 16` when `d.flags1` has bit 22, else `(d.altitude + b) << 16`,
  *b* being byte 1 of `SpatialBucket.bytes` of `u.spatialBucket`.
- **Waypoint *i*** of a goal with `points` and `count`: point *j* = min(*i*, `count` − 1), as the position
  (`x << 16`, 0, `y << 16`).
- **Report(t, f)**: when target *t*'s `source` is not 0, publish ORDER_FLAGS with `unit` the `Order.unit` of the
  source, `order` the source, and `flags` *f*. A target with no source reports nothing.

### Targets

Each target kind answers five questions. Asking can write the target and make imports; the tables say what.

**Its position, Pos(t)**, which fills *x*, *y* and *z* of an output or fails:

| Target | Pos(t) |
| --- | --- |
| move target, `flags` has 0x1 and not 0x80, and `ref.owner` is 0 or its `spatialBucket` is `Game.overflowBucket` | fails; nothing written |
| move target, `flags` has 0x1 and not 0x80, otherwise | `scripts.GetPiecePosition(ref.owner, piece)` gives *q*; when `flags` has 0x2, *q* becomes *q* − Step(*a*, `followOffset`), *a* being `ref.owner`'s `Unit.heading`, plus `heading` when `flags` has 0x40 (16-bit sum); then *q.y* += `altitude << 16`; `pos` becomes *q* with *y* at most 0x1ff0000; the answer is `pos` |
| move target, otherwise | when `flags` lacks 0x8, `pos.y` becomes Cruise(`unit`); then `pos.y` is kept to at most 0x1ff0000; the answer is `pos` |
| air maneuver | the answer is `target` as it was; then `target.x` += `other.x` and `target.z` += `other.z`; *h* = Head((0, 0, 0), `other`) as unsigned 16-bit; when `headingSet` and *h* ≠ `heading`: *δ* = `heading` − *h* (signed 16-bit), clamped to ±(`self`'s `turnrate` >> 3), and (`other.x`, `other.z`) becomes Rot(−*δ*, `other.x`, `other.z`), `other.y` 0 |
| disc | *x* and *z* are Centre(`pos`, the source's unit's `footprint`); *y* is not written |
| ring | *x* and *z* start as Centre(`pos`, the source's unit's footprint); *a* = Head(that unit's `pos`, that centre); then *x* += S(*a*, *r*) and *z* += C(*a*, *r*), with *r* = ((`inner` + `outer`) / 2) << 16; *y* is not written |
| footprint marker | *x* = `(fp.x + m × 2) << 19` with *m* = (`x1` + `x2`) / 2 kept to signed 16 bits, *z* = `(fp.y + y2 × 2) << 19` with `y2` kept to signed 16 bits (Q4), *fp* the source's unit's footprint; *y* is not written |

**Whether it holds a cell, In(t, x, y)**, with no writes: a disc, when (*x* − `pos.x`)² + (*y* − `pos.y`)² ≤
(`radius` / 16)²; a ring, when that square is at least (`inner` / 16)² and at most (`outer` / 16)²; a footprint
marker, when the cell is on the rectangle's border (*x* is `x1` or `x2` and `y1` ≤ *y* ≤ `y2`, or *y* is `y1` or
`y2` and `x1` ≤ *x* ≤ `x2`); a move or air maneuver target never.

**Whether a unit has reached it, Reached(t, u)**: for a disc, ring or footprint marker, In(t, `u.cell.x`,
`u.cell.y`). For a move target: Pos(t) (with its writes and import) gives *at*; *r* = hyp(`u.pos.x` − *at.x*,
`u.pos.z` − *at.z*) rounded to a float, times 2⁻¹⁶. Then, in order: with 0x10, the answer is `approachRadius` > *r*;
else *r* > 0.5 gives no; 0x1 with `ref.owner` 0 gives no; 0x4 with `u.heading` ≠ `ref.owner`'s heading (unsigned
16-bit) gives no; 0x8 with |`u.pos.y` − *at.y*| > 0x10000 gives no; otherwise yes. When Pos(t) fails here (a
followed unit that is gone or off the map), the answer is no ("Gaps in the kit"). For an air maneuver: yes when
hyp(`u.pos.x` − `target.x`, `u.pos.z` − `target.z`) rounded to a float, divided by 65536, is below 48; else yes when
`headingSet` and Head((0, 0, 0), `other`) is `heading`; else no. It writes nothing.

**Whether it stays after it is reached, Keep(t)**: a move target whose `flags` has 0x1 and whose `ref.owner` is
not 0; no other.

**The heading it wants, Want(t)**, which gives a heading or none, with no writes: a move target whose `flags` has
0x4 or 0x80 and whose `ref.owner` is not 0 gives Head(`unit`'s `pos`, `ref.owner`'s `pos`) when `flags` has 0x2,
else `heading` when it has 0x40, else `ref.owner`'s `heading`; a move target with 0x40 otherwise gives `heading`; an
air maneuver gives Head(`self`'s `pos`, `target`) (unsigned); every other gives none.

**Its area, Area(t)**, a `PathGoalArea` (`interfaces/pathfinding.h`): a disc gives kind 1 (`PATH_GOAL_CIRCLE`),
`centre` `pos`, `radius[0]` `radius`; a ring kind 2, `centre` `pos`, `radius[0]` `inner`, `radius[1]` `outer`; a
footprint marker kind 3, `left` `x1`, `right` `x2`, `top` `y1`, `bottom` `y2`; every field not named is 0, and a
move or air maneuver target gives all zeros (kind 0).

### Asking for a route

**Ask(g)**, for an AI search goal *g* with a target: `pathfinding.Submit` of a `PathRequest` whose `unit` is
*g*'s `owner`, `notBefore` `repathClaimTick` + 60 (unsigned), `goal` Area(target), `requester` *g*'s own block
offset, `target` the target, and `useStart` and `start` 0. The recordings write it as
`(unit,notBefore,kind,(centre),radius0,radius1,left,right,top,bottom,requester,target,useStart,(start))`.

### Goals

Each goal class answers two things: what giving it a target does (**Give(g, t)**, *t* a target or 0) and what
its tick does (**Tick(g)**). *o* is the goal's `owner`.

**Give(g, t)**:

- **AI search goal.** `pathfinding.Cancel(g)` (its block offset). Then Report(old target, 0x80) when it had one.
  `target` becomes *t*, and with *R* standing for *has a route* and *W* for *wants a search*, both start cleared;
  when *t* is not 0, *W* is set, and:
  1. when `count` ≥ 3 and In(*t*, last point's `x >> 4`, last point's `y >> 4`) (arithmetic shifts), *R* is set and
     *W* cleared;
  2. otherwise, when Pos(*t*) gives *p*: when `count` ≥ 3 and 2 × ftol(hyp(last.x << 16 − *p.x*, last.y << 16 −
     *p.z*)) < ftol(hyp(`o.pos.x` − *p.x*, `o.pos.z` − *p.z*)), *R* is set; when *R* is still clear and *o*'s
     `Unit.list` is an order whose `flags_42` lacks 0x800000, `count` becomes 2, point 0 (`o.xWhole`, `o.zWhole`),
     point 1 (*p.x* >> 16, *p.z* >> 16) kept to signed 16 bits, and *R* is set. The other points are left as they
     were.

  Then `repathClaimTick` becomes 0 when it is at most `Game.ticks` − 10 (both unsigned 32-bit, so in the first ten
  ticks it always does); `active` is *R*, `flag_1` is *W*, `flag_3` is set; and when *W* is set, Ask(g). Imports in
  order: `Cancel`, the report, the imports of Pos(*t*), `Submit`.
- **Lite goal.** Report(old target, 0x80) when it had one; `target` becomes *t*; `dirty` is set.
- **Patrol goal and packed position goal.** Report(old target, 0x80) when it had one; `target` becomes *t*.

**Tick(g)**:

- **AI search goal.** First, when it has a target *t* and Reached(*t*, *o*): Report(*t*, 0x20), then, unless
  Keep(*t*), Give(*g*, 0). Then, when `count` ≥ 2 and (`o.xWhole` − point 1's `x`)² + (`o.zWhole` − point 1's `y`)²
  ≤ 25: the points move down by one (point *i* becomes point *i* + 1 for *i* below `count` − 1; the old last point
  stays where it was, Q6), `count` drops by one, `active` is cleared when `count` is now below 2, and `flag_3` is set.
  Then, when it has a target (as `target` now is) and either *M*'s blocked bit is set or `count` < 2: `flag_1` is
  set and Ask(g).
- **Lite goal.** `dirty` is set when *M*'s `mode` differs from the goal's `mode` bits. Then as a packed position
  goal.
- **Packed position goal.** Nothing without a target *t*. Otherwise: *old* = `pos`; Pos(*t*) writes `pos` (when it
  fails, `pos` stays *old*); `vel` becomes `pos` − *old*. *dist* = ftol(hyp(`o.pos.x` − `pos.x`, `o.pos.z` −
  `pos.z`)). When *dist* > 0xa00000, `pos.y` becomes Cruise(*o*). Then: when *dist* > 0x1400000, `heading` becomes
  Head(`o.pos`, `pos`); otherwise, when Want(*t*) gives a heading, `heading` becomes it; otherwise, when *dist* >
  0x100000, `heading` becomes Head(`o.pos`, `pos`). Then, when Reached(*t*, *o*): Report(*t*, 0x20), and unless
  Keep(*t*), Give(*g*, 0) (the lite goal's Give for a lite goal).
- **Patrol goal.** Nothing.

### `Apply(event, size)`

A size below 8 gives 0. An unknown kind (325 included) gives 0, and so does a size below the kind's payload
(section 2). Then the event's limits, which a malformed event fails, with outcome 0 and no change, without a trap
(ozgb/btm#25):

- a *unit* is valid when it is not 0, at least `Game.units`, below `Game.unitsEnd`, and a multiple of 280 bytes from
  `Game.units`;
- a *target* is valid when it is not 0 and its class word is one of the five target kinds' (section 3);
- for DESTROY_TARGET, SET_ALTITUDE, SET_APPROACH_RADIUS and SET_HEADING, `target` must be valid, and `unit` is not
  looked at; for the others, `unit` must be valid and, but for NEW, have a motion; SET_TARGET's `target` must be 0
  or valid;
- a `mode` (SET_MODE, SET_FLIGHT_MODE, and LOAD's `state.mode`) must be 0 to 3, and LOAD's `state.flag` 0 or 1.

The recordings hold no event outside these limits, and no outcome 0.

| Kind | What it does | Outcome |
| --- | --- | --- |
| NEW 320 | `SimAlloc(47)` for the motion, then `SimAlloc` of the goal: when `u.player` has `active` not 0 and `type` 3, a packed position goal (39) for an aircraft and a patrol goal (28) otherwise; for every other player a lite goal (40) for an aircraft and an AI search goal (101) otherwise. `u.motion` becomes the motion (a motion it had is not freed). The motion: `obj` the goal, `movementClass` `d.movementclass`, `mode` 1, everything else 0. The goal: its class word, `owner` *u*, `target` 0; an AI search goal `flag_3` set (`flags` 8), the rest 0; a patrol goal all 0; a lite or packed position goal `pos` `u.pos`, `vel` 0, `heading` `u.heading`, and a lite goal `dirty` set (`netDirtyFlags` 1) | 1 |
| DESTROY 321 | when the motion has a goal: an AI search goal first makes `pathfinding.Cancel(goal)`; a packed position goal with a target first frees it as DESTROY_TARGET does and sets its `target` 0; then `SimFree(goal)`. Then `SimFree(motion)`, and `u.motion` becomes 0 | 1 |
| LOAD 322 | `velocity` and `p2` (from `smoothedAcceleration`) take the state's three words each, `speed` its `speed`, `turn` the low 16 bits of `turn`, `mode` and the blocked bit `mode` and `flag` | 1 |
| SET_MODE 323 | `mode` becomes the event's `mode`, nothing else | 1 |
| SET_SPEED 324 | `speed` becomes the event's `speed` | 1 |
| SET_TARGET 326 | Give(*M*'s goal, `target`) | 1 |
| DESTROY_TARGET 327 | a move target first makes `units.UnitRefUnlink` of its `ref`; then `SimFree(target)`. No goal's `target` changes | 1 |
| SET_FLIGHT_MODE 328 | when `mode` differs from the event's *m*: for *m* 1, `speed` and `velocity` become 0, then Tilt(*u*, (0, 0, 0)) (below), then SET_STATE_BITS with `mask` 1 and `set` 0; for any other *m*, SET_STATE_BITS with `mask` 1 and `set` 1; then `mode` becomes *m*. When it is the same, nothing | 1 |
| SET_ALTITUDE 329 | a move target: `flags` gains 0x8, `altitude` becomes the low 16 bits of `value`, and when `flags` has 0x20, `pos.y` becomes (*g* + `value`) << 16, at most 0x1ff0000, where *g* is `map.GetGroundHeight(&pos)` when that is above `Game.seaLevel`, and `Game.seaLevel` otherwise (Q1). An air maneuver target: nothing | 1 for those two kinds; 0 for the other kinds, with no change |
| SET_APPROACH_RADIUS 330 | a move target: `flags` gains 0x10 and `approachRadius` becomes the low 16 bits of `value` | 1; 0 for another kind |
| SET_HEADING 331 | a move target: `flags` gains 0x40 and `heading` becomes the low 16 bits of `value` | 1; 0 for another kind |

### `UpdateMotion(u)`

The engine calls it only for a unit with a motion. In this order:

1. **Tick(*M*'s goal)**.
2. **Steering**: an aircraft (`d.flags1` bit 11) steers as "Flying", every other unit as "Ground".
3. **The move** ("Moving").
4. **The move rate** ("Move rate").
5. **SFX_OCCUPY** for *u*.

**Turning.** To turn towards a wanted change *δ* (signed 16-bit), with *T* = `d.turnrate` (unsigned): when *δ* is
0, `turn` becomes 0 and nothing is published; else `turn` becomes *T* when *δ* ≥ *T*, −*T* when *δ* ≤ −*T*, and *δ*
otherwise (kept to signed 16 bits), and TURN with `delta` `turn` is published. Units adds it to `u.heading` at once.

**Speed along the heading**, Push(*a*), for a change *a*: `speed` += *a*, and becomes 0 when negative. *i* =
`u.pitch` >> 11 (arithmetic), clamped to −5 to 5, and *F*, a percentage, is by *i*: 25 at −5, 55 at −4, 70 at −3, 85 at −2, 100 at −1 and 0, then 75, 50, 25, 20
and 15 at 1 to 5.
The cap is *F* × `d.maxvelocity` / 100 (section 5); when `u.yWhole` < `Game.seaLevel` and `d.flags1` has neither
bit 12 nor bit 19, the cap is halved (section 5). `speed` becomes at most the cap, and `velocity` becomes
Step(`u.heading`, `speed`), with the heading as the block now holds it.

**Ground.** A ground unit has an AI search goal or a patrol goal. When the goal has no route (an AI search goal
without `active`, a patrol goal with `count` below 2), `turn` becomes 0 and Push(−`d.brakerate`). Otherwise, with *p0*, *p1*, *p2* its
waypoints 0, 1, 2 and *P* = `u.pos`:

- *g* = ftol(hyp(*p1.x* − *P.x*, *p1.z* − *P.z*)). When *g* > 5242880: *e* = *p1* − *p0* (*x* and *z*), *L* =
  ftol(hyp(*e.x*, *e.z*)); when *L* ≥ 0x10000, *n* = ((*e.x* << 16) / *L*, (*e.z* << 16) / *L*), *b* = min(*g* −
  5242880, *L*), and *p1* moves back by (*n.x* × *b*) >> 16 and (*n.z* × *b*) >> 16.
- *D1* = hi((*p1.x* − *P.x*)²) + hi((*p1.z* − *P.z*)²) and *D2* the same for *p2*, hi being the high 32 bits of the
  64-bit square.
- *δ* = Head(*P*, *p1*) − `u.heading` (signed 16-bit); turn by *δ*.
- *k* = (|*δ*| × `speed`) / `d.turnrate`; *t* = (`speed`²) >> 16; *q* = (*t* << 16) / (2 × `d.brakerate`); *r* =
  hi(*q*²); *lim* = hi(*k*²) × 4 (section 5 for the widths).
- When *D1* > *lim* and *D2* > *r*, Push(`d.acceleration`); else Push(−`d.brakerate`).

**Flying.** When `mode` is not 2: `velocity`, `speed` and `turn` become 0, and nothing else. Otherwise, with the
goal's `pos` *a*, `vel` *b* and `heading` *h*, and *v0* = `velocity`:

1. The drag: each word of `velocity` becomes (word × (0x10000 − (`d.acceleration` << 16) / `d.maxvelocity`)) >> 16.
2. The cap: *s* = hyp(`velocity.x`, `velocity.z`) rounded to a float, × 2⁻¹⁶; *m* = `d.brakerate` as a float ×
   2⁻¹⁶. When *s* > *m*: *f* = ftol(*m* / *s* × 65536) and `velocity.x` and `.z` become (word × *f*) >> 16; then
   *e* = ftol((*s* − *m*) × 65536), and `velocity` += Step(`u.heading`, *e*), with the heading before this tick's
   turn.
3. *da* = `u.pos` − *a*, *db* = `velocity` − *b*.
4. The climb: when `u.spatialBucket` is not `Game.overflowBucket`: *l* = 0x10000 when `speed` with its low two bits
   cleared is below 0x40000, else `speed` >> 2 (the speed of the last tick); `velocity.y` becomes *l* when *da.y* ≤
   −*l*, −*l* when *da.y* ≥ *l*, and −*da.y* otherwise.
5. Turn by *h* − `u.heading` (signed 16-bit).
6. The correction (section 5, "Flying"): with *A* = `d.acceleration` as a float × 2⁻¹⁶ and *H* = max(8, hyp(*da.x*,
   *da.z*) × 2⁻¹⁶), *κ* = −√(2*A* / *H*); *cx* = *da.x* × *κ* × 2⁻¹⁶ − *db.x* × 2⁻¹⁶ and *cz* likewise; when
   hyp(*cx*, *cz*) > *A*, both are scaled by *A* / hyp(*cx*, *cz*). `velocity.x` += ftol(*cx* × 65536) and
   `velocity.z` += ftol(*cz* × 65536).
7. `speed` becomes ftol(√(*x*² + *y*² + *z*²)) of `velocity`.
8. Tilt(*u*, `velocity` − *v0*).

**Tilt(*u*, *v*)**: each word of `p2` becomes (word × 0xf333) >> 16, then `p2` += *v*. (*x*, *z*) = Rot(`u.heading`
as the block now holds it, `p2.x`, `p2.z`); *n* = (`Game.gravity` << 16) / 0xccd; TILT is published with `bank` A((
`d.bankscale` × −*x*) >> 16, *n*) and `pitch` A((`d.pitchscale` × −*x*) >> 16, *n*) (Q5).

**Moving.**

- **Carried** (`u.carrier` not 0): with *c* the carrier and *k* = `u.transportPiece` as a signed byte:
  `scripts.GetPiecePosition(c, k)` gives *v*; when `d.flags1` has bit 19, *v.y* becomes at least `(s16)(seaLevel −
  draft) << 16` (section 5); SET_POSITION with `pos` *v* and `mode` the motion's. `scripts.GetPieceAngles(c, k)`
  gives the angles; ANGLES with them (`heading` zero-extended, `bank` and `pitch` sign-extended). `speed` and
  `velocity` become *c*'s motion's when *c* has a motion, and 0 otherwise. MOVED with `moved` 0. Nothing more.
- Otherwise *p* = `u.pos` + `velocity`. When *p* is `u.pos` and `mode` is the unit's mode (`u.flags` bits 0-1),
  nothing. Otherwise `lastMoveTick` becomes `Game.ticks` and *c* = Cell(*p*, `u.footprint`):
  - *c* is `u.cell` and the modes agree: MOVE with `pos` *p*.
  - Otherwise, when `u.player` has `active` not 0 and `type` 1 or 2, the blocked bit becomes 1 when
    `map.CanPlaceUnitFootprint(d, u.id, &c, mode)` gives 0 and 0 otherwise; for another player it keeps its value
    (Q7). Then:
    - **blocked**: *p.x* and *p.z* are clamped to within 0x7ffff of Centre(`u.cell`, `u.footprint`); when `speed`
      > `d.maxvelocity` / 2, `speed` becomes that half and `velocity` Step(`u.heading`, half); MOVE with `pos` *p*.
    - **not blocked**: map's REMOVE_UNIT for *u*, CHANGE_CELL with `pos` *p*, `cell` *c* and `mode`, map's ADD_UNIT
      for *u*, MOVED with `moved` 1, los_radar's UPDATE_UNIT for *u*, in that order.

**Move rate.** The rate is 0 when the blocked bit is set, *u* is carried, or `speed` and `turn` are both 0; else 1
when `speed` ≤ `d.moverate1`, 3 when `speed` > `d.moverate2`, and 2 otherwise. When it equals `u.flags` bits 2-3,
nothing. Otherwise: START_SCRIPT StopMoving (1) for rate 0, or START_SCRIPT StartMoving (0) when `u.flags` bits 2-3
are 0; then START_SCRIPT MoveRate1, 2 or 3 (2, 3, 4) for rates 1 to 3; then MOVE_RATE with `rate`.

### `ApplyPathEvent(event, size, waypoints)`

A size below 20 does nothing. *g* is the event's `requester`; an event whose requester is not an AI search goal of
this module (by its class word) does nothing (ozgb/btm#25).

- **STARTED** (size at least 24; otherwise nothing): when the event's `target` is a valid target (section 4,
  `Apply`), `repathClaimTick` becomes the event's `tick`, and Report(that target, 0x100 when `start` is
  `PATH_START_DIRECT`, 0x200 otherwise). A STARTED with an invalid target does nothing.
- **PROGRESS**: nothing.
- **FINISHED** (size at least 28; otherwise nothing): with *n* = `count`: when *n* is 0 and *g* has a target *t*
  that Reached(*t*, owner) says no to, Report(*t*, 0x40); `active` is cleared; `count` and `points` stay. When *n* >
  0: `count` becomes min(*n*, 20), points 0 to `count` − 1 become the first waypoints (the rest stay), and `active`
  is set. Either way `flag_1` is cleared and `flag_3` set.

### Building a target

Each `New*` slot makes `services.SimAlloc` of its record's size, writes the class word and `source` = `order`, and
returns the new target's block offset. The arguments are the header's, in its order. *U* is the order's `Order.unit` and *fp* its `footprint`. Fields not named
stay 0.

| Slot | Imports after the alloc | Fields |
| --- | --- | --- |
| `NewPathOrderAt` | `UnitRefInit(&ref, 0, 0)` | `flags` 0x20, `piece` −1, `unit` *U*, `pos` *pos* |
| `NewPathOrderToUnit` | `UnitRefInit(&ref, unit, 0)` | `piece` −1, `unit` *U*, `pos` *U*'s `pos`; when `ref.owner` (now *unit*) is an aircraft, `flags` 7 and `followOffset` *w* << 16 for *w* the `range` of *U*'s `weapons[0].weapon`, 0x640000 when *w* is 0; otherwise `flags` 1 |
| `NewPathOrderToUnitAt` | `UnitRefInit(&ref, unit, 0)` | `flags` 0xa3, `piece` −1, `unit` *U*, `pos` *pos* |
| `NewPathOrderToUnitAtAngle` | `UnitRefInit(&ref, unit, 0)` | `flags` 5, `piece` *angle* (Q3), `unit` *U*, `pos` *U*'s `pos` |
| `NewAirManeuverOrder` | none | `self` *U*, `target` *from*, `other` *to* |
| `NewApproachRadius` | none | `pos` Cell((*x*, 0, *y*), *fp*); `radius`; `radiusSq` (`radius` / 16)² |
| `NewRingApproach` | none | `pos` as a disc's; `inner` the argument *outer*, `outer` the argument *inner* (Q2); `innerSq` (field `inner` / 16)², `outerSq` (field `outer` / 16)² |
| `NewPointMarker` | none | `x1` = *pos.x* − *fp.x*, `y1` = *pos.y* − *fp.y*, `x2` = *size.x* + *pos.x*, `y2` = *size.y* + *pos.y* |

The record sizes: a move target 54, an air maneuver target 44, a disc 20, a ring 28, a footprint marker 24. The
unit reference is built in place at `ref` (byte 22 of the move target); `UnitRefInit` writes its words and links it
into the list of the unit it is given (the recordings show those writes as `o` lines of the import). The `New*`
slots are only called with an order whose `unit` is a unit, and the `ToUnit` slots with a unit.

### The queries

- **`HasNetState(u)`**: 0 when `u.motion` is 0. Otherwise by the goal: an AI search goal gives 1 when `flag_3` is
  set or the motion's blocked bit differs from `flag_2`, else 0; a lite goal gives its `dirty` bit; the others 0.
  Writes nothing.
- **`TargetType(target)`**: 0 for 0; otherwise the kind of the target (2 to 6, section 3). Writes nothing.
- **`UnitTarget(u)`**: 0 when `u.motion` is 0; otherwise the goal's `target`. Writes nothing.

### Effects

Each publish is `services.Emit(&payload, size)`. Every payload starts with `MovementEventHeader` (or map's or
los_radar's header) whose `unit` is the unit's block offset.

| Kind | Payload, size | Fields |
| --- | --- | --- |
| ORDER_FLAGS 336 | `MovementOrderFlags`, 16 | `unit` the order's `Order.unit`, `order`, `flags` |
| TURN 337 | `MovementTurn`, 12 | `delta`: `turn`, sign-extended |
| TILT 338 | `MovementTilt`, 16 | `bank`, `pitch`, sign-extended from 16 bits |
| ANGLES 339 | `MovementAngles`, 20 | `bank`, `heading`, `pitch` |
| MOVE 340 | `MovementMove`, 20 | `pos` |
| MOVED 341 | `MovementMoved`, 12 | `moved` 0 or 1 |
| CHANGE_CELL 342 | `MovementChangeCell`, 32 | `pos`, `cell` (sign-extended), `mode` |
| MOVE_RATE 343 | `MovementMoveRate`, 12 | `rate` |
| SET_POSITION 344 | `MovementSetPosition`, 24 | `pos`, `mode` |
| SET_STATE_BITS 345 | `MovementSetStateBits`, 16 | `mask`, `set` |
| SFX_OCCUPY 346 | the header alone, 8 | |
| START_SCRIPT 347 | `MovementStartScript`, 12 | `script` (`MovementScript`) |
| map ADD_UNIT 160, REMOVE_UNIT 161 | `MapEventHeader`, 8 | |
| los_radar UPDATE_UNIT 128 | `LosRadarUnit`, 8 | `unit` |

## 5. Exactness

- **Draws**: none. Movement draws from no stream.
- **Integers**: 32-bit two's complement; every sum, difference and negation wraps. A product written `(a × b) >> k`
  is the exact 64-bit product shifted arithmetically, kept as its low 32 bits; a quotient `(a << 16) / b` is the
  64-bit value divided and truncated toward zero, kept as its low 32 bits; a plain `/` of two ints truncates toward
  zero. hi(*x*²) is the high 32 bits of the exact 64-bit square. `>>` of a signed value is arithmetic. A value "kept
  to signed 16 bits" is its low 16 bits sign-extended.
- **The speed cap**: ((*F* << 16) × `maxvelocity`) >> 16, then that << 16 divided by 0x640000 (both in 64 bits);
  under water, (cap × 0x8000) >> 16.
- **The ground steering widths**: *k* is the 64-bit product of |*δ*| (0 to 32768) and `speed`, divided by
  `turnrate` and truncated, kept to 32 bits; `2 × brakerate` wraps at 32 bits; *lim* is hi(*k*²) times 4, wrapping.
  The reference never divides by a `turnrate` of 0 here (no moving ground unit has one); a module must not trap on
  it, and what it gives is not judged.
- **Comparisons**: `repathClaimTick` against the tick is unsigned. Every other comparison is signed, `turnrate`
  and the ANGLES heading being unsigned 16-bit values widened to int.
- **The carried water line**: `(seaLevel − draft)`, its low 16 bits as a signed whole, times 65536.
- **Flying, in floating point.** *s* is the double hyp rounded to a float, then multiplied by the float 2⁻¹⁶
  (exact); *m* is `brakerate` converted to a float, times 2⁻¹⁶. *m* / *s* and *s* − *m* are computed in double from
  the floats. *A* is `acceleration` converted to a float, times 2⁻¹⁶. *H* is the double hyp times 2⁻¹⁶, raised to
  8.0 when below it. *κ* = −sqrt((2.0 × *A*) / *H*) in double. *cx* = ((*da.x* × *κ*) × 2⁻¹⁶) − (*db.x* × 2⁻¹⁶) in
  double, *cz* likewise; *X* = *cx* rounded to a float, *Z* = *cz* rounded to a float; *μ* = hyp(*cx*, *cz*) of the
  doubles. When *μ* > *A*: the *x* step is *X* (as a double) × (*A* / *μ*), a double, and the *z* step is *Z* × (*A*
  / *μ*) rounded to a float; otherwise the steps are *X* and *Z*. `velocity.x` += ftol(*x* step × 65536.0) and
  `velocity.z` += ftol(*z* step (as a double) × 65536.0) (Q8). The new `speed` is ftol of the double square root of
  ((*x*² + *y*²) + *z*²) of the three words as doubles.
- **Reached, in floating point**: the move target's *r* is the double hyp rounded to a float, times 2⁻¹⁶;
  `approachRadius` > *r* and *r* > 0.5 compare in double. The air maneuver's distance is the hyp rounded to a float,
  divided by 65536 in float, compared with 48.
- **Conversions**: ftol truncates toward zero (the kit's `ftol`: 0 for a NaN or an infinity). A heading kept from
  A(...) is its low 16 bits.

## 6. Quirks

- **Q1**: SET_ALTITUDE asks `map.GetGroundHeight` twice, with the same position, when the ground is above the
  water line, and once when it is not.
- **Q2**: `NewRingApproach` stores its `inner` argument as the ring's `outer` and its `outer` argument as `inner`
  (so a caller that passes the outer radius first gets a ring the right way round).
- **Q3**: `NewPathOrderToUnitAtAngle` stores its `angle` as the target's `piece`; the target's `heading` stays 0.
- **Q4**: a footprint marker's position is the middle column of its rectangle but its last row (`y2`), not the
  middle row.
- **Q5**: TILT's pitch is computed from the same rotated *x* as the bank, scaled by `pitchscale`, not from *z*.
- **Q6**: a waypoint step leaves the old last point in the array past `count`.
- **Q7**: for a unit whose player is not active or not of type 1 or 2, a cell change does not check the map, and the
  blocked bit keeps the value it last had.
- **Q8**: after the flying correction is scaled down, the *x* step is used as a double and the *z* step rounded to a
  float.

## 7. Examples

From the kit's recordings. Each was recomputed from this spec's definitions.

- **NEW, an aircraft**: `tests/replay/movement/orders-air-fights.txt`, call at tick 10 of `Apply` (line 134), kind
  320 for `Unit#2` (an aircraft of player 0, type 1): `SimAlloc 47` then `SimAlloc 40`; `Unit#2.motion` becomes
  `UnitMotion#3`, whose `obj` is `LiteGoal#1`, `flags` 1, `movementClass` null; the goal's `pos` is the unit's
  (39321600, 65536, 19660800), `heading` −32768, `netDirtyFlags` 1. Outcome 1.
- **NEW, a ground unit**: the same file, tick 10 (line 294), `Unit#252` of player 1: `SimAlloc 47`, `SimAlloc 101`,
  an `AiSearchGoal#3` with `flags` 8, `count` 0, and the motion's `movementClass` `MovementClass#5`.
- **A disc**: `orders-build-patrol.txt`, tick 150 of `NewApproachRadius`: order `Order#3`, whose unit has footprint
  (2,2), x 45878325, y 19689825, radius 16: (45878325 − 1048576 + 524288) >> 20 = 43 and (19689825 − 1048576 +
  524288) >> 20 = 18, so `pos` (43,18), `radiusSq` 1; `SimAlloc 20`, the result `ApproachRadius#1`.
- **A ring** (Q2): the same file, tick 150 of `NewRingApproach`: `inner` 107, `outer` 47 give the fields `inner` 47,
  `outer` 107, `innerSq` (47 / 16)² = 4, `outerSq` (107 / 16)² = 36.
- **A footprint marker**: `orders-air-fights.txt`, tick 20 of `NewPointMarker`: `pos` (41,19), `size` (5,5), the
  unit's footprint (2,2): `x1` 39, `x2` 46, `y1` 17, `y2` 24.
- **A move target to a unit**: `orders-air.txt`, tick 4095 of `NewPathOrderToUnit`: `Order#2`, unit `Unit#1`, not an
  aircraft: `SimAlloc 54`, `UnitRefInit PathOrderAttach#1 Unit#1 null`, then `flags` 1, `piece` −1,
  `followOffset` 0, `unit` `Unit#3` (the order's), `pos` that unit's (51887393, 3014656, 25678911).
- **At an angle** (Q3): `orders-air-repair.txt`, tick 670 of `NewPathOrderToUnitAtAngle`, `angle` 1: `flags` 5,
  `piece` 1.
- **SET_TARGET on an AI search goal**: `orders-air-fights.txt`, tick 20 of `Apply` (line 651): `Unit#1` at cell
  (34,14), position (36700160, ·, 15728640), its goal with no points and `count` 0, its first order's `flags_42`
  1062153 (no 0x800000), target `PointMarker#1` (39, 46, 17, 24, footprint (2,2)). `Cancel AiSearchGoal#1`; no old
  target, so no report. Pos gives *x* = (2 + 42 × 2) << 19 = 45088768, *z* = (2 + 24 × 2) << 19 = 26214400, so
  `count` 2 and the points (560,240), (688,400); `active`, `flag_1` and `flag_3` set (`flags` 11); then
  `Submit (Unit#1,60,3,(0,0),0,0,39,46,17,24,AiSearchGoal#1,PointMarker#1,0,(0,0))`. (The recording carries no
  `w` line for `points`; the next call's `r AiSearchGoal#1.points` holds them: "Gaps in the kit".)
- **Ground steering on that route**: the same file, tick 20 of `UpdateMotion` (line 666), `Unit#1` heading 0,
  speed 0, `turnrate` 1044, `acceleration` 9830, `maxvelocity` 78643: the unit turns by −1044 (TURN, 12 bytes;
  units writes `heading` −1044), accelerates to 9830 (under the cap 78643), and `velocity` becomes Step(−1044,
  9830): S(−1044, 9830) uses entry 504, −803, and (−803 × 9830 + 4096) >> 13 = −964, so `velocityX` 964 and
  `velocityZ` −9783. The unit stays in its cell: MOVE (20). The rate becomes 1 from 0: START_SCRIPT StartMoving,
  START_SCRIPT MoveRate1, MOVE_RATE (12 bytes each), then SFX_OCCUPY (8). `lastMoveTick` 20.
- **Flying, with a cell change**: `orders-air.txt`, tick 4091 of `UpdateMotion` (line 469), `Unit#5` with
  `LiteGoal#3` (`pos` (43530660, 22347776, 65093149), `vel` 0, `heading` 27720), the unit at (66662745, 22347776,
  21084818) heading 27720, `velocity` (−258360, 0, 488660), `speed` 552755, `p2` (−98618, −49661, 98627),
  `acceleration` 22937, `maxvelocity` 655360, `brakerate` 393216. The drag scale is 0x10000 − 2293 = 63243; *s*
  8.1392… > *m* 6.0, so the cap applies; the heading wants no turn (no TURN); *κ* −0.03037…, the correction
  steps −0.16223… and 0.31012…, so `velocityX` −260514, `velocityZ` 491586, `speed` 556349, then `p2` (−95841,
  −47178, 96621) and TILT (16). The new position is in cell (62,20): `CanPlaceUnitFootprint UnitDef#51 5 (62,20) 2 =
  1`, then REMOVE_UNIT (8), CHANGE_CELL (32), ADD_UNIT (8), MOVED (12), UPDATE_UNIT (8); the rate stays 2; SFX_OCCUPY
  (8). `lastMoveTick` 4091.
- **Carried**: `units-transport.txt`, `Unit#1` carried by `Unit#3`, `transportPiece` 255: `GetPiecePosition Unit#3
  -1`, SET_POSITION (24), `GetPieceAngles Unit#3 -1 = (19,32865,0)`, ANGLES (20), MOVED (12), SFX_OCCUPY (8).
- **SET_ALTITUDE** (Q1): `orders-air-fights.txt`, tick 30 of `Apply` (line 1048): `PathOrder#1` with `flags` 32 and
  `pos` (41943040, 65536, 22282240), value 35: two `GetGroundHeight pos=(41943040,22282240) = 1` (1 is above the
  water line 0), `altitude` 35, `flags` 40, `posY` (1 + 35) << 16 = 2359296.
- **SET_APPROACH_RADIUS**: `orders-air.txt`, tick 4095: `PathOrder#4` `flags` 9 becomes 25, `approachRadius` 48.
- **DESTROY_TARGET**: `orders-air-repair.txt`, tick 685: `UnitRefUnlink PathOrder#2.ref`, then `SimFree`.
- **SET_TARGET 0 on a lite goal**: `orders-air-repair.txt`, tick 670: ORDER_FLAGS (16) on the old target, `target`
  null.
- **UnitTarget**: `orders-air-fights.txt`, tick 15: `Unit#1`'s goal has no target: null.

No recording covers: DESTROY, LOAD, SET_SPEED, SET_HEADING, a refused event (outcome 0), SET_ALTITUDE on an air
maneuver target or on a target of another kind, NEW for a player of type 3 (patrol and packed position goals) and
everything those goals do, a move target's follow offset (flags 0x2), an air maneuver target, `ApplyPathEvent`,
`HasNetState`, `TargetType`, `NewPathOrderToUnitAt` and `NewAirManeuverOrder`.

### Gaps in the kit

- **Class words.** The kit names the objects of the recordings by their class (`docs/replay-format.md`,
  "Polymorphic objects") and the engine slots read the class of a goal or target from its first word, but no file of
  the kit gives the value a module writes there for each class. A module built from this spec can't make an object
  the replay or an engine slot recognises until the kit gives those values.
- **A blocked entry.** The entry is `blocked`, with no budgets (ABI 4.2); the gate's recordings route judges a module
  without them (ABI section 15).
- **Payloads.** The recordings carry each publish as `services.SchedPublish 14 <size>`, without its fields, so the
  gate compares an effect by its size and by what its subscribers write (the `o` lines after it), not by its
  payload.
- **SET_TARGET's points.** An AI search goal's new two-point route is not written as a `w` line in the `Apply` that
  makes it; the following call's `r` line restores it.
- **An unset position.** When Reached asks a followed move target whose unit is gone or off the map, the reference
  compares with a position it never set. No recording reaches it; this spec answers no.
- **`Unit.pitch`.** The recordings carry no `pitch` line for the units they move, so the cap's slope index is 0 in
  every recorded call.

## 8. Kit version

Kit `interfaces-v68` and `reference-traces-v55`, the versions the gate judges against. Interface `movement` 5.0.
The imports are against `units` 7, `map` 9, `scripts` 3, `pathfinding` 4 and `services` 4.
