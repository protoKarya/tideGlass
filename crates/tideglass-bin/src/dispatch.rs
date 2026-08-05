// SPDX-License-Identifier: AGPL-3.0-or-later

//! JSON-RPC 2.0 method dispatch router for tideGlass science modules.

use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;
use tideglass_core::ipc::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, methods};
use tideglass_core::types::{DiseaseSignature, EnrichmentConfig, PerturbationSignature};
use tideglass_gps4drug::{
    ExpressionPredictor, LinearRegressionConfig, LinearRegressionPredictor, MolecularFeatures,
};
use tideglass_molsearch::{MctsConfig, MctsSearch};
use tideglass_nf::{NfScoringConfig, compute_nf_scores};
use tideglass_octad::{BenchmarkConfig, OctadComparison, RankedCompound};
use tideglass_rcl::{RclConfig, rank_cell_lines};
use tideglass_rges::{RankedRgesHit, RgesPipeline, ScreenConfig};
use tideglass_screen::{CompoundLibrary, ScreenFilterConfig, filter_ranked_hits};

use crate::scenes;

use crate::data::ModuleData;

const JSONRPC_VERSION: &str = "2.0";
const CAS_NOT_LOADED: &str = "Module not yet data-loaded — awaiting CAS initialization";

/// Dispatches a single NDJSON-framed JSON-RPC request line to the appropriate handler.
#[must_use]
pub fn dispatch_request(raw: &str, module_data: &ModuleData) -> JsonRpcResponse {
    let request: JsonRpcRequest = match serde_json::from_str(raw) {
        Ok(req) => req,
        Err(error) => {
            return error_response(Value::Null, -32_700, &format!("Parse error: {error}"));
        }
    };

    let result = match request.method.as_ref() {
        methods::CAPABILITIES_LIST => Ok(crate::capabilities::list()),
        methods::HEALTH_LIVENESS => Ok(crate::health::liveness()),
        methods::HEALTH_CHECK => Ok(crate::health::check_with_cas(module_data)),
        methods::HEALTH_READINESS => Ok(crate::health::readiness_with_cas(module_data)),
        methods::RGES_SCREEN => handle_rges_screen(request.params.as_ref()),
        methods::RCL_SELECT => handle_rcl_select(request.params.as_ref()),
        methods::GPS4DRUG_PREDICT => handle_gps4drug_predict(request.params.as_ref(), module_data),
        methods::COMPOUND_SCREEN => handle_compound_screen(request.params.as_ref(), module_data),
        methods::MCTS_OPTIMIZE => handle_mcts_optimize(request.params.as_ref()),
        methods::OCTAD_BENCHMARK => handle_octad_benchmark(request.params.as_ref(), module_data),
        methods::NF_SCORE => handle_nf_score(request.params.as_ref()),
        methods::VIZ_RGES_VOLCANO => handle_viz_rges_volcano(request.params.as_ref()),
        methods::VIZ_ENRICHMENT_CURVE => {
            handle_viz_enrichment_curve(request.params.as_ref(), module_data)
        }
        methods::VIZ_NF_DASHBOARD => handle_viz_nf_dashboard(request.params.as_ref()),
        methods::VIZ_GPS4DRUG_SCATTER => handle_viz_gps4drug_scatter(request.params.as_ref()),
        methods::VIZ_MCTS_TRACE => handle_viz_mcts_trace(request.params.as_ref()),
        methods::DATA_CATALOG => Ok(handle_data_catalog(module_data)),
        other => Err((-32_601, format!("Method not found: {other}"))),
    };

    match result {
        Ok(value) => success_response(request.id, value),
        Err((code, message)) => error_response(request.id, code, &message),
    }
}

#[derive(Debug, Deserialize)]
struct RgesScreenParams {
    disease: DiseaseSignature,
    perturbations: Vec<PerturbationSignature>,
    #[serde(default)]
    enrichment_config: EnrichmentConfig,
    #[serde(default)]
    screen_config: ScreenConfig,
}

