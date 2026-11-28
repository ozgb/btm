# map module: implementation notes

## What was built

`src/lib.rs` is a `no_std` Rust crate (`cdylib`, `panic = "abort"`) that exports the 15 slots of the map manifest
entry in the manifest's order, the four data words (`g_game`, `g_simGlobals`, `g_extTdf`, `g_blockBase`) and the two
fuel functions. `build.sh` links it at `data_base` 557056 and `table_base` 400 with a 4096-byte stack, then runs the
kit's `wasm_fuel.py` (rule v1). `ld-wrap.sh` drops the `--stack-first`, `--allow-undefined` and `__heap_base`
/`__data_end` arguments that rustc adds.

- Every block record is read through unaligned accesses at the offsets of `layout.json` (cells are 13 bytes).
- `BuildAllPassMaps` does the full build row by row with the scratch block as a per-column flag array, so a
  cell costs about `fz + 2` pass-value evaluations instead of the whole grown block.
- `RefreshMovedUnits` really writes `Game.ticks` into the unit's stamp for the call and restores the old stamp at
  the end, so the pass values and the synchronous subscribers see the spec's Q27 behaviour.
- The reclaim pick writes its list entries past the end of an allocation when there are more qualifying features
  than the capacity, as Q22 describes.
- The nine engine slots are not exported.

## Verdicts, in order

1. req1: `equal` on all five recordings of `tests/replay/map/` (scope `recordings`, budgets not judged).

## Where the spec or kit was unclear

- No recording calls `PickRandomReclaimableResourcesInRadius`, `RefreshMovedUnits`, `RefreshUnitIfStale`,
  `MAP_RECLAIM_FOOTPRINT`, `MAP_LOAD_PATH_LOCK_STAMP`, the claim re-claim paths, the off-map bucket, the player
  visibility test or the move tests, so those were written from the text alone and are unverified by the gate.
- The `y` of a reclaim scan point (Q21) is the `y` of `pos`.
- A reclaim list whose allocation returned 0 is not written (the spec leaves a null allocation undefined).
- A rebuild of a class with null `cells` writes nothing (the spec does not say; writing would hit block offset 0).
- The installed `wasm-ld` has no `--no-stack-first` (the ABI text says it should); the default there already puts the
  stack after the data, and the linked `__stack_pointer` (561184) lies inside the window.
- The kit has no map budgets (the entry is blocked), so fuel and size are not judged.
