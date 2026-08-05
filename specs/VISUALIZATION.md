<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# tideGlass Visualization Specification

**Renderer**: petalTongue (live mode) | **GPU**: westGate (if GPU available) or CPU fallback

**Implementation status**: P0 scenes implemented in `tideglass-bin/src/scenes.rs` with JSON-RPC dispatch via `visualization.*` methods. petalTongue client (`petaltongue.rs`) wired for socket discovery. Scene JSON is returned directly from dispatch handlers; forwarding to petalTongue render pipeline awaits co-deployment on westGate.

---

## Visualization Targets

Each module produces specific visualizations via petalTongue `visualization.render.scene` / `visualization.render.stream`.

### Module 1: RGES

| Visual | Type | Data | Priority | Status |
|--------|------|------|----------|--------|
| **RGES Volcano Plot** | Scatter | RGES score (x) vs -log10(p) (y) | P0 — core output | **Implemented** (`visualization.rges_volcano`) |
| **IC50 Correlation** | Scatter + regression | RGES vs IC50 for validation set | P0 — Table 1 reproduction | Awaiting Chen 2017 benchmark run |
| **Top Compounds Heatmap** | Heatmap | Expression profiles of top 20 reversers | P1 | Not started |
| **Gene Set Overlap** | Venn/UpSet | Disease vs drug gene set intersections | P2 | Not started |

### Module 2: RCL

| Visual | Type | Data | Priority | Status |
|--------|------|------|----------|--------|
| **Noise Reduction** | Before/after scatter | Raw vs cleaned expression correlation | P0 | Not started |
| **Training Convergence** | Line | Loss over epochs for teacher/student | P1 | Not started |
| **Membership Weights** | Histogram | Fuzzy membership distribution | P2 | Not started |

### Module 3: GPS4Drug

| Visual | Type | Data | Priority | Status |
|--------|------|------|----------|--------|
| **Prediction Accuracy** | Scatter | Predicted vs observed expression | P0 | **Implemented** (`visualization.gps4drug_scatter`) |
| **Molecular Embedding** | t-SNE/UMAP | Compound embedding space | P1 | Not started |

### Module 4: Screening

| Visual | Type | Data | Priority | Status |
|--------|------|------|----------|--------|
| **Enrichment Curve** | ROC-style | Cumulative recovery of known actives | P0 | **Implemented** (`visualization.enrichment_curve`) |
| **Compound Ranking** | Bar chart | Top 50 candidates with RGES scores | P1 | Not started |

### Module 5: MCTS

| Visual | Type | Data | Priority | Status |
|--------|------|------|----------|--------|
| **Optimization Trace** | Multi-line | IC50, selectivity, drug-likeness over MCTS iterations | P0 | **Implemented** (`visualization.mcts_trace`) |
| **Chemical Space Tree** | Tree graph | MCTS exploration tree (top branches) | P1 | Not started |

### Module 7: NF Extension

| Visual | Type | Data | Priority | Status |
|--------|------|------|----------|--------|
| **NF Candidate Dashboard** | Table + sparklines | Ranked NF candidates with scores | P0 | **Implemented** (`visualization.nf_dashboard`) |
| **NF vs HCC Comparison** | Dual volcano | Side-by-side RGES for NF and HCC | P1 | Not started |

### Data Infrastructure

| Visual | Type | Data | Priority | Status |
|--------|------|------|----------|--------|
| **Data Catalog** | Interactive table | CAS dataset inventory and load status | P0 | **Implemented** (`data.catalog`) |

---

## IPC Methods

| Method | Scene | Module |
|--------|-------|--------|
| `visualization.rges_volcano` | RGES volcano plot | RGES |
| `visualization.enrichment_curve` | Enrichment curve | Screening |
| `visualization.nf_dashboard` | NF candidate dashboard | NF Extension |
| `visualization.gps4drug_scatter` | Prediction accuracy scatter | GPS4Drug |
| `visualization.mcts_trace` | Optimization trace | MCTS |
| `data.catalog` | CAS data catalog | Infrastructure |

---

## petalTongue Integration

tideGlass renders via petalTongue IPC. The `PetalTongueClient` in `tideglass-bin/src/petaltongue.rs` discovers petalTongue sockets via Neural API `membrane/` scan or direct socket. Scene JSON follows the declarative format:

```rust
petaltongue_client.call("visualization.render.scene", json!({
    "scene": "rges_volcano",
    "data": {
        "points": rges_results.iter().map(|r| {
            json!({ "x": r.rges_score, "y": -r.p_value.log10(), "label": r.compound_id })
        }).collect::<Vec<_>>(),
        "highlight": top_10_compounds,
    },
    "format": "webgl",
    "interactive": true,
})).await?;
```

All visualization scenes are declarative JSON. petalTongue handles the WebGL/WASM rendering. tideGlass never touches GPU for visualization — that's petalTongue's domain.

**Current path**: Visualization dispatch handlers return scene JSON directly via JSON-RPC. When petalTongue is co-deployed on westGate via biomeOS cell boot, the `PetalTongueClient` will forward scenes for live WebGL rendering at `tideglass.primals.eco`.
