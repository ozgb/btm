# pathfinding implementation notes

## What was built
`pathfinding/` is a `no_std` Rust cdylib (`src/lib.rs`), built by `pathfinding/build.sh`:
- `ld-wrap.sh` removes rustc's `--stack-first`, `--allow-undefined` and the `__heap_base`/`__data_end` exports from the
  link line. Link options: data base 327680, table base 176, stack 4096, imported memory and table, `--gc-sections`.
- Fuel metering is added by the kit's `tools/wasm_fuel.py` as the last build step.
- Flags: `target-cpu=mvp` with sign-ext and bulk-memory. The module has no float code.
- The region is read and written in place at the offsets of spec section 3 (unaligned loads and stores); nothing is
  kept in statics. The five data words start at 1 so that they sit in the data segment and the module has no start
  function.
- The three imports no slot calls (`MoveClass`, `LineOfSight`, `TerrainPass`) are called behind a volatile test of a
  static that is 0, so that they are imported. The same block calls a function through a one-entry static table, which
  keeps the table import (the element segment is the one at table entry 176). A first version put the three imports
  themselves in the element segment, and the gate then wrote no result at all.
- `tools/pfreplay.js` is a local replayer for the recordings (node): it answers the imports from the recording and
  compares the region after every call. All five recordings are equal locally.

## Verdicts, in order
1. req1, req2, req3: no result (the gate exited 2 before a result; the element segment listed imported functions).
2. req4: `failed` on all five recordings, no detail.
3. req5 (Init takes the region from `g_region`): `failed`.
4. req6 (diagnostic: the same module with Init/Submit/Cancel/Step as no-ops): `failed`, so the failure does not depend on
   the slots' logic.
5. req7 (diagnostic: the old wind_meteors module under `wind_meteors`): `refused`, as its interface moved on.
6. req8 (data words in the data segment, no start function): `failed`.

7. req9 (req8 with the custom sections stripped): `failed`.

req1 to req9 were caused by a fault on the gate host, not by the module (operator's note).

8. req10 (same module, gate fixed): `equal` on all five recordings. Q-gate-1 is resolved.

9. req11 (review fixes, same logic): `equal` on all five recordings.

## Review answers
- Probe loop (risk 1): a spec gap; written as Q-probe-1 in SPEC-QUESTIONS.md. No guard, as it would change `steps`.
- Init with Game.width x Game.height above the map's cells (risk 2): outside the contract (spec section 4 `Init`: they
  are equal); a partial clamp would still leave `width`/`height` indexing past the cells, so none is added.
- `RegionSize` loop on a huge `unitSlots` (nit 3): bounded now. Dead code and warnings (nit 4): removed.
- Notes of wind_meteors (nit 5): kept in that module's own history; this file is pathfinding's.
- SPEC-QUESTIONS.md (nit 6): the resolved gate entry is dropped (it was a gate-host fault); it now holds Q-probe-1.
- build.sh (nit 7): intermediate file moved under target/, path quoted. The CI workflow is not changed here.
- pfreplay.js (nit 8): a local aid only, left as is.

## Unclear points
- A `failed` verdict has no message; the earlier ones were a gate host fault (Q-gate-1, resolved).
- `--no-stack-first` is not in the linker's help here (LLD 20.1.8); the default places the stack after the data, and the
  stack pointer was checked to lie in the window.
- Spec section 4 is otherwise sufficient: the five recordings replay equal locally from it. Not in any recording and
  written from the text alone: Shutdown, CellsChanged, PROGRESS, FAILED/PARTIAL outcomes, ring goals, Q5, Q6, event
  dropping in Cancel, Submit replacing an entry.

## Spec update (merged from spec-pathfinding)
The spec now states the probe's endless walk (section 7, "Edges") and puts the case outside the contract, so
Q-probe-1 is resolved and removed. The module's logic is unchanged; the last verdict (req11) is `equal`.

## Second review answers
- CI build (risk 1): `.github/workflows/module.yml` is shared repository infrastructure, not part of this crate; it knows
  only `wind_meteors`. `pathfinding/build.sh` is the build used for the gate (base 327680, table 176, checked with
  wasm-tools). Extending the workflow's `case` is left to its owner.
- `unitSlots` near 2^31 (nit 2): outside the contract (the recordings use 2501; the spec gives no range). Not guarded.
- `let _ = n;` (nit 3): removed; the warning is silenced with `#[allow(unused_assignments)]` on `goal_cells`.

10. req12 (after these changes, same logic): `equal` on all five recordings.
