# los_radar implementation notes

## What I built
`src/lib.rs`: a `#![no_std]` Rust module (no crates) for the `los_radar` entry. It follows spec sections 3 and 4
slot by slot: sight add/take-off/reveal/move/place (circle and true types), `Apply` with its limits, eyeballs,
`RECALCULATE`, the two radar snapshots, the sensor/jam/cloak passes, the unit index (lists, push, refresh), the radius
queries, `ClampWorldPosToTerrain`, `SoundHeardMask`, and the cell queries. No static state: the region comes from the
`g_region` data word, the unit-slot limit from `Game.poolCount` (equal to `map.unitSlots` in the recordings).
`build.sh` links at data base 425984 / table base 272 (stack 4096, `ld-wrap.sh` as in wind_meteors) and meters the
result with the kit's `tools/wasm_fuel.py`.

## Verdicts, in order
1. req1: `refused`: "has no data segment". Fixed by giving `g_extTdf` the initial value `tdf\0` (a data segment).
2. req2: `refused`: the module did not import `env.__indirect_function_table`. Fixed with a function pointer read
   and called in `Shutdown` (keeps the table import and an element segment at 272).
3. req3 to req9: no verdict. Each answer is `the gate wrote no usable result (exit 2): No such file or directory:
   .../gate/reqN/out/result.json`, returned within seconds. req7 was a stub module whose slots all return at once
   (same surface), and it failed the same way, so the fault is in the gate's replay of los_radar and not in the
   module's behaviour. req3, 4, 5, 6, 8, 9 are the same 18671-byte module.

So the module has not been judged against the recordings: no `equal` and no `differs` was ever seen. It is
validated with `wasm-tools validate`; exports are the entry's, plus the data words and `__fuel_*`; imports are the
eight listed plus `env.memory` and the table. I did not hand the branch over, since it is neither `equal` nor
blocked on a spec question.

## Unclear points
- The gate's `kit.interfaces` is `interfaces-v70`; the spec says v69 (and the branch `spec-los_radar` has no newer commit).
- Where `unitSlots` comes from after `Init`: the spec says the limits use `Init`'s map, but no slot gets the map
  later, and the region size is not in a data word. I use `Game.poolCount` (WriteState does too).
- Behaviours the spec leaves outside the contract are guarded rather than reproduced: a null `def`/`player` of a unit
  in a pass is skipped; a null projectile `attacker` counts as not the viewer's; a sight mask the import gives as null
  adds and reveals nothing; a division by zero in `ClampWorldPosToTerrain` leaves `p` as found.
- Comparisons of the 32-bit distance with a squared radius are signed (spec section 5 says signed for "the others").
