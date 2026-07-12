"""Integration tests for update field persistence bug class.

Tests that update operations properly persist all fields that create stores,
not just a subset. This catches the "Update operation silently drops fields
that Create stores" bug class.

Each test:
1. Creates the resource with SOME fields set to a baseline value.
2. Calls Update with the affected field(s) set to a NEW, distinguishable value.
3. Calls the matching Describe/Get op and asserts the NEW value round-tripped.

For Cognito specifically, also covers the create-time fields (create with them set,
describe, assert present) since that bug was at create time, not update time.
"""

import uuid

import pytest  # noqa: F401

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _suffix() -> str:
    """Return a short unique suffix for resource names."""
    return uuid.uuid4().hex[:8]


# ---------------------------------------------------------------------------
# EventBridge Scheduler - UpdateSchedule
# ---------------------------------------------------------------------------


class TestSchedulerUpdateFieldPersistence:
    """EventBridge Scheduler UpdateSchedule used to drop ScheduleExpressionTimezone,
    StartDate, EndDate, KmsKeyArn.
    """

    def test_update_schedule_expression_timezone(self, make_boto_client):
        """UpdateSchedule should persist ScheduleExpressionTimezone changes."""
        suffix = _suffix()
        scheduler = make_boto_client("scheduler")

        schedule_name = f"test-schedule-{suffix}"
        baseline_tz = "UTC"
        new_tz = "America/New_York"

        # Create with baseline timezone
        scheduler.create_schedule(
            Name=schedule_name,
            ScheduleExpression="rate(1 hour)",
            ScheduleExpressionTimezone=baseline_tz,
            Target={
                "Arn": "arn:aws:lambda:us-east-1:123456789012:function:test",
                "RoleArn": "arn:aws:iam::123456789012:role/scheduler-role",
            },
            FlexibleTimeWindow={"Mode": "OFF"},
        )

        try:
            # Update to new timezone (required params must be included)
            scheduler.update_schedule(
                Name=schedule_name,
                ScheduleExpression="rate(1 hour)",
                ScheduleExpressionTimezone=new_tz,
                Target={
                    "Arn": "arn:aws:lambda:us-east-1:123456789012:function:test",
                    "RoleArn": "arn:aws:iam::123456789012:role/scheduler-role",
                },
                FlexibleTimeWindow={"Mode": "OFF"},
            )

            # Verify the new timezone persisted
            result = scheduler.get_schedule(Name=schedule_name)
            assert result["ScheduleExpressionTimezone"] == new_tz, (
                f"UpdateSchedule dropped ScheduleExpressionTimezone: "
                f"expected {new_tz!r}, got {result.get('ScheduleExpressionTimezone', 'MISSING')!r}"
            )
        finally:
            try:
                scheduler.delete_schedule(Name=schedule_name)
            except Exception:
                pass  # best-effort cleanup

    def test_update_schedule_start_date(self, make_boto_client):
        """UpdateSchedule should persist StartDate changes."""
        suffix = _suffix()
        scheduler = make_boto_client("scheduler")

        schedule_name = f"test-schedule-{suffix}"
        baseline_start = "2025-01-01T00:00:00Z"
        new_start = "2025-06-15T12:00:00Z"

        # Create with baseline start date
        scheduler.create_schedule(
            Name=schedule_name,
            ScheduleExpression="rate(1 hour)",
            StartDate=baseline_start,
            Target={
                "Arn": "arn:aws:lambda:us-east-1:123456789012:function:test",
                "RoleArn": "arn:aws:iam::123456789012:role/scheduler-role",
            },
            FlexibleTimeWindow={"Mode": "OFF"},
        )

        try:
            # Update to new start date (required params must be included)
            scheduler.update_schedule(
                Name=schedule_name,
                ScheduleExpression="rate(1 hour)",
                StartDate=new_start,
                Target={
                    "Arn": "arn:aws:lambda:us-east-1:123456789012:function:test",
                    "RoleArn": "arn:aws:iam::123456789012:role/scheduler-role",
                },
                FlexibleTimeWindow={"Mode": "OFF"},
            )

            # Verify the new start date persisted (compare as datetime)
            result = scheduler.get_schedule(Name=schedule_name)
            from datetime import datetime

            expected_dt = datetime.fromisoformat(new_start.replace("Z", "+00:00"))
            actual_dt = result["StartDate"]
            assert actual_dt == expected_dt, (
                f"UpdateSchedule dropped StartDate: expected {expected_dt!r}, got {actual_dt!r}"
            )
        finally:
            try:
                scheduler.delete_schedule(Name=schedule_name)
            except Exception:
                pass  # best-effort cleanup

    def test_update_schedule_end_date(self, make_boto_client):
        """UpdateSchedule should persist EndDate changes."""
        suffix = _suffix()
        scheduler = make_boto_client("scheduler")

        schedule_name = f"test-schedule-{suffix}"
        baseline_end = "2025-12-31T23:59:59Z"
        new_end = "2025-06-30T23:59:59Z"

        # Create with baseline end date
        scheduler.create_schedule(
            Name=schedule_name,
            ScheduleExpression="rate(1 hour)",
            EndDate=baseline_end,
            Target={
                "Arn": "arn:aws:lambda:us-east-1:123456789012:function:test",
                "RoleArn": "arn:aws:iam::123456789012:role/scheduler-role",
            },
            FlexibleTimeWindow={"Mode": "OFF"},
        )

        try:
            # Update to new end date (required params must be included)
            scheduler.update_schedule(
                Name=schedule_name,
                ScheduleExpression="rate(1 hour)",
                EndDate=new_end,
                Target={
                    "Arn": "arn:aws:lambda:us-east-1:123456789012:function:test",
                    "RoleArn": "arn:aws:iam::123456789012:role/scheduler-role",
                },
                FlexibleTimeWindow={"Mode": "OFF"},
            )

            # Verify the new end date persisted (compare as datetime)
            result = scheduler.get_schedule(Name=schedule_name)
            from datetime import datetime

            expected_dt = datetime.fromisoformat(new_end.replace("Z", "+00:00"))
            actual_dt = result["EndDate"]
            assert actual_dt == expected_dt, (
                f"UpdateSchedule dropped EndDate: expected {expected_dt!r}, got {actual_dt!r}"
            )
        finally:
            try:
                scheduler.delete_schedule(Name=schedule_name)
            except Exception:
                pass  # best-effort cleanup

    def test_update_schedule_kms_key_arn(self, make_boto_client):
        """UpdateSchedule should persist KmsKeyArn changes."""
        suffix = _suffix()
        scheduler = make_boto_client("scheduler")

        schedule_name = f"test-schedule-{suffix}"
        new_kms_key = "arn:aws:kms:us-east-1:123456789012:key/12345678-1234-1234-1234-123456789012"

        # Create without KMS key
        scheduler.create_schedule(
            Name=schedule_name,
            ScheduleExpression="rate(1 hour)",
            Target={
                "Arn": "arn:aws:lambda:us-east-1:123456789012:function:test",
                "RoleArn": "arn:aws:iam::123456789012:role/scheduler-role",
            },
            FlexibleTimeWindow={"Mode": "OFF"},
        )

        try:
            # Update to add KMS key (required params must be included)
            scheduler.update_schedule(
                Name=schedule_name,
                ScheduleExpression="rate(1 hour)",
                KmsKeyArn=new_kms_key,
                Target={
                    "Arn": "arn:aws:lambda:us-east-1:123456789012:function:test",
                    "RoleArn": "arn:aws:iam::123456789012:role/scheduler-role",
                },
                FlexibleTimeWindow={"Mode": "OFF"},
            )

            # Verify the KMS key persisted
            result = scheduler.get_schedule(Name=schedule_name)
            assert result.get("KmsKeyArn") == new_kms_key, (
                f"UpdateSchedule dropped KmsKeyArn: "
                f"expected {new_kms_key!r}, got {result.get('KmsKeyArn', 'MISSING')!r}"
            )
        finally:
            try:
                scheduler.delete_schedule(Name=schedule_name)
            except Exception:
                pass  # best-effort cleanup


