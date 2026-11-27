# mission

## 1. Role

The mission is the match's description as the menus and the load of a match see it: the game type
(campaign, skirmish or multiplayer), the campaign file and which of its missions is current, the files
the mission names (the terrain, the briefing, the narration, the AI profile and others), the map's
figures that the simulation reads (wind limits, gravity, tides, sea damage, score multipliers, start
resources, surface metal), the start positions, and the units and features the mission places at the
start of a match. The engine holds one mission record. The slots that load or replace it run outside
the tick, from the menus, the load of a match and the saved game; the queries read it, some of them
inside the tick, and nothing in the tick writes it.

## 2. Interface

### Slots (`interfaces/mission.h`, version 2.0)

| Slot | When the engine calls it |
| --- | --- |
| `GetGameType` | whenever a caller needs the game type, inside the tick and out |
| `GetNameSlot` | when a caller needs one of the mission's file names (an AI's load, the use-only list, the terrain) |
| `SetMissionType` | from the menus, the lobby and the test harness, before a mission is loaded |
| `FreeOtaEnumCacheAndMission` | when the game shuts down |
| `AdvanceMission` | when a campaign mission is won and the game is saved |
| `GetCampaignName`, `GetMissionIndex`, `GetMissionName`, `HasMissionName` | from the menus, the saved game and the harness |
| `GetStartPosition`, `CountStartPositions` | at the load of a match, and for the camera |
| `LoadCampaign`, `LoadMissionByName`, `SelectMission`, `MissionExists`, `RefreshMapList` | from the menus and the harness, to pick the mission |
| `ComputeMapChecksum`, `GetTerrainLength`, `GetTerrainSizeTier` | the lobby's map check |
| `CountMissions`, `GetCampaignMissionName` | the campaign menus' list of missions |
| `GetBriefing`, `GetDescription`, `GetPlanet`, `GetTranslatedName`, `GetMemory`, `GetNumPlayers` | the menus' map and mission screens |
| `SelectSchema` | the front end's list of maps, with a map file it has parsed |
| `GetMissionParams` | whenever a subsystem needs the map's figures, inside the tick and out |
| `CountMissionUnits`, `GetMissionUnit`, `SetMissionUnitPosition` | at the load of a match, to create the mission's units; units' lookup of a mission unit |
| `CountMissionFeatures`, `GetMissionFeature` | at the load of a match, to place the mission's features |

`docs/schedule.md` (kit) gives the order of the load of a match; the mission is loaded before it starts.

### The engine's slots

For this milestone, the slots the kit's replay schema marks load class are the engine's: the engine's native code
runs them, and the module reads what they build in the block (ozgb/btm#43, which moves them into the modules
later). For mission these are `SetMissionType`, `FreeOtaEnumCacheAndMission`, `AdvanceMission`, `LoadCampaign`,
`LoadMissionByName`, `RefreshMapList`, `SelectMission`, `SelectSchema` and `SetMissionUnitPosition`. The module
doesn't export them. Section 4 still states what each leaves in the record, since that is the block data the
module's slots read, and the scenario traces judge it (section 3).

### The manifest entry

The kit's manifest (`wasm/modules.json`) has no entry and no `signatures.mission` for this module, only its
interface major, 2 (`majors.mission`). This section gives the entry for the module's 25 slots. It is exact for 24
of them. `GetCampaignMissionName` needs one value no import of the kit gives ("Gaps of the kit", below). The kit's
replay schema lists no imports for mission (`subsystems.mission.header.imports`) because no recorded call makes
one; the imports below are those of the unrecorded queries.

- `name` and `interface`: `mission`.
- `exports`, in the header's slot order: `GetGameType`, `GetNameSlot`, `GetCampaignName`, `GetMissionIndex`,
  `GetMissionName`, `GetStartPosition`, `MissionExists`, `ComputeMapChecksum`, `GetTerrainLength`,
  `CountMissions`, `GetCampaignMissionName`, `CountStartPositions`, `GetBriefing`, `GetDescription`, `GetPlanet`,
  `GetTerrainSizeTier`, `GetTranslatedName`, `HasMissionName`, `GetMissionParams`, `CountMissionUnits`,
  `GetMissionUnit`, `CountMissionFeatures`, `GetMissionFeature`, `GetMemory`, `GetNumPlayers`.
- `data`: `g_game`, `g_simGlobals`, `g_extTdf`, `g_blockBase` (ABI 8.2). The module reads `g_game` and
  `g_blockBase` only.
- `majors`: `mission` 2, `files` 3 and `services` 4.
- `data_base`, `table_base`, `fuel_budget` and `size_budget`: the build picks them (ABI 2, 11.1 and 11.3). The
  window must not overlap another module's window, nor its table range another module's range.
- Slot kinds. A text a slot returns points into the record, so it is a `block` result: the record's address
  plus the field's offset, or 0 for "none".

| Slot | Kinds |
| --- | --- |
| `GetGameType`, `GetMissionIndex`, `ComputeMapChecksum`, `GetTerrainLength`, `CountMissions`, `CountStartPositions`, `GetTerrainSizeTier`, `HasMissionName`, `CountMissionUnits`, `CountMissionFeatures` | `[] -> int` |
| `GetNameSlot` | `[int] -> block` |
| `GetCampaignName`, `GetMissionName`, `GetBriefing`, `GetDescription`, `GetPlanet`, `GetTranslatedName`, `GetMemory`, `GetNumPlayers` | `[] -> block` |
| `MissionExists` | `[int] -> int` |
| `GetStartPosition` | `[linear, int] -> int` (the `Vec3` out, the id) |
| `GetCampaignMissionName` | `[int, linear, int] -> int` |
| `GetMissionParams` | `[linear] -> void` |
| `GetMissionUnit`, `GetMissionFeature` | `[int, linear] -> int` |

`HasMissionName` returns 0 or 1.

- Imports, every one listed and nothing else. The slots not named in the table make none.

