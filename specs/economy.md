# economy

## 1. Role

Every player has two resources, energy and metal, each with a stock, a storage capacity and running
totals. Units make and use them: generators and the commander make energy, extractors and makers make
metal, and building, weapons and cloaking use them. Each unit keeps a resource account of what it made
and asked for since the last settlement, and the player has one more account of its own. Every 30
ticks the engine settles each player: the accounts are added up, the stock grows by what was made and
pays what was asked for, as far as it can, and what could not be paid is carried into each account as a
stall that blocks further requests until it is paid off. In a network game a player can also share
surplus metal and energy, and the map, with allied remote players.

## 2. Interface

### Slots (`interfaces/economy.h`, version 3.0)

| Slot | When the engine calls it |
| --- | --- |
| `UpdatePlayerEconomy` | every 30 ticks, once for each player taking part, in player order |
| `UpdateResourceSharing` | for the local player, from the in-game screen |
| `GetEnergyUse` | whenever the engine or the interface needs a unit type's energy balance |
| `UpdateMetalExtraction` | when a unit that may extract metal is placed on the map |
| `Reset` | when a unit is created, and for each player's own account at match start |
| `SaveUnitAccounts`, `LoadUnitAccounts` | for each unit, when a game is saved or loaded |
| `RequestEnergy` | when a unit asks for energy for work in progress |
| `RequestEnergyAndMetal` | when a unit asks for energy and metal for work in progress, such as building |
| `SpendEnergy`, `SpendMetal` | when a player gives energy or metal away |
| `SpendEnergyAndMetal` | when a weapon with a per-shot cost fires |

### The manifest entry

The kit's manifest (`wasm/modules.json`) carries the slot signatures of this module (`signatures.economy`)
and its interface major, 3 (`majors.economy`). It has no build entry for it: the build picks the four
figures of the last bullet below. This section gives the entry the module must have. The kinds `f32` and
`f64` are IEEE 754 floats of 32 and 64 bits that cross the boundary bit for bit (ABI section 7).

- `name` and `interface`: `economy`.
- `exports`, in this order: `UpdateResourceSharing`, `GetEnergyUse`, `UpdatePlayerEconomy`,
  `UpdateMetalExtraction`, `Reset`, `SaveUnitAccounts`, `LoadUnitAccounts`, `RequestEnergy`,
  `RequestEnergyAndMetal`, `SpendEnergy`, `SpendMetal`, `SpendEnergyAndMetal`.
- `data`: `g_game`, `g_simGlobals`, `g_extTdf`, `g_blockBase` (ABI 8.2). ABI 8.2 says a module exports
  four data words, and ABI 9.1 makes its exports equal its `exports`, its `data` and the two fuel
  functions; a module that lists and exports all four meets both sections. The module reads `g_game` and
  `g_blockBase` only.
