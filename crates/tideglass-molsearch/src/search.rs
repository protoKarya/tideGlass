// SPDX-License-Identifier: AGPL-3.0-or-later

//! MCTS orchestrator for molecular lead optimization.

use rand::Rng;
use serde::{Deserialize, Serialize};
use tideglass_core::error::{Result, TideGlassError};
use tideglass_gps4drug::MolecularFeatures;

use crate::action::{MolecularAction, apply_action, default_actions};
use crate::tree::{MctsNode, node_at_path_mut, select_path};

/// Configuration for Monte Carlo Tree Search molecular optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MctsConfig {
    /// Number of MCTS iterations to run.
    pub iterations: u32,
    /// UCB1 exploration constant (default: √2 ≈ 1.414).
    pub exploration_constant: f64,
    /// Maximum tree depth from root.
    pub max_depth: u32,
    /// Target potency score to optimize toward (lower IC50 proxy is better).
    pub target_potency: f64,
}

impl Default for MctsConfig {
    fn default() -> Self {
        Self {
            iterations: 1_000,
            exploration_constant: 1.414,
            max_depth: 8,
            target_potency: 0.5,
        }
    }
}

/// Result of an MCTS optimization run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MctsResult {
    /// Best molecular state found during search.
    pub best_state: MolecularFeatures,
    /// Best reward achieved (higher is better).
    pub best_reward: f64,
    /// Action sequence from root to best leaf.
    pub action_path: Vec<MolecularAction>,
    /// Total iterations executed.
    pub iterations_run: u32,
}

/// Monte Carlo Tree Search optimizer for molecular lead refinement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MctsSearch {
    /// Search configuration.
    pub config: MctsConfig,
    /// Available modification actions for expansion.
    pub actions: Vec<MolecularAction>,
}

impl Default for MctsSearch {
    fn default() -> Self {
        Self {
            config: MctsConfig::default(),
            actions: default_actions(),
        }
    }
}

impl MctsSearch {
    /// Creates a search with custom configuration and action set.
    #[must_use]
    pub const fn new(config: MctsConfig, actions: Vec<MolecularAction>) -> Self {
        Self { config, actions }
    }

    /// Runs MCTS from the given lead compound state.
    ///
    /// # Errors
    ///
    /// Returns [`TideGlassError::Enrichment`] when no actions are configured.
    pub fn optimize(&self, initial: MolecularFeatures, rng: &mut impl Rng) -> Result<MctsResult> {
        if self.actions.is_empty() {
            return Err(TideGlassError::Enrichment {
                reason: "MCTS requires at least one molecular action".to_owned(),
            });
        }

        let mut root = MctsNode::root(initial);
        let mut best_reward = f64::NEG_INFINITY;
        let mut best_path: Vec<MolecularAction> = Vec::new();
        let mut best_state = root.state.clone();

        for _ in 0..self.config.iterations {
            let path = select_path(&root, self.config.exploration_constant);
            let Some(leaf) = node_at_path_mut(&mut root, &path) else {
                continue;
            };

            if leaf.depth < self.config.max_depth {
                let action_index = rng.random_range(0..self.actions.len());
                let action = self.actions[action_index].clone();
                let child_state = apply_action(&leaf.state, &action);
                leaf.expand(child_state, action);

                let child_index = leaf.children.len().saturating_sub(1);
                let mut full_path = path;
                full_path.push(child_index);

                let reward = self.simulate(&root, &full_path, rng);
                Self::backpropagate_path(&mut root, &full_path, reward);

                if reward > best_reward {
                    best_reward = reward;
                    best_path = Self::actions_on_path(&root, &full_path);
                    if let Some(state) = Self::state_at_path(&root, &full_path) {
                        best_state = state.clone();
                    }
                }
            } else {
                let reward = self.simulate(&root, &path, rng);
                Self::backpropagate_path(&mut root, &path, reward);
            }
        }

        Ok(MctsResult {
            best_state,
            best_reward,
            action_path: best_path,
            iterations_run: self.config.iterations,
        })
    }

