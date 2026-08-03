<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# shaders/ -- WGSL shaders for barraCuda dispatch

Three shaders planned for GPU-accelerated computation:
1. rges_batch_score_f64.wgsl -- RGES batch scoring (KS enrichment)
2. mcts_rollout_f32.wgsl -- MCTS rollout evaluation
3. smiles_fingerprint_f32.wgsl -- Morgan fingerprint computation

All shaders follow the barraCuda dispatch pattern established by
healthSpring (hill_dose_response, population_pk, etc.).
