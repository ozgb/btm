# economy implementation notes

## What was built
`src/lib.rs`: a `no_std` Rust module following `specs/economy.md`: the eleven slots, the five data words,
imports of `econworld` and `network.SendShareMapInfo`. `build.sh` links at data base 229376 and table base 80
with the stack after the data, then meters with the kit's `tools/wasm_fuel.py` (rule v1). `ld-wrap.sh` drops
rustc's `--stack-first`/`--allow-undefined`. The only static that changes is `UNIT_SLOTS`, written once by
`Init` (the module has no other place to keep `map.unitSlots` for the refusals).

## Verdicts, in order
1. req1: refused, "has no data segment" (all statics were zero). Fixed by giving `g_extTdf` the value `tdf\0`.
2. req2: refused, the manifest lists `env.__indirect_function_table` and the module did not import it.
   Fixed by calling `Shutdown`'s result through a function pointer in a static, which makes the linker import the table.
3. req3, req4, req5 (same module): no verdict, host error "the host's build has no replay_native_test.exe
   (a fault of the host, not of the module)". The behaviour has therefore never been judged: it is NOT known to be `equal`.

## Unclear points
- The spec says kit v72, the gate reports v73 (see SPEC-QUESTIONS.md).
- A module with only zeroed statics and no function pointer is refused by the gate (data segment, table import);
  the ABI text does not say so directly.
- No local replayer exists in the kit, so the arithmetic was written from the spec alone and checked only by reading.

## Later verdicts
- req3 to req5: no verdict (the host replayer was missing).
- req10: `equal` on all 9 recordings (kit interfaces-v73 / reference-traces-v57). Question 1 in SPEC-QUESTIONS.md is answered: the spec's v72 pin stands.
