#!/usr/bin/env python3
"""Differential test: send the same boto3 calls to Python (:4566) and Rust (:4567), diff results.

Usage:
    diff_test.py --services s3,sqs,sts,dynamodb [--endpoint-python URL] [--endpoint-rust URL]
    diff_test.py --service s3 --ops CreateBucket,PutObject,GetObject

For each operation, sends a real boto3 call to both servers, normalizes volatile
fields, and reports PASS/FAIL with the first difference.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import time
import uuid
import boto3
from botocore.config import Config
from botocore.exceptions import ClientError


# Volatile fields to strip before comparison
VOLATILE_FIELDS = {
    "RequestId",
    "x-amz-request-id",
    "x-amzn-RequestId",
    "x-amz-id-2",
    "ETag",
    "LastModified",
    "Expiration",
    "HostId",
    "ChecksumCRC32",
    "ChecksumCRC32Truncated",
    "ChecksumMode",
    "ChecksumAlgorithm",
    "Metadata",
    "ServerSideEncryption",
    "RequestCharged",
    "Restore",
    "ContentRange",
    "AcceptRanges",
    "x-amz-version-id",
    "VersionId",
    "SentTimestamp",
    "ApproximateFirstReceiveTimestamp",
    "ApproximateReceiveCount",
    "ReceiptHandle",
    "MessageId",
    "MD5OfBody",
    "MD5OfMessageBody",
    "CreationDate",
    "TableArn",
    "CreationRequestTime",
    "ItemCount",
    "TableSizeBytes",
    "ProvisionedThroughput",
    "TableName",
    "LocationConstraint",
    "Policy",
    "TableArn",
    "ConsumedCapacity",
    "ScannedCount",
    "Count",
    "CapacityUnits",
}

VOLATILE_PATTERNS = [
    r"^arn:aws:",
    r"^https?://",
]


def normalize(obj):
    """Recursively strip volatile fields from a response dict."""
    if isinstance(obj, dict):
        return {
            k: normalize(v)
            for k, v in obj.items()
            if k not in VOLATILE_FIELDS
            and k != "ResponseMetadata"
            and not any(re.match(p, str(v)) for p in VOLATILE_PATTERNS if isinstance(v, str))
        }
    elif isinstance(obj, list):
        return [normalize(item) for item in obj]
    elif isinstance(obj, (int, float)):
        # Normalize timestamps (10+ digit numbers)
        if obj > 1_000_000_000:
            return "<timestamp>"
        return obj
    elif isinstance(obj, str):
        # Normalize UUIDs, ARNs, URLs
        if re.match(r"^[0-9a-f]{8}-[0-9a-f]{4}-", obj):
            return "<uuid>"
        if obj.startswith("arn:aws:"):
            return "<arn>"
        if obj.startswith(("http://", "https://")):
            return "<url>"
        return obj
    return obj


def diff_dicts(a, b, path=""):
    """Return list of differences between two dicts."""
    diffs = []
    if type(a) != type(b):
        diffs.append(f"{path}: type {type(a).__name__} != {type(b).__name__}")
        return diffs
    if isinstance(a, dict):
        for k in set(a.keys()) | set(b.keys()):
            if k not in a:
                diffs.append(f"{path}.{k}: missing in a")
            elif k not in b:
                diffs.append(f"{path}.{k}: missing in b")
            else:
                diffs.extend(diff_dicts(a[k], b[k], f"{path}.{k}"))
    elif isinstance(a, list):
        if len(a) != len(b):
            diffs.append(f"{path}: list length {len(a)} != {len(b)}")
        else:
            for i, (x, y) in enumerate(zip(a, b)):
                diffs.extend(diff_dicts(x, y, f"{path}[{i}]"))
    else:
        if a != b:
            diffs.append(f"{path}: {a!r} != {b!r}")
    return diffs


def make_client(service, endpoint):
    return boto3.client(
        service,
        endpoint_url=endpoint,
        aws_access_key_id="123456789012",
        aws_secret_access_key="test",
        region_name="us-east-1",
        config=Config(signature_version="s3v4" if service == "s3" else "v4"),
    )


def make_resource(service, endpoint):
    return boto3.resource(
        service,
        endpoint_url=endpoint,
        aws_access_key_id="123456789012",
        aws_secret_access_key="test",
        region_name="us-east-1",
    )


# ---- Per-service test definitions ----
# Each test: (name, setup_fn, call_fn, teardown_fn)
# setup/call/teardown take (client, suffix) and return nothing (or a value for the test)

def s3_tests(suffix):
    _test_bucket = {}

    def bucket_for(name):
        if name not in _test_bucket:
            _test_bucket[name] = f"dt-{name}-{suffix}"
        return _test_bucket[name]

    bucket = f"dt-main-{suffix}"
    key = f"hello-{suffix}.txt"

    def setup(c, s):
        import time
        for attempt in range(3):
            try:
                c.create_bucket(Bucket=bucket)
                break
            except Exception as e:
                if attempt < 2:
                    try:
                        c.delete_bucket(Bucket=bucket)
                    except Exception:
                        pass
                    time.sleep(0.5)
                else:
                    raise

    def teardown(c, s):
        try:
            c.delete_object(Bucket=bucket, Key=key)
        except Exception:
            pass
        try:
            c.delete_bucket(Bucket=bucket)
        except Exception:
            pass

    def call_create_bucket(c, s):
        return c.create_bucket(Bucket=f"diff-test-{s}-cb")

    def teardown_create_bucket(c, s):
        try:
            c.delete_bucket(Bucket=f"diff-test-{s}-cb")
        except ClientError:
            pass

    def call_put_object(c, s):
        return c.put_object(Bucket=bucket, Key=key, Body=b"Hello, World!")

    def call_get_object(c, s):
        return c.get_object(Bucket=bucket, Key=key)["Body"].read()

    def call_put_and_get(c, s):
        c.put_object(Bucket=bucket, Key=key, Body=b"Hello, World!")
        return c.get_object(Bucket=bucket, Key=key)["Body"].read()

    def call_head_object(c, s):
        return c.head_object(Bucket=bucket, Key=key)

    def call_list_objects(c, s):
        return c.list_objects_v2(Bucket=bucket)

    def call_delete_object(c, s):
        return c.delete_object(Bucket=bucket, Key=key)

    def call_copy_object(c, s):
        c.put_object(Bucket=bucket, Key=key, Body=b"Hello, World!")
        return c.copy_object(
            Bucket=bucket,
            Key=f"copy-{s}.txt",
            CopySource=f"{bucket}/{key}",
        )

    def call_put_and_head(c, s):
        c.put_object(Bucket=bucket, Key=key, Body=b"Hello, World!")
        return c.head_object(Bucket=bucket, Key=key)

    def call_put_and_delete(c, s):
        c.put_object(Bucket=bucket, Key=key, Body=b"Hello, World!")
        return c.delete_object(Bucket=bucket, Key=key)

    def call_put_and_get_policy(c, s):
        import json
        policy = json.dumps({
            "Version": "2012-10-17",
            "Statement": [{
                "Effect": "Allow",
                "Principal": "*",
                "Action": "s3:GetObject",
                "Resource": f"arn:aws:s3:::{bucket}/*",
            }]
        })
        c.put_bucket_policy(Bucket=bucket, Policy=policy)
        return c.get_bucket_policy(Bucket=bucket)["Policy"]

    def call_get_bucket_location(c, s):
        return c.get_bucket_location(Bucket=bucket)

    def call_put_bucket_policy(c, s):
        policy = json.dumps({
            "Version": "2012-10-17",
            "Statement": [{
                "Effect": "Allow",
                "Principal": "*",
                "Action": "s3:GetObject",
                "Resource": f"arn:aws:s3:::{bucket}/*",
            }]
        })
        return c.put_bucket_policy(Bucket=bucket, Policy=policy)

    def call_get_bucket_policy(c, s):
        return c.get_bucket_policy(Bucket=bucket)["Policy"]

    return [
        ("create_bucket", call_create_bucket, None, teardown_create_bucket),
        ("put_object", call_put_object, setup, teardown),
        ("get_object", call_put_and_get, setup, teardown),
        ("head_object", call_put_and_head, setup, teardown),
        ("list_objects_v2", call_list_objects, setup, teardown),
        ("delete_object", call_put_and_delete, setup, teardown),
        ("copy_object", call_copy_object, setup, teardown),
        ("get_bucket_location", call_get_bucket_location, setup, teardown),
        ("put_bucket_policy", call_put_bucket_policy, setup, teardown),
        ("get_bucket_policy", call_put_and_get_policy, setup, teardown),
    ]


def sqs_tests(suffix):
    queue = f"diff-test-{suffix}"

    def setup(c, s):
        try:
            url = c.get_queue_url(QueueName=queue).get("QueueUrl")
            if url:
                c.delete_queue(QueueUrl=url)
                import time; time.sleep(1)
        except Exception:
            pass
        c.create_queue(QueueName=queue)

    def teardown(c, s):
        url = c.get_queue_url(QueueName=queue).get("QueueUrl")
        if url:
            try:
                c.purge_queue(QueueUrl=url)
            except Exception:
                pass
            c.delete_queue(QueueUrl=url)

    def call_create_queue(c, s):
        return c.create_queue(QueueName=f"diff-test-{s}-cq")

    def teardown_create_queue(c, s):
        try:
            url = c.get_queue_url(QueueName=f"diff-test-{s}-cq").get("QueueUrl")
            if url:
                c.delete_queue(QueueUrl=url)
        except ClientError:
            pass

    def call_send_message(c, s):
        url = c.get_queue_url(QueueName=queue)["QueueUrl"]
        return c.send_message(QueueUrl=url, MessageBody="hello")

    def call_receive_message(c, s):
        url = c.get_queue_url(QueueName=queue)["QueueUrl"]
        c.send_message(QueueUrl=url, MessageBody="test")
        return c.receive_message(QueueUrl=url, MaxNumberOfMessages=1)

    def call_get_queue_attributes(c, s):
        url = c.get_queue_url(QueueName=queue)["QueueUrl"]
        return c.get_queue_attributes(QueueUrl=url, AttributeNames=["All"])

    def call_list_queues(c, s):
        return c.list_queues()

    return [
        ("create_queue", call_create_queue, None, teardown_create_queue),
        ("send_message", call_send_message, setup, teardown),
        ("receive_message", call_receive_message, setup, teardown),
        ("get_queue_attributes", call_get_queue_attributes, setup, teardown),
        ("list_queues", call_list_queues, setup, teardown),
    ]


def sts_tests(suffix):
    def call_get_caller_identity(c, s):
        return c.get_caller_identity()

    def call_get_access_key_info(c, s):
        return c.get_access_key_info()

    return [
        ("get_caller_identity", call_get_caller_identity, None, None),
        ("get_access_key_info", call_get_access_key_info, None, None),
    ]


def dynamodb_tests(suffix):
    table = f"diff-test-{suffix}"

    def setup(c, s):
        try:
            c.delete_table(TableName=table)
            c.get_waiter("table_not_exists").wait(TableName=table)
        except Exception:
            pass
        c.create_table(
            TableName=table,
            KeySchema=[{"AttributeName": "id", "KeyType": "HASH"}],
            AttributeDefinitions=[{"AttributeName": "id", "AttributeType": "S"}],
            BillingMode="PAY_PER_REQUEST",
        )
        c.get_waiter("table_exists").wait(TableName=table)

    def teardown(c, s):
        try:
            c.delete_table(TableName=table)
        except ClientError:
            pass

    def call_create_table(c, s):
        return c.create_table(
            TableName=f"diff-test-{s}-ct",
            KeySchema=[{"AttributeName": "id", "KeyType": "HASH"}],
            AttributeDefinitions=[{"AttributeName": "id", "AttributeType": "S"}],
            BillingMode="PAY_PER_REQUEST",
        )

    def teardown_create_table(c, s):
        try:
            c.delete_table(TableName=f"diff-test-{s}-ct")
        except ClientError:
            pass

    def call_put_item(c, s):
        return c.put_item(
            TableName=table,
            Item={"id": {"S": "u1"}, "name": {"S": "Alice"}},
        )

    def call_get_item(c, s):
        return c.get_item(TableName=table, Key={"id": {"S": "u1"}})

    def call_scan(c, s):
        return c.scan(TableName=table)

    def call_delete_item(c, s):
        return c.delete_item(TableName=table, Key={"id": {"S": "u1"}})

    def call_list_tables(c, s):
        return c.list_tables()

    def call_describe_table(c, s):
        return c.describe_table(TableName=table)

    return [
        ("create_table", call_create_table, None, teardown_create_table),
        ("put_item", call_put_item, setup, teardown),
        ("get_item", call_get_item, None, None),
        ("scan", call_scan, None, None),
        ("delete_item", call_delete_item, None, None),
        ("list_tables", call_list_tables, setup, teardown),
        ("describe_table", call_describe_table, setup, teardown),
    ]


TEST_DEFINITIONS = {
    "s3": s3_tests,
    "sqs": sqs_tests,
    "sts": sts_tests,
    "dynamodb": dynamodb_tests,
}


def run_diff_tests(services, endpoint_python, endpoint_rust, only_ops=None):
    results = {}
    for service in services:
        if service not in TEST_DEFINITIONS:
            print(f"SKIP: no test definitions for {service}")
            continue

        print(f"\n{'='*60}")
        print(f"Service: {service}")
        print(f"{'='*60}")

        client_py = make_client(service, endpoint_python)
        client_rust = make_client(service, endpoint_rust)
        suffix = uuid.uuid4().hex[:12]

        tests = TEST_DEFINITIONS[suffix](suffix) if False else TEST_DEFINITIONS[service](suffix)
        if only_ops:
            tests = [(n, f, s, t) for n, f, s, t in tests if n in only_ops]

        for name, call_fn, setup_fn, teardown_fn in tests:
            py_result = None
            rust_result = None
            py_error = None
            rust_error = None

            # Setup on both
            if setup_fn:
                try:
                    setup_fn(client_py, suffix + "-py")
                except Exception as e:
                    py_error = f"setup: {e}"
                try:
                    setup_fn(client_rust, suffix + "-rs")
                except Exception as e:
                    rust_error = f"setup: {e}"

            # Call on Python
            if not py_error:
                try:
                    py_result = call_fn(client_py, suffix + "-py")
                except ClientError as e:
                    py_error = e.response
                except Exception as e:
                    py_error = str(e)

            # Call on Rust
            if not rust_error:
                try:
                    rust_result = call_fn(client_rust, suffix + "-rs")
                except ClientError as e:
                    rust_error = e.response
                except Exception as e:
                    rust_error = str(e)

            # Teardown on both
            if teardown_fn:
                for c, s in [(client_py, suffix + "-py"), (client_rust, suffix + "-rs")]:
                    try:
                        teardown_fn(c, s)
                    except Exception:
                        pass

            # Compare
            status = "PASS"
            detail = ""
            if py_error and rust_error:
                py_code = py_error.get("Error", {}).get("Code", "") if isinstance(py_error, dict) else str(py_error)
                rust_code = rust_error.get("Error", {}).get("Code", "") if isinstance(rust_error, dict) else str(rust_error)
                if py_code == rust_code:
                    status = "PASS"
                    detail = f"both errored: {py_code}"
                else:
                    status = "FAIL"
                    detail = f"error mismatch: py={py_code} rust={rust_code}"
            elif py_error:
                status = "FAIL"
                detail = f"py error: {py_error}, rust ok: {rust_result}"
            elif rust_error:
                status = "FAIL"
                detail = f"rust error: {rust_error}, py ok: {py_result}"
            else:
                py_norm = normalize(py_result) if py_result is not None else None
                rust_norm = normalize(rust_result) if rust_result is not None else None
                if py_norm == rust_norm:
                    status = "PASS"
                else:
                    status = "FAIL"
                    diffs = diff_dicts(py_norm if isinstance(py_norm, dict) else {"v": py_norm},
                                      rust_norm if isinstance(rust_norm, dict) else {"v": rust_norm})
                    detail = "; ".join(diffs[:3]) if diffs else "unknown diff"

            mark = "✓" if status == "PASS" else "✗"
            print(f"  {mark} {name:30s} {status}  {detail[:80]}")
            results[f"{service}.{name}"] = {"status": status, "detail": detail}

    # Summary
    total = len(results)
    passed = sum(1 for r in results.values() if r["status"] == "PASS")
    failed = total - passed
    print(f"\n{'='*60}")
    print(f"TOTAL: {total}  PASS: {passed}  FAIL: {failed}")
    print(f"{'='*60}")

    if failed > 0:
        print("\nFailures:")
        for name, r in sorted(results.items()):
            if r["status"] == "FAIL":
                print(f"  {name}: {r['detail'][:100]}")

    return results


def main():
    parser = argparse.ArgumentParser(description="Differential test: Python vs Rust robotocore")
    parser.add_argument("--services", default="s3,sqs,sts,dynamodb", help="Comma-separated service list")
    parser.add_argument("--ops", default=None, help="Comma-separated operation filter")
    parser.add_argument("--endpoint-python", default="http://localhost:4566", help="Python server URL")
    parser.add_argument("--endpoint-rust", default="http://localhost:4567", help="Rust server URL")
    parser.add_argument("--json", action="store_true", help="Output JSON results")
    args = parser.parse_args()

    services = [s.strip() for s in args.services.split(",")]
    only_ops = [o.strip() for o in args.ops.split(",")] if args.ops else None

    results = run_diff_tests(services, args.endpoint_python, args.endpoint_rust, only_ops)

    if args.json:
        print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
