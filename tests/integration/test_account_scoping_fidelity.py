"""Account-scoping fidelity tests.

Each test creates the same-named resource under two distinct account IDs and
asserts that account A cannot see account B's resource.  A failure means a
cross-account state leak exists in the native provider.

AWS scoping reference:
- API Gateway V2 — per-account + per-region
- CloudWatch composite alarms — per-account + per-region
- CloudWatch dashboards — per-account (global)
- CloudWatch metric streams — per-account + per-region
- EventBridge connections — per-account + per-region
- EventBridge API destinations — per-account + per-region
- EventBridge endpoints — per-account + per-region
- Resource Groups Tagging API — per-account + per-region
- X-Ray sampling rules — per-account + per-region
- X-Ray groups — per-account + per-region
"""

import json
import uuid

ACCT_A = "111111111111"
ACCT_B = "222222222222"


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _client(make_boto_client, service: str, account_id: str, region: str = "us-east-1"):
    return make_boto_client(service, region_name=region, aws_access_key_id=account_id)


# ---------------------------------------------------------------------------
# API Gateway V2
# ---------------------------------------------------------------------------


class TestApiGatewayV2AccountIsolation:
    """API GW v2 HTTP APIs are per-account + per-region in AWS."""

    def test_apis_isolated_by_account(self, make_boto_client):
        suffix = uuid.uuid4().hex[:8]
        name_a = f"api-a-{suffix}"
        name_b = f"api-b-{suffix}"

        gw_a = _client(make_boto_client, "apigatewayv2", ACCT_A)
        gw_b = _client(make_boto_client, "apigatewayv2", ACCT_B)

        api_a = gw_a.create_api(Name=name_a, ProtocolType="HTTP")
        api_b = gw_b.create_api(Name=name_b, ProtocolType="HTTP")
        api_a_id = api_a["ApiId"]
        api_b_id = api_b["ApiId"]

        try:
            # Account A should see its own API
            apis_a = gw_a.get_apis()["Items"]
            ids_a = [a["ApiId"] for a in apis_a]
            assert api_a_id in ids_a, "Account A should see its own API"

            # Account A must NOT see account B's API
            assert api_b_id not in ids_a, (
                "Account A must not see account B's API (cross-account leak)"
            )

            # Account B should see its own API but not A's
            apis_b = gw_b.get_apis()["Items"]
            ids_b = [a["ApiId"] for a in apis_b]
            assert api_b_id in ids_b, "Account B should see its own API"
            assert api_a_id not in ids_b, (
                "Account B must not see account A's API (cross-account leak)"
            )

        finally:
            try:
                gw_a.delete_api(ApiId=api_a_id)
            except Exception:
                pass  # best-effort cleanup
            try:
                gw_b.delete_api(ApiId=api_b_id)
            except Exception:
                pass  # best-effort cleanup


# ---------------------------------------------------------------------------
# CloudWatch — composite alarms
# ---------------------------------------------------------------------------


