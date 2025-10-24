use apollo_federation::query_plan::query_planner::QueryPlanIncrementalDeliveryConfig;
use apollo_federation::query_plan::query_planner::QueryPlannerConfig;
use apollo_federation::query_plan::query_planner::QueryPlannerDebugConfig;
use serde::Deserialize;
use std::num::NonZeroU32;
use wasm_bindgen::prelude::*;

use qp_analyzer::get_override_labels;

/// Query planner arguments
/// - This struct mirrors `QueryPlannerArgs` in CLI crate.
#[derive(Deserialize)]
#[serde(default)]
struct QueryPlannerArgs {
    /// Disable optimization of subgraph fetch queries using fragments.
    pub(crate) disable_generate_query_fragments: bool,

    /// Disable defer support.
    pub(crate) disable_defer_support: bool,

    /// Enable type conditioned fetching.
    pub(crate) experimental_type_conditioned_fetching: bool,

    /// Sets a limit to the number of generated query plans.
    pub(crate) experimental_plans_limit: u32,

    /// Specify a per-path limit to the number of options considered.
    /// No limit is applied by default. Also, if set to `0`, it is treated as no limit.
    pub(crate) experimental_paths_limit: u32,
}

impl Default for QueryPlannerArgs {
    fn default() -> Self {
        QueryPlannerArgs {
            disable_generate_query_fragments: false,
            disable_defer_support: false,
            experimental_type_conditioned_fetching: false,
            experimental_plans_limit: 10_000,
            experimental_paths_limit: 0,
        }
    }
}

impl From<QueryPlannerArgs> for QueryPlannerConfig {
    fn from(args: QueryPlannerArgs) -> Self {
        let max_evaluated_plans = NonZeroU32::new(args.experimental_plans_limit)
            // If experimental_plans_limit is zero; use our default.
            .unwrap_or(NonZeroU32::new(10_000).unwrap());
        let paths_limit = if args.experimental_paths_limit == 0 {
            None
        } else {
            Some(args.experimental_paths_limit)
        };

        QueryPlannerConfig {
            // `subgraph_graphql_validation` is false in Router, but we may consider enabling it.
            subgraph_graphql_validation: false,
            generate_query_fragments: !args.disable_generate_query_fragments,
            incremental_delivery: QueryPlanIncrementalDeliveryConfig {
                enable_defer: !args.disable_defer_support,
            },
            type_conditioned_fetching: args.experimental_type_conditioned_fetching,
            debug: QueryPlannerDebugConfig {
                max_evaluated_plans,
                paths_limit,
            },
        }
    }
}

#[wasm_bindgen]
pub fn override_labels(schema_str: &str) -> Result<Vec<String>, String> {
    let override_labels = get_override_labels(schema_str).map_err(|e| e.to_string())?;
    Ok(override_labels.iter().map(|s| s.to_string()).collect())
}

#[wasm_bindgen]
pub fn build_all_plans(
    schema_str: &str,
    query_str: &str,
    query_path: &str,
    planner_args: JsValue,
    json_output: bool,
) -> Result<Vec<JsValue>, String> {
    let qp_args: QueryPlannerArgs =
        serde_wasm_bindgen::from_value(planner_args).map_err(|e| e.to_string())?;
    let plans = qp_analyzer::build_all_plans(
        schema_str,
        query_str,
        query_path,
        qp_args.into(),
        json_output,
    )
    .map_err(|e| e.to_string())?;

    let js_values = plans
        .into_iter()
        .map(|plan| serde_wasm_bindgen::to_value(&plan).unwrap())
        .collect();
    Ok(js_values)
}

#[wasm_bindgen]
pub fn build_one_plan(
    schema_str: &str,
    query_str: &str,
    query_path: &str,
    planner_args: JsValue,
    override_conditions: Vec<String>,
    override_all: bool,
) -> Result<JsValue, String> {
    let qp_args: QueryPlannerArgs =
        serde_wasm_bindgen::from_value(planner_args).map_err(|e| e.to_string())?;
    let plan = qp_analyzer::build_one_plan(
        schema_str,
        query_str,
        query_path,
        qp_args.into(),
        override_conditions,
        override_all,
    )
    .map_err(|e| e.to_string())?;

    let js_value = serde_wasm_bindgen::to_value(&plan).unwrap();
    Ok(js_value)
}