# ---------------------------------------------------------------------------
# AppSync - UpdateDataSource
# ---------------------------------------------------------------------------


class TestAppSyncUpdateFieldPersistence:
    """AppSync UpdateDataSource used to drop httpConfig."""

    def test_update_data_source_http_config(self, make_boto_client):
        """UpdateDataSource should persist httpConfig changes."""
        suffix = _suffix()
        appsync = make_boto_client("appsync")

        api_name = f"test-api-{suffix}"
        ds_name = f"test-ds-{suffix}"

        # Create API
        api = appsync.create_graphql_api(
            name=api_name,
            authenticationType="API_KEY",
        )
        api_id = api["graphqlApi"]["apiId"]

        try:
            # Create HTTP data source with baseline config
            baseline_http_config = {
                "endpoint": "https://api.example.com/v1",
                "authorizationConfig": {
                    "authorizationType": "AWS_IAM",
                },
            }
            appsync.create_data_source(
                apiId=api_id,
                name=ds_name,
                type="HTTP",
                httpConfig=baseline_http_config,
            )

            # Update to new HTTP config (type is required)
            new_http_config = {
                "endpoint": "https://api.example.com/v2",
            }
            appsync.update_data_source(
                apiId=api_id,
                name=ds_name,
                type="HTTP",
                httpConfig=new_http_config,
            )

            # Verify the new httpConfig persisted
            result = appsync.get_data_source(apiId=api_id, name=ds_name)
            actual_config = result.get("dataSource", {}).get("httpConfig", {})
            assert actual_config.get("endpoint") == "https://api.example.com/v2", (
                f"UpdateDataSource dropped httpConfig: "
                f"expected endpoint 'https://api.example.com/v2', got {actual_config!r}"
            )
        finally:
            try:
                appsync.delete_data_source(apiId=api_id, name=ds_name)
                appsync.delete_graphql_api(apiId=api_id)
            except Exception:
                pass  # best-effort cleanup