class TestCloudWatchCompositeAlarmIsolation:
    """CloudWatch composite alarms are per-account + per-region in AWS."""

    def test_composite_alarms_isolated_by_account(self, make_boto_client):
        suffix = uuid.uuid4().hex[:8]
        # We need at least one metric alarm as a dependency for the rule to parse.
        # Use a name that doesn't exist; rule syntax validation only needs valid form.
        alarm_name_a = f"composite-a-{suffix}"
        alarm_name_b = f"composite-b-{suffix}"

        cw_a = _client(make_boto_client, "cloudwatch", ACCT_A)
        cw_b = _client(make_boto_client, "cloudwatch", ACCT_B)

        # Create a simple base metric alarm in each account first
        base_a = f"base-a-{suffix}"
        base_b = f"base-b-{suffix}"

        cw_a.put_metric_alarm(
            AlarmName=base_a,
            ComparisonOperator="GreaterThanThreshold",
            EvaluationPeriods=1,
            MetricName="CPUUtilization",
            Namespace="AWS/EC2",
            Period=60,
            Statistic="Average",
            Threshold=80.0,
        )
        cw_b.put_metric_alarm(
            AlarmName=base_b,
            ComparisonOperator="GreaterThanThreshold",
            EvaluationPeriods=1,
            MetricName="CPUUtilization",
            Namespace="AWS/EC2",
            Period=60,
            Statistic="Average",
            Threshold=80.0,
        )

        cw_a.put_composite_alarm(
            AlarmName=alarm_name_a,
            AlarmRule=f'ALARM("{base_a}")',
        )
        cw_b.put_composite_alarm(
            AlarmName=alarm_name_b,
            AlarmRule=f'ALARM("{base_b}")',
        )

        try:
            resp_a = cw_a.describe_alarms(AlarmTypes=["CompositeAlarm"])
            names_a = [a["AlarmName"] for a in resp_a.get("CompositeAlarms", [])]
            assert alarm_name_a in names_a, "Account A should see its composite alarm"
            assert alarm_name_b not in names_a, (
                "Account A must not see account B's composite alarm (cross-account leak)"
            )

            resp_b = cw_b.describe_alarms(AlarmTypes=["CompositeAlarm"])
            names_b = [a["AlarmName"] for a in resp_b.get("CompositeAlarms", [])]
            assert alarm_name_b in names_b, "Account B should see its composite alarm"
            assert alarm_name_a not in names_b, (
                "Account B must not see account A's composite alarm (cross-account leak)"
            )
        finally:
            for cw, name in [
                (cw_a, alarm_name_a),
                (cw_b, alarm_name_b),
                (cw_a, base_a),
                (cw_b, base_b),
            ]:
                try:
                    cw.delete_alarms(AlarmNames=[name])
                except Exception:
                    pass  # best-effort cleanup


# ---------------------------------------------------------------------------
# CloudWatch — dashboards
# ---------------------------------------------------------------------------


class TestCloudWatchDashboardIsolation:
    """CloudWatch dashboards are per-account in AWS."""

    def _valid_body(self) -> str:
        return json.dumps({"widgets": [{"type": "text", "properties": {"markdown": "hi"}}]})

    def test_dashboards_isolated_by_account(self, make_boto_client):
        suffix = uuid.uuid4().hex[:8]
        name = f"dash-{suffix}"  # same name in both accounts

        cw_a = _client(make_boto_client, "cloudwatch", ACCT_A)
        cw_b = _client(make_boto_client, "cloudwatch", ACCT_B)

        cw_a.put_dashboard(DashboardName=name, DashboardBody=self._valid_body())
        cw_b.put_dashboard(DashboardName=name, DashboardBody=self._valid_body())

        try:
            # Both created successfully; now verify isolation
            resp_a = cw_a.list_dashboards()
            names_a = [d["DashboardName"] for d in resp_a.get("DashboardEntries", [])]

            resp_b = cw_b.list_dashboards()
            names_b = [d["DashboardName"] for d in resp_b.get("DashboardEntries", [])]

            # Both accounts should see their own dashboard
            assert name in names_a, "Account A should see its dashboard"
            assert name in names_b, "Account B should see its dashboard"

            # Verify the ARNs are account-specific (different accounts → different ARNs)
            arn_a_entries = [d for d in resp_a["DashboardEntries"] if d["DashboardName"] == name]
            arn_b_entries = [d for d in resp_b["DashboardEntries"] if d["DashboardName"] == name]
            assert len(arn_a_entries) == 1
            assert len(arn_b_entries) == 1
            assert arn_a_entries[0]["DashboardArn"] != arn_b_entries[0]["DashboardArn"], (
                "Same-name dashboards in different accounts must have different ARNs"
            )
        finally:
            for cw in [cw_a, cw_b]:
                try:
                    cw.delete_dashboards(DashboardNames=[name])
                except Exception:
                    pass  # best-effort cleanup


# ---------------------------------------------------------------------------
# CloudWatch — metric streams
# ---------------------------------------------------------------------------


