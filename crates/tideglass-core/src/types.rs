// SPDX-License-Identifier: AGPL-3.0-or-later

//! Zero-copy domain types for gene, compound, and signature identifiers.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Gene identifier — zero-copy via `Arc<str>` for O(1) clone on hot paths.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GeneId(pub Arc<str>);

impl GeneId {
    /// Creates a gene identifier from a string slice.
    #[must_use]
    pub fn new(id: &str) -> Self {
        Self(Arc::from(id))
    }

    /// Returns the underlying gene identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Compound identifier (`ChEMBL` ID, `ZINC` ID, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompoundId(pub Arc<str>);

impl CompoundId {
    /// Creates a compound identifier from a string slice.
    #[must_use]
    pub fn new(id: &str) -> Self {
        Self(Arc::from(id))
    }

    /// Returns the underlying compound identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Cell line identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CellLineId(pub Arc<str>);

impl CellLineId {
    /// Creates a cell line identifier from a string slice.
    #[must_use]
    pub fn new(id: &str) -> Self {
        Self(Arc::from(id))
    }

    /// Returns the underlying cell line identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Disease expression signature: gene-level fold changes with direction.
///
/// Derived from GEO/TCGA expression data via differential analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiseaseSignature {
    /// Human-readable name for this disease signature.
    pub name: Arc<str>,
    /// Genes upregulated in disease relative to control.
    pub up_genes: Vec<GeneId>,
    /// Genes downregulated in disease relative to control.
    pub down_genes: Vec<GeneId>,
    /// Provenance string (e.g. GEO accession, TCGA cohort).
    pub source: Arc<str>,
}

/// LINCS L1000 perturbation record: drug-induced gene expression changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerturbationSignature {
    /// Compound that produced this perturbation profile.
    pub compound_id: CompoundId,
    /// Cell line in which the perturbation was measured.
    pub cell_line: CellLineId,
    /// Dose in micromolar.
    pub dose_um: f64,
    /// Exposure duration in hours.
    pub duration_h: f64,
    /// Genes upregulated by the compound (ranked, strongest first).
    pub up_genes: Vec<GeneId>,
    /// Genes downregulated by the compound (ranked, strongest first).
    pub down_genes: Vec<GeneId>,
}

/// Reversal Gene Expression Score result for a single compound.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RgesResult {
    /// Compound evaluated in this RGES screen.
    pub compound_id: CompoundId,
    /// Combined reversal score (midpoint of up/down reversals).
    pub rges_score: f64,
    /// Permutation-based p-value for the combined enrichment.
    pub p_value: f64,
    /// Absolute reversal strength derived from `rges_score`.
    pub reversal_strength: f64,
    /// Number of permutations used for p-value estimation.
    pub n_permutations: u32,
}

/// Configuration for enrichment computation — no hardcoded values at call sites.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichmentConfig {
    /// Number of permutations for p-value estimation (default: `10_000`).
    pub n_permutations: u32,
    /// Enrichment weight exponent for weighted KS (default: 1.0).
    pub weight_exponent: f64,
    /// Minimum gene set size to consider (default: 5).
    pub min_gene_set_size: usize,
    /// FDR threshold for significance (default: 0.05).
    pub fdr_threshold: f64,
}

impl Default for EnrichmentConfig {
    fn default() -> Self {
        Self {
            n_permutations: 10_000,
            weight_exponent: 1.0,
            min_gene_set_size: 5,
            fdr_threshold: 0.05,
        }
    }
}

/// Validation target for a module — configurable, not hardcoded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationTarget {
    /// Name of the metric being validated.
    pub metric_name: Arc<str>,
    /// Threshold value for pass/fail comparison.
    pub threshold: f64,
    /// Comparison operator applied to the metric.
    pub comparison: Comparison,
    /// Reference dataset or benchmark identifier.
    pub reference: Arc<str>,
}

/// Comparison operator for validation targets.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Comparison {
    /// Metric must be greater than or equal to the threshold.
    GreaterOrEqual,
    /// Metric must be less than or equal to the threshold.
    LessOrEqual,
    /// Metric must be within `tolerance_pct` percent of the threshold.
    Within {
        /// Allowed deviation as a percentage of the threshold.
        tolerance_pct: u8,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gene_id_zero_copy_clone() {
        let a = GeneId::new("TP53");
        let b = a.clone();
        assert_eq!(a, b);
        assert!(Arc::ptr_eq(&a.0, &b.0));
    }

    #[test]
    fn identifiers_round_trip_json() {
        let gene = GeneId::new("BRCA1");
        let compound = CompoundId::new("CHEMBL25");
        let cell = CellLineId::new("A549");

        let gene_json = serde_json::to_string(&gene).expect("serialize gene");
        let gene_back: GeneId = serde_json::from_str(&gene_json).expect("deserialize gene");
        assert_eq!(gene, gene_back);

        let compound_json = serde_json::to_string(&compound).expect("serialize compound");
        let compound_back: CompoundId =
            serde_json::from_str(&compound_json).expect("deserialize compound");
        assert_eq!(compound, compound_back);

        let cell_json = serde_json::to_string(&cell).expect("serialize cell line");
        let cell_back: CellLineId = serde_json::from_str(&cell_json).expect("deserialize cell");
        assert_eq!(cell, cell_back);
    }

    #[test]
    fn enrichment_config_default_values() {
        let config = EnrichmentConfig::default();
        assert_eq!(config.n_permutations, 10_000);
        assert!((config.weight_exponent - 1.0).abs() < f64::EPSILON);
        assert_eq!(config.min_gene_set_size, 5);
        assert!((config.fdr_threshold - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn disease_signature_serializes() {
        let sig = DiseaseSignature {
            name: Arc::from("melanoma"),
            up_genes: vec![GeneId::new("GENE1")],
            down_genes: vec![GeneId::new("GENE2")],
            source: Arc::from("GSE12345"),
        };
        let json = serde_json::to_string(&sig).expect("serialize disease signature");
        let back: DiseaseSignature = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.name.as_ref(), "melanoma");
        assert_eq!(back.up_genes.len(), 1);
    }

    #[test]
    fn comparison_within_variant_serializes() {
        let target = ValidationTarget {
            metric_name: Arc::from("auroc"),
            threshold: 0.85,
            comparison: Comparison::Within { tolerance_pct: 5 },
            reference: Arc::from("octad-v2"),
        };
        let json = serde_json::to_string(&target).expect("serialize validation target");
        assert!(json.contains("Within"));
        assert!(json.contains("tolerance_pct"));
    }
}