# ---------------------------------------------------------------------------
# API Gateway v2 - UpdateStage
# ---------------------------------------------------------------------------


class TestApiGatewayV2UpdateFieldPersistence:
    """API Gateway v2 UpdateStage used to drop AccessLogSettings."""

    def test_update_stage_access_log_settings(self, make_boto_client):
        """UpdateStage should persist AccessLogSettings changes."""
        suffix = _suffix()
        apigw = make_boto_client("apigatewayv2")

        api_name = f"test-api-{suffix}"
        stage_name = "test-stage"

        # Create HTTP API
        api = apigw.create_api(
            Name=api_name,
            ProtocolType="HTTP",
        )
        api_id = api["ApiId"]

        try:
            # Create stage without access log settings
            apigw.create_stage(
                ApiId=api_id,
                StageName=stage_name,
            )

            # Update to add access log settings
            new_log_settings = {
                "DestinationArn": (
                    f"arn:aws:logs:us-east-1:123456789012:log-group:/aws/apigateway/{api_name}"
                ),
                "Format": '{"requestId":"$context.requestId"}',
            }
            apigw.update_stage(
                ApiId=api_id,
                StageName=stage_name,
                AccessLogSettings=new_log_settings,
            )

            # Verify the access log settings persisted
            result = apigw.get_stage(ApiId=api_id, StageName=stage_name)
            actual_settings = result.get("AccessLogSettings", {})
            assert actual_settings.get("DestinationArn") == new_log_settings["DestinationArn"], (
                f"UpdateStage dropped AccessLogSettings: expected DestinationArn "
                f"{new_log_settings['DestinationArn']!r}, got {actual_settings!r}"
            )
        finally:
            try:
                apigw.delete_stage(ApiId=api_id, StageName=stage_name)
                apigw.delete_api(ApiId=api_id)
            except Exception:
                pass  # best-effort cleanup


# ---------------------------------------------------------------------------
# Cognito - CreateUserPoolClient (create-time field drop)
# ---------------------------------------------------------------------------


