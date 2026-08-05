// SPDX-License-Identifier: AGPL-3.0-or-later

//! P0 declarative scene builders for petalTongue WebGL visualization.
//!
//! Each function converts tideGlass science output into a petalTongue-compatible
//! scene JSON payload for `visualization.render.scene`. tideGlass never renders
//! directly — petalTongue handles all WebGL/WASM rendering.
//!
//! Scene format follows `specs/VISUALIZATION.md`:
//! ```json
//! { "scene": "<name>", "data": {...}, "format": "webgl", "interactive": true }
//! ```

use serde_json::{Value, json};
use tideglass_molsearch::MctsResult;
use tideglass_nf::NfReversalScore;
use tideglass_rges::RankedRgesHit;

/// Builds an RGES volcano plot scene: RGES score (x) vs -log10(p) (y).
///
/// Highlights the top 10 compounds by reversal strength. This is the P0
/// core output visualization for the GPS platform.
#[must_use]
pub fn rges_volcano(hits: &[RankedRgesHit], top_n: usize) -> Value {
    let mut sorted: Vec<&RankedRgesHit> = hits.iter().collect();
    sorted.sort_by(|a, b| {
        b.reversal_strength
            .partial_cmp(&a.reversal_strength)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let highlight: Vec<&str> = sorted
        .iter()
        .take(top_n)
        .map(|hit| hit.compound_id.as_str())
        .collect();

    let points: Vec<Value> = hits
        .iter()
        .map(|hit| {
            let neg_log10_p = if hit.p_value > 0.0 {
                -hit.p_value.log10()
            } else {
                f64::MAX.min(300.0)
            };
            json!({
                "x": hit.rges_score,
                "y": neg_log10_p,
                "label": hit.compound_id.as_str(),
                "reversal_strength": hit.reversal_strength,
                "p_value": hit.p_value,
                "adjusted_p_value": hit.adjusted_p_value,
            })
        })
        .collect();

    json!({
        "scene": "rges_volcano",
        "data": {
            "points": points,
            "highlight": highlight,
            "axes": {
                "x": {"label": "RGES Score", "description": "Reversal Gene Expression Score"},
                "y": {"label": "-log10(p)", "description": "Negative log10 p-value"},
            },
            "total_compounds": hits.len(),
            "highlighted_count": highlight.len(),
        },
        "format": "webgl",
        "interactive": true,
    })
}

/// Builds an enrichment curve scene: cumulative recovery of known actives.
///
/// ROC-style curve where x = fraction of library screened, y = fraction of
/// known actives recovered. Used for OCTAD benchmark validation.
#[must_use]
pub fn enrichment_curve(hits: &[RankedRgesHit], known_active_ids: &[&str]) -> Value {
    let known_set: std::collections::HashSet<&str> = known_active_ids.iter().copied().collect();
    let total_actives = known_set.len();

    let mut recovered = 0usize;
    let n = hits.len();
    let mut curve_points: Vec<Value> = Vec::with_capacity(n + 1);

    curve_points.push(json!({"x": 0.0, "y": 0.0}));

    for (i, hit) in hits.iter().enumerate() {
        if known_set.contains(hit.compound_id.as_str()) {
            recovered += 1;
        }
        let x_frac = tideglass_core::count_as_f64(i + 1) / tideglass_core::count_as_f64(n).max(1.0);
        let y_frac = if total_actives > 0 {
            tideglass_core::count_as_f64(recovered) / tideglass_core::count_as_f64(total_actives)
        } else {
            0.0
        };
        curve_points.push(json!({"x": x_frac, "y": y_frac, "compound": hit.compound_id.as_str()}));
    }

    json!({
        "scene": "enrichment_curve",
        "data": {
            "curve": curve_points,
            "total_screened": n,
            "total_actives": total_actives,
            "recovered_actives": recovered,
            "axes": {
                "x": {"label": "Fraction Screened"},
                "y": {"label": "Fraction of Known Actives Recovered"},
            },
        },
        "format": "webgl",
        "interactive": true,
    })
}

/// Builds an NF candidate dashboard scene: ranked NF candidates with sparklines.
///
/// Table visualization with compound ranking, weighted/standard RGES scores,
/// p-values, and per-compound sparkline data for the NF extension module.
#[must_use]
pub fn nf_dashboard(scores: &[NfReversalScore]) -> Value {
    let mut sorted: Vec<&NfReversalScore> = scores.iter().collect();
    sorted.sort_by(|a, b| {
        b.reversal_strength
            .partial_cmp(&a.reversal_strength)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let rows: Vec<Value> = sorted
        .iter()
        .enumerate()
        .map(|(rank, score)| {
            json!({
                "rank": rank + 1,
                "compound_id": score.compound_id.as_str(),
                "weighted_rges": score.weighted_rges,
                "standard_rges": score.standard_rges,
                "p_value": score.p_value,
                "reversal_strength": score.reversal_strength,
                "geometry_scale_d": score.geometry_scale_d,
                "sparkline": [score.standard_rges, score.weighted_rges],
            })
        })
        .collect();

    json!({
        "scene": "nf_candidate_dashboard",
        "data": {
            "rows": rows,
            "total_candidates": scores.len(),
            "columns": [
                {"key": "rank", "label": "Rank"},
                {"key": "compound_id", "label": "Compound"},
                {"key": "weighted_rges", "label": "Weighted RGES"},
                {"key": "standard_rges", "label": "Standard RGES"},
                {"key": "p_value", "label": "p-value"},
                {"key": "reversal_strength", "label": "Reversal"},
                {"key": "sparkline", "label": "Trend"},
            ],
        },
        "format": "webgl",
        "interactive": true,
    })
}

/// Builds a `GPS4Drug` prediction accuracy scatter: predicted vs observed expression.
///
/// Each point represents a gene's predicted expression change vs the
/// experimentally measured value. The diagonal y=x line indicates perfect
/// prediction.
#[must_use]
pub fn gps4drug_scatter(predicted: &[f64], observed: &[f64], gene_labels: &[&str]) -> Value {
    let n = predicted.len().min(observed.len());
    let points: Vec<Value> = (0..n)
        .map(|i| {
            let label = gene_labels.get(i).copied().unwrap_or("—");
            json!({
                "x": observed[i],
                "y": predicted[i],
                "label": label,
            })
        })
        .collect();

    let (r_squared, rmse) = compute_regression_stats(predicted, observed);

    json!({
        "scene": "gps4drug_prediction_scatter",
        "data": {
            "points": points,
            "reference_line": {"slope": 1.0, "intercept": 0.0, "label": "y = x (perfect)"},
            "stats": {
                "r_squared": r_squared,
                "rmse": rmse,
                "n_genes": n,
            },
            "axes": {
                "x": {"label": "Observed Expression Change"},
                "y": {"label": "Predicted Expression Change"},
            },
        },
        "format": "webgl",
        "interactive": true,
    })
}

/// Builds an MCTS optimization trace: multi-line convergence over iterations.
///
/// Shows how the best reward evolves over MCTS iterations, with the action
/// path annotated at key improvement points.
#[must_use]
pub fn mcts_trace(result: &MctsResult) -> Value {
    let action_labels: Vec<Value> = result
        .action_path
        .iter()
        .enumerate()
        .map(|(step, action)| {
            json!({
                "step": step,
                "action": format!("{action:?}"),
            })
        })
        .collect();

    let best_features = &result.best_state;
    let property_traces = json!({
        "molecular_weight": best_features.properties.molecular_weight,
        "log_p": best_features.properties.log_p,
        "tpsa": best_features.properties.tpsa,
        "hba": best_features.properties.hba,
        "hbd": best_features.properties.hbd,
        "rotatable_bonds": best_features.properties.rotatable_bonds,
        "aromatic_rings": best_features.properties.aromatic_rings,
    });

    json!({
        "scene": "mcts_optimization_trace",
        "data": {
            "best_reward": result.best_reward,
            "iterations_run": result.iterations_run,
            "action_path": action_labels,
            "final_properties": property_traces,
            "compound_id": best_features.compound_id.as_str(),
            "axes": {
                "x": {"label": "MCTS Iteration"},
                "y": {"label": "Best Reward"},
            },
        },
        "format": "webgl",
        "interactive": true,
    })
}

/// Builds a data catalog scene: interactive table of CAS datasets.
#[must_use]
pub fn data_catalog(datasets: &[CatalogEntry]) -> Value {
    let rows: Vec<Value> = datasets
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            json!({
                "index": i,
                "key": entry.key,
                "status": entry.status,
                "description": entry.description,
                "cas_hash": entry.cas_hash.as_deref().unwrap_or("—"),
                "size_bytes": entry.size_bytes,
            })
        })
        .collect();

    json!({
        "scene": "data_catalog",
        "data": {
            "rows": rows,
            "total_datasets": datasets.len(),
            "columns": [
                {"key": "key", "label": "Dataset"},
                {"key": "status", "label": "Status"},
                {"key": "description", "label": "Description"},
                {"key": "cas_hash", "label": "CAS Hash"},
                {"key": "size_bytes", "label": "Size (bytes)"},
            ],
        },
        "format": "webgl",
        "interactive": true,
    })
}

/// A dataset entry for the data catalog visualization.
#[derive(Debug, Clone)]
pub struct CatalogEntry {
    pub key: String,
    pub status: String,
    pub description: String,
    pub cas_hash: Option<String>,
    pub size_bytes: u64,
}

fn compute_regression_stats(predicted: &[f64], observed: &[f64]) -> (f64, f64) {
    let n = predicted.len().min(observed.len());
    if n == 0 {
        return (0.0, 0.0);
    }

    let n_f = tideglass_core::count_as_f64(n);
    let mean_obs = observed.iter().take(n).sum::<f64>() / n_f;

    let ss_res: f64 = predicted
        .iter()
        .zip(observed.iter())
        .take(n)
        .map(|(p, o)| (o - p).powi(2))
        .sum();

    let ss_tot: f64 = observed
        .iter()
        .take(n)
        .map(|o| (o - mean_obs).powi(2))
        .sum();

    let r_squared = if ss_tot > 0.0 {
        1.0 - (ss_res / ss_tot)
    } else {
        0.0
    };

    let rmse = (ss_res / n_f).sqrt();
    (r_squared, rmse)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tideglass_core::types::CompoundId;
    use tideglass_gps4drug::{MolecularFeatures, PhysicochemicalProperties};
    use tideglass_molsearch::{MctsResult, MolecularAction};

    use super::*;

    fn sample_hits() -> Vec<RankedRgesHit> {
        vec![
            RankedRgesHit {
                compound_id: CompoundId::new("CHEMBL1"),
                rges_score: 0.85,
                p_value: 0.001,
                adjusted_p_value: 0.005,
                reversal_strength: 0.85,
                n_permutations: 1000,
            },
            RankedRgesHit {
                compound_id: CompoundId::new("CHEMBL2"),
                rges_score: 0.60,
                p_value: 0.01,
                adjusted_p_value: 0.03,
                reversal_strength: 0.60,
                n_permutations: 1000,
            },
            RankedRgesHit {
                compound_id: CompoundId::new("CHEMBL3"),
                rges_score: 0.40,
                p_value: 0.05,
                adjusted_p_value: 0.10,
                reversal_strength: 0.40,
                n_permutations: 1000,
            },
        ]
    }

    fn sample_nf_scores() -> Vec<NfReversalScore> {
        vec![
            NfReversalScore {
                compound_id: CompoundId::new("NF_DRUG_1"),
                weighted_rges: 0.75,
                standard_rges: 0.60,
                p_value: 0.002,
                reversal_strength: 0.75,
                geometry_scale_d: 2.0,
            },
            NfReversalScore {
                compound_id: CompoundId::new("NF_DRUG_2"),
                weighted_rges: 0.55,
                standard_rges: 0.50,
                p_value: 0.01,
                reversal_strength: 0.55,
                geometry_scale_d: 2.0,
            },
        ]
    }

    fn sample_mcts_result() -> MctsResult {
        MctsResult {
            best_state: MolecularFeatures {
                compound_id: CompoundId::new("OPTIMIZED"),
                smiles: Some(Arc::from("CCO")),
                fingerprint_bits: vec![1, 0, 1, 1],
                properties: PhysicochemicalProperties {
                    molecular_weight: 320.0,
                    log_p: 2.5,
                    tpsa: 60.0,
                    hba: 3,
                    hbd: 1,
                    rotatable_bonds: 4,
                    aromatic_rings: 2,
                },
            },
            best_reward: 0.87,
            action_path: vec![
                MolecularAction::AddSubstituent {
                    site_index: 0,
                    group: Arc::from("hydroxyl"),
                },
                MolecularAction::RemoveGroup { site_index: 1 },
            ],
            iterations_run: 100,
        }
    }

    #[test]
    fn rges_volcano_builds_valid_scene() {
        let hits = sample_hits();
        let scene = rges_volcano(&hits, 2);

        assert_eq!(scene["scene"], "rges_volcano");
        assert_eq!(scene["format"], "webgl");
        assert_eq!(scene["interactive"], true);
        assert_eq!(scene["data"]["total_compounds"], 3);
        assert_eq!(scene["data"]["highlighted_count"], 2);

        let points = scene["data"]["points"].as_array().expect("points array");
        assert_eq!(points.len(), 3);

        let highlight = scene["data"]["highlight"]
            .as_array()
            .expect("highlight array");
        assert_eq!(highlight.len(), 2);
        assert_eq!(highlight[0], "CHEMBL1");
    }

    #[test]
    fn rges_volcano_empty_hits() {
        let scene = rges_volcano(&[], 10);
        assert_eq!(scene["data"]["total_compounds"], 0);
        assert_eq!(
            scene["data"]["points"]
                .as_array()
                .expect("points array")
                .len(),
            0
        );
    }

    #[test]
    fn rges_volcano_zero_p_value_capped() {
        let hits = vec![RankedRgesHit {
            compound_id: CompoundId::new("ZERO_P"),
            rges_score: 0.99,
            p_value: 0.0,
            adjusted_p_value: 0.0,
            reversal_strength: 0.99,
            n_permutations: 10_000,
        }];
        let scene = rges_volcano(&hits, 1);
        let y = scene["data"]["points"][0]["y"].as_f64().expect("y value");
        assert!(y.is_finite());
        assert!(y > 0.0);
    }

    #[test]
    fn enrichment_curve_builds_valid_scene() {
        let hits = sample_hits();
        let scene = enrichment_curve(&hits, &["CHEMBL1", "CHEMBL3"]);

        assert_eq!(scene["scene"], "enrichment_curve");
        assert_eq!(scene["data"]["total_screened"], 3);
        assert_eq!(scene["data"]["total_actives"], 2);
        assert_eq!(scene["data"]["recovered_actives"], 2);

        let curve = scene["data"]["curve"].as_array().expect("curve array");
        assert_eq!(curve.len(), 4);
        assert_eq!(curve[0]["x"], 0.0);
        assert_eq!(curve[0]["y"], 0.0);
    }

    #[test]
    fn enrichment_curve_no_actives() {
        let hits = sample_hits();
        let scene = enrichment_curve(&hits, &[]);

        assert_eq!(scene["data"]["total_actives"], 0);
        assert_eq!(scene["data"]["recovered_actives"], 0);
    }

    #[test]
    fn enrichment_curve_empty_hits() {
        let scene = enrichment_curve(&[], &["A", "B"]);
        assert_eq!(scene["data"]["total_screened"], 0);
        let curve = scene["data"]["curve"].as_array().expect("curve array");
        assert_eq!(curve.len(), 1);
    }

    #[test]
    fn nf_dashboard_builds_valid_scene() {
        let scores = sample_nf_scores();
        let scene = nf_dashboard(&scores);

        assert_eq!(scene["scene"], "nf_candidate_dashboard");
        assert_eq!(scene["data"]["total_candidates"], 2);

        let rows = scene["data"]["rows"].as_array().expect("rows array");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["rank"], 1);
        assert_eq!(rows[0]["compound_id"], "NF_DRUG_1");
    }

    #[test]
    fn nf_dashboard_sorted_by_reversal_strength() {
        let scores = sample_nf_scores();
        let scene = nf_dashboard(&scores);
        let rows = scene["data"]["rows"].as_array().expect("rows array");

        let first_strength = rows[0]["reversal_strength"].as_f64().unwrap();
        let second_strength = rows[1]["reversal_strength"].as_f64().unwrap();
        assert!(first_strength >= second_strength);
    }

    #[test]
    fn nf_dashboard_empty() {
        let scene = nf_dashboard(&[]);
        assert_eq!(scene["data"]["total_candidates"], 0);
    }

    #[test]
    fn gps4drug_scatter_builds_valid_scene() {
        let predicted = vec![1.0, 2.0, 3.0];
        let observed = vec![1.1, 1.9, 3.2];
        let labels = vec!["TP53", "BRCA1", "EGFR"];

        let scene = gps4drug_scatter(&predicted, &observed, &labels);

        assert_eq!(scene["scene"], "gps4drug_prediction_scatter");
        assert_eq!(scene["data"]["stats"]["n_genes"], 3);

        let points = scene["data"]["points"].as_array().expect("points array");
        assert_eq!(points.len(), 3);
        assert_eq!(points[0]["label"], "TP53");

        let r2 = scene["data"]["stats"]["r_squared"].as_f64().unwrap();
        assert!(r2 > 0.9);
    }

    #[test]
    fn gps4drug_scatter_empty() {
        let scene = gps4drug_scatter(&[], &[], &[]);
        assert_eq!(scene["data"]["stats"]["n_genes"], 0);
    }

    #[test]
    fn gps4drug_scatter_mismatched_lengths_uses_min() {
        let predicted = vec![1.0, 2.0];
        let observed = vec![1.1];
        let scene = gps4drug_scatter(&predicted, &observed, &[]);
        assert_eq!(scene["data"]["stats"]["n_genes"], 1);
    }

    #[test]
    fn mcts_trace_builds_valid_scene() {
        let result = sample_mcts_result();
        let scene = mcts_trace(&result);

        assert_eq!(scene["scene"], "mcts_optimization_trace");
        assert_eq!(scene["data"]["iterations_run"], 100);

        let reward = scene["data"]["best_reward"].as_f64().unwrap();
        assert!((reward - 0.87).abs() < f64::EPSILON);

        let actions = scene["data"]["action_path"]
            .as_array()
            .expect("action_path array");
        assert_eq!(actions.len(), 2);
    }

    #[test]
    fn data_catalog_builds_valid_scene() {
        let entries = vec![
            CatalogEntry {
                key: "gps_platform".to_owned(),
                status: "loaded".to_owned(),
                description: "GPS platform data (1.4 GB)".to_owned(),
                cas_hash: Some("abc123".to_owned()),
                size_bytes: 1_400_000_000,
            },
            CatalogEntry {
                key: "nf_data_portal".to_owned(),
                status: "ingesting".to_owned(),
                description: "NF Data Portal gene expression".to_owned(),
                cas_hash: None,
                size_bytes: 0,
            },
        ];

        let scene = data_catalog(&entries);
        assert_eq!(scene["scene"], "data_catalog");
        assert_eq!(scene["data"]["total_datasets"], 2);

        let rows = scene["data"]["rows"].as_array().expect("rows array");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["key"], "gps_platform");
        assert_eq!(rows[1]["cas_hash"], "—");
    }

    #[test]
    fn data_catalog_empty() {
        let scene = data_catalog(&[]);
        assert_eq!(scene["data"]["total_datasets"], 0);
    }

    #[test]
    fn regression_stats_perfect_prediction() {
        let (r2, rmse) = compute_regression_stats(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]);
        assert!((r2 - 1.0).abs() < f64::EPSILON);
        assert!(rmse.abs() < f64::EPSILON);
    }

    #[test]
    fn regression_stats_empty() {
        let (r2, rmse) = compute_regression_stats(&[], &[]);
        assert!((r2).abs() < f64::EPSILON);
        assert!((rmse).abs() < f64::EPSILON);
    }
}
