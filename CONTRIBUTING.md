<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# Contributing to tideGlass

Thank you for contributing to tideGlass, a sovereign drug repurposing platform
in the ecoPrimals ecosystem. This guide follows wateringHole standards for
gen5-native protists.

## Prerequisites

- Rust 2024 edition on stable toolchain (MSRV 1.87; see `rust-toolchain.toml`)
- `cargo fmt`, `cargo clippy`, `cargo test`, and `cargo deny` available locally

## Development workflow

```bash
# Format
cargo fmt --all

# Lint (pedantic + nursery, zero warnings)
cargo clippy --workspace --all-targets -- -W clippy::pedantic -W clippy::nursery -D warnings

# Test
cargo test --workspace --all-features

# Coverage
cargo llvm-cov --workspace --all-features

# Supply chain audit
cargo deny check
```

Every change must pass format, lint, test, and deny before submission.
Coverage reporting via `cargo llvm-cov` is recommended for module changes.

## Code standards

### Safety and error handling

- `#![forbid(unsafe_code)]` on all crates — no exceptions
- Use `Result` everywhere in library code; no `.unwrap()` or `.expect()` in production paths
- All public types derive `Serialize` and `Deserialize` where applicable (IPC and guideStone parity)

### Documentation and headers

- SPDX header required on every file as the first line:
  - Rust: `// SPDX-License-Identifier: AGPL-3.0-or-later`
  - Markdown: `<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->`
  - TOML / shell / gitignore: `# SPDX-License-Identifier: AGPL-3.0-or-later`
- No `TODO`, `FIXME`, or `HACK` in committed code — open an issue or fix it

### Architecture constraints

- **JSON-RPC over UDS** for all IPC — no HTTP, no REST between primals
- **No cross-primal compile dependencies** — compose at runtime via biomeOS capability discovery
- Module crates depend on `tideglass-core`; `tideglass-screen` also depends on `tideglass-rges` and `tideglass-gps4drug`; `tideglass-molsearch` depends on `tideglass-gps4drug`
- `tideglass-bin` (UniBin) composes all module crates at runtime via JSON-RPC dispatch
- Data access goes through biomeOS Neural API (`neural-api-*.sock`, prefix-glob
  discovery) which routes `content.get`/`content.put`/`content.query` to nestGate
  CAS on westGate, with automatic direct fallback. No hardcoded socket paths.

### File size

- No single file may exceed 1000 lines — split modules when approaching the limit

## Project structure

| Path | Purpose |
|------|---------|
| `crates/tideglass-core/` | Shared types, RGES scorer, error handling |
| `crates/tideglass-rges/` | Module 1 — RGES correlation |
| `crates/tideglass-rcl/` | Module 2 — RCL noise cleaning |
| `crates/tideglass-gps4drug/` | Module 3 — expression prediction |
| `crates/tideglass-screen/` | Module 4 — reversal screening |
| `crates/tideglass-molsearch/` | Module 5 — MCTS optimization |
| `crates/tideglass-octad/` | Module 6 — OCTAD parity |
| `crates/tideglass-nf/` | Module 7 — NF extension |
| `crates/tideglass-bin/` | UniBin — IPC server, dispatch, health, CLI |
| `validation/` | Future Python parity layer; Rust tests are the current baseline |
| `graphs/` | biomeOS deploy graphs |

## Validation path

Rust unit/integration tests are the current baseline. Python parity is a future validation layer.

1. Implement or extend behavior in the module crate with Rust unit and integration tests
2. Wire IPC handler in `tideglass-bin` and register the canonical method name
3. Attach provenance chain when NUCLEUS primals are available (rhizoCrypt → loamSpine → sweetGrass)
4. Add expected-output fixtures under `validation/expected/` when Python parity lands

## Commit style

- Short imperative subject (50 chars or fewer)
- Body explains **why**, not what
- Reference module numbers or paper claims when relevant

## License

Contributions are governed by the scyBorg triple license:

- **Code**: AGPL-3.0-or-later
- **Game mechanics / rules** (if any): ORC
- **Creative works / documentation**: CC-BY-SA-4.0

By contributing, you agree that your contributions are licensed under these terms.
