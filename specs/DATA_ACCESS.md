<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# tideGlass Data Access Specification

**Gate**: westGate | **Access**: local (no inter-gate mesh needed)

---

## Data Locality

tideGlass runs on westGate. All 519 GB of science data is on the same ZFS pool. Data access is **local** — `content.get` calls go directly to the local nestGate CAS, no songBird mesh traversal required.

This is why tideGlass is assigned to westGate and is a Phase 4 boot target (no mesh validation needed).

---

## Dataset Catalog (tideGlass-relevant)

All datasets are ingested with full 5-step provenance pipeline.

### Drug Discovery & Chemistry (48 GB total on westGate)

| Dataset | Size | Path | Provenance | Module(s) |
|---------|------|------|------------|-----------|
| **LINCS L1000** | 20 GB | `data/drug_discovery/lincs_l1000/` | BLAKE3 + DAG | 1 (RGES), 2 (RCL), 3 (GPS4Drug) |
| **ChEMBL 37** | 15 GB | `data/drug_discovery/chembl/` | BLAKE3 + DAG | 1, 4 (screening), 5 (MCTS) |
| **PubChem** | 11 GB | `data/drug_discovery/pubchem/` | BLAKE3 + DAG | 1, 4 |
| **GPS Platform (Zenodo)** | 1.5 GB | `data/drug_discovery/gps_platform/` | BLAKE3 + DAG | all |
| **BindingDB** | 583 MB | `data/drug_discovery/bindingdb/` | BLAKE3 + DAG | 5 (MCTS), 7 (NF) |
| **ZINC20** | 244 MB | `data/drug_discovery/zinc/` | BLAKE3 + DAG | 4 (screening) |
| **ChEBI** | 129 MB | `data/drug_discovery/chebi/` | BLAKE3 + DAG | 3 |
| **Every Cure MATRIX** | 51 MB | `data/drug_discovery/every_cure/` | BLAKE3 + DAG | 1 |

### Genomics (relevant to tideGlass)

| Dataset | Size | Path | Module(s) |
|---------|------|------|-----------|
| **TCGA Xena Hub** | 15 GB | `data/genomics/tcga/` | 1 (disease signatures), 4 (screening) |
| **GEO SOFT** | 1.8 GB | `data/genomics/geo/` | 1 (expression profiles) |

### Pending

| Dataset | Size (est.) | Status | Blocker | Module(s) |
|---------|-------------|--------|---------|-----------|
| **NF Data Portal** | TBD | Not downloaded | Registration required | 7 (NF extension) |
| **DisGeNET** | ~2 GB | Credentials registered | Download pending | 7 |
| **SRA FASTQ** | ~220 GB | Not started | Large download | future |

---

## Access Patterns

### Pattern 1: Batch Read (Modules 1, 4, 6)

```
tideGlass → nestGate.content.get(hash) → local ZFS → data
```

Load large datasets (LINCS, ChEMBL) in bulk at pipeline start. Cache in memory for the duration of the screen. This is the primary access pattern.

**Performance**: Local ZFS, no network. Expected throughput: disk-limited (~500 MB/s sequential on NVMe).

### Pattern 2: Streaming Read (Modules 2, 3)

```
tideGlass → nestGate.storage.retrieve(key) → stream chunks → process
```

For training loops (RCL, GPS4Drug), stream data in batches rather than loading all at once. 20 GB LINCS doesn't fit in memory for all cell lines simultaneously.

### Pattern 3: Write (all modules)

```
tideGlass result → nestGate.content.put(data) → CAS hash
                 → rhizoCrypt.dag.event.append(hash)
                 → loamSpine.entry.append(hash)
                 → sweetGrass.braid.commit(attribution)
```

Every pipeline output goes through the full provenance chain:
1. Content stored in nestGate CAS (content-addressed by BLAKE3)
2. Execution event appended to rhizoCrypt DAG session
3. Provenance entry appended to loamSpine ledger
4. Attribution braid committed to sweetGrass

### Pattern 4: GPU Dispatch (Modules 2, 3, 5)

```
tideGlass → toadStool.compute.dispatch(shader, data) → barraCuda GPU
         → result → tideGlass
```

For matrix operations, training, and MCTS evaluation. barraCuda provides FP64 tensor math on westGate's GPU (if available) or CPU fallback.

---

## nestGate CAS Integration

tideGlass talks to nestGate via JSON-RPC over UDS:

```rust
// Fetch data by content hash
let data = nestgate_client.call("content.get", json!({
    "hash": "blake3:abc123...",
    "format": "gctx"
})).await?;

// Store pipeline output
let hash = nestgate_client.call("content.put", json!({
    "data": rges_results,
    "metadata": {
        "module": "rges",
        "disease": "HCC",
        "timestamp": "2026-08-03T12:00:00Z"
    }
})).await?;
```

---

## Provenance Chain per Execution

Every module execution creates a provenance chain:

```
rhizoCrypt: dag.session.create("tideglass-rges-screen-{timestamp}")
  → dag.event.append("input", { disease: "HCC", lincs_hash: "..." })
  → dag.event.append("compute", { compounds_scored: 15000, duration_ms: 4200 })
  → dag.event.append("output", { result_hash: "blake3:...", top_10: [...] })
  → dag.merkle.root() → session_root_hash

loamSpine: entry.append(session_root_hash)
  → certificate.mint({ module: "rges", claim: "r=0.52", validated: true })

sweetGrass: braid.create(["Chen2017", "LINCS_program", "ChEMBL"])
  → braid.commit(session_root_hash, attribution_chain)
```
