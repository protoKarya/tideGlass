// SPDX-License-Identifier: AGPL-3.0-or-later

//! Multi-criteria compound screening filters.

use std::collections::HashSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tideglass_core::types::CompoundId;
use tideglass_rges::RankedRgesHit;

use crate::library::{CompoundLibrary, LipinskiConfig};

/// Structural alert patterns (SMARTS-like substrings) for reactive or toxic motifs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralAlertConfig {
    /// Substrings matched against SMILES to flag structural alerts.
    pub alert_patterns: Vec<Arc<str>>,
}

impl Default for StructuralAlertConfig {
    fn default() -> Self {
        Self {
            alert_patterns: vec![
                Arc::from("N=N"),
                Arc::from("S(=O)(=O)Cl"),
                Arc::from("C(=O)Cl"),
            ],
        }
    }
}

/// Combined screening filter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenFilterConfig {
    /// Minimum RGES reversal strength to pass.
    pub min_rges_strength: f64,
    /// Maximum raw p-value to pass.
    pub max_p_value: f64,
    /// Maximum FDR-adjusted p-value to pass.
    pub max_adjusted_p_value: f64,
    /// Lipinski thresholds for drug-likeness.
    pub lipinski: LipinskiConfig,
    /// Structural alert patterns.
    pub structural_alerts: StructuralAlertConfig,
    /// When true, compounds must pass Lipinski filters.
    pub require_lipinski: bool,
    /// When true, compounds with structural alerts are excluded.
    pub reject_structural_alerts: bool,
}

impl Default for ScreenFilterConfig {
    fn default() -> Self {
        Self {
            min_rges_strength: 0.0,
            max_p_value: 0.05,
            max_adjusted_p_value: 0.05,
            lipinski: LipinskiConfig::default(),
            structural_alerts: StructuralAlertConfig::default(),
            require_lipinski: true,
            reject_structural_alerts: true,
        }
    }
}

/// Reason a compound failed a screening filter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FilterRejection {
    /// Compound that was rejected.
    pub compound_id: CompoundId,
    /// Human-readable rejection reasons.
    pub reasons: Vec<String>,
}

/// Applies RGES, p-value, Lipinski, and structural alert filters to ranked hits.
#[must_use]
pub fn filter_ranked_hits(
    hits: &[RankedRgesHit],
    library: &CompoundLibrary,
    config: &ScreenFilterConfig,
) -> Vec<RankedRgesHit> {
    let compound_index: HashSet<&str> = library
        .compounds
        .iter()
        .map(|compound| compound.compound_id.as_str())
        .collect();

    hits.iter()
        .filter(|hit| compound_index.contains(hit.compound_id.as_str()))
        .filter(|hit| hit.reversal_strength >= config.min_rges_strength)
        .filter(|hit| hit.p_value <= config.max_p_value)
        .filter(|hit| hit.adjusted_p_value <= config.max_adjusted_p_value)
        .filter(|hit| {
            if !config.require_lipinski && !config.reject_structural_alerts {
                return true;
            }
            let Some(compound) = library
                .compounds
                .iter()
                .find(|c| c.compound_id == hit.compound_id)
            else {
                return false;
            };

            if config.require_lipinski {
                let lipinski = CompoundLibrary {
                    lipinski_config: config.lipinski.clone(),
                    ..CompoundLibrary::new()
                };
                if !lipinski.check_lipinski(compound).is_empty() {
                    return false;
                }
            }

            if config.reject_structural_alerts {
                if let Some(smiles) = &compound.features.smiles {
                    for pattern in &config.structural_alerts.alert_patterns {
                        if smiles.contains(pattern.as_ref()) {
                            return false;
                        }
                    }
                }
            }

            true
        })
        .cloned()
        .collect()
}

