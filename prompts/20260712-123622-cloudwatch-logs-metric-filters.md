---
type: test
description: Integration tests for CloudWatch Logs metric filters and Insights queries
---

# CloudWatch Logs Metric Filters and Insights Integration Tests

## Summary

Created comprehensive integration tests for CloudWatch Logs metric filters and Logs Insights queries, validating fixes for several bugs found while reviewing a real Bedrock invocation-logging terraform module.

## Test Coverage

### Metric Filter Tests (`TestMetricFilterWithExtraction`)

1. **`test_metric_filter_wildcard_pattern_and_extraction`**
   - Validates `{ $.field = * }` wildcard pattern matches when field is present
   - Verifies `metricValue: "$.field.path"` extracts actual value (not hardcoded 1.0)
   - Confirms `dimensions: {Key: "$.field.path"}` attaches dimensions to emitted metrics
   - End-to-end: creates log group, metric filter, puts events, polls CloudWatch metrics

2. **`test_metric_filter_wildcard_does_not_match_missing_field`**
   - Ensures wildcard pattern does NOT match when field is absent
   - Verifies no metric is emitted for non-matching events

3. **`test_metric_filter_multiple_events_with_dimensions`**
   - Tests multiple events with different dimension values
   - Validates separate metric series are created per unique dimension combination
   - Verifies correct aggregation (sum) per dimension value

### Logs Insights Tests (`TestLogsInsightsQueries`)

1. **`test_insights_nested_json_auto_discovery`**
   - Validates nested JSON fields are auto-discovered as dotted paths (e.g., `input.inputTokenCount`)
   - Tests `stats sum(x.y) as alias` with grouping

2. **`test_insights_parse_with_dotted_source_and_glob_wildcards`**
   - Tests `parse identity.arn "*assumed-role/AWSReservedSSO_*_*/*"` with dotted source field
   - Validates glob wildcards extract ARN components correctly
   - Verifies stats aggregation by extracted field

3. **`test_insights_filter_single_quoted_string`**
   - Validates `filter field = 'value'` (single quotes) works correctly

4. **`test_insights_full_dashboard_query`**
   - Full dashboard-style query combining parse, stats, and alias
   - Tests grouping by extracted permission_set field
   - Validates correct sums per group (not empty, not one ungrouped bucket)

## Files Created

- `tests/integration/test_cloudwatch_logs_metric_filters.py` (7 tests)

## Verification

```bash
# Run new tests
uv run --no-sync python -m pytest tests/integration/test_cloudwatch_logs_metric_filters.py -v

# Run full integration suite
uv run --no-sync python -m pytest tests/integration/ -v

# Linting
uv run --no-sync ruff check tests/integration/test_cloudwatch_logs_metric_filters.py
uv run --no-sync ruff format tests/integration/test_cloudwatch_logs_metric_filters.py
uv run --no-sync mypy tests/integration/test_cloudwatch_logs_metric_filters.py
```

## Results

- All 7 new tests pass
- All 80 integration tests pass (4 skipped)
- ruff check: clean
- ruff format: applied
- mypy: no issues

## Bug Class Validated

Per the task description, these tests lock in fixes for:

1. Metric filter JSON pattern `{ $.field = * }` wildcard matching
2. Metric transformation `metricValue` as `$.field.path` extractor
3. Metric transformation `dimensions` attachment
4. Logs Insights nested JSON field auto-discovery
5. Insights `parse` with dotted source and glob wildcards
6. Insights `filter field = 'value'` (single-quoted)
7. `stats func(x) as alias` alias preservation
