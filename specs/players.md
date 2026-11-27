# players

## 1. Role

Players is the per-player state the simulation keeps besides the economy: each player's 30-tick update and
the end of the match, the victory and defeat conditions of a campaign mission, a unit handed to another
player, the killing of all of a player's units, the transfer of energy and metal between players, and the
console's and dialogs' changes to a player (its store, its type, the local and viewed player, the cheat
toggles, the end of the game). The engine runs a player's turn in the players phase in three parts around
other subsystems' work (the head, the body and the tail), then the finish once per tick. Other modules and
the engine reach players with events through `Apply`. Players' own writes into units go out as effects to
units, its grants of resources as events to economy.

## 2. Interface

### Slots (`interfaces/players.h`, version 5.4)

| Slot | When the engine calls it |
| --- | --- |
| `Apply` | for each event of players' inbox: before each run below, at each phase boundary, and at once for a synchronous route or an event published outside the tick (`docs/schedule.md`, "Routes"; `data/routes.csv`) |
| `UpdatePlayerHead` | in the players phase, for each player slot 0 to 9 in order |
| `UpdatePlayerBody` | in the same player's turn, when its head returned 1, after the other subsystems' work of the turn (`docs/schedule.md`, the pass `players`) |
| `UpdatePlayerTail` | in the same turn, when the body returned 1 or 2, after economy's settlement and the viewed player's radar passes |
| `UpdatePlayersFinish` | once per tick, after the tenth player's turn |
| `UpdatePlayers`, `UpdatePlayersOnStream` | the whole players pass, when the engine runs it outside the tick loop (`UpdatePlayersOnStream` with the load stream at the load of a match, `docs/schedule.md`, "The load"; `UpdatePlayers` from the in-game dialogs) |
| `DeactivateConditions` | when a unit command stops the conditions of the mission |
| `RegionSize`, `Init` | once per match, at its load, before any event and before the first tick |
| `Shutdown` | once per match, when it is unloaded |

### The engine's slots

The kit's entry (`wasm/modules.json`, `modules`, the entry named `players`) names 17 slots under `engine`. By
ABI section 6, "Engine slots", each stays in the engine: the host calls the engine's native slot, the module
doesn't export it, and the native slot reads and writes the module's state in the block, the region included
(section 3). They are the 15 "load class" slots (`LoadDefaultAIScript`, `InitPlayers`, `FreePlayers`,
`InitPlayerResources`, `LoadPlayerControllers`, `LoadPlayers`, `SavePlayers`, `RebuildAIFeatureCells`,
`SetupPlayerSlot`, `LoadConditions`, `SaveConditions`, `RegisterConditions`, `FreeConditions`,
`DeleteConditions`, `NewConditions`), `UpdateBlink` (presentation) and `WriteState` (a dump query, ABI 9.3,
"What stays in the engine"). ozgb/btm#43 moves load-class slots into modules later. This spec describes
only what those slots leave in the block that the module's slots read (section 3).

### The manifest entry

The kit's entry for players is the module's entry, and this spec follows it exactly:

- `name` and `interface`: `players`.
- `majors`: `players` 5, `mission` 2, `units` 7, `map` 9, `features` 5, `network` 2, `los_radar` 5,
  `economy` 5, `services` 4.
- `data_base` 524288 and `table_base` 368: the module's window is the 32768 bytes (the manifest's `window`)
  from 524288, and its table range the 32 entries (`table_slots`) from 368 (ABI 2).
- `exports`, in this order: `Apply`, `UpdatePlayers`, `DeactivateConditions`, `UpdatePlayersOnStream`,
  `UpdatePlayerHead`, `UpdatePlayerBody`, `UpdatePlayerTail`, `UpdatePlayersFinish`, `RegionSize`, `Init`,
  `Shutdown`.
- `data`: `g_game`, `g_simGlobals`, `g_extTdf`, `g_blockBase`, `g_region` (ABI 8.2). The module reads
  `g_game`, `g_simGlobals`, `g_blockBase` and `g_region`; `g_region` is the address of its region in every
  slot after `Init`, so the module keeps no pointer of its own.
- Slot kinds (the entry's `slots`):

| Slot | Kinds |
| --- | --- |
| `Apply` | `[linear, int] -> int` (the event, its size) |
| `UpdatePlayers`, `UpdatePlayersFinish` | `[] -> void` |
| `DeactivateConditions` | `[block] -> void` (the conditions object) |
| `UpdatePlayersOnStream` | `[int] -> void` (the stream: 0 main, 1 load) |
| `UpdatePlayerHead` | `[int] -> int` |
| `UpdatePlayerBody` | `[int, int] -> int` (the player, the stream) |
| `UpdatePlayerTail` | `[int] -> void` |
| `RegionSize` | `[linear] -> int` (a `PlayersMapInfo`) |
| `Init` | `[linear, block, int] -> void` (the `PlayersMapInfo`, the region, its size) |
| `Shutdown` | `[] -> block` |

- `imports`, every one listed and nothing else (ABI 9.1):

| Import | Kinds | Used for |
| --- | --- | --- |
| `mission.GetGameType` | `[] -> int` | the game type: 1 campaign, 2 skirmish, 3 multiplayer |
| `mission.GetMissionParams` | `[linear] -> void` | the respawn's lava test (a `MissionParams` of the module's own) |
| `units.CreateUnit` | `[int, int, linear, int, int, int] -> block` | the respawned commander |
| `units.FindUnitTypeId` | `[linear] -> int` | a unit type by name (conditions, the respawn) |
| `map.UnitsInRange` | `[linear, int, int, linear, int] -> int` | the units inside a radius condition's circle |
| `map.GetCellMeanHeight` | `[linear] -> int` | the respawn's lava test |
| `map.CanPlaceUnitFootprint` | `[block, int, linear, int] -> int` | the respawn's footprint test (a `Point16` cell of the module's own) |
| `features.FindFeatureAtPos` | `[linear, linear, linear] -> int` | the respawn's test for a feature |
| `network.BroadcastPlayerInfo` | `[] -> void` | a defeated local player put in watch mode |
| `network.SendShareMetal` | `[int, int, f32] -> void` | an announced transfer of energy (section 4, Q1) |
| `network.SendShareEnergy` | `[int, int, f32] -> void` | an announced transfer of metal (Q1) |
| `network.BroadcastPlayerEconomy` | `[block] -> void` | the viewed player's economy, in a multiplayer game |
| `los_radar.ClampWorldPosToTerrain` | `[int, int, linear] -> void` | a radius condition's centre, once |
| `economy.Spend` | `[linear] -> int` | the giver's spend of an announced transfer (an `EconomySpend` of the module's own) |
| `services.RandomInt` | `[int] -> int` | the respawn's position |
| `services.RecordSimEvent` | `[int, int, linear, int] -> void` | the sim events of section 2, "Events" |
| `services.Emit` | `[linear, int] -> void` | every effect and event the module publishes |