| Import | Kinds | Used for |
| --- | --- | --- |
| `services.TdfFile_Construct`, `services.TdfFile_Destruct` | `[linear] -> void` | the module's own parsed copy of the campaign file (`MissionExists`, `CountMissions`, `GetCampaignMissionName`) |
| `services.TdfFile_LoadFile` | `[linear, linear] -> int` | loading it from `names[0]` |
| `services.TdfFile_SelectRecord` | `[linear, linear] -> int` | its mission records |
| `services.TdfRecord_GetFieldString` | `[handle, linear, linear, int, linear] -> int` | the localised `missionname` (`GetCampaignMissionName`) |
| `files.OpenRead` | `[linear] -> handle` | the terrain file (`ComputeMapChecksum`); 0 when it doesn't open |
| `files.Read` | `[handle, linear, int] -> int` | its header, plot and features |
| `files.Seek` | `[handle, int] -> int` | the offsets of the plot and the features |
| `files.Close` | `[handle] -> int` | closing it |

The recordings hold no call of these slots, so their import order isn't compared; section 4 gives what each reads.

- Fuel: the module meters itself by rule version 1 (ABI section 11) and exports `__fuel_set` and `__fuel_get`.
- Hooks: every slot of `interfaces/mission.h` says "Hooks: none", so the module raises none.

### What the engine's slots need

The engine's slots reach these engine calls. All but item 2 are calls that no import of the kit's
`interfaces/services.h` (version 4.0) or of an interface offers. ozgb/btm#43 adds them when it moves the slots into the module. The list is for that
issue: the module of this spec exports none of the engine's slots, and imports only what "The manifest entry"
lists.

1. **Allocation in the block:** the record itself, the units and their strings, the start position rules, the
   features and the briefing's text; and their release, with the release of the conditions object players made. The block's allocations are zeroed.
2. **Files by name:** a file's length (0 for a missing file), whether it opens for reading, and reading its
   bytes at an offset. `interfaces/files.h` offers these (`LengthByName`, `OpenRead`, `ReadAt`).