/// Returns detailed rejection reasons for compounds that fail filters.
#[must_use]
pub fn explain_rejections(
    hits: &[RankedRgesHit],
    library: &CompoundLibrary,
    config: &ScreenFilterConfig,
) -> Vec<FilterRejection> {
    let passing: HashSet<CompoundId> = filter_ranked_hits(hits, library, config)
        .into_iter()
        .map(|hit| hit.compound_id)
        .collect();

    hits.iter()
        .filter(|hit| !passing.contains(&hit.compound_id))
        .filter_map(|hit| {
            let compound = library
                .compounds
                .iter()
                .find(|c| c.compound_id == hit.compound_id)?;

            let mut reasons = Vec::new();
            if hit.reversal_strength < config.min_rges_strength {
                reasons.push(format!(
                    "RGES strength {:.3} below threshold {:.3}",
                    hit.reversal_strength, config.min_rges_strength
                ));
            }
            if hit.p_value > config.max_p_value {
                reasons.push(format!(
                    "p-value {:.4} exceeds {:.4}",
                    hit.p_value, config.max_p_value
                ));
            }
            if hit.adjusted_p_value > config.max_adjusted_p_value {
                reasons.push(format!(
                    "adjusted p-value {:.4} exceeds {:.4}",
                    hit.adjusted_p_value, config.max_adjusted_p_value
                ));
            }

            let lipinski_lib = CompoundLibrary {
                lipinski_config: config.lipinski.clone(),
                ..CompoundLibrary::new()
            };
            for violation in lipinski_lib.check_lipinski(compound) {
                reasons.push(format!(
                    "Lipinski violation: {} = {} (limit {})",
                    violation.rule, violation.observed, violation.limit
                ));
            }

            if let Some(smiles) = &compound.features.smiles {
                for pattern in &config.structural_alerts.alert_patterns {
                    if smiles.contains(pattern.as_ref()) {
                        reasons.push(format!("structural alert: {pattern}"));
                    }
                }
            }

            if reasons.is_empty() {
                None
            } else {
                Some(FilterRejection {
                    compound_id: hit.compound_id.clone(),
                    reasons,
                })
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tideglass_core::types::CompoundId;
    use tideglass_gps4drug::{MolecularFeatures, PhysicochemicalProperties};
    use tideglass_rges::RankedRgesHit;

    use super::*;
    use crate::library::LibraryCompound;

    fn sample_hit(id: &str, strength: f64, p: f64, adj_p: f64) -> RankedRgesHit {
        RankedRgesHit {
            compound_id: CompoundId::new(id),
            rges_score: strength,
            p_value: p,
            adjusted_p_value: adj_p,
            reversal_strength: strength,
            n_permutations: 100,
        }
    }

    #[test]
    fn filter_applies_rges_and_p_value_thresholds() {
        let mut library = CompoundLibrary::new();
        library.add(LibraryCompound {
            compound_id: CompoundId::new("A"),
            features: MolecularFeatures {
                compound_id: CompoundId::new("A"),
                smiles: None,
                fingerprint_bits: vec![],
                properties: PhysicochemicalProperties::default(),
            },
            perturbation: None,
        });

        let hits = vec![
            sample_hit("A", 0.8, 0.01, 0.02),
            sample_hit("B", 0.1, 0.5, 0.5),
        ];

        let config = ScreenFilterConfig {
            min_rges_strength: 0.5,
            max_p_value: 0.05,
            require_lipinski: false,
            reject_structural_alerts: false,
            ..ScreenFilterConfig::default()
        };

        let filtered = filter_ranked_hits(&hits, &library, &config);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].compound_id.as_str(), "A");
    }

    #[test]
    fn filter_rejects_structural_alerts() {
        let mut library = CompoundLibrary::new();
        library.add(LibraryCompound {
            compound_id: CompoundId::new("ALERT"),
            features: MolecularFeatures {
                compound_id: CompoundId::new("ALERT"),
                smiles: Some(Arc::from("CCN=N")),
                fingerprint_bits: vec![],
                properties: PhysicochemicalProperties::default(),
            },
            perturbation: None,
        });

        let hits = vec![sample_hit("ALERT", 0.9, 0.001, 0.01)];
        let config = ScreenFilterConfig {
            require_lipinski: false,
            reject_structural_alerts: true,
            ..ScreenFilterConfig::default()
        };

        let filtered = filter_ranked_hits(&hits, &library, &config);
        assert!(filtered.is_empty());
    }

    fn library_compound(
        id: &str,
        properties: PhysicochemicalProperties,
        smiles: Option<&str>,
    ) -> LibraryCompound {
        LibraryCompound {
            compound_id: CompoundId::new(id),
            features: MolecularFeatures {
                compound_id: CompoundId::new(id),
                smiles: smiles.map(Arc::from),
                fingerprint_bits: vec![],
                properties,
            },
            perturbation: None,
        }
    }

    #[test]
    fn explain_rejections_reports_p_value_violations() {
        let mut library = CompoundLibrary::new();
        library.add(library_compound(
            "PVAL",
            PhysicochemicalProperties::default(),
            None,
        ));

        let hits = vec![sample_hit("PVAL", 0.9, 0.2, 0.15)];
        let config = ScreenFilterConfig {
            require_lipinski: false,
            reject_structural_alerts: false,
            ..ScreenFilterConfig::default()
        };

        let rejections = explain_rejections(&hits, &library, &config);
        assert_eq!(rejections.len(), 1);
        assert!(
            rejections[0]
                .reasons
                .iter()
                .any(|reason| reason.contains("p-value"))
        );
    }

    #[test]
    fn explain_rejections_reports_lipinski_violations() {
        let mut library = CompoundLibrary::new();
        library.add(library_compound(
            "HEAVY",
            PhysicochemicalProperties {
                molecular_weight: 600.0,
                ..PhysicochemicalProperties::default()
            },
            None,
        ));

        let hits = vec![sample_hit("HEAVY", 0.9, 0.01, 0.01)];
        let config = ScreenFilterConfig {
            require_lipinski: true,
            reject_structural_alerts: false,
            ..ScreenFilterConfig::default()
        };

        let rejections = explain_rejections(&hits, &library, &config);
        assert_eq!(rejections.len(), 1);
        assert!(
            rejections[0]
                .reasons
                .iter()
                .any(|reason| reason.contains("Lipinski violation"))
        );
    }

    #[test]
    fn explain_rejections_reports_structural_alerts() {
        let mut library = CompoundLibrary::new();
        library.add(library_compound(
            "ALERT",
            PhysicochemicalProperties::default(),
            Some("CCN=N"),
        ));

        let hits = vec![sample_hit("ALERT", 0.9, 0.01, 0.01)];
        let config = ScreenFilterConfig {
            require_lipinski: false,
            reject_structural_alerts: true,
            ..ScreenFilterConfig::default()
        };

        let rejections = explain_rejections(&hits, &library, &config);
        assert_eq!(rejections.len(), 1);
        assert!(
            rejections[0]
                .reasons
                .iter()
                .any(|reason| reason.contains("structural alert"))
        );
    }

    #[test]
    fn filter_ranked_hits_passes_lipinski_compliant_compounds() {
        let mut library = CompoundLibrary::new();
        library.add(library_compound(
            "GOOD",
            PhysicochemicalProperties {
                molecular_weight: 400.0,
                log_p: 3.0,
                hba: 5,
                hbd: 2,
                ..PhysicochemicalProperties::default()
            },
            None,
        ));

        let hits = vec![sample_hit("GOOD", 0.8, 0.01, 0.02)];
        let config = ScreenFilterConfig {
            min_rges_strength: 0.5,
            require_lipinski: true,
            reject_structural_alerts: true,
            ..ScreenFilterConfig::default()
        };

        let filtered = filter_ranked_hits(&hits, &library, &config);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].compound_id.as_str(), "GOOD");
    }

    #[test]
    fn filter_ranked_hits_rejects_overweight_with_lipinski_required() {
        let mut library = CompoundLibrary::new();
        library.add(library_compound(
            "HEAVY",
            PhysicochemicalProperties {
                molecular_weight: 600.0,
                ..PhysicochemicalProperties::default()
            },
            None,
        ));

        let hits = vec![sample_hit("HEAVY", 0.9, 0.01, 0.01)];
        let config = ScreenFilterConfig {
            require_lipinski: true,
            reject_structural_alerts: false,
            ..ScreenFilterConfig::default()
        };

        let filtered = filter_ranked_hits(&hits, &library, &config);
        assert!(filtered.is_empty());
    }

    #[test]
    fn filter_ranked_hits_empty_when_no_library_matches() {
        let library = CompoundLibrary::new();
        let hits = vec![sample_hit("MISSING", 0.9, 0.01, 0.01)];
        let config = ScreenFilterConfig::default();

        let filtered = filter_ranked_hits(&hits, &library, &config);
        assert!(filtered.is_empty());
    }

    #[test]
    fn explain_rejections_empty_when_all_compounds_pass() {
        let mut library = CompoundLibrary::new();
        library.add(library_compound(
            "PASS",
            PhysicochemicalProperties {
                molecular_weight: 400.0,
                ..PhysicochemicalProperties::default()
            },
            None,
        ));

        let hits = vec![sample_hit("PASS", 0.8, 0.01, 0.02)];
        let config = ScreenFilterConfig {
            require_lipinski: true,
            reject_structural_alerts: true,
            ..ScreenFilterConfig::default()
        };

        let rejections = explain_rejections(&hits, &library, &config);
        assert!(rejections.is_empty());
    }
}
