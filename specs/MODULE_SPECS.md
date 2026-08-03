# tideGlass Module Specifications

**Version**: 0.1.0 | **Standard**: TARGETED_GUIDESTONE_STANDARD v1.0

Each module validates a specific claim from the GPS paper chain. Modules are ordered by dependency — Module 1 (RGES) is the foundation, Modules 2-4 build on it, and Modules 5-7 extend.

---

## Module 1: RGES Correlation (`tideglass-rges`)

**Paper**: Chen 2017 — "Reversal of cancer gene expression correlates with drug efficacy and predicts therapeutic targets." *Nature Communications* 8:16022
**DOI**: 10.1038/ncomms16022
**Claim**: RGES correlates with IC50 (r=0.52, P=2.3e-3 in MCF7)
**Dependencies**: None (foundation module)

### What it does

Computes the Reversal Gene Expression Score (RGES) for a disease expression signature against LINCS L1000 perturbation profiles. A negative RGES indicates a drug that reverses the disease transcriptomic signature — a candidate for repurposing.

### Pipeline

```
1. Ingest disease expression profile (GEO/TCGA via nestGate CAS)
2. Extract up/down gene sets (fold-change + significance cutoffs)
3. Query LINCS L1000 perturbation signatures (20 GB on westGate)
4. Compute enrichment: overlap of disease-up in drug-down (and vice versa)
5. RGES = mean(up_reversal, down_reversal), normalized by gene set size
6. Permutation test for statistical significance (p-value)
7. Rank compounds by reversal strength
```

### Data requirements

| Source | westGate path | Size | Format |
|--------|--------------|------|--------|
| LINCS L1000 Level 5 | `data/drug_discovery/lincs_l1000/` | 20 GB | GCTx (HDF5) |
| GEO SOFT | `data/genomics/geo/` | 1.8 GB | SOFT/MINiML |
| TCGA Xena Hub | `data/genomics/tcga/` | 15 GB | TSV |
| ChEMBL 37 | `data/drug_discovery/chembl/` | 15 GB | SDF/SQLite |

### Validation target

Reproduce Table 1 from Chen 2017: RGES correlation with IC50 for 15 HCC compounds in MCF7 cells. Must achieve r ≥ 0.45 (published: r=0.52).

### Implementation notes

- `tideglass-core/src/rges.rs` has the scaffold (types + naive `compute_rges`)
- Permutation test (`p_value`) is currently stubbed — needs proper implementation
- Gene set enrichment currently uses simple overlap ratio — should implement weighted Kolmogorov-Smirnov (GSEA standard)
- barraCuda dispatch for large LINCS matrix operations (1.3M profiles × ~1000 landmark genes)
- Provenance: each RGES screen creates a rhizoCrypt DAG session

### Crate structure

```
crates/tideglass-rges/
├── Cargo.toml
└── src/
    ├── lib.rs          # module entry, IPC handlers
    ├── scorer.rs       # RGES computation (KS enrichment)
    ├── lincs.rs        # LINCS L1000 GCTx parser
    ├── disease.rs      # disease signature extraction (GEO/TCGA)
    ├── permutation.rs  # permutation test for p-values
    └── ranking.rs      # compound ranking + output formatting
```

---

## Module 2: RCL Noise Cleaning (`tideglass-rcl`)

**Paper**: Xing 2026 — "Deep learning-based screening and design of novel therapeutics." *Cell*
**DOI**: 10.1016/j.cell.2026.02.023
**Claim**: RCL improves VCAP_t1 profile signal-to-noise vs raw LINCS
**Dependencies**: None (parallel to Module 1)

### What it does

Robust Collaborative Learning (RCL) cleans noisy drug-induced gene expression profiles. LINCS L1000 data has biological and technical noise — RCL uses multi-network co-training with fuzzy membership and forget-rate scheduling to learn clean signal from noisy replicates.

### Pipeline

```
1. Load LINCS L1000 profiles for target cell line (e.g., VCAP_t1)
2. Initialize N teacher networks with different random seeds
3. Co-training loop:
   a. Each teacher scores samples by prediction confidence
   b. Fuzzy membership weights low-confidence samples down
   c. Forget-rate scheduler progressively excludes noisy samples
   d. Student network trains on consensus-weighted data
4. Output: cleaned expression profiles with noise scores per sample
```

### Data requirements

