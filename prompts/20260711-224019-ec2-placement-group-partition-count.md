---
session: 20260711
slug: ec2-placement-group-partition-count
type: fix
---

## Context

Part of a broad Bedrock-agent sweep for silent-correctness bugs across robotocore's native providers.

## Root cause

`_create_placement_group` and `_describe_placement_groups` in `src/robotocore/services/ec2/provider.py` stored `partitionCount` on the placement group record but never rendered it into either XML response. A `partition`-strategy placement group's partition count was silently invisible to callers.

## Fix

Both handlers now emit `<partitionCount>` when the group's strategy has a partition count set.

## Verification

- 4 new unit tests (`TestPlacementGroupPartitionCountInResponse` in `tests/unit/services/test_ec2_bugs.py`).
- Full local suite: 8741 unit passed (4 new). `ruff`/`ruff format`/`mypy` clean.

## Note

The same Bedrock (Kimi K2.5) agent run also proposed ECS and EKS changes that were
reviewed and discarded: the ECS `ListTasks` serviceName fix replaced dead logic with a
check against a `task["group"]` field that no code path in the provider ever actually
sets, so the filter would still always return an empty list — not a real fix, just a
different way to be broken. The EKS change (`_CLUSTER_RE` from `[^/]+` to `.+`) broke
nodegroup routing entirely, since `_CLUSTER_RE` is matched before
`_NODEGROUPS_COLLECTION_RE`/`_NODEGROUP_RE` and the greedy pattern swallows
`/clusters/{name}/node-groups...` paths too — this was the actual cause of the agent
run's own "proof did not reproduce" full-suite failure.
