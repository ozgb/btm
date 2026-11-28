# economy

## 1. Role

Every player has two resources, energy and metal, each with a stock, a storage capacity and running
totals in its `Player` record. Units make and use them: generators and finished units make energy,
extractors and makers make metal, and building, repairing, weapons and cloaking use them. Each unit
slot has a resource account of what its unit made and asked for since the last settlement, kept in the
module's own region, and each player has one more account of its own, a `UnitResources` record of the
block. Other modules change the accounts only by events in economy's inbox (a reset, a load, a request
for this tick's work, an income), and spend a player's stock through the synchronous `Spend`. Every 30
ticks the engine settles each player: the accounts are added up, the stock grows by what was made and
pays what was asked for, as far as it can, and what could not be paid is carried into each account as a
stall that blocks further requests until it is paid off. The settlement also charges each cloaked unit's
upkeep, and tells units whether the unit stays cloaked. A metal extractor's rate is set from the metal
under it when it is placed. In a network game a player gives surplus metal and energy, and its map, to
allied remote players.

## 2. Interface

### Slots (`interfaces/economy.h`, version 5.0)

| Slot | When the engine calls it |
| --- | --- |
| `RegionSize` | at match load, before `Init`, to size the region |
| `Init` | once per match, before the first tick and before any event, with the region the engine allocated |
| `Shutdown` | at match end; the engine frees the region it returns |
| `Apply` | for each event of economy's inbox, when the inbox is delivered (`docs/schedule.md`, "Routes") |
| `Spend` | synchronously, when a weapon fires with a cost or a player gives resources away; the engine delivers the inbox first |
| `UpdatePlayerEconomy` | in the players phase, every 30 ticks, once for each player taking part |
| `UpdateMetalExtraction` | synchronously, when a unit is created or loaded; the engine delivers the inbox first |
| `UpdateResourceSharing` | once a tick in the tail, for the local player |
| `RequestGranted` | just before a unit publishes the REQUEST it decides (a query; the inbox is not delivered first) |
| `GetEnergyUse` | whenever the engine needs a unit type's energy balance (a query) |
| `UnitAccount` | when the engine reads a unit's account, for a saved game or a display (a query) |
| `WriteState` | from the test harness, to dump the state (an engine slot, below) |

`WriteState` is the entry's one `engine` slot: the module does not export it, and the host calls the
engine's native slot instead, with the module's state in the block as the native slot reads it (ABI
section 6, "Engine slots"). It reads the region, so the region's layout (section 3) is the one the
module must keep.

### The manifest entry

The kit's manifest (`wasm/modules.json`) has the entry `economy`, and the signatures
`signatures.economy` and `signatures.econworld`. The module must have that entry, which is:

- `name` and `interface`: `economy`.
- `majors`: `economy` 5, `econworld` 5 (the world view's major is its header's, ABI section 4.2) and
  `network` 2.
- `data_base` 229376 (0x38000) and `table_base` 80: the window is `window` (32768) bytes from
  `data_base`, 0x38000 to 0x3FFFF, and the table range is `table_slots` (32) entries, 80 to 111 (ABI
  sections 2.2 and 2.3). Data, bss and the stack (`stack_size`, 4096 bytes) all lie in the window.
- `exports`, in this order: `RegionSize`, `Init`, `Shutdown`, `Apply`, `Spend`, `UpdatePlayerEconomy`,
  `UpdateMetalExtraction`, `UpdateResourceSharing`, `RequestGranted`, `GetEnergyUse`, `UnitAccount`.
- `engine`: `WriteState`, with the entry's reason.
- `data`: `g_game`, `g_simGlobals`, `g_extTdf`, `g_blockBase`, `g_region` (ABI 8.2: the four every module
  has, then `g_region` for a module whose slots take its region). The module reads `g_game` (the
  `Player` records), `g_blockBase` (to follow and to store block offsets) and `g_region` (the address
  of the region `Init` was given).
- `publishes`: `{"16": 16, "17": 16, "18": 28}`, the effects CLOAK, EXTRACTOR and SHARE with their
  payload sizes in bytes. Every `Emit` of the module passes one of these kinds with exactly that size.
- `blocked`: the kit's entry is `"blocked": true`, so it has no `fuel_budget` and no `size_budget`,
  and the kit has no economy `.wasm` (ABI section 4.2). The gate judges a module of it without budgets
  (ABI section 15, "A blocked entry is judged without budgets"): its size is not judged, and it runs
  with the largest fuel budget of ABI section 11.3.
- Slot kinds (`signatures.economy`):

| Slot | Kinds |
| --- | --- |
| `RegionSize` | `[linear] -> int` |
| `Init` | `[linear, linear, block, int] -> void` |
| `Shutdown` | `[] -> block` |
| `Apply` | `[linear, int] -> int` |
| `Spend` | `[linear] -> int` |
| `UpdatePlayerEconomy`, `UpdateMetalExtraction`, `UpdateResourceSharing` | `[int] -> void` |
| `RequestGranted` | `[int, int] -> int` |
| `GetEnergyUse` | `[int] -> f64` |
| `UnitAccount` | `[int, linear] -> void` |

An `EconUnit` is `int`: the block offset of a `Unit` record (0 for none), which the module passes back
to the world view, and follows (adding `g_blockBase`) only to write `Unit.extraction`. An `EconType` is
`int`, a number it passes back to `econworld.TypeInfo`. A player is an `int`, 0 to 9.

- Imports, every one listed and nothing else (ABI 9.1: the module imports every listed function, even
  one it never calls). The `econworld` group is the slots of `EconomyWorldView` of the same header (a
  world view of an interface header, ABI section 4.2; the recordings write them as `i econworld.<Slot>`).
  The module reaches the world view only through these imports; the `world` argument of `Init` is not
  read.

