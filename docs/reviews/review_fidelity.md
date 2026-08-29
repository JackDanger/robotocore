# Fidelity Strategy Review — robotocore-rust

Date: 2026-08-29. Scope: how the Rust port measures and closes the gap with the Python/moto server.

## What I read

- `scripts/harness/parity.py` + `.parity/state.json` (the measurement loop)
- `scripts/harness/capture.py`, `golden_test.py`, `diff_test.py` (the alternative harnesses)
- `tests/compatibility/test_{s3,iam,lambda,sts}_compat.py` (what the tests actually assert)
- Failure corpus from `.parity/xml/*_rust.xml` (~1,620 failing tests, categorized)
- `src/core/{spec,protocol,server}.rs`, `crates/{lambda,iam,s3}/...` (how responses are actually built)

## Current state (from state.json, 2026-08-29)

| Service | py_pass | rust_pass | fid% |
|---|---|---|---|
| s3 | 222 | 114 | 51 |
| secretsmanager | 46 | 29 | 63 |
| sqs | 95 | 51 | 54 |
| logs | 161 | 49 | 30 |
| ssm | 228 | 57 | 25 |
| iam | 319 | 85 | 27 |
| lambda | 157 | 19 | 12 |
| ... | | | |
| stepfunctions | 71 | 1 | 1 |
| rds (bridge) | 326 | 0 | 0 |

Overall native fidelity is roughly **20–30%**, and the "next work" the harness picks is a bridge routing bug (rds at 0%), not the biggest porting gap.

Categorizing the ~1,620 rust-side failures by symptom:

| Category | Count | % |
|---|---|---|
| `X is not implemented` (ClientError 400/501) | ~1,280 | 79% |
| Missing field in response (`KeyError`) | ~360 | 22% |
| Assertion (value/shape mismatch) | ~270 | 17% |
| Malformed response (botocore parser error) | ~120 | 7% |
| `DID NOT RAISE` (missing error) | ~7 | <1% |

Categories overlap (a single test can have multiple symptoms), but the dominance of "not implemented" is unambiguous: **the single biggest gap is operation coverage, not response fidelity.** A response is only "malformed" for a handful of operations; the rest simply don't exist.

## Failure patterns by service (from the XML)

- **STS (18% fid)**: every non-`GetCallerIdentity` op returns `{"error": "Unknown STS operation: X"}` — a **JSON body with a JSON content type on a query-protocol service**. botocore's QueryParser chokes, producing 40 `ResponseParserError` failures in one service. This is a systemic wire-protocol bug, not 40 individual bugs.
- **Lambda (12% fid)**: `resolve_rest_operation` in `src/core/protocol.rs` is a hand-rolled method+path→op matcher covering maybe 20 of Lambda's routes. Anything that doesn't match returns 400 `The operation  is not implemented` (empty name because no op was resolved). Same pattern in every rest-json service.
- **IAM (27% fid)**: 129 `KeyError: 'Users'` / `'RoleName'` / `'InstanceProfile'` — the response IS returned but is missing top-level envelope fields (e.g. `ListUsers` returns `[]` instead of `{"Users": []}`), or list operations don't include the resource list at all.
- **SSM (25% fid)**: mix of "not implemented" (84) and missing fields (TagList, Command, AssociationDescription).
- **Events (26% fid)**: 78 "not implemented" — only 4 of 12 ops handled.
- **S3 (51% fid, best)**: mostly missing error types (`NoSuchCORSConfiguration`) and missing fields on list responses (`Rules`, `ETag`, `VersionId`).

## Q1: Is running the Python compat tests the right strategy?

**Yes, as the acceptance gate — but no, not as the primary driver of work.**

What it does well:
- It measures the thing that actually matters: "does botocore, the real client, parse the response and get the values the user expects?" That's the ground truth no golden file can match, because it exercises botocore's real parsers (QueryParser, RESTParser, EC2QueryParser, ProtocolParser).
- Running the same suite against both servers (4-way diff) cleanly separates "Rust fidelity loss" (py_pass ∧ rust_fail) from "environmental noise" (both_fail) from "test bug" (py_fail). This is the correct metric.
- The state file + `next_work` derivation gives a persistent, machine-readable scoreboard.

