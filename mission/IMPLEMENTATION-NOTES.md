# mission module

Pure query module (12 exports, no imports) reading the Mission record through `g_game` + `Game.net` and `g_blockBase`.
Built by `build.sh` (rustc, wasm-ld wrapper, kit's `tools/wasm_fuel.py` for fuel rule v1).

## Verdicts
(see below, in order)

## Spec notes
- No ambiguity found; the engine slots are not part of this module.
