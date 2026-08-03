// SPDX-License-Identifier: AGPL-3.0-or-later

//! Tissue geometry types for NF1 nerve sheath reversal scoring.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Anatomical tissue compartment in NF1 nerve sheath geometry.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TissueCompartment {
    /// Outer nerve sheath layer.
    NerveSheath,
    /// Perineurium connective tissue boundary.
    Perineurium,
    /// Endoneurium inner compartment surrounding axons.
    Endoneurium,
}

impl TissueCompartment {
    /// Returns the canonical string label for this compartment.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NerveSheath => "nerve_sheath",
            Self::Perineurium => "perineurium",
            Self::Endoneurium => "endoneurium",
        }
    }
}

/// Distance-based weight for a tissue compartment pair.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompartmentDistance {
    /// Source compartment.
    pub from: TissueCompartment,
    /// Target compartment.
    pub to: TissueCompartment,
    /// Geometric distance between compartments (loaded from config, not hardcoded).
    pub distance: f64,
}

/// Configuration for tissue geometry weighting in NF reversal scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TissueWeight {
    /// Geometry scale parameter "d" for exponential distance decay.
    pub geometry_scale_d: f64,
    /// Pairwise compartment distances.
    pub distances: Vec<CompartmentDistance>,
    /// Primary compartment where NF1 tumors arise.
    pub primary_compartment: TissueCompartment,
}

impl Default for TissueWeight {
    fn default() -> Self {
        Self {
            geometry_scale_d: 2.4,
            primary_compartment: TissueCompartment::NerveSheath,
            distances: vec![
                CompartmentDistance {
                    from: TissueCompartment::NerveSheath,
                    to: TissueCompartment::NerveSheath,
                    distance: 0.0,
                },
                CompartmentDistance {
                    from: TissueCompartment::NerveSheath,
                    to: TissueCompartment::Perineurium,
                    distance: 1.0,
                },
                CompartmentDistance {
                    from: TissueCompartment::NerveSheath,
                    to: TissueCompartment::Endoneurium,
                    distance: 2.0,
                },
                CompartmentDistance {
                    from: TissueCompartment::Perineurium,
                    to: TissueCompartment::Endoneurium,
                    distance: 1.0,
                },
            ],
        }
    }
}

impl TissueWeight {
    /// Computes the exponential decay weight for a compartment relative to the primary site.
    ///
    /// Weight = exp(-distance / d) where d is `geometry_scale_d`.
    #[must_use]
    pub fn compartment_weight(&self, compartment: TissueCompartment) -> f64 {
        let distance = self
            .distances
            .iter()
            .find(|entry| entry.from == self.primary_compartment && entry.to == compartment)
            .map_or(f64::MAX, |entry| entry.distance);

        (-distance / self.geometry_scale_d).exp()
    }

    /// Builds a lookup map from compartment to weight.
    #[must_use]
    pub fn weight_map(&self) -> HashMap<TissueCompartment, f64> {
        [
            TissueCompartment::NerveSheath,
            TissueCompartment::Perineurium,
            TissueCompartment::Endoneurium,
        ]
        .into_iter()
        .map(|compartment| (compartment, self.compartment_weight(compartment)))
        .collect()
    }
}

/// Gene-to-compartment assignment for geometry-weighted enrichment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneCompartmentMap {
    /// Maps gene symbols to their primary tissue compartment.
    pub assignments: HashMap<Arc<str>, TissueCompartment>,
}

impl GeneCompartmentMap {
    /// Returns the compartment for a gene, defaulting to the primary nerve sheath.
    #[must_use]
    pub fn compartment_for(&self, gene: &str, default: TissueCompartment) -> TissueCompartment {
        self.assignments.get(gene).copied().unwrap_or(default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_compartment_has_unit_weight() {
        let config = TissueWeight::default();
        let weight = config.compartment_weight(TissueCompartment::NerveSheath);
        assert!((weight - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn distant_compartment_has_lower_weight() {
        let config = TissueWeight::default();
        let near = config.compartment_weight(TissueCompartment::Perineurium);
        let far = config.compartment_weight(TissueCompartment::Endoneurium);
        assert!(near > far);
    }

    #[test]
    fn weight_map_contains_all_compartments() {
        let config = TissueWeight::default();
        let map = config.weight_map();
        assert_eq!(map.len(), 3);
    }
}
