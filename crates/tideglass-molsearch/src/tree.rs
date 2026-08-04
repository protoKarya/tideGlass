// SPDX-License-Identifier: AGPL-3.0-or-later

//! Monte Carlo Tree Search node structure and UCB1 selection.

use serde::{Deserialize, Serialize};
use tideglass_gps4drug::MolecularFeatures;

use crate::action::MolecularAction;

/// A node in the MCTS search tree representing a molecular state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MctsNode {
    /// Molecular features at this tree node.
    pub state: MolecularFeatures,
    /// Action that produced this state from the parent (None for root).
    pub action: Option<MolecularAction>,
    /// Number of times this node has been visited.
    pub visits: u32,
    /// Cumulative reward from rollouts through this node.
    pub total_reward: f64,
    /// Child nodes produced by expansion.
    pub children: Vec<Self>,
    /// Depth of this node from the root (root = 0).
    pub depth: u32,
}

impl MctsNode {
    /// Creates a root node from an initial molecular state.
    #[must_use]
    pub const fn root(state: MolecularFeatures) -> Self {
        Self {
            state,
            action: None,
            visits: 0,
            total_reward: 0.0,
            children: Vec::new(),
            depth: 0,
        }
    }

    /// Mean reward per visit for this node.
    #[must_use]
    pub fn average_reward(&self) -> f64 {
        if self.visits == 0 {
            0.0
        } else {
            self.total_reward / f64::from(self.visits)
        }
    }

    /// Returns true when the node has no expanded children.
    #[must_use]
    pub const fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// UCB1 selection score balancing exploitation and exploration.
    #[must_use]
    pub fn ucb1_score(&self, parent_visits: u32, exploration_constant: f64) -> f64 {
        if self.visits == 0 {
            return f64::INFINITY;
        }
        let exploitation = self.average_reward();
        let exploration = exploration_constant
            * ((f64::from(parent_visits).ln()) / f64::from(self.visits)).sqrt();
        exploitation + exploration
    }

    /// Selects the child with the highest UCB1 score.
    #[must_use]
    pub fn select_best_child(&self, exploration_constant: f64) -> Option<&Self> {
        self.children.iter().max_by(|a, b| {
            a.ucb1_score(self.visits, exploration_constant)
                .partial_cmp(&b.ucb1_score(self.visits, exploration_constant))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Adds a child node produced by applying an action.
    pub fn expand(&mut self, child_state: MolecularFeatures, action: MolecularAction) {
        let depth = self.depth.saturating_add(1);
        self.children.push(Self {
            state: child_state,
            action: Some(action),
            visits: 0,
            total_reward: 0.0,
            children: Vec::new(),
            depth,
        });
    }

    /// Propagates a rollout reward up from this node (inclusive).
    pub fn backpropagate(&mut self, reward: f64) {
        self.visits = self.visits.saturating_add(1);
        self.total_reward += reward;
    }
}

/// Traverses from root to a leaf using UCB1 selection.
#[must_use]
pub fn select_path(root: &MctsNode, exploration_constant: f64) -> Vec<usize> {
    let mut path = Vec::new();
    let mut current = root;

    while !current.is_leaf() {
        if let Some((index, _)) = current
            .children
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                a.ucb1_score(current.visits, exploration_constant)
                    .partial_cmp(&b.ucb1_score(current.visits, exploration_constant))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        {
            path.push(index);
            current = &current.children[index];
        } else {
            break;
        }
    }

    path
}

/// Navigates to a node given a path of child indices.
#[must_use]
pub fn node_at_path<'a>(root: &'a MctsNode, path: &[usize]) -> &'a MctsNode {
    let mut current = root;
    for &index in path {
        current = current.children.get(index).unwrap_or(current);
    }
    current
}

/// Mutably navigates to a node given a path of child indices.
///
/// Returns `None` when any index is out of bounds.
pub fn node_at_path_mut<'a>(root: &'a mut MctsNode, path: &[usize]) -> Option<&'a mut MctsNode> {
    match path.split_first() {
        None => Some(root),
        Some((&index, tail)) => {
            let child = root.children.get_mut(index)?;
            node_at_path_mut(child, tail)
        }
    }
}

#[cfg(test)]
mod tests {
    use tideglass_core::types::CompoundId;
    use tideglass_gps4drug::{MolecularFeatures, PhysicochemicalProperties};

