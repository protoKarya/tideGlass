<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# tideGlass

**Cross-spring biological current parser — sovereign drug repurposing platform.**

| | |
|-|-|
| **Org** | [protoKarya](https://github.com/protoKarya) |
| **Ecosystem** | [ecoPrimals](https://github.com/sporeGarden/ecoPrimals) |
| **Origin** | GPS Platform — Bin Chen Lab, Michigan State University ([Cell 2026](https://www.cell.com/cell/fulltext/S0092-8674(26)00223-0)) |
| **Collaborator** | Andrea Gonzales (NF Data Portal) → Bin Chen (PI, offered) |
| **License** | AGPL-3.0-or-later + ORC + CC-BY-SA 4.0 (ScyBorg triple license) |
| **Status** | Phase 4 — Package |
| **Generation** | gen5-native (first `protoKarya` project) |

---

## What Is tideGlass?

tideGlass is a sovereign rebuild of the GPS (Gene-Expression-based Platform for
Screening) platform published in Cell (2026). It parses biological currents —
gene expression flows, protein dynamics, community metabolic shifts — through a
computational lens that screens, predicts, and optimizes therapeutic compounds.

GPS rebuild is the first lens ground into the glass. The platform will extend to
NF (neurofibromatosis) drug repurposing as its first novel application.

The as-built system is a pure-Rust workspace: nine crates, a single `tideglass`
UniBin binary, and a UDS JSON-RPC 2.0 server exposing seventeen IPC methods —
seven science modules, five petalTongue visualization scenes, a data catalog,
and the health/capabilities triad.

**We do not fork. We do not patch. We read the methods as a specification, fetch
raw data from primary sources, implement from the mathematics, train from scratch,
and compare our outputs against their claims.**

---

## Seven Science Modules

| Module | Crate | Binary | Paper | Claim |
|--------|-------|--------|-------|-------|
| RGES correlation | `tideglass-rges` | `tideglass (UniBin)` | Chen 2017 (Nat Commun) | RGES correlates with IC50 (r=0.52) |
| RCL noise cleaning | `tideglass-rcl` | `tideglass (UniBin)` | Xing 2026 (Cell) | RCL improves VCAP_t1 signal-to-noise |
| Expression prediction | `tideglass-gps4drug` | `tideglass (UniBin)` | Xing 2026 (Cell) | GPS4Drug predicts expression from structure |
| Reversal screening | `tideglass-screen` | `tideglass (UniBin)` | Xing 2026 (Cell) | ZINC screening recovers known HCC actives |
| MCTS optimization | `tideglass-molsearch` | `tideglass (UniBin)` | Xing 2026 (Cell) | MCTS optimizes HCC lead (IC50 4→0.5 µM) |
| OCTAD parity | `tideglass-octad` | `tideglass (UniBin)` | Zeng 2021 (Nat Protocols) | GPS exceeds OCTAD repurposing for HCC |
| NF extension | `tideglass-nf` | `tideglass (UniBin)` | Novel | GPS reversal scoring for NF1-driven tumors |

---

## Ecosystem Integration

tideGlass runs as a biomeOS NUCLEUS composition on westGate (3.21 TB / 452 GB CAS pool).
CAS requests route through the Neural API (`neural-api-*.sock`, prefix-glob
discovery) for capability-based routing — no hardcoded primal socket paths.

| Capability | Provider | How |
|------------|----------|-----|
| Content-addressed data | nestGate (via Neural API) | `content.get`, `content.put` |
| GPU compute | barraCuda → toadStool (via Neural API) | `compute.dispatch` |
| Provenance chain | rhizoCrypt → loamSpine → sweetGrass (via Neural API) | `dag.event.append`, `entry.append`, `braid.commit` |

---

## Phases

| Phase | Scope | Status |
|-------|-------|--------|
| 0. Archaeology | Download Zenodo artifact, inventory, map dependencies | Complete |
| 1. Reproduce | Stand up each module, match published outputs | Complete |
| 2. Validate | Cross-validate against primary data sources | Complete |
| 3. Rebuild Sovereign | Rust workspace, NestGate data, BLAKE3 provenance | Complete |
| 4. Package | pseudoSpore + lithoSpore + NF extension + Bin Chen review | **Current** |

---

## Data Sources

| Source | Size | Purpose |
|--------|------|---------|
| [Zenodo v5](https://zenodo.org/records/17653393) | 713 MB | Full GPS platform artifact |
| LINCS L1000 Level 5 | ~4 GB | Drug perturbation expression profiles |
| ChEMBL (via API) | Variable | Bioactivity data for compound panels |
| ZINC (screened subset) | Variable | Compound library for virtual screening |
| NF Data Portal | Variable | NF1-driven tumor transcriptomic signatures |

---

## Building

```bash
cargo build --release
cargo test --workspace
cargo clippy --workspace --all-targets -- -W clippy::pedantic -W clippy::nursery -D warnings
cargo deny check
./target/release/tideglass version
./target/release/tideglass capabilities
./target/release/tideglass run [--socket <path>]
```

---

## Quality

| Metric | Value |
|--------|-------|
| Tests | 220 |
| IPC methods | 17 (7 science + 5 viz + 1 catalog + 4 infra) |
| Clippy | pedantic + nursery clean |
| cargo deny | clean |
| Unsafe code | `#![forbid(unsafe_code)]` on all crates |
| CAS routing | G56 Neural API (prefix-glob discovery, direct fallback) |
| Convergence gate | `is_dataset_converged()` for mixed-state data |

---

## References

1. Xing et al. (2026). "Deep learning-based screening and design of novel therapeutics that reverse disease-associated transcriptional phenotype." *Cell*. [DOI](https://doi.org/10.1016/j.cell.2026.02.023)
2. Chen et al. (2017). "Reversal of cancer gene expression correlates with drug efficacy and predicts therapeutic targets." *Nature Communications* 8:16022.
3. Zeng et al. (2021). "OCTAD: an open workspace for virtually screening therapeutics targeting disease gene expression signatures." *Nature Protocols* 16:728-753.
4. Subramanian et al. (2017). "A Next Generation Connectivity Map." *Science* 358:eaal2159.

---

*"The motion is the reality. The glass is the instrument that parses it."*
