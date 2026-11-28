# Spec questions

## Q-gate-1: the gate answers `failed` on every recording, whatever the module does

**Resolved.** The failures came from a fault on the gate host (fixed by the operator). The unchanged module was resubmitted as req10: `equal` on all five recordings. No spec change needed.

Spec: section 2, "The gate" (recordings route, blocked entry, `--no-budgets`); kit: `docs/module-abi.md` section 15,
`docs/replay-format.md` "Against the WASM module".

Evidence:
- `tools/pfreplay.js` (in this branch) replays the five recordings of `tests/replay/pathfinding/` against
  `module.wasm` in node, answering the imports from the recording's `i` lines (same order, same arguments, the `o`
  lines written through the pointers), restoring the `r region` runs, and comparing the whole region after every
  call against the recorded `w region` runs and the `o out` / `o result` lines. Every file is equal: 12, 26, 41, 13
  and 30 calls. (Corrupting one region byte after a `Step` makes it report `differs`, so the check is not vacuous.)
- The gate's verdict for the same file (req4, req5, req8, req9) is `"verdict": "failed"` with `"status": "failed"` on all five
  recordings, no `tick`, `phase`, `field` or `value`, in about a second.
- req6 was the same module with `Init`, `Submit`, `Cancel` and `Step` reduced to no-ops (`RegionSize` and the exports
  and imports unchanged): the verdict is `failed` on all five recordings too, not `differs`. So the failure does not
  depend on what the slots compute.
- `check-module` passes (the verdict is not `refused`): imports are exactly the 18 of `pathworld`, the exports are the
  eight slots, the five data words and `__fuel_set`/`__fuel_get`, the stack is inside the window (stack pointer 331872
  in 327680..360448), the one element segment is at table entry 176.
- req1 to req3 (an element segment that listed three imported functions) gave "the gate wrote no usable result (exit
  2)"; removing that segment made the gate get to the recordings.

Question: what makes a replay `failed` before the first divergence (a load refusal, a trap, a missing file)? A
`failed` verdict carries no message, so the cause cannot be found from the verdict's fields. Is something in the
module's shape expected that the spec does not state (for example how `g_region` is set before `Init`, or whether the
module may have a start function), or is the recordings route of a blocked entry not yet runnable for pathfinding?
