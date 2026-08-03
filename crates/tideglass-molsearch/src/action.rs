// SPDX-License-Identifier: AGPL-3.0-or-later

//! Molecular modification actions for MCTS search.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tideglass_gps4drug::MolecularFeatures;

/// A discrete molecular modification applied during MCTS expansion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MolecularAction {
    /// Add a substituent group at a specified attachment site index.
    AddSubstituent {
        /// Index of the attachment site on the parent scaffold.
        site_index: u8,
        /// Substituent group label (e.g. methyl, hydroxyl).
        group: Arc<str>,
    },
    /// Remove a functional group at a specified site index.
    RemoveGroup {
        /// Index of the group to remove.
        site_index: u8,
    },
    /// Modify a ring system (fusion, saturation, or heteroatom insertion).
    RingModification {
        /// Ring identifier within the scaffold.
        ring_index: u8,
        /// Modification type label.
        modification: Arc<str>,
    },
    /// Replace one atom or group with another at a site.
    ReplaceAtom {
        /// Attachment site index.
        site_index: u8,
        /// Replacement group label.
        replacement: Arc<str>,
    },
}

impl MolecularAction {
    /// Returns a human-readable label for logging and serialization.
    #[must_use]
    pub fn label(&self) -> Arc<str> {
        match self {
            Self::AddSubstituent { group, .. } => Arc::clone(group),
            Self::RemoveGroup { .. } => Arc::from("remove"),
            Self::RingModification { modification, .. } => Arc::clone(modification),
            Self::ReplaceAtom { replacement, .. } => Arc::clone(replacement),
        }
    }
}

/// Applies a molecular action to features, producing a modified copy.
///
/// This is a deterministic heuristic mutator for MCTS simulation — it adjusts
/// physicochemical descriptors based on the action type rather than performing
/// full cheminformatics.
#[must_use]
pub fn apply_action(features: &MolecularFeatures, action: &MolecularAction) -> MolecularFeatures {
    let mut modified = features.clone();

    match action {
        MolecularAction::AddSubstituent { .. } => {
            modified.properties.molecular_weight += 15.0;
            modified.properties.log_p += 0.3;
            modified.fingerprint_bits.push(1);
        }
        MolecularAction::RemoveGroup { .. } => {
            modified.properties.molecular_weight =
                (modified.properties.molecular_weight - 15.0).max(50.0);
            modified.properties.log_p -= 0.2;
            if !modified.fingerprint_bits.is_empty() {
                modified.fingerprint_bits.pop();
            }
        }
        MolecularAction::RingModification { .. } => {
            modified.properties.aromatic_rings =
                modified.properties.aromatic_rings.saturating_add(1);
            modified.properties.tpsa += 5.0;
            modified.fingerprint_bits.push(1);
        }
        MolecularAction::ReplaceAtom { .. } => {
            modified.properties.hba = modified.properties.hba.saturating_add(1);
            modified.properties.tpsa += 3.0;
        }
    }

    modified
}

/// Returns the default action set available for MCTS expansion.
#[must_use]
pub fn default_actions() -> Vec<MolecularAction> {
    vec![
        MolecularAction::AddSubstituent {
            site_index: 0,
            group: Arc::from("methyl"),
        },
        MolecularAction::AddSubstituent {
            site_index: 1,
            group: Arc::from("hydroxyl"),
        },
        MolecularAction::RemoveGroup { site_index: 0 },
        MolecularAction::RingModification {
            ring_index: 0,
            modification: Arc::from("saturate"),
        },
        MolecularAction::ReplaceAtom {
            site_index: 0,
            replacement: Arc::from("fluorine"),
        },
    ]
}

#[cfg(test)]
mod tests {
    use tideglass_gps4drug::PhysicochemicalProperties;

    use super::*;

    #[test]
    fn add_substituent_increases_molecular_weight() {
        let features = MolecularFeatures {
            compound_id: tideglass_core::types::CompoundId::new("LEAD"),
            smiles: None,
            fingerprint_bits: vec![0, 1],
            properties: PhysicochemicalProperties {
                molecular_weight: 300.0,
                log_p: 2.0,
                ..PhysicochemicalProperties::default()
            },
        };

        let action = MolecularAction::AddSubstituent {
            site_index: 0,
            group: Arc::from("methyl"),
        };
        let modified = apply_action(&features, &action);
        assert!(modified.properties.molecular_weight > features.properties.molecular_weight);
    }

    fn base_features() -> MolecularFeatures {
        MolecularFeatures {
            compound_id: tideglass_core::types::CompoundId::new("LEAD"),
            smiles: None,
            fingerprint_bits: vec![0, 1],
            properties: PhysicochemicalProperties {
                molecular_weight: 300.0,
                log_p: 2.0,
                hba: 2,
                hbd: 1,
                aromatic_rings: 1,
                ..PhysicochemicalProperties::default()
            },
        }
    }

    #[test]
    fn ring_modification_increases_aromatic_rings() {
        let features = base_features();
        let action = MolecularAction::RingModification {
            ring_index: 0,
            modification: Arc::from("saturate"),
        };
        let modified = apply_action(&features, &action);
        assert_eq!(
            modified.properties.aromatic_rings,
            features.properties.aromatic_rings + 1
        );
    }

    #[test]
    fn replace_atom_increases_hba() {
        let features = base_features();
        let action = MolecularAction::ReplaceAtom {
            site_index: 0,
            replacement: Arc::from("fluorine"),
        };
        let modified = apply_action(&features, &action);
        assert_eq!(modified.properties.hba, features.properties.hba + 1);
    }

    #[test]
    fn add_substituent_increases_molecular_weight_for_chain_extension() {
        let features = base_features();
        let action = MolecularAction::AddSubstituent {
            site_index: 0,
            group: Arc::from("ethyl"),
        };
        let modified = apply_action(&features, &action);
        assert!(modified.properties.molecular_weight > features.properties.molecular_weight);
    }

    #[test]
    fn default_actions_returns_non_empty_list() {
        let actions = default_actions();
        assert!(!actions.is_empty());
    }
}
