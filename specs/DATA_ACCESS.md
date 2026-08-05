<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# tideGlass Data Access Specification

**Gate**: westGate | **Access**: local (no inter-gate mesh needed)

---

## Data Locality

tideGlass runs on westGate. All 519 GB of science data is on the same ZFS pool.
Data access routes through the **biomeOS Neural API** (`neural-api-default.sock`)
which capability-routes `content.get`/`content.put` to the local nestGate CAS.
No songBird mesh traversal required.

This is why tideGlass is assigned to westGate and is a Phase 4 boot target (no
mesh validation needed).

---

## CAS Routing (G56 Neural API Pattern)

tideGlass does **not** connect directly to `nestgate.sock`. All CAS requests
route through biomeOS Neural API for capability-based discovery:

```
tideGlass → neural-api-*.sock → biomeOS capability routing → nestgate CAS → ZFS
         ↘ (fallback) nestgate-*.sock → direct CAS
```

**Socket discovery priority** (prefix-glob scan):
1. `NEURAL_API_SOCKET` env var (explicit override)
2. `$XDG_RUNTIME_DIR/membrane/neural-api-*.sock` (Neural API, family-ID naming)
3. `$XDG_RUNTIME_DIR/biomeos/neural-api-*.sock` (alternate dir)
4. `NESTGATE_SOCKET` env var (direct fallback, bypasses routing)
5. `$XDG_RUNTIME_DIR/membrane/nestgate-*.sock` (direct, family-ID naming)
6. `$XDG_RUNTIME_DIR/biomeos/nestgate-*.sock` (alternate dir)
7. `/run/membrane/nestgate*.sock` (system membrane fallback)

Live NUCLEUS uses `$XDG_RUNTIME_DIR/membrane/` with family-ID-suffixed names
(e.g., `neural-api-westgate-tower-155f.sock`). Discovery uses prefix-glob rather
than fixed filenames. Neural API → direct nestGate fallback is automatic when
Neural API doesn't proxy `content.*` methods (DIV-8).

---

## Dataset Catalog (tideGlass-relevant)

All datasets are ingested with BLAKE3 content-addressing. Provenance state
varies — see Provenance Convergence section below.

### Drug Discovery & Chemistry (48 GB total on westGate)

| Dataset | Size | Path | Provenance | Module(s) |
|---------|------|------|------------|-----------|
| **LINCS L1000** | 20 GB | `data/drug_discovery/lincs_l1000/` | CAS-indexed | 1 (RGES), 2 (RCL), 3 (GPS4Drug) |
| **ChEMBL 37** | 15 GB | `data/drug_discovery/chembl/` | CAS-indexed | 1, 4 (screening), 5 (MCTS) |
| **PubChem** | 11 GB | `data/drug_discovery/pubchem/` | CAS-indexed | 1, 4 |
| **GPS Platform (Zenodo)** | 1.4 GB | `data/drug_discovery/gps_platform/` | CAS-indexed (NumPy/pickle — needs JSON conversion) | all |
| **BindingDB** | 583 MB | `data/drug_discovery/bindingdb/` | CAS-indexed | 5 (MCTS), 7 (NF) |
| **ZINC20** | 244 MB | `data/drug_discovery/zinc/` | CAS-indexed | 4 (screening) |
| **ChEBI** | 129 MB | `data/drug_discovery/chebi/` | CAS-indexed | 3 |
| **Every Cure MATRIX** | 51 MB | `data/drug_discovery/every_cure/` | CAS-indexed | 1 |

### Genomics (relevant to tideGlass)

| Dataset | Size | Path | Module(s) |
|---------|------|------|-----------|
| **TCGA Xena Hub** | 15 GB | `data/genomics/tcga/` | 1 (disease signatures), 4 (screening) |
| **GEO SOFT** | 1.8 GB | `data/genomics/geo/` | 1 (expression profiles) |

### Pending

| Dataset | Size (est.) | Status | Blocker | Module(s) |
|---------|-------------|--------|---------|-----------|
| **NF Data Portal** | ~220/658 files ingested | CAS ingest in progress | — | 7 (NF extension) |
| **DisGeNET** | ~2 GB | Credentials registered | Download pending | 7 |
| **SRA FASTQ** | ~220 GB | Not started | Large download | future |

---

## Access Patterns

### Pattern 1: Batch Read (Modules 1, 4, 6)

```
tideGlass → Neural API → content.get(hash) → nestGate CAS → local ZFS → data
```

Load large datasets (LINCS, ChEMBL) in bulk at pipeline start. Cache in memory
for the duration of the screen. This is the primary access pattern.

**Performance**: Local ZFS, no network. Expected throughput: disk-limited
(~500 MB/s sequential on NVMe).

