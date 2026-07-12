---
session: "support-bug-fix"
timestamp: "2026-07-11T22:30:00Z"
slug: "support-region-bug"
type: "bug-fix"
---

## Context

Found a correctness bug in the Support native provider where `DescribeCommunications`
was using a hardcoded region "us-east-1" instead of the region parameter when
accessing the Moto backend.

## Root cause

In `_describe_communications`, the code was hardcoding "us-east-1" when looking up
cases in the Moto backend:

```python
backend = get_backend("support")[account_id]["us-east-1"]  # BUG: hardcoded region
```

This meant that if a case was created in a different region (e.g., "eu-west-1"),
the initial case communication from Moto would not be found.

## Fix

Changed the hardcoded "us-east-1" to use the `region` parameter:

```python
backend = get_backend("support")[account_id][region]  # FIXED: uses region parameter
```

## Verification

Added regression test in `tests/unit/services/test_support_bugs.py` that verifies:
1. When a case exists in Moto's eu-west-1 backend
2. And we call DescribeCommunications with region=eu-west-1
3. The initial case communication from Moto is included in the response

All support tests pass (7 tests).
