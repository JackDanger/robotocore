---
role: assistant
model: claude-opus-4-6
timestamp: "2026-03-07T20:16:00Z"
session: "b9051329"
sequence: 2
tools: [subagent]
---

## Key decisions

**Batch registration approach**: Built a script that reads botocore `service-2.json` metadata (protocol, targetPrefix, signingName) and cross-references with Moto backend directories. The script handles all the edge cases:
- Moto dir names that differ from botocore names (e.g., `awslambda` -> `lambda`, `cognitoidp` -> `cognito-idp`)
- Ambiguous signing names where multiple services share one (e.g., `elasticloadbalancing` -> both `elb` and `elbv2`)
- Services already registered under different names (e.g., `awslambda` already registered as `lambda`)
- Compressed `.json.gz` botocore service files

**Routing strategy**: JSON-protocol services use `TARGET_PREFIX_MAP` (44 new entries). Services where the signing name differs from the Moto dir name use `SERVICE_NAME_ALIASES` (22 new entries). REST-json/REST-xml services with unique signing names need no special routing — the auth header extraction already maps correctly.

**Parallel test execution**: Used `pytest-xdist` with `--dist=loadfile` for compat tests — tests within the same file run serially (avoiding shared-state collisions from bucket/resource name reuse) but different files run in parallel across 8 cores. Unit tests use default `--dist=load` across 12 cores.

**Infrastructure modules**: Kept each feature in its own module (`chaos/`, `resources/`, `audit/`) with minimal coupling. Chaos engineering hooks into the handler chain between `populate_context_handler` and `iam_enforcement_handler` so faults fire before IAM checks. Audit logging is a response handler so it captures the final status code.

**What I skipped**: Operation-level routing for services that share both signing name AND target prefix (timestream-query vs timestream-write). These need the same approach as DynamoDB vs DynamoDB Streams — inspect the operation name. Deferred to a follow-up since both backends work independently via their Moto names.