| Import | Kinds | Used for |
| --- | --- | --- |
| `econworld.Tick` | `[] -> int` | the cloak test, and the sharing ticks |
| `econworld.Difficulty` | `[] -> int` | the handicap |
| `econworld.SharingEnabled` | `[] -> int` | whether sharing runs at all |
| `econworld.Wind` | `[] -> f32` | wind generators |
| `econworld.Tide` | `[] -> f32` | tidal generators |
| `econworld.PlayerInfo` | `[int, linear] -> void` | a player's `EconPlayerInfo` |
| `econworld.Allied` | `[int, int] -> int` | the partners of a share |
| `econworld.FirstUnit`, `econworld.LastUnit` | `[int] -> int` | a player's first and last unit slot |
| `econworld.NextUnit` | `[int] -> int` | the slot after a slot |
| `econworld.UnitSlot` | `[int] -> int` | a unit's account in the region |
| `econworld.UnitInfo` | `[int, linear] -> int` | a unit's `EconUnitInfo`, and whether it is in play |
| `econworld.TypeInfo` | `[int, linear] -> void` | a unit type's `EconTypeInfo` |
| `econworld.CellMetal` | `[int, int] -> int` | the metal under an extractor |
| `econworld.Emit` | `[linear, int] -> void` | the effects CLOAK, EXTRACTOR and SHARE |
| `network.SendShareMapInfo` | `[int, int] -> void` | sharing the map |

- Fuel: the module meters itself by rule version 1 (ABI section 11) and exports `__fuel_set` and
  `__fuel_get`.
- Hooks: `interfaces/economy.h` names `AllowCloak`, `ResourceIncome` and `StatsAccumulate` on
  `UpdatePlayerEconomy` and `ResourceIncome` on the event kind INCOME; every other slot says "Hooks:
  none". The kit defines no hook in the module ABI and the import list above has none, so the module
  raises none; no recording shows one.
- The gate: the entry is blocked, so the oracle takes its `recordings` route (ABI section 15): it replays
  the module on every recording of `tests/replay/economy/` through the generated glue
  (`docs/replay-format.md`, "Against the WASM module"). Its result has `scope` `recordings`, `scenarios`
  `not_judged` and `budgets` `not_judged`, and `verdict` `equal` when every recording is equal.

### Arguments outside the block

All of these are in the caller's memory (`linear`), read or written in place, little-endian, every
field 4 bytes (a signed 32-bit int unless it says unsigned or float):

- **`EconMapInfo`** (`RegionSize`, `Init`), 8 bytes: `unitSlots` (0), `playerCount` (4). The recordings
  write it as `a map (2501,2)` in that order. `playerCount` is not used.
- **The region** (`Init`): the address of its first byte in the block, and `size`, its length, the value
  `RegionSize` gave for the same map. The recordings give its block offset, `a region 6078584`, and
  `a size 120048`.
- **An event** (`Apply`): the address of an `EconomyEventHeader` and the size of its payload. The header
  is 12 bytes: `kind` (0), `player` (4), `unit` (8, an `EconUnit`). The payloads of the header file:
  `EconomyLoad` (60 bytes: the header, then `account`, an `EconAccount` of 48 bytes at 12),
  `EconomyRequest`, `EconomySpend` and `EconomyIncome` (24 bytes each: the header, `resources` at 12,
  unsigned, `energy` at 16 and `metal` at 20, floats). RESET is the header alone. The recordings write
  each field as `a event.<field>` (`a event.account.energy.makePrev f32:0x418c0000`), and the size as
  `a size 24`.
- **`EconomySpend`** (`Spend`), the 24 bytes above, written
  `a spend (3,0,null,2,f32:0x00000000,f32:0x424c0000)`: kind, player, unit, resources, energy, metal.
  `kind` is not read.
- **`EconAccount`**, 48 bytes: `energy` (0) and `metal` (24), each an `EconLedger` of six floats: `make`
  (0), `use` (4), `usePaid` (8), `stall` (12), `makePrev` (16), `usePrev` (20).
- **`UnitAccount`'s `out`**: an `EconAccount` the module fills, written `o out.energyMake ...` to
  `o out.metalUseprev ...`, all twelve.
- **The buffers given to the world view**: `econworld.PlayerInfo` fills an `EconPlayerInfo` (36 bytes:
  `active`, `controller`, `index`, `human`, `shareMetal`, `shareEnergy`, `shareMapping`, `inGame`,
  `storage`, in that order), written as the import's result, a tuple of the nine after ` = `.
  `econworld.TypeInfo` fills an `EconTypeInfo` (44 bytes: `energyMake`, `energyUse`, `metalMake`,
  `extractsMetal`, `windGenerator`, `tidalGenerator`, `cloakCost`, `cloakCostMoving`, `energyStorage`,
  `metalStorage` as floats, then `makesMetal` as an int), written the same way.
  `econworld.UnitInfo` fills an `EconUnitInfo` (60 bytes: `player`, `type`, `structure`, `active`,
  `cloakOn`, `cloakBlocked`, `moving`, `workTime` (unsigned), `buildLeft` and `extraction` (floats),
  `cellX`, `cellY`, `footprintX`, `footprintY`, `hasScript`), written `o uinfo (...)` after the
  import's line, which carries the result. The stub fills it whatever the result.
- **An effect** given to `econworld.Emit`: the address of a payload in the module's memory and its size.
  CLOAK is `EconomyCloak`, 16 bytes: the header (`kind` 16, `player`, `unit`), then `keep` at 12.
  EXTRACTOR is `EconomyExtractor`, 16 bytes: the header (`kind` 17, `player`, `unit`), then `metal` at 12.
  SHARE is `EconomyShare`, 28 bytes: the header (`kind` 18, `player`, `unit` 0), `resources` at 12
  (unsigned), `energy` at 16 and `metal` at 20 (floats), `to` at 24. The recordings write the payload as
  a tuple of its fields and then the size: `i econworld.Emit (16,0,Unit#1,0) 16`,
  `i econworld.Emit (18,0,null,2,f32:0x00000000,f32:0x42e81fa7,1) 28`.

### Events

None. No slot of this module records a sim event (kinds are listed in `docs/sim-events.md`). The events
of this spec are economy's own inbox kinds and effect kinds (`EconomyEventKind`, `EconomyEffectKind`).

## 3. State

### Owned

- **The region**, every byte of it, compared byte for byte at the end of every recorded call (the
  pseudo-field `region` of `docs/replay-format.md`, "A module's region and world view"). It is zero when
  `Init` starts. It is `unitSlots` accounts of 48 bytes, one per unit slot, with nothing before, between
  or after them: the account of slot *k* is the `EconAccount` at byte 48 *k* of the region, energy at 48
  *k* and metal at 48 *k* + 24. *The account of a unit u* is the account of slot `econworld.UnitSlot(u)`.
