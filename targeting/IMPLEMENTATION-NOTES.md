# targeting implementation notes

## What was built
`targeting/` is a `no_std` Rust cdylib (`src/lib.rs`), built by `build.sh` (linker wrapper `ld-wrap.sh`, fuel pass `/data/tools/wasm_fuel.py`).
Data base 393216, table base 240, stack 4096. The module is 4794 bytes.
- All block fields are read and written at their `layout.json` offsets with unaligned accesses.
- The candidate list is a 6144-entry static (24 KiB, bss). It is zeroed after every search, so the statics hash does not change between ticks. The import is called once, with that capacity.
- The lead distance is a correctly rounded `sqrt` truncated to an integer, done in integers (`trunc_sqrt`) because `no_std` Rust 1.87 has no `f64::sqrt` and an `env` import is not allowed. The squares and their sum are real f64 operations in the spec's order.
- `KEEP` is a static function-pointer table read volatile in `Apply`. It keeps the table import and element segment the ABI requires, and nothing observes it.

## Verdicts, in order
1. req1: `equal` on all 9 recordings, as the gate reported it (blocked entry, so recordings only; budgets, scenarios, digest and view not judged).

## Unclear points in the spec or kit
- No spec question was needed.
- `wasm-ld` here is 20.1.8 and does not list `--no-stack-first`, so the wrapper leaves it off. The stack lands after the data, inside the window (top 421920).
- `tools/wasm_modules.py check-module` is not in the kit. I checked exports, imports and the stack top by hand with `wasm-tools`.
- The spec says the buffer's size is the build's choice. A search with more than 6144 candidates is truncated, which the spec's 52-candidate maximum in the recordings and the pool of 2501 do not reach.
- The behaviour for inputs outside the contract (null defs, unit targets outside the pool) is not defined by the spec. The module guards little of it: `t.def` is null-checked (a null def counts as "flag clear" in check 2), the `UnitTypeSet` lookup returns false for an index of 512 or more, the paralyzer test checks the slot's weapon def for null, and `Apply` validates `unit` and `target`. Nothing else is guarded: the lead path reads the weapon def without a null check, and the query slots do not check unit targets or weapon numbers.

## Review round 1
Verdict of the review: 0 bug, 2 risk, 5 nit. Only the build command and this file changed; `src/lib.rs` is unchanged.

1. **Count above 6144 truncated.** Not changed. The spec (section 2) lets the build choose the buffer and says the ABI sizes a caller's buffer for the whole pool. The module cannot hold a list longer than its window allows (24 KiB of bss and a 4 KiB stack already fill most of the 32 KiB window), and the spec defines no behaviour for a list it cannot hold, so I do not invent one. 6144 exceeds the pool of 2501 in every recording and the largest list (52). `Game.poolCount` is u16, so a pool above 6144 would need a larger window than the manifest gives. The note in "Unclear points" already says this.
2. **Unit target index not checked against `poolCount` (query slots).** Not changed. Spec section 7 puts a target outside the pool outside the contract and defines no outcome. A guard would be invented behaviour.
3. **No null check on the weapon def in the lead.** Not changed, same reason (section 7, null weapon def). The notes now say so.
4. **`in_set` bound and null `t.def`.** Kept. They are defensive choices only and no recording reaches them. They would read out of the set otherwise; the spec defines neither. The notes now list them as guards.
5. **Weapon above 2 in the query slots.** Not changed. Spec section 4 says the engine never calls them so; `FindWeaponTarget` masks `w` with 0xff as the spec requires and nothing more.
6. **Emit before the slot is cleared.** Not changed. The spec orders only the imports and compares writes at call end; the order here is Emit then clear, which matches the spec's listing "publishing CLEARED" then the slot writes.
7. **Notes inaccuracy and build command.** The guard sentence is corrected above. The "equal" verdict is the gate's report, not something the kit can reproduce. `build.sh` now passes `--locked` as ABI 3.1 does. `ld-wrap.sh` adds `--no-stack-first` when the linker lists it (the ABI's linker does); with lld 20, which does not list it, the stack lands after the data (top 421920), which I checked in the built module.
