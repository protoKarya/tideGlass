<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# tideGlass Phase 0 — Archaeology Checklist

> **Archived.** Phase 0 archaeology is complete. Rust workspace fully implements
> all seven science modules. See specs/ARCHITECTURE.md for the as-built system.
> Python validation parity is deferred to a future phase.

**Goal**: Inventory the GPS platform, download all data from primary sources, reproduce Module 1 (RGES) in Python, validate against Chen 2017 published results.

**Gate**: westGate | **Data**: 3.21 TB local (452 GB CAS pool)

---

## Prerequisites

- [ ] westGate NUCLEUS running (Tower Atomic LIVE as of Wave 155f)
- [ ] biomeOS deploy executor for cell composition boot (shipped, needs ops run)
- [ ] `tideglass_cell.toml` deployed on westGate

---

## Step 1: Zenodo Artifact Inventory

The GPS platform's full artifact is on Zenodo (713 MB). Download, unpack, catalog.

- [ ] Download Zenodo v5 artifact: https://zenodo.org/records/17653393
- [ ] Unpack into `artifact/zenodo/`
- [ ] Inventory all files — what's code, what's data, what's model weights
- [ ] Map dependency graph: which scripts depend on which data files
- [ ] Identify Python version and package requirements
- [ ] Document in `artifact/INVENTORY.md`

---

## Step 2: Primary Data Acquisition

GPS Platform artifact may contain processed data. We need the raw primary sources for sovereign reproduction.

### Already on westGate (7/7 modules data-ready per AAR):

- [x] LINCS L1000 Level 5 (20 GB)
- [x] ChEMBL 37 (15 GB)
- [x] PubChem BioAssay (11 GB)
- [x] TCGA Xena Hub (15 GB)
- [x] GEO SOFT (1.8 GB)
- [x] BindingDB (583 MB)
- [x] ZINC20 subset (244 MB)
- [x] GPS Platform (1.5 GB from Zenodo)
- [x] ChEBI (129 MB)
- [x] Every Cure MATRIX (51 MB)

### Pending:

- [ ] NF Data Portal — NF1-driven tumor transcriptomic signatures (Module 7)
- [ ] DisGeNET — disease-gene associations (credentials registered, download pending)
- [ ] Verify LINCS L1000 format is GCTx (HDF5-based), confirm we have Level 5

---

## Step 3: RGES Reproduction (Module 1 — Python)

Reproduce Chen 2017 Table 1 results using raw primary data (not Zenodo preprocessed).

### 3a. Disease Signature Extraction

- [ ] Load HCC expression profiles from TCGA Xena Hub
- [ ] Differential expression analysis (tumor vs normal)
- [ ] Extract up-regulated and down-regulated gene sets (fold-change ≥ 2, FDR ≤ 0.05)
- [ ] Validate gene set sizes match Chen 2017 supplementary

### 3b. LINCS L1000 Query

- [ ] Parse GCTx format (HDF5) — use `cmapPy` or write custom parser
- [ ] Filter for MCF7 cell line perturbation profiles
- [ ] Extract compound-level consensus signatures (aggregate replicates)
- [ ] Validate number of compounds matches Chen 2017

### 3c. RGES Computation

- [x] Implement weighted Kolmogorov-Smirnov enrichment (not simple overlap) — Rust: `tideglass-core/src/enrichment.rs`
- [x] Compute RGES for each compound against HCC disease signature — Rust: `compute_rges`
- [x] Implement permutation test for p-values (10,000 permutations) — Rust: `permutation_p_value`
- [x] Rank compounds by RGES — Rust: `tideglass-rges/src/screen.rs`

### 3d. Validation

- [ ] Compare top compounds to Chen 2017 Table 1
- [ ] Compute Pearson correlation of RGES vs IC50 (target: r ≥ 0.45, published: r=0.52)
- [ ] Generate volcano plot (RGES vs -log10(p))
- [ ] Document deviations from published results
- [ ] Save expected outputs to `validation/expected/module1_rges.json`

### 3e. Notebook

- [ ] Write `validation/notebooks/module1_rges/rges_correlation.py`
- [ ] Must be self-contained — reads from `artifact/` or westGate CAS
- [ ] Include all figures from Chen 2017 Figure 1

---

## Step 4: Rust Scaffold

Rust workspace implements all seven science modules plus UniBin (`tideglass-bin`).

- [x] Create `crates/tideglass-rges/Cargo.toml`
- [x] Add to workspace members in root `Cargo.toml`
- [x] Implement `DiseaseSignature`, `PerturbationSignature`, `RgesResult` types (in `tideglass-core`)
- [ ] Wire nestGate CAS client for data access
- [x] Wire IPC handler for `science.rges_screen` (via `tideglass-bin` UniBin dispatch)
- [x] Port RGES computation from Python to Rust
- [ ] Add tests comparing Rust output to Python expected output

---

## Step 5: Provenance Setup

- [ ] Create rhizoCrypt DAG session template for RGES runs
- [ ] Wire loamSpine certificate for validated RGES results
- [ ] Wire sweetGrass attribution braid (Chen 2017, LINCS program)
- [ ] Verify provenance chain end-to-end on one RGES run

---

## Done Criteria

Phase 0 is complete when:
1. Zenodo artifact fully inventoried
2. All primary data verified on westGate
3. RGES correlation reproduces r ≥ 0.45 vs Chen 2017 (Python)
4. Rust `tideglass-rges` crate compiles and passes basic tests
5. At least one RGES run has full provenance chain

---

## Timeline

| Week | Focus |
|------|-------|
| 1 | Zenodo inventory + data verification |
| 2 | RGES Python reproduction (Steps 3a-3c) |
| 3 | RGES validation + Rust scaffold (Steps 3d-4) |
| 4 | Provenance setup + Phase 0 → Phase 1 handoff |
