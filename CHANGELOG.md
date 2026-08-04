<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# Changelog

All notable changes to tideGlass are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Complete Rust workspace: tideglass-core, tideglass-rges, tideglass-rcl, tideglass-gps4drug, tideglass-screen, tideglass-molsearch, tideglass-octad, tideglass-nf, tideglass-bin
- UniBin binary (`tideglass`) with run/version/capabilities/help subcommands
- UDS JSON-RPC 2.0 server with NDJSON framing and graceful shutdown
- 11 IPC methods covering health triad, capabilities, and all 7 science modules
- Weighted Kolmogorov-Smirnov enrichment with permutation p-values
- Benjamini-Hochberg FDR correction pipeline
- Monte Carlo Tree Search molecular optimization
- Lipinski Rule of Five screening with structural alert filtering
- NF1 tissue-weighted reversal scoring
- Cell line SNR ranking
- OCTAD benchmark evaluation with AUC, precision/recall, F1, concordance correlation
- Capability-based primal discovery system
- biomeOS cell graph and guideStone deploy definitions
- 176 tests, clippy pedantic/nursery clean, zero warnings
- cargo deny + clippy pedantic/nursery + rustfmt toolchain configs
- G56 Neural API routing: CAS via `neural-api-default.sock` with direct fallback
- Provenance convergence gate: `is_dataset_converged()` for mixed-state data
- CAS routing mode reporting in health triad responses
- Centralized `PRIMAL_NAME` constant — single source of truth across all crates
- `count_as_f64()` helper — eliminated 17 scattered `#[allow(clippy::cast_precision_loss)]` annotations
- `store_pipeline_result()` — provenance write path for CAS result persistence
- `CasClient::socket_path()` accessor for write-back client creation
- All 21 transitive dependencies verified pure Rust (no C FFI)
- ScyBorg triple license (AGPL-3.0-or-later, ORC, CC-BY-SA 4.0)
- SPDX headers on all source and documentation files

### Changed

- Replaced monolithic rges.rs with modular enrichment.rs in tideglass-core
- Zero-copy types (Arc<str> newtypes) for GeneId, CompoundId, CellLineId

### Removed

- Removed crates/tideglass-core/src/rges.rs (superseded by enrichment.rs)

## [0.1.0] - TBD

Release date pending Phase 4 validation gate.

[Unreleased]: https://git.primals.eco/protoKarya/tideGlass/compare/v0.1.0...HEAD
[0.1.0]: https://git.primals.eco/protoKarya/tideGlass/releases/tag/v0.1.0
