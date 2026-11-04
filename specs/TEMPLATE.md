# Spec template

Every subsystem spec in `specs/` has these sections, in this order. A spec states behaviour: what a
caller and the state block can see. It never describes how the original was written.

The kit is the data kit a spec is written against (`MANIFEST.json`, `layout.json`, `interfaces/`,
`docs/`, `tests/replay/`, `reference/`). A spec uses the kit's names (slots, services, `layout.json`
records and fields, replay line names) and no others. When the spec needs a word for something the
kit doesn't name, it defines the word in the spec.

## 1. Role

What the subsystem is responsible for in the game, in one paragraph.

## 2. Interface

- The slots it provides, by their names in `interfaces/<subsystem>.h`, and when the engine calls each.
- The imports it calls (interface imports and services, `docs/module-abi.md` section 9), each with what
  the subsystem uses it for.
- Any argument that isn't in the block (a params record, a console command, a saved-game bank): its
  layout, or where the kit gives it.

## 3. State

- The block fields it owns (writes and keeps between ticks), by `layout.json` record and field.
- The block fields it reads that other subsystems own.
- What has to hold between ticks.
- The types that matter for exactness: signedness and width of each field it computes.

## 4. Behaviour

One subsection per slot: its inputs, then the ordered list of effects seen outside it (state
writes, import calls with their arguments, the result). The order of import calls is part of the
behaviour: the replay gate compares them call by call.

Effects are stated as results ("`X` becomes ..."), never as the original's steps or helpers.

## 5. Exactness

Everything an implementation has to reproduce bit for bit, collected in one place:

- random draws: which stream, how many, in what order, and how each value is scaled;
- integer arithmetic: width, wrapping, signed or unsigned comparison, rounding of division and shifts;
- floating point: the precision of each operation and each conversion;
- fixed tables and functions that the kit doesn't provide, defined exactly.

## 6. Quirks

Each behaviour of 3.1c that looks wrong but that conformity requires, stated as behaviour, with an id
(`Q1`, `Q2`, ...) that the rest of the spec refers to.

## 7. Examples

Calls taken from the kit's recordings: "`tests/replay/<subsystem>/<file>`, call at tick N of `Slot`:
given these inputs, this happens", with enough numbers that an implementation can check a formula by
hand.

## 8. Kit version

The `interfaces-v*` and `reference-traces-v*` tags the spec was written against, from the kit's
`MANIFEST.json`, and the subsystem's interface version.
