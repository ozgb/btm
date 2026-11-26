# engine-core

This is the contract of the engine core: what a core takes in, what it is judged on, and what it is free to do
in between. It is meant to stay as it is once the port is done. What a core must also do while it runs the kit's
modules beside its own (the module ABI, the services, the scheduler's order, the state block's layout and the
replay of a recording) is in `specs/engine-core-transition.md`. That document is scaffolding, and nothing in it
is part of this contract.

## 1. Role

The engine core plays a match. It is a deterministic function: given the match setup and seeds, the game data
and the commands of each tick, it plays the ticks one after another, and after each tick it gives two things,
the judged state of the tick and the tick's sim events. Two cores conform when, for the same inputs, they give
the same judged state and the same sim events, up to the order the contract leaves free, at every tick.

Under the current architecture the engine core is native, and the subsystem modules and presentation are
WebAssembly that the core loads and calls (ozgb/byte-tactics-moddable#1047). Modules are the architecture, and
the surface a mod extends, so a conforming core runs the logic in them. What the contract leaves to the core is
how it gets there: it may run the logic in any order inside a tick, keep its state in any form and meet its
modules through any ABI (section 4.9), as long as both outputs come out the same.

## 2. Interface

### Inputs

| Input | Where the kit states it |
| --- | --- |
| The match setup and the seeds | a scenario file, `docs/scenario-format.md`, sections 1 to 3; the seeds are `game_seed` and `crt_seed` (section 3.1); `view_seed` seeds presentation only and is not an input of the logic |
| The game data | the files service, `interfaces/files.h`, version 3.0, whose behaviour `docs/files-gate.md` judges; the data is the install of `docs/reference-traces.md` |
| The commands of each tick | the scripted `spawn`, `order` and `console` lines of a scenario (`docs/scenario-format.md`, section 4), as events of the interfaces (`docs/commands.md`) |

A core reads nothing else. It reads no clock (the time of the logic is the tick counter), no host random
source, and no input but the commands. `input` lines (the presentation's mouse and keys) are not an input of
the core.

### Outputs

| Output | What it is | Where the kit states it |
| --- | --- | --- |
| The state of each tick | the `tick` row of `trace.txt`: one 64-bit hash of the judged state at the end of the tick (section 3) | `docs/trace-format.md`, sections 2, 3, 6 and 9 (the `state` column of a stored trace) |
| The sim events of each tick | the records the logic recorded in the tick, judged by the `tick` line of the event digest; the records the match load makes carry tick 0 and are judged as the line of tick 0 | `docs/sim-events.md`, `docs/events-log.md` ("The line of a tick") |

A core writes the other rows of `trace.txt` (the phase rows and `world`) and the lines of the runs of
`events.txt` if it keeps 3.1c's phases. They are diagnostics: the contract judges only the `tick` rows and the
`tick` lines.

### The interfaces it names

The files `interfaces/*.h` are the kit's interface export: C declarations used as a language-neutral contract
for layouts, kinds and signatures, which a core reads and does not compile.

The random streams and the generator of section 4.4 are reached through `interfaces/rand_stream.h`, version
1.0, and `interfaces/services.h`, version 4.0 (`GameRand`, `RandomInt`). The sim event kinds and their
arguments are `interfaces/sim_events.h`, version 1.0. Module-private state enters the judged state through
`interfaces/state_writer.h`, version 1.0, as `docs/trace-format.md` says for each section that a module writes;
how a core gives a module its `StateWriter` is scaffolding (`specs/engine-core-transition.md`, section 3).

The payloads of the events a console line publishes (section 4.3) are laid out by `interfaces/players.h`,
version 5.3, `interfaces/wind_meteors.h`, version 3.1, `interfaces/map.h`, version 9.2, `interfaces/units.h`,
version 7.2, and `interfaces/features.h`, version 5.1.

### Events

None. The core records no sim event kind of its own. It owns the queue the logic records into and hands each tick's
records over (section 4.5); the kinds, their arguments and the order that counts are those of
`docs/sim-events.md`.

### How it is judged

- **Now, the core gate:** `docs/core-gate.md`. A core is one program, run once for each vector of a group, as
  that page states ("Running a core"), and the gate compares what it writes with the vectors of
  `reference/core/` and `reference/files/`. The groups and what each covers:

  | Group | This spec |
  | --- | --- |
  | `hash` | section 3 and section 5, "The hash" |
  | `rand` | section 4.4 and section 5, "The random streams" |
  | `events` | section 4.5 and section 5, "The sim event queue" |
  | `commands` | section 4.3: the delivery only, its order and the tick each command precedes |
  | `console` | section 4.3, "Console lines" |
  | `runtime` | section 4.7 |
  | `loader`, `fuel` | section 4.8 for how the run ends; the checks themselves are scaffolding (`specs/engine-core-transition.md`, section 2) |
  | `host` | scaffolding (`specs/engine-core-transition.md`, section 4.4) |
  | `files` | the files service, `docs/files-gate.md` |

  The form of a core the gate runs (a static x86_64 Linux executable), the sandbox it runs in, the 120 s limit
  of each run and the result are `docs/core-gate.md`, "Running a core the gate doesn't trust" and "The verdict".
- **Later, the scenario mode:** `CORE play SCENARIO --out DIR` (`docs/core-gate.md`, "The scenario mode") plays
  a scenario that `docs/commands.md`, section 7, marks `judged`, and is judged on the `tick` rows of the trace
  against the `state` column of `reference/traces/` and on the `tick` lines of the event digest against
  `reference/events/`. It is the verdict on the whole contract, and it is `not_judged` until a core has the
  modules to play a scenario.

## 3. State

**The judged state** of a tick is what the `tick` row hashes (`docs/trace-format.md`, section 6): the tick
counter, the state of the generator of `RandomInt` and the word of the main stream (section 4.4), then the
sections of the phase rows and of the world row, read once at the end of the tick, and sections 29 to 31 at
tick 1 only. The trace format, its schema (`data/trace_schema.json`) and the rules of its section 11 decide
every hashed byte:

- **Zero fill.** Every byte of the state is zero until the logic writes it, padding and unused tails included.
  A field never written hashes as zero.
- **Pointers are IDs.** A pointer is hashed as a tag and an index (section 5 of the trace format), never as an
  address. So which element of a table an object takes (the unit slot, the projectile slot, the piece, the
  spatial grid record, the pathfinder node, the moving class) is judged; where in memory it lives is not. An
  order, an order effect and a link are numbered by the order of a walk of the units' chains, so their
  addresses are not judged either. A pointer to anything that is no element of its target table hashes as
  `heap`, whatever it points at.
- **Skips and clamps** are part of the format: a field the format leaves out is free, and a state past a limit
  of the format hashes as the format says.

The core's own structures are free. The state may be kept in any layout and any number of blocks, as long as
the hash is computed as if over the layout of `data/trace_schema.json` and the appendix of the trace format:
"an engine that lays its memory out differently implements the same test on its own positions"
(`docs/trace-format.md`, section 1).

**What is not judged state:** the load stream's word (in no row; its draws are judged through what they
change), the sim event queue and its radar arena (in no row; their records are the second output), the
presentation's state and its random generator, and every dumped-only section and record of the trace format.
The view bits that sit in hashed fields are masked out of the hash: the selection bit of a unit's `flags` and the
debug bits of `endGameFlags` (`docs/trace-format.md`, sections 7.1 and 8.1).

**Between ticks** a core carries the judged state, the load stream's word, the sim event queue's counters and
any state a module keeps of its own, and nothing else (section 4.6). The radar arena is empty between ticks
(section 4.5).

## 4. Behaviour

Within a tick only the two outputs are observable. The order in which the logic makes its writes is free except
where it changes a hashed byte, a draw of a random stream, or the order of two sim events whose order counts.

### 4.1 The start of a run

A test-mode run (`docs/scenario-format.md`, section 2) seeds, with the values of the scenario:

| When | What is seeded | With |
| --- | --- | --- |
| when the run starts | the main stream | `crt_seed` |
| when the match loads | the generator of `RandomInt` | `game_seed` |
| when the match loads | the load stream | `crt_seed` |

The main stream is not seeded again when the match loads: only the generator of `RandomInt` and the load stream
are (`docs/scenario-format.md`, section 2, "Seeds"). Seeding is section 5, "The random streams". The draws the
match load makes from these, and every other effect of the load, are the match's behaviour (the modules' specs),
not the core's. The tick counter is 0 while the match loads, so the sim events the load records carry tick 0 and
are judged as the line of tick 0 (section 2, "Outputs"). Every allocation of state is zero filled (section 3).

