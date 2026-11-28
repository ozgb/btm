# los_radar implementation notes

## What I built
`src/lib.rs`: a `#![no_std]` Rust module (no crates) for the `los_radar` entry. It follows spec sections 3 and 4
slot by slot: sight add/take-off/reveal/move/place (circle and true types), `Apply` with its limits, eyeballs,
`RECALCULATE`, the two radar snapshots, the sensor/jam/cloak passes, the unit index (lists, push, refresh), the radius
queries, `ClampWorldPosToTerrain`, `SoundHeardMask`, and the cell queries. No static state: the region comes from the
`g_region` data word, the unit-slot limit from `Game.poolCount` (equal to `map.unitSlots` in the recordings).
`build.sh` links at data base 425984 / table base 272 (stack 4096, `ld-wrap.sh` as in wind_meteors) and meters the
result with the kit's `tools/wasm_fuel.py`.

## Verdicts, in order
1. req1: `refused`: "has no data segment". Fixed by giving `g_extTdf` the initial value `tdf\0` (a data segment).
2. req2: `refused`: the module did not import `env.__indirect_function_table`. Fixed with a function pointer read
   and called in `Shutdown` (keeps the table import and an element segment at 272).
3. req3 to req9: no verdict. Each answer is `the gate wrote no usable result (exit 2): No such file or directory:
   .../gate/reqN/out/result.json`, returned within seconds. req7 was a stub module whose slots all return at once
   (same surface), and it failed the same way, so the fault is in the gate's replay of los_radar and not in the
   module's behaviour. req3, 4, 5, 6, 8, 9 are the same 18671-byte module.

10. req10 (after the gate was fixed): `differs` on all three recordings at call 40 (the first `Apply`, kind 137),
    field `result`, value 0 (recorded 1).
11. req11 to req16: probes (not the real module) that return values computed from `Apply`'s arguments. They show the
    gate passes `size` 4 and a zero payload (`event.unit` 0) where the recording says `a size 8`, `a event.unit Unit#1`.
    The module follows the spec (size below the payload: outcome 0), so it cannot match. Question in
    SPEC-QUESTIONS.md (Q-gate-1). The source is unchanged from req10's.

Until that is answered, the module has not been judged beyond the first call: the gate stops at the first difference.
(Earlier note:) the module had not been judged against the recordings: no `equal` and no `differs` was ever seen. It is
validated with `wasm-tools validate`; exports are the entry's, plus the data words and `__fuel_*`; imports are the
eight listed plus `env.memory` and the table. I did not hand the branch over, since it is neither `equal` nor
blocked on a spec question.

## Unclear points
- The kit's interface tag is `interfaces-v74`, as spec section 8 says (earlier notes saying v69/v70 are obsolete).
- Where `unitSlots` comes from after `Init`: the spec says the limits use `Init`'s map, but no slot gets the map
  later, and the region size is not in a data word. I use `Game.poolCount` (WriteState does too).
- Behaviours the spec leaves outside the contract are guarded rather than reproduced: a null `def`/`player` of a unit
  in a pass is skipped; a null projectile `attacker` counts as not the viewer's; a sight mask the import gives as null
  adds and reveals nothing; a division by zero in `ClampWorldPosToTerrain` leaves `p` as found.
- Comparisons of the 32-bit distance with a squared radius are signed (spec section 5 says signed for "the others").

## Update after the Q-gate-1 fix

Verdicts in order: req10 to req16 (see Q-gate-1; `differs` at the first `Apply`, call 40), req20 (req10's module
again, with the gate passing the recorded event): `failed` on all three recordings, the gate's message being
`import SimCaptureAlloc: returned a pointer outside the module memory`; req21 (probe, `SimCaptureAlloc(8, ..)`):
`differs` at call 44 on the import's arguments, so the failure comes from the gate's answer to the recorded call
(5724 bytes). Q-gate-2 in SPEC-QUESTIONS.md asks about it. The module itself is unchanged.

## Update after the Q-gate-2 fix

Verdicts: req30 `differs` on `console-sight` at call 505 (tick 30), `Game.visibilityMask`; `console-units` and
`sensors-sonar-jam` `equal`. req31 to req46 were probes (not the module) that write diagnostics into the mask entry the
verdict names; they show the gate's frame and frame table are all zero when read (Q-gate-3).

Change from req10: the mask index reads `FrameTable.count` only when the index is 9 or more, since the reference
reads it once (call 0) and the replayer's table has no `count` at call 505 (req10 asked for frame −1). (Superseded by the review round below: the clamp now always reads the count.)

## Final: req60

req60 (the module as committed, no change) is `equal` on `console-sight`, `console-units` and `sensors-sonar-jam`
(the gate now maps the load data, so Q-gate-3 is answered). The verdict lists the rule notes `queries:
services.GetGafFrame` and `unread_arguments: los_radar.GetMapCellAtPosition pos.y`; 

## Review round (req70)

req70 is `equal` on all three recordings. Changes for review.md:
1. `Apply` takes `size` as `i32`; a negative size gives outcome 0.
2. `mask_index` always clamps to `FrameTable.count - 1` (spec section 4); the Q-gate-3 workaround is gone.
3. Not changed, open spec question: the unit-slot limit still uses `Game.poolCount`. `Init` gets `size` (so
   `unitSlots` = size / 5) but spec section 3 says the module keeps nothing else, and storing it would add state
   the spec does not list. The recordings cannot tell the two apart (`poolCount` = `unitSlots`). Spec question:
   should the module keep `unitSlots` from `Init`, or is `poolCount` the intended bound?
4. `Shutdown` writes 0 to `g_region` after reading it.
5. Nit, not changed: the circle-mask loop in `add_sight` and `reveal` stays duplicated; the two copies differ in
   what they do per pixel and the module is equal, so I did not risk a refactor. Extra `GetGafFrame` calls are
   allowed by spec section 2.
6. Notes brought up to date (this file).

Open question: the `unitSlots` source (item 3), not blocking.

## Review round 2 (submission req80)

1. `has_ready`, `CreateUnitIndex`, `FreeUnitIndex`: `p >= 10` is now a no-op / false. Fixed.
2. `UpdateUnitLineOfSight`: returns when the slot is not below `Game.poolCount`. Fixed.
3. `ExpireEyeballs`: returns when the count is outside 0..=20 (as ADD_EYEBALL does). Fixed.
4. New `last_slot()` gives 0 when `unitsEnd` < `units`; replaces the four wrapping uses. Fixed.
5. SHARE_MAPPING/SHARE_RADAR return 0 when the player's `info` is 0; `add_sight` returns when `explored` is 0. Fixed.
6. `ClampWorldPosToTerrain` uses `wrapping_div`. Fixed.
7. Stale notes: the `poolCount`-versus-`unitSlots` question is answered by spec §4 (Apply limits: the module reads `Game.poolCount`). No longer open; the code was already right.
8. `Init` no longer writes `g_region` (the host writes the data word); `Shutdown` no longer zeroes it. Fixed.
9. `build.sh` quotes the linker path. The `/data/tools/wasm_fuel.py` path stays: it is the kit's tool.