class TestCloudWatchMetricStreamIsolation:
    """CloudWatch metric streams are per-account + per-region in AWS."""

    def test_metric_streams_isolated_by_account(self, make_boto_client):
        suffix = uuid.uuid4().hex[:8]
        name = f"stream-{suffix}"

        cw_a = _client(make_boto_client, "cloudwatch", ACCT_A)
        cw_b = _client(make_boto_client, "cloudwatch", ACCT_B)

        cw_a.put_metric_stream(
            Name=name,
            FirehoseArn=f"arn:aws:firehose:us-east-1:{ACCT_A}:deliverystream/test",
            RoleArn=f"arn:aws:iam::{ACCT_A}:role/cw-streams",
            OutputFormat="json",
        )
        cw_b.put_metric_stream(
            Name=name,
            FirehoseArn=f"arn:aws:firehose:us-east-1:{ACCT_B}:deliverystream/test",
            RoleArn=f"arn:aws:iam::{ACCT_B}:role/cw-streams",
            OutputFormat="json",
        )

        try:
            resp_a = cw_a.list_metric_streams()
            names_a = [s["Name"] for s in resp_a.get("Entries", [])]
            assert name in names_a, "Account A should see its metric stream"

            resp_b = cw_b.list_metric_streams()
            names_b = [s["Name"] for s in resp_b.get("Entries", [])]
            assert name in names_b, "Account B should see its metric stream"

            # ARNs must differ (account-specific)
            arn_a = next(s["Arn"] for s in resp_a["Entries"] if s["Name"] == name)
            arn_b = next(s["Arn"] for s in resp_b["Entries"] if s["Name"] == name)
            assert arn_a != arn_b, (
                "Same-name metric streams in different accounts must have different ARNs"
            )
        finally:
            for cw in [cw_a, cw_b]:
                try:
                    cw.delete_metric_stream(Name=name)
                except Exception:
                    pass  # best-effort cleanup


# ---------------------------------------------------------------------------
# EventBridge — connections
# ---------------------------------------------------------------------------


class TestEventBridgeConnectionIsolation:
    """EventBridge connections are per-account + per-region in AWS."""

    def test_connections_isolated_by_account(self, make_boto_client):
        suffix = uuid.uuid4().hex[:8]
        name = f"conn-{suffix}"

        eb_a = _client(make_boto_client, "events", ACCT_A)
        eb_b = _client(make_boto_client, "events", ACCT_B)

        eb_a.create_connection(
            Name=name,
            AuthorizationType="API_KEY",
            AuthParameters={
                "ApiKeyAuthParameters": {"ApiKeyName": "X-Api-Key", "ApiKeyValue": "secret-a"}
            },
        )
        eb_b.create_connection(
            Name=name,
            AuthorizationType="API_KEY",
            AuthParameters={
                "ApiKeyAuthParameters": {"ApiKeyName": "X-Api-Key", "ApiKeyValue": "secret-b"}
            },
        )

        try:
            resp_a = eb_a.list_connections()
            names_a = [c["Name"] for c in resp_a.get("Connections", [])]
            assert name in names_a, "Account A should see its connection"

            resp_b = eb_b.list_connections()
            names_b = [c["Name"] for c in resp_b.get("Connections", [])]
            assert name in names_b, "Account B should see its connection"

            # Each account should see exactly one entry with this name
            count_a = names_a.count(name)
            count_b = names_b.count(name)
            assert count_a == 1, f"Account A sees {count_a} connections named {name!r}, expected 1"
            assert count_b == 1, f"Account B sees {count_b} connections named {name!r}, expected 1"

            # ARNs must differ
            arn_a = next(c["ConnectionArn"] for c in resp_a["Connections"] if c["Name"] == name)
            arn_b = next(c["ConnectionArn"] for c in resp_b["Connections"] if c["Name"] == name)
            assert arn_a != arn_b, (
                "Same-name connections in different accounts must have different ARNs"
            )
        finally:
            for eb, n in [(eb_a, name), (eb_b, name)]:
                try:
                    eb.delete_connection(Name=n)
                except Exception:
                    pass  # best-effort cleanup


# ---------------------------------------------------------------------------
# EventBridge — API destinations
# ---------------------------------------------------------------------------


