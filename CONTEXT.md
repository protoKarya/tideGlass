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

- **validation/** — Python-first reproduction of published claims (Phase 0-2)
- **crates/** — Rust sovereign modules (Phase 3+)
  - `tideglass-rges` — RGES batch scoring
  - `tideglass-rcl` — RCL noise cleaning
  - `tideglass-gps4drug` — Structure → expression prediction
  - `tideglass-screen` — Reversal screening
  - `tideglass-molsearch` — MCTS compound optimization
  - `tideglass-octad` — OCTAD parity validation
  - `tideglass-nf` — NF extension (novel application)
- **artifact/** — guideStone-format data and expected outputs
- **notebooks/** — Tier 1 Python notebooks per module
- **graphs/** — Deploy graph TOMLs for NUCLEUS composition
- **shaders/** — WGSL shaders for barraCuda dispatch

## Key Data

- LINCS L1000 Level 5 — 1.3M drug perturbation profiles (primary source)
- ChEMBL — bioactivity panels (JAK, kinase selectivity)
- ZINC — screened compound library (750M+ structures)
- NF Data Portal — NF1-driven tumor transcriptomic signatures (novel extension)

## Dependencies

Phase 0-2: Python stack (reproducing original GPS codebase)
Phase 3+: Rust + ecoPrimals primals (NUCLEUS composition)

## Status

Phase 0 — Archaeology. Inventorying the 713 MB Zenodo artifact, mapping
dependency graph, tracing data lineages from primary sources.