- **The player accounts**: for each player *p* with a nonzero `Player.econ`, the `UnitResources` record at
  that block offset (52 bytes: `energyMake`, `energyUse`, `energyUsePaid`, `energyStall`,
  `energyMakePrev`, `energyUsePrev` at 0 to 20, the same six for metal at 24 to 44, and `player`, a
  `Ref<Player>`, at 48). Its first 48 bytes are an `EconAccount` in the same order as a region account.
  *The account of player p* is this record of `Game.players[p]`.
- **`Player`** (element *p* of `Game.players`): the stocks `energy` and `metal`, `energyIncome`,
  `metalIncome`, `energyUsage`, `metalUsage`, `energyCapacity`, `metalCapacity` (32-bit floats), and
  `totalEnergyProduced`, `totalMetalProduced`, `totalEnergyConsumed`, `totalMetalConsumed`,
  `energyWasted`, `metalWasted` (64-bit floats).
- **`Unit.extraction`**: a 32-bit float stored in the 4 bytes the layout types as `int` (byte 88 of
  `Unit`), the metal an extractor makes per settlement before any handicap.

### Read

- **`Player`**: `index` (unsigned byte), `econ` (`Ref<UnitResources>`), `energyStorageBonus`,
  `metalStorageBonus`, `shareMetal` and `shareEnergy` (floats: the amounts a sharing player keeps back),
  besides the owned fields above.
- Everything else the module learns of the world comes from the world view: a unit's player, type and
  state from `econworld.UnitInfo`, a type's figures from `econworld.TypeInfo`, a player's set-up from
  `econworld.PlayerInfo`.

### Layout

Element *p* of `Game.players` is at byte 7011 + 331 *p* of `Game` (the address in the data word
`g_game`). The `Player` fields are at their `layout.json` offsets in it: `energy` 140, `energyIncome`
144, `energyUsage` 148, `metal` 152, `metalIncome` 156, `metalUsage` 160, `energyCapacity` 164,
`metalCapacity` 168, `totalEnergyProduced` 172, `totalMetalProduced` 180, `totalEnergyConsumed` 188,
`totalMetalConsumed` 196, `energyWasted` 204, `metalWasted` 212, `energyStorageBonus` 220,
`metalStorageBonus` 224, `shareMetal` 228, `shareEnergy` 232, `econ` 236, `index` 326. Most of these
addresses are not a multiple of the field's size, so block accesses are unaligned. A block offset (a
`Ref`, `Player.econ`, an `EconUnit`) is followed by adding the value of `g_blockBase`, and stored by
subtracting it (ABI 7.1); 0 is null. The value of a data word is the 4 bytes at the address its export
holds (ABI 8.2).

### Between ticks

Nothing outside the region and the fields above. The region's address is the value of `g_region`
whenever a slot runs.

## 4. Behaviour

Within one call, only the order of import calls is observable. The block writes and the region are
compared at the end of the call, so the order they are listed in carries no meaning, except where a
later effect of the same call reads them: then the order given here is the one to use.

Section 5 defines the arithmetic each formula below uses: *add* ("`F` adds *v*"), *allow*, *income*
(`I(F, v, h)`), the *handicap* *h* of a player and the conversion *ftol*. The imports of each slot are
listed in the order they are made; an import a row does not list is not made.

### `RegionSize(map)`

The result is `map.unitSlots` × 48, as an unsigned 32-bit product. No imports, no writes.

### `Init`

The arguments are `map`, `world`, `region` and `size`.

No imports, and no writes: the region stays zero. From this call on, the region is the one given (the
value of `g_region`) until `Shutdown`. `world` and `size` are not read. `map.unitSlots` gives the slot range of the refusals of
`Apply` and `Spend`; it does not change in a match.

### `Shutdown()`

The result is the address of the region `Init` was given. No imports, no writes. A slot called after
`Shutdown` and before the next `Init` is outside the contract.

### `Apply(event, size)`

The result is the event's outcome. *The account of the event* is the account of unit `event.unit`
when it is not 0, and the account of player `event.player` when it is 0.

**Refused events.** Each of these has outcome 0, makes only the imports its row lists, and writes
nothing:

| The event | Imports |
| --- | --- |
| `size` below 12 (`kind` is not read) | none |
| a `kind` other than 0, 1, 2 and 4 (SPEND, 3, comes only through `Spend`; an effect kind is refused too) | none |
| LOAD with `size` below 60; REQUEST or INCOME with `size` below 24 | none |
| `event.player` outside 0 to 9 | none |
| `event.unit` 0 and `Game.players[event.player].econ` null (RESET, REQUEST, INCOME), or LOAD with `event.unit` 0 | none |
| `event.unit` not 0 and `econworld.UnitSlot(event.unit)` outside 0 to `map.unitSlots` − 1 | `econworld.UnitSlot(event.unit)` |

These are the limits ozgb/btm#25 asks for; no recording reaches them. Every other event is applied as
below.

**RESET (0)**, the header alone. Outcome 1.

| `event.unit` | Imports | Writes |
| --- | --- | --- |
| not 0 | `econworld.UnitSlot(event.unit)` | the unit's account: all 48 bytes become 0 |
| 0 | none | the account of player `event.player`: its twelve floats become +0.0, and its `player` becomes the block offset of `Game.players[`*i*`]`, where *i* = `Game.players[event.player].index` (0 to 255): the offset of `Game` from the block's base, plus 7011, plus 331 *i* |

**LOAD (1).** Imports `econworld.UnitSlot(event.unit)`. The unit's account becomes the 48 bytes of
`event.account`. Outcome 1.

**REQUEST (2).** Imports `econworld.UnitSlot(event.unit)` when `event.unit` is not 0, and nothing else.
With *a* the account of the event, by the bits of `event.resources`:

| `resources` & 3 | Writes | Outcome |
| --- | --- | --- |
| 3 | *a*.`energy.use` adds `energy` and *a*.`metal.use` adds `metal`; then, when *a*.`energy.stall` and *a*.`metal.stall` both allow, *a*.`energy.usePaid` adds `energy` and *a*.`metal.usePaid` adds `metal` | 1 when both allow, else 0 |
| 1 | *a*.`energy.use` adds `energy`; then, when *a*.`energy.stall` allows, *a*.`energy.usePaid` adds `energy` | 1 when it allows, else 0 |
| 2 or 0 | the same with `metal`, `metal.use`, `metal.stall` and `metal.usePaid` (Q10) | as for 1 |