3. **The parser's records beyond the imports:** unloading a file, resetting its current record to none,
   counting and indexing the sub-records of a record, finding a sub-record by name, setting the current record
   to a record found earlier, the checksum the parser keeps for a record's text, and a parsed file kept between
   calls (the record's `list`, while ABI 8.1 makes handles last one outermost call).
4. **The language:** the player's preferred language (none, or a name such as `english`), the language prefix the
   translation table uses for localised keys, the translation of a key, and the key whose translation equals a
   text.
5. **A message box** with the caption `Error`, shown on top of the game window.
6. **The front end's list of maps** (built for skirmish or for multiplayer) and the release of its map cache.
7. **Publishing to wind_meteors from the engine's outbox:** `WIND_METEORS_ENABLE`, `WIND_METEORS_DISABLE` and
   `WIND_METEORS_PARAMS`. `services.Emit` publishes to the module's own outbox only (ABI 9.3), and
   `interfaces/mission.h` has no effects.
8. **The skirmish options:** `Game.options` points at a table the kit's `layout.json` doesn't describe; the load
   reads each player slot's controller from it (section 4, `SelectSchema`).

### Gaps of the kit

Each mission record a campaign query selects takes one handle, and so does the campaign file's root.
`CountMissions` and `MissionExists` walk every mission record, so on a campaign with more than 31 of them they go
past ABI 8.1's 32 handles in one call, a fatal error the engine's own slots don't have. `GetCampaignMissionName` takes 2 at most. The kit offers no way to release a handle within a call.

`GetCampaignMissionName` reads the localised key `missionname`: first the key made of the translation table's
language prefix followed by `missionname`, then `missionname` itself (gap 4 of the list above). No import gives the prefix
(ozgb/byte-tactics-moddable#1206). The prefix is the name of the language the game's translations were loaded
for, such as `english`.

### Arguments outside the block

- **`MissionParams`** (`GetMissionParams`): the struct of `interfaces/mission.h`, 124 bytes: eleven 32-bit fields
  (`surfaceMetal`, `minWindSpeed`, `maxWindSpeed`, `gravity` as int, `tidalStrength` as float, `lavaWorld`,
  `noSeaLevelTrigger`, `waterDoesDamage`, `waterDamage` as int, `killMul`, `timeMul` as float), then
  `startMetal[10]` and `startEnergy[10]` (floats). The caller owns it.
- **`MissionUnitInfo`** (`GetMissionUnit`): 36 bytes, the same layout as the record `MissionUnit` of `layout.json`,
  but `name`, `ident` and `initialMission` are linear addresses (0 for none), not block offsets.
- **`MissionFeatureInfo`** (`GetMissionFeature`): 12 bytes: `name` (a linear address), `x`, `y` (int).
- **`Vec3`** (`GetStartPosition`, `SetMissionUnitPosition`): `layout.json` record `Vec3`, three ints.
- **Texts** given to a slot (`LoadCampaign`, `LoadMissionByName`) are NUL-terminated strings in shared memory.
- **The parsed file of `SelectSchema`** is the caller's `TdfFile` (ABI 8.1). The slot moves its current record. Its
  `schema` argument is a buffer the slot writes the chosen schema's record name to, or 0; the header's "by name
  when schema is given" means the weighing of section 4 runs only when it is given, for types 2 and 3.

### Events

None. No slot of this module records a sim event (kinds are listed in `docs/sim-events.md`).

## 3. State

### Owned

- **`Game.net`**, a `Ref<Mission>`: the record, or null before the engine first makes one (`SetMissionType`) and
  after `FreeOtaEnumCacheAndMission`. The engine makes a record before it calls any of the module's slots, and
  calls them only while there is one, so the module's slots never see a null `Game.net`. "The record" below is the
  `Mission` it points at.
- **`Mission`** (`layout.json`, 3780 bytes). Its fields, with what the slots of section 4 make of them:

| Field | Type | Meaning |
| --- | --- | --- |
| `type` | int | the game type: 0 none, 1 campaign, 2 skirmish, 3 multiplayer |
| `campaign` | char[256] | the campaign file's name as given, or empty |
| `names` | char[9][256] | the file names by slot (below), each empty for none |
| `exists` | int | the length in bytes of the file `names[1]` names, 0 for none or a missing file |
| `list` | `TdfFile` | the campaign file, parsed (a gap, section 2) |
| `missionName` | char[256] | the mission's name |
| `text_b14` | char[256] | the map's name in the player's language |
| `briefing` | `Ref<char>` | the briefing's text, NUL-terminated, or null |
| `missionIndex` | int | the index of the campaign's current mission |
| `tntChecksum` | int | the terrain file's checksum, 0 while not computed |
| `headerChecksum` | int | the checksum the parser keeps for the map file's `GlobalHeader` record |
| `description`, `planet` | char[128] | the map's description and planet |
| `mapList`, `mapCount` | `char *`, int | the front end's list of maps and its length (`mapList` is a raw pointer, not a `Ref`) |
| `multi` | int | the list of maps is for multiplayer |
| `surfaceMetal`, `minWindSpeed`, `maxWindSpeed`, `gravity` | int | the map's figures; −1 for "the terrain file's" |
| `tidalStrength` | float | the tidal strength; −1.0 for "the terrain file's" |
| `lavaWorld`, `noSeaLevelTrigger`, `waterDoesDamage`, `waterDamage` | int | the sea's behaviour |
| `killMul`, `timeMul` | float | the score multipliers |
| `startMetal`, `startEnergy` | float[10] | the start resources by player |
| `units`, `unitCount` | `Ref<MissionUnit>`, int | the mission's units |
| `rules`, `ruleCount` | `Ref<MissionRule>`, int | the start position rules |
| `features`, `featureCount` | `Ref<MissionFeature>`, int | the mission's features |
| `memory`, `numPlayers` | char[128] | the map's `memory` and `numplayers` texts |

The file name slots: 0 the campaign file, 1 the terrain file, 2 the briefing, 3 the narration, 4 the mission
hint, 5 the glamour picture, 6 the use-only unit list, 7 the AI profile, 8 the glamour sound.

- **`MissionRule`** (12 bytes): `type` (1 a start position, 0 anything else), `id` (the player the start position
  is for, from 0), `x`, `z` (short, in world units, as the map file gives them).
- **`MissionUnit`** (36 bytes): `name`, `ident`, `initialMission` (`Ref<char>`, each to a NUL-terminated string in
  the block, or null), `x`, `y`, `z` (16.16 fixed point), `angle`, `health` (short), `creationCountdown` (int),
  `buildPriority` (short), `player` (unsigned char), `flags` (unsigned char: bits 0 to 3 `initialGroup`, bit 4
  `missionCritical`, bit 5 `aiIgnore`, bit 6 `aiPriorityTarget`, bit 7 `immunity`).
- **`MissionFeature`** (136 bytes): `name` (char[128]), `x`, `y` (int, in map cells).
- **`Game`**, written by a load: `maxUnitsSetting` (unsigned short), `singleMapping`, `singleLineOfSight`,
  `singleLOSType`, `singleCommanderDeath`, `skip` (int), and `list_391ed` (`Ref<MissionConditions>`, the object
  players' conditions slots work on).

### Read

`Game.numPlayers` (unsigned short), `Game.difficulty` (int), `Game.table_2c28` (int[10]) and `Game.options`
(section 2, gap 8), by `SelectSchema` and the load.

### Between ticks

Nothing outside the record, the tables and texts it points at, and the `Game` fields above. The trace hashes the
record at tick 1 (`docs/trace-format.md`, section 8.20, and the tables `MissionData` and `MissionStartRest` of
`data/trace_schema.json`), so these must hold what section 4 says a load leaves:

- `type`, `multi`, `surfaceMetal`, `gravity`, `tidalStrength`, `lavaWorld`, `noSeaLevelTrigger`,
  `waterDoesDamage`, `waterDamage`, `killMul` and `timeMul`;
- `startMetal` and `startEnergy` of players 0 and 1, and those of players 2 to 9 when `type` is 1;
- `unitCount`, `ruleCount` and `featureCount`, and every unit, rule and feature, with their strings.

The trace schema is the authority on the rest, which isn't hashed (`minWindSpeed` and `maxWindSpeed` among it: the
trace hashes their copies in `Game`). A query slot changes nothing, but `ComputeMapChecksum` sets `tntChecksum`.

## 4. Behaviour

Within one call, only the order of import calls is observable. The block writes are compared at the end of
the call, so the order they are listed in carries no meaning.

The subsections below on the engine's slots (listed in section 2, "The engine's slots") state what the engine's native code leaves in the record and in
`Game`, for the module's slots to read.

Every query below reads the record `Game.net` points at; the engine calls them only while there is one. "The
address of a field" is the record's address (`g_blockBase` plus `Game.net`) plus the field's offset in
`layout.json`. A `Ref` given out as an address is `g_blockBase` plus its value, and 0 when it is null.

### The queries

| Slot | Result |
| --- | --- |
| `GetGameType()` | `type` |
| `GetNameSlot(index)` | the address of `names[index]` when its first byte isn't 0, else 0. Callers pass 0 to 8 only; for another index the result is never judged, and 0 will do (the slot must not trap) |
| `GetCampaignName()` | the address of `campaign` when its first byte isn't 0, else 0 |
| `GetMissionIndex()` | `missionIndex` |
| `GetMissionName()` | the address of `missionName`, even when it is empty |
| `HasMissionName()` | 1 when the first byte of `missionName` isn't 0, else 0 |
| `GetTranslatedName()` | the address of `text_b14` |
| `GetDescription()`, `GetPlanet()`, `GetMemory()`, `GetNumPlayers()` | the address of `description`, `planet`, `memory`, `numPlayers` |
| `GetBriefing()` | `briefing` as an address (0 when null) |
| `GetTerrainLength()` | `exists` |
| `GetTerrainSizeTier()` | from `exists`, compared as signed: below 4089446 (0x3e6666) 16; below 0x600000 24; below 0x800000 32; below 0xa00000 48; below 0xc00000 64; otherwise 128 |
| `CountStartPositions()` | how many of the `ruleCount` rules have `type` 1 |
| `CountMissionUnits()` | `unitCount` |
| `CountMissionFeatures()` | `featureCount` |

### `GetStartPosition(out, id)`

The rule is the first of the `ruleCount` rules, in table order, whose `type` is 1 and whose `id` equals `id`.

| Outcome | `out` | Result |
| --- | --- | --- |
| A rule is found | `out.x` = the rule's `x`, sign-extended, × 65536; `out.y` = 0; `out.z` = the rule's `z`, sign-extended, × 65536 | 1 |
| None | unchanged | 0 |

### `GetMissionParams(out)`

`out` becomes the record's figures, field for field: `surfaceMetal`, `minWindSpeed`, `maxWindSpeed`, `gravity`,
`tidalStrength`, `lavaWorld`, `noSeaLevelTrigger`, `waterDoesDamage`, `waterDamage`, `killMul`, `timeMul`, and the
ten `startMetal` and ten `startEnergy` values. Floats are copied as their 32 bits.

### `GetMissionUnit(index, out)`

| Outcome | `out` | Result |
| --- | --- | --- |
| `index` is below 0 or not below `unitCount` | unchanged | 0 |
| Otherwise, with *u* the unit `units[index]` | `name`, `ident`, `initialMission` = *u*'s as addresses (0 for null); `x`, `y`, `z`, `angle`, `health`, `creationCountdown`, `buildPriority`, `player`, `flags` = *u*'s | 1 |

### `GetMissionFeature(index, out)`

| Outcome | `out` | Result |
| --- | --- | --- |
| `index` is below 0 or not below `featureCount` | unchanged | 0 |
| Otherwise, with *f* the feature `features[index]` | `name` = the address of *f*'s `name` (never 0, possibly empty); `x`, `y` = *f*'s | 1 |

### `SetMissionUnitPosition(index, pos)`

When 0 ≤ `index` < `unitCount`, the unit `units[index]` gets `x`, `y`, `z` = `pos.x`, `pos.y`, `pos.z`. Otherwise
nothing changes.

### `SetMissionType(type)`

- When `Game.net` isn't null and the record's `type` equals `type`: nothing changes.
- Otherwise the record, if there is one, is freed ("Freeing the record", below), and `Game.net` becomes a new
  record, zeroed, with `type` = `type`. Every text of the new record is empty, every count 0, every `Ref` null,
  `mapList` 0, and `list` holds no file. The new record holds no campaign (as `LoadCampaign` with an empty name
  leaves it).

**Freeing the record:** when `Game.list_391ed` isn't null, `players.FreeConditions(Game.list_391ed)` is called,
the object's memory is released, and `Game.list_391ed` becomes null. The record's units, rules, features,
briefing and map list are released, and then the record.

### `FreeOtaEnumCacheAndMission()`

The front end's map cache is released (section 2, gap 6), the record is freed as above, and `Game.net` becomes
null.

### File names

"Slot *i* is named from (*dir*, *name*, *ext*)" means:

- When *name* is empty, `names[i]` becomes empty.
- Otherwise the path is *dir*, a backslash, *name*, with everything from the last `.` of that text on removed,
  then `.` and *ext*. When the player has a preferred language *L*, the same is first tried with *dir* followed
  by `-` and *L*: if that file opens for reading, it is the path. `names[i]` becomes the path.
- Whenever `names[1]` changes, `exists` becomes the length of the file it names, or 0 when it is empty.

An empty *dir* gives a path that starts with the backslash. Paths are at most 255 characters; a longer one is
undefined.

### `LoadCampaign(file)`

- `list` is unloaded, `campaign` becomes a copy of `file`, and all nine `names` become empty (so `exists`
  becomes 0).
- When `file` isn't empty: slot 0 is named from (`camps`, `campaign`, `TDF`), and `list` loads the file
  `names[0]` names.
  - When it doesn't load: a message box shows `The requested campaign file, <names[0]>, does not exist.`, and
    the record ends as `LoadCampaign` with an empty name leaves it.
  - When it loads: `tntChecksum` and `missionIndex` become 0, and the mission is loaded ("Loading the mission").

A record of a type other than 1 is given a non-empty campaign by no caller.

### Campaign records

"Mission record *n*" is the top-level record named `MISSION`*n* (*n* in decimal, from 0) of the campaign file
`list`. "The campaign's count" is 0 when `campaign` is empty, and otherwise the number of mission records 0, 1,
2, ... that exist before the first missing one. A lookup of a mission record by the engine's slots leaves it as
`list`'s current record; nothing else reads that.

The module's `MissionExists`, `CountMissions` and `GetCampaignMissionName` don't use `list`: its record words are
handles from an earlier call, which ABI 8.1 makes invalid. Each call parses the campaign file again, in a
`TdfFile` *F* of its own. Its imports, in order:

- When `campaign` is empty (and, for `GetCampaignMissionName`, when `size` is below 1): none.
- Otherwise `services.TdfFile_Construct(F)`, then `services.TdfFile_LoadFile(F, names[0])` (`names[0]` is the file
  `list` was loaded from). When that gives 0, the file has no mission records: `services.TdfFile_Destruct(F)`
  follows at once.
- Each lookup of mission record *n* is `services.TdfFile_SelectRecord(F, "MISSION<n>")`, with *n* in decimal (a
  minus sign before a negative one), made with *F*'s current record word (byte offset 4) set to 0 first, the null
  handle, so that the lookup is among the top-level records (ABI 9.3).
- `CountMissions` and `MissionExists` look up records 0, 1, 2, ... until a lookup gives 0, whatever `index` is.
  `GetCampaignMissionName` looks up record `index` only, then reads its name (below).
- `services.TdfFile_Destruct(F)` last.

The results are those of the table below; `list` is left as it was.

| Slot | Result | Writes |
| --- | --- | --- |
| `CountMissions()` | the campaign's count | none |
| `MissionExists(index)` | 1 when the campaign's count is above `index` (signed), else 0 | none |
| `AdvanceMission()` | 1 when the campaign's count is above `missionIndex` + 1, else 0 | when the result is 1: `tntChecksum` = 0, `missionIndex` grows by 1, and the mission is loaded (its result is ignored) |
| `SelectMission(index)` | the load's result | `tntChecksum` = 0, `missionIndex` = `index`, and the mission is loaded |

### `GetCampaignMissionName(index, out, size)`

| Outcome | `out` | Result |
| --- | --- | --- |
| `size` is below 1, or `campaign` is empty | unchanged | 0 |
| Mission record `index` is missing | unchanged | 0 |
| It has the localised key `missionname` | what `services.TdfRecord_GetFieldString` writes for that key with `size` (the key's text, cut to `size` − 1 bytes, padded with zero bytes up to `out[size − 1]`, which is 0: all `size` bytes are written) | 1 |
| It hasn't | the first `size` − 1 bytes of `Error -- Unnamed Mission`, padded with zero bytes to `size` − 1, then a 0 at `out[size − 1]`: all `size` bytes are written | 1 |

**A localised key** *k* is read as the key made of the translation table's language prefix followed by *k*, and,
when the record hasn't that key, as *k* itself (section 2, gap 4). The default text is empty.

The module's `GetCampaignMissionName` reads it, after the lookup of mission record `index` gives 1, with
`services.TdfRecord_GetFieldString(current, out, key, size, "")`: *current* is *F*'s current record word, and the key
is the prefixed one, then, when that gives 0, `missionname`. Either call may write `out`. When both give 0, the
fallback row's bytes are written over it.

### `LoadMissionByName(name)`

`tntChecksum` becomes 0 in every outcome.

| `type` | Effect | Result |
| --- | --- | --- |
| 1 | The campaign's mission names are the localised `missionname` of mission records 0 to count − 1, each read into 256 bytes with the fallback text of `GetCampaignMissionName`'s last outcome. For the first index *i* whose name equals `name`, ignoring case: `missionIndex` = *i*, and the mission is loaded. No such index: nothing else changes | the load's result, or 0 |
| 2, 3 | The mission is loaded with `name` as the map. Then `text_b14` is set (below) | the load's result |
| other | nothing else | 0 |

**`text_b14`**, for types 2 and 3, whatever the load gave (Q7): when the load gave 1 and the player has a preferred
language that isn't `english` (ignoring case), bytes 0 to 254 of `text_b14` become the translation of `name` in
lower case, cut to 255 bytes and padded with zero bytes to 255 (byte 255 keeps its value); and then `text_b14`
becomes `name` itself when that text equals `name` ignoring case. Otherwise `text_b14` becomes `name`. A key with no
translation is its own translation. In the first case, a `name` of 200 bytes or more is undefined.

### `RefreshMapList(multiplayer)`

- When `multi` isn't 0, `multiplayer` is 0 and `mapList` isn't 0: the list is released and `mapList` becomes 0.
- When `mapList` is 0 (then): the front end builds its list of maps for multiplayer when `multiplayer` isn't 0,
  for skirmish otherwise (section 2, gap 6); `mapList` points at it (its maps' names, each NUL-terminated, one
  after the other) and `mapCount` becomes the number of maps.
- Then the record ends as `LoadMissionByName` with the list's first name leaves it (Q6); that result is ignored.
- `multi` becomes `multiplayer`.

### Loading the mission

The load takes the map's name for types 2 and 3 and nothing for type 1, and gives 1 when it succeeds and 0 when
it doesn't. Every load starts with these writes, whatever follows:

- `players.DeleteConditions(Game.list_391ed)`, then `players.NewConditions()`, whose result becomes
  `Game.list_391ed`.
- `surfaceMetal`, `minWindSpeed`, `maxWindSpeed`, `gravity` = −1; `tidalStrength` = −1.0; `lavaWorld`,
  `noSeaLevelTrigger` = 0; `planet` and `description` become empty; the units, rules, features and briefing are
  released, with `unitCount`, `ruleCount`, `featureCount` = 0 and the three `Ref`s and `briefing` null.

A load that gives 0 ends there, with what it wrote so far (Q8). "The map file" is a parsed file of the load's own,
in the module's memory.

**Finding the map file**, by type:

| `type` | Outcome | Writes, then | Result |
| --- | --- | --- | --- |
| 1 | mission record `missionIndex` is missing | message `The requested mission file, MISSION<n>, does not exist.` (*n* is `missionIndex`) | 0 |
| 1 | it has no `missionfile` key | `missionName` = its localised `missionname`; message `Old TED format no longer supported!` | 0 |
| 1 | the file `Maps\<missionfile>.OTA` (`services.BuildDataPath` with `Maps`, the key's text and `OTA`) doesn't load | `missionName` as above; message `Hey, joker!  There is no mission defintion for this mission: <missionfile>` | 0 |
| 1 | it loads but has no top-level `GlobalHeader` record | `missionName` as above; message `Hey, joker!  Mission file <missionfile> is corrupt (no header found).` | 0 |
| 1 | it loads with a `GlobalHeader` | `missionName` as above; `Game.maxUnitsSetting` = the header's `maxunits` (default 200), its low 16 bits; slot 1 is named from (`Maps`, the key's text, `TNT`) | continue |
| 2, 3 | `Maps\<name>.OTA` loads | `exists` = 0; `missionName` = `name`; slot 1 is named from (`Maps`, `name`, `TNT`) | continue |
| 2, 3 | it doesn't, and no translation's text equals `name` ignoring case | `exists` = 0; `missionName` = `name` | 0 |
| 2, 3 | it doesn't; the key *m* of the first translation (in the table's order) whose text equals `name` ignoring case; `Maps\<m>.OTA` doesn't load | `exists` = 0; `missionName` = *m* | 0 |
| 2, 3 | as above, and `Maps\<m>.OTA` loads | `exists` = 0; `missionName` = *m*; slot 1 is named from (`Maps`, *m*, `TNT`) | continue |
| 0, other | | | 0 |

The texts are read into 256 bytes. Each message is a message box (section 2, gap 5).

**The header.** When the map file has no top-level `GlobalHeader` record: message `No GlobalHeader block in mission
file!`, result 0. Otherwise, from that record ("the header"):

| Writes | From the header |
| --- | --- |
| `headerChecksum` | the checksum the parser keeps for the record (section 2, gap 3) |
| slot 2, then `briefing` | localised `brief` → slot 2 from (`camps\briefs`, text, `TXT`); then `briefing` becomes a copy of that file's bytes with a 0 after them, or null when the slot is empty or the file's length is 0 |
| slot 3 | localised `narration` → (`camps\briefs`, text, `WAV`) |
| slot 4 | localised `missionhint` → (`camps\hints`, text, `TXT`) |
| slot 5 | `glamour` → (empty, text, `PCX`) |
| slot 8 | `glamoursound` → (`camps\briefs`, text, `WAV`) |
| slot 6 | `UseOnlyUnits` → (`camps\useonly`, text, `TDF`) |
| `Game.singleMapping`, `Game.singleLineOfSight` | `mapping`, `lineofsight` (int, default 0) |
| `Game.singleLOSType`, `Game.singleCommanderDeath` | 1 and 0 |
| `memory`, `numPlayers`, `planet` | `memory`, `numplayers`, `Planet`, each read into 128 bytes, default empty |
| `Game.skip` | `nomovie` (int, default 0) |
| `description` | `missiondescription` read into 128 bytes, default `No description available`, as *d*. With *k* the lower-case copy of *d*: the translation of *k*; then *d* when that equals *k* ignoring case (Q1). A translation of 128 bytes or more is undefined |
| `minWindSpeed`, `maxWindSpeed`, `gravity`, `lavaWorld`, `noSeaLevelTrigger`, `waterDoesDamage`, `waterDamage` | `minwindspeed`, `maxwindspeed`, `gravity`, `lavaworld`, `nosealeveltrigger`, `waterdoesdamage`, `waterdamage` (int, default 0) |
| `tidalStrength`, `killMul`, `timeMul` | `tidalstrength`, `killmul`, `timemul` (double, default 0.0, rounded to float) |

The engine's slots aren't recorded, so the order of their parser imports isn't compared.

Text keys without "localised" are read with an empty default into 256 bytes, unless the table says otherwise.

`players.RegisterConditions(Game.list_391ed, map file)` is called with the header as the file's current record.

**The schema** is the one `SelectSchema` would choose for the record's `type`, the map file and a name buffer, and
"the schema's name" is the name that choice gives. When there is none: message `No suitable schema type in mission file!`, result 0. Otherwise, from the schema
record the choice leaves current ("the schema"):

| Writes | From the schema |
| --- | --- |
| `startMetal[0]`, `startEnergy[0]` | `HumanMetal`, `HumanEnergy` (int, default 0), converted to float |
| `startMetal[1]`, `startEnergy[1]` | `ComputerMetal`, `ComputerEnergy`, the same way |
| `surfaceMetal` | `SurfaceMetal` (int, default 0) |
| slot 7 | `aiprofile` → (`ai`, text, `txt`); when that leaves slot 7 empty, (`ai`, `default`, `txt`) |

`startMetal` and `startEnergy` 2 to 9 aren't written (Q4).

**The meteor shower.** A `MeteorShowerParams` (`interfaces/wind_meteors.h`) of the load's own: `weapon` = the
schema's `MeteorWeapon` read into 32 bytes, default empty.

| `weapon` | Then |
| --- | --- |
| not empty | `radius` = `MeteorRadius` (int, default 0); `density`, `duration`, `interval` = `MeteorDensity`, `MeteorDuration`, `MeteorInterval` (double, default 0.0, rounded to float). When `radius` is 0 or any of the three is 0.0: `wind_meteors.LoadMeteorDefaults(params)`. Then `WIND_METEORS_ENABLE` is published |
| empty | `WIND_METEORS_DISABLE` is published, then `wind_meteors.LoadMeteorDefaults(params)` (Q2) |

Then `WIND_METEORS_PARAMS` is published with the params. The three are published from the engine's outbox (section
2, gap 7), and wind_meteors receives each at once.

**The tables** come from the record named by the schema's name under the top-level `globalheader` record (the
names are matched as the parser matches them). When either is missing, the tables stay empty and the load still
gives 1. Otherwise, with *S* that record:

