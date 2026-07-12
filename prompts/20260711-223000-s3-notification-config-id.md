---
session: "s3-notification-config-id"
slug: "s3-notification-config-id-bug"
type: "bugfix"
---

## Context

While reviewing the S3 native provider, I found that the notification configuration XML parsing was silently ignoring the `<Id>` element in QueueConfiguration, TopicConfiguration, and LambdaFunctionConfiguration. This element is optional in AWS S3 but when provided, it should be preserved and returned in GET responses.

## Root Cause

In `_parse_notification_config_xml()`, the code parsed `Queue`, `Event`, and `Filter` elements but did not handle the `Id` element. Similarly, `_notification_config_to_xml()` did not include the `Id` element in the output.

## Fix

Modified `src/robotocore/services/s3/provider.py`:

1. In `_parse_notification_config_xml()`: Added parsing of the `Id` element for all three configuration types (QueueConfiguration, TopicConfiguration, LambdaFunctionConfiguration).

2. In `_notification_config_to_xml()`: Added serialization of the `Id` element for all three configuration types.

## Verification

Added tests in `tests/unit/services/test_s3_bugs_new.py`:
- `test_queue_configuration_id_is_preserved`
- `test_topic_configuration_id_is_preserved`
- `test_lambda_configuration_id_is_preserved`
- `test_id_is_included_in_xml_output`

All tests pass, and the full S3 test suite (51 tests) continues to pass.
