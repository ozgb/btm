# The module gate

A pull request that adds or changes a module is judged by a host-side gate. This page is the
contract between the pull request and the gate.

## Naming the module

A pull request names its module with a file `gate.toml` at the repository root:

```toml
crate = "path/to/a/cargo/crate"
subsystem = "wind_meteors"
```

`crate` is a relative path to a Cargo crate directory with a committed `Cargo.lock`. `subsystem`
is the name of the subsystem the module implements.

## The artifact

The `module` workflow runs on every pull request, on `ubuntu-latest`, with `contents: read` and no
secrets. It builds the crate for `wasm32-unknown-unknown` in release mode with the link options
the module ABI requires for the subsystem (fixed data base and table base, imported memory and
table, no stack first, no entry), applies the fuel rule of the module ABI to the linked module with
`tools/meter.py`, and uploads one artifact named `module` with exactly two files
at its root:

- `module.wasm`: the one `.wasm` the build made, metered.
- `subsystem`: one line, the subsystem name.

## The verdict

The workflow does not judge anything. A gate on a separate host downloads the artifact, plays it
against the reference traces and posts only its verdict on the pull request: equal, differs,
failed or refused, with the first divergence for a difference.

## Eligibility

The gate judges an artifact only when the pull request's author has write access to the
repository, or the pull request carries the `gate` label. Only the repository owner adds that
label.
