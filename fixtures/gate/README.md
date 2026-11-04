# Gate fixtures

These are gate fixtures: standing draft pull requests that test the gate's posting and
eligibility (see `docs/gate.md`). They are not engine code and are never merged.

Each fixture is a small crate that a pull request names in `gate.toml`.

- `wind-meteors-const`: a `wind_meteors` module whose every slot does nothing. It loads, so the
  gate reaches the trace comparison, and it differs from 3.1c.
