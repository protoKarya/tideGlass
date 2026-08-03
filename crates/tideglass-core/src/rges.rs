// SPDX-License-Identifier: AGPL-3.0-or-later
// Reversal Gene Expression Score (RGES) pipeline
//
// The RGES approach identifies drugs that reverse disease gene expression
// signatures. This module implements the core computation:
//   1. Ingest disease expression profile (from GEO/TCGA via westGate CAS)
//   2. Query LINCS L1000 perturbation signatures
//   3. Compute RGES = enrichment of disease-up genes in drug-down signature
//   4. Rank compounds by reversal strength
//
// Data sources (all on westGate ZFS, accessible via content.get):
//   - ChEMBL 35 (4.9 GB) — compound bioactivity
//   - LINCS L1000 (12 GB) — perturbation signatures
//   - PubChem BioAssay (8.2 GB) — assay results
//   - GEO SOFT (1.8 GB) — expression profiles
//   - TCGA Xena Hub (15 GB) — cancer expression
//   - BindingDB (3.8 GB) — binding affinities

/// Disease expression signature: gene-level fold changes
pub struct DiseaseSignature {
    pub name: String,
    pub up_genes: Vec<String>,
    pub down_genes: Vec<String>,
    pub source: String,
}

/// LINCS perturbation record
pub struct PerturbationSignature {
    pub compound_id: String,
    pub cell_line: String,
    pub dose_um: f64,
    pub duration_h: f64,
    pub up_genes: Vec<String>,
    pub down_genes: Vec<String>,
}

/// Reversal Gene Expression Score result
pub struct RgesResult {
    pub compound_id: String,
    pub rges_score: f64,
    pub p_value: f64,
    pub reversal_strength: f64,
}

/// Compute RGES for a disease signature against a set of perturbation signatures
pub fn compute_rges(
    disease: &DiseaseSignature,
    perturbations: &[PerturbationSignature],
) -> Vec<RgesResult> {
    perturbations
        .iter()
        .map(|pert| {
            let up_reversal = gene_set_enrichment(&disease.up_genes, &pert.down_genes);
            let down_reversal = gene_set_enrichment(&disease.down_genes, &pert.up_genes);
            let rges = (up_reversal + down_reversal) / 2.0;

            RgesResult {
                compound_id: pert.compound_id.clone(),
                rges_score: rges,
                p_value: 1.0, // TODO: permutation test
                reversal_strength: rges.abs(),
            }
        })
        .collect()
}

fn gene_set_enrichment(query_genes: &[String], target_genes: &[String]) -> f64 {
    if query_genes.is_empty() || target_genes.is_empty() {
        return 0.0;
    }
    let overlap = query_genes
        .iter()
        .filter(|g| target_genes.contains(g))
        .count();
    overlap as f64 / query_genes.len() as f64
}
