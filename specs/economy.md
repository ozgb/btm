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

### Slots (`interfaces/economy.h`, version 2.0)

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

Every slot that the header marks "thiscall in 3.1c" takes the account (`self`) as its first argument,
as the header lists.

### The manifest entry

The kit's manifest (`wasm/modules.json`) carries the slot signatures of this module (`signatures.economy`)
and its interface major, 2 (`majors.economy`). It has no build entry for it and no budget: a clean-room
module sets its own (see Fuel). This section gives the entry the module must have. The kind `f32` is a
32-bit IEEE 754 float that crosses the boundary bit for bit (ABI section 7).

- `name` and `interface`: `economy`.
- `exports`, in this order: `UpdateResourceSharing`, `GetEnergyUse`, `UpdatePlayerEconomy`,
  `UpdateMetalExtraction`, `Reset`, `SaveUnitAccounts`, `LoadUnitAccounts`, `RequestEnergy`,
  `RequestEnergyAndMetal`, `SpendEnergy`, `SpendMetal`, `SpendEnergyAndMetal`.
- `data`: `g_game`, `g_blockBase` (ABI 8.2).
- Slot kinds:

| Slot | Kinds |
| --- | --- |
| `UpdateResourceSharing`, `UpdatePlayerEconomy`, `UpdateMetalExtraction` | `[block] -> void` |
| `GetEnergyUse` | `[block] -> f32` |
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
  `__fuel_get`. The manifest gives it no budget: the module's budget is set as ABI section 11.3 says.
- Hooks: the header names the hooks `ResourceIncome` and `StatsAccumulate` for `UpdatePlayerEconomy`.
  This module raises neither, and that is intended: no recording of this module holds a hook call. The kit
  doesn't yet define how a module raises a hook (a kit gap, raised on #823).

### Arguments outside the block

- **The saved-game bank** (`SaveUnitAccounts`, `LoadUnitAccounts`) is a `handle`, passed back to the
  `files.Bank*` imports only.
- **The unit** given as `info` to `SaveUnitAccounts` and `LoadUnitAccounts` is the `Unit` whose
  `resourceSlot` is `self`. The module reads its `id` only.
- **Text and buffers** given to imports (a box name, the script name `"SetSpeed"`) are NUL-terminated
  strings in the module's own memory. The buffers given to `files.BankReadBox` and `files.BankWriteBox`
  are the account itself, in the block. A `files.BankWriteBox` line of a recording carries the bank
  handle, the bytes written (`x:` and their hexadecimal), then the length and the result.

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
- `PlayerInfo`: `kind` (byte), and the bits `shareMetal`, `shareEnergy`, `shareMapping`.
- `Unit`: `flags`, `activateFlags`, `def`, `resourceSlot`, `buildLeft` (float), `workTime` (int),
  `cell`, `footprint` (`Point16`), `script`, `id`.
- `UnitDef`: `energyuse`, `energymake`, `metalmake`, `extractsMetal`, `windgenerator`,
  `tidalgenerator`, `cloakcost`, `cloakcostmoving`, `energystorage`, `metalstorage` (floats), and
  `makesMetal` (unsigned byte).
- `Cell`: `metal` (unsigned byte).

### Bits of `Unit.flags`

