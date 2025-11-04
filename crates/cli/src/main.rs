use anyhow::Error as AnyError;
use apollo_federation::query_plan::query_planner::QueryPlanIncrementalDeliveryConfig;
use apollo_federation::query_plan::query_planner::QueryPlannerConfig;
use apollo_federation::query_plan::query_planner::QueryPlannerDebugConfig;
use clap::Parser;
use std::fs;
use std::io;
use std::num::NonZeroU32;
use std::path::Path;
use std::path::PathBuf;
use tracing_subscriber::prelude::*;

use qp_analyzer::build_all_plans;
use qp_analyzer::build_one_plan;
use qp_analyzer::get_override_labels;

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
    /// Plan a query plan for supergraph schema, query and override conditions
    PlanOne {
        /// Path to the supergraph schema file.
        schema: PathBuf,
        /// Path to the query file, `-` for stdin.
        query: PathBuf,
        /// Override conditions labels
        override_conditions: Vec<String>,
        /// Override all conditions (equivalent to specifying all labels)
        #[arg(long)]
        override_all: bool,
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
        Command::PlanOne {
            schema,
            query,
            planner_args,
            override_conditions,
            override_all,
            json,
        } => cmd_build_one_plan(
            &schema,
            &query,
            planner_args,
            override_conditions,
            override_all,
            json,
        ),
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
    let override_labels = get_override_labels(&read_input(schema_path))?;
    for label in override_labels {
        println!("{label}");
    }
    Ok(())
}

fn cmd_build_all_plans(
    schema_path: &Path,
    query_path: &Path,
    planner_args: QueryPlannerArgs,
    json_output: bool,
) -> Result<(), AnyError> {
    let results = build_all_plans(
        &read_input(schema_path),
        &read_input(query_path),
        query_path,
        planner_args.into(),
        !json_output,
    )?;
    if json_output {
        println!("{}", serde_json::to_string_pretty(&results).unwrap());
    }
    Ok(())
}

fn cmd_build_one_plan(
    schema_path: &Path,
    query_path: &Path,
    planner_args: QueryPlannerArgs,
    override_conditions: Vec<String>,
    override_all: bool,
    json_output: bool,
) -> Result<(), AnyError> {
    let override_conditions = if override_all && override_conditions.is_empty() {
        None
    } else {
        Some(override_conditions)
    };
    let result = build_one_plan(
        &read_input(schema_path),
        &read_input(query_path),
        query_path,
        planner_args.into(),
        override_all,
        override_conditions,
    )?;
    if json_output {
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
    } else {
        println!("{}", result.query_plan_display);
    }
    Ok(())
}

fn read_input(input_path: &Path) -> String {
    if input_path == std::path::Path::new("-") {
        io::read_to_string(io::stdin()).unwrap()
    } else {
        fs::read_to_string(input_path).unwrap()
    }
}