**INCOME (4).** Imports, in order: `econworld.UnitSlot(event.unit)` when `event.unit` is not 0, then the
handicap's imports for player `event.player` (section 5). With *a* the account of the event and *h* the
handicap of `event.player`: when bit 0 of `resources` is set, *a*.`energy.make` becomes
`I(`*a*.`energy.make`, `energy`, *h*`)`; when bit 1 is set, *a*.`metal.make` becomes
`I(`*a*.`metal.make`, `metal`, *h*`)`. The handicap's imports are made whatever the bits. Outcome 1.

### `Spend(spend)`

The refusals of `Apply` hold for `Spend` too, read as an event of 24 bytes (`spend.kind` is not read):
`spend.player` outside 0 to 9, `spend.unit` 0 with a null `econ`, or a slot outside the range, gives 0
with no write. Otherwise *a* is the account of `spend.unit` (importing `econworld.UnitSlot(spend.unit)`)
when it is not 0, else the account of player `spend.player` (no import), and *o* is
`Game.players[spend.player]`:

| `resources` & 3 | Case | Writes | Result |
| --- | --- | --- | --- |
| 3 | *o*.`energy` ≥ `energy` and *o*.`metal` ≥ `metal` | *o*.`energy` becomes *o*.`energy` ⊖ `energy`, *o*.`metal` becomes *o*.`metal` ⊖ `metal`; *a*.`energy.use` adds `energy`, *a*.`metal.use` adds `metal` | 1 |
| 1 | *o*.`energy` ≥ `energy` | *o*.`energy` becomes *o*.`energy` ⊖ `energy`; *a*.`energy.use` adds `energy` | 1 |
| 2 or 0 | *o*.`metal` ≥ `metal` | *o*.`metal` becomes *o*.`metal` ⊖ `metal`; *a*.`metal.use` adds `metal` (Q10) | 1 |
| any | otherwise | none | 0 |

### `RequestGranted(unit, resources)`

Imports `econworld.UnitSlot(unit)`. With *a* the unit's account, the result is, by `resources` & 3: for 3,
1 when *a*.`energy.stall` and *a*.`metal.stall` both allow, else 0; for 1, 1 when *a*.`energy.stall`
allows, else 0; for 2 or 0, the same with *a*.`metal.stall`. No writes.

### `UnitAccount(unit, out)`

Imports `econworld.UnitSlot(unit)`. The 48 bytes of `out` become the unit's account. No block writes.

### `GetEnergyUse(type)`

Imports `econworld.TypeInfo(type)` first; with *t* its result, the rest by the first row that matches:

| Case | Then imports | Result |
| --- | --- | --- |
| *t*.`energyUse` ≠ 0 (a NaN is ≠ 0) | none | *t*.`energyUse`, widened exactly to 64 bits |
| *t*.`windGenerator` > 0 | `econworld.Wind` | −(*w* × *t*.`windGenerator`), *w* the wind, the exact product as a 64-bit float |
| *t*.`tidalGenerator` > 0 | `econworld.Tide` | −(*d* × *t*.`tidalGenerator`), *d* the tide, likewise |
| otherwise | none | +0.0 |

No writes. A generator row's product is exact in 64 bits, and a product of +0.0 gives −0.0, the
pattern 0x8000000000000000.

### `UpdateMetalExtraction(unit)`

Imports `econworld.UnitInfo(unit)` (its result is ignored: *u* is the info it filled), then
`econworld.TypeInfo(`*u*`.type)` (*t*). When *t*.`extractsMetal` > 0 is false, nothing more happens.
Otherwise:

- imports `econworld.CellMetal(x, y)` once for each cell of the footprint: *y* from *u*.`cellY` to
  *u*.`cellY` + *u*.`footprintY` − 1, and for each *y*, *x* from *u*.`cellX` to *u*.`cellX` +
  *u*.`footprintX` − 1 (32-bit sums; a size of 0 or less on either axis has no cells). Cells off the map
  are asked for like any other.
- *W* is the 16-bit sum (section 5) of *m* + 1 over every cell whose result *m* is 0 or more; a result
  below 0 (−1, a cell off the map) adds nothing.
- `Unit.extraction` of the record at block offset `unit` becomes f32(*t*.`extractsMetal` × *W*), the
  product exact. This holds whatever `econworld.UnitInfo` returned (Q11).
- When *u*.`hasScript` ≠ 0, imports `econworld.Emit` of an EXTRACTOR, after the last `CellMetal`:
  `kind` 17, `player` *u*.`player`, `unit` `unit`, `metal` *W* (sign-extended to 32 bits), size 16.

### `UpdatePlayerEconomy(player)`

*p* is `Game.players[player]` and *e* the account of player `player`. *pinfo* is the
`EconPlayerInfo` of the first import.

**The imports**, in order:

1. `econworld.PlayerInfo(player)` (*pinfo*), `econworld.LastUnit(player)` (*last*),
   `econworld.FirstUnit(player)` (*u*).
2. *The first walk*: while *u* ≤ *last* (as unsigned 32-bit integers), for the slot *u*:
   1. `econworld.UnitInfo(u)`. When it returns 0 the slot is not in play, and the next import is the
      `NextUnit` of item 6.
   2. `econworld.TypeInfo(`*ui*`.type)`, with *ui* the info it filled; *ti* is its result.
   3. `econworld.UnitSlot(u)`: *a* is the unit's account.
   4. The handicap's imports for player *ui*`.player` (section 5): *h* is its handicap.
   5. Then, as the unit's rows below say: `econworld.Wind` or `econworld.Tide` (the production table),
      `econworld.Tick` (the cloak), and `econworld.Emit` of its CLOAK.
   6. `econworld.NextUnit(u)`, which gives the next *u*.
3. `econworld.FirstUnit(player)` (*u*), and *the second walk*: while *u* ≤ *last*,
   `econworld.UnitInfo(u)`; when it returns nonzero, `econworld.UnitSlot(u)`; then
   `econworld.NextUnit(u)`, which gives the next *u*. `econworld.LastUnit` is not asked again.

The units of the first walk with a nonzero `UnitInfo` are *in play*. The second walk settles each slot
whose `UnitInfo` is nonzero in that walk.

**Each unit in play, in walk order.** Its effects take place unit by unit: a cloak payment lowers a
stock that a later unit's cloak test reads, and each sum rounds on what came before.

