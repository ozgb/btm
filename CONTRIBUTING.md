# Contributing

btm is a clean-room re-implementation of Total Annihilation 3.1c's engine. Its modules are written from the
behavioural specs in `specs/` and the data kit, by people and agents who have never read the original's code or
any decompilation of it. The rules for implementation contributors, and the sign-off every commit carries, are in
#2.

## Specs

A spec in `specs/` is written by a spec writer, follows `specs/TEMPLATE.md`, and merges only after two reviews by
someone who didn't write it:

1. **Leakage:** a reviewer with the original's source checks that the spec states behaviour only. Nothing reaches
   this repository before this review approves: not a draft, a branch or a revision.
2. **Completeness:** a reviewer with only the spec and the kit, in the sandbox, replays the kit's recordings
   against a scratch implementation and lists every question the spec leaves open.

Each verdict is posted on the spec's pull request, signed `Spec reviewer:`. A revision goes through the leakage
review again, on its diff, before it is pushed.

## Spec questions

When a spec is wrong, incomplete, or contradicts the kit, open an issue labelled `spec-question`, with the spec's
section, the kit file or recording, and what you observed. A spec writer answers it by changing the spec, which
goes through review again. A spec question is never answered in free text.

## Commits

- Source only. Built files (`*.wasm`, `target/`) are never committed: the hand-over refuses a branch with a
  built file in any commit.
- Messages are `<Type>: <Subject>`, where Type is one of Add, Fix, Update, Remove, Refactor or Docs, and the
  subject is imperative, at most 50 characters, with no full stop. A message ends with
  `Assisted-by: <agent>:<model>` when an agent wrote it.
