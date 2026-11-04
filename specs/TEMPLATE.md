# Spec template

Every subsystem spec in `specs/` has these sections, in this order. A spec states behaviour: what a
caller and the state block can see. It never describes how the original was written, gives no build
advice, and gives no reasons for the behaviour it states.

The kit is the data kit a spec is written against (`MANIFEST.json`, `layout.json`, `interfaces/`,
`docs/`, `tests/replay/`, `reference/`). A spec uses the kit's names (slots, services, `layout.json`
records and fields, replay line names) and no others. When the spec needs a word for something the
kit doesn't name, it defines the word in the spec. Observable data (saved-game keys, messages, file
and record names) is quoted exactly.

## 1. Role

What the subsystem is responsible for in the game, in one paragraph.

## 2. Interface

- The slots it provides, by their names in `interfaces/<subsystem>.h`, and when the engine calls each.
- **The manifest entry, in full:** `name` and `interface`, `exports` in order, `data` words, the kinds
  of every slot, and every import by exact group and name with its kinds (no wildcards: the lists are
  exact, ABI 9.1). Add what each import is used for.
- Fuel: the rule version and the budget the module must meet.
- Every argument that isn't in the block (a params record, a console command, a saved-game bank,
  a `TdfFile`): its layout, or where the kit gives it, and any field the module reads directly.

## 3. State

- The block fields it owns (writes and keeps between ticks), by `layout.json` record and field, with
  type and meaning.
- The block fields it reads that other subsystems own.
- The bits of any flags word it reads, when `layout.json` doesn't list them.
- What has to hold between ticks.

## 4. Behaviour

Start with one line: within one call, only the order of import calls is observable. The block writes
are compared at the end of the call, so the order they are listed in carries no meaning.

One subsection per slot: its inputs, then the effects seen outside it (state writes, import calls with
their arguments, the result). Effects are stated as results ("`X` becomes ..."), never as the
original's steps or helpers, and never as "behaves as another slot": state what that slot does.

- A slot whose effects depend on an import's result is a table of outcomes, each with its imports in
  order and what is written. It is never a list of steps with jumps.
- For every import whose result can mean failure, say whether the result is ignored, and what is still
  written.
- Say what a copy leaves in the rest of its destination, and how a stored pointer becomes a block
  offset (and what null stores).

## 5. Exactness

Everything an implementation has to reproduce bit for bit, collected in one place:

- random draws: which stream, how many, in what order, and how each value is scaled;
- integer arithmetic: width, wrapping, signed or unsigned comparison, sign extension of narrow fields,
  rounding of division and shifts, and negative inputs;
- floating point: the precision of each operation and each conversion, and what an out-of-range or NaN
  conversion gives without a trap;
- fixed tables and functions that the kit doesn't provide, defined as arithmetic.

## 6. Quirks

Each behaviour of 3.1c that looks wrong but that conformity requires, stated as behaviour, with an id
(`Q1`, `Q2`, ...) that the rest of the spec refers to.

## 7. Examples

Calls taken from the kit's recordings: "`tests/replay/<subsystem>/<file>`, call at tick N of `Slot`:
given these inputs, this happens", with enough numbers that an implementation can check a formula by
hand. Every example is recomputed from this spec's own definitions before review.

End with the slots and outcomes no recording covers.

## 8. Kit version

The `interfaces-v*` and `reference-traces-v*` pair the gate judges against, and the subsystem's
interface version.
