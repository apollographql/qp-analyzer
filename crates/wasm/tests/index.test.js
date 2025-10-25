import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import analyzer from "../pkg/qp-analyzer.js";
import { fileURLToPath } from "node:url";

const supergraph = readFileSync(
    fileURLToPath(new URL("../../../example/supergraph.graphql", import.meta.url)),
    "utf8"
);

const query_path = "example/op.graphql";
const query = readFileSync(
    fileURLToPath(new URL("../../../" + query_path, import.meta.url)),
    "utf8"
);

test("override_labels works", async () => {
    const labels = analyzer.override_labels(supergraph);
    assert.deepEqual(labels, [
        "percent(50)",
        "percent(90)",
    ]);
});

test("build_all_plans works", async () => {
    const plans = analyzer.build_all_plans(supergraph, query, query_path, {}, false);
    assert.deepEqual(plans.length, 4);
    assert.deepEqual(plans[0].query_plan_display,
`QueryPlan {
  Sequence {
    Fetch(service: "entrypoint") {
      {
        test {
          __typename
          id
        }
      }
    },
    Flatten(path: "test") {
      Fetch(service: "monolith") {
        {
          ... on T {
            __typename
            id
          }
        } =>
        {
          ... on T {
            data1
            data2
          }
        }
      },
    },
  },
}`
    );
});
