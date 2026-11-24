# Nanolathe (imported subset)

Third-party code and research from [nanolathe-gg/nanolathe](https://github.com/nanolathe-gg/nanolathe), a Go
re-implementation of Total Annihilation's engine. It is the source for btm's data loading: HPI archives and the
overlay of loose files, TDF and FBI, COB, GAF and 3DO.

| | |
| --- | --- |
| Commit | `9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46` (2026-11-17) |
| Licence | MIT, Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors (`LICENSE`) |
| Changes | none: every file is a byte-for-byte copy at its own path |

## What was taken

| Path | What it is |
| --- | --- |
| `vfs/` | HPI archive reader and writer, the mount order and the overlay of loose files, with its tests |
| `formats/*.go` | the `formats` package: TDF and FBI, GAF, 3DO, and the other formats it holds (TNT, OTA, PCX, PAL, FNT, GUI, WAV), with its tests |
| `internal/cob/load.go`, `load_test.go` | the COB file loader |
| `research/formats/` | byte layouts: `hpi`, `tdf`, `fbi`, `cob`, `gaf`, `3do`, and the index |
| `LICENSE`, `NOTICE.md`, `docs/PUBLICATION.md` | the licence and Nanolathe's own statement of provenance |

## What was left out

- The rest of `internal/cob/` (the script VM), and every other package: game logic in btm is written from
  `specs/` and the kit, never from another engine.
- `research/retail-executable-spec/`, Nanolathe's description of the retail game's behaviour. It covers far more
  than loading (the unit, weapon and feature catalogs, map loading), which btm takes from `specs/`. The code
  comments cite it as `[02 ...]`, `[04 ...]` and the like.
- `formats/zrb/` (video) and the other research documents.
- `internal/sim/numeric`, which `formats/tdf_typed.go` imports for number parsing. A port takes its number
  parsing from the spec and the vectors.
- `go.mod`: nothing here is built in btm.

## How it is used

- It is reference material for a port to Rust. Nothing in this directory is compiled or run by btm.
- A ported file keeps the MIT notice of `LICENSE` at its head and names the file it came from.
- Nanolathe's claims about the retail game are its own and are not btm's contract. Where a port and the kit's
  vectors disagree, the vectors win, and the difference is a `spec-question`.
- Files the code and documents cite that were not imported are not part of btm and are not to be fetched.

## Provenance, as Nanolathe states it

Nanolathe's research comes from its authors' static analysis of the retail executable, whose MD5 it records as
`8e74a1dffa1f5988624c52048f5b20cd` (3.1c). `NOTICE.md` says repository inspection cannot certify that every
line is independently authored, and `docs/PUBLICATION.md` lists community-format and "OpenTA" attributions as
unresolved.

Implementers may read this directory (the human, 2026-11-24). It is the one exception to the rule of #2 that an
implementer has read no analysis of the retail executable.
