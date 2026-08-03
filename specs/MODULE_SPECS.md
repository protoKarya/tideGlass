<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# tideGlass Module Specifications

**Version**: 0.1.0 — As-Built | **Standard**: TARGETED_GUIDESTONE_STANDARD v1.0

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

- Weighted Kolmogorov–Smirnov enrichment lives in `tideglass-core/src/enrichment.rs` (`weighted_ks_enrichment`, `compute_rges`, permutation p-values)
- Benjamini–Hochberg FDR correction and hit ranking in `tideglass-rges/src/screen.rs`
- End-to-end orchestration in `tideglass-rges/src/pipeline.rs` (`RgesPipeline::run`)
- IPC handler: `science.rges_screen` via UniBin dispatch
- barraCuda dispatch for large LINCS matrix operations remains a future integration point
- Provenance: each RGES screen creates a rhizoCrypt DAG session (planned NUCLEUS wiring)

### Crate structure

```
crates/tideglass-rges/
├── Cargo.toml
└── src/
    ├── lib.rs          # module entry, re-exports
    ├── pipeline.rs     # RgesPipeline orchestration
    └── screen.rs       # BH-FDR correction + ranked hit output
```

---

## Module 2: Representative Cell Line Selection (`tideglass-rcl`)

**Paper**: Xing 2026 — "Deep learning-based screening and design of novel therapeutics." *Cell*
**DOI**: 10.1016/j.cell.2026.02.023
**Claim**: RCL improves VCAP_t1 profile signal-to-noise vs raw LINCS
**Dependencies**: Module 1 (RGES scoring for per-compound evaluation)

### What it does

Selects the most representative cell line for a disease signature by ranking LINCS cell lines on signal-to-noise ratio (SNR) of absolute RGES across compounds. Higher SNR indicates a cell line where reversal signal is consistent and distinguishable from noise — the line best suited for downstream perturbation matching.

### Pipeline

```
1. Group perturbation signatures by cell line
2. For each cell line, compute RGES for every compound (Module 1 enrichment)
3. Compute mean(|RGES|) and std(|RGES|) per cell line
4. SNR = mean(|RGES|) / std(|RGES|)
5. Rank cell lines by SNR; return top representative line(s)
```

### Data requirements

| Source | Size | Format |
|--------|------|--------|
| LINCS L1000 Level 5 (VCAP_t1 subset) | ~2 GB | GCTx (HDF5) |
| GPS Platform artifact (Zenodo) | 1.5 GB | NumPy/pickle |

### Validation target

Compare cleaned vs raw VCAP_t1 profiles. Signal-to-noise ratio must improve by ≥ 15% (per published figure).

### Implementation notes

- SNR-based cell line ranking in `tideglass-rcl/src/selection.rs` (`rank_cell_lines`, `group_by_cell_line`)
- Uses core RGES enrichment — not deep-learning noise cleaning
- Configurable minimum compounds per line (`RclConfig::min_compounds_per_line`)
- IPC handler: `science.rcl_select` via UniBin dispatch
- Deep-learning RCL reproduction (PyTorch multi-network co-training) remains a future validation track

### Crate structure

```
crates/tideglass-rcl/
├── Cargo.toml
└── src/
    ├── lib.rs          # module entry, re-exports
    └── selection.rs    # SNR ranking + cell line grouping
```

---

## Module 3: Expression Prediction (`tideglass-gps4drug`)

**Paper**: Xing 2026 (*Cell*, GPS4Drug component)
**DOI**: 10.1016/j.cell.2026.02.023
**Claim**: GPS4Drug predicts expression from structure (R² on held-out test set)
**Dependencies**: Module 1 (RGES for validation); training data may use RCL-selected cell lines

### What it does

Given a chemical structure (SMILES string), predicts the induced gene expression changes. This enables virtual screening — predict how a novel compound would alter gene expression without running wet-lab experiments.

### Pipeline

```
1. Encode compound SMILES → molecular features (physicochemical descriptors + fingerprint)
2. Linear regression predictor: y = intercept + W · x
3. Output: predicted expression changes (landmark gene vector)
4. Compare predicted vs observed profiles (validation)
```

### Data requirements

| Source | Size | Format |
|--------|------|--------|
| LINCS perturbation profiles | 20 GB | GCTx (HDF5) |
| ChEMBL 37 (compound structures) | 15 GB | SMILES/SDF |
| GPS Platform model weights (Zenodo) | ~500 MB | PyTorch (reference) |

### Validation target

R² ≥ 0.75 on held-out test set (per published figure). Profile-level Pearson correlation ≥ 0.65.

### Implementation notes

