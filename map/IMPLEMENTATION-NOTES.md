# map module: implementation notes

## What was built

`src/lib.rs` is a `no_std` Rust crate (`cdylib`, `panic = "abort"`) that exports the 15 slots of the map manifest
entry (the linker orders the export section by symbol, not by the manifest's list; the names are what the loader reads), the four data words (`g_game`, `g_simGlobals`, `g_extTdf`, `g_blockBase`) and the two
fuel functions. `build.sh` links it at `data_base` 557056 and `table_base` 400 with a 4096-byte stack, then runs the
kit's `wasm_fuel.py` (rule v1). `ld-wrap.sh` adds `--no-stack-first` when the linker lists it, and drops the `--stack-first`, `--allow-undefined` and `__heap_base`
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

1. req1 (first build): `equal` on all five recordings of `tests/replay/map/` (scope `recordings`, budgets not judged).
2. req1 (after the review fixes, resubmitted as req1 in a fresh outbox): `equal` on all five recordings.
3. req1 (after merging the spec's X36 wrap text and the second review): `equal` on all five recordings.

## Where the spec or kit was unclear

- No recording calls `PickRandomReclaimableResourcesInRadius`, `RefreshMovedUnits`, `RefreshUnitIfStale`,
  `MAP_RECLAIM_FOOTPRINT`, `MAP_LOAD_PATH_LOCK_STAMP`, the claim re-claim paths, the off-map bucket, the player
  visibility test or the move tests, so those were written from the text alone and are unverified by the gate.
- The `y` of a reclaim scan point (Q21) is the `y` of `pos`.
- A rebuild of a class with null `cells` writes nothing (the spec does not say; writing would hit block offset 0).
- The installed `wasm-ld` has no `--no-stack-first`; the default there already puts the stack after the data, and the
  linked `__stack_pointer` (561184) lies inside the window. `ld-wrap.sh` passes the option to a linker that has it.
- The kit has no map budgets (the entry is blocked), so fuel and size are not judged.

## Review answers

1. `--no-stack-first`: fixed in `ld-wrap.sh` (same `--help` test as the other modules).
2. Pick scan wrap: a step of 0x300000 that would pass `i32::MAX` now ends the row (or the scan) instead of wrapping
   into an endless loop. Spec X36 is silent on that case; no recording calls the pick.
3. Pick outputs: the picked entries are copied into locals before the two frees; the import order is unchanged.
4. The dead null-allocation branches are removed (`SimAlloc` never returns null, ABI 9.3).
5. Export order: the note is corrected. The linker fixes the order; the loader looks exports up by name.
6. Incremental rebuild cost: left as is. Fuel is not judged for this blocked entry, and a grown block is at most
   (fx+2)×(fz+2) cells; making it incremental would risk the recorded behaviour for no judged gain.
7. `build.sh` takes the fuel tool from `$WASM_FUEL` (default `/data/tools/wasm_fuel.py`); artifacts stay ignored.

## Second review answers

1. Pick scan wrap: the merged spec (X36) now says a position whose scan step wraps is outside the contract and a module may
   behave in any way. The module keeps its stop at the wrap (no endless scan); this is a deliberate departure from the
   wrapping text, allowed by that sentence.
2. `do_removal` now skips cells outside `0..Game.width` x `0..Game.height`, so a removal or clear of a unit with an off-map
   footprint and a null bucket cannot write outside `Game.cells`. The spec is silent; in-map cells behave as before.
3. The three dead guards on `SimAlloc` results are removed.
4. The panic handler executes `unreachable`, so a panic traps at once.