### 4.2 The tick

A tick is these, in this order:

1. The tick counter is incremented by 1. Tick *n* of the trace is the tick whose counter is *n*.
2. The scenario's settings that take effect before a step are applied: the `player<N>.metal_storage`,
   `energy_storage` and `share_*` keys before the step of tick 1 (`docs/scenario-format.md`, section 3.2), and,
   when the scenario's `game_type` is 3, every player's load progress set to 100 before every step (section 3.3),
   and, with `debug_keys` 1, bit 1 (0x2) of `uiOptionFlags` set before the step of tick 1 (section 2).
3. The commands of the tick are delivered (section 4.3), each to the end of its effects, before any logic of the
   tick runs. Their effects carry the new counter: a sim event a command's effect records has tick *n*.
4. The logic of the tick runs. Which logic runs is the modules' (their specs, and 3.1c's order in
   `docs/schedule.md`); the core is judged only on what results.
5. The judged state is projected: the `tick` row is the hash of section 3 over the state as it is now.
6. The sim events of the tick are handed over: every record whose tick is *n* is in the tick's output.

A test-mode run ends after the tick `ticks`, or at the end of the tick in which the match is declared over,
whichever is first (`docs/scenario-format.md`, section 2); that tick is played and hashed in full.

### 4.3 The commands of a tick