| Source | Size | Format |
|--------|------|--------|
| LINCS L1000 Level 5 (VCAP_t1 subset) | ~2 GB | GCTx (HDF5) |
| GPS Platform artifact (Zenodo) | 1.5 GB | NumPy/pickle |

### Validation target

Compare cleaned vs raw VCAP_t1 profiles. Signal-to-noise ratio must improve by ≥ 15% (per published figure).

### Implementation notes

- This is the most ML-heavy module — needs a training loop
- Phase 0-2: reproduce in Python with PyTorch (GPS used PyTorch)
- Phase 3: Rust implementation via barraCuda tensor ops + toadStool GPU dispatch
- neuralSpring provides multi-network architecture patterns
- Model weights get loamSpine certificate chain for verification

### Crate structure

```
crates/tideglass-rcl/
├── Cargo.toml
└── src/
    ├── lib.rs          # module entry
    ├── teacher.rs      # teacher network definition
    ├── student.rs      # student network (consensus learner)
    ├── membership.rs   # fuzzy membership computation
    ├── scheduler.rs    # forget-rate scheduling
    └── evaluate.rs     # signal-to-noise evaluation
```

---

## Module 3: Expression Prediction (`tideglass-gps4drug`)

**Paper**: Xing 2026 (*Cell*, GPS4Drug component)
**DOI**: 10.1016/j.cell.2026.02.023
**Claim**: GPS4Drug predicts expression from structure (R² on held-out test set)
**Dependencies**: Module 2 (RCL-cleaned profiles for training data)

### What it does

Given a chemical structure (SMILES string), predicts the induced gene expression changes. This enables virtual screening — predict how a novel compound would alter gene expression without running wet-lab experiments.

### Pipeline

```
1. Encode compound SMILES → learned molecular representation
2. Feed representation through GPS4Drug predictor network
3. Output: predicted expression changes (landmark gene vector)
4. Compare predicted vs RCL-cleaned observed profiles (validation)
```

### Data requirements

| Source | Size | Format |
|--------|------|--------|
| RCL-cleaned LINCS profiles | Output from Module 2 | Internal |
| ChEMBL 37 (compound structures) | 15 GB | SMILES/SDF |
| GPS Platform model weights (Zenodo) | ~500 MB | PyTorch |

### Validation target

R² ≥ 0.75 on held-out test set (per published figure). Profile-level Pearson correlation ≥ 0.65.

### Crate structure

```
crates/tideglass-gps4drug/
├── Cargo.toml
└── src/
    ├── lib.rs          # module entry
    ├── encoder.rs      # SMILES → molecular representation
    ├── predictor.rs    # expression prediction network
    ├── training.rs     # training loop (Phase 3)
    └── validate.rs     # held-out evaluation
```

---

## Module 4: Reversal Screening (`tideglass-screen`)

**Paper**: Xing 2026 (*Cell*, screening validation)
**DOI**: 10.1016/j.cell.2026.02.023
**Claim**: ZINC screening recovers known HCC actives (enrichment AUC)
**Dependencies**: Module 3 (GPS4Drug predictions for large-scale screening)

### What it does

Screens a large compound library (ZINC) for molecules that reverse a disease gene expression signature. Combines Module 1 (RGES scoring) with Module 3 (GPS4Drug predictions) for high-throughput virtual screening.

### Pipeline

```
1. Define disease signature (HCC from TCGA)
2. For each ZINC compound:
   a. Predict expression profile (Module 3 GPS4Drug)
   b. Compute RGES against disease signature (Module 1)
3. Rank compounds by RGES
4. Validate: check enrichment of known HCC actives in top-ranked
```

### Data requirements

| Source | Size | Format |
|--------|------|--------|
| ZINC20 (screened subset) | 244 MB | SMILES |
| TCGA HCC profiles | ~1 GB | TSV |
| Known HCC actives (ChEMBL) | ~10 MB | CSV |

### Validation target

Enrichment AUC ≥ 0.70 for recovery of known HCC actives from ZINC library.

### Crate structure

```
crates/tideglass-screen/
├── Cargo.toml
└── src/
    ├── lib.rs          # module entry
    ├── library.rs      # compound library loader (ZINC)
    ├── screener.rs     # batch screening orchestrator
    ├── enrichment.rs   # enrichment AUC calculation
    └── report.rs       # screening results + provenance
```

---

## Module 5: MCTS Optimization (`tideglass-molsearch`)