class TestEventBridgeApiDestinationIsolation:
    """EventBridge API destinations are per-account + per-region in AWS."""

    def _setup_connection(self, eb_client, account_id: str, suffix: str) -> str:
        name = f"conn-dest-{account_id[-4:]}-{suffix}"
        resp = eb_client.create_connection(
            Name=name,
            AuthorizationType="API_KEY",
            AuthParameters={
                "ApiKeyAuthParameters": {"ApiKeyName": "X-Api-Key", "ApiKeyValue": "v"}
            },
        )
        return resp["ConnectionArn"]

    def test_api_destinations_isolated_by_account(self, make_boto_client):
        suffix = uuid.uuid4().hex[:8]
        dest_name = f"dest-{suffix}"

        eb_a = _client(make_boto_client, "events", ACCT_A)
        eb_b = _client(make_boto_client, "events", ACCT_B)

        conn_arn_a = self._setup_connection(eb_a, ACCT_A, suffix)
        conn_arn_b = self._setup_connection(eb_b, ACCT_B, suffix)

        eb_a.create_api_destination(
            Name=dest_name,
            ConnectionArn=conn_arn_a,
            InvocationEndpoint="https://example.com/a",
            HttpMethod="POST",
        )
        eb_b.create_api_destination(
            Name=dest_name,
            ConnectionArn=conn_arn_b,
            InvocationEndpoint="https://example.com/b",
            HttpMethod="POST",
        )

        try:
            resp_a = eb_a.list_api_destinations()
            names_a = [d["Name"] for d in resp_a.get("ApiDestinations", [])]
            assert dest_name in names_a, "Account A should see its API destination"

            resp_b = eb_b.list_api_destinations()
            names_b = [d["Name"] for d in resp_b.get("ApiDestinations", [])]
            assert dest_name in names_b, "Account B should see its API destination"

            count_a = names_a.count(dest_name)
            count_b = names_b.count(dest_name)
            assert count_a == 1, (
                f"Account A sees {count_a} destinations named {dest_name!r}, expected 1"
            )
            assert count_b == 1, (
                f"Account B sees {count_b} destinations named {dest_name!r}, expected 1"
            )

            arn_a = next(
                d["ApiDestinationArn"] for d in resp_a["ApiDestinations"] if d["Name"] == dest_name
            )
            arn_b = next(
                d["ApiDestinationArn"] for d in resp_b["ApiDestinations"] if d["Name"] == dest_name
            )
            assert arn_a != arn_b, (
                "Same-name API destinations in different accounts must have different ARNs"
            )
        finally:
            for eb, n in [(eb_a, dest_name), (eb_b, dest_name)]:
                try:
                    eb.delete_api_destination(Name=n)
                except Exception:
                    pass  # best-effort cleanup
            conn_name_a = f"conn-dest-{ACCT_A[-4:]}-{suffix}"
            conn_name_b = f"conn-dest-{ACCT_B[-4:]}-{suffix}"
            for eb, n in [(eb_a, conn_name_a), (eb_b, conn_name_b)]:
                try:
                    eb.delete_connection(Name=n)
                except Exception:
                    pass  # best-effort cleanup


# ---------------------------------------------------------------------------
# EventBridge — endpoints
# ---------------------------------------------------------------------------