class TestCognitoCreateUserPoolClientFieldPersistence:
    """Cognito CreateUserPoolClient used to drop LogoutURLs, DefaultRedirectURI,
    ReadAttributes, WriteAttributes, SupportedIdentityProviders,
    AllowedOAuthFlowsUserPoolClient, TokenValidityUnits, AccessTokenValidity,
    IdTokenValidity, RefreshTokenValidity.
    """

    def test_create_user_pool_client_logout_urls(self, make_boto_client):
        """CreateUserPoolClient should persist LogoutURLs."""
        suffix = _suffix()
        cognito = make_boto_client("cognito-idp")

        pool_name = f"test-pool-{suffix}"
        client_name = f"test-client-{suffix}"

        # Create user pool
        pool = cognito.create_user_pool(PoolName=pool_name)
        pool_id = pool["UserPool"]["Id"]

        try:
            # Create client with LogoutURLs
            expected_logout_urls = ["https://example.com/logout", "https://example.com/signout"]
            client = cognito.create_user_pool_client(
                UserPoolId=pool_id,
                ClientName=client_name,
                LogoutURLs=expected_logout_urls,
            )

            # Verify LogoutURLs persisted
            actual_logout_urls = client["UserPoolClient"].get("LogoutURLs", [])
            assert set(actual_logout_urls) == set(expected_logout_urls), (
                f"CreateUserPoolClient dropped LogoutURLs: "
                f"expected {expected_logout_urls!r}, got {actual_logout_urls!r}"
            )

            # Also verify via describe
            described = cognito.describe_user_pool_client(
                UserPoolId=pool_id,
                ClientId=client["UserPoolClient"]["ClientId"],
            )
            actual_logout_urls = described["UserPoolClient"].get("LogoutURLs", [])
            assert set(actual_logout_urls) == set(expected_logout_urls), (
                f"DescribeUserPoolClient shows dropped LogoutURLs: "
                f"expected {expected_logout_urls!r}, got {actual_logout_urls!r}"
            )
        finally:
            try:
                cognito.delete_user_pool(UserPoolId=pool_id)
            except Exception:
                pass  # best-effort cleanup

    def test_create_user_pool_client_default_redirect_uri(self, make_boto_client):
        """CreateUserPoolClient should persist DefaultRedirectURI."""
        suffix = _suffix()
        cognito = make_boto_client("cognito-idp")

        pool_name = f"test-pool-{suffix}"
        client_name = f"test-client-{suffix}"

        # Create user pool
        pool = cognito.create_user_pool(PoolName=pool_name)
        pool_id = pool["UserPool"]["Id"]

        try:
            # Create client with DefaultRedirectURI
            expected_redirect_uri = "https://example.com/callback"
            client = cognito.create_user_pool_client(
                UserPoolId=pool_id,
                ClientName=client_name,
                DefaultRedirectURI=expected_redirect_uri,
            )

            # Verify DefaultRedirectURI persisted
            actual_redirect_uri = client["UserPoolClient"].get("DefaultRedirectURI", "")
            assert actual_redirect_uri == expected_redirect_uri, (
                f"CreateUserPoolClient dropped DefaultRedirectURI: "
                f"expected {expected_redirect_uri!r}, got {actual_redirect_uri!r}"
            )
        finally:
            try:
                cognito.delete_user_pool(UserPoolId=pool_id)
            except Exception:
                pass  # best-effort cleanup

    def test_create_user_pool_client_read_write_attributes(self, make_boto_client):
        """CreateUserPoolClient should persist ReadAttributes and WriteAttributes."""
        suffix = _suffix()
        cognito = make_boto_client("cognito-idp")

        pool_name = f"test-pool-{suffix}"
        client_name = f"test-client-{suffix}"

        # Create user pool
        pool = cognito.create_user_pool(PoolName=pool_name)
        pool_id = pool["UserPool"]["Id"]

        try:
            # Create client with ReadAttributes and WriteAttributes
            expected_read_attrs = ["email", "name", "phone_number"]
            expected_write_attrs = ["email", "phone_number"]
            client = cognito.create_user_pool_client(
                UserPoolId=pool_id,
                ClientName=client_name,
                ReadAttributes=expected_read_attrs,
                WriteAttributes=expected_write_attrs,
            )

            # Verify ReadAttributes persisted
            actual_read_attrs = client["UserPoolClient"].get("ReadAttributes", [])
            assert set(actual_read_attrs) == set(expected_read_attrs), (
                f"CreateUserPoolClient dropped ReadAttributes: "
                f"expected {expected_read_attrs!r}, got {actual_read_attrs!r}"
            )

            # Verify WriteAttributes persisted
            actual_write_attrs = client["UserPoolClient"].get("WriteAttributes", [])
            assert set(actual_write_attrs) == set(expected_write_attrs), (
                f"CreateUserPoolClient dropped WriteAttributes: "
                f"expected {expected_write_attrs!r}, got {actual_write_attrs!r}"
            )
        finally:
            try:
                cognito.delete_user_pool(UserPoolId=pool_id)
            except Exception:
                pass  # best-effort cleanup

    def test_create_user_pool_client_supported_identity_providers(self, make_boto_client):
        """CreateUserPoolClient should persist SupportedIdentityProviders."""
        suffix = _suffix()
        cognito = make_boto_client("cognito-idp")

        pool_name = f"test-pool-{suffix}"
        client_name = f"test-client-{suffix}"

        # Create user pool
        pool = cognito.create_user_pool(PoolName=pool_name)
        pool_id = pool["UserPool"]["Id"]

        try:
            # Create client with SupportedIdentityProviders
            expected_providers = ["COGNITO", "Google"]
            client = cognito.create_user_pool_client(
                UserPoolId=pool_id,
                ClientName=client_name,
                SupportedIdentityProviders=expected_providers,
            )

            # Verify SupportedIdentityProviders persisted
            actual_providers = client["UserPoolClient"].get("SupportedIdentityProviders", [])
            assert set(actual_providers) == set(expected_providers), (
                f"CreateUserPoolClient dropped SupportedIdentityProviders: "
                f"expected {expected_providers!r}, got {actual_providers!r}"
            )
        finally:
            try:
                cognito.delete_user_pool(UserPoolId=pool_id)
            except Exception:
                pass  # best-effort cleanup

    def test_create_user_pool_client_token_validity_units(self, make_boto_client):
        """CreateUserPoolClient should persist TokenValidityUnits."""
        suffix = _suffix()
        cognito = make_boto_client("cognito-idp")

        pool_name = f"test-pool-{suffix}"
        client_name = f"test-client-{suffix}"

        # Create user pool
        pool = cognito.create_user_pool(PoolName=pool_name)
        pool_id = pool["UserPool"]["Id"]

        try:
            # Create client with TokenValidityUnits
            expected_units = {
                "AccessToken": "hours",
                "IdToken": "hours",
                "RefreshToken": "days",
            }
            client = cognito.create_user_pool_client(
                UserPoolId=pool_id,
                ClientName=client_name,
                TokenValidityUnits=expected_units,
            )

            # Verify TokenValidityUnits persisted
            actual_units = client["UserPoolClient"].get("TokenValidityUnits", {})
            assert actual_units == expected_units, (
                f"CreateUserPoolClient dropped TokenValidityUnits: "
                f"expected {expected_units!r}, got {actual_units!r}"
            )
        finally:
            try:
                cognito.delete_user_pool(UserPoolId=pool_id)
            except Exception:
                pass  # best-effort cleanup

    def test_create_user_pool_client_token_validity_values(self, make_boto_client):
        """CreateUserPoolClient should persist AccessTokenValidity, IdTokenValidity,
        RefreshTokenValidity.
        """
        suffix = _suffix()
        cognito = make_boto_client("cognito-idp")

        pool_name = f"test-pool-{suffix}"
        client_name = f"test-client-{suffix}"

        # Create user pool
        pool = cognito.create_user_pool(PoolName=pool_name)
        pool_id = pool["UserPool"]["Id"]

        try:
            # Create client with token validity values
            client = cognito.create_user_pool_client(
                UserPoolId=pool_id,
                ClientName=client_name,
                AccessTokenValidity=2,
                IdTokenValidity=2,
                RefreshTokenValidity=30,
            )

            # Verify AccessTokenValidity persisted
            actual_access = client["UserPoolClient"].get("AccessTokenValidity", 0)
            assert actual_access == 2, (
                f"CreateUserPoolClient dropped AccessTokenValidity: "
                f"expected 2, got {actual_access!r}"
            )

            # Verify IdTokenValidity persisted
            actual_id = client["UserPoolClient"].get("IdTokenValidity", 0)
            assert actual_id == 2, (
                f"CreateUserPoolClient dropped IdTokenValidity: expected 2, got {actual_id!r}"
            )

            # Verify RefreshTokenValidity persisted
            actual_refresh = client["UserPoolClient"].get("RefreshTokenValidity", 0)
            assert actual_refresh == 30, (
                f"CreateUserPoolClient dropped RefreshTokenValidity: "
                f"expected 30, got {actual_refresh!r}"
            )
        finally:
            try:
                cognito.delete_user_pool(UserPoolId=pool_id)
            except Exception:
                pass  # best-effort cleanup

    def test_create_user_pool_client_allowed_oauth_flows_user_pool_client(self, make_boto_client):
        """CreateUserPoolClient should persist AllowedOAuthFlowsUserPoolClient."""
        suffix = _suffix()
        cognito = make_boto_client("cognito-idp")

        pool_name = f"test-pool-{suffix}"
        client_name = f"test-client-{suffix}"

        # Create user pool
        pool = cognito.create_user_pool(PoolName=pool_name)
        pool_id = pool["UserPool"]["Id"]

        try:
            # Create client with AllowedOAuthFlowsUserPoolClient=True
            client = cognito.create_user_pool_client(
                UserPoolId=pool_id,
                ClientName=client_name,
                AllowedOAuthFlowsUserPoolClient=True,
            )

            # Verify AllowedOAuthFlowsUserPoolClient persisted
            actual_value = client["UserPoolClient"].get("AllowedOAuthFlowsUserPoolClient", False)
            assert actual_value is True, (
                f"CreateUserPoolClient dropped AllowedOAuthFlowsUserPoolClient: "
                f"expected True, got {actual_value!r}"
            )
        finally:
            try:
                cognito.delete_user_pool(UserPoolId=pool_id)
            except Exception:
                pass  # best-effort cleanup