1. *Charges and production*, by the first row that matches, with *v* = *ti*.`energyUse`. *Charge v* means:
   when *v* ≥ 0, *a*.`energy.use` adds *v*, and the unit is *funded* when *a*.`energy.stall` allows, and
   then *a*.`energy.usePaid` adds *v* too; when *v* is below 0 or a NaN, *a*.`energy.make` becomes
   `I(`*a*.`energy.make`, −*v*, *h*`)` and the unit is not funded.

| The unit | Effect |
| --- | --- |
| *ui*.`structure` ≠ 0 and *ui*.`active` = 0 | none |
| *ui*.`structure` ≠ 0 and *ui*.`active` ≠ 0 | charge *v*, then the first row of the production table that matches |
| *ui*.`structure` = 0, and *ui*.`active` ≠ 0 or *ui*.`moving` ≠ 0 | charge *v* |
| any other | none |

| Production (a structure that is active) | Effect |
| --- | --- |
| *ti*.`extractsMetal` > 0 | when funded: *a*.`metal.make` becomes `I(`*a*.`metal.make`, *ui*.`extraction`, *h*`)` |
| *ti*.`makesMetal` ≠ 0 | when funded: *a*.`metal.make` becomes `I(`*a*.`metal.make`, f32(*ti*.`makesMetal`), *h*`)`: the integer rounded to the nearest 32-bit float, ties to even |
| *ti*.`windGenerator` > 0 | imports `econworld.Wind` (*w*); *a*.`energy.make` becomes `I(`*a*.`energy.make`, *w* × *ti*.`windGenerator`, *h*`)` |
| *ti*.`tidalGenerator` > 0 | imports `econworld.Tide` (*d*); *a*.`energy.make` becomes `I(`*a*.`energy.make`, *d* × *ti*.`tidalGenerator`, *h*`)` |
| any other | none |

A row that matches excludes the rows after it, whether or not its effect happens (Q5).

2. *Finished units*: when *ui*.`buildLeft` = 0 or is a NaN, *a*.`energy.make` becomes
   `I(`*a*.`energy.make`, *ti*.`energyMake`, *h*`)`, then *a*.`metal.make` becomes
   `I(`*a*.`metal.make`, *ti*.`metalMake`, *h*`)`; the capacity sum *Cm* adds *ti*.`metalStorage` and
   *Ce* adds *ti*.`energyStorage`.
3. *Cloak*: when *pinfo*.`active` ≠ 0 and *pinfo*.`controller` = 3, nothing (no import). Otherwise the
   unit's CLOAK is emitted, with *keep* as follows, as the import `econworld.Emit` of `kind` 16,
   `player` *ui*.`player`, `unit` *u*, `keep` *keep*, size 16:

| Case | Imports before the `Emit` | *keep*, and the payment |
| --- | --- | --- |
| *ui*.`cloakOn` = 0, or *ui*.`cloakBlocked` ≠ 0 | none | 0 |
| otherwise, *ui*.`workTime` > *T* (unsigned), *T* the result of `econworld.Tick` | `econworld.Tick` | 0 |
| otherwise, the cost *c* is not payable | `econworld.Tick` | 0 |
| otherwise | `econworld.Tick` | 1: *o*.`energy` becomes *o*.`energy` ⊖ *c*, and *a*.`energy.use` adds *c* |

   *o* is `Game.players[`*ui*`.player]`, the unit's own player, which need not be `player` (Q6). The cost
   *c* is ftol(*ti*.`cloakCostMoving`) when *ui*.`moving` ≠ 0, else ftol(*ti*.`cloakCost`), converted back
   to a float (section 5). It is *payable* when *c* ≤ *o*.`energy`, or when either is a NaN.
4. *Sums*: the unit's account goes into the four energy sums and the four metal sums, *M*, *U*, *P*, *S*,
   of its `make`, `use`, `usePaid` and `stall`, after the steps above. Each sum starts at +0.0 for the
   call and adds each unit in play in walk order (section 5).

**The player.** After the first walk: *e* adds last to the sums, with one difference: for each resource,
*M*\* is *M* + *e*.`make` rounded to 64 bits, and *M* becomes *M*\* rounded to 32 bits (section 5); *U*,
*P* and *S* add *e*'s `use`, `usePaid` and `stall` as the units' did. *Ce* and *Cm* start at +0.0 for the
call; when *pinfo*.`storage` ≠ 0, *Ce* adds *p*.`energyStorageBonus` and *Cm* adds *p*.`metalStorageBonus`.
Then, for each resource *X* (energy, then metal), with its sums and its capacity *C* (*Ce* or *Cm*):

- *p*.`XCapacity` becomes *C*, *p*.`XIncome` becomes *M* and *p*.`XUsage` becomes *U*.
- *p*.`totalEnergyProduced` adds *M*\* for energy, and *p*.`totalMetalProduced` adds *M* for metal (Q1).
  *p*.`totalXConsumed` adds *U*.
- The amount *A* is *M*\* + *p*.`X`, rounded to 32 bits. The stall is paid first, then the paid use,
  each as far as *A* goes:

| Case | Stall ratio *rs* | Remainder *L* |
| --- | --- | --- |
| *S* ≤ *A*, or either is a NaN | 1 | *A* − *S* |
| otherwise | *A* / *S* | 0 |

| Case | Paid-use ratio *rp* | Stock left *B* |
| --- | --- | --- |
| *P* ≤ *L*, or either is a NaN | 1 | *L* − *P* |
| otherwise | *L* / *P* | 0 |

- When *B* > *C*, *p*.`X` becomes *C* and *p*.`XWasted` adds *B* − *C*. Otherwise *p*.`X` becomes *B*.

**Settlement.** The account of every unit the second walk settles, in walk order, and then *e*, ends the
call in this state, for each resource, with the ratios *rs* and *rp* of that resource:

- `makePrev` is `make`, and `usePrev` is `use` (their values after the first walk);
- `stall` is (`stall` − *rs* × `stall`) + (`usePaid` − *rp* × `usePaid`), with the values after the first
  walk (section 5);
- `make`, `use` and `usePaid` are +0.0.

A walk with no unit in play makes only its walk imports, and the player part runs as for any walk, with
the sums before *e* at +0.0. A *last* below the first *u* has no slots at all. A null *p*.`econ` is
outside the contract.

