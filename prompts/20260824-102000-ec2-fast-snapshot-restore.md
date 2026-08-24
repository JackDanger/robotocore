---
session: "ebs-fsr-001"
timestamp: "2026-08-24T10:20:00Z"
model: claude-opus-4-6
---

## Human

Implement Feature 2 from FEATURE_REQUESTS.md: modern EBS snapshot restoration (Fast Snapshot Restore + volume initialization / lazy hydration modeling).

Robotocore's EC2 implementation currently returns an empty stub for `DescribeFastSnapshotRestores` and has no model of lazy snapshot hydration or `VolumeInitializationRate`. Find the current EC2 code first: `src/robotocore/services/ec2/` (check for existing snapshot/volume provider files) and `vendor/moto/moto/ec2/` for how moto models snapshots/volumes today, per CLAUDE.md's "Reference Materials" section — read vendor source before implementing, and prefer extending moto's models over reimplementing from scratch (CLAUDE.md "Fixing gaps in Moto" — if the right fix belongs in the vendored moto fork, implement it there, push to the fork per that section's steps, then `uv lock`; if it's fidelity moto's design can't accommodate, implement a native provider in `src/robotocore/services/ec2/` instead).

Scope — implement:
- `EnableFastSnapshotRestores`, `DisableFastSnapshotRestores`, `DescribeFastSnapshotRestores`: track FSR state per (snapshot-id, availability-zone) pair, with the real AWS state machine (`enabling` → `optimizing` → `enabled`; `disabling` → `disabled`), returned in AWS's actual response shape (`SuccessfulItems`/`UnsuccessfulItems` etc.)
- `VolumeInitializationRate` on `CreateVolume` (when creating from a snapshot): validate it's only accepted together with a `SnapshotId`, validate it's within AWS's allowed numeric range, and reject with the correct AWS-shaped error otherwise.
- Volume-from-snapshot creation must preserve: source snapshot id, encryption/KMS key, size, and a synthetic "filesystem-tested" readiness marker consistent with an already-hydrated vs cold volume.
- A deterministic hydration profile: expose (via chaos/audit interfaces, not real byte allocation) three distinguishable volume states — cold (lazy-loaded, would be slow on first read in real AWS), initialized (fully hydrated), and FSR-backed (instant-ready because FSR was enabled for that snapshot/AZ). Tests must be able to assert which state a given volume is in without allocating real bytes — model this as state/metadata, not actual disk I/O timing.
- Wire state transitions and any injected failures through the existing chaos-engineering (`POST /_robotocore/chaos/rules`) and audit-log (`GET /_robotocore/audit`) interfaces the same way other EC2 state transitions already do.

## Assistant

## Key decisions

**Implementation approach**: Implemented as a native provider in `src/robotocore/services/ec2/provider.py` rather than modifying Moto. This is because:
1. The FSR feature requires state tracking per (snapshot-id, AZ) pair with a state machine
2. Volume hydration state needs to be modeled and exposed for testing
3. Chaos/audit integration requires native provider hooks

**State management**: Used in-memory dictionaries with threading locks for:
- `_fsr_store`: {(account_id, region, snapshot_id, az) -> fsr_state}
- `_volume_hydration`: {(account_id, region, volume_id) -> hydration_state}

**State machine**: Implemented the full AWS state machine:
- `enabling` → `optimizing` → `enabled` (for enable operations)
- `disabling` → `disabled` (for disable operations)

**Volume hydration states**:
- `cold`: Default for volumes from snapshots without FSR or init rate
- `initialized`: Volumes created with VolumeInitializationRate or non-snapshot volumes
- `fsr-backed`: Volumes from FSR-enabled snapshots

**XML element names**: Used PascalCase for all XML elements to match AWS API format (e.g., `<SnapshotId>`, `<AvailabilityZone>`, `<FastSnapshotRestoreStateErrors>`).

**Chaos/audit integration**: Added `_check_chaos_injection()` and `_record_audit()` calls in the request handler to wire into existing infrastructure.

**Testing**: Created 15 comprehensive tests covering:
- FSR enable/disable/describe operations
- Invalid snapshot handling
- Volume initialization rate validation
- Volume hydration states
- Snapshot property preservation (encryption, size)

## Files changed

- `src/robotocore/services/ec2/provider.py` - Added FSR and volume hydration implementation
- `tests/compatibility/test_ec2_fsr_hydration.py` - New test file with 15 tests
- `CHANGELOG.md` - Added entry for 2026.8.24

## Verification

All verification commands passed:
- `uv run ruff check src/robotocore/services/ec2/ tests/` - All checks passed
- `uv run ruff format --check src/robotocore/services/ec2/` - Already formatted
- `uv run mypy src/robotocore/services/ec2/ --ignore-missing-imports` - Success
- `uv run python scripts/fix_empty_except.py --check` - No issues in new code
- `uv run python scripts/validate_test_quality.py --file tests/compatibility/test_ec2_fsr_hydration.py` - 100% effective test rate
- `uv run pytest tests/compatibility/test_ec2_fsr_hydration.py -v` - 15/15 tests passed
