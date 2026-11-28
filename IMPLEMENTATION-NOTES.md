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
- The module is 6850 bytes against a 13100 budget.

## Update to spec v59 / kit interfaces-v59 (wind_meteors 3.1)
- `UpdateWind(stream)` is one slot now; `Apply(event, size)` replaces the Enable/Disable/SetMeteorParams/CmdMeteor entry points (kinds 64-69, size checks per the spec).
- A meteor is published with `services.Emit` (a 36-byte SPAWN effect, weapon as a slot in `Game.weapons`, -1 for none) instead of `weapons.SpawnProjectile`. The `CommandArgs_GetIntArg` import is gone.
- `SimGlobals` meteor offsets moved down by 4 bytes (checked in `layout.json`); `Game` offsets are unchanged.
- `d2i` now follows the spec: truncate to 64 bits and keep the low 32; NaN, infinity or 2^63 and above gives 0 (the old one gave 0x80000000 on overflow).

## Verdicts, in order
1. req1: `equal` (3 recordings, every scenario, digest and view).

2. req2, req3: no verdict. The gate answered at once with `the gate wrote no usable result (exit 2): No such file or directory: .../gate/reqN/out/result.json`. The module submitted is byte-identical to req1's (6850 bytes), so this is a gate fault, not a difference.
3. req4 (after the review answers, same 6850-byte module): `equal` (3 recordings, every scenario).

## Answers to /srv/inbox/review.md
- 1 (`--no-stack-first`): `ld-wrap.sh` now appends `--no-stack-first` when the linker's help lists it. The local LLD 20.1.8 doesn't, and rejects it, so it's left off there and the stack falls after the data (top 202176, in the window). Not verifiable against 22.1.8 here.
- 2 (`EmptyShutdownPreCleanup`): kept. The volatile read keeps the table import and element segment ABI 9.1 requires; commented in the source. Unobservable.
- 3 (unterminated name): the spec calls it undefined; the copy is bounded at 32 bytes. Kept.
- 4 (weapon slot range): the spec says it never happens in a match. No guard.
- 5-8: nits, left. The dead `return` and spare NUL in `BAD` are harmless; `d2i` guards follow spec 5; the size-guarded arms are correct.
- 9: `fuel.py` and `build.sh` now use temporary directories and the unused `ng` is gone.

## Unclear points in the spec or kit
- ABI 3.1 says `--no-stack-first`; this toolchain's linker default already puts the stack after the data, so the option is not passed. rustc's `--stack-first` is removed by the linker wrapper. The stack top (202176) lies inside the window.
- `fuel.py` is my own pass (the kit now has `tools/wasm_fuel.py`; its `--check` accepts the module).
- `SaveMeteors`, `LoadMeteors`, `FatalError` and the Q3 outcomes follow the spec text only; no recording has them.
- The header's DISABLE comment ("drops no meteor") reads differently from spec Q4 (a due shower starts and is then made inactive); the module follows the spec and was equal.

---

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