| Bits | Mask | Word used here |
| --- | --- | --- |
| 2 and 3 | 0x0000000c | *moving*: either bit set |
| 11 | 0x00000800 | `cloakOn` (the layout's bit-field) |
| 12 | 0x00001000 | *cloak held*: no cloaking while set |
| 28 | 0x10000000 | *counted*: the unit takes part in the economy |
| 29 | 0x20000000 | *producer*: the unit runs on its own production rules (below) |

A unit is *on* when bit 0 of `activateFlags` is set, and *complete* when `buildLeft` equals 0.0.

### Alignment

Fields of `Player`, `Unit` and `UnitResources` sit at offsets that aren't a multiple of their size
(`layout.json` gives each offset: `Player`'s 64-bit floats from offset 172, and its `flags` at 329). `Player`
is 331 bytes long, so the elements of `Game.players` after the first shift every field off its natural
alignment. Every load and store of a field must accept any alignment. The kit doesn't yet say this in the
ABI (a gap, raised on #823).

### Between ticks

Nothing outside the fields above.

## 4. Behaviour

Within one call, only the order of import calls is observable. The block writes are compared at the
end of the call, so their order matters only where two of them add to the same field: those add in
the order given here, because each addition rounds.

Section 5 defines the arithmetic each formula below uses: *add* (`F` ← *F* ⊕ *v*), *income* (`I(a, F, v)`)
and *request* (`R(a, v)`).

### `UpdatePlayerEconomy(p)`

The *slice* is every unit of the unit table from `p.unitsBegin` to `p.unitsEnd`, both included, in
table order; the counted units of the slice take part. For a unit *u*, *a* is `u.resourceSlot` and *d*
is `u.def`.

**Per counted unit, in slice order.** The imports and the writes of the steps below come out in slice
order, unit by unit. A cloak payment lowers its owner's `energy` at once, so a later unit of the same
owner decides its own cloak against the lowered `energy`, and an income added to an account is in
that account's `Make` for the unit's own sums.

1. *Charges and production*, by the first row that matches:

| The unit | Effect |
| --- | --- |
| a producer that is not on | none |
| a producer that is on | *e* = `d.energyuse`. If *e* ≥ 0: *granted* = `R(a, e)`. If *e* < 0: `I(a, a.energyMake, −e)`, and *granted* is 0. Then the first row of the next table that matches. |
| not a producer, and on or moving | *e* = `d.energyuse`. If *e* ≥ 0: `R(a, e)`. If *e* < 0: `I(a, a.energyMake, −e)`. |
| any other | none |

| The producer's type | Effect |
| --- | --- |
| `d.extractsMetal` > 0 | if *granted*: `I(a, a.metalMake, u.extraction)` |
| `d.makesMetal` ≠ 0 | if *granted*: `I(a, a.metalMake, d.makesMetal)` |
| `d.windgenerator` > 0 | `I(a, a.energyMake, Game.windStrength × d.windgenerator)` |
| `d.tidalgenerator` > 0 | `I(a, a.energyMake, Game.tidal × d.tidalgenerator)` |
| any other | none |

A row that matches excludes the rows after it, whether or not its effect happens: an extractor that isn't
*granted* makes no metal, and no later row applies to it (Q5).

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
   of its `Make`, `Use`, `UsePaid` and `Stall` fields. Each sum starts at +0.0 and adds each counted unit
   in slice order.

At the start of the call `p.energyCapacity` and `p.metalCapacity` are +0.0, before the first unit adds
to them. A call whose slice has no counted unit makes no imports.

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

**Settlement.** Every counted unit of the slice, in slice order, and then *e*, has its account settled
with the ratios of each resource. The counted units are the ones the walk above used: the
`units.SetStateBits` imports of the cloak step (mask 4) change the unit's activation bits only, never
its counted bit, and the replay stub changes nothing in the block. For each resource of an account:

- `XMakePrev` becomes `XMake`, and `XUsePrev` becomes `XUse` (their values in this call, after the
  charges above);
- `XStall` becomes (`XStall` − *rs* × `XStall`) + (`XUsePaid` − *rp* × `XUsePaid`), with the old values;
- `XMake`, `XUse` and `XUsePaid` become +0.0.

### `RequestEnergy(self, r, amount)`

`self` is not read (Q2). Every effect is on the account `r`: `r.energyUse` adds `amount`; then if
`r.energyStall` > 0 the result is 0, else `r.energyUsePaid` adds `amount` and the result is 1. This is
`R(r, amount)` of section 5.

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
| `def.energyuse` ≠ 0 | `def.energyuse` |
| `def.windgenerator` > 0 | −(`Game.windStrength` ⊗ `def.windgenerator`) |
| `def.tidalgenerator` > 0 | −(`Game.tidal` ⊗ `def.tidalgenerator`) |
| otherwise | +0.0 |

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
  0. Its result is ignored. The recordings print `?` for the callback argument whatever the module passes
  (the `pending` mode of `docs/replay-format.md`), so the replay doesn't judge it; a null is the value
  the reference passes.

### `Reset(self, playerIndex)`

The twelve floats of `self` become +0.0, and `self.player` becomes a reference to element *k* of
`Game.players`, where *k* is the low 8 bits of `playerIndex`. Like every `Ref`, it is stored as a block
offset (ABI 7.1): the offset of `Game.players` in the block plus 331 × *k*. No imports. Callers pass a
player's slot, 0 to 9, only; for any other *k* the result is undefined and the module need not check.

### `SaveUnitAccounts(self, info, bank)`

The box name is `u` followed by `info.id` as four lower-case hexadecimal digits and then `acc`
(`"u0001acc"` for id 1). The imports, in this order, with every result ignored:

1. `files.BankOpenNamedBox(bank, name)`;
2. `files.BankSeekBox(bank, 0)`;
3. `files.BankWriteBox(bank, energy group, 24)`, the energy group being the 24 bytes at `self`;
4. `files.BankWriteBox(bank, metal group, 24)`, the metal group being the 24 bytes at `self` + 24.

`self.player` isn't saved. Nothing is written to the block.

### `LoadUnitAccounts(self, info, bank)`

The box name is as for `SaveUnitAccounts`.

| `files.BankOpenNamedBox(bank, name)` | Then |
| --- | --- |
| 0 | nothing more |
| other | `files.BankSeekBox(bank, 0)`, then `files.BankReadBox(bank, energy group, 24)`, then `files.BankReadBox(bank, metal group, 24)` |

Each read result is ignored by the module, but it says how much the read returned: after the call, the
energy group of the account holds the bytes the first read returned, followed by the bytes it had before
for any part of the 24 the read didn't return, and the metal group is the same for the second read.
`self.player` isn't read from the bank.

### `UpdateResourceSharing(player)`

Nothing happens unless bit 0 of `Game.netGameFlags` is set. *t* is `Game.ticks` as an unsigned 32-bit
integer.

A *partner* is an element *q* = `Game.players[i]`, for *i* from 0 to 9, with `q.active` ≠ 0, `q.type` = 3,
`q.index` ≠ 10 (a free slot has index 10), `q.unitCount` ≠ 0 or `q.unitsCreated` = 0 (a player that
has never made a unit counts, and so does one that has units), `q.info.kind` = 1, and `player.allied[i]` ≠ 0.
`player` itself is a partner when it meets these.

When *t* is a multiple of 60, metal is shared, and then energy:

- **Metal**, when the `shareMetal` bit of `player.info` is set and `player.metal` > `player.shareMetal`:
  the *recipient* is the partner with the highest *i* whose `metal` < `player.metal`. If there is one,
  the import `players.TransferEnergy(player.index, recipient.index, amount, 1)` is made, with *amount*
  the smaller of (`player.metal` − `player.shareMetal`) × *k3* and `recipient.metalCapacity` −
  `recipient.metal`, where *k3* is the 32-bit float 0x3eaaaaab (Q3). The import is made whatever
  *amount* is: a recipient with no room left gives an *amount* of 0 or below, and the import is still
  made with it.
- **Energy**, read after the metal import: the same with the `shareEnergy` bit, `energy`, `shareEnergy`
  and `energyCapacity`, the factor 0.5 in place of *k3*, and the import `players.TransferMetal`.

When *t* is a multiple of 450 and the `shareMapping` bit of `player.info` is set, then for each partner
in order of *i*, the import `network.SendShareMapInfo(player.index, q.index)` is made, unless
`player.index` is 10 (the partner's index is never 10). The player itself is sent its own map when it
is a partner (Q4), but never receives metal or energy, since its stock isn't below its own. On a tick that
is a multiple of both, the sharing imports come before the map imports.

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
- ***Request*** `R(a, v)`: `a.energyUse` adds *v*. If `a.energyStall` > 0 the request is refused (0);
  otherwise `a.energyUsePaid` adds *v* and the request is granted (1).
- **Products in `I`:** `Game.windStrength` × `d.windgenerator` and `Game.tidal` × `d.tidalgenerator` are
  exact products, not rounded to 32 bits before they enter `I`. The pin is
  `tests/replay/economy/tidal-rounding.txt`, the call at tick 6270 of `UpdatePlayerEconomy(Player#0)`: the
  account's `energyMake` is 0x437b4f71, `Game.tidal` is 0x42436dc7 and `tidalgenerator` is 0x41a80d9b, and
  `energyIncome` becomes 0x449fb45c; adding the product rounded to 32 bits first gives 0x449fb45b.
- **The result of `GetEnergyUse`:** for the wind and tidal rows it is the negated exact product rounded
  once to 32 bits (to nearest, ties to even), which is the slot's `f32` result. The recorded results
  (`o result f32:...`) are the rounded values.
- **The sums of `UpdatePlayerEconomy`**: after each counted unit's term, a sum is the 32-bit float nearest
  to the exact sum of its value before and the term (*M* ⊕ `a.energyMake`, and so on). *M*\* is the 64-bit
  float nearest to *M* + *e*.`Make`; *M* is the 32-bit float nearest to *M*\*. *A* is the 32-bit float
  nearest to *M*\* + `p.X`.
- **The ratios:** *L* = f64(*A* − *S*), a 64-bit value that is not rounded to 32 bits, or 0. When they
  are not 1, *rs* = f32(*A* / *S*) and *rp* = f32(f64(*L* / *P*)). *B* = f32(f64(*L* − *P*)), or 0. The
  wasted amount adds f64(*B* − `p.XCapacity`).
- **The stall:** with *s* = `XStall` and *q* = `XUsePaid`, both 32-bit, the new `XStall` is
  f32(f64(f64(*s* − f64(*rs* × *s*)) + f64(*q* − f64(*rp* × *q*)))): the value of that expression, with
  each f64 a rounding to 64 bits as written, and the f32 the final rounding to 32 bits.
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
- **NaN:** the kit has no recording with a NaN in any field this module reads, and this spec doesn't
  define the result of one.

## 6. Quirks

- **Q1:** `totalEnergyProduced` adds the energy made before its rounding to 32 bits, *M*\*, while
  `totalMetalProduced` adds the metal made after it, *M*.
- **Q2:** `RequestEnergy` ignores its first argument and acts on its second.
- **Q3:** a player shares a third of its surplus metal and half of its surplus energy, and gives it to
  the partner with the highest index whose stock is below its own, not the one with the least.
- **Q4:** shared map information can go to the sharing player itself.
- **Q5:** a producer whose `energyuse` is below 0 earns that energy, but its request counts as refused,
  so an extractor or a metal maker with a negative `energyuse` makes no metal.
- **Q6:** capacities go to `p`, the player whose slice is walked, but the handicap and the cloak payment
  go to each account's owner.
- **Q7:** the cloak cost is truncated to a whole number before it is paid.
- **Q8:** a spend (`SpendEnergy`, `SpendMetal`, `SpendEnergyAndMetal`, and the cloak payment) takes the
  amount from the stock at once and also counts it in the account's `Use`, so it appears in the next
  settlement's usage.
- **Q9:** *W* wraps at 16 bits.

## 7. Examples

Values are given as decimals, with the 32-bit pattern where the rounding matters.

1. **`tests/replay/economy/ten-ring-atoll.txt`, the first call at tick 6270 of
   `UpdatePlayerEconomy(Player#0)`.** `Game.difficulty` is 2, so no handicap. `Game.tidal` is 30.0. The
   counted units are `Unit#1` (not a producer, on, `energyuse` 1, complete, `energymake` 25,
   `metalmake` 1), `Unit#2` (a producer that is not on), `Unit#3` and `Unit#6` (producers, on,
   `energyuse` 3, `extractsMetal` 0.001, `extraction` 1.692 and 1.053), and `Unit#4` and `Unit#5`
   (producers, on, `tidalgenerator` 1). Every account starts at 0.
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
   (25, plus 20 from each of three producers whose `energyuse` is −20), *U* = *P* = 223.72653
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
   the result is −(0.1246 ⊗ 30) = −3.738 (0xc06f3b64).

**Not recorded in the kit's recordings** (`docs/replay-format.md`, "Not recorded"), each with the kit's reason:

- `UpdateResourceSharing`: it acts only in a network game, and test mode runs every player on one machine.
- `UpdatePlayerEconomy` for a player of type 3: type 3 is a remote network player, which only a lobby
  with network players creates.
- `UpdatePlayerEconomy` for a unit whose resource account belongs to another player than the one settled:
  every unit is created in its owner's part of the unit table and `Reset` ties the account to that owner,
  so only a corrupt saved game could differ.
- `LoadUnitAccounts` with a missing box: every saved unit has its account box, so only a saved game from
  an older build lacks one.
- The false sides of `SpendMetal`, `SpendEnergyAndMetal` and `SpendEnergy` (not enough in stock), outside
  the cloak payment: their callers check the stock first (a give or a transfer caps the amount at the
  stock, and a weapon with a per-shot cost fires only when both stocks cover it).
- The false side of `SpendEnergy` in the cloak payment of `UpdatePlayerEconomy` (a cloaked unit that
  cannot pay, which `units.SetStateBits <unit> 4 0` records and which decloaks it): the one cloaked unit of
  the recordings, `Unit#3` of `edges-economy-targeting`, pays at all eight calls (ticks 30 to 240,
  `units.SetStateBits Unit#3 4 1`) from the 60000 energy of `player0.energy`, and no scenario runs a
  cloaked unit out of energy.
- `UpdateMetalExtraction` for an extractor with no script: every shipped extractor has one.

No recording reaches the following, and the rule of the spec is the authority for each:

- `UpdatePlayerEconomy`, cloak step: the moving cost (the one cloaked unit is stationary), the cloak held
  bit set, `workTime` above `Game.ticks`, a fractional cost truncated (Q7, the recorded cost is 7.0), a
  negative or NaN cost, and the player of type 3 skipping the `units.SetStateBits` imports.
- `UpdatePlayerEconomy`, income: no producer with a negative `energyuse` has `extractsMetal` or
  `makesMetal` (Q5), so which metal row wins is not recorded; and a difficulty of 3 or more with an owner of
  type 2 (no handicap).
- The energy half of Q1 (`totalEnergyProduced` adding *M*\* instead of *M*), and the 16-bit wrap of *W*
  (Q9).
- `RequestEnergy` returning 0, `RequestEnergy` with `self` not `r` (Q2), and `RequestEnergyAndMetal`
  refused for a metal stall alone.
- `LoadUnitAccounts` with a read that returns fewer than 24 bytes.
- `UpdateResourceSharing` in full: the partner conditions (index 10, `unitCount`, `unitsCreated`), a
  transfer with an *amount* of 0 or below, the sends of the shared map every 450 ticks, and the player as
  its own partner.
- `Reset` with a player index of 10 or more (undefined).

Not recorded, and the kit gives no reason: the rounding of *M*\*, *A* and *L* and the exact wind product,
which give the same results as a 32-bit reading on every recorded call (only the tidal product has a pin,
section 5).

## 8. Kit version

Kit `interfaces-v3` and `reference-traces-v23`, the versions the gate judges against. Interface
`economy` 2.0.
