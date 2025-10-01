use anyhow::Error as AnyError;
use apollo_compiler::ExecutableDocument;
use apollo_compiler::collections::IndexSet;
use apollo_federation::error::FederationError;
use apollo_federation::query_plan::QueryPlan;
use apollo_federation::query_plan::query_planner::QueryPlanIncrementalDeliveryConfig;
use apollo_federation::query_plan::query_planner::QueryPlanOptions;
use apollo_federation::query_plan::query_planner::QueryPlanner;
use apollo_federation::query_plan::query_planner::QueryPlannerConfig;
use apollo_federation::query_plan::query_planner::QueryPlannerDebugConfig;
use clap::Parser;
use std::fs;
use std::io;
use std::num::NonZeroU32;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::prelude::*;

#[derive(clap::Parser)]
enum Command {
    /// List all override condition labels in supergraph schema
    ListOverrides {
        /// Path to the supergraph schema file.
        schema: PathBuf,
    },
    /// Plan all possible query plans for supergraph schema and query
    Plan {
        /// Path to the supergraph schema file.
        schema: PathBuf,
        /// Path to the query file, `-` for stdin.
        query: PathBuf,
        /// Output results in JSON format.
        #[arg(long)]
        json: bool,
        /// Query planner arguments
        #[command(flatten)]
        planner_args: QueryPlannerArgs,
    },
}

/// Query-planner-related arguments
/// * Reflecting the Router configuration options.
#[derive(Parser)]
struct QueryPlannerArgs {
    /// Disable optimization of subgraph fetch queries using fragments.
    #[arg(long)]
    pub(crate) disable_generate_query_fragments: bool,

    /// Disable defer support.
    #[arg(long)]
    pub(crate) disable_defer_support: bool,

    /// Enable type conditioned fetching.
    #[arg(long, default_value_t = false)]
    pub(crate) experimental_type_conditioned_fetching: bool,

    /// Sets a limit to the number of generated query plans.
    #[arg(long, default_value_t = 10_000)]
    pub(crate) experimental_plans_limit: u32,

    /// Specify a per-path limit to the number of options considered.
    /// No limit is applied by default. Also, if set to `0`, it is treated as no limit.
    #[arg(long, default_value_t = 0)]
    pub(crate) experimental_paths_limit: u32,
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

fn main() {
    init_tracing();
    let cmd = Command::parse();
    let result = match cmd {
        Command::ListOverrides { schema } => cmd_overrides(&schema),
        Command::Plan {
            schema,
            query,
            planner_args,
            json,
        } => cmd_build_all_plans(&schema, &query, planner_args, json),
    };
    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

/// Set up the tracing subscriber
fn init_tracing() {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .without_time()
        .with_target(false);
    let filter_layer = tracing_subscriber::EnvFilter::from_default_env();
    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(filter_layer)
        .init();
}

fn cmd_overrides(schema_path: &Path) -> Result<(), AnyError> {
    let supergraph = load_supergraph_file(schema_path)?;
    let planner = QueryPlanner::new(&supergraph, QueryPlannerConfig::default())?;
    let override_labels = planner.override_condition_labels();
    for label in override_labels {
        println!("{label}");
    }
    Ok(())
}

/// Enumerate all possible combinations of override conditions and build query plans for them.
fn cmd_build_all_plans(
    schema_path: &Path,
    query_path: &Path,
    planner_args: QueryPlannerArgs,
    json_output: bool,
) -> Result<(), AnyError> {
    let supergraph = load_supergraph_file(schema_path)?;
    let config = QueryPlannerConfig::from(planner_args);
    let planner = QueryPlanner::new(&supergraph, config)?;

    let override_labels = planner.override_condition_labels();
    tracing::info!("Override condition labels: {override_labels:?}");

    // enumerate all combinations of override labels.
    let override_combinations = generate_all_possible_override_conditions(override_labels);
    tracing::info!("Override condition combinations: {override_combinations:#?}");

    let query = read_input(query_path);
    let query_doc =
        ExecutableDocument::parse_and_validate(planner.api_schema().schema(), query, query_path)
            .map_err(FederationError::from)?;
    let mut json = Vec::new();
    for (i, override_conditions) in override_combinations.into_iter().enumerate() {
        if !json_output {
            println!("-----------------------------------------------------------------------");
            println!("Override Combination #{i}: {override_conditions:?}");
            println!("-----------------------------------------------------------------------");
        }
        let qp_opts = QueryPlanOptions {
            override_conditions: override_conditions.clone(),
            ..Default::default()
        };
        let query_plan = planner.build_query_plan(&query_doc, None, qp_opts)?;
        if !json_output {
            println!("{query_plan}\n");
        }
        json.push(QueryPlanResult {
            query_plan_config: QueryPlanConfig {
                override_conditions,
            },
            query_plan_display: format!("{query_plan}"),
            experimental_query_plan_serialized: query_plan,
        });
    }
    if json_output {
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    }
    Ok(())
}

#[derive(serde::Serialize)]
struct QueryPlanResult {
    /// The configuration affecting the generation of this query plan
    query_plan_config: QueryPlanConfig,

    /// The human-readable text representation of the generated query plan
    query_plan_display: String,

    /// (experimental) Apollo's internal representation of the generated query plan
    experimental_query_plan_serialized: QueryPlan,
}

#[derive(serde::Serialize)]
struct QueryPlanConfig {
    override_conditions: Vec<String>,
}

fn read_input(input_path: &Path) -> String {
    if input_path == std::path::Path::new("-") {
        io::read_to_string(io::stdin()).unwrap()
    } else {
        fs::read_to_string(input_path).unwrap()
    }
}

fn load_supergraph_file(
    file_path: &Path,
) -> Result<apollo_federation::Supergraph, FederationError> {
    let doc_str = read_input(file_path);
    apollo_federation::Supergraph::new_with_router_specs(&doc_str)
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
