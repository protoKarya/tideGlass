# tideGlass Architecture

**Version**: 0.1.0 | **Phase**: 0 (Archaeology → Reproduction) | **Gate**: westGate

---

## Overview

tideGlass is a sovereign rebuild of the GPS (Gene-Expression-based Platform for Screening) published in Cell 2026. It validates published drug repurposing claims by reproducing from primary data, then rebuilds sovereign infrastructure for independent screening and compound optimization.

tideGlass is a **protist** — an application composed *from* primals, not a primal itself. It runs as a NUCLEUS cell composition on westGate where all 519 GB of science data is local.

---

## Workspace Layout

```
protists/tideGlass/
├── Cargo.toml              # workspace root (edition 2024)
├── crates/
│   ├── tideglass-core/     # shared types, RGES scorer, data access
│   ├── tideglass-rcl/      # RCL noise cleaning (Phase 1)
│   ├── tideglass-gps4drug/ # Structure → expression prediction (Phase 1)
│   ├── tideglass-screen/   # Reversal screening (Phase 1)
│   ├── tideglass-molsearch/ # MCTS compound optimization (Phase 2)
│   ├── tideglass-octad/    # OCTAD parity validation (Phase 2)
│   └── tideglass-nf/       # NF extension — novel application (Phase 3)
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
- tideGlass domain node (order 12) with `science.rges_screen`, `science.gene_set_enrichment`, `data.fetch` capabilities
- petalTongue in live mode for visualization
- All data access via local westGate CAS (`content.get` / `content.put`)

---

## IPC Methods (planned)

tideGlass exposes its own JSON-RPC methods via UDS:

| Method | Description | Phase |
|--------|-------------|-------|
| `science.rges_screen` | Full RGES screen against LINCS L1000 | Phase 1 |
| `science.gene_set_enrichment` | Gene set enrichment analysis | Phase 1 |
| `science.rcl_clean` | RCL noise cleaning on expression profiles | Phase 1 |
| `science.gps4drug_predict` | Structure → expression prediction | Phase 1 |
| `science.reversal_screen` | Reversal screening against compound library | Phase 1 |
| `science.mcts_optimize` | MCTS compound optimization | Phase 2 |
| `science.octad_compare` | OCTAD parity benchmark | Phase 2 |
| `science.nf_screen` | NF1-driven tumor reversal screen | Phase 3 |
| `data.fetch` | Fetch data from westGate CAS by content hash | Phase 1 |
| `health.liveness` | Standard primal health probe | Phase 1 |
| `health.readiness` | Module readiness (which modules are operational) | Phase 1 |

---

## Build & Test

```bash
# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Run with NUCLEUS (on westGate)
biomeos deploy --graph graphs/cells/tideglass_cell.toml
```
