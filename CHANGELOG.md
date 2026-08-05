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
- 17 IPC methods: health triad, capabilities, 7 science modules, 5 visualization scenes, data catalog
- Weighted Kolmogorov-Smirnov enrichment with permutation p-values
- Benjamini-Hochberg FDR correction pipeline
- Monte Carlo Tree Search molecular optimization
- Lipinski Rule of Five screening with structural alert filtering
- NF1 tissue-weighted reversal scoring
- Cell line SNR ranking
- OCTAD benchmark evaluation with AUC, precision/recall, F1, concordance correlation
- Capability-based primal discovery system
- biomeOS cell graph and guideStone deploy definitions
- 220 tests, clippy pedantic/nursery clean, zero warnings
- Live NUCLEUS validation on westGate — first RGES computation on hardware
- cargo deny + clippy pedantic/nursery + rustfmt toolchain configs
- G56 Neural API routing: prefix-glob socket discovery in `membrane/` with direct fallback
- Neural API → direct nestGate automatic fallback when Neural API is unresponsive (DIV-8)
- Socket discovery via `find_socket_by_prefix` — handles family-ID naming convention (DIV-7)
- Provenance convergence gate: `is_dataset_converged()` for mixed-state data
- CAS routing mode reporting in health triad responses
- Centralized `PRIMAL_NAME` constant — single source of truth across all crates
- `count_as_f64()` helper — eliminated 17 scattered `#[allow(clippy::cast_precision_loss)]` annotations
- `store_pipeline_result()` — provenance write path for CAS result persistence
- `CasClient::socket_path()` accessor for write-back client creation
- petalTongue IPC client (`petaltongue.rs`) with Neural API socket discovery
- 5 P0 visualization scene builders (`scenes.rs`): RGES volcano, enrichment curve, NF dashboard, GPS4Drug scatter, MCTS trace
- `visualization.*` JSON-RPC dispatch methods for petalTongue-first web presence
- `data.catalog` JSON-RPC method for CAS dataset inventory
- `content.query` CAS metadata search (nestGate v4.57+, DIV-2 resolved)
- `CasClient::query()` — discover datasets by pipeline tag at startup
- `query_dataset_hash()` replaces dead `resolve_dataset_hash()` — live CAS resolution
- `CasQueryParams`, `CasQueryResponse`, `CasQueryEntry` types in `tideglass-core`
- `PetalTongueClient` activated — instantiated at startup, viz scenes forwarded to petalTongue
- `ServerContext` replaces separate `Arc<ModuleData>` — carries both data and petalTongue client
- `is_viz_method()` and `extract_method()` for server-side scene forwarding
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
