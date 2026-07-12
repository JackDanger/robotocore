---
session: 20260711
slug: dynamodb-replica-stream-config
type: fix
---

## Context

Part of a broad Bedrock-agent sweep for silent-correctness bugs across robotocore's native providers (same bug class as the CloudWatch/Scheduler/AppSync/API Gateway v2 fixes in the same session).

## Root cause

`create_replica_table` (DynamoDB Global Tables replication) hardcoded `streams=None` when creating the replica table's backing table, regardless of whether the source table had a stream enabled. A global table replica would silently never get a DynamoDB Stream, even when the source table's `StreamSpecification` had `StreamEnabled: true` — no error, just a replica that can't be used with anything depending on streams (Lambda triggers, cross-region replication tooling, etc).

## Fix

Read the source table's `stream_specification` when `latest_stream_label` indicates a stream is active, and pass it through to `target_backend.create_table(..., streams=...)` instead of the hardcoded `None`.

## Verification

- 2 new unit tests (`TestGlobalTableReplicaStreamConfig`): replica inherits stream config when source has one; replica gets `streams=None` when source has none (regression guard for the correct no-stream case).
- Full local suite: 8739 unit passed (2 new). `ruff`/`mypy` clean on changed files.