- **Units**: the sub-records of *S*'s sub-record `units`, in order (none when it is missing). `unitCount` is their
  number, and `units` a table of that many `MissionUnit`s in the block, each sub-record giving:
  - `name`, `ident`, `initialMission`: the keys `Unitname`, `Ident`, `InitialMission`, each read with an empty
    default into at most 1024 bytes. When the key exists (even with an empty text), the field points at a copy of
    its text, NUL-terminated, in the block; when it doesn't, the field is null. The copies stay valid until the
    next load.
  - `x`, `y`, `z` = `XPos`, `YPos`, `ZPos` (int, default 0) × 65536, wrapping at 32 bits.
  - `angle` = (`Angle` (default 0) × 65536, wrapping) / 360, truncated toward zero, its low 16 bits.
  - `player` = the low 8 bits of `Player` (default 0); when that is 0, 1.
  - `health` = `HealthPercentage` (default 100), `buildPriority` = `BuildPriority` (default 0), low 16 bits.
  - `creationCountdown` = `CreationCountdown` (default 0).
  - `flags`: bit 4 the low bit of `MissionCriticalUnit`, bit 5 of `AiIgnore`, bit 6 of `AiPriorityTarget`, bit 7
    of `Immunity`, bits 0 to 3 the low 4 bits of `InitialGroup` (each default 0).
