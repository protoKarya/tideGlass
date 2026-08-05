<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# tideGlass Architecture

**Version**: 0.1.0 | **Phase**: 4 (Package) | **Gate**: westGate

---

## Overview

tideGlass is a sovereign rebuild of the GPS (Gene-Expression-based Platform for Screening) published in Cell 2026. It validates published drug repurposing claims by reproducing from primary data, then rebuilds sovereign infrastructure for independent screening and compound optimization.

tideGlass is a **protist** — an application composed *from* primals, not a primal itself. It runs as a NUCLEUS cell composition on westGate where all 519 GB of science data is local.

---

## Workspace Layout

```
gardens/tideGlass/
├── Cargo.toml              # workspace root (edition 2024)
├── crates/
│   ├── tideglass-core/     # Shared types, enrichment, error handling, IPC, discovery
│   ├── tideglass-bin/      # UniBin binary: UDS server, dispatch, health, CLI
│   ├── tideglass-rges/     # RGES pipeline + BH-FDR screening
│   ├── tideglass-rcl/      # Representative cell line SNR ranking
│   ├── tideglass-gps4drug/ # Structure-to-expression prediction
│   ├── tideglass-screen/   # Compound library + Lipinski/structural alert filters
│   ├── tideglass-molsearch/# MCTS molecular optimization
│   ├── tideglass-octad/    # Benchmark evaluation metrics
│   └── tideglass-nf/       # NF1 tissue-weighted reversal scoring
├── validation/             # Python reproduction of published claims
│   ├── notebooks/          # Tier 1 Python notebooks per module
│   └── expected/           # guideStone expected outputs
├── artifact/               # Zenodo data + reference outputs
├── graphs/                 # biomeOS cell/deploy TOMLs
├── shaders/                # WGSL shaders for barraCuda dispatch
├── specs/                  # ← this directory
├── scope.toml              # guideStone birth certificate
├── domain_profile.toml     # ecosystem domain classification
├── CONTEXT.md              # project context
└── README.md
```

---

## Composition Model

tideGlass composes from NUCLEUS primals via UDS IPC. No HTTP. No REST. All capabilities are discovered at runtime via biomeOS socket scanning.

### Primal Dependencies

| Primal | Tier | Required? | Capability | Usage |
|--------|------|-----------|------------|-------|
| **nestGate** | 2 | Yes | `content.get`, `content.put`, `storage.store`, `storage.retrieve` | CAS data fetch — LINCS, ChEMBL, PubChem, ZINC, GEO, TCGA. All 519 GB on local ZFS. |
| **barraCuda** | 2 | Yes | `tensor.matmul`, `tensor.create`, `stats.mean`, `linalg.solve` | RGES batch scoring matrices, GPS4Drug inference, MCTS evaluation. |
| **toadStool** | 3 | No | `compute.dispatch`, `compute.execute` | GPU streaming dispatch for RCL training, MCTS tree search. Graceful degradation to CPU. |
| **rhizoCrypt** | 3 | No | `dag.session.create`, `dag.event.append`, `dag.merkle.root` | Provenance DAG for each pipeline execution. |
| **loamSpine** | 3 | No | `spine.create`, `entry.append`, `certificate.mint` | Certificate chain for model weight verification. |
| **sweetGrass** | 3 | No | `braid.create`, `braid.commit`, `anchoring.anchor` | Attribution braid (6 papers, LINCS program, ChEMBL, NF Data Portal). |
| **squirrel** | 3 | No | `ai.inference` | Optional — NLP-assisted compound literature search. |
| **petalTongue** | 2 | Yes | `visualization.render`, `visualization.render.scene` | RGES volcano plots, screening dashboards, MCTS optimization traces. |

### Spring Dependencies (validated science consumed)

| Spring | What tideGlass consumes |
|--------|------------------------|
| **healthSpring** | Hill dose-response (IC50), MATRIX scoring, RGES enrichment, MCTS patterns, ChEMBL bioactivity pipeline, selectivity index |
| **wetSpring** | ChEMBL/PubChem/NCBI data fetch patterns, GCTx/HDF5 streaming parse, provenance trio patterns |
| **neuralSpring** | Deep learning training patterns, multi-network architecture (RCL), SMILES encoding |
| **hotSpring** | Anderson disorder analogy (tissue geometry), effective dimension mapping |