### `UpdateResourceSharing(player)`

*s* is `Game.players[player]`. The imports, and what each decides, in order:

1. `econworld.SharingEnabled()`. When it returns 0, nothing more happens.
2. `econworld.PlayerInfo(player)`: *me*.
3. `econworld.Tick()` (*T*). When *T* is a multiple of 60 (unsigned; 0 is one), the metal share and then
   the energy share, below.
4. `econworld.Tick()` again (*T*′). When *T*′ is a multiple of 450 and *me*.`shareMapping` ≠ 0, the share
   of the map, below.

A *partner* is a player *i* whose `EconPlayerInfo` has `active` ≠ 0, `controller` = 3, `index` ≠ 10,
`inGame` ≠ 0 and `human` ≠ 0. *The partner test of i* is the import `econworld.PlayerInfo(i)`, and when
*i* is a partner by it, the import `econworld.Allied(player, i)`; *i* passes when that returns nonzero.

**The metal share.** When *me*.`shareMetal` ≠ 0 and *s*.`metal` > *s*.`shareMetal`: for *i* from 0 to 9,
the partner test of *i* (`player` itself included); the *recipient* is the last *i* that passes and whose
`Game.players[i].metal` < *s*.`metal`, or of which either is a NaN. When there is a recipient *r* and
*r* ≠ `player` (Q3), the import `econworld.Emit` of a SHARE: `kind` 18, `player` *me*.`index`, `unit` 0,
`resources` 2, `energy` +0.0, `metal` the share amount (section 5) of *s*.`metal`, *s*.`shareMetal`,
the factor *k3* = 0x3eaaaaab, `Game.players[r].metalCapacity` and `Game.players[r].metal`, and `to` the
`index` of *r*'s `EconPlayerInfo`; size 28. When no *i* is a recipient, or the recipient is `player`, no
`Emit`. When the first condition fails, no import of the loop is made.

**The energy share**, after the metal share's imports: the same with *me*.`shareEnergy`, `energy`,
`shareEnergy`, `energyCapacity`, the factor 0.5, and a SHARE with `resources` 1, `energy` the amount
and `metal` +0.0.

**The share of the map.** For *i* from 0 to 9, the partner test of *i*; when *i* passes and neither
*me*.`index` nor the `index` of *i*'s info is 10 as an unsigned byte (their low 8 bits), the import
`network.SendShareMapInfo(a, b)` right after the `Allied`, with *a* and *b* those low 8 bits,
zero-extended. `player` itself is sent its own map when it passes (Q4).

No block or region writes.

## 5. Exactness

**Notation.** f32(*x*) rounds a real *x* to the nearest 32-bit float, ties to even, and f64(*x*) to the
nearest 64-bit float. ⊕ and ⊖ are the 32-bit operations: *a* ⊕ *b* = f32(*a* + *b*). A comparison is of
the exact values, and is false when an operand is a NaN unless the text says "or either is a NaN". A
float field or argument is a 32-bit float unless section 3 says 64.

- ***Add***: "`F` adds *v*" means `F` becomes `F` ⊕ *v*. For a 64-bit field (the totals and the
  wasted amounts) it means `F` becomes f64(`F` + *v*), with *v* as the text gives it (a 32-bit sum, *M*\*,
  or a 64-bit difference).
- ***Allow***: a stall allows a request when it is ≤ 0 or a NaN (Q2).
- ***The handicap*** *h* of a player *q*: the import `econworld.PlayerInfo(q)`; when its `active` ≠ 0
  and its `controller` = 2, the import `econworld.Difficulty()`, and *h* is 0.5 when it returns 0 and
  *h7* = f64(0.7) = 0x3fe6666666666666 when it returns 1. In every other case there is no handicap (and
  no `Difficulty` import when the first test fails).
- ***Income*** `I(F, v, h)`, where *v* is exact (a float, or an exact product of two floats; an
  integer enters as f32 of it): with a handicap, f32(f64(`F` + f64(*v* × *h*))); without one, f32(f64(`F` + *v*)).
- **Products in `I`:** *w* × `windGenerator` and *d* × `tidalGenerator` are exact products, not rounded
  to 32 bits before they enter `I`.
- **The sums of `UpdatePlayerEconomy`**: after each unit's term, a sum is the 32-bit float nearest the
  exact sum of its value before and the term (*M* ⊕ *a*.`energy.make`, and so on). *M*\* is
  f64(*M* + *e*.`make`) and *M* then becomes f32(*M*\*). *A* is f32(f64(*M*\* + *p*.`X`)). *Ce* and *Cm*
  add with ⊕.
- **The ratios:** *A*, *S* and *P* are 32-bit floats. In the first row, *L* is f64(*A* − *S*), not
  rounded to 32 bits, and *rs* is 1; otherwise *L* is 0 and *rs* is f32(*A* / *S*), the IEEE quotient
  (*S* = 0 with *A* < 0 is −∞). In the second row *rp* is 1 and *B* is f32(f64(*L* − *P*)); otherwise *B*
  is 0 and *rp* is f32(f64(*L* / *P*)), the 64-bit quotient rounded to 32 bits. The wasted amount adds
  f64(*B* − *C*).
- **The new stall of an account:** with *s* = `stall` and *q* = `usePaid` (both 32-bit), the 64-bit
  values *x* = f64(*s* − f64(*rs* × *s*)) and *y* = f64(*q* − f64(*rp* × *q*)), and the new `stall` is
  f32(f64(*x* + *y*)).
- **ftol, the cloak cost:** the 32-bit float is widened to 64 bits (exactly). A NaN, an infinity or a
  magnitude of 2^63 or more gives 0. Otherwise the value is truncated toward zero to an exact integer *t*,
  and the result is *t* mod 2^32, read as a signed 32-bit integer. Within the int range this is plain
  truncation; outside it the value wraps: 2147483648.0 gives −2147483648, 2147483649.0 gives
  −2147483647, −2147483649.0 gives 2147483647, 4294967296.0 gives 0, 2621440000.0 gives −1673527296,
  1e15 gives −1530494976, and 1e20 gives 0. The cost *c* is then f32 of that integer. A module must not
  trap on any of these values (Q7).
