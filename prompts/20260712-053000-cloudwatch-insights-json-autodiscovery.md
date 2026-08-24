---
session: 20260712
slug: cloudwatch-insights-json-autodiscovery
type: fix
---

## Context

Continuing the token-logging correctness review of `launchdarkly/terraform#25240` (follow-up to `fix/cloudwatch-metric-filter-value-extraction`, #291): the dashboard's Logs Insights widgets — the per-engineer/per-permission-set cost breakdown, the whole point of the "oversight" panel — never worked against robotocore's query engine.

## Root cause

Four compounding bugs in `robotocore/services/cloudwatch/insights.py`:

1. **JSON log fields were never auto-discovered.** Real Logs Insights auto-flattens a JSON log message's nested fields into dotted-path row fields (`input.inputTokenCount`, `identity.arn`, ...) with no `parse` needed. robotocore's rows only ever had `timestamp`/`message`/`logStream`/`ptr` — so `stats sum(input.inputTokenCount)` referenced a field that never existed.
2. **Aggregation and filter field-name regexes rejected dots.** Even with (1) fixed, `sum(input.inputTokenCount)` failed to parse as an aggregation at all (its field regex was `\w*`, no dot) and silently degraded to `count(*)`. Same story for `filter`/`sort` field names.
3. **`parse` rejected dotted source fields and glob wildcards.** `parse identity.arn "*assumed-role/AWSReservedSSO_*_*/*" as ...` — the standard AWS glob-parse idiom this module uses to split an ARN into permission-set/engineer — never matched `_parse_command`'s regex (`\w+` source, and the `*` wildcards were later passed straight to `re.search` as a literal regex, which would raise `nothing to repeat` on the leading `*`). The whole `parse` step silently vanished from the pipeline.
4. **`filter field = 'value'` (single-quoted) wasn't recognized.** Every specific matcher in `_evaluate_filter` only accepted double quotes; single-quoted filters (used by this module's "still on Administrator" oversight query) fell through to a substring-of-the-raw-message fallback that never matches.

Also fixed in passing: `stats sum(x) as alias` — the `as` alias was parsed and thrown away, so results always came back keyed by the raw `func(field)` label instead of the name the query asked for.

All four/five fail silently: valid-looking queries that always return zero, or one big bucket instead of a real breakdown.

## Fix

- `execute_pipeline` now flattens each JSON log message into dotted-path fields (`_flatten_json_message`) and merges them into the row before running the pipeline.
- Field-name character classes in the aggregation, filter, sort, and parse-source regexes now allow dots/brackets/hyphens (`[\w.\[\]-]`).
- `parse`'s regex now accepts either delimiter (`/regex/` for real regex — unchanged behavior — or `"glob"` for AWS's bare-`*`-wildcard syntax, converted to an anchored regex with one capture group per `*` via `_glob_to_regex`).
- Filter comparisons (`=`, `!=`, `like`) accept single or double quotes via a backreferenced delimiter group.
- `stats func(field) as alias` now uses `alias` as the result column name when present.

## Verification

- Manual end-to-end repro of all three dashboard log-widget queries against realistic Bedrock invocation-log JSON: before, every one returned zero rows or a single ungrouped bucket; after, correct per-permission-set/per-engineer/per-model token breakdowns.
- 4 new unit tests (`TestInsightsJsonAutoDiscovery`): JSON auto-discovery without `parse`, dotted-source glob `parse`, regex-mode `parse` still works unchanged, single-quoted `filter`.
- Full local suite: 8741 unit (4 new) passed. `ruff`/`mypy`/`bandit`/`validate_test_quality`/`lint_project --fail` all clean.