class TestEventBridgeEndpointIsolation:
    """EventBridge endpoints are per-account in AWS (global, not per-region)."""

    def _make_endpoint_params(self, account_id: str, suffix: str) -> dict:
        bus_arn_1 = f"arn:aws:events:us-east-1:{account_id}:event-bus/default"
        bus_arn_2 = f"arn:aws:events:us-west-2:{account_id}:event-bus/default"
        return {
            "RoutingConfig": {
                "FailoverConfig": {
                    "Primary": {"HealthCheck": f"arn:aws:route53:::healthcheck/fake-{suffix}"},
                    "Secondary": {"Route": f"https://ep-{suffix}.us-west-2.events.amazonaws.com"},
                }
            },
            "EventBuses": [{"EventBusArn": bus_arn_1}, {"EventBusArn": bus_arn_2}],
            "RoleArn": f"arn:aws:iam::{account_id}:role/eb-endpoint",
        }

    def test_endpoints_isolated_by_account(self, make_boto_client):
        suffix = uuid.uuid4().hex[:8]
        ep_name = f"ep-{suffix}"

        eb_a = _client(make_boto_client, "events", ACCT_A)
        eb_b = _client(make_boto_client, "events", ACCT_B)

        eb_a.create_endpoint(Name=ep_name, **self._make_endpoint_params(ACCT_A, suffix))
        eb_b.create_endpoint(Name=ep_name, **self._make_endpoint_params(ACCT_B, suffix))

        try:
            resp_a = eb_a.list_endpoints()
            names_a = [e["Name"] for e in resp_a.get("Endpoints", [])]
            assert ep_name in names_a, "Account A should see its endpoint"

            resp_b = eb_b.list_endpoints()
            names_b = [e["Name"] for e in resp_b.get("Endpoints", [])]
            assert ep_name in names_b, "Account B should see its endpoint"

            count_a = names_a.count(ep_name)
            count_b = names_b.count(ep_name)
            assert count_a == 1, f"Account A sees {count_a} endpoints named {ep_name!r}, expected 1"
            assert count_b == 1, f"Account B sees {count_b} endpoints named {ep_name!r}, expected 1"

            arn_a = next(
                (e.get("EndpointArn") or e.get("Arn", ""))
                for e in resp_a["Endpoints"]
                if e["Name"] == ep_name
            )
            arn_b = next(
                (e.get("EndpointArn") or e.get("Arn", ""))
                for e in resp_b["Endpoints"]
                if e["Name"] == ep_name
            )
            assert arn_a != arn_b, (
                "Same-name endpoints in different accounts must have different ARNs"
            )
        finally:
            for eb in [eb_a, eb_b]:
                try:
                    eb.delete_endpoint(Name=ep_name)
                except Exception:
                    pass  # best-effort cleanup


# ---------------------------------------------------------------------------
# X-Ray — sampling rules and groups
# ---------------------------------------------------------------------------