- `publishes`, the kinds and sizes `Emit` takes (ABI 9.3): 0 (`ECONOMY_RESET`, 12), 4 (`ECONOMY_INCOME`, 24),
  128 (`LOS_RADAR_UPDATE_UNIT`, 8), 133 (`LOS_RADAR_RECALCULATE`, 8), 208 (`PLAYERS_EFFECT_KILL_UNIT`, 12), 209
  (`PLAYERS_EFFECT_SEND_UNIT`, 12), 210 (`PLAYERS_EFFECT_CAPTURE_UNIT`, 48). The module's slots publish 4, 133,
  208, 209 and 210; 0 and 128 are published only by engine slots. Every route of these kinds is synchronous
  except economy's two (`data/routes.csv`), so an effect to units or los_radar has been applied when `Emit`
  returns, and its writes are in the block for what the module reads after it.
- `blocked`: true. By ABI 4.2 an entry with `blocked` has no `.wasm` and no budgets (`fuel_budget`,
  `size_budget`), and the host runs the native default for every slot. A module built from this spec meters
  itself by rule version 1 (ABI section 11) and exports `__fuel_set` and `__fuel_get`; its budgets come with a
  later entry.
- Hooks: every slot of `interfaces/players.h` says "Hooks: none"; the event kind `PLAYERS_GIVE_UNIT` names
  `UnitTransfer`. The kit defines no hook in the module ABI and the import list has none, so the module raises
  none; no recording shows one.

### Routes and the recordings

A recording shows the module's `Emit` as `i services.SchedPublish 4 <size>` (the recorded call of ABI 9.3,
"Routes in the recordings"; 4 is players' outbox), its `economy.Spend` as `i services.ScheduleEconomySpend`, and its `map.CanPlaceUnitFootprint` as
`i services.ScheduleMapCanPlaceUnitFootprint`.
An `o rng.seed` or `w Game.playerIndex` line after a publish is what the synchronous delivery of the effect
changed, not a write of the module. A `c` line under `map.UnitsInRange` doesn't occur: the module takes ids,
not a callback.

### Arguments outside the block

- **The event** of `Apply` is the payload struct of its kind in `interfaces/players.h`, laid out with no
  padding (ABI 6.1): `PlayersEventHeader` (8 bytes: `kind`, `unit` as a block offset), then the kind's fields.
  Sizes: `PLAYERS_GIVE_UNIT` 48, `PLAYERS_KILL_UNITS` 12, `PLAYERS_TRANSFER` 28, `PLAYERS_SET_STORE` 24,
  `PLAYERS_SET_TYPE` 16, `PLAYERS_END_GAME` 12, `PLAYERS_SET_CHEATS` 16, `PLAYERS_SET_CONTROL` and
  `PLAYERS_SET_VIEW` 12, every other kind of the header (the header alone) 8, and the effect kinds 208, 209
  and 210 their payloads' 12, 12 and 48.
- **`PlayersMapInfo`** (`RegionSize`, `Init`): one `int`, `playerCount`. The module doesn't read it.
- **What the module builds for an import** lives on its stack or in its window: the `EconomySpend` of
  `economy.Spend` (24 bytes: `kind` 3, `player`, `unit` 0, `resources`, `energy`, `metal`), the effects and
  events it gives `Emit`, the argument array of `services.RecordSimEvent`, the `Vec3` of a position, the
  `MissionParams` (124 bytes; the module reads `lavaWorld`, the sixth 32-bit field, at offset 20), the id
  buffer of `map.UnitsInRange` (256 unsigned 16-bit ids) and the NUL-terminated names given to
  `units.FindUnitTypeId`. A name given to `units.FindUnitTypeId` may also be the field itself in the block
  (a condition's `name`, a `SideDef.commander`): the recording shows the text.
- **The `Vec3` of a radius condition** is the condition's own `pos` in the block: `los_radar.ClampWorldPosToTerrain`
  writes it in place, and `map.UnitsInRange` reads it there.

### Events

The module's slots record these kinds through `services.RecordSimEvent`, with flags 0; the kit's
`docs/sim-events.md` gives their arguments and phases:

- `SIM_EVENT_SOUND`, arguments `[0, 0]` (sound name 0, the victory condition sound, and play flag 0): when a
  victory condition is satisfied for the first time, in the cases of section 4, "Conditions" (it is recorded
  only while the condition's `celebrated` is 0, which it then sets to 1).
- `SIM_EVENT_FOCUS_COMMANDER`, arguments `[1]`: when the commander of the local player is respawned
  (section 4, "The respawn").
- `SIM_EVENT_WATCH_PROMPT`, arguments `[p]`: when the defeated local player *p* of a multiplayer game is put
  in watch mode and no active AI player is left (section 4, "Watch mode").
- `SIM_EVENT_WATCH_MODE_NOTICE`, arguments `[p]`: on the same path, when an active AI player is left and at
  least one combat player.

The terms "unit flag 0x10000000", "unit flag 0x4000", "map flag 1", "map flag 2" and "info flag 0x40" are
those of `docs/sim-events.md`, Terms.

### Kit gaps

What this spec needs and the kit doesn't give; the maintainer routes them (K2 and K3 were closed by the kit at `interfaces-v68`, so the numbers skip them):

- **K1, the class of a condition.** A condition (section 3) is told apart by its first word, the vtable
  pointer of its class, which the engine's load-class slots write and the kit gives no value for (the
  layout's `MissionCondition` has `vptr` true and no field for it; the recordings name the class,
  `VictoryKillAllOfType#1`, but never the word). Every slot that runs a condition needs the class. Until the
  kit gives the 18 words (or a class number in the block), this spec states each class's behaviour, and a
  module can't pick it from the block alone.
- **K4, the saved game's account names** serve only the engine
  slots `LoadConditions` and `SaveConditions`; no exported slot needs them.
- **K5, the whole pass.** `UpdatePlayers` and `UpdatePlayersOnStream` are exported, but the players pass
  calls other subsystems' slots (pathfinding, movement, ai, los_radar, units, economy; `docs/schedule.md`, the
  passes `players` and `players.los`), and the entry lists none of them as an import. No recording holds
  either slot.
