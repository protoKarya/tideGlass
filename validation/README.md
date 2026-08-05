<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# validation/ — Reference outputs and future Python parity

The as-built system validates claims through Rust unit and integration tests
across the workspace crates and `tideglass-bin` dispatch handlers.

## Current validation baseline

- **214 Rust tests** across all nine workspace crates
- Zero clippy warnings (pedantic + nursery)
- `cargo deny` clean (advisories, bans, licenses, sources)

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
cargo deny check
```

## Future milestones

- **Expected output fixtures** — `validation/expected/` will hold JSON reference
  files (one per module) for guideStone and cell deploy validation gates.
- **Chen 2017 benchmark** — RGES correlation target (r >= 0.52) against
  published compound rankings. Requires GPS data JSON conversion.
- **Python parity layer** — deferred. If needed, notebooks under `notebooks/`
  will reproduce published claims from primary data.

Until those land, module correctness is established by in-crate tests and IPC
handler integration tests in `tideglass-bin`.
