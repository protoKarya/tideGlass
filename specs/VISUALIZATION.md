# tideGlass Visualization Specification

**Renderer**: petalTongue (live mode) | **GPU**: westGate (if GPU available) or CPU fallback

---

## Visualization Targets

Each module produces specific visualizations via petalTongue `visualization.render.scene` / `visualization.render.stream`.

### Module 1: RGES

| Visual | Type | Data | Priority |
|--------|------|------|----------|
| **RGES Volcano Plot** | Scatter | RGES score (x) vs -log10(p) (y) | P0 — core output |
| **IC50 Correlation** | Scatter + regression | RGES vs IC50 for validation set | P0 — Table 1 reproduction |
| **Top Compounds Heatmap** | Heatmap | Expression profiles of top 20 reversers | P1 |
| **Gene Set Overlap** | Venn/UpSet | Disease vs drug gene set intersections | P2 |

### Module 2: RCL

| Visual | Type | Data | Priority |
|--------|------|------|----------|
| **Noise Reduction** | Before/after scatter | Raw vs cleaned expression correlation | P0 |
| **Training Convergence** | Line | Loss over epochs for teacher/student | P1 |
| **Membership Weights** | Histogram | Fuzzy membership distribution | P2 |

### Module 3: GPS4Drug

| Visual | Type | Data | Priority |
|--------|------|------|----------|
| **Prediction Accuracy** | Scatter | Predicted vs observed expression | P0 |
| **Molecular Embedding** | t-SNE/UMAP | Compound embedding space | P1 |

### Module 4: Screening

| Visual | Type | Data | Priority |
|--------|------|------|----------|
| **Enrichment Curve** | ROC-style | Cumulative recovery of known actives | P0 |
| **Compound Ranking** | Bar chart | Top 50 candidates with RGES scores | P1 |

### Module 5: MCTS

| Visual | Type | Data | Priority |
|--------|------|------|----------|
| **Optimization Trace** | Multi-line | IC50, selectivity, drug-likeness over MCTS iterations | P0 |
| **Chemical Space Tree** | Tree graph | MCTS exploration tree (top branches) | P1 |

### Module 7: NF Extension

| Visual | Type | Data | Priority |
|--------|------|------|----------|
| **NF Candidate Dashboard** | Table + sparklines | Ranked NF candidates with scores | P0 |
| **NF vs HCC Comparison** | Dual volcano | Side-by-side RGES for NF and HCC | P1 |

---

## petalTongue Integration

tideGlass renders via petalTongue IPC:

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
