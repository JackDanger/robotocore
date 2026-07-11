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
