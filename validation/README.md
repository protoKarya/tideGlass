<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# validation/ — Reference outputs and future Python parity

Python-first reproduction of GPS platform claims is **deferred**. The as-built
system validates claims through Rust unit and integration tests in the workspace
crates and `tideglass-bin` dispatch handlers.

## Current validation baseline

- **147 Rust tests** across all nine workspace crates
- **92.71% line coverage** (`cargo llvm-cov --workspace --all-features`)

Run the full suite locally:

```bash
cargo test --workspace --all-features
cargo llvm-cov --workspace --all-features
```

## Future milestones

- **Python parity layer** — notebooks under `notebooks/` will reproduce published
  claims from primary data; Rust modules will be checked against those outputs.
- **Expected output fixtures** — `validation/expected/` will hold JSON reference
  files (one per module) for guideStone and cell deploy validation gates.

Until those land, module correctness is established by in-crate tests and IPC
handler integration tests in `tideglass-bin`.