fn handle_rges_screen(params: Option<&Value>) -> Result<Value, (i64, String)> {
    let params = require_params(params)?;
    let parsed: RgesScreenParams = deserialize_params(params)?;

    let pipeline = RgesPipeline {
        enrichment_config: parsed.enrichment_config,
        screen_config: parsed.screen_config,
    };

    let hits = pipeline
        .run(&parsed.disease, &parsed.perturbations)
        .map_err(|error| module_error(&error))?;

    serde_json::to_value(hits).map_err(|error| (-32_603, error.to_string()))
}

#[derive(Debug, Deserialize)]
struct RclSelectParams {
    disease: DiseaseSignature,
    perturbations: Vec<PerturbationSignature>,
    #[serde(default)]
    config: RclConfig,
}

fn handle_rcl_select(params: Option<&Value>) -> Result<Value, (i64, String)> {
    let params = require_params(params)?;
    let parsed: RclSelectParams = deserialize_params(params)?;

    let rankings = rank_cell_lines(&parsed.disease, &parsed.perturbations, &parsed.config)
        .map_err(|error| module_error(&error))?;

    serde_json::to_value(rankings).map_err(|error| (-32_603, error.to_string()))
}

#[derive(Debug, Deserialize)]
struct Gps4DrugPredictParams {
    features: MolecularFeatures,
    weights: Option<Vec<Vec<f64>>>,
    config: Option<LinearRegressionConfig>,
}

fn handle_gps4drug_predict(
    params: Option<&Value>,
    module_data: &ModuleData,
) -> Result<Value, (i64, String)> {
    let params = require_params(params)?;
    let parsed: Gps4DrugPredictParams = deserialize_params(params)?;

    let (weights, config) = match (parsed.weights, parsed.config) {
        (Some(w), Some(c)) => (w, c),
        _ => match &module_data.gps4drug_weights {
            Some(loaded) => (loaded.weights.clone(), loaded.config.clone()),
            None => return Err(cas_not_loaded()),
        },
    };

    let predictor =
        LinearRegressionPredictor::new(weights, config).map_err(|error| module_error(&error))?;
    let prediction = predictor
        .predict(&parsed.features)
        .map_err(|error| module_error(&error))?;

    serde_json::to_value(prediction).map_err(|error| (-32_603, error.to_string()))
}

#[derive(Debug, Deserialize)]
struct CompoundScreenParams {
    hits: Vec<RankedRgesHit>,
    library: Option<CompoundLibrary>,
    #[serde(default)]
    filter_config: ScreenFilterConfig,
}

fn handle_compound_screen(
    params: Option<&Value>,
    module_data: &ModuleData,
) -> Result<Value, (i64, String)> {
    let params = require_params(params)?;
    let parsed: CompoundScreenParams = deserialize_params(params)?;

    let library = match parsed.library {
        Some(lib) => lib,
        None => match &module_data.compound_library {
            Some(loaded) => loaded.clone(),
            None => return Err(cas_not_loaded()),
        },
    };
    let filtered = filter_ranked_hits(&parsed.hits, &library, &parsed.filter_config);

    serde_json::to_value(filtered).map_err(|error| (-32_603, error.to_string()))
}

#[derive(Debug, Deserialize)]
struct MctsOptimizeParams {
    initial: MolecularFeatures,
    #[serde(default)]
    config: MctsConfig,
}

fn handle_mcts_optimize(params: Option<&Value>) -> Result<Value, (i64, String)> {
    let params = require_params(params)?;
    let parsed: MctsOptimizeParams = deserialize_params(params)?;

    let search = MctsSearch::new(parsed.config, tideglass_molsearch::default_actions());
    let mut rng = rand::rng();
    let result = search
        .optimize(parsed.initial, &mut rng)
        .map_err(|error| module_error(&error))?;

    serde_json::to_value(result).map_err(|error| (-32_603, error.to_string()))
}

#[derive(Debug, Deserialize)]
struct OctadBenchmarkParams {
    ranked: Vec<RankedCompound>,
    known_actives: Option<HashMap<tideglass_core::types::CompoundId, bool>>,
    #[serde(default)]
    config: BenchmarkConfig,
}