- **K6, a condition list that is empty.** A campaign check of an empty list adds a condition to it (section
  4, "Checks"), which needs a block allocation (`services.SimAlloc`, not in the entry) and the class word of
  K1. No recording has an empty list.
- **K7, `PLAYERS_DEACTIVATE_CONDITIONS` with no conditions**: the header lists no refusal for it. This spec
  makes it change nothing with outcome 0 (section 4, `Apply`).
- **K8, a respawn whose `units.CreateUnit` gives null.** The reference reads the new unit for its incomes
  without a test, so the case has no defined result; the kit says nothing of it either.

## 3. State

### Owned

**The region** (`RegionSize`, `Init`; ABI 8.2 `g_region`), 84 bytes, little-endian. Its layout is fixed by the
engine slots, which write it too (`InitPlayers` sets each player's clocks at the start of a match, `LoadPlayers`
restores them, `SavePlayers` stores them):

| Bytes | Field |
| --- | --- |
| 0 to 1 | the *countdown*, a signed 16-bit number; below 0 means not running |
| 2 to 3 | unused, 0 |
| 4 to 43 | the *update clock* of players 0 to 9, ten unsigned 32-bit ticks (player *p* at 4 + 4*p*) |
| 44 to 83 | the *win clock* of players 0 to 9, ten 32-bit ticks; the module's slots never read or write it |

**`Player`** (the array `Game.players`, 331 bytes apart from offset 7011; index 10 is the record past the ten
players, `Game.players_1b63`): `rank` (byte), `energy`, `metal` (floats; written by `PLAYERS_SET_STORE`),
`type` (byte), `energyStorageBonus`, `metalStorageBonus` (floats) and bit 0 of `flags` (the respawn).