`docs/commands.md`, sections 1 and 2, is the rule; in short:

- A command is delivered between two ticks: after the counter of the tick it precedes is incremented, before
  the logic of that tick runs. The tick a scenario line names is the tick the command precedes.
- Within one tick: every `spawn` line, then every `order` line, then every `console` line, each kind in the
  order of its scenario line numbers (the order of the file).
- A command reads the state as every earlier command of the tick left it, and its events reach the modules that
  handle their kinds at once, in the order published; nothing a command publishes waits for a phase.
- Only the events a command publishes itself are its own (the `count` of `commands.txt`); an event that a
  handler publishes while it handles one is not.

What events a `spawn` or `order` line becomes needs the modules (the map and the orders, `docs/commands.md`,
sections 5 and 6). The core gate's `commands` group judges the delivery only.

**Console lines.** A console line is read as `docs/commands.md`, section 3, states: its words, its number
arguments, which command it is, the classes it is run with, the cheat code, the local player and a player that
exists. Its events are the ones section 4.2 of that page gives for its command, in publication order. Each event
is its payload as the interface header of its kind lays it out (`interfaces/players.h` and the others of
"The interfaces it names"), from the `kind` field on.

- A number argument is what the runtime's `atoi` (section 4.7) gives for the word, so it wraps modulo 2^32
  rather than stopping at the limits of an `int`: `give 4294967297 5 metal` gives to player 1.
- A line with no word (empty, all blanks, or a `#` before its first word) publishes nothing, and does not change
  the cheat code.
- The commands `give`, `meteor`, `atm`, `nometal`, `noenergy`, `sealevel`, `iwin`, `ilose`, `ai`, `kill`,
  `burnall` and `now` need nothing but the match setup: the game type, `localPlayer`, `playerIndex`, bit 0x2 of
  `uiOptionFlags`, and for each of the ten slots whether its record is active, its type, the side of its player
  info and its index. The core gate's `console` group states that setup and nothing else, runs lines of those
  commands one after another, and delivers their events to no module, so no state but bit 0x2 of
  `uiOptionFlags` changes between two lines (`docs/core-gate.md`, "console").
- `control` and `view` (`docs/commands.md`, section 4.2) also need nothing but that setup, but no vector of the
  `console` group runs them. The commands of the group `inert` publish nothing in any scenario
  (`docs/commands.md`, section 4.3). The other commands need the cursor, the units, or the unit and feature types,
  and only the scenario mode judges them.

