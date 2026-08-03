// SPDX-License-Identifier: AGPL-3.0-or-later

//! Compound library types with Lipinski Rule-of-Five filtering.

use serde::{Deserialize, Serialize};
use tideglass_core::types::{CompoundId, PerturbationSignature};

use tideglass_gps4drug::MolecularFeatures;

/// Lipinski Rule-of-Five thresholds for oral bioavailability heuristics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LipinskiConfig {
    /// Maximum molecular weight (default: 500 Da).
    pub max_molecular_weight: f64,
    /// Maximum `LogP` (default: 5.0).
    pub max_log_p: f64,
    /// Maximum hydrogen bond acceptors (default: 10).
    pub max_hba: u8,
    /// Maximum hydrogen bond donors (default: 5).
    pub max_hbd: u8,
}

impl Default for LipinskiConfig {
    fn default() -> Self {
        Self {
            max_molecular_weight: 500.0,
            max_log_p: 5.0,
            max_hba: 10,
            max_hbd: 5,
        }
    }
}

/// A compound entry in a screening library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryCompound {
    /// Compound identifier.
    pub compound_id: CompoundId,
    /// Molecular features for structural filtering and prediction.
    pub features: MolecularFeatures,
    /// Optional LINCS perturbation signature for RGES screening.
    pub perturbation: Option<PerturbationSignature>,
}

/// Filter result describing Lipinski violations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LipinskiViolation {
    /// Name of the violated rule.
    pub rule: String,
    /// Observed value that exceeded the threshold.
    pub observed: String,
    /// Configured threshold that was exceeded.
    pub limit: String,
}

/// Compound library for ZINC/ChEMBL-style reversal screening.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompoundLibrary {
    /// Compounds indexed in insertion order.
    pub compounds: Vec<LibraryCompound>,
    /// Lipinski thresholds applied by [`CompoundLibrary::filter_lipinski`].
    pub lipinski_config: LipinskiConfig,
}

impl CompoundLibrary {
    /// Creates an empty library with default Lipinski configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a compound to the library.
    pub fn add(&mut self, compound: LibraryCompound) {
        self.compounds.push(compound);
    }

    /// Returns compounds passing all Lipinski Rule-of-Five criteria.
    #[must_use]
    pub fn filter_lipinski(&self) -> Vec<&LibraryCompound> {
        self.compounds
            .iter()
            .filter(|compound| self.check_lipinski(compound).is_empty())
            .collect()
    }

    /// Returns Lipinski violations for a single compound.
    #[must_use]
    pub fn check_lipinski(&self, compound: &LibraryCompound) -> Vec<LipinskiViolation> {
        let props = &compound.features.properties;
        let config = &self.lipinski_config;
        let mut violations = Vec::new();

        if props.molecular_weight > config.max_molecular_weight {
            violations.push(LipinskiViolation {
                rule: "molecular_weight".to_owned(),
                observed: format!("{:.2}", props.molecular_weight),
                limit: format!("{:.2}", config.max_molecular_weight),
            });
        }
        if props.log_p > config.max_log_p {
            violations.push(LipinskiViolation {
                rule: "log_p".to_owned(),
                observed: format!("{:.2}", props.log_p),
                limit: format!("{:.2}", config.max_log_p),
            });
        }
        if props.hba > config.max_hba {
            violations.push(LipinskiViolation {
                rule: "hba".to_owned(),
                observed: props.hba.to_string(),
                limit: config.max_hba.to_string(),
            });
        }
        if props.hbd > config.max_hbd {
            violations.push(LipinskiViolation {
                rule: "hbd".to_owned(),
                observed: props.hbd.to_string(),
                limit: config.max_hbd.to_string(),
            });
        }

        violations
    }

    /// Returns compounds that have associated perturbation signatures.
    #[must_use]
    pub fn with_signatures(&self) -> Vec<&LibraryCompound> {
        self.compounds
            .iter()
            .filter(|compound| compound.perturbation.is_some())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use tideglass_gps4drug::PhysicochemicalProperties;

    use super::*;

    fn make_compound(id: &str, mw: f64, log_p: f64, hba: u8, hbd: u8) -> LibraryCompound {
        LibraryCompound {
            compound_id: CompoundId::new(id),
            features: MolecularFeatures {
                compound_id: CompoundId::new(id),
                smiles: None,
                fingerprint_bits: vec![],
                properties: PhysicochemicalProperties {
                    molecular_weight: mw,
                    log_p,
                    tpsa: 50.0,
                    hba,
                    hbd,
                    rotatable_bonds: 2,
                    aromatic_rings: 1,
                },
            },
            perturbation: None,
        }
    }

    #[test]
    fn lipinski_filters_overweight_compounds() {
        let mut library = CompoundLibrary::new();
        library.add(make_compound("GOOD", 400.0, 3.0, 5, 2));
        library.add(make_compound("BAD", 600.0, 3.0, 5, 2));

        let passing = library.filter_lipinski();
        assert_eq!(passing.len(), 1);
        assert_eq!(passing[0].compound_id.as_str(), "GOOD");
    }

    #[test]
    fn lipinski_detects_multiple_violations() {
        let library = CompoundLibrary::new();
        let compound = make_compound("VIOLATOR", 600.0, 6.0, 12, 6);
        let violations = library.check_lipinski(&compound);
        assert!(violations.len() >= 3);
    }
}