- **Extraction:** *W* is a signed 16-bit integer: it starts at 0, each cell adds *m* + 1, and it wraps
  modulo 65536 into −32768 to 32767 (Q9). `Unit.extraction` = f32(*t*.`extractsMetal` × *W*), the
  product exact. The `metal` of the EXTRACTOR is *W* sign-extended to 32 bits.
- **The share amount** of a stock *x*, a kept amount *keep*, a factor *k*, a capacity *cap* and the
  recipient's stock *y*: *D* = f64(f64(*x* − *keep*) × *k*) and *R* = f64(*cap* − *y*); the amount is
  f32(*D*) when *D* < *R* or either is a NaN, else f32(*R*). It is emitted whatever its sign: a recipient
  with no room gives 0 or less.
- **Ticks:** the results of `econworld.Tick` and *ui*.`workTime` are compared as unsigned 32-bit
  integers, and the multiples of 60 and 450 are of the unsigned value.
- **`RegionSize`:** `unitSlots` × 48 modulo 2^32.
- **NaN (not recorded):** a NaN reaches a field only from a NaN or an infinity that the world view or an
  event gives, or when a negative income exceeds the stock with no stall owed (*S* = 0, *A* < 0: then
  *rs* is −∞ and the new `stall` of an account with `stall` = 0 is a NaN). The kit's ABI (section 3.2)
  lets the engine canonicalise NaNs, and a module must not let a result depend on a NaN's bit pattern, so
  this spec fixes no pattern. The rules above that say "or either is a NaN" state what a NaN does in each
  test; everything else about a NaN is outside the contract.

The roundings above state results: an implementation may compute them in any way that gives the same
bits.

## 6. Quirks

- **Q1:** `totalEnergyProduced` grows by the 64-bit energy sum *M*\*, while `totalMetalProduced` grows by
  the 32-bit metal sum *M*.
- **Q2:** a stall that is NaN allows a request, and a NaN in a settlement's tests takes the first row.
- **Q3:** a player shares a third (0x3eaaaaab) of its surplus metal and half of its surplus energy, with
  the partner of the highest index whose stock is below its own; when that partner is the sharing player
  itself, nothing is shared, even when a lower partner qualifies.
- **Q4:** shared map information can go to the sharing player itself.
- **Q5:** an active structure with an `energyUse` below 0 earns that energy, and an extractor or a metal
  maker with a negative `energyUse` makes no metal (it is never funded); a structure that extracts makes
  no energy from wind or tide.
- **Q6:** the capacities and the stocks settled are `player`'s, but the handicap and the cloak payment
  are each unit's own player's (*ui*.`player`).
- **Q7:** the cloak cost is truncated to a whole number before it is paid, and wraps outside the int
  range.
- **Q8:** a spend (`Spend`, and the cloak payment) takes the amount from the stock at once and also adds
  it to the account's `use`.
- **Q9:** *W* wraps at 16 bits.
- **Q10:** a REQUEST or a `Spend` whose `resources` has neither bit set is a metal one.
- **Q11:** `UpdateMetalExtraction` writes `Unit.extraction` whatever `econworld.UnitInfo` returned.

## 7. Examples

Values are given as decimals, with the 32-bit pattern where the rounding matters.

1. **`tests/replay/economy/duel-shore-to-shore.txt`, the call at tick 600 of
   `UpdatePlayerEconomy(0)`.** *pinfo* has `active` 1, `controller` 2 and `storage` 1: a computer player, and
   `econworld.Difficulty` returns 0, so *h* = 0.5 for every unit of player 0. In play: `Unit#1` (type 34:
   not a structure, active, finished, `energyMake` 25, `metalMake` 1, `energyUse` 0) and `Unit#2` (type
   78: a structure, active, `energyUse` 3, `extractsMetal` 0.001, *ui*.`extraction` 2.016, 0x40010625).
   The `make`, `use`, `usePaid` and `stall` of both accounts and of *e* start at 0 (their `makePrev` and
   `usePrev` hold the last settlement), and neither unit cloaks: the two `Emit`s are `(16,0,Unit#1,0)` and
   `(16,0,Unit#2,0)`.
   - `Unit#1`: `energy.make` = 12.5, `metal.make` = 0.5. `Unit#2`: `use` = `usePaid` = 3, funded, so
     `metal.make` = f32(2.016 × 0.5) = 0x3f810625 (1.008).
   - Energy: *M* = 12.5, *U* = *P* = 3, *S* = 0. `energy` is 4563.5 (0x458e9c00): *A* = 4576, *L* =
     4576, *B* = 4573 (0x458ee800), below *Ce* = 5000 (the storage bonus 0x459c4000). `energyUsage`
     becomes 3, `totalEnergyProduced` goes from 250 to 262.5 (0x4070680000000000).
   - Metal: *M* = 0.5 ⊕ 1.008 = 0x3fc10625 (1.508). `metal` is 0x459ad40f, and becomes
     f32(*M*\* + 0x459ad40f) = 0x459ae01f. `totalMetalProduced` becomes 0x4029083128000000.
   - Settlement: `Unit#2`'s `energy.usePrev` is 3 (region bytes 116 to 119: slot 2 at 96, plus 20);
     every `make`, `use`, `usePaid` and `stall` is 0.
2. **`tests/replay/economy/console-economy.txt`, the call at tick 40 of `Apply`**: an INCOME (kind 4) of
   player 1, `unit` 0, `resources` 2, `metal` 51 (0x424c0000), size 24. `econworld.PlayerInfo(1)` gives
   (1,2,1,...) and `econworld.Difficulty` 1: *h* = *h7*. Player 1's account (`UnitResources#2`) has
   `metalMake` 0, and it becomes f32(f64(0 + f64(51 × *h7*))) = 0x420ecccd (35.7). Outcome 1.
3. **The same file, the call at tick 40 of `Spend`**: `(3,0,null,2,f32:0x00000000,f32:0x424c0000)`,
   metal only, from player 0's own account: `Player#0.metal` is 51 and becomes 0,
   `UnitResources#1.metalUse` becomes 51. Result 1, no imports.
4. **The same file, the call at tick 169 of `Apply`**: a REQUEST of `Unit#251` (slot 251), `resources` 3,
   `energy` 0x4051456b, `metal` 0x3f15675a. The account's stalls are 0, so `use` and `usePaid` of both
   resources become those amounts: the region's bytes 12052 to 12059 (slot 251 at 12048, `energy.use` and
   `energy.usePaid`) are `6b4551406b455140`, and bytes 12076 to 12083 `5a67153f5a67153f`. Outcome 1.
