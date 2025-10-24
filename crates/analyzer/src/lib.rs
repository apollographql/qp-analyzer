use std::path::Path;
use std::sync::Arc;

use apollo_compiler::ExecutableDocument;
use apollo_compiler::collections::IndexSet;
use apollo_federation::error::FederationError;
use apollo_federation::internal_error;
use apollo_federation::query_plan::QueryPlan;
use apollo_federation::query_plan::query_planner::QueryPlanOptions;
use apollo_federation::query_plan::query_planner::QueryPlanner;
use apollo_federation::query_plan::query_planner::QueryPlannerConfig;

#[derive(serde::Serialize)]
pub struct QueryPlanResult {
    /// The configuration affecting the generation of this query plan
    pub query_plan_config: QueryPlanConfig,

    /// The human-readable text representation of the generated query plan
    pub query_plan_display: String,

    /// (experimental) Apollo's internal representation of the generated query plan
    pub experimental_query_plan_serialized: QueryPlan,
}

#[derive(serde::Serialize)]
pub struct QueryPlanConfig {
    pub override_conditions: Vec<String>,
}

pub fn get_override_labels(schema_str: &str) -> Result<IndexSet<Arc<str>>, FederationError> {
    let supergraph = apollo_federation::Supergraph::new_with_router_specs(schema_str)?;
    let planner = QueryPlanner::new(&supergraph, QueryPlannerConfig::default())?;
    let override_labels = planner.override_condition_labels();
    Ok(override_labels.clone())
}

/// Enumerate all possible combinations of override conditions and build query plans for them.
pub fn build_all_plans(
    schema_str: &str,
    query_str: &str,
    query_path: impl AsRef<Path>,
    config: QueryPlannerConfig,
    verbose: bool,
) -> Result<Vec<QueryPlanResult>, FederationError> {
    let supergraph = apollo_federation::Supergraph::new_with_router_specs(schema_str)?;
    let planner = QueryPlanner::new(&supergraph, config)?;

    let query_doc = ExecutableDocument::parse_and_validate(
        planner.api_schema().schema(),
        query_str,
        query_path,
    )
    .map_err(FederationError::from)?;

    let override_labels = planner.override_condition_labels();
    tracing::info!("Override condition labels: {override_labels:?}");

    // enumerate all combinations of override labels.
    let override_combinations = generate_all_possible_override_conditions(override_labels);
    tracing::info!("Override condition combinations: {override_combinations:#?}");

    let mut results = Vec::new();
    for (i, override_conditions) in override_combinations.into_iter().enumerate() {
        if verbose {
            println!("-----------------------------------------------------------------------");
            println!("Override Combination #{i}: {override_conditions:?}");
            println!("-----------------------------------------------------------------------");
        }
        let qp_opts = QueryPlanOptions {
            override_conditions: override_conditions.clone(),
            ..Default::default()
        };
        let query_plan = planner.build_query_plan(&query_doc, None, qp_opts)?;
        if verbose {
            println!("{query_plan}\n");
        }
        results.push(QueryPlanResult {
            query_plan_config: QueryPlanConfig {
                override_conditions,
            },
            query_plan_display: format!("{query_plan}"),
            experimental_query_plan_serialized: query_plan,
        });
    }
    Ok(results)
}

pub fn build_one_plan(
    schema_str: &str,
    query_str: &str,
    query_path: impl AsRef<Path>,
    config: QueryPlannerConfig,
    override_conditions: Vec<String>,
    override_all: bool,
) -> Result<QueryPlanResult, FederationError> {
    let supergraph = apollo_federation::Supergraph::new_with_router_specs(schema_str)?;
    let planner = QueryPlanner::new(&supergraph, config)?;

    let query_doc = ExecutableDocument::parse_and_validate(
        planner.api_schema().schema(),
        query_str,
        query_path,
    )
    .map_err(FederationError::from)?;

    let override_labels = planner.override_condition_labels();
    tracing::info!("Override condition labels: {override_labels:?}");

    check_override_conditions(override_labels, &override_conditions)?;

    let override_conditions = if override_all {
        if !override_conditions.is_empty() {
            return Err(internal_error!(
                "`override_all` cannot be used with specific override conditions",
            ));
        }
        override_labels.iter().map(|s| s.to_string()).collect()
    } else {
        override_conditions
    };

    let qp_opts = QueryPlanOptions {
        override_conditions: override_conditions.clone(),
        ..Default::default()
    };
    let query_plan = planner.build_query_plan(&query_doc, None, qp_opts)?;
    Ok(QueryPlanResult {
        query_plan_config: QueryPlanConfig {
            override_conditions,
        },
        query_plan_display: format!("{query_plan}"),
        experimental_query_plan_serialized: query_plan,
    })
}

fn generate_all_possible_override_conditions(labels: &IndexSet<Arc<str>>) -> Vec<Vec<String>> {
    let mut result = Vec::new(); // all collected combinations
    let mut state = Vec::new(); // current (partial) combination
    fn inner_generate<'a>(
        result: &mut Vec<Vec<String>>,
        state: &mut Vec<Arc<str>>,
        mut remaining: impl Iterator<Item = &'a Arc<str>> + Clone,
    ) {
        match remaining.next() {
            None => {
                // No more labels left, add the current state to the result
                result.push(state.iter().rev().map(|s| s.to_string()).collect());
            }
            Some(label) => {
                // Exclude the current label from the state
                inner_generate(result, state, remaining.clone());

                // Include the current label in the state
                state.push(label.clone());
                inner_generate(result, state, remaining);
                state.pop(); // backtrack to remove the last added label
            }
        }
    }
    inner_generate(&mut result, &mut state, labels.iter().rev());
    result
}

fn check_override_conditions(
    override_labels: &IndexSet<Arc<str>>,
    override_conditions: &[String],
) -> Result<(), FederationError> {
    // Check invalid labels
    for cond in override_conditions {
        if !override_labels.contains(cond.as_str()) {
            return Err(internal_error!(
                "Unknown override condition label: {cond}. Available labels: {override_labels:?}"
            ));
        }
    }

    // Check duplicate labels
    let mut seen = IndexSet::default();
    for cond in override_conditions {
        if !seen.insert(cond) {
            return Err(internal_error!(
                "Duplicate override condition label: {cond}"
            ));
        }
    }

    Ok(())
}
