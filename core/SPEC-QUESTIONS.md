# Spec questions

## Q1. The wind_meteors module of /work/wind_meteors/ is an older interface than the kit the core is judged against

**What I found.** `/work/wind_meteors/build.sh` builds a module for the wind_meteors interface 1.2
(`git show origin/main:specs/wind_meteors.md`), not for 3.1 (`/data/interfaces/wind_meteors.h`, the kit
`interfaces-v59`, `/work/specs/engine-core.md` section 8). Evidence (`wasm-tools print module.wasm`, `src/lib.rs`):

- exports `UpdateWindOnStream`, `EnableMeteors`, `DisableMeteors`, `SetMeteorParams`, `CmdMeteor`, and
  `UpdateWind` with no argument; it has no `Apply`. The manifest entry of the kit (`/data/wasm/modules.json`,
  `wind_meteors`) lists the exports `UpdateWind(int)`, `UpdateMeteors`, `Apply`, `InitMeteors`,
  `EmptyShutdownPreCleanup`, `LoadMeteorDefaults`, `SaveMeteors`, `LoadMeteors`, and nothing else
  (`docs/module-abi.md` 9.1: the exports must equal the entry's exports, the data words and the two fuel
  functions).
- imports `weapons.SpawnProjectile` and `services.CommandArgs_GetIntArg`, which the kit's manifest and
  `interfaces/services.h` 4.0 do not have, and does not import `services.Emit`. The loader refuses it:
  `cannot find definition for import (weapons,SpawnProjectile)`, class `instantiate`.
- its `SimGlobals` offsets (`S_NEXT_STRIKE` 568 ... `S_GAP` 648) are 4 bytes above those of the kit's
  `layout.json` (`g_meteorNextStrikeTime` 564 ... `g_meteorScheduleGap` 644).

**Consequence.** The core gate's `host` group replays `tests/replay/wind_meteors/` against the submitted
module with `Apply` calls (kinds 64 to 69) and `i services.Emit` lines. The module of /work/wind_meteors/ cannot
answer them, so the `host` group cannot be `equal` with it, whatever the core does. I was told not to change
/work/wind_meteors/ and I did not.

**What I did instead of guessing.** I checked the core's host side with a throw-away copy of the module that I
patched in /tmp (`Apply` over the old slots, `Emit` for the spawn, the sim offsets of the kit, `UpdateWind(stream)`;
it is not in the repository and is not what I submit). With it the core's replay equals every `w` and closing `o`
line of all three recordings in `tests/replay/wind_meteors/` (`tools/host_check.py`: 613, 349 and 309 calls
equal, no fuel overrun at the 250 budget). So the remaining difference is the module.

**Question.** Is the module at /work/wind_meteors/ meant to be rebuilt for 3.1 (by whom: the module's own
session), or is the `host` group expected to be judged with the module as it is? I submitted the module as built.

## Q2. The row order of the `hash` group

`docs/core-gate.md` says the rows are "in the order of the dump's blocks"; `tools/tracerehash.py` sorts them by tick
and phase. They agree on the five vectors. The core follows the dump's order. (Not blocking.)
