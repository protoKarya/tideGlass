# crates/ -- Rust sovereign modules (Phase 3+)

Each crate implements one science module from the GPS platform rebuild.
Crates compose into a NUCLEUS via deploy graphs and consume primal
capabilities (nestGate data fetch, barraCuda GPU dispatch, provenance trio).

Planned crates:
- tideglass-rges -- RGES batch scoring
- tideglass-rcl -- RCL noise cleaning
- tideglass-gps4drug -- Structure to expression prediction
- tideglass-screen -- Reversal screening
- tideglass-molsearch -- MCTS compound optimization
- tideglass-octad -- OCTAD parity validation
- tideglass-nf -- NF extension (novel application)