class TestXRayAccountIsolation:
    """X-Ray sampling rules and groups are per-account + per-region in AWS."""

    def test_sampling_rules_isolated_by_account(self, make_boto_client):
        """Create same-named sampling rule in both accounts; verify isolation."""
        suffix = uuid.uuid4().hex[:8]
        rule_name = f"rule-{suffix}"

        xray_a = _client(make_boto_client, "xray", ACCT_A)
        xray_b = _client(make_boto_client, "xray", ACCT_B)

        xray_a.create_sampling_rule(
            SamplingRule={
                "RuleName": rule_name,
                "ResourceARN": "*",
                "Priority": 1000,
                "FixedRate": 0.05,
                "ReservoirSize": 1,
                "ServiceName": "*",
                "ServiceType": "*",
                "Host": "*",
                "HTTPMethod": "*",
                "URLPath": "*",
                "Version": 1,
            }
        )
        xray_b.create_sampling_rule(
            SamplingRule={
                "RuleName": rule_name,
                "ResourceARN": "*",
                "Priority": 1000,
                "FixedRate": 0.05,
                "ReservoirSize": 1,
                "ServiceName": "*",
                "ServiceType": "*",
                "Host": "*",
                "HTTPMethod": "*",
                "URLPath": "*",
                "Version": 1,
            }
        )

        try:
            # Account A should see only its own rule
            rules_a = xray_a.get_sampling_rules()["SamplingRuleRecords"]
            names_a = [r["SamplingRule"]["RuleName"] for r in rules_a]
            assert rule_name in names_a, "Account A should see its sampling rule"
            assert names_a.count(rule_name) == 1, (
                f"Account A sees {names_a.count(rule_name)} rules named {rule_name!r}"
            )

            # Account B should see only its own rule
            rules_b = xray_b.get_sampling_rules()["SamplingRuleRecords"]
            names_b = [r["SamplingRule"]["RuleName"] for r in rules_b]
            assert rule_name in names_b, "Account B should see its sampling rule"
            assert names_b.count(rule_name) == 1, (
                f"Account B sees {names_b.count(rule_name)} rules named {rule_name!r}"
            )

            # ARNs must differ (account-specific)
            arn_a = next(
                r["SamplingRule"]["RuleARN"]
                for r in rules_a
                if r["SamplingRule"]["RuleName"] == rule_name
            )
            arn_b = next(
                r["SamplingRule"]["RuleARN"]
                for r in rules_b
                if r["SamplingRule"]["RuleName"] == rule_name
            )
            assert arn_a != arn_b, (
                "Same-name sampling rules in different accounts must have different ARNs"
            )

        finally:
            for xray in [xray_a, xray_b]:
                try:
                    xray.delete_sampling_rule(RuleName=rule_name)
                except Exception:
                    pass  # best-effort cleanup

    def test_groups_isolated_by_account(self, make_boto_client):
        """Create same-named group in both accounts; verify isolation."""
        suffix = uuid.uuid4().hex[:8]
        group_name = f"group-{suffix}"

        xray_a = _client(make_boto_client, "xray", ACCT_A)
        xray_b = _client(make_boto_client, "xray", ACCT_B)

        xray_a.create_group(GroupName=group_name, FilterExpression="service(a)")
        xray_b.create_group(GroupName=group_name, FilterExpression="service(b)")

        try:
            # Account A should see only its own group
            groups_a = xray_a.get_groups()["Groups"]
            names_a = [g["GroupName"] for g in groups_a]
            assert group_name in names_a, "Account A should see its group"
            assert names_a.count(group_name) == 1, (
                f"Account A sees {names_a.count(group_name)} groups named {group_name!r}"
            )

            # Account B should see only its own group
            groups_b = xray_b.get_groups()["Groups"]
            names_b = [g["GroupName"] for g in groups_b]
            assert group_name in names_b, "Account B should see its group"
            assert names_b.count(group_name) == 1, (
                f"Account B sees {names_b.count(group_name)} groups named {group_name!r}"
            )

            # ARNs must differ (account-specific)
            arn_a = next(g["GroupARN"] for g in groups_a if g["GroupName"] == group_name)
            arn_b = next(g["GroupARN"] for g in groups_b if g["GroupName"] == group_name)
            assert arn_a != arn_b, "Same-name groups in different accounts must have different ARNs"

            # Verify GetGroup returns only the account's own group
            group_a = xray_a.get_group(GroupName=group_name)["Group"]
            assert ACCT_A in group_a["GroupARN"]

            group_b = xray_b.get_group(GroupName=group_name)["Group"]
            assert ACCT_B in group_b["GroupARN"]

        finally:
            for xray in [xray_a, xray_b]:
                try:
                    xray.delete_group(GroupName=group_name)
                except Exception:
                    pass  # best-effort cleanup


# ---------------------------------------------------------------------------
# Resource Groups Tagging API
# ---------------------------------------------------------------------------


