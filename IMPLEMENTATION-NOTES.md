# wind_meteors implementation notes

## What was built
`wind_meteors/` is a `no_std` Rust cdylib (`src/lib.rs`) built by `wind_meteors/build.sh`:
- `ld-wrap.sh` is the linker wrapper. rustc passes `--stack-first` and `--allow-undefined`, which the ABI forbids, so the wrapper removes them. The `__heap_base` and `__data_end` exports are removed the same way.
- `fuel.py` is the fuel pass (ABI 11, rule v1). It works on the `wasm-tools print` text: it inserts the decrement at every function entry and loop top, then adds the counter global, `__fuel_set` and `__fuel_get`.
- Link options: data base 196608, table base 48, stack 4096.
- Flags: `target-cpu=mvp` plus sign-ext and bulk-memory, so no non-trapping float conversions. `d2i` does the guarded double-to-int conversion by hand.
- Every helper is `#[inline(always)]`, to keep the fuel use of a tick small against the 250 budget.
- Block fields are packed (unaligned), so all accesses are unaligned loads and stores at the offsets in `layout.json`.
- The sine table is a 512-entry `static` generated from the spec formula.
- `EmptyShutdownPreCleanup` volatile-reads a static function-pointer table. That keeps the `env.__indirect_function_table` import, which ABI 9.1 requires, and it is the element segment at table base 48.
- The module is 7421 bytes against a 13100 budget.

## Verdicts, in order
1. req1: `equal`. All 3 recordings and all 30 tier scenarios were equal.

## Unclear points in the spec or kit
- `--no-stack-first` (ABI 3.1) is not an option of the toolchain here (LLD 20.1.8). The default is already not stack-first, so I dropped it. rustc's own `--stack-first` had to be filtered out with a linker wrapper.
- The kit has no `tools/wasm_fuel.py`, so I wrote the metering pass from the rule text. It was accepted.
- I did not check `SaveMeteors`, `LoadMeteors`, `FatalError` or the Q3 outcomes against a recording, because none has them. They follow the spec text only, and the tier scenarios `save-midgame` and `load-midgame` were equal.
- I did not check fuel use against the 250 budget locally. The gate did not report out-of-fuel.
