// SPDX-License-Identifier: AGPL-3.0-or-later

//! RGES screening pipeline orchestrating enrichment, FDR correction, and ranking.

use serde::{Deserialize, Serialize};
use tideglass_core::enrichment::compute_rges;
use tideglass_core::error::Result;
use tideglass_core::types::{DiseaseSignature, EnrichmentConfig, PerturbationSignature};

use crate::screen::{RankedRgesHit, ScreenConfig, screen_compounds};

/// End-to-end RGES reversal screening pipeline.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RgesPipeline {
    /// Enrichment and permutation configuration passed to core.
    pub enrichment_config: EnrichmentConfig,
    /// Post-enrichment screening and FDR configuration.
    pub screen_config: ScreenConfig,
}

impl RgesPipeline {
    /// Creates a pipeline with the given enrichment and screening configuration.
    #[must_use]
    pub const fn new(enrichment_config: EnrichmentConfig, screen_config: ScreenConfig) -> Self {
        Self {
            enrichment_config,
            screen_config,
        }
    }

    /// Runs the full pipeline: compute RGES, apply BH FDR, rank by reversal strength.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`compute_rges`] or [`screen_compounds`].
    pub fn run(
        &self,
        disease: &DiseaseSignature,
        perturbations: &[PerturbationSignature],
    ) -> Result<Vec<RankedRgesHit>> {
        let raw_results = compute_rges(disease, perturbations, &self.enrichment_config)?;
        screen_compounds(&raw_results, &self.screen_config)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tideglass_core::types::{CellLineId, CompoundId, GeneId};

    use super::*;

    fn gene(name: &str) -> GeneId {
        GeneId::new(name)
    }

    fn genes(names: &[&str]) -> Vec<GeneId> {
        names.iter().map(|name| gene(name)).collect()
    }

    #[test]
    fn pipeline_returns_ranked_hits() {
        let disease = DiseaseSignature {
            name: Arc::from("test-disease"),
            up_genes: genes(&["U1", "U2", "U3", "U4", "U5"]),
            down_genes: genes(&["D1", "D2", "D3", "D4", "D5"]),
            source: Arc::from("synthetic"),
        };

        let perturbations = vec![PerturbationSignature {
            compound_id: CompoundId::new("CHEMBL1"),
            cell_line: CellLineId::new("A549"),
            dose_um: 10.0,
            duration_h: 24.0,
            up_genes: genes(&["D1", "D2", "D3", "X1", "X2"]),
            down_genes: genes(&["U1", "U2", "U3", "Y1", "Y2"]),
        }];

        let pipeline = RgesPipeline {
            enrichment_config: EnrichmentConfig {
                n_permutations: 50,
                min_gene_set_size: 5,
                ..EnrichmentConfig::default()
            },
            screen_config: ScreenConfig {
                fdr_threshold: 1.0,
                ..ScreenConfig::default()
            },
        };

        let hits = pipeline
            .run(&disease, &perturbations)
            .expect("pipeline run");
        assert_eq!(hits.len(), 1);
        assert!(hits[0].reversal_strength > 0.0);
        assert!(hits[0].adjusted_p_value <= 1.0);
    }

    #[test]
    fn pipeline_new_uses_provided_config() {
        let enrichment = EnrichmentConfig {
            n_permutations: 42,
            min_gene_set_size: 3,
            ..EnrichmentConfig::default()
        };
        let screen = ScreenConfig {
            fdr_threshold: 0.1,
            ..ScreenConfig::default()
        };
        let pipeline = RgesPipeline::new(enrichment, screen);
        assert_eq!(pipeline.enrichment_config.n_permutations, 42);
        assert!((pipeline.screen_config.fdr_threshold - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn pipeline_default_has_standard_values() {
        let pipeline = RgesPipeline::default();
        assert!(pipeline.enrichment_config.n_permutations > 0);
    }

    #[test]
    fn pipeline_rejects_empty_perturbations() {
        let disease = DiseaseSignature {
            name: Arc::from("test"),
            up_genes: genes(&["U1", "U2", "U3", "U4", "U5"]),
            down_genes: genes(&["D1", "D2", "D3", "D4", "D5"]),
            source: Arc::from("synthetic"),
        };
        let pipeline = RgesPipeline::default();
        let err = pipeline.run(&disease, &[]);
        assert!(err.is_err());
    }
}
