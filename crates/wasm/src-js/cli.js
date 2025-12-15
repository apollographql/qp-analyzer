#!/usr/bin/env node

const { build_all_plans, build_one_plan, override_labels, compare_plans } = require('./qp-analyzer.js');
const fs = require('fs');
const path = require('path');

const args = process.argv.slice(2);

function printHelp() {
  console.log(`
Apollo Query Plan Analyzer CLI

Usage: qp-analyzer <command> [options]

Commands:
  override-labels      List override labels in a schema
  plan-all             Build all possible query plans
  plan-one             Build a single optimized query plan
  compare-plans        Compare two query plans
  help                 Show this help message

Common planner options:
  --disable-generate-query-fragments
  --disable-defer-support
  --experimental-type-conditioned-fetching
  --experimental-plans-limit <number>
  --experimental-paths-limit <number>

Output options:
  --json               Print JSON (for plan-all / plan-one)

Plan-one options:
  --override-all       Treat all override labels as applied
  <override-labels...> Additional override labels (positional)

Examples:
  qp-analyzer override-labels schema.graphql
  qp-analyzer plan-all schema.graphql query.graphql --json
  qp-analyzer plan-one schema.graphql query.graphql --override-all
  qp-analyzer plan-one schema.graphql query.graphql labelA labelB
  qp-analyzer compare-plans schema.graphql plan1.json plan2.json
  `);
}

function parsePlannerFlags(argv, startIndex = 0) {
  const planner = {
    disable_generate_query_fragments: false,
    disable_defer_support: false,
    experimental_type_conditioned_fetching: false,
    experimental_plans_limit: 10_000,
    experimental_paths_limit: 0,
  };

  let json = false;
  let overrideAll = false;
  const positional = [];

  for (let i = startIndex; i < argv.length; i += 1) {
    const token = argv[i];
    if (token === '--json') {
      json = true;
      continue;
    }
    if (token === '--override-all') {
      overrideAll = true;
      continue;
    }
    if (token === '--disable-generate-query-fragments') {
      planner.disable_generate_query_fragments = true;
      continue;
    }
    if (token === '--disable-defer-support') {
      planner.disable_defer_support = true;
      continue;
    }
    if (token === '--experimental-type-conditioned-fetching') {
      planner.experimental_type_conditioned_fetching = true;
      continue;
    }
    if (token === '--experimental-plans-limit') {
      const next = argv[++i];
      planner.experimental_plans_limit = next ? Number(next) : planner.experimental_plans_limit;
      continue;
    }
    if (token === '--experimental-paths-limit') {
      const next = argv[++i];
      planner.experimental_paths_limit = next ? Number(next) : planner.experimental_paths_limit;
      continue;
    }

    // positional
    positional.push(token);
  }

  return { planner, json, overrideAll, positional };
}

function ensureFileExists(label, filePath) {
  if (!filePath) {
    console.error(`Error: ${label} file required`);
    process.exit(1);
  }
  if (!fs.existsSync(filePath)) {
    console.error(`Error: ${label} file not found: ${filePath}`);
    process.exit(1);
  }
}

function handleOverrideLabels(argv) {
  const { positional } = parsePlannerFlags(argv);
  const schemaFile = positional[0];

  if (!schemaFile) {
    console.error('Usage: qp-analyzer override-labels <schema-file>');
    process.exit(1);
  }

  ensureFileExists('Schema', schemaFile);
  const schema = fs.readFileSync(schemaFile, 'utf-8');

  try {
    const labels = override_labels(schema);
    console.log(JSON.stringify(labels, null, 2));
  } catch (error) {
    console.error('Error:', error);
    process.exit(1);
  }
}

