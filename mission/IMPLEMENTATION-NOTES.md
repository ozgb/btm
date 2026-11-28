# mission module

A query-only module: 12 exports, no imports. It reads the Mission record through `g_game` + `Game.net` (offset 233961)
and `g_blockBase`, using the offsets of `layout.json`. The engine's slots are not in the module (spec section 2).
`build.sh` builds it with rustc and `ld-wrap.sh`, then runs the kit's `tools/wasm_fuel.py` (fuel rule v1).

## Verdicts, in order
1. req1: refused, "has no data segment, so its data and stack cannot be placed".
2. req2: refused, "the manifest lists env.__indirect_function_table, which the module does not import".
3. req3: equal (recording mission/console-ilose; budgets not judged, the entry is blocked).

## Unclear points
- Neither the spec nor the ABI says the gate refuses a module with no data segment, or one that doesn't import the
  table. A module of only queries has no function pointers, so it needs a static array holding a slot
  (`TABLE_ANCHOR`) to get a table import, an element segment at 496 and a data segment.
- The verdict lists only `console-ilose`; `long/mission-build-unit` isn't reported.