**`PlayerInfo`** (a player's `info`, a `Ref`): `kind` (byte, `PLAYERS_SET_TYPE`) and `bit6` (info flag 0x40).

**`Game`:** `endGameFlags` (16 bits: bit 2 *ended*, bit 5 *won*, bit 6 *lost*), `endGameVictory` (byte),
`localPlayer`, `playerIndex` (bytes, the local and the viewed player), `uiOptionFlags` (16 bits) and bits 0 and
1 of `mapFlags` (map flags 1 and 2).

**`SimGlobals.g_economyPacketTick`** (32 bits): the count of the viewed player's tails in a multiplayer game.

**The conditions** (`Game.list_391ed`, a `Ref<MissionConditions>`, null when there are none): the record
`MissionConditions` (140 bytes): `victory` and `defeat` (16 `Ref<MissionCondition>` each), `victoryCount`,
`defeatCount` (at most 16) and `active`. Each condition starts with the class word (K1), then `satisfied` and
`celebrated` at 4 and 8. The classes, by their `layout.json` record, and the fields the module writes:

| Class | List | Fields after `celebrated` |
| --- | --- | --- |
| `VictoryKillEnemyCommander` | victory | none |
| `VictoryDestroyAllUnits` | victory | none |
| `VictoryKillAllMobileUnits` | victory | `numUnits` (16) |
| `VictoryBuildUnitType` | victory | `name` (16, 32 bytes), `id` (48, 16 bits) |
| `VictoryCaptureUnitType` | victory | `name` (12) |
| `VictoryKillAllOfType` | victory | `name` (16), `id` (48), `count` (50, 32 bits) |
| `VictoryKillUnitType` | victory | `name` (12), `numLeftToKill` (44) |
| `VictoryMoveUnitToRadius` | victory | `name` (16), `pos` (48, a `Vec3`), `radius` (60) |
| `VictoryUnitTypePassesX` | victory | `name` (16), `x` (48) |
| `VictoryUnitTypePassesZ` | victory | `name` (16), `z` (48) |
| `VictoryTimerRunsOut` | victory | `endTick` (12, unsigned) |
| `DefeatCommanderKilled` | defeat | none |
| `DefeatAllUnitsKilled` | defeat | none |
| `DefeatAllUnitsKilledOfType` | defeat | `name` (16), `id` (48), `count` (50) |
| `DefeatUnitTypeKilled` | defeat | `name` (12), `numLeftToKill` (44) |
| `DefeatDeathTimerRunsOut` | defeat | `endTick` (12) |
| `DefeatAnyUnitPassesX` | defeat | `x` (16) |
| `DefeatAnyUnitPassesZ` | defeat | `z` (16) |

Offsets are bytes from the condition's start; `id` and `count` are unaligned. A `name` is NUL-terminated
(empty for "any type" in the radius and passes classes). The module never writes a class word, a `name`, an
`x`, a `z`, a `radius` or an `endTick`. A radius condition's `pos` holds the centre's x and z as given, and y
the marker 0x12345678 until the centre is put on the ground (section 4).

### Read

- `Game`: `ticks` (unsigned for every comparison), `players`, `numPlayers` (unsigned 16 bits),
  `commanderDeath` (int: 2 is the respawn of a commander, "Deathmatch"), `baseX`, `baseY`, `width`, `height`
  (ints), `seaLevel` (unsigned byte), `units` and `defs` (`Ref`s to the unit table, 280 bytes a unit, and the
  unit types, 585 bytes a type), `sideDefs` (`SideDef[5]`, 562 bytes each; its `commander` is the name of the
  side's commander type), `field_1df2` (16 bits at 7666, the same word as `Game.players[1].unitCount`),
  `SimGlobals.g_quirks.rankCompactionPerTick` (`interfaces/quirks.h`; 0 in every recording).
- `Player`: `active` (int), `type`, `index`, `rejectReason` (bytes), `info`, `unitsBegin`, `unitsEnd` (`Ref`s),
  `unitCount` (unsigned 16 bits), `unitsCreated` (int), `allied`, `alliedBy` (11 bytes each, indexed by player).
- `PlayerInfo`: `side` (byte), `host` (bit 0 of `flags_97`), `bit6` (info flag 0x40, watch mode), `watching`
  (bit 7 of `flags_9b`, the host's "watching allowed"), `f9d_1` (bit 1 of `flags_9d`, allied victory),
  `energy`, `metal` (unsigned 16 bits: the host's start resources, in hundreds).
- `Unit`: `flags` (bit 28 unit flag 0x10000000 alive, bit 14 unit flag 0x4000 dying, bit 5 (0x20), bit 30 of a
  carrier (0x40000000)), `player` (`Ref<Player>`), `playerIndex` (byte), `def` (`Ref<UnitDef>`),
  `unitDefIndex` (16 bits; 0 is an empty slot), `motion` (`Ref`, null for a unit that doesn't move), `buildLeft`
  (float), `postTransferHoldoff` (int), `carrier` (`Ref<Unit>`), `cell` (`Point16`, signed `x`, `y`).
- `UnitDef.unitname` (32 bytes, NUL-terminated).

### Words used here

- A player *plays* when its `active` is not 0, its `type` is 1, 2 or 3 and its `index` is not 10.
- A player is *still in* when it plays and its `unitCount` is not 0 or its `unitsCreated` is 0.
- *Active AI players*: the players 0 to 9 that are still in and whose `type` is 2.
- *Combat players*: the players 0 to 9 that are still in, whose `type` is 1, or 3 with `info.kind` 1, and whose
  info flag 0x40 is clear.
- A player is *counted for rank* when it plays, is not (`unitCount` 0 with `unitsCreated` not 0), and its info
  flag 0x40 is clear.
- The *host slot*: the lowest player 0 to 9 whose `type` is not 0 and whose `info.host` is set, else 10.
- *L* is `Game.localPlayer`; *local* means player *L*.
- A player's *slice* is the unit table from its `unitsBegin` to its `unitsEnd`, both included, 280 bytes apart.
  A *scan* of a slice visits, in table order, each unit whose `unitDefIndex` is not 0 and stops after the first
  visit that says stop.
- Two names *match* when they are equal after mapping `A` to `Z` to `a` to `z` (ASCII, nothing else).
- *A unit can be counted* when its bit 5 (0x20) is set, its `buildLeft` is +0.0 or −0.0, its
  `postTransferHoldoff` is 0, and its `carrier` is null or a unit whose bit 30 (0x40000000) is set.
- *Celebrate* a condition: when its `celebrated` is 0, record `SIM_EVENT_SOUND` and set `celebrated` to 1.
- *Advance the countdown* with *v* (0 or 1): when the countdown is below 0 it becomes 4; otherwise it becomes
  one less, and when that is below 0 the match *ends with v*: `endGameFlags` gets bit 2 and bit 5 (*v* 1) or
  bit 6 (*v* 0), and `endGameVictory` becomes *v*.

### Between ticks

The region, the conditions and the fields above. Nothing else: the module keeps no statics between calls.

## 4. Behaviour

Within one call, only the order of import calls is observable. The block writes are compared at the end of
the call, so the order they are listed in carries no meaning, except where a later read in the same call sees
an earlier write or an effect's synchronous delivery, as said.

### `RegionSize(info)`

Returns 84. No import, no write.

### `Init(info, region, size)`

The region is zero on entry. The countdown becomes −1 (bytes 0 and 1 are 0xff); nothing else is written. The
engine passes the size `RegionSize` gave.

### `Shutdown()`

Returns the region `Init` was given (as its address, which the host checks is in the block); no import, no
write. Without an `Init` it returns 0.

### `UpdatePlayerHead(player)`

*p* is the low byte of `player`. Returns 1 when *p* is below 10 and player *p* plays, else 0. No import, no
write.

### `UpdatePlayerBody(player, stream)`

*p* is the low byte of `player`. The engine passes 0 to 9 only; for a *p* above 9 the reference has no
defined result, and this spec makes the slot return 0 and change nothing (ozgb/btm#25). The stream is not
used.

1. When player *p*'s update clock is above `Game.ticks` (unsigned), the result is 0 and nothing else happens.
2. Otherwise the clock adds 30 (32 bits, wrapping). When *p* is not *L*, the part below is skipped.
3. For *p* = *L*, the first import is `mission.GetGameType`, *g*. The outcomes, with the imports after that
   first one in order (*V* is a victory check and *D* a defeat check, "Checks" below, each with its imports):

| Case | Imports after the first | Effect |
| --- | --- | --- |
| *g* = 1, *V* is 1 | *V* | advance the countdown with 1 |
| *g* = 1, *V* is 0, *D* is 1 | *V*, *D* | advance the countdown with 0 |
| *g* = 1, *V* and *D* are 0 | *V*, *D* | none |
| *g* ≠ 1, player *L* is active and its info flag 0x40 is set, *V* is 1 | *V* | advance the countdown with 1 |
| *g* ≠ 1, the flag case above, *V* is 0 | *V* | none |
| *g* ≠ 1, player *L* is not active or its info flag 0x40 is clear, *D* is 0, *V* is 1 | *D*, *V* | advance the countdown with 1 |
| the same, *D* is 0, *V* is 0 | *D*, *V* | none |
| the same, *D* is 1, the countdown is below 0 | *D* | the countdown becomes 4 |
| the same, *D* is 1, the countdown is 1 or more | *D* | the countdown becomes one less |
| the same, *D* is 1, the countdown is 0, `Game.commanderDeath` is 2 | *D*, the respawn's | the countdown becomes −1; the respawn (below) |
| the same, *D* is 1, the countdown is 0, `Game.commanderDeath` is not 2 | *D*, watch mode's | the countdown becomes −1; watch mode (below) |

4. The result, read after the writes above: 2 when player *p* is still in, its `type` is 1 or 2, bit 2 of
   `endGameFlags` is clear and the countdown is below 0; else 1.

**Watch mode** (player *L*, defeated; *h* is the host slot): the next import is `mission.GetGameType`, *g2*.

| Case | Imports after *g2*, in order | Effect |
| --- | --- | --- |
| *g2* = 3, *L*'s `rejectReason` is 0, `Game.players[h].info.watching` is set or an active AI player exists; no active AI player | `Emit` RECALCULATE, `network.BroadcastPlayerInfo`, `RecordSimEvent` WATCH_PROMPT | *L*'s info flag 0x40 is set; map flags 1 and 2 are cleared |
| the same, with an active AI player and a combat player | `Emit` RECALCULATE, `network.BroadcastPlayerInfo`, `RecordSimEvent` WATCH_MODE_NOTICE | as above, and `endGameVictory` becomes 0 |
| the same, with an active AI player and no combat player | `Emit` RECALCULATE, `network.BroadcastPlayerInfo` | as the first row |
| any other | none | `endGameFlags` gets bit 2, `endGameVictory` becomes 0, and bit 6 too when *L*'s `rejectReason` is 0 |

RECALCULATE is a `LosRadarRecalculate` (`kind` 133, `reset` 1, 8 bytes); the two events carry `[L]`. The
counts of active AI and combat players are taken after *L*'s info flag 0x40 is set.

**The respawn** (player *L*; *h* is the host slot and *H* is `Game.players[h].info`, the record past the ten
players when *h* is 10), with these imports in order:

1. `units.FindUnitTypeId` with `Game.sideDefs[s].commander`, *s* the `side` of player *L*'s info; *t* is the
   result's low 16 bits, and *d* the unit type *t* of `Game.defs`.
2. Up to 9999 tries of a position *P*: *cx* = `baseX` / 10 and *cz* = `baseY` / 10 (section 5);
   `services.RandomInt(baseX − 2·cx)` gives *a*, then `services.RandomInt(baseY − 2·cz)` gives *b*;
   *P* = ((*a* + *cx*) · 65536, 0, (*b* + *cz*) · 65536). Then nine `map.CanPlaceUnitFootprint(d, 0, cell, 1)`,
   the cell (*X*, *Z*) for *Z* from *P*.z − *Hh* and *X* from *P*.x − *Hw*, each in three steps of *Hh* and *Hw*
   (*Hh* = `height`·65536, *Hw* = `width`·65536), *Z* the outer loop, the cell's `x` and `y` the 16-bit
   truncations of the unsigned values shifted right by 20 (section 5). *P* is taken when all nine answer
   non-zero and then the low 16 bits of `features.FindFeatureAtPos(P, 0, 0)` are 0xffff and either `mission.GetMissionParams`
   gives `lavaWorld` 0, or `lavaWorld` is not 0 and `map.GetCellMeanHeight(P)` is above `seaLevel`. A try whose
   footprint count is below 9 makes neither of the last calls, and one whose feature test fails doesn't ask
   for the mission's figures. After the 9999th try the last *P* is taken.
3. `units.CreateUnit(L, t, P, 1, 1, 0)` gives the unit *u*. Whatever it gives, player *L*'s `flags` gets bit 0,
   its `energyStorageBonus` becomes the float of max(*H*.energy·100, 200) and its `metalStorageBonus` the float
   of max(*H*.metal·100, 200). Then, for a *u* that is not null, `Emit` INCOME (`EconomyIncome`, 24 bytes: `kind` 4, `player`
   *u*.`playerIndex`, `unit` the block offset of *u*, `resources` 1, `energy` float(*H*.energy)·100.0, `metal`
   0.0), `Emit` INCOME (`resources` 2, `energy` 0.0, `metal` float(*H*.metal)·100.0), `Emit` RECALCULATE
   (`reset` 1), and `RecordSimEvent` FOCUS_COMMANDER `[1]`. What follows a null *u* is not defined by the
   reference or any recording (K8).

### Checks

A victory check *V* and a defeat check *D* read the conditions object *C* = `Game.list_391ed`. When *C*.`active`
is 0 the check gives 0 with no import. Otherwise its first import is `mission.GetGameType`, *g*:

| *g* | *V* | *D* |
| --- | --- | --- |
| 1 | each victory condition in list order, stopping at the first that isn't met; 1 when all are met | each defeat condition in list order, stopping at the first that is met; 1 when one is |
| 2 | 1 when every player *i* other than *L* with `players[L].allied[i]` 0 has `unitCount` 0 | 1 when player *L*'s `unitCount` is 0 |
| 3 | the allied victory below | as for 2 |
| other | 0 | 0 |

An empty list, in a check of game type 1, first gets one condition: `VictoryDestroyAllUnits` for the victory
list, `DefeatAllUnitsKilled` for the defeat list, both with `satisfied` and `celebrated` 0 and the count 1 (K6).

**The allied victory** is 0 when `Game.commanderDeath` is 2. Otherwise, for each player *i* from 0 to 9 other
than *L* that plays and whose info flag 0x40 is clear: the result is 0 when its `unitsCreated` is 0; when its
`unitCount` is not 0, the result is 0 unless its `info.f9d_1` and player *L*'s are set, `players[L].allied[i]`
and `players[L].alliedBy[i]` are not 0, and `players[i].allied[j]` is not 0 for every player *j* 0 to 9 that
is still in (*L* and *i* included). After every *i*, the result is 1.

**A condition is met** (game type 1), by class, with its imports and writes:

| Class | Met when | Imports and writes |
| --- | --- | --- |
| `VictoryDestroyAllUnits` | `Game.field_1df2` is 0 | when met: celebrate |
| `VictoryBuildUnitType` | `satisfied` is not 0, or becomes so | when `satisfied` is 0: if `id` is 0, `units.FindUnitTypeId(name)` and `id` becomes its 16-bit result; then a scan of player 0's slice: a unit whose `unitDefIndex` equals `id` and whose `buildLeft` is ±0.0 sets `satisfied` to 1 and celebrates; the scan stops once `satisfied` is not 0 |
| `VictoryMoveUnitToRadius` | `satisfied` is not 0 after the visits | when `pos.y` is 0x12345678: `los_radar.ClampWorldPosToTerrain` with the x and z of `pos` and `pos` itself as the output. Then `map.UnitsInRange(pos, radius, k, ids, 256)` with *k* 0, 256, 512 and so on, while each answer is 256; each id *n* of an answer, in order, visits unit *n* of `Game.units`: a unit with `playerIndex` 0, whose type matches `name` (any type when `name` is empty) and that can be counted sets `satisfied` to 1 and celebrates. The visits never stop early |
| `VictoryUnitTypePassesX` (`Z`) | `satisfied` is not 0, or becomes so | when `satisfied` is 0, a scan of player 0's slice: a unit whose type matches `name` (any when empty) and whose `cell.x` (`cell.y`) is within 2 of `x` (`z`) sets `satisfied` and celebrates; stop once set |
| `VictoryTimerRunsOut`, `DefeatDeathTimerRunsOut` | `Game.ticks` ≥ `endTick`, both unsigned | none |
| `DefeatAllUnitsKilled` | no unit of player 0's slice can be counted | `satisfied` becomes 1, then a scan of player 0's slice sets it to 0 at the first unit that can be counted, and stops there |
| `DefeatAnyUnitPassesX` (`Z`) | `satisfied` is not 0, or becomes so | when `satisfied` is 0, a scan of player 1's slice: a unit whose `cell.x` (`cell.y`) is within 2 of `x` (`z`) sets `satisfied` to 1 (no sound); stop once set |
| every other class | `satisfied` is not 0 | none |

A unit's type matches when `name` matches its `def.unitname`. *Within 2*: |*c* − *x*| ≤ 2, with the cell's
coordinate sign-extended and the difference a 32-bit signed number.

### Conditions

The events `PLAYERS_UNIT_DIED` and `PLAYERS_GIVE_UNIT` tell every condition of the victory list, in order,
then every condition of the defeat list, of the unit *u*. Only these classes act; *d* is *u*.`def`, and the
commander name is `Game.sideDefs[s].commander` with *s* the `side` of *u*.`player`'s info:

| Notice | Class | When | Effect |
| --- | --- | --- | --- |
| died | `VictoryKillEnemyCommander` | *u*.`playerIndex` is 1 and *d*.`unitname` matches the commander name | `satisfied` 1; celebrate |
| died | `VictoryKillAllMobileUnits` | *u*.`playerIndex` is 1 and *u*.`motion` is not null | `numUnits` becomes 0, then a scan of player 1's slice adds 1 for each unit with a `motion`, stopping once `numUnits` is above 1; when `numUnits` is at most 1: `satisfied` 1, celebrate |
| died | `VictoryKillAllOfType` | `satisfied` is 0, *u*.`playerIndex` is 1 and `name` matches *d*.`unitname` | `units.FindUnitTypeId(name)`, `id` becomes its 16-bit result, `count` becomes 0, then a scan of player 1's slice adds 1 for each unit whose `unitDefIndex` equals `id`, stopping once `count` is above 1; when `count` is at most 1: `satisfied` 1, celebrate |
| died | `VictoryKillUnitType` | `numLeftToKill` is above 0, *u*.`playerIndex` is 1 and `name` matches *d*.`unitname` | `numLeftToKill` becomes one less; when that is at most 0: `satisfied` 1, celebrate |
| died | `DefeatCommanderKilled` | *u*.`playerIndex` is 0 and *d*.`unitname` matches the commander name | `satisfied` 1 |
| died | `DefeatUnitTypeKilled` | `name` matches *d*.`unitname`, any player (Q3) | `numLeftToKill` becomes one less; when that is at most 0: `satisfied` 1 |
| died | `DefeatAllUnitsKilledOfType` | `name` matches *d*.`unitname`, any player | `units.FindUnitTypeId(name)`, `id` becomes its result, `count` 0, then a scan of player 0's slice and then one of player 1's, each adding 1 per unit whose `unitDefIndex` equals `id` and stopping once `count` is above 1 (so the second scan visits at most one unit when the first ended above 1, Q4); when `count` is at most 1: `satisfied` 1 |
| captured | `VictoryCaptureUnitType` | *u*.`playerIndex` is 1 and `name` matches *d*.`unitname` | `satisfied` 1; celebrate |

No class acts on `PLAYERS_UNIT_CREATED`.

### `Apply(event, size)`

The result is the outcome. A `size` below 8, or below the size of the event's kind (section 2), gives 0 with
no change and no import. *u* is the unit at block offset `event.unit` (null for 0), and *C* the conditions
object. By kind:

| Kind | Refused (outcome 0, no change) when | Effect, imports in order | Outcome |
| --- | --- | --- | --- |
| `PLAYERS_GIVE_UNIT` | *u* is null, `player` is above 9 (unsigned), or *C* is null | the hand-over below | 0 or 1 |
| `PLAYERS_KILL_UNITS` | `player` is above 10 (unsigned) | the kill below | 1 |
| `PLAYERS_UNIT_CREATED` | *u* or *C* is null | none | 1 |
| `PLAYERS_UNIT_DIED` | *u* or *C* is null | the died notice of "Conditions" | 1 |
| `PLAYERS_TRANSFER` | `from` or `to` is above 10 (unsigned), or `resource` is not 0 or 1 | the transfer below | 0 or 1 |
| `PLAYERS_SET_STORE` | `player` is above 9, or `resource` is not 0 or 1 | the store *S* (`energy` for 0, `metal` for 1) of the player becomes `amount` when `add` is 0, else *S* + `amount` (section 5) | 1 |
| `PLAYERS_SET_TYPE` | `player` is above 9 | the player's `type` becomes the low byte of `type`; when the 32-bit `type` is not 3, its `info.kind` becomes that byte too | 1 |
| `PLAYERS_END_GAME` | `victory` is not 0 or 1 | `endGameVictory` becomes `victory`; `endGameFlags` gets bit 2 and bit 5 (1) or bit 6 (0) | 1 |
| `PLAYERS_DEACTIVATE_CONDITIONS` | *C* is null (K7) | *C*.`active` becomes 0 | 1 |
| `PLAYERS_SET_CHEATS` | `clear` or `set` has a bit outside 0x788 | `uiOptionFlags` becomes (`uiOptionFlags` with the bits of `clear` cleared) with the bits of `set` set | 1 |
| `PLAYERS_LEAVE_GAME` | never | `endGameFlags` gets bit 2 | 1 |
| `PLAYERS_SET_CONTROL` | `player` is above 9, or the player doesn't play | `localPlayer` and `playerIndex` become `player` | 1 |
| `PLAYERS_SET_VIEW` | as SET_CONTROL | `playerIndex` becomes `player` | 1 |
| `PLAYERS_COMPACT_RANKS` | `g_quirks.rankCompactionPerTick` is not 0 | the rank pass below | 1 |
| any other kind | always | none | 0 |

A field the table doesn't use is not read: the `unit` of a kind that names none is ignored.

**The hand-over** of *u* to player *r* = `player`, with *o* = *u*.`player`:

| Case, by the first row that matches | Imports | Outcome |
| --- | --- | --- |
| *o* is player *r* (the same record) | none | 0 |
| *u* is not alive (unit flag 0x10000000 clear) or is dying (unit flag 0x4000 set) | none | 0 |
| *o* is active with `type` 1 or 2, and *r* is not active | the captured notice | 0 |
| *o* is active with `type` 1 or 2, and *r* has `type` 3 | the captured notice, `Emit` SEND_UNIT | 1 |
| *r* is not active, or its `type` is not 1 or 2 | the captured notice | 0 |
| any other | the captured notice, `Emit` CAPTURE_UNIT | 1 |

The captured notice (section "Conditions") comes before the `Emit`, and its `RecordSimEvent`, when it has one,
is the first import. SEND_UNIT is a `PlayersSendUnit` (12 bytes: 209, *u*'s offset, *r*); CAPTURE_UNIT a
`PlayersCaptureUnit` (48 bytes: 210, *u*'s offset, *r*, then the event's `record`, all nine words copied as
they are).

**The kill** of player *k*'s units (*k* = `player`, 10 being the record past the ten players): nothing happens
when the player's `unitCount` is 0 or its `unitsBegin` is null. Otherwise `unitsEnd` is read once, and each unit
from `unitsBegin` to it, in table order, that is alive and not dying when its turn comes gives one `Emit`
KILL_UNIT (`PlayersKillUnit`, 12 bytes: 208, the unit's offset, `selfDestruct`): `selfDestruct` is 0 when the
unit's `player` is active with `type` 1 or 2, else 1. Each KILL_UNIT is delivered before `Emit` returns, so a
later unit's flags are read after the kills before it took effect.

**The transfer** (`from` *f*, `to` *t*, `amount` *a*, `announce` *n*, nonzero meaning announced):

| Case | Imports | Outcome |
| --- | --- | --- |
| *f* or *t* is 10 | none | 0 |
| *n* is not 0, and *a*, lowered to the giver's store (`energy` for resource 0, `metal` for 1) when it is above it (section 5), is 0.0 | none | 0 |
| *n* is not 0, and the lowered *a* is not 0.0 | `economy.Spend`, `Emit` INCOME, the network send | 1 |
| *n* is 0, *a* = 0.0 | none | 0 |
| *n* is 0, *a* ≠ 0.0 | `Emit` INCOME | 1 |

The spend is an `EconomySpend` for player *f*, unit 0, with `resources` 1 and `energy` *a* (resource 0), or
`resources` 2 and `metal` *a* (resource 1), the other amount 0.0; its result is ignored. INCOME is an
`EconomyIncome` (24 bytes: 4, player *t*, unit 0, the same `resources` and amounts). The network send is
`network.SendShareMetal(f, t, a)` for resource 0 and `network.SendShareEnergy(f, t, a)` for resource 1 (Q1).
The module writes no store: the spend and the income are economy's.

**The rank pass**: for each place *i* from 0 while *i* < `numPlayers`: when no player 0 to 9 counted for rank
has `rank` *i*, every player counted for rank whose `rank` is above *i* gets `rank` one less. The pass goes on to
*i* + 1 either way (Q2). No import.

### `UpdatePlayerTail(player)`

*p* is the low byte of `player`. When *p* is `Game.playerIndex`: `mission.GetGameType`; when it gives 3,
`g_economyPacketTick` adds 1 (32 bits, wrapping), and when the new value's low two bits are 0,
`network.BroadcastPlayerEconomy` with the address of player *p*'s record. Otherwise nothing.

### `UpdatePlayersFinish()`

When `g_quirks.rankCompactionPerTick` is not 0, the rank pass runs first. Then `mission.GetGameType`; when it
gives 3, `Game.commanderDeath` is not 2 and there is no combat player, advance the countdown with 0.

### `DeactivateConditions(conditions)`

The `active` of the given object becomes 0; a null object changes nothing. No import.

### `UpdatePlayers()`, `UpdatePlayersOnStream(stream)`

The players pass of `docs/schedule.md` (the passes `players` and `players.los`), with *stream* (the main
stream for `UpdatePlayers`) given to each body: for each player 0 to 9, the head, and when it gives 1 the other
subsystems' steps of the turn, the body, economy's settlement when the body gives 2, the viewed player's radar
steps, and the tail when the body gives 1 or 2; then the finish. The steps of this module are the slots above.
The other subsystems' steps need imports the entry doesn't list (K5); no recording holds either slot.

## 5. Exactness

- **Random draws:** none in any recorded call. Only the respawn draws, from `services.RandomInt` (the
  generator of `rng.seed`, not a game logic stream), two draws per try, x first; the ranges are
  `baseX − 2·(baseX / 10)` and `baseY − 2·(baseY / 10)`, 32-bit signed, the division truncating toward 0.
  `RandomInt` of a range below 2 gives 0.
- **The respawn's arithmetic:** (*a* + *cx*)·65536 and (*b* + *cz*)·65536 are 32-bit, wrapping. The footprint
  cells start at *P*.z − *Hh* and *P*.x − *Hw*, computed as unsigned 32-bit differences, add *Hh* or *Hw* per
  step (wrapping), and are shifted right by 20 as unsigned numbers; the cell's `x` and `y` keep the low 16 bits.
  max(·, 200) is a signed 32-bit comparison of *H*.energy·100 (a 16-bit value times 100, an int). The storage
  bonuses are the exact float of that int. The incomes are float(*H*.energy) × 100.0 in 32-bit float, rounded
  to nearest even.
- **The countdown** is signed 16-bit: 0 less one is −1, stored as 0xffff.
- **Clocks and counters:** the update clock adds 30, `g_economyPacketTick` adds 1, both 32-bit wrapping. The
  clock's test is unsigned: clock > `Game.ticks`.
- **Transfers:** the clamp is the 32-bit float comparison *a* > store (false for a NaN on either side, so a NaN
  amount is kept); the zero test is *a* = 0.0 in float, true for +0.0 and −0.0 and false for a NaN. The amount
  passed on is the clamped float, bit for bit.
- **SET_STORE:** *S* + `amount` is one 32-bit float addition, rounded to nearest even.
- **Integer comparisons:** `player`, `from`, `to` and the cheat bits are tested as unsigned 32-bit numbers.
  `unitCount` and `numPlayers` are unsigned 16 bits; `rank` is an unsigned byte compared with the place *i*.
  `numLeftToKill`, `numUnits` and `count` are signed 32-bit; `numLeftToKill` of `DefeatUnitTypeKilled` can go
  below 0.
- **Ids:** `units.FindUnitTypeId` returns an unsigned 16-bit value (ABI 7: zero-extended); `id` stores its low 16
  bits, and a unit's `unitDefIndex` is compared with `id` as 16-bit values.
- **`buildLeft`** is compared with 0.0 as a float: −0.0 counts as 0, a NaN does not.

## 6. Quirks

- **Q1.** `network.SendShareMetal` announces a transfer of energy and `network.SendShareEnergy` one of metal,
  as `interfaces/network.h` names its resource kinds.
- **Q2.** The rank pass looks at each place once, so a run of gaps or a rank past `numPlayers` takes more than
  one pass to close; one `PLAYERS_COMPACT_RANKS` is one pass.
- **Q3.** `DefeatUnitTypeKilled` counts the deaths of matching units of any player, and its `numLeftToKill`
  keeps going down past 0.
- **Q4.** `DefeatAllUnitsKilledOfType` scans player 1's slice after player 0's even when the count is already
  above 1, so that scan visits one unit and may add one more to `count`.
- **Q5.** The respawn's footprint test offsets the position by the whole map's `height` and `width` in
  position units, not by a cell: of the nine cells most lie off the map.
- **Q6.** `VictoryDestroyAllUnits` is met while player 1 has no units, whatever the other players hold, and it
  never sets its `satisfied`.

## 7. Examples

Each is taken from the kit's recordings and recomputed from this spec's definitions.

1. `tests/replay/players/console-economy.txt`, call at tick 10 of `Apply`: `PLAYERS_SET_STORE`, player 0,
   resource 0, `add` 1, `amount` f32:0x447a0000 (1000.0), size 24; `Player#0.energy` is f32:0x447a0000.
   *S* + `amount` = 2000.0: `w Player#0.energy f32:0x44fa0000`, result 1, no import.
2. The same file, tick 40, `PLAYERS_TRANSFER` from 0 to 1, resource 1 (metal), `amount` f32:0x42c80000
   (100.0), announced. `Player#0.metal` is f32:0x424c0000 (51.0), so *a* becomes 51.0: `economy.Spend` (the line
   `i services.ScheduleEconomySpend = 1`), `Emit` INCOME of 24 bytes (`i services.SchedPublish 4 24`), then
   `i network.SendShareEnergy 0 1 f32:0x424c0000`; result 1. At tick 50, resource 0 with `Player#0.energy`
   1000.0 keeps 100.0 and sends `network.SendShareMetal 0 1 f32:0x42c80000`.
3. `tests/replay/players/mission-defeat-all-units.txt`, tick 360, `UpdatePlayerBody(0)`: game type 1, clock 360
   (not above the tick), countdown 0, `active` 1. The clock becomes 390 (`4:86`, 0x186). *V*: one
   `GetGameType`, `VictoryDestroyAllUnits` not met (`Game.field_1df2` is not 0). *D*: one `GetGameType`;
   `DefeatAllUnitsKilled` finds no unit of player 0 that can be counted, so it is met. The countdown goes from 0
   to −1 (`0:ffff`) and the match ends with 0: `endGameFlags` 0 → 68 (bits 2 and 6). Three `GetGameType` in all;
   result 1, since bit 2 is now set.
4. `tests/replay/players/allied-victory.txt`, tick 330, `UpdatePlayerBody(0)`, game type 3, countdown 1: *D* is
   0 (player 0's `unitCount` is not 0), *V* (the allied victory) is 1, so the countdown becomes 0 (`0:00`); clock
   330 → 360 (`4:68`, 0x168); result 1. `UpdatePlayerTail(0)` then: `GetGameType` 3, `g_economyPacketTick` 11 →
   12, whose low two bits are 0, so `network.BroadcastPlayerEconomy Player#0`. At tick 360 the countdown goes
   from 0 to −1 and the match ends with 1: `endGameFlags` 36 (bits 2 and 5), `endGameVictory` 1.
5. `tests/replay/players/mission-capture.txt`, tick 499, `Apply` of `PLAYERS_GIVE_UNIT`, `Unit#282` to player
   0, size 48. *o* is `Player#1` (active, `type` 2) and *r* `Player#0` (active, `type` 1); the unit is alive
   (`flags` 0x30280321, bit 28 set, bit 14 clear). The captured notice: `VictoryCaptureUnitType#1` (`name`
   "CORGEO") matches `UnitDef#123` "CORGEO" and `playerIndex` 1, so `satisfied` and `celebrated` become 1 with
   `i services.RecordSimEvent 2 0 [0,0] 2`; then `Emit` CAPTURE_UNIT, 48 bytes (`i services.SchedPublish 4
   48`); result 1. The `UNIT_DIED` of `Unit#282` in the same tick runs `DefeatAllUnitsKilledOfType#1`:
   `units.FindUnitTypeId "CORGEO" = 123`, `id` 123, and the scans give `count` 2.
6. `tests/replay/players/skirmish2.txt`, tick 29 and 30, `UpdatePlayerHead`: players 0 and 1 (`active` 1,
   `type` 2, `index` 0 and 1) give 1, players 2 to 9 (`active` 0) give 0. At tick 30 player 0's body (clock 30)
   gives 2: it is still in, `type` 2, `endGameFlags` bit 2 clear, countdown −1.
7. `tests/replay/players/mission-defeat-passes-x.txt`, tick 0, `UpdatePlayerBody(0)`: the
   `VictoryMoveUnitToRadius#1` centre (8000, 4700) has the marker y, so
   `los_radar.ClampWorldPosToTerrain 8000 4700` writes (524288000, 5505024, 310771712) to `pos`; one
   `map.UnitsInRange` with skip 0 and capacity 256 answers 14, under 256, so it is the only one; none of the 14
   units meets the condition.

Not covered by any recording: `UpdatePlayers`, `UpdatePlayersOnStream`, `DeactivateConditions` and `Shutdown`;
the respawn and watch mode; `PLAYERS_SET_CHEATS`, `PLAYERS_LEAVE_GAME`, `PLAYERS_COMPACT_RANKS` and every
refusal of `Apply`; SEND_UNIT and the refusals of the hand-over; an unannounced transfer and the zero amount;
the classes `VictoryKillEnemyCommander`, `VictoryKillAllMobileUnits`, `VictoryBuildUnitType`,
`VictoryUnitTypePassesX`, `VictoryUnitTypePassesZ`, `VictoryTimerRunsOut`, `DefeatUnitTypeKilled`,
`DefeatDeathTimerRunsOut` and `DefeatAnyUnitPassesZ`, and a radius condition met; an empty condition list; a
second `map.UnitsInRange` call; the rank pass of `UpdatePlayersFinish`.

## 8. Kit version

`interfaces-v68` and `reference-traces-v55` (commit 906e69ae4), with players at `interfaces/players.h` version
5.4.