5. **`tests/replay/economy/console-edge.txt`, the calls at tick 1321 of `RequestGranted(Unit#1, 3)` and
   `Apply`.** The settlement at tick 1320 left `Unit#1`'s `energy.stall` at 0x421518e1 (37.27), above 0:
   `RequestGranted` returns 0, and the REQUEST that follows (`resources` 3, `energy` 0x40393e93,
   `metal` 0x3e8e38e3) writes only `energy.use` and `metal.use` (region bytes 52 and 76) and has
   outcome 0.
6. **`tests/replay/economy/edges-economy-targeting.txt`, the call at tick 5 of
   `UpdateMetalExtraction(Unit#2)`.** *u*: cell (−1,−1), footprint (3,3), `hasScript` 1; type 78,
   `extractsMetal` 0x3a83126f. The nine `CellMetal` imports are (−1,−1), (0,−1), (1,−1), (−1,0), (0,0),
   (1,0), (−1,1), (0,1), (1,1); the five off the map give −1, and the four others 3, so *W* = 4 × 4 = 16.
   `extraction` becomes f32(0.001 × 16) = 0x3c83126f, and the import `econworld.Emit (17,0,Unit#2,16) 16`
   follows the last `CellMetal`.
7. **The same file, the call at tick 30 of `UpdatePlayerEconomy(0)`: a cloak.** `Unit#3` (type 80, a
   structure, not active, `cloakOn` 1, `cloakBlocked` 0, `workTime` 0) has `cloakCost` 7.0. *pinfo* is
   (1,1,...), a local player, so CLOAKs are emitted. The imports for it are `UnitInfo`, `TypeInfo`,
   `UnitSlot` (3), `PlayerInfo 0` (no `Difficulty`: the controller is 1), `Tick` (30; the
   `workTime` 0 is not above it), and `Emit (16,0,Unit#3,1) 16`: player 0's `energy` pays 7, and the account's
   `energy.use` is 7, which the settlement moves to `energy.usePrev` (region bytes 164 to 167,
   0x40e00000).
8. **`tests/replay/economy/console-iwin.txt`, a call at tick 0 of `GetEnergyUse(135)`**: `energyUse` 0
   and `windGenerator` 30, so `econworld.Wind` is imported; it returns +0.0, and the result is
   −(0 × 30) = −0.0, `f64:0x8000000000000000`.
9. **`tests/replay/economy/load-midgame.txt`, the first call at tick 900 of `Apply`**: a LOAD of
   `Unit#1` (slot 1), size 60, with `energy.makePrev` 0x418c0000 (17.5) and `metal.makePrev` 0x3f333333
   (0.7) and every other float 0: the region's bytes 64 to 67 and 88 to 91 take those patterns. Outcome 1.
10. **`tests/replay/economy/console-iwin.txt`, the first call at tick 0 of `Apply`**: a RESET of player 0
    with `unit` 0. `Player#0.econ` is `UnitResources#1` and `Player#0.index` is 0, so the account's
    floats stay 0 and its `player` becomes the offset of `Player#0`. Outcome 1, no imports.
11. **`tests/replay/economy/share-all.txt`, the call at tick 900 of `UpdateResourceSharing(0)`.**
    `SharingEnabled` 1; *me* is (1,1,0,1,1,1,1,1,1): index 0, sharing metal, energy and the map. `Tick`
    gives 900, a multiple of 60 and of 450. `Player#0.metal` is 0x43e017bd (448.19) and its `shareMetal`
    100; of the ten players only player 1 is a partner (active, `controller` 3, `index` 1, `human` 1, `inGame` 1; `Allied 0 1` = 1; and its
    `metal` 0 is below), with `metalCapacity` 100000. The amount is f32(f64(348.19 × 0x3eaaaaab)) =
    0x42e81fa7, and the import is `Emit (18,0,null,2,f32:0x00000000,f32:0x42e81fa7,1) 28`. Energy: the
    ten `PlayerInfo`s and the one `Allied` again, `energy` 0x434e1760 (206.09) less 100, times 0.5, gives
    0x42542ec0: `Emit (18,0,null,1,f32:0x42542ec0,f32:0x00000000,1) 28`. Then `Tick` (900) and the map:
    the ten `PlayerInfo`s, `Allied 0 1`, and `network.SendShareMapInfo 0 1` right after it. At ticks 899
    and 901 the call makes only `SharingEnabled`, `PlayerInfo 0` and the two `Tick`s.

**Not recorded in the kit's recordings** (`docs/replay-format.md`, "Not recorded"), with this spec as the
authority for each:

- Every refused event of `Apply` and `Spend` (section 4), a `kind` outside 0, 1, 2 and 4 included, and a
  `Spend` that the stock cannot pay.
- A REQUEST with `resources` 2 or 0, and a `Spend` with `resources` 0 (Q10).
- `UpdatePlayerEconomy`: a player whose *pinfo* has `controller` 3 (no CLOAK at all); the moving cloak
  cost, `cloakBlocked` set, a `workTime` above the tick, a cost that is not payable or not whole, and a
  NaN or out-of-range cost (Q7); a nonzero wind or tide (every recorded `Wind` and `Tide` is 0); a
  handicap of 0.7 in a settlement (difficulty 1); a structure with a negative `energyUse` (Q5); a metal
  maker; a unit that is not a structure and not active but moving; a unit of another player than
  `player` (Q6); *pinfo*.`storage` 0; and the case *S* = 0 with *A* < 0.
- `UpdateMetalExtraction` for a unit with no script, with a footprint of 0 or less, or with a wrap of *W*
  (Q9).
- `UpdateResourceSharing`: a share of zero or less, a recipient that is the sharing player (Q3), the
  player as its own partner in the share of the map (Q4), an index of 10, and the partner conditions
  `inGame` and `human` at 0 for an active remote player.
- `GetEnergyUse` with a nonzero wind or tide.
- The 64-bit roundings of *M*\*, *A*, *L* and the quotient *L* / *P*: a 32-bit reading of them gives the
  same results on every recording, and so does adding *M* in place of *M*\* to `totalEnergyProduced`
  (Q1). This spec is the authority for them.

## 8. Kit version

Kit `interfaces-v72` and `reference-traces-v57`, the versions the gate judges against. Interface
`economy` 5.0.
