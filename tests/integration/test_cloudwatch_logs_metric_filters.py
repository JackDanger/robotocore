"""Integration tests for CloudWatch Logs metric filters and Insights queries.

These tests validate the end-to-end behavior of:
1. Metric filters with JSON wildcard patterns (`{ $.field = * }`) and field extraction
2. Metric transformations with `$.field.path` metricValue extractors and dimensions
3. Logs Insights queries with nested JSON field auto-discovery, parse with glob wildcards,
   and stats with aliases

Modeled on real-world Bedrock invocation-logging scenarios.
"""

import json
import time
import uuid
from contextlib import contextmanager


@contextmanager
def _cleanup_log_group(logs, log_group_name):
    """Context manager to ensure log group cleanup."""
    try:
        yield log_group_name
    finally:
        try:
            logs.delete_log_group(logGroupName=log_group_name)
        except Exception:
            pass


class TestMetricFilterWithExtraction:
    """Metric filters with wildcard patterns and field extraction.

    Validates that:
    - `{ $.field = * }` matches when field is present (not literal "*")
    - `metricValue: "$.field.path"` extracts the actual value (not hardcoded 1.0)
    - `dimensions: {Key: "$.field.path"}` attaches dimensions to emitted metrics
    """

    def test_metric_filter_wildcard_pattern_and_extraction(self, make_boto_client):
        """End-to-end: metric filter with * wildcard, extracted value, and dimension."""
        suffix = uuid.uuid4().hex[:8]
        logs = make_boto_client("logs")
        cloudwatch = make_boto_client("cloudwatch")

        log_group_name = f"/aws/bedrock/test-{suffix}"
        filter_name = f"BedrockInputTokens-{suffix}"
        metric_name = f"InputTokens-{suffix}"
        metric_namespace = f"TestBedrockUsage-{suffix}"

        with _cleanup_log_group(logs, log_group_name):
            # Create log group and stream
            logs.create_log_group(logGroupName=log_group_name)
            logs.create_log_stream(logGroupName=log_group_name, logStreamName="stream1")

            # Create metric filter with:
            # - Wildcard pattern: { $.input.inputTokenCount = * }
            # - Extracted metric value: $.input.inputTokenCount
            # - Dimension: ModelId from $.modelId
            logs.put_metric_filter(
                logGroupName=log_group_name,
                filterName=filter_name,
                filterPattern="{ $.input.inputTokenCount = * }",
                metricTransformations=[
                    {
                        "metricName": metric_name,
                        "metricNamespace": metric_namespace,
                        "metricValue": "$.input.inputTokenCount",
                        "dimensions": {"ModelId": "$.modelId"},
                    }
                ],
            )

            # Put a log event with Bedrock invocation format
            log_event = {
                "identity": {
                    "arn": (
                        "arn:aws:sts::123456789012:assumed-role/"
                        "AWSReservedSSO_SomeRole_abc123/user@example.com"
                    )
                },
                "modelId": "anthropic.claude-3-sonnet",
                "input": {"inputTokenCount": 1500},
                "output": {"outputTokenCount": 400},
            }

            logs.put_log_events(
                logGroupName=log_group_name,
                logStreamName="stream1",
                logEvents=[
                    {
                        "timestamp": int(time.time() * 1000),
                        "message": json.dumps(log_event),
                    }
                ],
            )

            # Allow time for metric to be emitted
            time.sleep(1)

            # Verify metric exists with correct value and dimension
            metrics = cloudwatch.list_metrics(
                Namespace=metric_namespace,
                MetricName=metric_name,
            )["Metrics"]

            assert len(metrics) == 1, f"Expected 1 metric, got {len(metrics)}"
            metric = metrics[0]

            # Verify dimension is present and correct
            dim_dict = {d["Name"]: d["Value"] for d in metric["Dimensions"]}
            assert "ModelId" in dim_dict, "ModelId dimension should be present"
            assert dim_dict["ModelId"] == "anthropic.claude-3-sonnet"

            # Verify the metric value via GetMetricStatistics
            end_time = time.time()
            start_time = end_time - 300  # 5 minutes ago

            stats = cloudwatch.get_metric_statistics(
                Namespace=metric_namespace,
                MetricName=metric_name,
                Dimensions=metric["Dimensions"],
                StartTime=start_time,
                EndTime=end_time,
                Period=60,
                Statistics=["Sum"],
            )

            assert len(stats["Datapoints"]) >= 1, "Expected at least one datapoint"
            # The value should be 1500.0 (extracted from input.inputTokenCount), not 1.0
            datapoint = stats["Datapoints"][0]
            assert datapoint["Sum"] == 1500.0, f"Expected Sum=1500.0, got {datapoint['Sum']}"

    def test_metric_filter_wildcard_does_not_match_missing_field(self, make_boto_client):
        """Wildcard pattern { $.field = * } should NOT match when field is absent."""
        suffix = uuid.uuid4().hex[:8]
        logs = make_boto_client("logs")
        cloudwatch = make_boto_client("cloudwatch")

        log_group_name = f"/aws/test/missing-field-{suffix}"
        filter_name = f"MissingFieldFilter-{suffix}"
        metric_name = f"MissingFieldMetric-{suffix}"
        metric_namespace = f"TestMissing-{suffix}"

        with _cleanup_log_group(logs, log_group_name):
            logs.create_log_group(logGroupName=log_group_name)
            logs.create_log_stream(logGroupName=log_group_name, logStreamName="stream1")

            logs.put_metric_filter(
                logGroupName=log_group_name,
                filterName=filter_name,
                filterPattern="{ $.input.inputTokenCount = * }",
                metricTransformations=[
                    {
                        "metricName": metric_name,
                        "metricNamespace": metric_namespace,
                        "metricValue": "$.input.inputTokenCount",
                    }
                ],
            )

            # Put a log event WITHOUT the input.inputTokenCount field
            log_event = {
                "modelId": "some-model",
                "output": {"outputTokenCount": 400},  # No input field
            }

            logs.put_log_events(
                logGroupName=log_group_name,
                logStreamName="stream1",
                logEvents=[
                    {
                        "timestamp": int(time.time() * 1000),
                        "message": json.dumps(log_event),
                    }
                ],
            )

            time.sleep(1)

            # Metric should NOT have been emitted
            metrics = cloudwatch.list_metrics(
                Namespace=metric_namespace,
                MetricName=metric_name,
            )["Metrics"]

            assert len(metrics) == 0, "Expected no metrics when field is missing"

    def test_metric_filter_multiple_events_with_dimensions(self, make_boto_client):
        """Multiple events with different dimension values create separate metric series."""
        suffix = uuid.uuid4().hex[:8]
        logs = make_boto_client("logs")
        cloudwatch = make_boto_client("cloudwatch")

        log_group_name = f"/aws/test/multi-dim-{suffix}"
        filter_name = f"MultiDimFilter-{suffix}"
        metric_name = f"TokenCount-{suffix}"
        metric_namespace = f"TestMultiDim-{suffix}"

        with _cleanup_log_group(logs, log_group_name):
            logs.create_log_group(logGroupName=log_group_name)
            logs.create_log_stream(logGroupName=log_group_name, logStreamName="stream1")

            logs.put_metric_filter(
                logGroupName=log_group_name,
                filterName=filter_name,
                filterPattern="{ $.input.inputTokenCount = * }",
                metricTransformations=[
                    {
                        "metricName": metric_name,
                        "metricNamespace": metric_namespace,
                        "metricValue": "$.input.inputTokenCount",
                        "dimensions": {"ModelId": "$.modelId"},
                    }
                ],
            )

            # Put events with different model IDs
            models = [
                ("anthropic.claude-3-sonnet", 1500),
                ("anthropic.claude-3-haiku", 800),
                ("anthropic.claude-3-sonnet", 2000),  # Same model, different count
            ]

            for model_id, token_count in models:
                log_event = {
                    "modelId": model_id,
                    "input": {"inputTokenCount": token_count},
                }
                logs.put_log_events(
                    logGroupName=log_group_name,
                    logStreamName="stream1",
                    logEvents=[
                        {
                            "timestamp": int(time.time() * 1000),
                            "message": json.dumps(log_event),
                        }
                    ],
                )

            time.sleep(1)

            # Should have 2 metric series (one per unique ModelId)
            metrics = cloudwatch.list_metrics(
                Namespace=metric_namespace,
                MetricName=metric_name,
            )["Metrics"]

            assert len(metrics) == 2, f"Expected 2 metric series, got {len(metrics)}"

            # Verify each model has correct total
            end_time = time.time()
            start_time = end_time - 300

            for metric in metrics:
                dim_dict = {d["Name"]: d["Value"] for d in metric["Dimensions"]}
                model_id = dim_dict["ModelId"]

                stats = cloudwatch.get_metric_statistics(
                    Namespace=metric_namespace,
                    MetricName=metric_name,
                    Dimensions=metric["Dimensions"],
                    StartTime=start_time,
                    EndTime=end_time,
                    Period=60,
                    Statistics=["Sum"],
                )

                assert len(stats["Datapoints"]) >= 1
                total = stats["Datapoints"][0]["Sum"]

                if model_id == "anthropic.claude-3-sonnet":
                    assert total == 3500.0, f"Expected 3500 for sonnet, got {total}"
                elif model_id == "anthropic.claude-3-haiku":
                    assert total == 800.0, f"Expected 800 for haiku, got {total}"