**Paper**: Xing 2026 (*Cell*, MolSearch component)
**DOI**: 10.1016/j.cell.2026.02.023
**Claim**: MCTS optimizes HCC lead: IC50 4µM → 0.5µM, selectivity maintained
**Dependencies**: Module 3 (GPS4Drug for evaluation function)

### What it does

Monte Carlo Tree Search (MCTS) with Structure-Gene-Activity Relationships (SGAR) for simultaneous optimization of potency, selectivity, drug-likeness, and synthetic accessibility. Given a lead compound, explores chemical modifications to improve the multi-objective score.

### Pipeline

```
1. Start from lead compound (HCC active, IC50 ~4µM)
2. MCTS tree expansion: enumerate valid chemical modifications
3. Evaluation: GPS4Drug prediction → RGES score → multi-objective
4. Selection: UCB1 policy with SGAR-weighted rollouts
5. Output: optimized compound(s) with predicted IC50 ≤ 1µM
```

### Data requirements

| Source | Size | Format |
|--------|------|--------|
| ChEMBL 37 (selectivity data) | 15 GB | SQLite |
| BindingDB (binding affinities) | 583 MB | TSV |
| GPS Platform MCTS config | ~50 MB | JSON/YAML |

### Validation target

From HCC lead (IC50 ~4µM), produce optimized compound with predicted IC50 ≤ 1µM and selectivity index ≥ 10.

### Crate structure

```
crates/tideglass-molsearch/
├── Cargo.toml
└── src/
    ├── lib.rs          # module entry
    ├── tree.rs         # MCTS tree structure
    ├── search.rs       # UCB1 selection + rollout
    ├── chemistry.rs    # chemical modification enumeration
    ├── objective.rs    # multi-objective evaluation (SGAR)
    └── synthesize.rs   # synthetic accessibility scoring
```

---

## Module 6: OCTAD Parity (`tideglass-octad`)

**Paper**: Zeng 2021 — "OCTAD: an open workspace for virtually screening therapeutics." *Nature Protocols* 16:728-753
**DOI**: 10.1038/s41596-020-00430-z
**Claim**: GPS screening exceeds OCTAD repurposing for HCC targets
**Dependencies**: Module 4 (reversal screening results for comparison)

### What it does

Benchmark GPS reversal screening against the OCTAD platform. Both platforms identify drug candidates for disease reversal — this module validates that GPS achieves superior enrichment.

### Validation target

GPS enrichment AUC must exceed OCTAD enrichment AUC for the same HCC target set. Published margin: GPS AUC ~0.78 vs OCTAD AUC ~0.65.

### Crate structure

```
crates/tideglass-octad/
├── Cargo.toml
└── src/
    ├── lib.rs          # module entry
    ├── octad.rs        # OCTAD pipeline reproduction
    ├── compare.rs      # GPS vs OCTAD head-to-head
    └── report.rs       # parity analysis output
```

---

## Module 7: NF Extension (`tideglass-nf`)

**Paper**: Novel — first NF application of GPS
**Claim**: GPS reversal scoring identifies candidates for NF1-driven tumors
**Dependencies**: Module 4 (reversal screening pipeline)

### What it does

Extends GPS reversal screening to neurofibromatosis type 1 (NF1). NF1 loss-of-function drives aberrant Ras signaling in Schwann cells and other tissues. This module applies GPS scoring to NF1-driven tumor transcriptomic signatures from the NF Data Portal to identify repurposing candidates.

This is the **novel scientific contribution** — the reason Andrea Gonzales initiated the collaboration.

### Data requirements

| Source | Size | Format |
|--------|------|--------|
| NF Data Portal | TBD | Expression matrices |
| NF1-related compounds (ChEMBL) | ~50 MB | CSV |

### Validation target

Generate candidate compound list for NF1-driven tumors. No published baseline — this is novel work. Validation by:
1. Literature cross-reference (are candidates known NF-relevant?)
2. BindingDB affinity filter (do candidates bind NF1-relevant targets?)
3. Selectivity screen (off-target profile)

### Crate structure

```
crates/tideglass-nf/
├── Cargo.toml
└── src/
    ├── lib.rs          # module entry
    ├── nf_signature.rs # NF1 disease signature extraction
    ├── screen.rs       # GPS reversal screen for NF signatures
    ├── affinity.rs     # BindingDB affinity filter
    └── candidates.rs   # candidate prioritization + report
```
