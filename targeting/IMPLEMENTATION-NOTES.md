# targeting implementation notes

## What was built
`targeting/` is a `no_std` Rust cdylib (`src/lib.rs`), built by `build.sh` (linker wrapper `ld-wrap.sh`, fuel pass `/data/tools/wasm_fuel.py`).
Data base 393216, table base 240, stack 4096. The module is 4794 bytes.
- All block fields are read and written at their `layout.json` offsets with unaligned accesses.
- The candidate list is a 6144-entry static (24 KiB, bss). It is zeroed after every search, so the statics hash does not change between ticks. The import is called once, with that capacity.
- The lead distance is a correctly rounded `sqrt` truncated to an integer, done in integers (`trunc_sqrt`) because `no_std` Rust 1.87 has no `f64::sqrt` and an `env` import is not allowed. The squares and their sum are real f64 operations in the spec's order.
- `KEEP` is a static function-pointer table read volatile in `Apply`. It keeps the table import and element segment the ABI requires, and nothing observes it.

## Verdicts, in order
1. req1: `equal` on all 9 recordings (blocked entry, so recordings only; budgets, scenarios, digest and view not judged).

## Unclear points in the spec or kit
- No spec question was needed.
- `wasm-ld` here is 20.1.8 and does not list `--no-stack-first`, so the wrapper leaves it off. The stack lands after the data, inside the window (top 421920).
- `tools/wasm_modules.py check-module` is not in the kit. I checked exports, imports and the stack top by hand with `wasm-tools`.
- The spec says the buffer's size is the build's choice. A search with more than 6144 candidates is truncated, which the spec's 52-candidate maximum in the recordings and the pool of 2501 do not reach.
- The behaviour for inputs outside the contract (null defs, unit targets outside the pool) is not defined by the spec. The module does not guard them, apart from null checks on the weapon def and the `UnitTypeSet` refs.