class TestLogsInsightsQueries:
    """Logs Insights query execution with real-world patterns.

    Validates that:
    - Nested JSON fields are auto-discovered as dotted paths (input.inputTokenCount)
    - `parse` with dotted source and glob wildcards works
      (identity.arn "*assumed-role/AWSReservedSSO_*_*/*")
    - `stats sum(x.y) as alias` preserves the alias
    - `filter field = 'value'` (single quotes) works
    """

    def _wait_for_query_complete(self, logs, query_id, timeout=30, interval=0.5):
        """Poll until query completes or times out."""
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            result = logs.get_query_results(queryId=query_id)
            if result["status"] == "Complete":
                return result
            if result["status"] in ["Failed", "Cancelled"]:
                raise RuntimeError(f"Query failed with status: {result['status']}")
            time.sleep(interval)
        raise TimeoutError(f"Query did not complete within {timeout}s")

    def test_insights_nested_json_auto_discovery(self, make_boto_client):
        """Nested JSON fields are auto-discovered as dotted paths."""
        suffix = uuid.uuid4().hex[:8]
        logs = make_boto_client("logs")

        log_group_name = f"/aws/test/insights-nested-{suffix}"

        with _cleanup_log_group(logs, log_group_name):
            logs.create_log_group(logGroupName=log_group_name)
            logs.create_log_stream(logGroupName=log_group_name, logStreamName="stream1")

            # Put events with nested JSON
            events = [
                {
                    "modelId": "model-a",
                    "input": {"inputTokenCount": 100},
                    "output": {"outputTokenCount": 50},
                },
                {
                    "modelId": "model-a",
                    "input": {"inputTokenCount": 200},
                    "output": {"outputTokenCount": 100},
                },
                {
                    "modelId": "model-b",
                    "input": {"inputTokenCount": 300},
                    "output": {"outputTokenCount": 150},
                },
            ]

            logs.put_log_events(
                logGroupName=log_group_name,
                logStreamName="stream1",
                logEvents=[
                    {
                        "timestamp": int(time.time() * 1000) + i,
                        "message": json.dumps(e),
                    }
                    for i, e in enumerate(events)
                ],
            )

            # Query using nested field paths
            now = int(time.time())
            query_resp = logs.start_query(
                logGroupName=log_group_name,
                startTime=now - 3600,
                endTime=now + 60,
                queryString="stats sum(input.inputTokenCount) as input_tokens by modelId",
            )
            query_id = query_resp["queryId"]

            result = self._wait_for_query_complete(logs, query_id)

            assert result["status"] == "Complete"
            assert len(result["results"]) == 2  # Two models

            # Find results by modelId
            by_model = {}
            for row in result["results"]:
                model_id = next((f["value"] for f in row if f["field"] == "modelId"), None)
                tokens = next((f["value"] for f in row if f["field"] == "input_tokens"), None)
                if model_id:
                    by_model[model_id] = tokens

            assert "model-a" in by_model
            assert "model-b" in by_model
            # Sum for model-a should be 300 (100 + 200)
            assert float(by_model["model-a"]) == 300.0
            # Sum for model-b should be 300
            assert float(by_model["model-b"]) == 300.0

    def test_insights_parse_with_dotted_source_and_glob_wildcards(self, make_boto_client):
        """Parse with dotted source field and glob wildcards extracts ARN components."""
        suffix = uuid.uuid4().hex[:8]
        logs = make_boto_client("logs")

        log_group_name = f"/aws/test/insights-parse-{suffix}"

        with _cleanup_log_group(logs, log_group_name):
            logs.create_log_group(logGroupName=log_group_name)
            logs.create_log_stream(logGroupName=log_group_name, logStreamName="stream1")

            # Put events with SSO assumed-role ARNs
            events = [
                {
                    "identity": {
                        "arn": (
                            "arn:aws:sts::123456789012:assumed-role/"
                            "AWSReservedSSO_Administrator_abc123/alice@example.com"
                        )
                    },
                    "input": {"inputTokenCount": 100},
                },
                {
                    "identity": {
                        "arn": (
                            "arn:aws:sts::123456789012:assumed-role/"
                            "AWSReservedSSO_BedrockEngineer_def456/bob@example.com"
                        )
                    },
                    "input": {"inputTokenCount": 200},
                },
            ]

            logs.put_log_events(
                logGroupName=log_group_name,
                logStreamName="stream1",
                logEvents=[
                    {
                        "timestamp": int(time.time() * 1000) + i,
                        "message": json.dumps(e),
                    }
                    for i, e in enumerate(events)
                ],
            )

            # Query using parse with glob wildcards on dotted source field
            now = int(time.time())
            query_resp = logs.start_query(
                logGroupName=log_group_name,
                startTime=now - 3600,
                endTime=now + 60,
                queryString=(
                    'parse identity.arn "*assumed-role/AWSReservedSSO_*_*/*" as arn_prefix, '
                    "permission_set, sso_id, engineer "
                    "| stats sum(input.inputTokenCount) as input_tokens by permission_set"
                ),
            )
            query_id = query_resp["queryId"]

            result = self._wait_for_query_complete(logs, query_id)

            assert result["status"] == "Complete"
            assert len(result["results"]) == 2  # Two permission sets

            # Find results by permission_set
            by_perm = {}
            for row in result["results"]:
                perm_set = next((f["value"] for f in row if f["field"] == "permission_set"), None)
                tokens = next((f["value"] for f in row if f["field"] == "input_tokens"), None)
                if perm_set:
                    by_perm[perm_set] = tokens

            assert "Administrator" in by_perm
            assert "BedrockEngineer" in by_perm
            assert float(by_perm["Administrator"]) == 100.0
            assert float(by_perm["BedrockEngineer"]) == 200.0

    def test_insights_filter_single_quoted_string(self, make_boto_client):
        """Filter with single-quoted string literal works."""
        suffix = uuid.uuid4().hex[:8]
        logs = make_boto_client("logs")

        log_group_name = f"/aws/test/insights-filter-{suffix}"

        with _cleanup_log_group(logs, log_group_name):
            logs.create_log_group(logGroupName=log_group_name)
            logs.create_log_stream(logGroupName=log_group_name, logStreamName="stream1")

            events = [
                {"status": "ERROR", "message": "Something failed"},
                {"status": "INFO", "message": "All good"},
                {"status": "ERROR", "message": "Another error"},
            ]

            logs.put_log_events(
                logGroupName=log_group_name,
                logStreamName="stream1",
                logEvents=[
                    {
                        "timestamp": int(time.time() * 1000) + i,
                        "message": json.dumps(e),
                    }
                    for i, e in enumerate(events)
                ],
            )

            # Query using single-quoted filter
            now = int(time.time())
            query_resp = logs.start_query(
                logGroupName=log_group_name,
                startTime=now - 3600,
                endTime=now + 60,
                queryString="filter status = 'ERROR' | stats count(*) as error_count",
            )
            query_id = query_resp["queryId"]

            result = self._wait_for_query_complete(logs, query_id)

            assert result["status"] == "Complete"
            assert len(result["results"]) == 1

            # The alias should be honored even without a group by clause.
            count_val = next(
                (f["value"] for f in result["results"][0] if f["field"] == "error_count"),
                None,
            )
            assert count_val is not None, (
                f"Expected field 'error_count' (the query's alias), got: {result['results'][0]}"
            )
            assert float(count_val) == 2.0

    def test_insights_full_dashboard_query(self, make_boto_client):
        """Full dashboard-style query with parse, stats, and alias."""
        suffix = uuid.uuid4().hex[:8]
        logs = make_boto_client("logs")

        log_group_name = f"/aws/test/insights-full-{suffix}"

        with _cleanup_log_group(logs, log_group_name):
            logs.create_log_group(logGroupName=log_group_name)
            logs.create_log_stream(logGroupName=log_group_name, logStreamName="stream1")

            # Multiple events across different permission sets
            events = [
                # Admin users
                {
                    "identity": {
                        "arn": (
                            "arn:aws:sts::123:assumed-role/AWSReservedSSO_Administrator_abc/alice"
                        )
                    },
                    "input": {"inputTokenCount": 500},
                },
                {
                    "identity": {
                        "arn": (
                            "arn:aws:sts::123:assumed-role/AWSReservedSSO_Administrator_abc/bob"
                        )
                    },
                    "input": {"inputTokenCount": 300},
                },
                # Engineer users
                {
                    "identity": {
                        "arn": (
                            "arn:aws:sts::123:assumed-role/AWSReservedSSO_BedrockEngineer_def/carol"
                        )
                    },
                    "input": {"inputTokenCount": 1000},
                },
                {
                    "identity": {
                        "arn": (
                            "arn:aws:sts::123:assumed-role/AWSReservedSSO_BedrockEngineer_def/dave"
                        )
                    },
                    "input": {"inputTokenCount": 600},
                },
                {
                    "identity": {
                        "arn": (
                            "arn:aws:sts::123:assumed-role/AWSReservedSSO_BedrockEngineer_def/eve"
                        )
                    },
                    "input": {"inputTokenCount": 400},
                },
            ]

            logs.put_log_events(
                logGroupName=log_group_name,
                logStreamName="stream1",
                logEvents=[
                    {
                        "timestamp": int(time.time() * 1000) + i,
                        "message": json.dumps(e),
                    }
                    for i, e in enumerate(events)
                ],
            )

            # Full dashboard query
            now = int(time.time())
            query = (
                'parse identity.arn "*assumed-role/AWSReservedSSO_*_*/*" as arn_prefix, '
                "permission_set, sso_id, engineer "
                "| stats sum(input.inputTokenCount) as input_tokens by permission_set"
            )
            query_resp = logs.start_query(
                logGroupName=log_group_name,
                startTime=now - 3600,
                endTime=now + 60,
                queryString=query,
            )
            query_id = query_resp["queryId"]

            result = self._wait_for_query_complete(logs, query_id)

            assert result["status"] == "Complete"
            # Should have 2 groups, not 1 ungrouped bucket
            assert len(result["results"]) == 2, f"Expected 2 groups, got {len(result['results'])}"

            # Build result map
            by_perm = {}
            for row in result["results"]:
                perm_set = next((f["value"] for f in row if f["field"] == "permission_set"), None)
                tokens = next((f["value"] for f in row if f["field"] == "input_tokens"), None)
                if perm_set:
                    by_perm[perm_set] = float(tokens) if tokens else 0

            # Verify grouped sums
            assert "Administrator" in by_perm
            assert "BedrockEngineer" in by_perm
            assert by_perm["Administrator"] == 800.0  # 500 + 300
            assert by_perm["BedrockEngineer"] == 2000.0  # 1000 + 600 + 400