- **Rules**: the sub-records of *S*'s `specials`, in order. `ruleCount` is their number, and `rules` a table of
  that many `MissionRule`s, zeroed. A sub-record whose `specialwhat` (read into 256 bytes) starts with `StartPos`,
  ignoring case, gives `type` 1, `x` = `XPos`, `z` = `ZPos` (int, default 0, their low 16 bits) and `id` (Q3):
  when the character after `StartPos` is a decimal digit, the decimal number that starts there; otherwise the
  count of such unnumbered start positions so far, this one included (1 for the first). Then, when that value is
  above 0, `id` is that value − 1. Any other sub-record leaves its rule zeroed.
- **Features**: the sub-records of *S*'s `features`, in order. `featureCount` is their number, and `features` a
  table of that many `MissionFeature`s, each giving `name` = `Featurename` (read into 128 bytes; empty when the
  key is missing), `x` = `XPos`, `y` = `ZPos` (int, default −1). When `x` or `y` is negative, `name` becomes
  empty (its first byte 0).

The load then gives 1.

### `SelectSchema(type, parser, schema)`

The *names* to look for, in order:

| `type` | Names |
| --- | --- |
| 1, with `Game.difficulty` 0 | `Easy`, `Medium`, `Hard` |
| 1, with `Game.difficulty` 1 | `Medium`, `Easy`, `Hard` |
| 1, with `Game.difficulty` 2 | `Hard`, `Medium`, `Easy` |
| 1, another difficulty; 0; another type | none: the result is 0, and nothing is read |
| 2, 3 | `Network 1`, `Network 2`, `Network 3`, `Network 4` |