fn handle_octad_benchmark(
    params: Option<&Value>,
    module_data: &ModuleData,
) -> Result<Value, (i64, String)> {
    let params = require_params(params)?;
    let parsed: OctadBenchmarkParams = deserialize_params(params)?;

    let known_actives = match parsed.known_actives {
        Some(actives) => actives,
        None => match &module_data.known_actives {
            Some(loaded) => loaded.clone(),
            None => return Err(cas_not_loaded()),
        },
    };
    let comparison = OctadComparison::new(parsed.config);
    let result = comparison
        .evaluate_gps(&parsed.ranked, &known_actives)
        .map_err(|error| module_error(&error))?;

    serde_json::to_value(result).map_err(|error| (-32_603, error.to_string()))
}

#[derive(Debug, Deserialize)]
struct NfScoreParams {
    disease: DiseaseSignature,
    perturbations: Vec<PerturbationSignature>,
    #[serde(default)]
    config: NfScoringConfig,
}

fn handle_nf_score(params: Option<&Value>) -> Result<Value, (i64, String)> {
    let params = require_params(params)?;
    let parsed: NfScoreParams = deserialize_params(params)?;

    let scores = compute_nf_scores(&parsed.disease, &parsed.perturbations, &parsed.config)
        .map_err(|error| module_error(&error))?;

    serde_json::to_value(scores).map_err(|error| (-32_603, error.to_string()))
}

fn require_params(params: Option<&Value>) -> Result<&Value, (i64, String)> {
    params.ok_or_else(|| {
        (
            -32_602,
            "Invalid params: missing request parameters".to_owned(),
        )
    })
}

fn deserialize_params<T: for<'de> Deserialize<'de>>(params: &Value) -> Result<T, (i64, String)> {
    serde_json::from_value(params.clone())
        .map_err(|error| (-32_602, format!("Invalid params: {error}")))
}

fn module_error(error: &tideglass_core::error::TideGlassError) -> (i64, String) {
    (-32_603, error.to_string())
}

fn cas_not_loaded() -> (i64, String) {
    (-32_603, CAS_NOT_LOADED.to_owned())
}

// --- Visualization handlers ---

#[derive(Debug, Deserialize)]
struct VizRgesVolcanoParams {
    disease: DiseaseSignature,
    perturbations: Vec<PerturbationSignature>,
    #[serde(default)]
    enrichment_config: EnrichmentConfig,
    #[serde(default)]
    screen_config: ScreenConfig,
    #[serde(default = "default_top_n")]
    top_n: usize,
}

const fn default_top_n() -> usize {
    10
}

fn handle_viz_rges_volcano(params: Option<&Value>) -> Result<Value, (i64, String)> {
    let params = require_params(params)?;
    let parsed: VizRgesVolcanoParams = deserialize_params(params)?;

    let pipeline = RgesPipeline {
        enrichment_config: parsed.enrichment_config,
        screen_config: parsed.screen_config,
    };

    let hits = pipeline
        .run(&parsed.disease, &parsed.perturbations)
        .map_err(|error| module_error(&error))?;

    Ok(scenes::rges_volcano(&hits, parsed.top_n))
}

#[derive(Debug, Deserialize)]
struct VizEnrichmentCurveParams {
    disease: DiseaseSignature,
    perturbations: Vec<PerturbationSignature>,
    known_active_ids: Option<Vec<String>>,
    #[serde(default)]
    enrichment_config: EnrichmentConfig,
    #[serde(default)]
    screen_config: ScreenConfig,
}

fn handle_viz_enrichment_curve(
    params: Option<&Value>,
    module_data: &ModuleData,
) -> Result<Value, (i64, String)> {
    let params = require_params(params)?;
    let parsed: VizEnrichmentCurveParams = deserialize_params(params)?;

    let pipeline = RgesPipeline {
        enrichment_config: parsed.enrichment_config,
        screen_config: parsed.screen_config,
    };

    let hits = pipeline
        .run(&parsed.disease, &parsed.perturbations)
        .map_err(|error| module_error(&error))?;

    let active_ids: Vec<String> = parsed.known_active_ids.unwrap_or_else(|| {
        module_data
            .known_actives
            .as_ref()
            .map(|actives| actives.keys().map(|k| k.as_str().to_owned()).collect())
            .unwrap_or_default()
    });

    let active_refs: Vec<&str> = active_ids.iter().map(String::as_str).collect();
    Ok(scenes::enrichment_curve(&hits, &active_refs))
}