- Linear regression predictor in `tideglass-gps4drug/src/prediction.rs` (`LinearRegressionPredictor`, `ExpressionPredictor` trait)
- Molecular feature extraction in `tideglass-gps4drug/src/features.rs` (`MolecularFeatures`, physicochemical descriptors)
- Not a deep-learning model — multivariate linear regression from hand-crafted features
- IPC handler: `science.gps4drug_predict` via UniBin dispatch
- Deep-learning GPS4Drug reproduction remains a future validation track

### Crate structure

```
crates/tideglass-gps4drug/
├── Cargo.toml
└── src/
    ├── lib.rs          # module entry, re-exports
    ├── features.rs     # molecular feature extraction
    └── prediction.rs   # linear regression predictor
```

---

## Module 4: Reversal Screening (`tideglass-screen`)

**Paper**: Xing 2026 (*Cell*, screening validation)
**DOI**: 10.1016/j.cell.2026.02.023
**Claim**: ZINC screening recovers known HCC actives (enrichment AUC)
**Dependencies**: Module 1 (RGES-ranked hits); Module 3 (optional GPS4Drug predictions for virtual compounds)

### What it does

Screens a large compound library (ZINC) for molecules that reverse a disease gene expression signature. Combines Module 1 (RGES scoring) with Module 3 (GPS4Drug predictions) for high-throughput virtual screening.

### Pipeline

```
1. Load compound library (ZINC subset) with SMILES and properties
2. Apply multi-criteria filters to RGES-ranked hits:
   a. Lipinski rule-of-five (MW, logP, HBD, HBA)
   b. Structural alert pattern matching (reactive/toxic motifs)
   c. RGES reversal strength and p-value / FDR thresholds
3. Return filtered, ranked compound list
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

### Implementation notes

- Compound library loader in `tideglass-screen/src/library.rs` (`CompoundLibrary`, `LipinskiConfig`)
- Multi-criteria filtering in `tideglass-screen/src/filter.rs` (`filter_ranked_hits`, `ScreenFilterConfig`)
- Filters: Lipinski rule-of-five, structural alert patterns, RGES strength, raw p-value, FDR-adjusted p-value
- IPC handler: `science.compound_screen` via UniBin dispatch

### Crate structure

```
crates/tideglass-screen/
├── Cargo.toml
└── src/
    ├── lib.rs          # module entry, re-exports
    ├── library.rs      # compound library loader + Lipinski config
    └── filter.rs       # Lipinski + structural alert + RGES/p-value filtering
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

### Implementation notes

- MCTS orchestrator in `tideglass-molsearch/src/search.rs` (`MctsSearch`, UCB1 selection, configurable iterations)
- Tree structure in `tideglass-molsearch/src/tree.rs` (`MctsNode`, path selection)
- Five action types in `tideglass-molsearch/src/action.rs`: `AddSubstituent`, `RemoveGroup`, `RingModification`, `ReplaceAtom` (default set of 5 actions)
- Configurable via `MctsConfig` (iterations, exploration constant, max depth, target potency)
- IPC handler: `science.mcts_optimize` via UniBin dispatch

### Crate structure

```
crates/tideglass-molsearch/
├── Cargo.toml
└── src/
    ├── lib.rs          # module entry, re-exports
    ├── tree.rs         # MCTS tree structure + UCB1 path selection
    ├── search.rs       # MCTS orchestrator (selection, expansion, rollout, backprop)
    └── action.rs       # molecular modification actions + default action set
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

### Implementation notes

- Classification metrics in `tideglass-octad/src/metrics.rs`: AUC (trapezoidal ROC), precision/recall, F1, concordance correlation
- Benchmark framework in `tideglass-octad/src/benchmark.rs` (`OctadComparison`, `BenchmarkResult`)
- Compares GPS-ranked compounds against OCTAD reference active lists
- IPC handler: `science.octad_benchmark` via UniBin dispatch

### Crate structure

```
crates/tideglass-octad/
├── Cargo.toml
└── src/
    ├── lib.rs          # module entry, re-exports
    ├── metrics.rs      # AUC, precision/recall, F1, concordance correlation
    └── benchmark.rs    # OCTAD comparison framework
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

### Implementation notes

- Tissue-weighted reversal scoring in `tideglass-nf/src/scoring.rs` (`compute_nf_scores`, `NfReversalScore`)
- Compartment geometry in `tideglass-nf/src/tissue.rs` (`TissueCompartment`, `GeneCompartmentMap`, `TissueWeight`)
- Extends core RGES with geometry-weighted enrichment using Anderson disorder analogy (hotSpring)
- IPC handler: `science.nf_score` via UniBin dispatch

### Crate structure

```
crates/tideglass-nf/
├── Cargo.toml
└── src/
    ├── lib.rs          # module entry, re-exports
    ├── tissue.rs       # tissue compartment geometry + gene assignments
    └── scoring.rs      # tissue-weighted reversal scoring
```