What it gets wrong as a *primary driver*:
- **It's test-name-level, not failure-cause-level.** The "next work" says "fix iam: 234 fidelity gap(s)". That's a list of test names. It doesn't say *why* they fail, and 200 of those 234 failures share one root cause (e.g. `ListUsers` missing the `Users` envelope). A one-by-one fix loop will re-discover the same root cause 200 times.
- **It measures, it doesn't tell you where the code should change.** The mapping from "test_X fails" → "crates/iam/src/handler.rs::list_users" is manual.
- **It's slow (~8 min full run) and has a 300 s per-suite timeout.** A single hang in one service (say `stepfunctions` at 1.4% fid) eats the whole budget and produces a partial result.
- **It's biased toward what the Python tests happen to cover.** `stepfunctions` has only 71 py_pass tests; the spec has 37 ops. That's fine as a smoke test, but it means you can't see the shape of the remaining gap until you're already in the service.
- **It conflates "not implemented" with "wrong response".** In the state.json, `rust_gap` lumps both together. But they need different fixes: the first is coverage, the second is fidelity. The fix strategy should not be the same.

**Better shape**: keep the compat suite as the *acceptance gate* (CI: "does the whole suite still pass on Rust?"), but move the *driver* to a **per-operation matrix** (see Q2). The compat tests remain the final "did we actually fix it?" check; the matrix is what you work off day-to-day.

## Q2: Is one-by-one fixing systematic enough? No. Here's what to do instead.

The current loop is:

1. Run compat suite
2. Pick the failing test with the biggest name
3. Fix the handler
4. Re-run

This is O(failures) work, and it re-derives the root cause for every failure that shares one. The systematic alternative is to **collapse the failure space to a matrix** and work the matrix, not the test list.

### The matrix: service × operation → (status, cause)

For every operation in the botocore spec (already loaded by `src/core/spec.rs`), classify the current Rust behavior:

| Cell value | Meaning | Example |
|---|---|---|
| `implemented, passing` | Compat test for this op passes | s3.PutObject |
| `implemented, wrong response` | Handler exists but botocore mis-parses or value is wrong | iam.ListUsers (missing `Users` key) |
| `implemented, wrong error` | Error path exists but code/type/shape is wrong | s3.GetBucketCors (missing `NoSuchCORSConfiguration`) |
| `not implemented` | No handler; returns 400/501 | lambda.PutFunctionConcurrency |
| `not reachable` | Routing can't even find the op (rest-json path mismatch) | lambda.UpdateFunctionCode |

The matrix has one cell per (service, op), so the work item is never "234 iam failures" — it's "iam.ListUsers: missing Users envelope" or "lambda.UpdateFunctionCode: not reachable". A single handler fix usually clears a *block* of matrix cells at once (e.g. fixing `ListUsers`'s response envelope fixes every test that calls it).

**How to build it mechanically, not by hand:**