### 4.4 The random streams

The logic draws from three generators, and from nothing else:

| Generator | Reached by | In the judged state |
| --- | --- | --- |
| the main stream | `GameRand(RAND_STREAM_MAIN)` | yes, as `rng.crtHoldrand` (the second random word of a row) |
| the load stream | `GameRand(RAND_STREAM_LOAD)` | no |
| the generator of `RandomInt` | `RandomInt(range)` | yes, as the first random word of a row |

Each is one 32-bit word, and section 5 gives each step. The main word and the generator's word are hashed at
the end of every tick, and every value drawn lands in state or in a sim event, so the number of draws a tick
makes from each generator, and their order within one generator, are fixed in effect. The order of draws from
two different generators against each other is free.

### 4.5 The sim events of a tick

The logic records sim events into a queue that the core owns, laid out and behaving as `docs/sim-events.md`,
"The record" and "The queue", say. A record carries the tick counter at the moment it is recorded. The records
of a tick are the ones that carry its counter, whichever part of the tick recorded them: the commands, the logic,
or the end of the tick.

What is judged of them is the `tick` line of `docs/events-log.md`: every field of every record but the phase,
as a multiset, and the order of the records only between kinds whose `Order:` lines in `docs/sim-events.md` name
each other. A core may record the events of unrelated kinds in any order.

The queue's bookkeeping is in section 5, "The sim event queue". A core that drains the queue does so at points
of its own choosing, as long as no record is lost and none is drained twice.

**The radar arena.** The two radar kinds carry the offset of a snapshot that the logic writes into a second
block beside the queue, the radar arena of 2 MiB (`docs/sim-events.md`, "Radar events"). The snapshot's bytes
are part of the record as the event log writes it, so they are judged with it. Every drain empties the arena
once it has handed over the records, whether or not the queue held a radar event, and so does the start of a
match: a snapshot lives from its recording to the next drain, and the arena is empty between ticks
(`docs/sim-events.md`, "Radar events").

### 4.6 Capture

All the state that affects a later tick can be captured between two ticks: the judged state, the load stream's
word, the sim event queue's counters and any state a module keeps of its own (section 3, "Between ticks"). A run that captures its state after a tick,
replaces it with anything, restores the capture and plays on gives the same later ticks as a run that did not.
`docs/scenario-format.md` states this as the `roundtrip` key, which copies the state block, fills it and copies
it back after the step of a tick. How a core captures (a copy of one block, a serialisation) is its own; what
is judged is that the later ticks are equal.

Saving and loading a game are not part of the contract (section 4.9).

### 4.7 The runtime

The logic calls a C runtime, among it the fixed-point sine and cosine, the products, quotients and roots, the
rotations and angles, `acos` and `hypot`, the conversions between text and numbers (`atoi`, `atof`, `%f`,
`sprintf`, `sscanf`, `_itoa`, `_ftol`), `rand` and the shuffle of a range, and the string, memory and character
functions. The kit's `runtime/vectors/` holds one entry for each function the logic calls:
`README.md` states what each entry computes and the format of its records, and `layouts.txt` the type of each
field. Each record is the inputs of one call and the results the original program gave for them, and a core
gives exactly those results: every bit of a double or of a float, the bytes it writes, the bytes of a buffer
after the call, the value it returns for an input out of range or NaN, and its treatment of bytes from 0x80 to
0xff. The core gate's `runtime` group runs each entry as one vector and each record as one call, as
`docs/core-gate.md`, "runtime", states.

The entry `rand` is the step of the main stream (section 5). The entries `shuffle` and `load_sequence` are the
shuffle of a range (section 5, "The shuffle of a range"), drawing from a generator stepped as the main stream
is: `shuffle` from a freshly seeded one, and `load_sequence` after the draws of the match load's explosion setup.

### 4.8 Failure

The run ends with an error, the same way on every machine, when:

| Cause | When | How it ends |
| --- | --- | --- |
| a full sim event queue | an event of a test-mode run does not fit (section 5) | a fatal error that says the queue is full; in the core gate, the value `"full"` |
| a full radar arena | a snapshot of a test-mode run does not fit (`docs/sim-events.md`, "Radar events") | a fatal error that says the arena is full; no core gate group calls the arena |
| a trap | a module traps, or an import refuses a call (`docs/module-abi.md`, section 13) | a fatal error naming the module and the tick; in the core gate, an error of the classes of `docs/core-gate.md`, "loader and fuel" |
| fuel run out | a module burns more than its budget in one tick (`docs/module-abi.md`, section 11) | a fatal error naming the module, the tick and the budget; in the core gate, `out_of_fuel` |
| a start-up refusal | the loader refuses the manifest or a module (`docs/module-abi.md`, section 5.2) | a fatal error before the match; in the core gate, the refusal's class |

A fatal error of a test-mode run ends it with exit status 2 and the message on the error stream. The core gate is
the exception: there the core writes the line of the call that ended the run, with the value the group states,
answers no further call and exits with status 0 (`docs/core-gate.md`, "An error that ends the run"). The error
happens at the same tick and the same call on every machine, so the trace up to it is the same; a core that
recovers from one of these and plays on does not conform. A run that is playing (not in test mode) counts an
event that does not fit instead of ending (section 5), and drops a snapshot that does not fit with its event.

### 4.9 Not part of the contract

These are free once the port is done. While it lasts they are scaffolding, in
`specs/engine-core-transition.md`, and no verdict of this contract rests on them. That the subsystems and
presentation are WebAssembly modules the core loads and calls is not among them (section 1): what is free is the
details of how the core and its modules meet.

- **The phases:** the nine rows of a test-mode tick, the order of the logic inside a tick, and the drains of the
  sim event queue between phases;
- **the route table:** which module receives which effect, and which deliveries are synchronous;
- **inboxes and `Apply`:** how an effect reaches a module;
- **the details of the module ABI:** the shared memory, the manifest, the loader, the data words, the imports and
  fuel;
- **the layout of the state block:** `layout.json` and where in memory each record is;
- **owner-only state:** what a module keeps that no other module reads, beyond what the trace hashes;
- **the recordings:** `tests/replay/` and replaying them;
- **saving and loading a game:** the save file, and the reseeding of the streams at a load.

## 5. Exactness

### The hash

The hash, the pointer IDs, the walk of each section and the rules of the hashed bytes are
`docs/trace-format.md`, sections 3 to 8 and 11; `tools/tracerehash.py` is a second implementation of every hash
from a dump. The core gate's `hash` group recomputes the rows of a dump, so it judges the hash apart from the
logic.

### The random streams

Every value is an unsigned 32-bit word, and every sum and product wraps modulo 2^32.

- **A stream** (main or load) is one word *w*. Seeding it with *s* sets *w* = *s*. A draw sets
  *w* = *w* × 214013 + 2531011 and returns bits 16 to 30 of the new *w*, that is (*w* >> 16) AND 0x7fff, a
  value from 0 to 0x7fff. This is the kit's `runtime/rand.c`.
- **The generator of `RandomInt`** is one word *g*. Seeding it with *s* sets *g* = (*s* XOR 0x66e29572) OR 1.
  `RandomInt(range)` with *range* below 2 (0, 1 or negative) returns 0 and leaves *g* as it was. Otherwise:
  let *q* = *g* div 127773 (unsigned), and *t* = *g* × 16807 − *q* × 2147483647, modulo 2^32. If *t*, read as a
  signed 32-bit integer, is 0 or below, *t* becomes *t* + 2147483647, modulo 2^32. Then *g* = *t*, and the value
  is *t* mod *range*, both unsigned.

  For a word from 1 to 2147483646 this is the minimal standard generator, *g* × 16807 mod 2147483647; a word
  that seeding left above that range is brought into it by its first step.