#[derive(Debug, Deserialize)]
struct VizNfDashboardParams {
    disease: DiseaseSignature,
    perturbations: Vec<PerturbationSignature>,
    #[serde(default)]
    config: NfScoringConfig,
}

fn handle_viz_nf_dashboard(params: Option<&Value>) -> Result<Value, (i64, String)> {
    let params = require_params(params)?;
    let parsed: VizNfDashboardParams = deserialize_params(params)?;

    let scores = compute_nf_scores(&parsed.disease, &parsed.perturbations, &parsed.config)
        .map_err(|error| module_error(&error))?;

    Ok(scenes::nf_dashboard(&scores))
}

#[derive(Debug, Deserialize)]
struct VizGps4DrugScatterParams {
    features: MolecularFeatures,
    weights: Option<Vec<Vec<f64>>>,
    config: Option<LinearRegressionConfig>,
    gene_labels: Option<Vec<String>>,
}

fn handle_viz_gps4drug_scatter(params: Option<&Value>) -> Result<Value, (i64, String)> {
    let params = require_params(params)?;
    let parsed: VizGps4DrugScatterParams = deserialize_params(params)?;

    let (Some(weights), Some(config)) = (parsed.weights, parsed.config) else {
        return Err(cas_not_loaded());
    };

    let predictor =
        LinearRegressionPredictor::new(weights, config).map_err(|error| module_error(&error))?;
    let prediction = predictor
        .predict(&parsed.features)
        .map_err(|error| module_error(&error))?;

    let predicted_values: Vec<f64> = prediction
        .genes
        .iter()
        .map(|g| g.log2_fold_change)
        .collect();

    // Use predicted as observed when no experimental data is supplied —
    // the scatter will show the model's self-consistency (y=x line).
    let observed_values = predicted_values.clone();

    let gene_labels: Vec<&str> = match &parsed.gene_labels {
        Some(labels) => labels.iter().map(String::as_str).collect(),
        None => prediction
            .genes
            .iter()
            .map(|g| g.gene_id.as_str())
            .collect(),
    };

    Ok(scenes::gps4drug_scatter(
        &predicted_values,
        &observed_values,
        &gene_labels,
    ))
}

#[derive(Debug, Deserialize)]
struct VizMctsTraceParams {
    initial: MolecularFeatures,
    #[serde(default)]
    config: MctsConfig,
}

fn handle_viz_mcts_trace(params: Option<&Value>) -> Result<Value, (i64, String)> {
    let params = require_params(params)?;
    let parsed: VizMctsTraceParams = deserialize_params(params)?;

    let search = MctsSearch::new(parsed.config, tideglass_molsearch::default_actions());
    let mut rng = rand::rng();
    let result = search
        .optimize(parsed.initial, &mut rng)
        .map_err(|error| module_error(&error))?;

    Ok(scenes::mcts_trace(&result))
}

fn handle_data_catalog(module_data: &ModuleData) -> Value {
    let datasets = vec![
        catalog_entry(
            "compound_library",
            "Compound library for screening (Module 4)",
            module_data.compound_library.is_some(),
        ),
        catalog_entry(
            "gps4drug_weights",
            "GPS4Drug weight matrix + config (Module 3)",
            module_data.gps4drug_weights.is_some(),
        ),
        catalog_entry(
            "octad_known_actives",
            "OCTAD known actives for benchmarking (Module 6)",
            module_data.known_actives.is_some(),
        ),
    ];

    scenes::data_catalog(&datasets)
}

fn catalog_entry(key: &str, description: &str, loaded: bool) -> scenes::CatalogEntry {
    scenes::CatalogEntry {
        key: key.to_owned(),
        status: if loaded {
            "loaded".to_owned()
        } else {
            "awaiting CAS".to_owned()
        },
        description: description.to_owned(),
        cas_hash: None,
        size_bytes: 0,
    }
}

fn success_response(id: Value, value: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: Arc::from(JSONRPC_VERSION),
        result: Some(value),
        error: None,
        id,
    }
}

