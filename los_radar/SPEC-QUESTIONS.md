# Spec questions

## Q-gate-1 (ANSWERED: a fault in the host's generated glue; the gate now passes the recorded size and payload): the gate hands `Apply` an event of 4 bytes whose payload is zero

**Evidence.** Submissions req10 to req16 (req10 is the real module; req11 to req16 are probes whose `Apply` returns a
value computed from its arguments, to read what the gate passes: the verdict's `value` is the module's own result).

- req10 (real module): all three recordings differ at call 40, `Apply`, field `result`, value `0`. Call 40 is the
  first `Apply` of every recording: kind 137 (SIGHT_RESET), `a event.unit Unit#1`, `a size 8`, recorded result 1.
- req11: for kind 137 return `Game.units`: `592312` (so the state is right; the unit table is at block offset 592312).
- req12: return `event.unit - Game.units`: `-592312`, so `event.unit` (the word at event + 4) is **0**, not `Unit#1`.
- req14: return `event[2] + (size << 24)`: `67108864` = `4 << 24`, so word 2 is 0 and `size` is **4**.
- req15: return `size`: `4`.

So the replayer passes `size` 4 (the header alone) with the payload bytes zero, where the recording says `a size 8`
and `a event.unit Unit#1` (replay-format.md, "The payload in memory": the buffer is built from the `a event.*` lines
and passed with the recorded `a size`).

**Likely cause.** module-abi.md section 6: "The in-repo host copies at most the largest payload of the module's inbox
kinds (the header's size when no kind has a payload struct) and passes the smaller of that and the size it was
given." The `los_radar` entry of `wasm/modules.json` (interface v70 / kit) has no inbox/payload table (compare
`publishes` of wind_meteors and targeting, which exist for effects), so the host takes the header's size, 4, as the
largest payload and cuts every event to it. Spec section 3 (`Apply`) says a size below the kind's payload is
outcome 0 and nothing else, so a module that follows the spec answers 0 to every event the gate gives it; the
reference answers 1.

**What I cannot do.** The event reaches the module as 4 bytes: kind only. No unit, player, cell or flag reaches it, so
no module can reproduce the recorded writes (the sight of Unit#1, the explored counts) or the result 1. A module that
ignored `size` would still read a zero `unit`, which the spec's limits make outcome 0 (null unit).

**Question.** Should the gate's host know the payload sizes of los_radar's inbox kinds (8, 28, 20, 12, 16: spec
section 3)? Everything after the first `Apply` call (call 40) is hidden behind this, since the gate stops at the first
difference, so the rest of my module (the passes, queries, unit index) is not judged yet.

## Q-gate-2: `SimCaptureAlloc` returns a pointer the gate calls outside the module memory

**Evidence.** After the Q-gate-1 fix, req20 (the unchanged req10 module) fails on all three recordings with status
`failed` and no field (the verdict carries no message; the session brief names it: `import SimCaptureAlloc: returned a
pointer outside the module memory`). The first call is `RecordRadarMapped` inside `Apply` kind 133 (call 44):
`i services.SimCaptureAlloc 5724 = snapshot#1`, which my module makes with exactly those arguments (`bytes` 5724, `offset` a
4-byte local on my stack, inside the window).
req21 (same module, but `SimCaptureAlloc(8, &off)` instead): `differs` at call 44, `import 0 services.SimCaptureAlloc`,
value `services.SimCaptureAlloc 8`, i.e. the gate compares the import's arguments before it answers, so the
failure with the right arguments comes from the answer: the pointer the replayer's arena returns for 5724 bytes at offset 0.

**Reading.** The answer is the host's, not something the module controls: the module only passes `bytes` and the `offset`
address, both as recorded. replay-format.md ("Snapshots") says the replayer allocates the buffer from "an arena of its own"
and returns that pointer; module-abi.md section 6 says the module reads and writes it in place, so that arena must lie
in the shared memory. It looks like the same kind of fault as Q-gate-1 (the generated glue for the `snapshot` service).

**Question.** Where does the gate's arena for `snapshot` buffers lie, and is the pointer it returns for the first
call (5724 bytes at offset 0) inside the shared memory? If the arena is meant to be in the module's own window,
how large must the window's free part be (my data+stack use under 32768 bytes of it, the snapshot is 5728 bytes)?
Everything after this call is hidden behind it.
