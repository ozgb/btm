# pathfinding

## 1. Role

Pathfinding finds routes over the cell grid for units that ask for one. A requester (an order of a unit)
submits a request naming the unit and a goal area; once a tick, in the players phase, the module shares a
budget of search work among the players, walks each player's unit slots in turn, starts a search for a
unit whose request is due, and carries the search on over as many ticks as it needs. A search first walks
straight at the goal and follows walls, then runs a best-first search over the eight grid directions, and
reports its start and its end as events that the engine polls in the same phase. The waypoints of a route
are the corners of its cells, in world units. All of the module's state is one region of the block that
the engine allocates at match load.

## 2. Interface

### Slots (`interfaces/pathfinding.h`, version 4.1)

| Slot | When the engine calls it |
| --- | --- |
| `RegionSize` | at match load, before `Init`, to size the region |
| `Init` | once per match, after the map has loaded, with the region the engine allocated |
| `Shutdown` | at match end; the engine frees the region it returns |
| `Submit` | when an order wants a route for its unit |
| `Cancel` | when an order drops its request or the search for it |
| `CellsChanged` | when map rewrites a rectangle of a pass map (map's interface, its effect CELLS_CHANGED) |
| `Step` | once per tick, first in the players phase |
| `Poll` | right after `Step`, in the same phase, until it returns less than the header's size |
| `WriteState` | from the test harness, to dump the module's state |
| `SetSearchScale` | by the `search` console command, through the engine's handler of SET_SEARCH |
| `DebugCell` | by the debug overlay, per cell it draws |

`SetSearchScale` and `DebugCell` are optional in the header (null for an implementation without them);
this module provides both.

### The manifest entry

The kit's manifest (`wasm/modules.json`) has no entry for pathfinding: it has no `signatures.pathfinding`
and no signatures for the world view, and it gives the interface major, 4 (`majors.pathfinding`). The kinds
below follow from `interfaces/pathfinding.h` by the rules of ABI section 7: a pointer is `linear` (or
`block` when it always lies in the block), every `int`, `unsigned int` and `unsigned short` is `int`, and a
`PathUnit`, `PathClass` or `PathHandle` is `int` too: each is a block offset or a number that the module
stores and hands back, never follows, and never an ABI handle (section 8.1). A module's build sets
its `data_base` and `table_base` (within the windows and table ranges of ABI sections 2.2 and 2.3), its
`majors`, its `slots`, and its fuel and size budgets (ABI sections 11.1 and 11.3: a budget belongs to its
build, set from the build's own runs). None of these can be checked against the kit at `interfaces-v58`,
which has no pathfinding entry; the entry the gate will hold is the one ozgb/byte-tactics-moddable#1206 adds.

- `name` and `interface`: `pathfinding`.
- `exports`, in this order: `RegionSize`, `Init`, `Shutdown`, `Submit`, `Cancel`, `CellsChanged`, `Step`,
  `Poll`, `WriteState`, `SetSearchScale`, `DebugCell`.
- `data`: `g_game`, `g_blockBase` (ABI 8.2). The module reads both and no other data word.
- `majors`: `pathfinding` 4.
- Slot kinds:

| Slot | Kinds |
| --- | --- |
| `RegionSize` | `[linear] -> int` |
| `Init` | `[linear, block, int] -> void` |
| `Shutdown` | `[] -> block` |
| `Submit` | `[linear] -> void` |
| `Cancel` | `[int] -> void` |
| `CellsChanged` | `[int, int, int, int, int, int] -> void` |
| `Step` | `[] -> void` |
| `Poll` | `[linear, int, linear, int] -> int` |
| `WriteState` | `[linear] -> void` |
| `SetSearchScale` | `[int] -> void` |
| `DebugCell` | `[int, int, linear, linear] -> void` |

- Imports, every one listed and nothing else. They are the slots of `PathWorldView` of the same header, as
  the group `pathworld` (the name the recordings give them, `i pathworld.<Slot>`):

| Import | Kinds | Used for |
| --- | --- | --- |
| `pathworld.Tick` | `[] -> int` | the tick of an event, and whether a request is due |
| `pathworld.SearchStepLimit` | `[] -> int` | the work per tick |
| `pathworld.PlayerCount` | `[] -> int` | the share of each player |
| `pathworld.IsPlaying` | `[int] -> int` | which players get a share |
| `pathworld.UnitsPerPlayer` | `[] -> int` | the heuristic weights |
| `pathworld.FirstUnit`, `pathworld.LastUnit` | `[int] -> int` | a player's first and last unit slot |
| `pathworld.NextUnit` | `[int] -> int` | the slot after a slot |
| `pathworld.UnitState` | `[int, linear] -> int` | the start of a search |
| `pathworld.InBounds`, `pathworld.Pass` | `[int, int, int] -> int` | the pass state of a cell |
| `pathworld.SeenSize` | `[linear, linear] -> void` | the size of the visibility grid |
| `pathworld.Seen` | `[int, int, int] -> int` | whether the searching player has seen a cell |
| `pathworld.BeginSearch`, `pathworld.EndSearch` | `[int] -> void` | around a search |

The world view's `MoveClass`, `LineOfSight` and `TerrainPass` are not imported: this module never calls
them. The kit's replay schema lists all eighteen slots of the world view, as the world view's own table,
not as this module's import list.

- Fuel: the module meters itself by rule version 1 (ABI section 11) and exports `__fuel_set` and
  `__fuel_get`. Its fuel budget and `size_budget` are its build's (ABI sections 11.1 and 11.3).
- Hooks: every slot of the header says "Hooks: none".
- The kit's manifest has no entry for this module, so the replay gate of ABI section 15 (`replay.py --check
  --subsystem pathfinding`) cannot load a WASM pathfinding at this kit, and no recording is of one (see
  "Gaps in the kit", section 7).

### Arguments outside the block

All of these are in the caller's memory (`linear`), read or written in place, little-endian:

- **`PathMapInfo`** (`RegionSize`, `Init`), 16 bytes: `width` (0), `height` (4), `unitSlots` (8),
  `moveClassCount` (12), each a signed 32-bit int. The recordings write it as `a map (224,102,2501,15)` in
  that order. `moveClassCount` is not used.
- **The region** (`Init`): the address of the first byte of the region in the block, and `size`, its
  length, which is the value `RegionSize` gave for the same map. Its block offset, the address less
  `g_blockBase`, is *R* below: the recordings give it as `a region 4341432`.
- **`PathRequest`** (`Submit`), 56 bytes: `unit` (0), `notBefore` (4, unsigned), `goal` (8, a
  `PathGoalArea` of 32 bytes), `requester` (40), `target` (44), `useStart` (48), `start` (52, a `PathPoint`).
  The module reads `unit`, `notBefore`, `goal`, `requester` and `target`, and ignores `useStart` and
  `start`. The recordings write each field as `a request.<field>`, the goal's as `request.goalKind`,
  `request.goalCentre`, `request.goalRadius0`, `request.goalRadius1`, `request.goalLeft`, `request.goalRight`,
  `request.goalTop`, `request.goalBottom`.
- **`PathGoalArea`**, 32 bytes: `kind` (0), `centre` (4: *x* then *y*, signed 16-bit each), `radius[0]` (8),
  `radius[1]` (12), `left` (16), `right` (20), `top` (24), `bottom` (28), signed 32-bit each.
- **`PathUnitState`**, the buffer the module gives `pathworld.UnitState` (24 bytes): `player` (0),
  `cell` (4: *x*, *y*, signed 16-bit), `heading` (8, unsigned 16-bit), two bytes the module ignores,
  `moveClass` (12), `footprintX` (16), `footprintY` (20). The recordings write it as
  `o out (1,(210,87),1030,780,2,2)`: player, cell, heading, class, footprints. The stub writes it only when
  the import returns nonzero.
- **`SeenSize`'s two pointers**: two 32-bit ints the import writes, the width and the height of the
  visibility grid (`o width 112`, `o height 51`).
- **`Poll`'s `out`** is a buffer of `size` bytes, which the caller has zeroed; `points` is a buffer of
  `capacity` `PathPoint`s (4 bytes each). The recordings compare the bytes of `out` that the call writes
  (`o out x:...`), and never `points` (`docs/replay-format.md`, the `scratch` form).
- **`WriteState`'s `writer`** is a `StateWriter` (`interfaces/state_writer.h`), a table of five calls.
- **`DebugCell`'s `flags` and `dir`**: two 32-bit ints the slot writes.

### Events

None. No slot of this module records a sim event (kinds are listed in `docs/sim-events.md`). The events
of this spec are the module's own path events (`PathEventKind`), polled through `Poll`.

## 3. State

### Owned

- **`Game.pathfinder`** (`layout.json`: byte 82439 of `Game`, unaligned): the block offset of the region, *R*, from `Init` until `Shutdown`, which sets it
  to 0. Every slot but `RegionSize` and `Init` finds the region through it; while it is 0, `Submit`,
  `Cancel` and `Step` do nothing and `Poll` returns 0 (`DebugCell` is outside the contract then).
- **The region**, every byte of it, compared byte for byte at the end of every recorded call (the pseudo-field
  `region` of `docs/replay-format.md`, "A module's region and world view"). It is zero when `Init` starts.
  Its parts, for a map of *w* × *h* cells with *s* unit slots:

| Part | Offset in the region | Size in bytes |
| --- | --- | --- |
| the head (the trace schema's `Pathfinder`) | 0 | 208 (201 used; bytes 201 to 207 stay 0) |
| the settings | 208 | 200 |
| the pool: *N* nodes | 408 | 20 *N* |
| the items: *N* node references | 408 + 20 *N* | 4 *N* |
| the cells: *C* cells | after the items | 4 *C* |
| the dirty words | after the cells | 4 ⌈*C* / 256⌉ |
| the requests: *P* entries | after the dirty words | 56 *P* |
| the requester buckets: *B* words | after the requests | 4 *B* |
| the unit buckets: *B* words | after those | 4 *B* |
| the events: 512 records | after those | 284 × 512 |

with *C* = *w* × *h* rounded up to a multiple of 8, *N* = the smaller of *C* + 1 and 65536, *P* = *s* + 16,
and *B* the smallest power of two that is at least 16 and at least *P*. The region's size is the sum.

A *reference* stored in the region is a block offset: *R* plus the offset in the region of what it names.

**The head** (bytes 0 to 200; the trace schema's names, `Pathfinder` and `PathfinderSearch`, where it has one):

| Offset | Field | Type | Meaning |
| --- | --- | --- | --- |
| 0 | `pool` | u32 | reference to the pool, 0 until the first node is taken in the match |
| 4 | `items` | u32 | reference to the items, set with `pool` |
| 8 | `freeHead` | i32 | the first node of the free list, −1 for none |
| 12 | `used` | i32 | the nodes taken from the pool since the search started |
| 16 | *capacity* | i32 | the open list's capacity, which only grows in a match |
| 20 | `count` | i32 | the length of the open list (the items in use) |
| 24 | `topPopped` | i32 | 1 when item 0 has been taken and still sits in the list |
| 28 | `cells` | u32 | reference to the cells |
| 32, 36 | `width`, `height` | u32 | the grid's size, *w* and *h* |
| 40 | `cellCount` | i32 | *C* |
| 44 | `dirty` | u32 | reference to the dirty words |
| 48, 50 | `start_x`, `start_y` | i16 | the start cell of the search |
| 52, 54 | `found_x`, `found_y` | i16 | the cell at which the last search that found a route ended |
| 56, 60 | `goalX`, `goalY` | i32 | the aim cell of the search |
| 64 | `probe` | i32 | the probe's result |
| 68 | `range` | i32 | the turn range of the next expansion: 4, then 2 |
| 72 | `stepsPerTick` | i32 | the work per tick |
| 76 | `steps` | i32 | the work of the current turn |
| 80 | `costScale` | i32 | the heuristic weight of the search, 16.16 |
| 84 | `baseScale` | i32 | the base weight, 16.16 |
| 88 | `object` | u32 | the searching unit, 0 when no search runs |
| 92 | `path` | u32 | the requester of the last search started (kept after it ends) |
| 96 | `target` | u32 | that request's `target` |
| 100 | `owner` | u32 | the searching unit's movement class, 0 when no search runs |
| 104 | `turnCost` | 8 bytes | 0, 40, 60, 80, 100, 80, 60, 40 |
| 112 | `stepCost` | 8 bytes | 16, 22, 16, 22, 16, 22, 16, 22 |
| 120 | `player` | u8 | the player whose bank is being spent |
| 121 | `cursor[0..9]` | 10 × u32 | each player's unit slot cursor (unaligned) |
| 161 | `budget[0..9]` | 10 × i32 | each player's bank (unaligned) |

**The settings** (offsets in the region):

| Offset | Field | Meaning |
| --- | --- | --- |
| 208 | *node limit* | *N* |
| 212 | *goal capacity* | 2(*w* + *h*) + 256 |
| 216 | *request capacity* | *P* |
| 220 | *request count* | the requests held |
| 224 | *first free* | the first free request entry, 0 for none |
| 228 | *bucket mask* | *B* − 1 |
| 232 | *event capacity* | 512 |
| 236 | *event count* | the events of the last step |
| 240 | *next event* | the next event `Poll` gives |
| 244, 248 | *seen width*, *seen height* | the visibility grid's size, at the last search start |
| 252, 256 | *footprint x*, *footprint y* | the searching class's footprint, at the last search start |
| 260 to 288 | eight offsets | the offsets in the region of the pool, the items, the cells, the dirty words, the requests, the requester buckets, the unit buckets and the events, in that order |
| 292 | *goal area* | the `PathGoalArea` of the last search started, 32 bytes |
| 324 | `playerBudgetCap[0..9]` | each player's heuristic weight, 16.16 |
| 364 | `playerTickLoad[0..9]` | each player's unit slot visits since the last refresh |
| 404 | `budgetRefreshCounter` | the steps since the last refresh |

All are signed 32-bit.

**A node** (20 bytes; the trace schema's `Node`): `index` (i32, its place in the items while it is in the
open list, or the next node of the free list while it is free), `pos_x`, `pos_y` (i16, its cell), `g` (i32,
the cost so far), `f` (i32, *g* plus the estimate), `penalty` (i16 at byte 16, 0 or 30), `steps` (i16 at
byte 18, the length of the straight run that reached it). Node *k* is at pool + 20 *k*.

**An item** is a reference to a node (`pool` + 20 *k*). The items 0 to `count` − 1 are a binary min-heap on
`f` (section 4, "The open list").

**A cell** (4 bytes): *flags* (u8), *dir* (u8, the direction it was entered by), *node* (u16, its node
while it is open). Cell (*x*, *y*) is cell *w* *y* + *x*. Flags: bits 0 and 1 are the cell's *mark*: 0
not visited, 1 open, 2 closed, 3 blocked; bit 2 (value 4) *goal*; bit 3 (value 8) *probed*.

**The dirty words**: bit *b* of word *j* says the eight cells 256 *j* + 8 *b* to 256 *j* + 8 *b* + 7 may hold
flags. To *dirty* cell *i* sets bit ⌊*i* / 8⌋ mod 32 of word ⌊*i* / 256⌋.

**A request entry** (56 bytes; entries are numbered from 1, entry *e* at requests + 56 (*e* − 1)): requester
(0), unit (4), target (8), notBefore (12), goal (16, a `PathGoalArea`), *next by requester* (48), *next by
unit* (52). A free entry has requester 0, and its next by requester is the next free entry.

**The buckets**: requester bucket *b* holds the first entry of a chain through *next by requester*, unit
bucket *b* the first of a chain through *next by unit*, 0 ending a chain. The bucket of a number *v* is
⌊(*v* × 0x9E3779B1 mod 2^32) / 65536⌋ AND *bucket mask*.

**An event record** (284 bytes): kind (0), status (4), unit (8), requester (12), target (16), tick (20),
count (24), and 64 points (28: *x*, *y*, i16 each, in world units).

### Read

- `Game.pathfinder` (above).
- `Game.width` and `Game.height` (signed 32-bit, bytes 82483 and 82487 of `Game`), at `Init` only: the grid's size (below). `RegionSize`
  uses `map` alone and reads neither. The recordings write the two fields as `r Game.width 224` and
  `r Game.height 102` under the first call of each file, `RegionSize`: those lines put the values in the block
  before the replay's first call, so they are there when `Init` reads them. They equal `map.width` and
  `map.height`.

The module reads no other field of the block outside its region.

### Between ticks

Everything in the region carries over. The events are dropped at the start of each `Step`. Nothing outside
the region and `Game.pathfinder` is kept.

**A window that starts later.** Every recording starts with `RegionSize` and `Init` at tick 0, then jumps to
its window (`docs/replay-format.md`, "A module's region and world view"): the first call of the window
carries `r region` runs that restore the region as the game had it at that tick. The module takes the
region as it finds it at every call, whatever state of this section it is in (a search may be running,
requests pending, the open list in any state the slots of section 4 can leave), and keeps no copy of
anything in it. `Game.pathfinder` is in no read or write set of the recordings: the module's own `Init` sets
it, and the replay never compares it.

**Why the mechanics are specified.** The gate compares every byte of the region, so the exact layout and the
exact operations of section 4 are observable: which node number a push takes, the order of the items, the
free list of nodes and of request entries, the dirty bits, the unaligned `cursor` and `budget` fields, and
the stale bytes a freed node, entry or event keeps. An implementation that reaches the same routes another
way does not replay equal.

## 4. Behaviour

Within one call, only the order of import calls is observable. The block writes are compared at the
end of the call, so the order they are listed in carries no meaning.

Every write named below is to the region or to `Game.pathfinder`. A byte the section does not say a call
writes keeps its value: a freed node, a freed request entry and an event record past *event count* keep
their old bytes, and those bytes are compared.

### Definitions

**Goal tests**, for a `PathGoalArea` *A* and a cell (*x*, *y*), with *q*(*r*) = (*r* / 16)², the division
truncated toward zero:

| `kind` | *x*, *y* is *in A* when | the distance *D*(*x*, *y*) is |
| --- | --- | --- |
| 1 (circle) | (*x* − *cx*)² + (*y* − *cy*)² ≤ *q*(`radius[0]`) | *E* − `radius[0]`, or 0 when *E* < `radius[0]` |
| 2 (ring) | *q*(`radius[0]`) ≤ (*x* − *cx*)² + (*y* − *cy*)² ≤ *q*(`radius[1]`) | *E* − `radius[1]` when *E* > `radius[1]`; `radius[0]` − *E* when *E* < `radius[0]`; else 0 |
| 3 (rectangle) | *x* is `left` or `right` and `top` ≤ *y* ≤ `bottom`, or *y* is `top` or `bottom` and `left` ≤ *x* ≤ `right` | *R*(*x*, *y*), below |
| anything else | never | 0 |

where (*cx*, *cy*) is `centre`, and *E* = 18 *a* + 7 *b* with *a* the larger and *b* the smaller of
|*x* − *cx*| and |*y* − *cy*| (when they are equal, 18 times the one plus 7 times the other is the same).
For the rectangle, with *ox* the distance of *x* outside [`left`, `right`] (`left` − *x* or *x* − `right`,
0 inside) and *oy* likewise for *y* and [`top`, `bottom`]:
- *ox* = 0 and *oy* = 0: 16 times the smallest of *x* − `left`, `right` − *x*, *y* − `top`, `bottom` − *y*;
- exactly one of them 0: 16 times the other;
- both above 0: 16 *a* + 6 *b*, with *a* the larger and *b* the smaller of *ox* and *oy*.

**Goal cells** of *A*, a list in this order: a circle, its centre; a ring, (*cx*, *cy* + (`radius[1]` +
`radius[0]`) / 32), truncated toward zero; a rectangle, for each column *i* from `left` to `right`, (*i*,
`top`) then (*i*, `bottom`), then for each row *j* from `top` + 1 to `bottom` − 1, (`left`, *j*) then
(`right`, *j*); anything else, none. Only the first *goal capacity* of them are used, and each
coordinate is taken as its low 16 bits, signed.

**The estimate** of a cell, *H*(*x*, *y*) = ⌊`costScale` × *D*(*x*, *y*) / 65536⌋, of the search's
*goal area*, in the arithmetic of section 5.

**The cell state** *S*(*x*, *y*) of the search, which makes these imports in order:
1. `pathworld.InBounds(owner, x, y)`; 0 gives *S* = 0.
2. With *vx* = ⌊*x* / 2⌋ + ⌊*footprint x* / 4⌋ and *vy* = ⌊*y* / 2⌋ + ⌊*footprint y* / 4⌋ (floor
   division, signed): when *vx* ≥ *seen width* or *vy* ≥ *seen height* (signed comparisons), *S* = 0
   with no further import.
3. `pathworld.Seen(player, vx, vy)`; 0 gives *S* = 2 (Q3). `player` is the head's `player`, the player whose
   turn started the search; `state.player` of `pathworld.UnitState` is never read.
4. Otherwise *S* is the result of `pathworld.Pass(owner, x, y)`, taken as unsigned.

*S* = 0 is *blocked*. A cell is *passable* when *S* ≥ 1.

**Directions** *d* = 0 to 7 step by (*dx*, *dy*): 0 (0, −1), 1 (−1, −1), 2 (−1, 0), 3 (−1, 1), 4 (0, 1),
5 (1, 1), 6 (1, 0), 7 (1, −1). Direction arithmetic is mod 8.

**On the grid** means 0 ≤ *x* < *w* and 0 ≤ *y* < *h*, with the coordinates sign-extended and compared
as unsigned 32-bit numbers (so a negative one is off).

**To mark a goal** cell (*x*, *y*) on the grid: its flags become 4 (the other bits cleared), and it is
dirtied. Off the grid, nothing.

**To probe-visit** cell (*x*, *y*) entering by *d*: it is dirtied (no bounds test), its dir becomes *d*, and
its flags gain bit 3. The visit *hits* when the flags then have bit 2. The probe visits only passable cells,
and `pathworld.InBounds` gives a nonzero result only for cells of the map's grid, so the cell is always on
the grid; a world view that answers otherwise is outside the contract.

**To add an event** of kind *k* and status *t*: when *event count* ≥ *event capacity*, nothing (and no
import). Otherwise record *event count* gets kind *k*, status *t*, unit = `object`, requester = `path`,
target = `target`, tick = the result of `pathworld.Tick()`, count 0, and *event count* goes up by 1. Its
points keep their old bytes.

**To end a search**: `pathworld.EndSearch(object)`, then `object` and `owner` become 0. **To fail a
search**: add an event FINISHED (2) with status FAILED (2), then end the search.

### The open list

The items 0 to `count` − 1 hold a binary heap: item *i*'s children are items 2*i* + 1 and 2*i* + 2. A node's
`index` is its item's place. *To place* node *k* at item *i* is to make item *i* refer to node *k* and node
*k*'s `index` *i*. Comparisons are on `f`, signed.

- **Sift up** from item *i*: when *i* is 0, nothing. Otherwise let *n* be the node of item *i*. While *i* > 0
  and *n*'s `f` is strictly below that of the node of the parent item (*i* − 1) / 2, the parent's node is
  placed at *i* and *i* becomes the parent. *n* is then placed at the final *i*.
- **Sift down** from item *i*: let *n* be its node. Repeat: if the right child exists (2*i* + 2 < `count`),
  the child is the right one when its `f` is strictly below the left one's, else the left one; if only the
  left child exists, it is the child; if none, stop. When the child's `f` ≥ *n*'s `f`, stop; otherwise the
  child's node is placed at *i* and *i* becomes the child. At the end *n* is placed at *i*.
- **Remove** node *k* (at item *j* = its `index`): node *k*'s `index` becomes `freeHead` and `freeHead`
  becomes *k*; `count` goes down by 1; then, when *j* < `count`, the node of item `count` is placed at *j*
  and sifted down from *j*.
- **Push** a node with data (*x*, *y*, *g*, *f*, penalty, steps), giving a node number:
  - When `topPopped` is 1: the node of item 0 takes the data (bytes 4 to 19), it is sifted down from 0,
    `topPopped` becomes 0, and the result is that node.
  - Otherwise, when `count` = *capacity*, the capacity grows: it becomes the smaller of *capacity* +
    ⌊*capacity* / 2⌋ + 16 and *node limit*, and if `pool` is 0, `pool` and `items` become their references
    (*R* + the pool's and the items' offsets). If `count` still equals *capacity*, nothing is pushed and the
    result is 0 (Q6).
  - Otherwise the node is `freeHead` when that is not −1 (and `freeHead` becomes that node's `index`),
    else `used` (and `used` goes up by 1); it takes the data; it is placed at item `count`; `count` goes up
    by 1; and it is sifted up from its item.
- **Lower** node *k* (after its `f` fell): when `topPopped` is 0, sift up from node *k*'s `index`. When it
  is 1, let *t* be the node of item 0; sift up from node *k*'s `index`; then, when *t*'s `index` is no longer
  0, `topPopped` becomes 0 and *t* is removed.
- **Take** the best node: when `topPopped` is 1, it becomes 0 and the node of item 0 is removed. Then the
  node of item 0 is the one taken; it stays in item 0 and `topPopped` becomes 1.
- The list's *size* is `count` − `topPopped`.

### The probe

The probe walks from the start towards `goalX`, `goalY` and returns a number, the best estimate it reached
(0 when it reached a goal cell). Every step of it named "a turn" adds 1 to `steps`. With (*x*, *y*) the
start and *best* = *H*(start):

1. When *S*(start) is blocked, the result is *best*.
2. **Greedy walk.** Repeat: a turn; when *best* is 0, the result is 0. The direction *d* is 2 when `goalX`
   < *x*, 6 when `goalX` > *x*, else 4 when `goalY` > *y*, else 0. When *S* of the next cell (*x*, *y*) +
   step(*d*) is blocked, leave the walk. Otherwise move there, probe-visit it by *d* (a hit gives the
   result 0), and *best* becomes the smaller of *best* and *H* of it.
3. **Wall walk.** Two walkers *a* and *b* start at (*x*, *y*), the place where the greedy walk stopped, each
   with the direction *d* + 2 (*d* the direction that was blocked); *started* is false. Every direction here
   is a number 0 to 7, its arithmetic mod 8, and directions are compared as those numbers. Each time the
   probe goes back to step 2 and then reaches this step again, both walkers and *started* start afresh from
   the new place and direction; *best* and `steps` carry on. Repeat:
   1. A turn. Walker *a*: let *stop* = its direction − 3; its direction goes down by 2; while *S* of its
      cell + step(its direction) is blocked: when its direction is *stop*, the result is *best*; else its
      direction goes up by 1.
   2. When *a* and *b* are on the same cell with the same direction and *started* is true, the result is
      *best*.
   3. *a* moves to that cell; *started* becomes true; probe-visit it by *a*'s direction (a hit gives 0).
      When the cell is *on the line* (below), (*x*, *y*) becomes it and the probe goes back to step 2.
      Otherwise *best* becomes the smaller of *best* and *H* of it.
   4. Walker *b*: let *stop* = its direction + 3; its direction goes up by 2; while *S* of its cell −
      step(its direction) is blocked: when its direction is *stop*, the result is *best*; else its
      direction goes down by 1. (*b* moves against its direction.)
   5. When *a* and *b* are on the same cell with the same direction, the result is *best*.
   6. *b* moves to that cell; probe-visit it by *b*'s direction (a hit gives 0); on the line: (*x*, *y*)
      becomes it and back to step 2; else *best* becomes the smaller of *best* and *H* of it.

A cell (*nx*, *ny*) is *on the line* from (*x*, *y*) when, with *gx* = `goalX` − *x*, *gy* = `goalY` − *y*,
*px* = *nx* − *x*, *py* = *ny* − *y*, and *gx* and *px* both negated when *gx* < 0, *gy* and *py* both negated
when *gy* < 0: either *py* = 0 and 0 < *px* ≤ *gx*, or *px* = *gx* and 0 < *py* ≤ *gy*. That is the path the
greedy walk takes from (*x*, *y*): along the row first, then along the column.

### The expansion

To *expand* a node taken from the open list, with data (*x*, *y*, *g*, …, steps *s*) in cell *c*, these
values being the node's as they were when it was taken (the node itself stays in item 0, and the
expansion's first push overwrites it, while every neighbour still uses the values of the take): the
cell's flags become 2 (closed). Then for each *t* from −`range` to `range`, in that order, with *u* = *t*
mod 8 (the *turn*), the neighbour in direction *e* = (*c*'s dir + *u*) mod 8 is considered, *c*'s dir read
at that moment:

- The neighbour (*nx*, *ny*) = (*x*, *y*) + step(*e*), each coordinate as a 32-bit number. Off the grid:
  nothing.
- Its *added cost* is `turnCost`[*u*] + `stepCost`[*e*], plus 75 when *u* ≠ 0 and *s* < 5.
- Mark 0 (not visited): the cell is dirtied, then *r* = *S*(*nx*, *ny*).
  - When *r* is 0 and the cell is not probed: its flags gain 3 (blocked). Nothing else.
  - Otherwise, with *h* = *H*(*nx*, *ny*): its flags gain 5 (open and goal) when *h* ≤ `probe` (Q4), else
    1; its dir becomes *e*; a node is pushed with *x* = *nx*, *y* = *ny*, penalty *p* = 0 when *r* > 1 and 30
    otherwise, *g*′ = *g* + added cost + *p*, *f* = *g*′ + *h*, steps 1 when *u* ≠ 0 and *s* + 1 otherwise;
    the cell's node becomes the push's result (its low 16 bits).
- Mark 1 (open), with *n* the cell's node: *g*′ = *g* + added cost + *n*'s penalty. When *g*′ < *n*'s *g*:
  the cell's dir becomes *e*, *n*'s `f` goes up by *g*′ − its *g*, its *g* becomes *g*′, its steps become 1
  when *u* ≠ 0 and *s* + 1 otherwise, and *n* is lowered. Otherwise nothing.
- Mark 2 or 3: nothing.

With `range` 4 the turns run from 4 up to 7, then from 0 up to 4: the reverse direction is considered twice.

### The route

When the node taken is in a goal cell, the search has found its route. `found_x`, `found_y` become that
cell. The route is traced back from it: with *dir* the found cell's dir (read as signed 8-bit), the list
starts with the found cell; at each cell *k* from the found cell, until the start cell: when *k*'s dir
differs from *dir*, *dir* becomes *k*'s dir and *k* is added to the list; then *k* moves back one step,
against *dir*. The start cell is added last, so a found cell that is the start gives the list [start,
start] and a route of two points. With *n* the list's length, the route is the list's last 64
entries (all of them when *n* ≤ 64) in reverse order: the start first, then the corners towards the found
cell, which is last when *n* ≤ 64.

An event FINISHED (2) is added with status PARTIAL (1) when *n* > 64, else FOUND (0). When it was added, its
count becomes the smaller of *n* and 64, and point *i* (0 from the start) becomes ((2 *x* + *footprint x*)
× 8, (2 *y* + *footprint y*) × 8) of entry *i* of the route, each stored as its low 16 bits. Then the search
ends (`pathworld.EndSearch`, as above).

### `RegionSize(map)`

Returns the region's size for `map` (section 3). No imports, no writes.

### `Init(map, region, size)`

With *w*, *h*, *s* from `map` and *R* the region's offset, the region (zero on entry) becomes:

- the settings' *node limit*, *goal capacity*, *request capacity*, *bucket mask*, *event capacity* and
  the eight offsets as section 3 gives them; *first free* 1; `playerBudgetCap`[*p*] 0x18000 for each
  player *p* from 0 to 9; every other field of the settings 0;
- each request entry *e* from 1 to *P* − 1 gets *next by requester* *e* + 1 (entry *P* keeps 0);
- the head: `freeHead` −1; `width` `Game.width`, `height` `Game.height`, and `cellCount` their product
  rounded up to a multiple of 8 (in the recordings these are *w*, *h* and *C*: the grid's size is the
  game's, while the layout of the parts follows `map`); `cells` and `dirty` their
  references; `stepsPerTick` the result of `pathworld.SearchStepLimit()`, or 0x535 (1333) when it is 0;
  `baseScale` 0x18000; `cursor`[*p*] the result of `pathworld.FirstUnit(p)`, for each player *p* from 0 to 9;
  every other byte of the head 0;
- the cells and the dirty words all 0.

`Game.pathfinder` becomes *R*. Imports, in order: `pathworld.SearchStepLimit`, then `pathworld.FirstUnit`
for 0 to 9. `size` is not read.

### `Shutdown()`

`Game.pathfinder` becomes 0; the result is the region's address (`g_blockBase` + *R*), or null (0) when
`Game.pathfinder` was already 0. No imports. The region is not changed.

### `Submit(request)`

With *q* = `request.requester`:

| Case | Writes |
| --- | --- |
| `object` ≠ 0 and `path` = *q* | none |
| an entry *e* holds *q* (found by walking the requester bucket of *q*) | when its unit differs from `request.unit`: *e* is unlinked from the chain of its unit's bucket (the link that names *e*, in the bucket or in an entry's *next by unit*, takes *e*'s *next by unit*), its unit becomes `request.unit`, its *next by unit* becomes the head of the unit bucket of the new unit, and that bucket becomes *e* |
| no entry holds *q*, *first free* = 0 | none (the request is dropped) |
| no entry holds *q* | *e* = *first free*; *first free* becomes *e*'s *next by requester*; *request count* goes up by 1; *e* gets requester *q* and unit `request.unit`; it goes at the front of the requester bucket of *q* (its *next by requester* the bucket's head, the bucket *e*) and at the front of the unit bucket of its unit |

In the second and the last case, *e*'s target, notBefore and goal become `request.target`,
`request.notBefore` and `request.goal` (32 bytes). No imports.

To find the entry of a unit *u* walks the unit bucket of *u* through *next by unit* and takes the first
entry whose unit is *u*. To *remove* entry *e*: it is unlinked from its requester chain and from its unit
chain (as above, each by the link that names it); its requester becomes 0; its *next by requester* becomes
*first free*; *first free* becomes *e*; *request count* goes down by 1. Its other fields keep their values.

### `Cancel(requester)`

1. When `path` = `requester`: `object`, `path`, `target` and `owner` become 0. The search, if one runs,
   is dropped with no `pathworld.EndSearch` (Q5).
2. When an entry holds `requester`, it is removed.
3. The events from *next event* to *event count* − 1 whose requester is `requester` are dropped: the others
   move down, in order, to fill the gaps from *next event* on (each record copied whole, 284 bytes), and
   *event count* becomes *next event* plus their number. The records past it keep their bytes.

No imports.

### `CellsChanged(left, top, right, bottom, classes, cause)`

Does nothing. No imports, no writes.

### `Step()`

1. *event count* and *next event* become 0.
2. `pathworld.PlayerCount()` gives *players*. When it is 0, the step ends.
3. `pathworld.SearchStepLimit()`; `stepsPerTick` becomes its result, or 0x535 when that is 0 (the result
   is the header's `unsigned int`, stored as its 32 bits and used as a signed int from then on, so a limit of
   2^31 or more is negative; `pathworld.Tick` is unsigned and compared unsigned). The *share*
   is `stepsPerTick` / *players*, truncated toward zero.
4. `budgetRefreshCounter` goes up by 1. When it is then 150 or more: it becomes 0, and for each player *p*
   from 0 to 9: *v* = `playerTickLoad`[*p*] / `pathworld.UnitsPerPlayer()` (one import per player,
   truncated toward zero); `playerBudgetCap`[*p*] becomes 6 × `baseScale` when *v* < 1, 3 × `baseScale` when
   *v* < 2, else `baseScale`; `playerTickLoad`[*p*] becomes 0.
5. *total* is 0. For each player *p* from 0 to 9, `pathworld.IsPlaying(p)`; when nonzero, `budget`[*p*] goes
   up by the share and *total* by the new `budget`[*p*].
6. **Turns.** While *total* > 0 and *event count* + 3 ≤ *event capacity*: `steps` becomes 0, then one of
   the three turns below, then *total* and `budget`[`player`] both go down by `steps`.

**A turn with no search** (`object` = 0): `steps` becomes 1. While `budget`[`player`] ≤ 0, `player` moves to
the next player, 9 wrapping to 0. `playerTickLoad`[`player`] goes up by 1. The player's cursor moves:
`pathworld.LastUnit(player)`; when the cursor equals it, the cursor becomes `pathworld.FirstUnit(player)`,
else `pathworld.NextUnit(cursor)`. Let *u* be the new cursor. The unit's request is the entry of *u*
(section 4, `Submit`). It *starts* when there is one, `pathworld.Tick()` ≥ its notBefore (unsigned), and
`pathworld.UnitState(u, state)` is nonzero; those imports are made in that order and stop at the first
that fails (no entry: no import). When it starts:

- the entry's fields are kept (a copy), and the entry is removed;
- `path` becomes its requester, `object` *u*, `steps` goes up by 100, `costScale` becomes
  `playerBudgetCap`[`player`];
- the search starts (below).

**The start of a search**, from the copy and `state`:

1. `owner` becomes `state.moveClass`, `target` the copy's target, *goal area* its goal, *footprint x* and
   *footprint y* `state.footprintX` and `state.footprintY`.
2. `pathworld.SeenSize`, whose two results become *seen width* and *seen height*.
3. `start_x`, `start_y` become `state.cell`.
4. `pathworld.BeginSearch(object)`.
5. `turnCost` and `stepCost` become their values (section 3).
6. The cells' flags are cleared: for each of the first ⌈`cellCount` / 256⌉ dirty words that has any bit
   set, the word becomes 0 and each cell its set bits name gets flags 0 (dir and node kept).
7. Each goal cell of the goal area, in its order (only the first *goal capacity* of them), is marked as a
   goal; one off the grid is not marked but still counts here. Among them all, the first with the smallest
   (`start_x` − *x*)² + (`start_y` − *y*)² gives `goalX`, `goalY` (when there are none, they keep their
   values).
8. The outcome is the first row of the table below that holds, in its order (a start both in the goal area
   and off the grid is the first row):

| Case | Events, in order, and what follows |
| --- | --- |
| the start is in the goal area | STARTED with DIRECT (0); the search fails |
| the start is off the grid | STARTED with INDIRECT (1); the search fails |
| the probe gives 0 | `probe` becomes 0; STARTED with DIRECT (0); the open list starts |
| the probe gives *v* ≥ *H*(start) | `probe` becomes *v*; STARTED with INDIRECT (1); the search fails |
| the probe gives *v* < *H*(start) | `probe` becomes *v*; STARTED with INDIRECT (1); the open list starts |

*H*(start) is taken before the probe runs (it reads nothing but the goal area and `costScale`).

**The open list starts**: `count`, `used` and `topPopped` become 0 and `freeHead` −1 (*capacity*, `pool`
and `items` are kept); the start cell is dirtied, gains flag bit 0, and gets dir ((`state.heading` +
0x1000) mod 65536) / 8192, truncated; a node is pushed with the start cell, *g* 0, *f* = *H*(start), steps
100 and penalty 0 (Q7); the start cell's node becomes the result; `range` becomes
4.

**A turn of a search with an open list** (`object` ≠ 0, size > 0): repeat while the size is above 0: `steps`
goes up by 1; the best node is taken; when its cell has the goal bit, the route is traced and the search
ends (the turn ends); otherwise the node is expanded, `range` becomes 2, and when `steps` ≥ 100 the turn
ends.

**A turn of a search with an empty open list** (`object` ≠ 0, size 0): the search fails.

### `Poll`

The arguments are `out`, `size`, `points` and `capacity`.

When *next event* ≥ *event count*, the result is 0 and nothing is written. Otherwise record *next event*
is given and *next event* goes up by 1. Its payload is 28 bytes, zero but for: kind (0), unit (4),
requester (8), target (12), tick (16) from the record; then for kind STARTED (0) the record's status at
20, and a size of 24; for PROGRESS (1) its count at 20 and a size of 24; for FINISHED (2) its status at 20
and its count at 24, and a size of 28; for any other kind, a size of 20. The first min(`size`, that size)
bytes of the payload are copied to `out` (`size` is the header's `unsigned int`, compared unsigned, so the whole payload when `size`
is at least its size, or 2^31 or more, and nothing when `size` is 0), and the
record's first min(count, `capacity`) points to `points` (none when `capacity` ≤ 0). The result is the
payload's size, whatever `size` is. No imports.

### `WriteState(writer)`

Does nothing in this module: no imports, no writes. `writer` is a table of calls that the ABI gives a module
no way to call (no import reaches it, and ABI section 9.1 allows no other call out of the module), so the
module cannot write its state through it. The game's own pathfinder writes, through it, the lines of the kit's
`docs/trace-format.md`, section 8.7 (sections 16 and 17). No trace row hashes those sections (the kit's `docs/trace-format.md`, the paragraph "The rows hash state,
and events are only dumped": sections 16 and 17 are in a dump and in no row), and no recording
holds the slot, so the gate does not judge it. It is a kit gap (section 7).

### `SetSearchScale(scale)`

`baseScale` becomes `scale` (nothing when `Game.pathfinder` is 0). It changes the weights at the next
refresh (`Step`, step 4).

The engine's handler of SET_SEARCH, not this module, applies the event: its `steps` sets the match's search
step limit, which this module reads through `pathworld.SearchStepLimit` at every `Step`, and its scale is
passed to this slot. The module has no inbox and takes no effect.

### `DebugCell(x, y, flags, dir)`

`*flags` becomes cell (*x*, *y*)'s flags and `*dir` its dir, each zero-extended. No bounds test.

## 5. Exactness

- **Integers.** Fields are 32-bit two's complement unless section 3 says otherwise. Sums and products wrap at
  32 bits. Signed division truncates toward zero; "⌊*v* / 2^k⌋" of a signed *v* is an arithmetic shift (the
  floor). Coordinates and `steps` of a node are signed 16-bit: sign-extended when read, their low 16 bits
  kept when stored. The node number of a cell is unsigned 16-bit.
- **The estimate.** *H* = the 64-bit product of `costScale` and *D* (both signed 32-bit), shifted right by 16
  with the sign kept, then its low 32 bits as a signed int.
- **The squared distances** of the goal tests and of the aim cell are 32-bit products.
- **The bucket hash** is the unsigned 32-bit product, wrapping, of the number and 0x9E3779B1.
- **The heading** is the unsigned 16-bit `state.heading`; ((heading + 0x1000) mod 65536) / 8192 is the top
  three bits of the sum.
- **Tick comparisons** are unsigned.
- **Comparisons** in the probe (with *best*), the expansion (with `probe`, of *g*) and the open list (of
  `f`) are signed. `pathworld.Pass`'s result is unsigned, so any nonzero one is passable; a result of 1 gives
  the penalty 30 and any other nonzero one 0.
- No floating point and no random draws.

## 6. Quirks

- **Q1. Slots, not units.** The cursor walks every unit slot of a player, empty or not, and each visit costs
  one step of the player's bank and counts in `playerTickLoad`.
- **Q2. One bank pays for the whole search.** The work of a search, over all its ticks, is charged to the bank
  of the player whose turn it is, which can go far below 0 and keep that player waiting later. The weights
  of a refresh follow the visits, not the searches.
- **Q3. Unseen cells are open.** A cell in bounds whose visibility cell the player has not seen is passable
  at no penalty, whatever is there; a visibility cell outside the grid counts as blocked.
- **Q4. Near enough is a goal.** In the open list, a newly reached cell whose estimate is at most `probe`
  gets the goal bit, so the search can end at a cell outside the goal area, the first such cell it takes.
- **Q5. A cancel does not end the search.** `Cancel` drops a running search with no `pathworld.EndSearch`
  and no event, although the header's comment on `EndSearch` says every implementation calls it when a
  search is dropped. A later `Cancel` of the same requester, after its search ended, still clears `path`,
  `target` and `owner` (recorded: section 7, "Cancel after the search ended"). The dropped running search
  is in no recording.
- **Q6. A full list.** A push into a full open list (*node limit* nodes) adds nothing and gives node 0, which
  the cell then names.
- **Q7. The start node's penalty.** The game pushes the start node with a penalty (bytes 16 and 17 of the
  node) that it leaves unset (the kit's trace schema, `Node`, `excluded`), so its value is not part of the
  behaviour. This module writes 0 there. The bytes are compared while no later push has overwritten the node:
  when a `Step` runs out of budget right after the start of a search, or when the first expansion pushes
  nothing (the start node is then removed, which writes only its `index`, and its penalty stays in the pool
  until a push reuses it). No recording reaches either case, so no recording can tell 0 from another value.
- **Q8. The probe reports DIRECT and the search goes on.** A probe that reaches a goal cell gives the event
  STARTED with DIRECT, and the open list still runs.
- **Q9. The reverse direction twice.** The first expansion of a search considers the reverse of the start's
  heading twice, as turns −4 and 4.

## 7. Examples

From `tests/replay/pathfinding/` of the kit. Every file's map is (224, 102, 2501, 15).

**The region.** `RegionSize` of (224, 102, 2501, 15): *C* = 22848, *N* = 22849, *P* = 2517, *B* = 4096.
The pool is at 408, the items at 408 + 456980 = 457388, the cells at 548784, the dirty words at 640176
(90 words), the requests at 640536, the buckets at 781488 and 797872, the events at 814256, and the size is
814256 + 145408 = 959664 (`o result 959664`).

**Init**, `orders-move-attack.txt`, call at tick 0: *R* = 4341432 and `pathworld.SearchStepLimit` gives 0,
so `stepsPerTick` is 0x535: bytes 72 to 75 are `35050000`. `cells` is 4341432 + 548784 = 4890216
(`689e4a00` at byte 28) and `dirty` 4981608 (`68034c00` at 44); `cellCount` is 22848 (`40590000` at 40); request entry
*e* has *next by requester* *e* + 1, so the region holds 2 at 640536 + 48 and 10 at 640536 + 8 × 56 + 48 =
641032.

**A search found in one tick**, `orders-move-attack.txt`, call at tick 181 of `Step`. Two players play, so
the share is 1333 / 2 = 666. Player 1's cursor is 3455584, the result of `pathworld.LastUnit 1`, so it
becomes the result of `pathworld.FirstUnit 1`, unit 3385864, whose request (notBefore 60, a rectangle 183 to 188 by 76 to 81) is
due at tick 181. `UnitState` gives the cell (210, 87), heading 1030, class 780 and footprint (2, 2); the
visibility grid is 112 by 51; `costScale` is 0x18000.
- The nearest goal cell is (188, 81): 22² + 6² = 520. *D*(start) = 16 × 22 + 6 × 6 = 388, and *H* = 388 ×
  1.5 = 582.
- *S*(210, 87): `InBounds 780 210 87`, then `Seen 1 105 43` (210 / 2 + 2 / 4, 87 / 2 + 0), then `Pass`.
- The probe walks west from (210, 87), meets blocked cells, follows the wall, and probe-visits a goal cell,
  so it gives 0, with `steps` 130 after it (1 + 100 + 29 turns); STARTED DIRECT (Q8). The open list then
  takes 30 nodes in the next turn and finds (188, 81), with `used` 66.
- The route has four points, ((210 × 2 + 2) × 8, (87 × 2 + 2) × 8) = (3376, 1408) first, then (3296, 1328),
  (3040, 1328) and (3024, 1312), the last being (188, 81).
- `Poll` gives `o out x:0000000008aa3300b8975200c0985200b500000000000000` (STARTED: unit 3385864, requester
  5412792, target 5413056, tick 181, status 0; size 24), then the FINISHED of 28 bytes with status 0 and count
  4, then 0.

**A search that ends near the goal**, `orders-tier2-core.txt`, call at tick 1030 of `Step`. The goal is a
circle at (39, 15) of radius 40 (2 cells), the start (32, 13). *D*(start) = 18 × 7 + 7 × 2 − 40 = 100, *H* =
150. The probe gives 21, so STARTED INDIRECT (`o out x:...01000000`), and 21 < 150, so the open list runs.
A cell with *H* ≤ 21 gets the goal bit (Q4): the search finds (39, 12), which is not in the circle ((0)² +
(−3)² = 9 > 4), with four points (528, 224), (560, 192), (640, 192), (640, 208).

**Submit**, `orders-move-attack.txt`, call at tick 181: no entry holds requester 5412792 and *first free* is
1, so entry 1 takes the request; *first free* becomes 2 (entry 1's *next by requester* before the call) and
*request count* 1.

**Cancel of a pending request**, `orders-factory.txt`, call at tick 18 of `Cancel 5406992`: `path` is 0, and
entry 1 holds the requester, with *first free* 2 and *request count* 1. The entry is removed: its requester
becomes 0, its *next by requester* 2 (the old *first free*), *first free* 1 and *request count* 0; its unit
(3315864) stays.

**Cancel after the search ended**, `orders-tier2-core.txt`, call at tick 1030 of `Cancel 5427680`: `object`
is 0 and `path` is 5427680, so `path` and `target` (5413312) become 0 (`w region 92:000000 96:000000`,
Q5); no entry holds the requester. The second `Cancel 5427680` that follows changes nothing.

**The refresh of the weights**, `orders-air.txt`, call at tick 3599 of `Step`: `budgetRefreshCounter`
reaches 150. `playerTickLoad` is 99900 for players 0 and 1 and 0 for the rest, and
`pathworld.UnitsPerPlayer` gives 250 (ten imports, one per player), so *v* is 399 for players 0 and 1, whose
weights become `baseScale` (0x18000), and 0 for players 2 to 9, whose weights become 6 × 0x18000 = 0x90000.
Every `playerTickLoad` becomes 0 at the refresh; the turns of the same `Step` then count their visits, so the
call ends with 666 for players 0 and 1 (`w region 364:9a0200 368:9a0200`).

**Not in any recording:** these are stated by this spec alone, and no recording checks them. The gate's
verdict at this stage is on the recordings alone (ozgb/byte-tactics-moddable#1209); the scenarios that would
reach more of them are judged in the final validation, once the modules are in.
- The slots `Shutdown`, `WriteState`, `SetSearchScale` and `DebugCell`, and `CellsChanged` with the cause 1
  (with the cause 2 it is recorded, and does nothing).
- An event PROGRESS, and the outcomes PARTIAL and FAILED.
- A start in the goal area or off the grid, and a probe at or above *H*(start).
- A search over more than one tick: all four recorded searches start and end in one `Step`. Also the start
  node left in the open list at the end of a call (Q7).
- A goal of kind 2 (ring), of kind 0 or any other kind, and a circle in the probe's DIRECT case.
- The growth of the open list past the recorded capacity, and Q6.
- A `Cancel` of a running search (Q5), and a `Cancel` that drops events.
- A `Submit` that replaces a pending request or changes its unit.
- A player picked for a turn while not playing (below).

The events of the recordings are each recorded search's STARTED and FINISHED FOUND. This module never
records an event of kind PROGRESS, and never gives the status NEAREST (3): section 4 names every event it
records.

**Edges.**
- `pathworld.UnitsPerPlayer` gives 250 in the recordings; a 0 is outside the contract, and the division
  traps (ABI section 13).
- The turn loop picks the next player whose `budget` is above 0 whether or not that player plays: a player
  who stopped playing with a positive bank is served until the bank is spent.
- A requester or a unit of 0 is a number like any other in `Submit`, `Cancel` and the chains (a free entry
  is in no chain). The engine's handles are block offsets and never 0.
- The probe has no limit of its own on its turns: it ends only by the rules of section 4.

**Gaps in the kit.**
- The kit's manifest has no entry and no signatures for pathfinding or for `pathworld`, so the kinds of
  section 2 are this spec's reading of the header. The ABI (section 9.1) names an import group after an
  interface or `services`; `pathworld` is a world view inside `interfaces/pathfinding.h`.
- No WASM gate runs a pathfinding module at this kit, and no recording is of one.
- `WriteState` takes a `StateWriter`, a table of calls, which the ABI gives a module no way to call.

## 8. Kit version

Kit `interfaces-v58` and `reference-traces-v53`, the versions the gate judges against. Interface
`pathfinding` 4.1.
