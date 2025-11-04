# @apollo/qp-analyzer

Apollo Federation Query Plan Analyzer (compiled to WebAssembly for Node.js environments).
This tool can be used to predict how GraphQL queries may be planned by Apollo Router.

## Installation

```bash
npm install @apollo/qp-analyzer
```

## Usage

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

const plan = analyzer.build_all_plans(supergraph, query, query_path, query_planner_args, override_all, override_conditions);
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