fn error_response(id: Value, code: i64, message: &str) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: Arc::from(JSONRPC_VERSION),
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.to_owned(),
            data: None,
        }),
        id,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tideglass_core::ipc::methods;

    use super::*;

    fn rpc(method: &str, params: &serde_json::Value) -> String {
        serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        }))
        .expect("serialize request")
    }

    fn rpc_no_params(method: &str) -> String {
        serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "id": 1
        }))
        .expect("serialize request")
    }

    fn sample_disease() -> serde_json::Value {
        json!({
            "name": "test-disease",
            "up_genes": ["U1", "U2", "U3", "U4", "U5"],
            "down_genes": ["D1", "D2", "D3", "D4", "D5"],
            "source": "synthetic"
        })
    }

    fn sample_perturbation(compound_id: &str) -> serde_json::Value {
        json!({
            "compound_id": compound_id,
            "cell_line": "A549",
            "dose_um": 10.0,
            "duration_h": 24.0,
            "up_genes": ["D1", "D2", "D3", "X1", "X2"],
            "down_genes": ["U1", "U2", "U3", "Y1", "Y2"]
        })
    }

    fn varied_perturbation(compound_id: &str, suffix: &str) -> serde_json::Value {
        json!({
            "compound_id": compound_id,
            "cell_line": "A549",
            "dose_um": 10.0,
            "duration_h": 24.0,
            "up_genes": ["D1", "D2", "D3", "X1", "X2", format!("N{suffix}")],
            "down_genes": ["U1", "U2", "U3", "Y1", "Y2", format!("M{suffix}")]
        })
    }

    fn sample_features() -> serde_json::Value {
        json!({
            "compound_id": "LEAD",
            "smiles": null,
            "fingerprint_bits": [1, 0, 1],
            "properties": {
                "molecular_weight": 350.0,
                "log_p": 3.0,
                "tpsa": 70.0,
                "hba": 4,
                "hbd": 2,
                "rotatable_bonds": 3,
                "aromatic_rings": 2
            }
        })
    }

    fn fast_enrichment_config() -> serde_json::Value {
        json!({
            "n_permutations": 50,
            "weight_exponent": 1.0,
            "min_gene_set_size": 5,
            "fdr_threshold": 0.05
        })
    }

    fn permissive_screen_config() -> serde_json::Value {
        json!({
            "p_value_threshold": 1.0,
            "fdr_threshold": 1.0,
            "min_reversal_strength": 0.0
        })
    }

    fn fast_mcts_config() -> serde_json::Value {
        json!({
            "iterations": 10,
            "exploration_constant": 1.414,
            "max_depth": 4,
            "target_potency": 0.5
        })
    }

    fn default_data() -> ModuleData {
        ModuleData::default()
    }

    fn error_code(response: &JsonRpcResponse) -> i64 {
        response
            .error
            .as_ref()
            .expect("expected JSON-RPC error")
            .code
    }

    fn result_value(response: &JsonRpcResponse) -> &Value {
        response.result.as_ref().expect("expected JSON-RPC result")
    }

    #[test]
    fn parse_error_returns_invalid_request_code() {
        let response = dispatch_request("{not valid json", &default_data());
        assert_eq!(error_code(&response), -32_700);
        assert!(response.result.is_none());
    }

    #[test]
    fn missing_params_returns_invalid_params_code() {
        let response = dispatch_request(&rpc_no_params(methods::RGES_SCREEN), &default_data());
        assert_eq!(error_code(&response), -32_602);
    }

    #[test]
    fn method_not_found_returns_error_code() {
        let response = dispatch_request(&rpc_no_params("science.unknown_method"), &default_data());
        assert_eq!(error_code(&response), -32_601);
    }

    #[test]
    fn health_liveness_returns_alive() {
        let response = dispatch_request(&rpc_no_params(methods::HEALTH_LIVENESS), &default_data());
        let result = result_value(&response);
        assert_eq!(result["alive"], true);
    }

    #[test]
    fn health_check_returns_healthy_with_seven_components() {
        let response = dispatch_request(&rpc_no_params(methods::HEALTH_CHECK), &default_data());
        let result = result_value(&response);
        assert_eq!(result["status"], "healthy");
        assert_eq!(
            result["components"]
                .as_object()
                .expect("components object")
                .len(),
            7
        );
    }

    #[test]
    fn health_readiness_returns_ready() {
        let response = dispatch_request(&rpc_no_params(methods::HEALTH_READINESS), &default_data());
        let result = result_value(&response);
        assert_eq!(result["ready"], true);
    }

    #[test]
    fn capabilities_list_returns_seventeen_capabilities() {
        let response =
            dispatch_request(&rpc_no_params(methods::CAPABILITIES_LIST), &default_data());
        let result = result_value(&response);
        assert_eq!(result["count"], 17);
        assert_eq!(
            result["capabilities"]
                .as_array()
                .expect("capabilities array")
                .len(),
            17
        );
    }

    #[test]
    fn rges_screen_with_valid_params_returns_results() {
        let response = dispatch_request(
            &rpc(
                methods::RGES_SCREEN,
                &json!({
                    "disease": sample_disease(),
                    "perturbations": [sample_perturbation("CHEMBL1")],
                    "enrichment_config": fast_enrichment_config(),
                    "screen_config": permissive_screen_config()
                }),
            ),
            &default_data(),
        );
        let hits = result_value(&response).as_array().expect("hits array");
        assert_eq!(hits.len(), 1);
        assert!(
            hits[0]["reversal_strength"]
                .as_f64()
                .is_some_and(|v| v > 0.0)
        );
    }

    #[test]
    fn rcl_select_with_valid_params_returns_rankings() {
        let response = dispatch_request(
            &rpc(
                methods::RCL_SELECT,
                &json!({
                    "disease": sample_disease(),
                    "perturbations": [
                        varied_perturbation("CHEMBL1", "1"),
                        varied_perturbation("CHEMBL2", "2"),
                    ],
                    "config": {
                        "enrichment_config": fast_enrichment_config(),
                        "min_compounds_per_line": 2
                    }
                }),
            ),
            &default_data(),
        );
        let rankings = result_value(&response).as_array().expect("rankings array");
        assert_eq!(rankings.len(), 1);
        assert_eq!(rankings[0]["cell_line"], "A549");
        assert_eq!(rankings[0]["n_compounds"], 2);
        assert!(
            rankings[0]["mean_abs_rges"]
                .as_f64()
                .is_some_and(|v| v > 0.0)
        );
    }

    #[test]
    fn mcts_optimize_with_valid_params_returns_result() {
        let response = dispatch_request(
            &rpc(
                methods::MCTS_OPTIMIZE,
                &json!({
                    "initial": sample_features(),
                    "config": fast_mcts_config()
                }),
            ),
            &default_data(),
        );
        let result = result_value(&response);
        assert_eq!(result["iterations_run"], 10);
        assert!(result["best_reward"].as_f64().is_some());
    }

    #[test]
    fn nf_score_with_valid_params_returns_scores() {
        let response = dispatch_request(
            &rpc(
                methods::NF_SCORE,
                &json!({
                    "disease": sample_disease(),
                    "perturbations": [sample_perturbation("CHEMBL_NF")],
                }),
            ),
            &default_data(),
        );
        let scores = result_value(&response).as_array().expect("scores array");
        assert_eq!(scores.len(), 1);
        assert!(
            scores[0]["reversal_strength"]
                .as_f64()
                .is_some_and(|v| v > 0.0)
        );
    }

    #[test]
    fn gps4drug_predict_without_weights_returns_cas_not_loaded_error() {
        let response = dispatch_request(
            &rpc(
                methods::GPS4DRUG_PREDICT,
                &json!({ "features": sample_features() }),
            ),
            &default_data(),
        );
        assert_eq!(error_code(&response), -32_603);
        assert!(
            response
                .error
                .as_ref()
                .expect("error")
                .message
                .contains("CAS initialization")
        );
    }

    #[test]
    fn compound_screen_without_library_returns_cas_not_loaded_error() {
        let response = dispatch_request(
            &rpc(
                methods::COMPOUND_SCREEN,
                &json!({
                    "hits": [{
                        "compound_id": "CHEMBL1",
                        "rges_score": 0.5,
                        "p_value": 0.01,
                        "adjusted_p_value": 0.02,
                        "reversal_strength": 0.5,
                        "n_permutations": 100
                    }]
                }),
            ),
            &default_data(),
        );
        assert_eq!(error_code(&response), -32_603);
        assert!(
            response
                .error
                .as_ref()
                .expect("error")
                .message
                .contains("CAS initialization")
        );
    }

    #[test]
    fn octad_benchmark_without_known_actives_returns_cas_not_loaded_error() {
        let response = dispatch_request(
            &rpc(
                methods::OCTAD_BENCHMARK,
                &json!({
                    "ranked": [{ "compound_id": "A", "score": 0.9 }]
                }),
            ),
            &default_data(),
        );
        assert_eq!(error_code(&response), -32_603);
        assert!(
            response
                .error
                .as_ref()
                .expect("error")
                .message
                .contains("CAS initialization")
        );
    }

    // --- Visualization dispatch tests ---

    #[test]
    fn viz_rges_volcano_returns_scene_json() {
        let response = dispatch_request(
            &rpc(
                methods::VIZ_RGES_VOLCANO,
                &json!({
                    "disease": sample_disease(),
                    "perturbations": [sample_perturbation("CHEMBL_VIZ")],
                    "enrichment_config": fast_enrichment_config(),
                    "screen_config": permissive_screen_config(),
                    "top_n": 5
                }),
            ),
            &default_data(),
        );
        let result = result_value(&response);
        assert_eq!(result["scene"], "rges_volcano");
        assert_eq!(result["format"], "webgl");
        assert_eq!(result["interactive"], true);
        assert!(result["data"]["points"].as_array().is_some());
    }

    #[test]
    fn viz_rges_volcano_missing_params_returns_error() {
        let response = dispatch_request(&rpc_no_params(methods::VIZ_RGES_VOLCANO), &default_data());
        assert_eq!(error_code(&response), -32_602);
    }

    #[test]
    fn viz_enrichment_curve_returns_scene_json() {
        let response = dispatch_request(
            &rpc(
                methods::VIZ_ENRICHMENT_CURVE,
                &json!({
                    "disease": sample_disease(),
                    "perturbations": [sample_perturbation("CHEMBL_EC")],
                    "enrichment_config": fast_enrichment_config(),
                    "screen_config": permissive_screen_config(),
                    "known_active_ids": ["CHEMBL_EC"]
                }),
            ),
            &default_data(),
        );
        let result = result_value(&response);
        assert_eq!(result["scene"], "enrichment_curve");
        assert!(result["data"]["curve"].as_array().is_some());
    }

    #[test]
    fn viz_nf_dashboard_returns_scene_json() {
        let response = dispatch_request(
            &rpc(
                methods::VIZ_NF_DASHBOARD,
                &json!({
                    "disease": sample_disease(),
                    "perturbations": [sample_perturbation("NF_VIZ")],
                }),
            ),
            &default_data(),
        );
        let result = result_value(&response);
        assert_eq!(result["scene"], "nf_candidate_dashboard");
        assert!(result["data"]["rows"].as_array().is_some());
    }

    #[test]
    fn viz_gps4drug_scatter_without_weights_returns_error() {
        let response = dispatch_request(
            &rpc(
                methods::VIZ_GPS4DRUG_SCATTER,
                &json!({ "features": sample_features() }),
            ),
            &default_data(),
        );
        assert_eq!(error_code(&response), -32_603);
    }

    #[test]
    fn viz_mcts_trace_returns_scene_json() {
        let response = dispatch_request(
            &rpc(
                methods::VIZ_MCTS_TRACE,
                &json!({
                    "initial": sample_features(),
                    "config": fast_mcts_config()
                }),
            ),
            &default_data(),
        );
        let result = result_value(&response);
        assert_eq!(result["scene"], "mcts_optimization_trace");
        assert!(result["data"]["best_reward"].as_f64().is_some());
        assert_eq!(result["data"]["iterations_run"], 10);
    }

    #[test]
    fn data_catalog_returns_scene_json() {
        let response = dispatch_request(&rpc_no_params(methods::DATA_CATALOG), &default_data());
        let result = result_value(&response);
        assert_eq!(result["scene"], "data_catalog");
        assert_eq!(result["data"]["total_datasets"], 3);
        let rows = result["data"]["rows"].as_array().expect("rows array");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0]["key"], "compound_library");
        assert_eq!(rows[0]["status"], "awaiting CAS");
    }

    #[test]
    fn data_catalog_no_params_required() {
        let response = dispatch_request(&rpc_no_params(methods::DATA_CATALOG), &default_data());
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }
}
