// SPDX-License-Identifier: AGPL-3.0-or-later

//! Molecular feature vectors for structure-to-expression prediction.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tideglass_core::types::CompoundId;

/// Physicochemical descriptors used in structure-to-expression models.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhysicochemicalProperties {
    /// Molecular weight in Daltons.
    pub molecular_weight: f64,
    /// Calculated octanol-water partition coefficient (`LogP`).
    pub log_p: f64,
    /// Topological polar surface area in Ų.
    pub tpsa: f64,
    /// Hydrogen bond acceptor count.
    pub hba: u8,
    /// Hydrogen bond donor count.
    pub hbd: u8,
    /// Rotatable bond count.
    pub rotatable_bonds: u8,
    /// Aromatic ring count.
    pub aromatic_rings: u8,
}

impl Default for PhysicochemicalProperties {
    fn default() -> Self {
        Self {
            molecular_weight: 0.0,
            log_p: 0.0,
            tpsa: 0.0,
            hba: 0,
            hbd: 0,
            rotatable_bonds: 0,
            aromatic_rings: 0,
        }
    }
}

/// Complete molecular feature vector for a compound.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MolecularFeatures {
    /// Compound identifier (`ChEMBL`, ZINC, etc.).
    pub compound_id: CompoundId,
    /// Optional SMILES representation.
    pub smiles: Option<Arc<str>>,
    /// Binary fingerprint bit vector (e.g. Morgan/ECFP bits).
    pub fingerprint_bits: Vec<u8>,
    /// Scalar physicochemical descriptors.
    pub properties: PhysicochemicalProperties,
}

impl MolecularFeatures {
    /// Returns a flat numeric feature vector: descriptors followed by fingerprint bits.
    #[must_use]
    pub fn to_feature_vector(&self) -> Vec<f64> {
        let mut features = vec![
            self.properties.molecular_weight,
            self.properties.log_p,
            self.properties.tpsa,
            f64::from(self.properties.hba),
            f64::from(self.properties.hbd),
            f64::from(self.properties.rotatable_bonds),
            f64::from(self.properties.aromatic_rings),
        ];
        features.extend(self.fingerprint_bits.iter().map(|&bit| f64::from(bit)));
        features
    }

    /// Number of scalar descriptor dimensions (excluding fingerprint bits).
    #[must_use]
    pub const fn descriptor_count() -> usize {
        7
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_vector_includes_descriptors_and_bits() {
        let features = MolecularFeatures {
            compound_id: CompoundId::new("CHEMBL1"),
            smiles: Some(Arc::from("CCO")),
            fingerprint_bits: vec![1, 0, 1],
            properties: PhysicochemicalProperties {
                molecular_weight: 46.07,
                log_p: -0.31,
                tpsa: 20.23,
                hba: 1,
                hbd: 1,
                rotatable_bonds: 0,
                aromatic_rings: 0,
            },
        };

        let vector = features.to_feature_vector();
        assert_eq!(vector.len(), MolecularFeatures::descriptor_count() + 3);
        assert!((vector[0] - 46.07).abs() < f64::EPSILON);
        assert!((vector[7] - 1.0).abs() < f64::EPSILON);
    }
}
