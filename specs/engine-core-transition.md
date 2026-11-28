# engine-core-transition

**This document is removed when the last module is ported.** It is the scaffolding of the port: what an engine
core must also do while the kit's modules, its own modules and modules written from other specs run together in
one game. Nothing in it is a verdict on its own, but the core gate's `loader`, `fuel` and `host` groups judge
parts of it, and they count in the gate's verdict like every other group (`specs/engine-core.md`, section 2, "How
it is judged"). The contract of the core is `specs/engine-core.md`, and a core that meets that contract keeps its
logic in modules (`specs/engine-core.md`, section 1), and is free to meet them through details of its own in place
of those here once it no longer runs a module it didn't write.

## 1. Role

Under the current architecture the core is native, and the subsystem modules and presentation are WebAssembly
that it loads and calls (ozgb/byte-tactics-moddable#1047). During the port it hosts
them through the kit's module ABI. Every subsystem is a WebAssembly module over one shared memory, called through
its interface's slots, and the core loads the modules, defines the services they import, calls
their slots in 3.1c's order, carries the effects one module publishes to the modules that handle them, and keeps
the state block they all read and write. It can also replay a module's recordings, which is how a module, or the
core's hosting of one, is checked before a scenario can be played.

## 2. Interface

### The module ABI

`docs/module-abi.md` is the whole of it: the shared memory and table (section 2), the manifest
(`wasm/modules.json`, section 4), what the loader does and refuses (section 5), slots and event payloads
(section 6), argument kinds and the pointer rule (section 7), handles and the loader data words (section 8),
imports (section 9), interface majors and adapters (section 10), fuel (section 11), no hidden state (section 12)
and errors and traps (section 13). The core gate's `loader` and `fuel` groups (`docs/core-gate.md`) judge
sections 5.2, 11 and 13 on the fixtures of `reference/core/loader/` and `reference/core/fuel/`. When a manifest
holds more than one fault, the loader reports the first in the order of `docs/core-gate.md`, "loader and fuel"
("Which error comes first"): `stack_size` after the four required numbers (after `table_slots`) and before their
ranges, and in each module entry `fuel_budget` and then `size_budget` after `table_base` and before `exports`.

The numbers a manifest takes are `docs/module-abi.md`, section 4.1: a non-negative integer of at most 2147483647,
and a number with a fraction or an exponent makes the manifest invalid JSON. `stack_size` and a module entry's
`size_budget` are refused (`manifest`) when they are not non-negative integers, and a `fuel_budget` when it is
not an integer from 1 to 2147483647 (section 11.3). A manifest of `memory_pages` 65536 with `block_base`
2147418112 and a `fuel_budget` of 2147483647 is accepted (vector `manifest_pages_max`); no vector calls `memory` on it,
and a core must not need 4 GiB of RAM for it (`docs/core-gate.md`, "Known limits"). Which modules each class of
error names, the class `table` of a call through an index at or past `table_size`, and how a pair of faults is
ordered are `docs/core-gate.md`, "loader and fuel".

In the `loader` and `fuel` groups the `memory` call defines, beside the memory and the table, one import: the
name `mix` of the import module `fixture`, of kinds `[[int, int, int], int]`, which returns *a* + 10 *b* + 100 *c* of its arguments in i32
arithmetic that wraps (`docs/core-gate.md`, "loader and fuel"; vector `host_import`). It is the only import the
host defines there, so an adapted import (below) resolves against it alone. Every NaN a module's arithmetic
returns or stores is the canonical one, 0x7fc00000 for an f32, and the words 0 and 0x7ff80000 for an f64
(`docs/module-abi.md`, section 3.2; vector `nan_canonical`, which judges addition, division and the square root); an f32
that crosses the boundary keeps its 32 bits, and a module must not let a result depend on a NaN's bits.

**Adapters** (`docs/module-abi.md`, section 10.2) let a module built against an older major load: the loader
judges them from the entry's `slots` and `imports` and the manifest's `signatures` alone, so a refusal does not
depend on what the module exports or imports. Each refusal is a `manifest` error at
`load` that names no module. The forms they take and every refusal are `docs/core-gate.md`, "Adapters".

### The services

A module imports two kinds of group (`docs/module-abi.md`, sections 9.1 to 9.3): interface groups, each a slot
of another subsystem's interface under its name (`weapons.FindWeaponByName`), which the core defines by calling
that subsystem's slot of the same name (section 9.2); and the `services` group of `interfaces/services.h`,
version 4.2. The core defines every import of the services group, with the kinds of `signatures.services` of the manifest and the behaviour
of `docs/module-abi.md`, section 9.3:

| Import | What the core does |
| --- | --- |
| `GameRand`, `RandomInt` | the generators of `specs/engine-core.md`, sections 4.4 and 5, on the streams named by `interfaces/rand_stream.h`, version 1.0 |
| `FatalError` | ends the run with the message once the call has unwound (`docs/module-abi.md`, section 13) |
| `BuildDataPath`, `TdfFile_Construct`, `TdfFile_Destruct`, `TdfFile_LoadFile`, `TdfFile_SelectRecord`, `TdfRecord_GetFieldInt`, `TdfRecord_GetFieldDouble`, `TdfRecord_GetFieldString` | reads the game data through the files service (`interfaces/files.h`, version 3.0) and hands the records out as handles |
| `HapiBank_OpenAccount`, `HapiBank_SetIntegerItem`, `HapiBank_GetIntegerItem` | a saved-game bank's items, as handles; used only by save and load, which are outside the contract |
| `SimAlloc`, `SimAllocTagged`, `SimFree` | allocations in the state block, zeroed and 8-byte aligned, that last until they are freed |
| `RecordSimEvent`, `SimCaptureAlloc` | the sim event queue and the radar arena of `specs/engine-core.md`, section 4.5 |
| `GetGafFrame`, `InitGafSequence`, `StepGafSequence` | the frames of a frame table, load data outside the block (`docs/module-abi.md`, section 7.1), and the sequence cursors in the block that step through them |
| `GetSlotDpid`, `FindSlotByDpid` | the network ids of the game record's player slots |
| `Emit` | puts an effect in the calling module's outbox (section 4.2 below), after the checks of section 9.3 in its order: a pointer outside the module memory, a size outside 4 to 128 bytes, an effect that runs past the module memory, a kind not in the entry's `publishes` table, a size other than that kind's; then the kind's hook (wind_meteors' SPAWN: the weapon slot is one of the 256 of `Game.weapons` in `layout.json`, 0 to 255) |

A malformed argument to one of the services of minors 1 and 2 changes nothing and does not trap: the call returns
0 (`-1` for `GetSlotDpid`, `10` for `FindSlotByDpid`) and writes nothing, for the cases section 9.3 lists. An
import that refuses a call traps with section 13's text, `import <name>: ` and its cause, `Emit`'s five causes and
its hook's among them. `pointer argument <n>` counts the import's arguments from 0. Each refusal vector holds one
fault, so the order between two faults of one call is not judged.

The header comments give each import's result, and the recordings carry every call a recorded slot makes, with
its arguments and result (`docs/replay-format.md`, "Services").

### Events

None. The core records no sim event kind. A module records sim events through the queue of
`specs/engine-core.md`, section 4.5, whose layout is `docs/sim-events.md`, "The queue".

## 3. State

**The state block** is one region of the shared memory, at `block_base`, from `block_size` to `block_max` bytes
(`docs/module-abi.md`, section 2.1). Its records and their fields are `layout.json`: the game record, the
simulation globals record, the tables of units, projectiles and the rest, and the sim event queue. A module
reaches the game record and the globals record through the data words `g_game` and `g_simGlobals` and adds
`g_blockBase` to the block offsets a record stores (`docs/module-abi.md`, sections 7.1 and 8.2). Every
allocation in the block is zero filled (`docs/trace-format.md`, section 11).

**Owner-only state** is what a module keeps that no other module reads. Where the trace hashes or dumps it, the
module writes it through a `StateWriter` (`interfaces/state_writer.h`, version 1.0), which the core gives it in
the module's `WriteState` slot: a record per `Begin`, integers, byte runs and handles, each hashed as that
header and `docs/trace-format.md` (the section of each module) say. A handle is turned into the same pointer ID
on every build, from its kind.

**No hidden state.** A module keeps nothing between ticks outside the block; in a test-mode run the core hashes
each module's statics after every tick and ends the run when they change (`docs/module-abi.md`, section 12).

## 4. Behaviour

Within one slot call only the order of import calls is observable. The block writes are compared at the end of
the call, so the order they are listed in carries no meaning.

### 4.1 The scheduler's order

`docs/schedule.md` is 3.1c's order of a tick: the phases and what runs in each ("The tick"), the entry points the
load of a match calls before the first tick ("The load"), the loop of each pass and the order of the entry points
for one entity ("Passes"), the steps after the last phase ("After the phases"), and where a write is read back
inside one entity's update ("Split points"). The core calls the slots in that order. With the kit's modules and the
modules of the specs, a core that keeps it writes every row of `trace.txt`, the phase rows included, equal to
`reference/traces/` (whose digest folds the `view` row too, `docs/trace-format.md`, section 9, so that equality
also needs the view of `docs/view.md`), and the lines of the runs of the event digest equal to `reference/events/`:
the gate inside the reference repository. The contract judges only the `tick` rows and the `tick` lines
(`specs/engine-core.md`, section 2).

The sim event queue is drained between the phases, as `docs/sim-events.md`, "Draining", lists.

### 4.2 Routes, inboxes and `Apply`

- **Publishing.** A module publishes an effect, a payload that starts with its interface's event header, to its
  own outbox, through `Emit` (`docs/module-abi.md`, section 9.3). The kinds a module may publish, and each kind's
  payload size, are the `publishes` table of its manifest entry.
- **Routes.** A route takes the effects of one kind from one outbox to one subscribing module. A route is
  **queued** (the effect waits in the subscriber's inbox) or **synchronous** (the effect is delivered before the
  publishing call returns). Outside the tick (between ticks, and before the first) every route delivers at
  once (`docs/commands.md`, section 1, and `docs/schedule.md`, "Routes").
- **Inboxes and `Apply`.** A delivery copies the effect into the subscriber's inbox and calls its `Apply` slot
  with the event and its size (`docs/module-abi.md`, section 6.1). The subscriber's interface header gives the
  payload of each kind it handles.
- **The route table** is `docs/schedule.md`, "Routes": a row for each publisher, kind and subscriber, in the
  order the effect is delivered, with whether the route is queued or synchronous, the size of an inbox, and when
  each inbox is delivered (`docs/module-abi.md`, section 6.1, "Where routes are stated"). The core keeps that
  table and those rules.

### 4.3 Calling a slot

Before each slot call the core writes the module's data words. At the first outermost slot call of a tick it
sets the module's fuel to its `fuel_budget` (`docs/module-abi.md`, sections 5.1, 8.2 and 11.2). A slot call made
while an import runs is an error (section 13). A slot's arguments follow the kinds of `signatures` and the
pointer rule (sections 7 and 7.1); handles are valid until the outermost call returns (section 8.1).

### 4.4 Replaying a recording

`tests/replay/<subsystem>/` holds recordings of 3.1c's slot calls, in the format of `docs/replay-format.md`.
Replaying one against a module (`docs/replay-format.md`, "Replaying"): for each `call`, restore the fields of
its `r` lines into the block, call the slot with its `a` lines (an event payload built as "Event arguments",
"The payload in memory", states), answer each import the module makes from the recording's `i` and `o` lines in
order, and at the end of the call compare the fields of the write set and the closing `o` lines with the
recording. The core gate's `host` group judges the same replay from the other side, on the recordings of
`tests/replay/wind_meteors/` with the core's own wind_meteors module: every `w` line and every closing `o` line is
taken out of the recording, and the core writes them for each call, a `w` line for each field of the write set
whose value after the call differs from its last known value (`docs/replay-format.md`, "Lines"), then the closing
lines (`docs/core-gate.md`, "host").

- **The module** is the `replay` call's second argument, a path like every file a call names: `module.wasm`, the
  core's wind_meteors module, for the recordings of `tests/replay/wind_meteors/`, and a fixture module beside the
  recording, `host/wind_meteors_<vector>.wasm`, for each refusal vector (`docs/core-gate.md`, "The refusal
  vectors"). A fixture is loaded as the wind_meteors module of the game manifest, the kit's `wasm/modules.json`
  (its entry gives the data and table bases, `publishes`, the majors and the fuel budget). No call names that
  manifest, so the core carries it in itself. One instance of the module serves every `call` of a recording, so a static
  the module keeps lasts from one call to the next, and the handle table is cleared after each.
- **A text out with a `size`** (`TdfRecord_GetFieldString`'s `dst`) is written as at most *size* - 1 bytes of the
  recorded text and then a 0, so a text longer than the buffer is cut (`docs/replay-format.md`, "Services";
  vector `import_string_cut`).
- **An error that ends the replay** is the whole value of the `replay` call, in place of its lines: a trap or running out of fuel, an error of the
  loader classes; an import's refusal, `{"error": "import", "modules": ["wind_meteors"], "message": TEXT}`, TEXT
  section 13's text from `import ` to the end of its line; and a full handle table,
  `{"error": "glue", "modules": ["wind_meteors"], "message": "wind_meteors: the handle table is full"}`.

## 5. Exactness

- **Argument kinds.** An `f32` crosses the boundary as its 32 bits, unchanged; a narrow integer argument or
  result is extended by the side that produces it (`docs/module-abi.md`, section 7).
- **Pointers.** A `block` or `linear` argument is an address in the shared memory, 0 for null; a stored pointer
  in the block is an offset from `block_base`, 0 for null (`docs/module-abi.md`, sections 7 and 7.1).
- **Fuel.** Rule v1 of `docs/module-abi.md`, section 11.1, says what a call burns: one unit at each function entry
  and at the top of each loop body of the module's own code, and the call traps when the counter goes below
  zero. That metering is in the module's own code; the core adds none, and the engine it runs modules on has its
  own fuel off (`docs/module-abi.md`, section 11). The core sets the budget through `__fuel_set` once per tick, before the module's
  first outermost slot call of the tick, and never in between, and reads what is left through `__fuel_get`. A
  module whose manifest entry has no `fuel_budget` is never set: its counter starts at 9223372036854775807 and
  only its own burning lowers it (`docs/core-gate.md`, "loader and fuel").
- **Handles.** An opaque u32, index plus 1 into a table of 32 that the core clears when the outermost call
  returns (`docs/module-abi.md`, section 8.1): 32 handles in one outermost call are numbered 1 to 32, and a 33rd
  is the glue error of a full table (vectors `handles_32`, `handles_full`). Handle 0 is `null handle`, and a handle above
  the number issued in this outermost call is `unknown handle <n>`: a handle kept from an earlier call is garbage,
  and is unknown when the current call has issued fewer (`import_stale_handle`).

## 6. Quirks

None.

## 7. Examples

All from the kit's `reference/core/`.

1. **An overlap.** `loader.json`, vector `manifest_overlap`: `loader/overlap.json` has a `window` of 4096 and
   modules `a` at `data_base` 4096 and `b` at 6144, so `a`'s window (4096 up to 8192) holds `b`'s base; the
   line is `{"error": "overlap", "modules": ["a", "b"]}`, and it is the vector's only line.
2. **Fuel.** `fuel.json`, vector `exact_budget`: module `m` of `fuel/base.json` has a `fuel_budget` of 100, and
   its `spin n` (`fuel/metered.wat`) enters once and runs its loop body *n* times. `call m spin 98` sets the
   budget, burns 1 + 98 and gives `{"result": 98}`; `fuel m` is 1. With no `tick` in between, `call m spin 0`
   burns 1 at the entry and 1 at the loop's first body, the counter goes below zero, and the line is
   `{"error": "out_of_fuel", "modules": ["m"]}`. In `tick_resets`, a `tick` before each of three
   `call m spin 50` sets the budget again, so each gives `{"result": 50}` with `fuel m` 49 after it; a fourth
   `call m spin 50` with no `tick` before it starts from 49, burns past zero and is `out_of_fuel`.
3. **A replay.** `host.json`, vector `meteors-earthquake`: the recording's first `call`, `call 0 Apply` with
   `a event.kind 65` and `a size 4`, has no `w` line in `host/meteors-earthquake.txt`; the value's first entry is
   its write set, `w Game.windCounter 0` up to `w SimGlobals.g_meteorNextHitTime 0`, eleven lines. The second
   `call`, `LoadMeteorDefaults`, keeps the `o` lines of its imports (`o file#1.root rec#1`), and its entry is the
   five closing lines of what the slot wrote through its argument, `o params.weapon "Meteor"` up to
   `o params.interval f32:0x42700000`.

**Not covered by a vector:** the two import refusals of a service's own result (`returned a buffer other than
the one it was given`, `returned a weapon outside the sim block`: a replay answers each import from its
recording), the glue errors of a slot (`slot <name>: ...`), the services a module imports beyond what the
recordings call, `memory` at 65536 pages, the hidden-state check, a `StateWriter`, the routes and the scheduler's
order (`docs/core-gate.md`, "Known limits"). The scenario mode, and the module gate of each module, judge them.

## 8. Kit version

Kit `interfaces-v64` and `reference-traces-v54`. Interfaces `services` 4.2, `rand_stream` 1.0, `state_writer`
1.0, `sim_events` 1.0 and `files` 3.0.