class TestTaggingApiAccountIsolation:
    """Resource Groups Tagging API get_resources is per-account + per-region in AWS."""

    def test_sqs_resources_isolated_by_account(self, make_boto_client):
        """Create tagged SQS queues in both accounts; verify tagging API isolation."""
        suffix = uuid.uuid4().hex[:8]
        queue_name = f"queue-{suffix}"
        tag_key = "TestTag"

        sqs_a = _client(make_boto_client, "sqs", ACCT_A)
        sqs_b = _client(make_boto_client, "sqs", ACCT_B)
        tagging_a = _client(make_boto_client, "resourcegroupstaggingapi", ACCT_A)
        tagging_b = _client(make_boto_client, "resourcegroupstaggingapi", ACCT_B)

        # Create queues with distinctive tags
        resp_a = sqs_a.create_queue(QueueName=queue_name, tags={tag_key: "account-a"})
        resp_b = sqs_b.create_queue(QueueName=queue_name, tags={tag_key: "account-b"})
        url_a = resp_a["QueueUrl"]
        url_b = resp_b["QueueUrl"]

        try:
            # Account A's tagging API should only see its own queue
            resources_a = tagging_a.get_resources(
                ResourceTypeFilters=["sqs"],
                TagFilters=[{"Key": tag_key}],
            )["ResourceTagMappingList"]
            arns_a = [r["ResourceARN"] for r in resources_a]
            assert any(ACCT_A in arn for arn in arns_a), "Account A should see its SQS queue"
            assert not any(ACCT_B in arn for arn in arns_a), (
                "Account A must not see account B's SQS queue (cross-account leak)"
            )

            # Account B's tagging API should only see its own queue
            resources_b = tagging_b.get_resources(
                ResourceTypeFilters=["sqs"],
                TagFilters=[{"Key": tag_key}],
            )["ResourceTagMappingList"]
            arns_b = [r["ResourceARN"] for r in resources_b]
            assert any(ACCT_B in arn for arn in arns_b), "Account B should see its SQS queue"
            assert not any(ACCT_A in arn for arn in arns_b), (
                "Account B must not see account A's SQS queue (cross-account leak)"
            )

            # Verify tag values are correct per account
            for r in resources_a:
                tags = {t["Key"]: t["Value"] for t in r.get("Tags", [])}
                assert tags.get(tag_key) == "account-a", (
                    "Account A's queue should have account-a tag"
                )

            for r in resources_b:
                tags = {t["Key"]: t["Value"] for t in r.get("Tags", [])}
                assert tags.get(tag_key) == "account-b", (
                    "Account B's queue should have account-b tag"
                )

        finally:
            for sqs, url in [(sqs_a, url_a), (sqs_b, url_b)]:
                try:
                    sqs.delete_queue(QueueUrl=url)
                except Exception:
                    pass  # best-effort cleanup

    def test_sns_resources_isolated_by_account(self, make_boto_client):
        """Create tagged SNS topics in both accounts; verify tagging API isolation."""
        suffix = uuid.uuid4().hex[:8]
        topic_name = f"topic-{suffix}"
        tag_key = "TestTag"

        sns_a = _client(make_boto_client, "sns", ACCT_A)
        sns_b = _client(make_boto_client, "sns", ACCT_B)
        tagging_a = _client(make_boto_client, "resourcegroupstaggingapi", ACCT_A)
        tagging_b = _client(make_boto_client, "resourcegroupstaggingapi", ACCT_B)

        # Create topics with distinctive tags
        resp_a = sns_a.create_topic(Name=topic_name, Tags=[{"Key": tag_key, "Value": "account-a"}])
        resp_b = sns_b.create_topic(Name=topic_name, Tags=[{"Key": tag_key, "Value": "account-b"}])
        arn_a = resp_a["TopicArn"]
        arn_b = resp_b["TopicArn"]

        try:
            # Account A's tagging API should only see its own topic
            resources_a = tagging_a.get_resources(
                ResourceTypeFilters=["sns"],
                TagFilters=[{"Key": tag_key}],
            )["ResourceTagMappingList"]
            arns_a = [r["ResourceARN"] for r in resources_a]
            assert any(ACCT_A in arn for arn in arns_a), "Account A should see its SNS topic"
            assert not any(ACCT_B in arn for arn in arns_a), (
                "Account A must not see account B's SNS topic (cross-account leak)"
            )

            # Account B's tagging API should only see its own topic
            resources_b = tagging_b.get_resources(
                ResourceTypeFilters=["sns"],
                TagFilters=[{"Key": tag_key}],
            )["ResourceTagMappingList"]
            arns_b = [r["ResourceARN"] for r in resources_b]
            assert any(ACCT_B in arn for arn in arns_b), "Account B should see its SNS topic"
            assert not any(ACCT_A in arn for arn in arns_b), (
                "Account B must not see account A's SNS topic (cross-account leak)"
            )

            # Verify tag values are correct per account
            for r in resources_a:
                tags = {t["Key"]: t["Value"] for t in r.get("Tags", [])}
                assert tags.get(tag_key) == "account-a", (
                    "Account A's topic should have account-a tag"
                )

            for r in resources_b:
                tags = {t["Key"]: t["Value"] for t in r.get("Tags", [])}
                assert tags.get(tag_key) == "account-b", (
                    "Account B's topic should have account-b tag"
                )

        finally:
            for sns, arn in [(sns_a, arn_a), (sns_b, arn_b)]:
                try:
                    sns.delete_topic(TopicArn=arn)
                except Exception:
                    pass  # best-effort cleanup