*P*, the players of the match: `Game.numPlayers`, then for type 3 one more than the highest *i* (0 to 9) whose
`Game.table_2c28[i]` is neither −1 nor 0, and for type 2 one more than the highest *i* whose skirmish options'
player *i* has a controller that isn't 0 (section 2, gap 8). With no such *i*, *P* stays `Game.numPlayers`.

**Schema *n*** is the record named `Schema <n>` (*n* in decimal, from 0) under the top-level `GlobalHeader`. Each
lookup starts from the top of the file; when `GlobalHeader` is missing, `services.FatalError("Very bad news!  No
MSG!")` is called. For each name in order, the schemas 0, 1, 2, ... are taken until the first missing one. A
schema matches the name when its `type` key (read into 32 bytes) exists and equals the name, ignoring case.

| Case | Outcome |
| --- | --- |
| `type` is 1, or `schema` is 0 | the first matching schema (names in order, schemas in order): it is left as `parser`'s current record, its record name (`Schema <n>`) is copied into `schema` when `schema` isn't 0, and the result is 1. With none, the result is 0 |
| `type` is 2 or 3 and `schema` isn't 0 | every matching schema of every name is weighed in that order, with *best* none and *top* 0 at the start. A schema's count *c* is the number of sub-records of its `specials` sub-record whose `specialwhat` (read into 16 bytes) starts with `StartPos`, ignoring case (0 when it has none). A schema with *c* ≠ 0 and (*c* = *P*, or *P* = 0, or (*c* > *top* and *top* ≠ *P*)) becomes *best*, *top* = *c*, and its record name is copied into `schema`. At the end, when there is a *best*, it becomes `parser`'s current record; the result is 1 when there is one, else 0 |