- `majors`: the majors the module is built against (ABI 10.1, the values of the manifest's `majors`):
  `economy` 3, and for its import groups `units` 1, `los_radar` 2, `files` 2, `players` 1, `network` 1.
- `data_base`, `table_base`, `fuel_budget` and `size_budget`: the build picks them, as ABI 11.1 and 11.3
  say a budget belongs to its build. `fuel_budget` is set from the build's own runs of the conformity gate
  (ten times the most fuel any tick burns, all the slot calls of a tick together, ABI 11.2 and 11.3), and
  `size_budget` is the most bytes the build's `.wasm` may be. `data_base` and `table_base` must meet the
  rules of ABI 2: a window of `window` (65536) bytes at or above 0x10000 that ends at or below the scratch
  base 0xF0000 and does not overlap `wind_meteors`'s window (0x30000 to 0x3FFFF); a range of
  `table_slots` (32) entries inside the 80 of the table that does not overlap entries 48 to 79 of
  `wind_meteors`, so a `table_base` from 0 to 16.
- Slot kinds:

| Slot | Kinds |
| --- | --- |
| `UpdateResourceSharing`, `UpdatePlayerEconomy`, `UpdateMetalExtraction` | `[block] -> void` |
| `GetEnergyUse` | `[block] -> f64` |
| `Reset` | `[block, int] -> void` |
| `SaveUnitAccounts`, `LoadUnitAccounts` | `[block, block, handle] -> void` |
| `RequestEnergy` | `[block, block, f32] -> int` |
| `RequestEnergyAndMetal`, `SpendEnergyAndMetal` | `[block, f32, f32] -> int` |
| `SpendEnergy`, `SpendMetal` | `[block, f32] -> int` |

- Imports, every one listed and nothing else:

| Import | Kinds | Used for |
| --- | --- | --- |
| `units.SetStateBits` | `[block, int, int] -> void` | a unit's cloak state, in `UpdatePlayerEconomy` |
| `units.StartScriptWithArgs` | `[block, linear, linear, int, int, int, int, int, int] -> int` | telling an extractor's script its rate |
| `los_radar.GetMapCell` | `[int, int] -> block` | the cells under an extractor |
| `files.BankOpenNamedBox` | `[handle, linear] -> int` | saved games |
| `files.BankSeekBox` | `[handle, int] -> void` | saved games |
| `files.BankReadBox` | `[handle, linear, int] -> int` | loading an account |
| `files.BankWriteBox` | `[handle, linear, int] -> int` | saving an account |
| `players.TransferEnergy` | `[int, int, f32, int] -> void` | sharing metal (the import named `TransferEnergy` moves metal) |
| `players.TransferMetal` | `[int, int, f32, int] -> void` | sharing energy (the import named `TransferMetal` moves energy) |
| `network.SendShareMapInfo` | `[int, int] -> void` | sharing the map |

`interfaces/players.h` says the same: `TransferMetal` moves energy and `TransferEnergy` moves metal. This
module calls each by its name, as section 4 gives.

- Fuel: the module meters itself by rule version 1 (ABI section 11) and exports `__fuel_set` and
  `__fuel_get`.
- Hooks: `interfaces/economy.h` annotates `UpdatePlayerEconomy` with "Hooks: ResourceIncome,
  StatsAccumulate" and every other slot with "Hooks: none". The kit defines no hook in the module ABI, and
  the import list above (exact, ABI 9.1) has none, so the module raises none; no recording shows one.

### Arguments outside the block

- **The saved-game bank** (`SaveUnitAccounts`, `LoadUnitAccounts`) is a `handle`, passed back to the
  `files.Bank*` imports only.
- **The unit** given as `info` to `SaveUnitAccounts` and `LoadUnitAccounts` is the `Unit` whose
  `resourceSlot` is `self`. The module reads its `id` only.
- **Text and buffers** given to imports (a box name, the script name `"SetSpeed"`) are NUL-terminated
  strings in the module's own memory. The buffers given to `files.BankReadBox` and `files.BankWriteBox`
  are the account itself, in the block. A `files.BankReadBox` line of a recording prints `?` for the
  destination, and the bytes read follow in an `o dst` line. A `files.BankWriteBox` line of a recording
  carries the bank handle, the bytes written (`x:` and their hexadecimal), then the length and the
  result.

### Events

None. No slot of this module records a sim event (kinds are listed in `docs/sim-events.md`).

## 3. State

### Owned

**`UnitResources`** (52 bytes): two groups of six 32-bit floats, then the owner.

| Field | Meaning |
| --- | --- |
| `energyMake`, `metalMake` | made since the last settlement |
| `energyUse`, `metalUse` | asked for or spent since the last settlement |
| `energyUsePaid`, `metalUsePaid` | asked for and granted since the last settlement |
| `energyStall`, `metalStall` | owed from earlier settlements; while above zero, requests are refused |
| `energyMakePrev`, `metalMakePrev` | `energyMake` and `metalMake` at the last settlement |
| `energyUsePrev`, `metalUsePrev` | `energyUse` and `metalUse` at the last settlement |
| `player` | `Ref<Player>`: the account's *owner* |

The energy group is bytes 0 to 23 (`energyMake`, `energyUse`, `energyUsePaid`, `energyStall`,
`energyMakePrev`, `energyUsePrev`), the metal group bytes 24 to 47 in the same order.

**`Player`:** `energy`, `metal` (the stocks), `energyIncome`, `metalIncome`, `energyUsage`,
`metalUsage`, `energyCapacity`, `metalCapacity` (32-bit floats), and `totalEnergyProduced`,
`totalMetalProduced`, `totalEnergyConsumed`, `totalMetalConsumed`, `energyWasted`, `metalWasted`
(64-bit floats).

**`Unit`:** `extraction`, a 32-bit float stored in the 32-bit field the layout types as `int`: the metal
an extractor makes per settlement before any handicap.

### Read

- `Game`: `ticks`, `difficulty` (int), `windStrength`, `tidal` (float), `netGameFlags` (bit 0: a network
  game), `players` (`Player[10]`).
- `Player`: `active` (int), `type` (byte), `index` (byte), `flags` (bit 0: add the storage bonuses),
  `energyStorageBonus`, `metalStorageBonus`, `shareMetal`, `shareEnergy` (floats, the amounts kept back
  when sharing), `unitsBegin`, `unitsEnd` (`Ref<Unit>`, the first and last unit of the player's slice of
  the unit table), `econ` (`Ref<UnitResources>`, the player's own account), `info`
  (`Ref<PlayerInfo>`), `allied` (bytes), `unitCount` (unsigned short), `unitsCreated` (int).
- `PlayerInfo`: `kind` (byte), and the bits `shareMetal` (bit 1), `shareEnergy` (bit 2) and `shareMapping`
  (bit 5) of the 16-bit word `flags_97_wide`, which the schema's read set of `UpdateResourceSharing`
  names. That read set names `Game.flags_2a44`, the 16-bit word at the offset of the byte `netGameFlags`,
  of which the slot uses bit 0.
- `UpdateMetalExtraction`: the schema lists `Game.width` and `Game.height` among its reads. The behaviour
  doesn't depend on them: the coordinates of every cell of the footprint go to `los_radar.GetMapCell` as
  they are, off the map included (`edges-economy-targeting` has cells at (−1,−1)).
- `Unit`: `flags`, `activateFlags`, `def`, `resourceSlot` (its `player` field is the account's owner),
  `buildLeft` (float), `workTime` (int), `cell`, `footprint` (`Point16`), `script`, `id`.
- `UnitDef`: `energyuse`, `energymake`, `metalmake`, `extractsMetal`, `windgenerator`,
  `tidalgenerator`, `cloakcost`, `cloakcostmoving`, `energystorage`, `metalstorage` (floats), and
  `makesMetal` (unsigned byte).
- `Cell`: `metal` (unsigned byte).

### Bits of `Unit.flags`

| Bits | Mask | Word used here |
| --- | --- | --- |
| 2 and 3 | 0x0000000c | *moving*: either bit of `unknown_110_2` set |
| 11 | 0x00000800 | `cloakOn` (the layout's bit-field) |
| 12 | 0x00001000 | *cloak held* (`unknown_110_12`): no cloaking while set |
| 28 | 0x10000000 | `Unit.alive`: only alive units take part in the economy |
| 29 | 0x20000000 | `Unit.immobile`: an immobile unit runs on its own production rules (below) |

A unit is *on* when bit 0 of `activateFlags` is set, and *complete* when `buildLeft` equals 0.0.

### Layout

Consecutive units of the unit table are 280 bytes apart (the size of `Unit` in `layout.json`), the
elements of `Game.players` 331 bytes (the size of `Player`), and a `UnitResources` is 52 bytes. Every
field is read and written at its offset in `layout.json`, whether or not the offset is a multiple of the
field's size: `Game.players` is at 7011, the 64-bit floats of `Player` start at 172, and its `flags` is at
329, so block accesses are unaligned.

### Between ticks

Nothing outside the fields above.

## 4. Behaviour

Within one call, only the order of import calls is observable. The order of block writes carries no
meaning, except through the sums: where two additions go to the same field, each rounds, so the result
depends on the order, and the order given here is the one to use.

Section 5 defines the arithmetic each formula below uses: *add* (`F` adds *v*) and *income*
(`I(a, F, v)`).

### `UpdatePlayerEconomy(p)`

The *slice* is every unit of the unit table from `p.unitsBegin` to `p.unitsEnd`, both included, in table
order (280 bytes apart); the alive units of the slice take part. For a unit *u*, *a* is `u.resourceSlot`
and *d* is `u.def`.

**Per alive unit, in slice order.** The order that is required is this: the imports (the
`units.SetStateBits` of the cloak step) come out unit by unit in slice order; a cloak payment and an
income take effect at the point the steps below give, since a later unit's cloak decision and every sum
rounds on what came before. The steps are otherwise a list of outcomes, and any order of independent
writes that gives the same final values is equal. A cloak payment lowers its owner's `energy` at once,
so a later unit of the same owner decides its own cloak against the lowered `energy`, and an income
added to an account is in that account's `Make` for the unit's own sums.

1. *Charges and production*, by the first row that matches. A unit with *e* = `d.energyuse` is *funded*
   when *e* ≥ 0 and `a.energyStall` is not above 0 (a negative `energyuse` never funds it):

| The unit | Effect |
| --- | --- |
| immobile and not on | none |
| immobile and on, or not immobile and on or moving | If *e* ≥ 0: `a.energyUse` adds *e*, and if the unit is funded `a.energyUsePaid` adds *e* too. If *e* < 0: `I(a, a.energyMake, −e)`. An immobile unit then takes the first row of the next table that matches. |
| any other | none |

| The immobile unit's type | Effect |
| --- | --- |
| `d.extractsMetal` > 0 | if funded: `I(a, a.metalMake, u.extraction)` |
| `d.makesMetal` ≠ 0 | if funded: `I(a, a.metalMake, d.makesMetal)` |
| `d.windgenerator` > 0 | `I(a, a.energyMake, Game.windStrength × d.windgenerator)` |
| `d.tidalgenerator` > 0 | `I(a, a.energyMake, Game.tidal × d.tidalgenerator)` |
| any other | none |

A row that matches excludes the rows after it, whether or not its effect happens: an extractor that isn't
funded makes no metal, and no later row applies to it (Q5).

2. *Completed units*: if the unit is complete, `I(a, a.energyMake, d.energymake)`, then
   `I(a, a.metalMake, d.metalmake)`; `p.metalCapacity` adds `d.metalstorage` and `p.energyCapacity` adds
   `d.energystorage`.
3. *Cloak*: unless `p.active` ≠ 0 and `p.type` = 3, the unit's cloak state *s* is decided, and the import
   `units.SetStateBits(u, 4, s)` is made:
   - *s* = 1 when `cloakOn` is set, cloak held is clear, `u.workTime` ≤ `Game.ticks` (unsigned 32-bit
     comparison) and the owner of *a* can pay the cost *c*: *c* is `d.cloakcostmoving` when the unit is
     moving, else `d.cloakcost`, truncated to an integer (section 5) and then converted back to a float.
     The owner can pay when its `energy` ≥ *c*. Paying: the owner's `energy` becomes `energy` ⊖ *c*, and
     `a.energyUse` adds *c*.
   - *s* = 0 otherwise, with nothing paid.
4. *Sums*: the unit's account goes into the four energy sums and the four metal sums, *M*, *U*, *P*, *S*,
   of its `Make`, `Use`, `UsePaid` and `Stall` fields. Each sum starts at +0.0 and adds each alive unit
   in slice order.

At the start of the call `p.energyCapacity` and `p.metalCapacity` are +0.0, before the first unit adds
to them. A call whose slice has no alive unit makes no imports, and only the unit part is skipped:
the player part below runs as it does for any slice, with every sum before the player's own account
at +0.0 and the capacities at +0.0 (so `p.energy` and `p.metal` are then capped at 0, or at the
storage bonuses when bit 0 of `p.flags` is set). A slice with no unit at all, one in which
`p.unitsBegin` is after `p.unitsEnd`, is the same case. The callers pass the first and the last unit of
one player's block of the unit table, a block of the same length for every player, so a slice has one or
more units and `p.unitsEnd` is a whole number of units (280 bytes each) after `p.unitsBegin`. The
recordings have 250 units in most calls, and the one-unit slice of
`tests/replay/economy/tidal-rounding.txt` (`Unit#4` for both ends). A distance between the two that isn't
a multiple of 280, and a null `p.unitsBegin` or `p.unitsEnd`, are outside the contract.

**The player.** The player's own account *e* = `p.econ` adds last to the sums, with one difference:
for each resource, *M*\* is the sum *M* + *e*.`Make` rounded to 64 bits, and *M* is *M*\* rounded
to 32 bits (section 5). *U*, *P* and *S* add *e*'s fields as the units' did. If bit 0 of `p.flags` is set,
`p.energyCapacity` adds `p.energyStorageBonus` and `p.metalCapacity` adds `p.metalStorageBonus`. Then,
for each resource (energy, then metal; `X` stands for `energy` or `metal`):

- `p.XIncome` becomes *M*, and `p.XUsage` becomes *U*.
- `p.totalEnergyProduced` adds *M*\* for energy, and `p.totalMetalProduced` adds *M* for metal (Q1).
  `p.totalXConsumed` adds *U*.
- The amount *A* is *M*\* + `p.X`, rounded to 32 bits.
- The stall is paid first, then the paid use, each as far as *A* goes:

| Case | Stall ratio *rs* | Remainder *L* |
| --- | --- | --- |
| *S* ≤ *A* | 1 | *A* − *S* |
| otherwise | *A* / *S* | 0 |

| Case | Paid-use ratio *rp* | Stock left *B* |
| --- | --- | --- |
| *P* ≤ *L* | 1 | *L* − *P* |
| otherwise | *L* / *P* | 0 |

- If *B* > `p.XCapacity`, `p.X` becomes `p.XCapacity` and `p.XWasted` adds *B* − `p.XCapacity`.
  Otherwise `p.X` becomes *B*.

**Settlement.** After the player part, the account of every alive unit of the slice, in slice order, and
then *e*, ends the call in this state, for each resource (`X` stands for `energy` or `metal`), with the
ratios *rs* and *rp* of that resource:

- `XMakePrev` is `XMake`, and `XUsePrev` is `XUse` (their values in this call, after the charges above);
- `XStall` is (`XStall` − *rs* × `XStall`) + (`XUsePaid` − *rp* × `XUsePaid`), with the old values;
- `XMake`, `XUse` and `XUsePaid` are +0.0.

### `RequestEnergy(self, r, amount)`

`self` is not read. Every effect is on the account `r`: `r.energyUse` adds `amount`; then if
`r.energyStall` > 0 the result is 0, else `r.energyUsePaid` adds `amount` and the result is 1.

### `RequestEnergyAndMetal(self, energy, metal)`

`self.energyUse` adds `energy` and `self.metalUse` adds `metal`. Then:

| Case | Writes | Result |
| --- | --- | --- |
| `self.energyStall` ≤ 0 and `self.metalStall` ≤ 0 | `self.energyUsePaid` adds `energy`, `self.metalUsePaid` adds `metal` | 1 |
| otherwise | none more | 0 |

### `SpendEnergy(self, amount)`, `SpendMetal(self, amount)`

With *o* the owner of `self`:

| Case | Writes | Result |
| --- | --- | --- |
| `SpendEnergy`, *o*.`energy` ≥ `amount` | *o*.`energy` becomes *o*.`energy` ⊖ `amount`; `self.energyUse` adds `amount` | 1 |
| `SpendMetal`, *o*.`metal` ≥ `amount` | *o*.`metal` becomes *o*.`metal` ⊖ `amount`; `self.metalUse` adds `amount` | 1 |
| otherwise | none | 0 |

### `SpendEnergyAndMetal(self, energy, metal)`

With *o* the owner of `self`:

| Case | Writes | Result |
| --- | --- | --- |
| *o*.`energy` ≥ `energy` and *o*.`metal` ≥ `metal` | *o*.`energy` becomes *o*.`energy` ⊖ `energy`, *o*.`metal` becomes *o*.`metal` ⊖ `metal`; `self.energyUse` adds `energy`, `self.metalUse` adds `metal` | 1 |
| otherwise | none | 0 |

### `GetEnergyUse(def)`

The result, by the first row that matches; no writes and no imports:

| Case | Result |
| --- | --- |
| `def.energyuse` ≠ 0 | `def.energyuse`, a 32-bit float, widened exactly to 64 bits |
| `def.windgenerator` > 0 | −(the exact product of `Game.windStrength` and `def.windgenerator`), as a 64-bit float |
| `def.tidalgenerator` > 0 | −(the exact product of `Game.tidal` and `def.tidalgenerator`), as a 64-bit float |
| otherwise | +0.0 |

The result is a 64-bit float. The product of two 32-bit floats is exactly a 64-bit float, so the
generator rows round nothing, and the consumer row is the widened value, whatever its sign. The "exact
product of its two 32-bit factors" that the kit's `interfaces/economy.h` and `docs/ai-ratings.md` give
is the generator rows only. The wind and tidal rows negate their product, so a product of +0.0 gives
−0.0, the pattern 0x8000000000000000 (`UnitDef#124` at tick 0 of `edges-economy-targeting`:
`Game.tidal` 0.0, `tidalgenerator` 1.0). Recorded (kit `reference-traces-v23`, `economy 3.0`):

- a consumer: `UnitDef#3` of `duel-shore-to-shore`, `energyuse` 0x3dcccccd, gives 0x3fb99999a0000000
  (the widened value); `UnitDef#112` of `edges-economy-targeting`, `energyuse` −20.0, gives
  0xc034000000000000;
- a tidal generator: `UnitDef#259` of `duel-coast-to-coast` at tick 0, `Game.tidal` 15.0 (0x41700000),
  `tidalgenerator` 1.0, gives 0xc02e000000000000 (−15.0);
- a wind generator: example 7 of section 7.

### `UpdateMetalExtraction(unit)`

With *d* = `unit.def`: if `d.extractsMetal` > 0 is false, nothing happens, with no imports. Otherwise:

- Imports `los_radar.GetMapCell(x, y)` once for each cell of the unit's footprint: *y* from `cell.y` to
  `cell.y` + `footprint.y` − 1, and for each *y*, *x* from `cell.x` to `cell.x` + `footprint.x` − 1. A
  footprint with a size of 0 or less on either axis has no cells.
- *W* is the 16-bit sum, over every cell the import returned (not null), of `Cell.metal` + 1. A null
  result adds nothing.
- `unit.extraction` becomes `d.extractsMetal` × *W*, rounded to 32 bits.
- If `unit.script` is not null, imports `units.StartScriptWithArgs` after the last `GetMapCell`, with
  `script` the script's address (the block base plus `unit.script`), `name` the string `"SetSpeed"`,
  `callback` the null address (0), `update` 0, `count` 1, `a` *W* as a signed integer, and `b`, `c`, `d`
  0. Its result is ignored. The recordings print `?` for the callback argument
  (the `pending` mode of `docs/replay-format.md`).

### `Reset(self, playerIndex)`

The twelve floats of `self` become +0.0, and `self.player` becomes a reference to element *k* of
`Game.players`, where *k* is the low 8 bits of `playerIndex`. Like every `Ref`, it is stored as a block
offset (ABI 7.1), a 32-bit offset from the block's base (the data word `g_blockBase`; 0 is null): the
address of that element, minus `g_blockBase`. That is the offset of the game record from the block's
base (the value of `g_game` minus the value of `g_blockBase`), plus the offset of `Game.players` in
`Game` (7011), plus 331 × *k*. The value of a data word is the 4 bytes at the address its export holds
(ABI 8.2: the export is an i32 global holding the address of a 4-byte location, and the host writes the
word there before every slot call). No imports. Callers pass a player's slot, 0 to 9, only; for any
other *k* the result is undefined and the module need not check.

### `SaveUnitAccounts(self, info, bank)`

The box name is `u` followed by `info.id` as four lower-case hexadecimal digits and then `acc`
(`"u0001acc"` for id 1). The imports, in this order, with every result ignored:

1. `files.BankOpenNamedBox(bank, name)`;
2. `files.BankSeekBox(bank, 0)`;
3. `files.BankWriteBox(bank, energy group, 24)`, the energy group being the 24 bytes at `self`;
4. `files.BankWriteBox(bank, metal group, 24)`, the metal group being the 24 bytes at `self` + 24.

`self.player` isn't saved. `SaveUnitAccounts` reads the whole account, `energyMake` and `metalMake`
included (the first float of each group), and `info.id`. It writes nothing to the block.

### `LoadUnitAccounts(self, info, bank)`

The box name is as for `SaveUnitAccounts`.

| `files.BankOpenNamedBox(bank, name)` | Then |
| --- | --- |
| 0 | nothing more |
| other | `files.BankSeekBox(bank, 0)`, then `files.BankReadBox(bank, energy group, 24)`, then `files.BankReadBox(bank, metal group, 24)` |

The module ignores each read result: it makes the second read whatever the first returned, and makes no
read other than these two. It writes nothing to the account itself: the bytes of each group after the call
are the ones the read delivered, and a read that delivers fewer than 24 bytes (the number it returns, up
to the length asked, 0 at the end of the box) leaves the rest of its group as it was. The position of the
box moves on by the bytes read, so the second read continues where the first stopped. A box of 30 bytes
fills the energy group and the first 6 bytes of the metal group; a box shorter than 24 bytes fills the
first part of the energy group and leaves the metal group unchanged. `self.player` isn't read from the
bank.

### `UpdateResourceSharing(player)`

Nothing happens unless bit 0 of `Game.netGameFlags` is set. *t* is `Game.ticks` as an unsigned 32-bit
integer, and tick 0 is a multiple of both 60 and 450. A null `player.info`, or a null `info` of a player
that meets the other partner conditions, is outside the contract.

A *partner* is an element *q* = `Game.players[i]`, for *i* from 0 to 9, with `q.active` ≠ 0, `q.type` = 3,
`q.index` ≠ 10, `q.unitCount` ≠ 0 or `q.unitsCreated` = 0 (a player that
has never made a unit counts, and so does one that has units), `q.info.kind` = 1, and `player.allied[i]` ≠ 0.
`player` itself is a partner when it meets these.

When *t* is a multiple of 60 (0 included), metal is shared, and then energy:

- **Metal**, when the `shareMetal` bit of `player.info` is set and `player.metal` > `player.shareMetal`:
  the *recipient* is the partner with the highest *i* whose `metal` < `player.metal`. If there is one,
  the import `players.TransferEnergy(player.index, recipient.index, amount, 1)` is made, with *amount*
  the smaller of (`player.metal` − `player.shareMetal`) × *k3* and `recipient.metalCapacity` −
  `recipient.metal`, where *k3* is the 32-bit float 0x3eaaaaab (Q3). The import is made whatever
  *amount* is: a recipient with no room left gives an *amount* of 0 or below, and the import is still
  made with it.
- **Energy**, after the metal part (its import, when made, comes first): the same with the `shareEnergy`
  bit, `energy`, `shareEnergy` and `energyCapacity`, the factor 0.5 in place of *k3*, and the import
  `players.TransferMetal(player.index, recipient.index, amount, 1)`, with the last argument 1 as for the
  metal import.

When *t* is a multiple of 450 (0 included) and the `shareMapping` bit of `player.info` is set, then
for each partner in order of *i*, the import `network.SendShareMapInfo(player.index, q.index)` is made,
unless `player.index` is 10 (the partner's index is not 10). The player and partner indices, bytes of
0 to 255, cross the `int` kind of the three sharing imports zero-extended: index 3 is the integer 3.
The player itself is sent its own map when it is a partner (Q4), but never receives metal or energy. On
a tick that is a multiple of both, the sharing imports come before the map imports.

`player` itself has no condition of its own: it needn't be active or of type 3. As the sharing player the
slot reads its `info`, `index`, `metal`, `energy`, `shareMetal`, `shareEnergy` and `allied` bytes (and
`player` is read again as an element of `Game.players` in the partner test). A recording of these imports
would print each as the other imports are printed (`docs/replay-format.md`, "Values": decimal integers, a
float as its bits), the sharing imports `i players.TransferEnergy 0 1 f32:0x41200000 1` for a transfer
from player 0 to player 1 of 10.0, and `i network.SendShareMapInfo 0 1`, with no result on either line.
The kit has no recording of them, so the line forms are the kit's rules, not an observation.

No writes.

## 5. Exactness

**Notation.** f32(*x*) rounds a real *x* to the nearest 32-bit float, ties to even, and f64(*x*) to the
nearest 64-bit float. ⊕, ⊖ and ⊗ are the 32-bit operations: *a* ⊕ *b* = f32(*a* + *b*). Every comparison
is of the exact values. A float field or argument is a 32-bit float unless section 3 says 64.

- ***Add***: "`F` adds *v*" means `F` becomes `F` ⊕ *v*. For a 64-bit field (the totals and the
  wasted amounts) it means `F` becomes f64(`F` + *v*).
- ***Income*** `I(a, F, v)`, where *v* is exact (a float, a byte, or an exact product of two floats): let
  *o* be the owner of the account *a*. When *o*.`active` ≠ 0 and *o*.`type` = 2, the *handicap* *h*
  applies: 0.5 when `Game.difficulty` = 0, and *h7* = f64(0.7) = 0x3fe6666666666666 when it is 1. `F`
  becomes f32(f64(`F` + f64(*v* × *h*))). Without a handicap (any other owner, or any other
  difficulty), `F` becomes f32(f64(`F` + *v*)). The handicap is the owner's, not `p`'s, in
  `UpdatePlayerEconomy`.
- **Products in `I`:** `Game.windStrength` × `d.windgenerator` and `Game.tidal` × `d.tidalgenerator` are
  exact products, not rounded to 32 bits before they enter `I`. The pin is
  `tests/replay/economy/tidal-rounding.txt`, the call at tick 6270 of `UpdatePlayerEconomy(Player#0)`: the
  account's `energyMake` is 0x437b4f71, `Game.tidal` is 0x42436dc7 and `tidalgenerator` is 0x41a80d9b, and
  `energyIncome` becomes 0x449fb45c; adding the product rounded to 32 bits first gives 0x449fb45b.
- **The result of `GetEnergyUse`:** the slot's result is a 64-bit float: for the wind and tidal rows the
  negated exact product (no rounding is needed), and for a consumer its 32-bit `energyuse` widened. The
  recorded results are `o result f64:...`.
- **The sums of `UpdatePlayerEconomy`**: after each alive unit's term, a sum is the 32-bit float nearest
  to the exact sum of its value before and the term (*M* ⊕ `a.energyMake`, and so on). *M*\* is the 64-bit
  float nearest to *M* + *e*.`Make`; *M* is the 32-bit float nearest to *M*\*. *A* is
  f32(f64(*M*\* + `p.X`)).
- **The ratios:** *A*, *S* and *P* are 32-bit floats. When *S* ≤ *A*, *L* is the difference *A* − *S*
  rounded to 64 bits only (it is not rounded to 32 bits), and *rs* is 1; otherwise *L* is 0 and *rs* is
  the quotient *A* / *S* of two 32-bit floats rounded once to 32 bits. When *P* ≤ *L*, *rp* is 1 and *B*
  is the difference *L* − *P* rounded to 64 bits, then to 32 bits; otherwise *B* is 0 and *rp* is the
  64-bit quotient *L* / *P* rounded to 32 bits. The wasted amount adds the difference *B* −
  `p.XCapacity` rounded to 64 bits.
- **The new stall of an account:** with *s* = `XStall` and *q* = `XUsePaid` (both 32-bit), the result
  is formed in this order. The product *rs* × *s* is rounded to 64 bits, and *s* minus it is rounded to
  64 bits. The product *rp* × *q* is rounded to 64 bits, and *q* minus it is rounded to 64 bits. The two
  differences are added and rounded to 64 bits, and that sum is rounded to 32 bits; this is the new
  `XStall`.
- **The cloak cost:** the float is truncated towards zero to a 32-bit integer, and the integer converted
  to a float exactly. A NaN or a value outside the 32-bit range gives the integer −2147483648.
- **Extraction:** *W* is a signed 16-bit integer: it starts at 0, each cell adds `Cell.metal` + 1, and it
  wraps modulo 65536 into −32768 to 32767. The cell coordinates are the signed 16-bit `Point16` fields,
  widened to 32 bits before the footprint is added. `unit.extraction` = f32(`d.extractsMetal` × *W*),
  the product exact. *W* is passed to the script import sign-extended to 32 bits.
- **Sharing:** the amount is f32(min(*D* × *k*, *C*)), where *D* = f64(`player.X` − `player.shareX`),
  the product *D* × *k* is rounded to 64 bits, and *C* = f64(`recipient.XCapacity` − `recipient.X`).
- **Ticks:** `Game.ticks` and `Unit.workTime` are compared as unsigned 32-bit integers, and the
  multiples of 60 and 450 are of the unsigned value.
- **Division:** a quotient is the IEEE 754 quotient: *A* / *S* with *S* = 0 and *A* < 0 is −∞.
- **NaN:** when the income of a call is negative and larger than the stock (a unit type with a negative
  `energymake` or a negative `extraction`, which a wrapped *W* gives), *A* is below 0, and when no account
  owes a stall *S* is +0.0 (the sums start at +0.0, and adding stalls never gives −0.0). For that
  resource the stock then becomes 0, and the new `XStall` of every account with `XStall` = 0 is a NaN. The
  kit's ABI (section 3.2) says the engine canonicalises NaNs and a module must not let a result depend on
  a NaN's bit pattern, so this spec fixes no pattern for it. It defines no result of a call that reads a
  NaN in a field other than those named in the next bullet.
- **A NaN stall or stock (not recorded):** it is reached only after a negative income exceeds the stock
  with no stall owed, and no recording reaches it. The outcomes are these.
  - A NaN stall counts as not above 0: `RequestEnergy`, the funded test (for an immobile and for any
    other unit) and `RequestEnergyAndMetal` all grant on it (`RequestEnergyAndMetal` on a NaN
    `energyStall` or `metalStall`, or both, writes as its first row does and returns 1).
  - The settlement of a call in which an account's stall is NaN has a NaN *S*, and takes the first row
    of each table: *S* ≤ *A* gives *rs* = 1 and *L* = *A* − *S* = NaN, and *P* ≤ *L* gives *rp* = 1
    and *B* = *L* − *P* = NaN. *B* > `p.XCapacity` is false, so `p.X` becomes NaN and `p.XWasted` is
    unchanged. Each account's new `XStall` is then +0.0 when its `XStall` and `XUsePaid` are finite,
    and NaN when one of them is NaN. From then on *A* is NaN, so `p.X` stays NaN on every later call.
  - With a NaN `energy`, the cloak payment is made: *s* = 1, `energy` stays NaN and `a.energyUse` adds
    *c*. `SpendEnergy`, `SpendMetal` and `SpendEnergyAndMetal` refuse on a NaN stock or a NaN amount.
  - In `UpdateResourceSharing`, a comparison "below" with a NaN operand is true: the recipient tests
    `q.metal` < `player.metal` and `q.energy` < `player.energy`, and the choice of the smaller of the
    amount and the room left.

The roundings above state results: an implementation may compute them in any way that gives the same
bits.

## 6. Quirks

- **Q1:** `totalEnergyProduced` grows by the 64-bit energy sum *M*\*, while `totalMetalProduced` grows by
  the 32-bit metal sum *M*.
- **Q2:** a stall that is NaN counts as not above 0: `RequestEnergy`, the funded test and
  `RequestEnergyAndMetal` all grant on it (section 5, NaN).
- **Q3:** a player shares a third of its surplus metal and half of its surplus energy, and gives it to
  the partner with the highest index whose stock is below its own.
- **Q4:** shared map information can go to the sharing player itself.
- **Q5:** an immobile unit that is on, with an `energyuse` below 0, earns that energy, and an extractor
  or a metal maker with a negative `energyuse` makes no metal.
- **Q6:** capacities go to `p`, the player whose slice is walked, but the handicap and the cloak payment
  go to each account's owner.
- **Q7:** the cloak cost is truncated to a whole number before it is paid.
- **Q8:** a spend (`SpendEnergy`, `SpendMetal`, `SpendEnergyAndMetal`, and the cloak payment) takes the
  amount from the stock at once and also adds it to the account's `Use`.
- **Q9:** *W* wraps at 16 bits.

## 7. Examples

Values are given as decimals, with the 32-bit pattern where the rounding matters.

1. **`tests/replay/economy/ten-ring-atoll.txt`, the first call at tick 6270 of
   `UpdatePlayerEconomy(Player#0)`.** `Game.difficulty` is 2, so no handicap. `Game.tidal` is 30.0. The
   alive units are `Unit#1` (not immobile, on, `energyuse` 1, complete, `energymake` 25,
   `metalmake` 1), `Unit#2` (immobile, not on), `Unit#3` and `Unit#6` (immobile, on,
   `energyuse` 3, `extractsMetal` 0.001, `extraction` 1.692 and 1.053), and `Unit#4` and `Unit#5`
   (immobile, on, `tidalgenerator` 1). Every account starts at 0.
   - Energy: *M* = 25 + 30 + 30 = 85, *U* = *P* = 1 + 3 + 3 = 7, *S* = 0. *A* = 85 + 1000 = 1085.
     *rs* = 1, *L* = 1085, *rp* = 1, *B* = 1078. `energyCapacity` is 1000 (the storage bonus), so
     `energy` stays 1000 and `energyWasted` grows by 78, from 5093.99951171875 to 5171.99951171875.
   - Metal: *M* = 1 ⊕ 1.692 ⊕ 1.053 = 3.745 (0x406fae14), *U* = 0. *A* = f32(*M*\* + 980.13208)
     = 983.8770751953125 (0x4475f822), below the capacity of 1000, so `metal` becomes it.
2. **`tests/replay/economy/duel-shore-to-shore.txt`, the first call at tick 1650 of
   `UpdatePlayerEconomy(Player#0)`.** `Player#0.type` is 2 and `Game.difficulty` 0: the handicap is 0.5.
   `Game.windStrength` is 0.2288 (0x3e6a4a8c). `Unit#1` (complete, `energymake` 25, `metalmake` 1),
   two extractors (`extraction` 2.016 and 0.783, `energyuse` 3) and a wind generator (`windgenerator`
   30). Energy *M* = 12.5 ⊕ f32(0.2288 × 30 × 0.5) = 15.932 (0x417ee979), *U* = 6. Metal *M* =
   0.5 ⊕ 1.008 ⊕ 0.3915 = 1.8995 (0x3ff322d1). `energy` goes from 3827.431640625 to 3837.363525390625
   (0x456fd5d1), `metal` from 4910.76123046875 to 4912.66064453125 (0x45998549).
3. **`tests/replay/economy/duel-coast-to-coast.txt`, the first call at tick 4590 of
   `UpdatePlayerEconomy(Player#0)`: a stall.** `Unit#1`'s account holds `energyUse` = `energyUsePaid` =
   216.72653 when the call starts, and the unit's own request adds 1, giving 217.72653. Energy *M* = 85
   (25, plus 20 from each of three immobile units whose `energyuse` is −20), *U* = *P* = 223.72653
   (0x435fb9fe), *S* = 0, and `energy` is 28.933624 (0x41e77810). *A* = 113.933624, *rs* = 1,
   *L* = 113.933624; *P* > *L*, so *rp* = f32(113.933624 / 223.72653) and *B* = 0. `energy` becomes 0,
   and `Unit#1.resourceSlot.energyStall` becomes 217.72653 − *rp* × 217.72653 = 0x42d5b265 (106.85).
4. **The same file, the call at tick 4591 of `RequestEnergyAndMetal(Unit#1.resourceSlot, energy,
   metal)`**, with `energy` 0x40e72cd0 and `metal` 0x3ffb48a7: `energyStall` is 0x42d5b265, above 0,
   so `energyUse` and `metalUse` grow from 0 to those amounts, `UsePaid` doesn't change, and the result
   is 0.
5. **The same file, the call at tick 197 of `UpdateMetalExtraction(Unit#2)`.** `cell` is (168,8),
   `footprint` (3,3), `extractsMetal` 0.001 (0x3a83126f). The nine `GetMapCell` imports are (168,8),
   (169,8), (170,8), (168,9), ... (170,10), each cell with `metal` 187, so *W* = 9 × 188 = 1692.
   `extraction` becomes f32(0.001 × 1692) = 0x3fd89375 (1.692), and the import
   `units.StartScriptWithArgs(CobScript#3, "SetSpeed", ?, 0, 1, 1692, 0, 0, 0)` follows, the `?` standing
   for the callback argument, which the line doesn't show.
6. **`tests/replay/economy/load-midgame.txt`, the call at tick 900 of `LoadUnitAccounts` for `Unit#2`**
   (`id` 2): the box is `"u0002acc"`. The energy group read holds 3.0 at bytes 20 to 23 and the metal
   group 0x3fb4a234 at bytes 16 to 19, so `energyUsePrev` becomes 3.0 and `metalMakePrev` 1.4112.
7. **`tests/replay/economy/duel-shore-to-shore.txt`, a call at tick 2310 of `GetEnergyUse(UnitDef#135)`**,
   a wind generator: `energyuse` is 0, `windgenerator` 30 and `Game.windStrength` 0.1246 (0x3dff2e49), so
   the result is −(0.1246 × 30), the exact product, −3.7380000203847885 (0xc00de76c8e000000 as a 64-bit
   float).
8. **Not from a recording: `UpdateResourceSharing` at tick 60.** `Game.netGameFlags` has bit 0 set.
   `player` is `Game.players[0]` (index 0, type 1) with `metal` 1000.0, `shareMetal` 100.0, `energy` 500.0
   and `shareEnergy` 200.0, and its `info` has `shareMetal` and `shareEnergy` set and `shareMapping`
   clear.
   `Game.players[1]` is active, of type 3, with index 1, `unitCount` 5, `info.kind` 1, `metal` 200.0,
   `metalCapacity` 1000.0 and `energy` 600.0, and `player.allied[1]` ≠ 0. No other slot is a partner. The
   tick is a multiple of 60 and not of 450. Metal: 1000 > 100, and the recipient is player 1 (200 < 1000).
   *amount* = f32(min(900 × 0.3333333432674408, 1000 − 200)) = f32(300.0000089406967) = 300.0
   (0x43960000), so the one import is `players.TransferEnergy(0, 1, 300.0, 1)`, recorded as
   `i players.TransferEnergy 0 1 f32:0x43960000 1`. Energy: 500 > 200, but no partner has `energy` below
   500 (player 1 has 600), so no import is made. No map is sent. Nothing is written.

9. **Not from a recording: the map sends at tick 450.** `Game.netGameFlags` has bit 0 set, and `player` is
   `Game.players[0]` (index 0) whose `info` has `shareMapping` set. Players 0, 1 and 2 are active, of type
   3, with indices 0, 1 and 2, `info.kind` 1, a nonzero `unitCount`, and `player.allied[0]`, `[1]` and
   `[2]` nonzero; no other slot is a partner. The tick is a multiple of 450 and not of 60, so there is
   no transfer. The imports, in order, are `i network.SendShareMapInfo 0 0`,
   `i network.SendShareMapInfo 0 1` and `i network.SendShareMapInfo 0 2`: the player is its own partner
   (Q4). At tick 900, a multiple of both, any transfer imports come before these.
10. **Not from a recording: a transfer of zero or less.** As example 8, but `Game.players[1]` has
    `metalCapacity` 200.0 (it is full): *amount* = f32(min(300.0000089406967, 200 − 200)) = 0.0, and the
    import is made, `i players.TransferEnergy 0 1 f32:0x00000000 1`. With `metalCapacity` 150.0 the amount
    is 150 − 200 = −50.0, and the import is `i players.TransferEnergy 0 1 f32:0xc2480000 1`.

**Not recorded in the kit's recordings** (`docs/replay-format.md`, "Not recorded"):

- `UpdateResourceSharing`.
- `UpdatePlayerEconomy` for a player of type 3.
- `UpdatePlayerEconomy` for a unit whose resource account belongs to another player than the one settled.
- `LoadUnitAccounts` with a missing box.
- The false sides of `SpendMetal`, `SpendEnergyAndMetal` and `SpendEnergy` (not enough in stock), outside
  the cloak payment.
- The false side of `SpendEnergy` in the cloak payment of `UpdatePlayerEconomy` (a cloaked unit that
  cannot pay, which `units.SetStateBits <unit> 4 0` records and which decloaks it): the one cloaked unit of
  the recordings, `Unit#3` of `edges-economy-targeting`, pays at all eight calls (ticks 30 to 240,
  `units.SetStateBits Unit#3 4 1`).
- `UpdateMetalExtraction` for an extractor with no script.

Recorded: a null result of `los_radar.GetMapCell` (five imports of `edges-economy-targeting`, in the
call at tick 5 of `UpdateMetalExtraction(Unit#2)`, for cells with x or y of −1), and the
`files.BankWriteBox` lines of `save-midgame`.

No recording reaches the following, and the rule of the spec is the authority for each:

- `UpdatePlayerEconomy`, cloak step: the moving cost (the one cloaked unit is stationary), the cloak held
  bit set, `workTime` above `Game.ticks`, a fractional cost truncated (Q7, the recorded cost is 7.0), a
  negative or NaN cost, and the player of type 3 skipping the `units.SetStateBits` imports.
- `UpdatePlayerEconomy`, income: no immobile unit with a negative `energyuse` has `extractsMetal` or
  `makesMetal` (Q5), so which metal row wins is not recorded; and a difficulty of 3 or more with an owner of
  type 2 (no handicap).
- The energy half of Q1 (`totalEnergyProduced` adding *M*\* instead of *M*), and the 16-bit wrap of *W*
  (Q9).
- `RequestEnergy` returning 0, and `RequestEnergy` with `self` not `r`.
- `LoadUnitAccounts` with a read that returns fewer than 24 bytes.
- `UpdateResourceSharing` in full: the partner conditions (index 10, `unitCount`, `unitsCreated`), a
  transfer with an *amount* of 0 or below, the sends of the shared map every 450 ticks, and the player as
  its own partner.
- `Reset` with a player index of 10 or more (undefined).
- `UpdatePlayerEconomy` with bit 0 of `p.flags` clear (no storage bonus is added; it is set in every
  recording).
- `UpdatePlayerEconomy` for a unit that is not immobile, is on or moving and has an `energyuse` below 0
  (it earns that energy).
- An immobile unit with `makesMetal` that is not funded (it makes no metal).
- `UpdatePlayerEconomy` with an owner whose `active` is 0 in the handicap test (no handicap).
- `UpdateMetalExtraction` with a footprint of size 0 or less on an axis (no cells, no imports of
  `los_radar.GetMapCell`).
- `UpdatePlayerEconomy` with *S* = 0 and *A* < 0 (the NaN of section 5), and with a negative income.

Pinned by a recording: the exact tidal product (`tidal-rounding.txt`). Not pinned: a 32-bit reading of
*M*\*, *A*, *L* and the exact wind product gives the same results on every recording, and so does
rounding *L* / *P* once to 32 bits in place of the 64-bit quotient rounded to 32 bits, and so does
adding *M* in place of *M*\* to `totalEnergyProduced` (Q1). For all of them this spec is the authority,
and no recording judges them: the gate's full scenarios are their only judge.

## 8. Kit version

Kit `interfaces-v7` and `reference-traces-v24`, the versions the gate judges against. Interface
`economy` 3.0.
