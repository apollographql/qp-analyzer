# @apollo/qp-analyzer

Apollo Federation Query Plan Analyzer (compiled to WebAssembly for Node.js environments).
This tool can be used to predict how GraphQL queries may be planned by Apollo Router.

## Installation

```bash
npm install @apollo/qp-analyzer
```

Or

```bash
npx  @apollo/qp-analyzer
```

## CLI Usage

```
Usage: qp-analyzer <command> [options]

Commands:
  override-labels      List override labels in a schema
  plan-all             Build all possible query plans
  plan-one             Build a single optimized query plan
  compare-plans        Compare two query plan JSON files (produced using the plan-one command)
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
```

## Library Usage

### List all override labels from supergraph schema

```javascript
import analyzer from '@apollo/qp-analyzer';

const labels = analyzer.override_labels(supergraph);
for (const label of labels) {
    console.log(label);
}
```

* supergraph (String): Supergraph schema document
* Return value (String[]): array of labels

### Compute query plans for all possible override configurations

```javascript
import analyzer from '@apollo/qp-analyzer';

const plans = analyzer.build_all_plans(supergraph, query, query_path, query_planner_args);
for (const plan of plans) {
    console.log(plan.query_plan_display);
}
```

* `supergraph` (String): Supergraph schema document
* `query` (String): Operation document
* `query_path` (String): The nominal path to the Operation document
* `query_planner_args` (Object) has the following fields:
  - `disable_generate_query_fragments` (bool): Disable optimization of subgraph fetch queries using fragments.
  - `disable_defer_support` (bool): Disable defer support.
* Return value (Object[]) is an array of objects with the following fields:
  - `query_plan_config` (Object): Query plan configuration
  - `query_plan_display` (String): Query plan display text
  - `experimental_query_plan_serialized` (Object): Serialized query plan


### Compute one query plan for given override configuration

```javascript
import analyzer from '@apollo/qp-analyzer';

const plan = analyzer.build_one_plan(supergraph, query, query_path, query_planner_args, override_all, override_conditions);
console.log(plan.query_plan_display);
```

* supergraph (String): Supergraph schema document
* query (String): Operation document
* query_path (String): The nominal path to the Operation document
* `query_planner_args` (Object) has the following fields:
  - `disable_generate_query_fragments` (bool): Disable optimization of subgraph fetch queries using fragments.
  - `disable_defer_support` (bool): Disable defer support.
* override_all (bool): enable all override labels, if true
* override_conditions (String[]; optional): enabled override labels
* Return value (Object) has the following fields:
  - `query_plan_config` (Object): Query plan configuration
  - `query_plan_display` (String): Query plan display text
  - `experimental_query_plan_serialized` (Object): Serialized query plan

### Compare two query plans

The `compare_plans` function compares two query plans and returns `undefined` if they are identical; Otherwise, returns an object with the following fields:

* `full_diff`: The textual difference between the YAML versions of input query plans.
* `diff_description`: The description of the first difference found.

```javascript
import analyzer from '@apollo/qp-analyzer';

const plan1 = analyzer.build_one_plan( ... );
const plan2 = analyzer.build_one_plan( ... );
const diff = analyzer.compare_plans(supergraph, plan1, plan2);
if (diff != null) {
  console.log(diff.full_diff);
  console.log(diff.diff_description);
}
```