function handleBuildAllPlans(argv) {
  const { planner, json, positional } = parsePlannerFlags(argv);
  const schemaFile = positional[0];
  const queryFile = positional[1];

  if (!schemaFile || !queryFile) {
    console.error('Usage: qp-analyzer plan-all <schema-file> <query-file> [planner-options] [--json]');
    process.exit(1);
  }

  ensureFileExists('Schema', schemaFile);
  ensureFileExists('Query', queryFile);

  const schema = fs.readFileSync(schemaFile, 'utf-8');
  const query = fs.readFileSync(queryFile, 'utf-8');

  try {
    const plans = build_all_plans(schema, query, queryFile, planner);
    if (json) {
      console.log(JSON.stringify(plans, null, 2));
    } else {
      plans.forEach((plan, i) => {
        console.log('-----------------------------------------------------------------------');
        console.log(`Override Combination #${i}: ${JSON.stringify(plan.query_plan_config.override_conditions)}`);
        console.log('-----------------------------------------------------------------------');
        console.log(plan.query_plan_display);
        console.log();
      });
    }
  } catch (error) {
    console.error('Error:', error);
    process.exit(1);
  }
}

function handleBuildOnePlan(argv) {
  const { planner, json, overrideAll, positional } = parsePlannerFlags(argv);
  const schemaFile = positional[0];
  const queryFile = positional[1];
  const overrideConditions = positional.slice(2);

  if (!schemaFile || !queryFile) {
    console.error('Usage: qp-analyzer plan-one <schema-file> <query-file> [override-labels...] [planner-options] [--override-all] [--json]');
    process.exit(1);
  }

  ensureFileExists('Schema', schemaFile);
  ensureFileExists('Query', queryFile);

  const schema = fs.readFileSync(schemaFile, 'utf-8');
  const query = fs.readFileSync(queryFile, 'utf-8');

  const overrides = overrideAll ? null : overrideConditions;

  try {
    const plan = build_one_plan(schema, query, queryFile, planner, overrideAll, overrides);
    if (json) {
      console.log(JSON.stringify(plan, null, 2));
    } else {
      console.log(plan.query_plan_display);
    }
  } catch (error) {
    console.error('Error:', error);
    process.exit(1);
  }
}

function handleComparePlans(argv) {
  const { json, positional } = parsePlannerFlags(argv);
  const schemaFile = positional[0];
  const plan1File = positional[1];
  const plan2File = positional[2];

  if (!schemaFile || !plan1File || !plan2File) {
    console.error('Usage: qp-analyzer compare-plans <schema-file> <plan1-file> <plan2-file> [--json]');
    process.exit(1);
  }

  ensureFileExists('Schema', schemaFile);
  ensureFileExists('Plan1', plan1File);
  ensureFileExists('Plan2', plan2File);

  const schema = fs.readFileSync(schemaFile, 'utf-8');
  const plan1 = JSON.parse(fs.readFileSync(plan1File, 'utf-8'));
  const plan2 = JSON.parse(fs.readFileSync(plan2File, 'utf-8'));

  try {
    const comparison = compare_plans(schema, plan1, plan2);
    if (json) {
      console.log(JSON.stringify(comparison, null, 2));
    } else {
      if (comparison == null) {
        console.log('The two query plans are identical.');
      } else {
        console.log('The two query plans are different:');
        console.log();
        console.log('--- Full Diff ---');
        console.log(comparison.full_diff);
        console.log('--- Diff Description ---');
        console.log(comparison.diff_description);
      }
    }
  } catch (error) {
    console.error('Error:', error);
    process.exit(1);
  }
}

// Main command routing
if (args.length === 0 || args[0] === 'help' || args[0] === '-h' || args[0] === '--help') {
  printHelp();
  process.exit(args.length === 0 ? 1 : 0);
}

const command = args[0];
const rest = args.slice(1);

switch (command) {
  case 'override-labels':
    handleOverrideLabels(rest);
    break;

  case 'plan-all':
    handleBuildAllPlans(rest);
    break;

  case 'plan-one':
    handleBuildOnePlan(rest);
    break;

  case 'compare-plans':
    handleComparePlans(rest);
    break;

  default:
    console.error(`Error: Unknown command '${command}'`);
    console.error('Run with --help to see available commands');
    process.exit(1);
}