    use super::*;

    fn sample_state(id: &str) -> MolecularFeatures {
        MolecularFeatures {
            compound_id: CompoundId::new(id),
            smiles: None,
            fingerprint_bits: vec![1, 0],
            properties: PhysicochemicalProperties::default(),
        }
    }

    #[test]
    fn ucb1_prefers_unvisited_child() {
        let mut root = MctsNode::root(sample_state("ROOT"));
        root.expand(
            sample_state("CHILD"),
            crate::action::MolecularAction::RemoveGroup { site_index: 0 },
        );
        root.visits = 10;

        let score = root.children[0].ucb1_score(root.visits, 1.414);
        assert!(score.is_infinite());
    }

    #[test]
    fn backpropagate_updates_visits_and_reward() {
        let mut node = MctsNode::root(sample_state("ROOT"));
        node.backpropagate(0.8);
        assert_eq!(node.visits, 1);
        assert!((node.average_reward() - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn select_path_reaches_leaf() {
        let mut root = MctsNode::root(sample_state("ROOT"));
        root.expand(
            sample_state("CHILD"),
            crate::action::MolecularAction::RemoveGroup { site_index: 0 },
        );
        root.children[0].visits = 1;
        root.children[0].total_reward = 0.5;
        root.visits = 1;

        let path = select_path(&root, 1.414);
        assert!(path.is_empty() || path.len() == 1);
    }

    #[test]
    fn average_reward_zero_for_unvisited_node() {
        let node = MctsNode::root(sample_state("ROOT"));
        assert!((node.average_reward()).abs() < f64::EPSILON);
    }

    #[test]
    fn select_path_empty_for_leaf_root() {
        let root = MctsNode::root(sample_state("ROOT"));
        let path = select_path(&root, 1.414);
        assert!(path.is_empty());
    }

    #[test]
    fn select_best_child_picks_highest_ucb1() {
        let mut root = MctsNode::root(sample_state("ROOT"));
        root.expand(
            sample_state("LOW"),
            crate::action::MolecularAction::RemoveGroup { site_index: 0 },
        );
        root.expand(
            sample_state("HIGH"),
            crate::action::MolecularAction::RemoveGroup { site_index: 1 },
        );
        root.visits = 20;
        root.children[0].visits = 5;
        root.children[0].total_reward = 1.0;
        root.children[1].visits = 5;
        root.children[1].total_reward = 4.0;

        let best = root
            .select_best_child(1.414)
            .expect("expected a best child");
        assert_eq!(best.state.compound_id.as_str(), "HIGH");
    }

    #[test]
    fn expand_sets_child_depth() {
        let mut root = MctsNode::root(sample_state("ROOT"));
        root.expand(
            sample_state("CHILD"),
            crate::action::MolecularAction::RemoveGroup { site_index: 0 },
        );
        assert_eq!(root.children[0].depth, 1);

        root.children[0].expand(
            sample_state("GRANDCHILD"),
            crate::action::MolecularAction::RemoveGroup { site_index: 0 },
        );
        assert_eq!(root.children[0].children[0].depth, 2);
    }

    #[test]
    fn node_at_path_navigates_multi_level_tree() {
        let mut root = MctsNode::root(sample_state("ROOT"));
        root.expand(
            sample_state("A"),
            crate::action::MolecularAction::RemoveGroup { site_index: 0 },
        );
        root.children[0].expand(
            sample_state("B"),
            crate::action::MolecularAction::RemoveGroup { site_index: 0 },
        );

        let node = node_at_path(&root, &[0, 0]);
        assert_eq!(node.state.compound_id.as_str(), "B");
        assert_eq!(node.depth, 2);
    }

    #[test]
    fn node_at_path_mut_modifies_deep_node() {
        let mut root = MctsNode::root(sample_state("ROOT"));
        root.expand(
            sample_state("A"),
            crate::action::MolecularAction::RemoveGroup { site_index: 0 },
        );
        root.children[0].expand(
            sample_state("B"),
            crate::action::MolecularAction::RemoveGroup { site_index: 0 },
        );

        let deep = node_at_path_mut(&mut root, &[0, 0]).expect("deep node");
        deep.backpropagate(0.75);
        assert_eq!(deep.visits, 1);
        assert!((deep.average_reward() - 0.75).abs() < f64::EPSILON);
    }
}
