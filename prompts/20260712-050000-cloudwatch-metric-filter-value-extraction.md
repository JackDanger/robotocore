---
session: 20260712
slug: cloudwatch-metric-filter-value-extraction
type: fix
---

## Context

Re-reviewing `launchdarkly/terraform#25240` (the Bedrock invocation-logging module) against latest robotocore, specifically checking that token-count logging is correct end-to-end — not just that `terraform apply` succeeds.

## Root cause

Three compounding bugs in `robotocore/services/cloudwatch/filters.py`, all in the metric-filter engine that turns `PutLogEvents` into CloudWatch metrics:

1. **JSON pattern `*` wildcard was matched literally.** `{ $.input.inputTokenCount = * }` (the standard "field is present" idiom, and the exact pattern this module uses) compared the field's value against the literal string `"*"` instead of treating it as a wildcard — so the filter never matched anything, and no metric was ever emitted.
2. **`metricValue` was never extracted from the message.** AWS lets `metricValue` be a `$.field.path` extractor (used here for the real token count) or a literal. `_emit_metric_from_filter` only ever did `float(literal)`; for an extractor string like `$.input.inputTokenCount` that raises `ValueError` and falls back to a hardcoded `1.0` — so even with (1) fixed, every metric point would silently record "1" instead of the real token count.
3. **Metric-transformation `dimensions` were dropped entirely.** `MetricTransformation` never stored them and `_emit_metric_from_filter` never built a `Dimensions` list, so per-`ModelId` breakdown (what the whole dashboard is built around) was never possible — `PutMetricData` calls came back dimensionless.

All three fail silently: no error, no hang, just permanently empty or wrong data. Same class of bug as the Smithy CBOR gap (`fix/cloudwatch-smithy-rpc-v2-cbor`) — a real, common AWS idiom nobody had exercised — but worse, because nothing anywhere signals it's broken.

## Fix

- `matches_filter_pattern`/`_match_json_pattern`: extracted field-path resolution into `_resolve_json_field`; `expected == "*"` with `=` now means "field present", matching real AWS semantics.
- `MetricTransformation` gained a `dimensions: dict[str, str]` field (the raw `$.field` template per dimension name).
- `_emit_metric_from_filter` now takes the raw log message, resolves `metricValue` and every dimension's template via a shared `_resolve_transform_field` helper (literal or `$.path` extractor), and skips the metric point cleanly if any referenced field is missing (falls back to `defaultValue` only for the metric value itself, matching AWS).

## Verification

- Real `terraform apply` on the actual PR module against a fresh robotocore instance: all 7 resources apply clean (unchanged from before this fix — the bug was invisible to `terraform apply`, only to the data the module produces).
- End-to-end manual repro: `PutLogEvents` with a realistic Bedrock invocation log record, then `GetMetricStatistics`/`ListMetrics` — before the fix, zero datapoints; after, `InputTokens`/`OutputTokens` show the real token counts (1500/400, not "1") with the correct `ModelId` dimension.
- 5 new unit tests (`TestMetricFilterExtraction` in `test_cloudwatch_bugs.py`): wildcard match/no-match, value extraction with dimensions, a filter with no `dimensions` key (the common case, must be unaffected), and a missing-field extractor skipping cleanly instead of emitting garbage.
- Full local suite: 8737 unit (5 new) + existing compat/integration untouched. `ruff`/`mypy`/`bandit`/`validate_test_quality`/`lint_project --fail` all clean.
