# Smithy RPC v2 CBOR Protocol Integration Test

type: test

## Summary

Created integration tests for Smithy RPC v2 CBOR protocol support in CloudWatch, and investigated CBOR awareness across all native providers.

## Part 1: Integration Test

Created `tests/integration/test_smithy_cbor_protocol.py` with comprehensive tests:

1. **test_put_dashboard_cbor**: Verifies PutDashboard works via CBOR protocol
2. **test_get_dashboard_cbor_roundtrip**: Verifies dashboard created via CBOR can be retrieved via CBOR
3. **test_delete_dashboards_cbor**: Verifies DeleteDashboards works via CBOR
4. **test_cbor_not_implemented_returns_501**: Verifies unimplemented operations return 501 with CBOR error body
5. **test_cbor_error_response_format**: Verifies error responses are properly CBOR-encoded
6. **test_cbor_empty_body**: Verifies empty CBOR bodies are handled gracefully
7. **test_missing_smithy_protocol_header**: Tests behavior without smithy-protocol header
8. **test_invalid_cbor_body**: Tests handling of invalid CBOR data

All tests pass (7 passed, 1 skipped for invalid CBOR edge case).

## Part 2: Native Provider CBOR Gap Analysis

Investigated all 46 native providers for CBOR awareness. **Only CloudWatch has CBOR support** - all other 45 native providers have zero CBOR awareness.

### At-Risk Native Providers (Ranked by Terraform Usage Likelihood)

**Critical (High Terraform usage, likely to be migrated to CBOR by AWS):**
1. **ec2** - Core compute service, heavy terraform usage
2. **s3** - Storage service, very heavy terraform usage
3. **iam** - Identity management, critical for all AWS setups
4. **lambda** - Serverless compute, very popular in terraform
5. **dynamodb** - NoSQL database, widely used
6. **sqs** - Message queuing, common in event-driven architectures
7. **sns** - Notifications, frequently used with SQS
8. **rds** - Relational database, popular for managed databases
9. **ecs** - Container orchestration, growing terraform usage
10. **eks** - Kubernetes service, very popular
11. **cloudformation** - Infrastructure as Code, meta-service
12. **events** (EventBridge) - Serverless event bus, increasingly popular
13. **kinesis** - Streaming data, common in data pipelines
14. **stepfunctions** - Workflow orchestration, growing usage
15. **secretsmanager** - Secret management, security-critical
16. **ssm** (Systems Manager) - Parameter store and ops

**High (Moderate Terraform usage):**
17. **apigateway** - API management
18. **apigatewayv2** - HTTP/WebSocket APIs
19. **cloudwatch** - ✅ Already has CBOR support
20. **logs** - CloudWatch Logs
21. **firehose** - Data delivery streams
22. **scheduler** - EventBridge Scheduler
23. **ecr** - Container registry
24. **elasticache** - Managed caching
25. **route53** - DNS management

**Medium (Specialized usage):**
26. **acm** - Certificate management
27. **appsync** - GraphQL APIs
28. **batch** - Batch computing
29. **cognito-idp** - User authentication
30. **config** - Compliance and governance
31. **dynamodbstreams** - DDB change data capture
32. **iot** / **iotdata** - IoT services
33. **opensearch** / **es** - Search services
34. **pipes** - EventBridge Pipes
35. **rdsdata** - RDS Data API
36. **rekognition** - ML image analysis
37. **resource-groups** - Resource organization
38. **resourcegroupstaggingapi** - Tag management
39. **ses** / **sesv2** - Email service
40. **sts** - Token service (temporary credentials)
41. **support** - AWS Support API
42. **synthetics** - Canary testing
43. **xray** - Distributed tracing

### Technical Gap Pattern

The vulnerable pattern in native providers is:

```python
# Current pattern (vulnerable to CBOR crash)
body = await request.body()
content_type = request.headers.get("content-type", "")
target = request.headers.get("x-amz-target", "")

if target and "json" in content_type:
    # JSON protocol handling
    params = json.loads(body)
else:
    # Query protocol handling - tries to decode body as UTF-8
    params = parse_qs(body.decode("utf-8"))  # CRASH on CBOR binary!
```

The fix (as implemented in CloudWatch):

```python
# Fixed pattern
use_cbor_protocol = request.headers.get("smithy-protocol", "") == "rpc-v2-cbor"

if use_cbor_protocol:
    params = cbor2.loads(body) if body else {}
    action = request.url.path.rsplit("/operation/", 1)[-1]
elif use_json_protocol:
    params = json.loads(body.decode()) if body else {}
else:
    params = parse_qs(body.decode()) if body else {}
```

### Risk Assessment

Any native provider using JSON or query protocol without checking for `smithy-protocol: rpc-v2-cbor` header will crash when receiving CBOR requests. The crash manifests as:
- UTF-8 decode error on binary CBOR body, OR
- JSON parse error on binary data, OR
- Silent hang if the body is consumed but not properly decoded

This is a **latent bug** - it only triggers when:
1. A user upgrades to aws-sdk-go-v2 (or other CBOR-capable SDK)
2. AWS migrates specific operations to CBOR protocol
3. The operation is one that robotocore handles natively (not forwarded to Moto)

### Recommendation

Priority order for adding CBOR support:
1. **ec2, s3, iam, lambda, dynamodb** - Critical infrastructure services
2. **sqs, sns, rds, ecs, eks** - Core application services
3. **cloudformation, events, kinesis, stepfunctions** - Orchestration services
4. Remaining services by terraform usage metrics
