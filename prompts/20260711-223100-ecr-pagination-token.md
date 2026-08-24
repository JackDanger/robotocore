---
session: "ecr-pagination-token"
slug: "ecr-pagination-token-bug"
type: "bugfix"
---

## Context

While reviewing the ECR native provider, I found that the `DescribeRepositories` pagination handling had a bug where the `nextToken` was hardcoded to the string `"pagination-token"` instead of encoding the actual position in the list. This broke pagination because clients couldn't resume from the correct position.

## Root Cause

In `handle_ecr_request()`, when truncating results for pagination, the code set:
```python
resp_body["nextToken"] = "pagination-token"
```

This hardcoded token doesn't encode any position information, so subsequent requests with this token would return the first page again.

## Fix

Modified `src/robotocore/services/ecr/provider.py`:

1. Changed the pagination logic to use the offset (start position) as the token value.

2. Added handling for the `nextToken` parameter in requests - when provided, it's parsed as an integer offset and used to skip the first N repositories.

3. The new `nextToken` value is set to `str(start_idx + max_results)` to indicate the next position.

4. When there are no more results, the `nextToken` is removed from the response.

## Verification

Added tests in `tests/unit/services/test_ecr_bugs_new.py`:
- `test_next_token_encodes_position`: Verifies that the token encodes the position (e.g., "5" after returning first 5 items)
- `test_next_token_is_used_for_offset`: Verifies that providing a token correctly resumes from that position

All tests pass, and the existing ECR test suite (4 tests) continues to pass.