A result of 0 leaves `parser`'s current record wherever the last lookup left it.

### `ComputeMapChecksum()`

| Outcome | Result | Writes |
| --- | --- | --- |
| `tntChecksum` isn't 0 | `headerChecksum` XOR `tntChecksum` | none |
| the file `names[1]` names doesn't open | 0 | none |
| its first 64 bytes (the header) don't start with the 32-bit value 0x2000 | 0 | none |
| otherwise | `headerChecksum` XOR `tntChecksum` | `tntChecksum` = K(header) XOR K(plot) XOR K(features) |

With the header's 32-bit fields *w* at byte 4, *h* at byte 8, *p* at byte 16, *n* at byte 28 and *o* at byte 32:
the plot is the *w* × *h* × 4 bytes at offset *p* of the file, and the features the *n* × 132 bytes at offset *o*
(none when that size isn't above 0). K is section 5's checksum. The file is opened with `files.OpenRead(names[1])`
(0 is "doesn't open"). The header is the first 64 bytes `files.Read` gives after the open; the plot and the
features are read with `files.Read` after `files.Seek` to their offsets. A read may be split into parts of any
size: K depends only on the bytes and their positions. The file is closed with `files.Close` once it is open,
on every outcome. A file shorter than its header says, a *w* × *h* × 4 outside 0 to 2^31 − 1, or an *n* × 132 above 2^31 − 1, is
undefined. The slot runs outside the tick, from the lobby, so no tick's fuel includes it. `headerChecksum` is the engine's
(section 3); the module only reads it. `names[1]` is never empty when the engine calls this slot.

## 5. Exactness

- **Integers.** Fields are 32-bit two's complement unless the record says otherwise. Products by 65536 wrap at 32
  bits. A field narrower than 32 bits keeps the low bits of the value stored; read back, a signed one (`short`) is
  sign-extended and an unsigned one zero-extended. `GetStartPosition`'s × 65536 is of the sign-extended 16-bit
  value, wrapping at 32 bits. The angle's division truncates toward zero, before the low 16 bits are kept.
  Signed comparisons everywhere a table above doesn't say otherwise.
- **Floats.** A double the parser gives is rounded to the nearest float (IEEE 754, ties to even). An int converted
  to float is rounded to nearest the same way. `GetMissionParams` copies floats as their bits.
- **Reads into *n* bytes.** A text read into *n* bytes keeps at most *n* − 1 bytes of the key's text, then a 0; the
  rest of the *n* bytes are zero.
- **Case.** "Ignoring case" and "lower case" act on the ASCII letters `A` to `Z` only.
- **The checksum K** of *len* bytes *b*[0] to *b*[*len* − 1]: four 8-bit values *a*, *x*, *c*, *d*, all 0 at the start.
  For each *i* from 0: *a* = (*a* + *b*[*i*]) mod 256; *x* = *x* XOR *b*[*i*]; *c* = (*c* + ((*i* XOR *b*[*i*]) mod
  256)) mod 256; *d* = *d* XOR ((*i* + *b*[*i*]) mod 256). K = *d* × 2^24 + *c* × 2^16 + *x* × 2^8 + *a*, as a 32-bit
  pattern.
- **Decimal numbers** in a `specialwhat` are read as the longest run of decimal digits there, as a 32-bit int;
  one beyond the int range is undefined.

## 6. Quirks

- **Q1.** A description or map name is looked up in the translation table in lower case, and the original text is
  kept only when the translation equals that lower-case key ignoring case; a key with no translation therefore
  gives the original.
- **Q2.** A map with no `MeteorWeapon` still has `wind_meteors.LoadMeteorDefaults` fill the params, after the
  shower is switched off, and `WIND_METEORS_PARAMS` is still published. When the defaults' file is missing, the
  params' `radius`, `density`, `duration` and `interval` are undefined.
- **Q3.** `StartPos1` and `StartPos0` both give `id` 0. Unnumbered start positions count from 1 and then take 1
  off, so the first is also `id` 0.
- **Q4.** A load writes the start resources of players 0 and 1 only. Those of players 2 to 9 keep what the record
  had, which is 0 for every record the engine makes.
- **Q5.** The start resources, `SurfaceMetal`, the AI profile and the meteor shower come from the chosen schema,
  and every other figure from the `GlobalHeader`.
- **Q6.** `RefreshMapList` loads the first map of the list, and keeps the list it has when it stays in the same
  mode or goes from skirmish to multiplayer.
- **Q7.** `LoadMissionByName` sets `text_b14` for types 2 and 3 even when the load fails.
- **Q8.** A load that fails leaves the record half-written: the figures at −1, the tables empty, and whatever the
  failed step wrote before.
- **Q9.** `AdvanceMission` returns 1 when the next mission exists, whatever its load gives.

## 7. Examples

From `tests/replay/mission/` of the kit. The recordings name element *i* of `names`, `startMetal` and `startEnergy`
as `names<i>`, `startMetal<i>` and `startEnergy<i>`, and record a text result (a `block` address) as the text it
points at, which is what the gate compares.

**A start position**, `console-ilose.txt`, the first `GetStartPosition` at tick 0, with `id` 0. The record holds 10
rules of `type` 1 with ids 0, 2, 4, 6, 8, 1, 3, 5, 7, 9. Rule 0 has `id` 0, `x` 560, `z` 240: `out.x` = 560 × 65536 =
36700160, `out.y` = 0, `out.z` = 240 × 65536 = 15728640, result 1. With `id` 1 the rule is rule 5 (`x` 3376, `z`
1408): `out` = (221249536, 0, 92274688).

**Counting start positions**, `console-ilose.txt`, `CountStartPositions` at tick 0: `ruleCount` 10, every rule of
`type` 1, result 10.

**A file name**, `console-ilose.txt`, `GetNameSlot` at tick 0 with `index` 1: `names[1]` is `Maps\The Pass.TNT` (the
recording writes the backslash doubled), so the result is its address, recorded as that text. With `index` 7 the
text is `ai\default.txt`.

**The figures**, `console-ilose.txt`, the first `GetMissionParams` at tick 0. The recording writes the out struct as
one value, `o result (...)`, its two arrays as `x:` bytes (`docs/replay-format.md`): `surfaceMetal` 3, `minWindSpeed` 100,
`maxWindSpeed` 3000, `gravity` 112, `tidalStrength` f32:0x00000000, `killMul` f32:0x42480000 (50.0), `timeMul` 0,
`startMetal` and `startEnergy` f32:0x447a0000 (1000.0) for players 0 and 1 and 0 for 2 to 9.

**The game type**, `console-ilose.txt`: the first `GetGameType` at tick 0 reads `type` 0, the second `type` 2. Between
the two, the engine's `SetMissionType` replaced the record. The recordings don't record the engine's slots, so the
replay restores the new record's fields from the next call's `r` lines.

**A mission unit**, `long/mission-build-unit.txt`, the first `GetMissionUnit` at tick 0, `index` 0, `unitCount` 160:
`out.name` `ARMCK`, `out.ident` the empty text (the key exists with no text), `out.initialMission` null (0),
`out.x` 249561088 (3808 × 65536), `out.y` 16056320, `out.z` 46137344, `out.angle` −31312, `out.health` 100,
`out.player` 1, `out.flags` 0, result 1. An `Angle` of 188 would give 188 × 65536 / 360 = 34224.3…, truncated to
34224, whose 16 bits read as −31312.

**A mission feature**, `long/mission-build-unit.txt`, `GetMissionFeature` at tick 0 with `index` 0: `featureCount` 3;
`out.name` `IceMetal03`, `out.x` 257, `out.y` 106, result 1. The campaign's `names[7]` there is `ai\DEFAULT.txt`.

**Not in any recording:** every slot that loads or frees a mission (`SetMissionType`, `FreeOtaEnumCacheAndMission`,
`LoadCampaign`, `LoadMissionByName`, `SelectMission`, `AdvanceMission`, `RefreshMapList`, `SelectSchema`,
`SetMissionUnitPosition`); the campaign and menu queries `GetCampaignName`, `MissionExists`, `CountMissions`,
`GetCampaignMissionName`, `ComputeMapChecksum`, `GetTerrainLength`, `GetTerrainSizeTier`, `GetBriefing`,
`GetDescription`, `GetPlanet`, `GetTranslatedName`, `GetMemory`, `GetNumPlayers`; `GetMissionIndex` and
`HasMissionName`, which no recording calls; `GetNameSlot` of an empty slot; `GetStartPosition` with no rule; and
`GetMissionUnit` and `GetMissionFeature` with an index outside the table. The scenario traces judge what a load
leaves in the record (section 3).

## 8. Kit version

Kit `interfaces-v58` and `reference-traces-v53` (both at ozgb/byte-tactics-moddable 27e198291), the versions the
gate judges against. Interface `mission` 2.0.
