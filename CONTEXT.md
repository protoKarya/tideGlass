# SPDX-License-Identifier: AGPL-3.0-or-later

# tideGlass — Context

## What

tideGlass is a sovereign rebuild of the GPS (Gene-Expression-based Platform for
Screening) platform. It validates published drug repurposing claims by
reproducing from primary data, then rebuilds sovereign infrastructure for
independent screening and compound optimization.

First project of the protoKarya organization. First gen5-native product in the
ecoPrimals ecosystem — no gen3 baseCamp origin, no gen4 predecessor.

## Role

tideGlass sits in the gen5 product layer. It consumes validated science from
healthSpring, wetSpring, neuralSpring, and hotSpring. It consumes primal
capabilities via NUCLEUS composition (nestGate for data, barraCuda for GPU,
provenance trio for chains). It produces pseudoSpores and lithoSpores for
collaborator delivery.

Unlike gen4 products that emerged from internal experiments, tideGlass was
assigned by an external collaborator (Andrea Gonzales) with a specific
codebase target (GPS, Cell 2026). The rebuild forces capability gaps that
the ecosystem would not have discovered organically.

## Architecture

- **crates/** — Rust workspace (9 crates, complete)
  - `tideglass-core` — shared types, enrichment, IPC, discovery
  - `tideglass-rges` — RGES batch scoring
  - `tideglass-rcl` — RCL noise cleaning
  - `tideglass-gps4drug` — Structure → expression prediction
  - `tideglass-screen` — Reversal screening
  - `tideglass-molsearch` — MCTS compound optimization
  - `tideglass-octad` — OCTAD parity validation
  - `tideglass-nf` — NF extension (novel application)
  - `tideglass-bin` — UniBin server (`run`, `version`, `capabilities`, `help`)
- **artifact/** — guideStone-format data and expected outputs
- **graphs/** — Deploy graph TOMLs for NUCLEUS composition
- **shaders/** — WGSL shaders for barraCuda dispatch
- **validation/** — reference outputs and future fixture milestone targets

## IPC Methods

The `tideglass` UniBin serves UDS JSON-RPC 2.0 over NDJSON framing. Eleven
methods are implemented:

| Domain | Methods |
|--------|---------|
| Capabilities | `capabilities.list` |
| Health | `health.liveness`, `health.check`, `health.readiness` |
| Science | `science.rges_screen`, `science.rcl_select`, `science.gps4drug_predict`, `science.compound_screen`, `science.mcts_optimize`, `science.octad_benchmark`, `science.nf_score` |

## Key Data

- westGate CAS federation — 519 GB local science data (13 datasets in tideGlass domain)
- LINCS L1000 Level 5 — 1.3M drug perturbation profiles (primary source)
- ChEMBL — bioactivity panels (JAK, kinase selectivity)
- ZINC — screened compound library (750M+ structures)
- NF Data Portal — NF1-driven tumor transcriptomic signatures (novel extension)

CAS data loading is wired via biomeOS Neural API (`neural-api-default.sock`)
with graceful degradation. GPS platform data is CAS-indexed (NumPy/pickle,
needs JSON conversion). Provenance convergence gate implemented for mixed-state
data on westGate.

## Dependencies

Rust workspace is complete (serde, serde_json, thiserror, rand, tokio, base64).
Python validation is deferred to a future phase. Remaining: GPS data JSON
conversion, Chen 2017 benchmark, provenance write via Neural API.

## Status

Phase 4 — Package. Rust workspace rebuilt and tested (176 tests). G56 Neural
API routing complete. Provenance convergence gate implemented. CAS wiring live
with graceful degradation. Current work: GPS JSON conversion, Chen 2017
benchmark (r >= 0.52), provenance write chain.