---

## Data Sources (all on westGate ZFS)

| Source | Size | Domain | Purpose |
|--------|------|--------|---------|
| **LINCS L1000** | 20 GB | Drug discovery | 1.3M drug perturbation expression profiles |
| **ChEMBL 37** | 15 GB | Drug discovery | Bioactivity panels (JAK, kinase selectivity) |
| **PubChem** | 11 GB | Drug discovery | Assay results |
| **TCGA Xena Hub** | 15 GB | Genomics | Cancer expression profiles |
| **GEO SOFT** | 1.8 GB | Genomics | Expression profiles |
| **BindingDB** | 583 MB | Drug discovery | Binding affinities |
| **GPS Platform** | 1.5 GB | Drug discovery | Gonzales/Bin Chen scoring artifacts (Zenodo) |
| **ZINC20** | 244 MB | Drug discovery | Screened compound subset |
| **ChEBI** | 129 MB | Drug discovery | Chemical ontology |
| **Every Cure MATRIX** | 51 MB | Drug discovery | Repurposing framework |
| **NF Data Portal** | TBD | NF discovery | NF1-driven tumor transcriptomic signatures |

All data has full 5-step provenance: BLAKE3 → nestGate → rhizoCrypt → loamSpine → bearDog → sweetGrass.

---

## Cell Graph

Deploy command:

```bash
biomeos deploy --graph graphs/cells/tideglass_cell.toml
```

The cell graph (`tideglass_cell.toml`) defines:
- Full NUCLEUS base (13 primals)
- tideGlass domain node (order 12) with all seven `science.*` IPC methods
- petalTongue in live mode for visualization
- All data access via local westGate CAS (`content.get` / `content.put`)

---

## Binary Model (UniBin)

tideGlass ships as a single **`tideglass` binary** (`tideglass-bin`). There are no per-module executables. All science modules compile as library crates linked into the UniBin.

The binary listens on a Unix domain socket (default: `/run/tideglass/tideglass.sock`) and accepts NDJSON-framed JSON-RPC 2.0 requests. A central dispatch router in `tideglass-bin/src/dispatch.rs` matches each request's `method` field to the appropriate library crate handler. Health and capability discovery endpoints are served from the same process.

CLI subcommands (`capabilities`, `health`, `serve`) support local inspection and orchestrator integration without starting the full UDS server.

---

## IPC Methods (implemented)

tideGlass exposes its own JSON-RPC methods via UDS:

| Method | Module | Description |
|--------|--------|-------------|
| `capabilities.list` | core | List all tideGlass JSON-RPC methods |
| `health.liveness` | bin | Liveness probe |
| `health.check` | bin | Health check with component status |
| `health.readiness` | bin | Readiness probe |
| `science.rges_screen` | rges | RGES compound screening against disease signature |
| `science.rcl_select` | rcl | Representative cell line selection |
| `science.gps4drug_predict` | gps4drug | Structure-to-expression prediction |
| `science.compound_screen` | screen | Compound library screening with filters |
| `science.mcts_optimize` | molsearch | MCTS-based molecular optimization |
| `science.octad_benchmark` | octad | OCTAD benchmark evaluation |
| `science.nf_score` | nf | NF1 tissue-weighted reversal scoring |

---

## Build & Test

```bash
cargo build --release
cargo test --workspace
cargo clippy --workspace --all-targets -- -W clippy::pedantic -W clippy::nursery -D warnings
cargo deny check
cargo llvm-cov --workspace --all-features
```

Deploy on westGate:

```bash
biomeos deploy --graph graphs/cells/tideglass_cell.toml
```

---

## Quality Metrics

| Metric | Value |
|--------|-------|
| Tests | 214 (`cargo test --workspace`) |
| Coverage | `cargo llvm-cov --workspace --all-features` (run locally for current numbers) |
| Unsafe code | `#![forbid(unsafe_code)]` on all workspace crates |
| Dependencies | Pure Rust — no C bindings or FFI in the workspace |
