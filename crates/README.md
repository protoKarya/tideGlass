<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# crates/ — Rust sovereign modules

Nine workspace crates implement the GPS platform rebuild. Seven are science
modules; one is shared core; one is the UniBin entry point that composes them
into a single JSON-RPC server over UDS.

## Workspace crates

| Crate | Role |
|-------|------|
| `tideglass-core` | Shared types, enrichment, IPC constants, error handling, capability discovery |
| `tideglass-rges` | Module 1: RGES batch scoring and ranked hit output (Chen 2017) |
| `tideglass-rcl` | Module 2: representative cell line selection and noise scoring (Xing 2026) |
| `tideglass-gps4drug` | Module 3: structure-to-expression prediction (Xing 2026) |
| `tideglass-screen` | Module 4: compound library reversal screening (Xing 2026) |
| `tideglass-molsearch` | Module 5: MCTS compound optimization (Xing 2026) |
| `tideglass-octad` | Module 6: OCTAD benchmark parity (Zeng 2021) |
| `tideglass-nf` | Module 7: network fragmentation scoring for NF extension (novel) |
| `tideglass-bin` | **UniBin entry point** — IPC server, JSON-RPC dispatch, health probes, CLI |

## Dependency model

- All module crates depend on `tideglass-core`.
- `tideglass-screen` also depends on `tideglass-rges` and `tideglass-gps4drug`.
- `tideglass-molsearch` depends on `tideglass-gps4drug`.
- `tideglass-bin` links all module crates and exposes seventeen JSON-RPC methods
  (`capabilities.list`, three health probes, seven science methods).
- Module crates do not compile against primal crates — composition with nestGate,
  barraCuda, and the provenance trio happens at runtime via biomeOS deploy graphs.

## Build

```bash
cargo build --release -p tideglass-bin
```

The release binary is named `tideglass` and serves as the sole IPC endpoint for
all seven science modules.
