# mission module

A query-only module: 12 exports, no imports. It reads the Mission record through `g_game` + `Game.net` (offset 233961)
and `g_blockBase`, using the offsets of `layout.json`. The engine's slots are not in the module (spec section 2).
`build.sh` builds it with rustc and `ld-wrap.sh`, then runs the kit's `tools/wasm_fuel.py` (fuel rule v1).

## Verdicts, in order
1. req1: refused, "has no data segment, so its data and stack cannot be placed".
2. req2: refused, "the manifest lists env.__indirect_function_table, which the module does not import".
3. req3: equal (recording mission/console-ilose; budgets not judged, the entry is blocked).

## Review answers (review.md)
1. Risk: the notes were wrong; ABI 9.1/2.2/2.3 do require the table import, element segment and data segment. `TABLE_ANCHOR`
   is the documented approach. `build.sh` now runs `wasm_fuel.py --check` and fails if the table import is missing.
2. Panic handler is now `unreachable()`.
3. All address arithmetic uses wrapping ops.
4. Named constants (`RULE_SIZE`, `UNIT_SIZE`, `FEATURE_SIZE`, ...) and shared `rule_count()`/`rule_at()`.
5. `GetMissionParams` and the tail of `GetMissionUnit` use `copy_nonoverlapping` (`memory.copy`).
6. `build.sh` quotes the linker path; the tool path comes from `FUEL_TOOL` (default /data/tools/wasm_fuel.py).

## Unclear points
- Earlier note "the spec/ABI doesn't say the table import is required" was wrong (ABI 9.1); spec section 2 now says
  "no function imports", which resolves the conflict.
- The verdict lists only `console-ilose`; `long/mission-build-unit` isn't reported (see req15 verdict below).