1. **Enumerate**: for each native service, iterate `spec.operations.keys()`. (Already doable — spec.rs loads them.)
2. **Probe**: for each op, send a *minimal valid* request (the `scripts/lib/param_filler.py` in the Python repo already does this; port it to Rust or call it from a script). Record:
   - HTTP status code
   - Raw response body (first 200 bytes)
   - Whether botocore parsed it (try a dry parse with the spec's output shape)
   - Whether the parsed response contains all required top-level fields
3. **Classify** into the 5 states above. The "not reachable" state is detected by an empty operation name in the server's audit log, or by a 400 with "The operation  is not implemented" (note the empty op name).
4. **Persist** the matrix as JSON in `.parity/`, alongside `state.json`. Re-run the matrix (fast, ~1 min) after each fix; the compat suite (slow, ~8 min) runs nightly.

The matrix also makes the *fidelity %* metric honest: today it's `rust_pass_tests / py_pass_tests`, which depends on which tests happen to exist. The matrix gives you `implemented_ops / total_ops` per service, which is the real porting progress.

### Why this is better than the current loop

- **Root causes, not test names.** 234 iam failures → ~30 root causes (envelope missing on List*, missing TagResource path, missing error codes, etc.). You fix the 30, the 234 disappear.
- **Visible progress.** You can see "iam: 85/176 ops implemented, 12 wrong-response, 30 not-implemented" at a glance. The compat test count can go up *without* progress (if tests are added) or down *with* progress (if a test is flaky). The op count doesn't lie.
- **Fast inner loop.** A full matrix probe is ~1 min; a full compat run is ~8 min. You iterate 8x faster on the matrix, then confirm with the compat suite at the end of a session.
- **Catches the "not reachable" class** that the compat tests only surface indirectly (as "ClientError 400"). The matrix directly shows "this op's path doesn't match the resolver".
- **Decomposes the work.** You can assign "fix all wrong-response cells in s3" to one session, "implement the 15 missing events ops" to another. The current "fix the biggest gap" instruction is unassignable.

### The 3 highest-leverage changes

Given the data, I'd rank them:

**1. Replace hand-rolled operation resolution with spec-driven routing.** (Highest leverage, biggest blast radius)

The single largest class of failures (~1,280) is "not implemented", and a big chunk of that is actually "not *reachable*" — the rest-json path resolver in `src/core/protocol.rs` is a hand-rolled `match` that covers maybe 20–40 routes per service and silently returns `None` for the rest. The botocore spec (`service-2.json`) already contains the exact `(method, path)` → operation mapping for every service. `src/core/spec.rs` loads the spec but **never uses it for routing**. Wire the spec's operation map into the router:

- At startup, for each rest-json/rest-xml service, build a table from `spec.operations[op].http.{method,uri}`.
- The path is a template like `/functions/{FunctionName}/code`; compile it to a regex or a tree-walk.
- Replace `resolve_rest_operation` with a lookup in that table.

This alone should fix the bulk of the lambda "not implemented" failures (73 of 136), the s3 rest-xml gaps, and any other rest-* service. It also makes adding a new service a config change, not a code change. The `contracts/*.json` files in the repo (s3, sts, sqs, iam, lambda, sns, dynamodb, events, logs, cloudformation) look like an earlier attempt at this — they're per-operation wire contracts captured from the Python server. If they're complete, they're the routing table; if not, they should be regenerated from the spec.

**2. Add a response-shape validator that runs against the spec, not against tests.** (Second highest — fixes the ~360 missing-field failures)

The ~360 `KeyError` failures are all the same bug class: the handler returns a JSON object that's missing a field the spec's output shape requires. For example, `iam.ListUsers` spec says the response has `Users` (a list of `User`), but the handler returns `[]` at the top level, or `{"Members": [...]}` instead of `{"Users": [...]}`.

`scripts/validate_response_shapes.py` in the Python repo already does this against the live Python server. Port it (or call it) against the Rust server. For every implemented op, call it with a valid request, parse the response with the spec's output shape, and report missing/extra/mis-typed fields. This turns 360 separate test failures into a single report: "iam.ListUsers: missing Users; iam.ListRoles: missing Roles; ..." — a checklist you work through in one sitting.

The key insight: **the spec is the contract.** botocore will reject any response that doesn't match the output shape. If you validate against the shape at dev time, you never ship a response botocore can't parse. This is the "shift left" version of the compat suite: same ground truth (the spec), but 100x faster and no test infrastructure.

**3. Fix the query-protocol error format.** (Smallest change, clearest win)

STS is at 18% fidelity with 40 of 49 tests failing on `ResponseParserError: not well-formed`. The Rust server returns a **JSON** body with `Content-Type: application/json` for a query-protocol service. botocore's QueryParser expects XML. The fix is in `src/core/protocol.rs::serialize_query_response` / `serialize_query_error` — make sure query-protocol services (sts, iam, sns, sqs, etc.) always emit well-formed XML with the correct namespace and envelope. The same bug likely exists for every query-protocol service (sns at 17% fid, iam at 27%).

This is a one-file fix that should move 3–4 services from ~20% to 60%+ in an afternoon. It's the highest ROI per line of code.

## What I would NOT do

- **Don't keep the "next work = single string" model.** The state.json's `next_work` is a one-liner. After change #1 (the matrix), the "next work" becomes a *list* of work items, each with a service, op, and cause. The harness should emit a prioritized work list, not a single string.
- **Don't chase the bridge (rds/ec2/cloudformation) until the native services are at 80%.** The bridge is a proxy to moto — its fidelity is limited by moto's own fidelity, and it's not the point of the port. Fix the 18 native crates first; the bridge is a fallback, not the product.
- **Don't add more compat tests to fill gaps.** The compat suite is the acceptance gate, not the work queue. Adding tests to cover missing ops is the *last* step, after the op is implemented. The matrix already tells you which ops are missing; you don't need a test to tell you.

## Summary

The current loop (run compat suite → pick biggest gap → fix) is a reasonable start but it's O(failures) and re-derives root causes. The systematic alternative is:

1. **Spec-driven routing** (replace hand-rolled path matchers) — fixes the 79% "not implemented" class.
2. **Spec-driven response validation** (port the shape validator to Rust) — fixes the 22% "missing field" class.
3. **Query-protocol XML fix** (one file) — fixes the 7% "malformed response" class in one stroke.

Keep the compat suite as the nightly acceptance gate. Build the op matrix as the daily work queue. The three changes above, in that order, should take the native fleet from ~25% to ~70%+ fidelity, with the remainder being genuine per-op porting work that the matrix makes visible and assignable.