### Pattern 2: Chunked Read (Modules 2, 3)

```
tideGlass → Neural API → content.get(hash) → nestGate CAS → chunked response
```

For training loops (RCL, GPS4Drug), load data in chunked content.get calls
rather than all at once. 20 GB LINCS doesn't fit in memory for all cell lines
simultaneously.

> **Note**: nestGate returns chunked responses for large objects (>1 MB) with
> base64-encoded `data` field. There is no separate streaming API — `content.get`
> handles both small and large objects.

### Pattern 3: Write (all modules)

```
tideGlass result → Neural API → content.put(data) → nestGate CAS → BLAKE3 hash
                 → Neural API → dag.event.append(hash) → rhizoCrypt
                 → Neural API → entry.append(hash) → loamSpine
                 → Neural API → braid.commit(attribution) → sweetGrass
```

Every pipeline output goes through the full provenance chain via Neural API:
1. Content stored in nestGate CAS (content-addressed by BLAKE3)
2. Execution event appended to rhizoCrypt DAG session
3. Provenance entry appended to loamSpine ledger
4. Attribution braid committed to sweetGrass

### Pattern 4: GPU Dispatch (Modules 2, 3, 5)

```
tideGlass → Neural API → compute.dispatch(shader, data) → toadStool → barraCuda GPU
         → result → tideGlass
```

For matrix operations, training, and MCTS evaluation. barraCuda provides FP64
tensor math on westGate's GPU (if available) or CPU fallback.

---

## nestGate CAS Integration

tideGlass talks to CAS via JSON-RPC over the Neural API socket:

```rust
// Content hash is bare BLAKE3 hex (64 chars, no prefix)
let hash = "a1b2c3d4e5f6..."; // 64-char lowercase hex

// Fetch data by content hash (routed via Neural API → nestGate)
let data = cas_client.get(hash).await?;

// Check existence (returns metadata: hash, size, stored_at, derivation_depth)
let exists = cas_client.exists(hash).await?;

// Store pipeline output
let response = cas_client.put(result_bytes, metadata).await?;
// response.hash: BLAKE3 hex of stored content
```

> **DIV-1 (Resolved)**: Hash format is bare 64-char BLAKE3 hex — no `blake3:`
> prefix. The original spec used `blake3:abc123...` which is not what nestGate
> returns or accepts.

---

## Provenance Convergence

westGate data exists in three provenance states (Wave 155u discovery):

| State | Meaning | Can compute? |
|-------|---------|--------------|
| **Primordial** | On disk, no CAS hash | No |
| **CAS-only** | BLAKE3 hash in CAS, no DAG/spine/braid | Yes (with caveat) |
| **Fully braided** | CAS + rhizoCrypt DAG + loamSpine + sweetGrass | Yes |

tideGlass implements `is_dataset_converged()` to check provenance state before
running science pipelines. Currently permits CAS-only state (most GPS platform
data). In production, only fully braided data should be trusted for results
that enter the provenance chain.

```rust
let convergence = data::is_dataset_converged(&cas_client, "tideglass.gps4drug_weights").await;
if convergence.is_computation_safe() {
    // proceed with pipeline
}
```

---

## Provenance Chain per Execution

Every module execution creates a provenance chain via Neural API routing:

```
rhizoCrypt: dag.session.create("tideglass-rges-screen-{timestamp}")
  → dag.event.append("input", { disease: "HCC", lincs_hash: "..." })
  → dag.event.append("compute", { compounds_scored: 15000, duration_ms: 4200 })
  → dag.event.append("output", { result_hash: "...", top_10: [...] })
  → dag.merkle.root() → session_root_hash

loamSpine: entry.append(session_root_hash)
  → certificate.mint({ module: "rges", claim: "r=0.52", validated: true })

sweetGrass: braid.create(["Chen2017", "LINCS_program", "ChEMBL"])
  → braid.commit(session_root_hash, attribution_chain)
```

---

## Current Implementation Status

- CAS client: **Implemented** in `tideglass-bin/src/cas_client.rs`
- Neural API routing: **Implemented** — prefers `neural-api-*.sock`, falls back to direct
- Data loading: **Implemented** — `load_from_cas()` on startup, graceful degradation
- Dataset discovery: **Implemented** — `content.query` by pipeline tag (nestGate v4.57+, DIV-2 RESOLVED)
- Convergence gate: **Implemented** — `is_dataset_converged()` API uses `content.query` for hash resolution
- GPS data format: **RESOLVED** (DIV-4) — 11 JSON files (103 MB) CAS-ingested with BLAKE3 provenance
- `content.query` types: `CasQueryParams`, `CasQueryResponse`, `CasQueryEntry` in `tideglass-core`
- Provenance write: **Not yet** — awaiting Neural API routing to provenance trio primals