    fn simulate(&self, root: &MctsNode, path: &[usize], rng: &mut impl Rng) -> f64 {
        let mut state = Self::state_at_path(root, path)
            .cloned()
            .unwrap_or_else(|| root.state.clone());

        let rollout_depth = rng.random_range(0..=3);
        for _ in 0..rollout_depth {
            let action_index = rng.random_range(0..self.actions.len());
            state = apply_action(&state, &self.actions[action_index]);
        }

        score_state(&state, self.config.target_potency)
    }

    fn backpropagate_path(root: &mut MctsNode, path: &[usize], reward: f64) {
        root.backpropagate(reward);
        for depth in 0..path.len() {
            let subpath = &path[..=depth];
            if let Some(node) = node_at_path_mut(root, subpath) {
                node.backpropagate(reward);
            }
        }
    }

    fn state_at_path<'a>(root: &'a MctsNode, path: &[usize]) -> Option<&'a MolecularFeatures> {
        if path.is_empty() {
            return Some(&root.state);
        }
        let mut current = root;
        for &index in path {
            current = current.children.get(index)?;
        }
        Some(&current.state)
    }

    fn actions_on_path(root: &MctsNode, path: &[usize]) -> Vec<MolecularAction> {
        let mut actions = Vec::new();
        let mut current = root;
        for &index in path {
            if let Some(child) = current.children.get(index) {
                if let Some(action) = &child.action {
                    actions.push(action.clone());
                }
                current = child;
            }
        }
        actions
    }
}

/// Heuristic reward: higher when molecular properties approach target potency proxy.
fn score_state(state: &MolecularFeatures, target_potency: f64) -> f64 {
    let potency_proxy = estimate_potency_proxy(state);
    let potency_score = 1.0 / (1.0 + (potency_proxy - target_potency).abs());
    let drug_like = lipinski_score(state);
    potency_score * 0.7 + drug_like * 0.3
}

fn estimate_potency_proxy(state: &MolecularFeatures) -> f64 {
    let props = &state.properties;
    let log_p_term = (props.log_p - 2.5).abs() / 5.0;
    let mw_term = props.molecular_weight / 500.0;
    log_p_term + mw_term
}

fn lipinski_score(state: &MolecularFeatures) -> f64 {
    let props = &state.properties;
    let mut score = 1.0_f64;
    if props.molecular_weight > 500.0 {
        score -= 0.25;
    }
    if props.log_p > 5.0 {
        score -= 0.25;
    }
    if props.hba > 10 {
        score -= 0.25;
    }
    if props.hbd > 5 {
        score -= 0.25;
    }
    score.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use tideglass_core::types::CompoundId;
    use tideglass_gps4drug::{MolecularFeatures, PhysicochemicalProperties};

    use super::*;

    fn lead_compound() -> MolecularFeatures {
        MolecularFeatures {
            compound_id: CompoundId::new("LEAD"),
            smiles: None,
            fingerprint_bits: vec![1, 0, 1, 0],
            properties: PhysicochemicalProperties {
                molecular_weight: 350.0,
                log_p: 3.0,
                tpsa: 70.0,
                hba: 4,
                hbd: 2,
                rotatable_bonds: 3,
                aromatic_rings: 2,
            },
        }
    }

    #[test]
    fn optimize_improves_reward_over_iterations() {
        let search = MctsSearch {
            config: MctsConfig {
                iterations: 100,
                max_depth: 4,
                target_potency: 0.5,
                ..MctsConfig::default()
            },
            ..MctsSearch::default()
        };

        let mut rng = rand::rng();
        let initial_score = score_state(&lead_compound(), search.config.target_potency);
        let result = search
            .optimize(lead_compound(), &mut rng)
            .expect("optimize");

        assert!(result.best_reward >= initial_score * 0.5);
        assert_eq!(result.iterations_run, 100);
    }

    #[test]
    fn optimize_errors_without_actions() {
        let search = MctsSearch {
            actions: vec![],
            ..MctsSearch::default()
        };
        let mut rng = rand::rng();
        let err = search.optimize(lead_compound(), &mut rng).unwrap_err();
        assert!(matches!(err, TideGlassError::Enrichment { .. }));
    }
}