- **The shuffle draw** for a range *d* (the core gate's `shuffle`) makes one draw *r* from the stream, then
  widens: with *m* = 0x7fff, while *m* < *d* and *m* is not 0xffffffff, *m* becomes (*m* << 15) OR 0x7fff and
  *r* becomes (*r* << 15) OR 0x7fff, each modulo 2^32. The value is *r* mod *d*, unsigned. It is one draw from
  the stream whatever *d* is (Q1).
- **The shuffle of a range** of *n* elements *a*[0] to *a*[*n* − 1] does, for *i* from 1 to *n* − 1 in that
  order, one shuffle draw with *d* = *i*, giving *j*, and swaps *a*[*i*] and *a*[*j*]. It makes *n* − 1 draws
  (none for *n* of 0 or 1), and the first draw, with *d* = 1, is always 0 (Q3).

### The runtime

Bit for bit, as section 4.7 says: a result that is the correctly rounded value but not the original's is wrong.
The entries the README marks dropped (some `acos` inputs, and the character classes of -128 to -2) are free.

### The sim event queue

- **Records of an event.** An event of up to six arguments is one record; one of seven to twelve is two. The
  second record has the same `kind` and `tick` as the first, `flags` equal to the event's flags with bit 15
  (0x8000) set, and the arguments seven to twelve, with 0 in the arguments it lacks. Every argument a record
  doesn't carry is 0. `kind` and `flags` are the low 16 bits of what the logic gives.
- **Recording.** The queue is full for an event when `tail` − `head` + its records is above 4096. When it is not,
  its records go to the ring at index `tail` mod 4096 and the one after, and `tail` grows by the number of
  records. When it is full, nothing of the event is recorded: a test-mode run ends (section 4.8), and a run that
  is playing adds 1 to `dropped` and goes on.
- **Draining** hands over the records from `head` up to `tail`, oldest first, then sets `head` to `tail`. It
  does not clear the ring and does not change `dropped`. `reserved` is always 0.
- `head`, `tail` and `dropped` are 32-bit counts that only grow during a match; the queue is empty (all three 0)
  at the start of a match.

## 6. Quirks

- **Q1.** The shuffle draw makes one draw however wide the range is, and widens it by shifting in 1 bits, not
  further draws. For a range above 0x7fff the widened value therefore always has its low 15 bits all 1 before the
  modulo: for *d* = 32768 the value is always 32767.
- **Q2.** Seeding the generator of `RandomInt` always sets the lowest bit of the word, so two seeds that differ
  only in their lowest bit give the same generator.
- **Q3.** The shuffle of a range draws each *j* from 0 to *i* − 1, never *i* itself, so at its step an element
  always moves: a range of two elements always comes out swapped, and not every order of the range can come out.

## 7. Examples

All from the kit's `reference/core/`.

1. **A stream draw.** `rand.json`, vector `main-seed-1`: `srand main 1` sets the main word to 1. The first
   `rand main` gives *w* = 1 × 214013 + 2531011 = 2745024, and 2745024 >> 16 = 41, so the line is
   `[41, 2745024]`. `state` at the end is `[1924036713, 0, 0]`: the load stream and the generator were never
   seeded in this vector. The vector `load-seed-1` gives the same values from the load stream, and its `state`
   is `[0, 1924036713, 0]`.
2. **Seeding and stepping the generator.** `rand.json`, vector `int-seed-1`: `seed 1` gives (1 XOR 0x66e29572)
   OR 1 = 0x66e29573 = 1726125427. `int 0` and `int 1` give `[0, 1726125427]`, the word unchanged. The first
   `int 2`: *q* = 1726125427 div 127773 = 13509, *t* = 1726125427 × 16807 − 13509 × 2147483647 modulo 2^32 =
   633464266, positive as a signed integer, so the value is 633464266 mod 2 = 0 and the line is
   `[0, 633464266]`. The next `int 2` gives `[1, 1557480483]`.
3. **The shuffle draw.** `rand.json`, vector `shuffle-main`: after the call that leaves the main word at
   1850286559, `shuffle main 32769` draws *w* = 2280092966 and *r* = 2023. 0x7fff < 32769, so *m* becomes
   0x3fffffff and *r* becomes 2023 × 32768 + 32767 = 66322431, and 66322431 mod 32769 = 30744: the line is
   `[30744, 2280092966]`. Every `shuffle main 32768` of the vector gives 32767 (Q1).
4. **A two-record event.** `events.json`, vector `two-record`: at tick 9, `record 3 1` with seven arguments
   gives `[0, 2, 0]`; `pending` shows `[3, 1, 9, [18953, 69710, 3807, 11756, 52645, 59354]]` then
   `[3, 32769, 9, [68075, 0, 0, 0, 0, 0]]`: the second record has flags 1 with bit 15 set, and the seventh
   argument first. After `drain`, `counters` is `[6, 6, 0, 0]`.
5. **A full queue.** `events.json`, vector `test-overflow`: at tick 11, 4095 one-record events and one more fill
   the ring (`counters` `[0, 4096, 0, 0]`); the next `record` is `"full"`, the last line of the vector. In
   `play-overflow`, the same event in play mode gives `[0, 4096, 1]`: nothing recorded, `dropped` 1; after the
   `drain`, `counters` is `[4096, 4096, 2, 0]`: the drain leaves `dropped` as it was.
6. **Delivery.** `commands.json`, vector `console-iwin`: the script's lines are `12 console 1 0` (line 12 of the
   scenario, at tick 1) and `13 console 30 2 PLAYERS_KILL_UNITS ... PLAYERS_END_GAME ...` (line 13, at tick 30);
   the delivery is `1 console 12 0`, then `30 console 13 2 ...`. The `-reversed` vectors list the same commands with
   their scenario line numbers in the opposite order, and give the same order of delivery (`1 console 13 0`, then
   `30 console 12 2 ...`): the tick and the kind decide it before the line number.

