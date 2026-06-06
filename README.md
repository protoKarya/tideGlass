# tideGlass

**Cross-spring biological current parser — sovereign drug repurposing platform.**

| | |
|-|-|
| **Org** | [protoKarya](https://github.com/protoKarya) |
| **Ecosystem** | [ecoPrimals](https://github.com/sporeGarden/ecoPrimals) |
| **Origin** | GPS Platform — Bin Chen Lab, Michigan State University ([Cell 2026](https://www.cell.com/cell/fulltext/S0092-8674(26)00223-0)) |
| **Collaborator** | Andrea Gonzales (NF Data Portal) → Bin Chen (PI, offered) |
| **License** | AGPL-3.0-or-later |
| **Status** | Phase 0 — Archaeology |
| **Generation** | gen5-native (first `protoKarya` project) |

---

## What Is tideGlass?

tideGlass is a sovereign rebuild of the GPS (Gene-Expression-based Platform for
Screening) platform published in Cell (2026). It parses biological currents —
gene expression flows, protein dynamics, community metabolic shifts — through a
computational lens that screens, predicts, and optimizes therapeutic compounds.

GPS rebuild is the first lens ground into the glass. The platform will extend to
NF (neurofibromatosis) drug repurposing as its first novel application.

**We do not fork. We do not patch. We read the methods as a specification, fetch
raw data from primary sources, implement from the mathematics, train from scratch,
and compare our outputs against their claims.**

---

## Three Modules

### 1. RCL — Robust Collaborative Learning

Clean noisy drug-induced gene expression profiles from LINCS/L1000 using
multi-network co-training with fuzzy membership and forget-rate scheduling.

### 2. GPS4Drug — Structure → Expression → Screening

Given a chemical structure (SMILES), predict induced gene expression changes.
Screen compound libraries for molecules that *reverse* a disease transcriptomic
signature. Virtual screening of millions of compounds in hours.

### 3. MolSearch — Multi-Objective Compound Optimization

Monte Carlo Tree Search with Structure-Gene-Activity Relationships (SGAR) for
simultaneous optimization of potency, selectivity, drug-likeness, and synthetic
accessibility.

---

## Seven Science Modules (guideStone)

| Module | Binary | Paper | Claim |
|--------|--------|-------|-------|
| RGES correlation | `tideglass-rges` | Chen 2017 (Nat Commun) | RGES correlates with IC50 (r=0.52) |
| RCL noise cleaning | `tideglass-rcl` | Xing 2026 (Cell) | RCL improves VCAP_t1 signal-to-noise |
| Expression prediction | `tideglass-gps4drug` | Xing 2026 (Cell) | GPS4Drug predicts expression from structure |
| Reversal screening | `tideglass-screen` | Xing 2026 (Cell) | ZINC screening recovers known HCC actives |
| MCTS optimization | `tideglass-molsearch` | Xing 2026 (Cell) | MCTS optimizes HCC lead (IC50 4→0.5 µM) |
| OCTAD parity | `tideglass-octad` | Zeng 2021 (Nat Protocols) | GPS exceeds OCTAD repurposing for HCC |
| NF extension | `tideglass-nf` | Novel | GPS reversal scoring for NF1-driven tumors |

---

## Ecosystem Integration

tideGlass runs as a NUCLEUS composition. Primals provide capabilities via UDS:

| Primal | Role | Tier |
|--------|------|------|
| nestGate | Content-addressed data fetch (LINCS, ChEMBL, PubChem, ZINC) | 2 |
| barraCuda | WGSL shader dispatch (RGES batch scoring, MCTS evaluation) | 2 |
| toadStool | GPU streaming dispatch (RCL training, MCTS tree search) | 3 |
| rhizoCrypt | DAG provenance chain for each module execution | 3 |
| loamSpine | Certificate chain for model weight verification | 3 |
| sweetGrass | Attribution braid (6 papers, LINCS program, ChEMBL, NF Data Portal) | 3 |

Springs contribute validated science:

- **healthSpring** — Hill dose-response, MATRIX scoring, RGES enrichment, MCTS
- **wetSpring** — ChEMBL/PubChem/NCBI data fetch, GCTx/HDF5 parse, provenance
- **neuralSpring** — Deep learning patterns, multi-network architecture, SMILES encoding
- **hotSpring** — Anderson disorder analogy, effective dimension mapping

---

## Phases

| Phase | Scope | Target |
|-------|-------|--------|
| 0. Archaeology | Download Zenodo artifact, inventory, map dependencies | Week 1-2 |
| 1. Reproduce | Stand up each module, match published outputs | Week 2-4 |
| 2. Validate | Cross-validate against primary data sources | Week 4-6 |
| 3. Rebuild Sovereign | Modern Python → Rust where needed, NestGate data, BLAKE3 provenance | Week 6-12 |
| 4. Package | pseudoSpore + lithoSpore + NF extension + Bin Chen review | Week 12+ |

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
# Phase 0-2: Python validation (reproducing original)
pip install -r requirements.txt
python -m tideglass.validate

# Phase 3+: Rust sovereign modules
cargo build --release
./target/release/tideglass validate --all
```

---

## References

1. Xing et al. (2026). "Deep learning-based screening and design of novel therapeutics that reverse disease-associated transcriptional phenotype." *Cell*. [DOI](https://doi.org/10.1016/j.cell.2026.02.023)
2. Chen et al. (2017). "Reversal of cancer gene expression correlates with drug efficacy and predicts therapeutic targets." *Nature Communications* 8:16022.
3. Zeng et al. (2021). "OCTAD: an open workspace for virtually screening therapeutics targeting disease gene expression signatures." *Nature Protocols* 16:728-753.
4. Subramanian et al. (2017). "A Next Generation Connectivity Map." *Science* 358:eaal2159.

---

*"The motion is the reality. The glass is the instrument that parses it."*