7. **The cheat code.** `console.json`, vector `now-code`: a skirmish (`game_type` 2) with the code clear, the
   local and viewing player 0, and slot 0 active, of type 1, side 0 (Arm). The line
   `now Film Chris Include Reload Assert` gives `{"events": [], "code": 1}`, and the next line, `iwin`, gives
   `PLAYERS_KILL_UNITS` with payload `c10000000000000001000000` (kind 193, unit 0, player 1: the local player's
   side is 0) and then `PLAYERS_END_GAME` with `c70000000000000001000000` (victory 1). The same code in lower
   case, with five words or with seven clears the bit, and `iwin` then publishes nothing: it is of the class
   `cheat`, and a skirmish runs only `always` and `skirmish`. Blanks before `now`, a `#` comment after the sixth
   word and the command word `NOW` all still set it.
8. **A runtime record.** The runtime entry `atoi`: the record whose input is the text ` -12` (`202d3132`) gives
   `[-12]`. In the `console` group the same reading makes `give 4294967297 5 metal` a gift to player 1
   (vector `give-numbers`).

9. **The shuffle of a range.** The runtime entry `shuffle`, seed 1, size 4: the range is 1, 4, 7, 10 and the
   main word starts at 1. Step 1 draws 41 with *d* = 1, so *j* = 0: 4, 1, 7, 10. Step 2 draws 18467 with *d* =
   2, so *j* = 1: 4, 7, 1, 10. Step 3 draws 6334 with *d* = 3, so *j* = 1: 4, 10, 1, 7, the record's result. The
   next four values of rand, 26500, 19169, 15724 and 11478, are the record's last field. With size 2 the result
   is 4, 1 (Q3).

**Not covered by a vector:** the hash of a tick other than the five the `hash` group dumps (the scenario mode
judges every tick), a `link:` value of 2^31 or more and the pathfinder's `node:` pointers (`docs/core-gate.md`,
"hash" and "Known limits"), a shuffle draw of a range of 0 or of a negative range (the logic asks for
neither), and a `kind` or `flags` above 0xffff. The seeding of section 4.1, the tick of section 4.2, the end of
a run and capture are judged only by the scenario mode.

## 8. Kit version

Kit `interfaces-v49` and `reference-traces-v46`. Interfaces `services` 4.0, `rand_stream` 1.0, `state_writer`
1.0, `sim_events` 1.0, `files` 3.0, `players` 5.3, `wind_meteors` 3.1, `map` 9.2, `units` 7.2 and `features` 5.1.

The majors of `services`, `rand_stream`, `state_writer`, `sim_events` and `files`, and what those headers mean (the
order of sim events and of random draws, what `WriteState` hashes, how `Emit` picks an outbox), are frozen until a
clean-room core passes the core gate; their minors may still add to them (`docs/core-gate.md`, "The freeze";
ozgb/byte-tactics-moddable#1042).
