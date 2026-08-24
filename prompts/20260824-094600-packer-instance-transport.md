---
session: "packer-instance-transport"
timestamp: "2026-08-24T09:46:00Z"
model: claude-opus-4-6
---

## Human

Implement Feature 4 from FEATURE_REQUESTS.md: a Packer-compatible virtual instance transport.

Packer's Amazon (`amazon-ebs`) builder against Robotocore today only exercises HCL/API validation — it never actually provisions a real instance, so file uploads, provisioner ordering, SSM/SSH reachability, cleanup timing, and the resulting AMI's file/service state are all untested. Two real failure modes motivate this: an upload whose destination was a file instead of a directory, and a stall caused by clearing machine identity too early.

Scope — implement:
- A documented SSH or SSM transport that Packer's `amazon-ebs` builder can actually connect through, backed by an opt-in local/container-backed EC2 instance (not just control-plane state).
- Provisioner support sufficient for Packer's file/shell provisioners to run for real: file upload (validate destination path semantics — reject/handle a file-vs-directory destination the way real SSH/SFTP would, since that's a documented real failure mode), and shell provisioner execution in order.
- Persist whatever files/services a provisioner run created into the registered AMI model, so a later `RunInstances` from that AMI reflects them.
- Verify (via a test you write) that an instance launched from the resulting AMI has the expected provisioner-created files and does NOT carry over the source instance's identity (instance-id, hostname, machine-id — whatever "identity" means for this guest model) — this directly covers the "identity cleared too early / too late" failure mode from the acceptance criteria.
- Explicitly do NOT attempt GPU or driver fidelity — document in code/comments and in your report that target-hardware acceptance (GPU, NVIDIA drivers) remains a real-AWS test out of scope here.

## Assistant

## Key decisions

**No existing guest/user-data executor**: Checked `git log main` and the ec2 services directory — Feature 3 (EC2 guest/user-data executor) has not landed on main. Built minimal instance-provisioning primitive from scratch.

**Design approach**:
- Created `src/robotocore/services/ec2/packer/` module with two main components:
  1. `instance_transport.py`: Container-backed EC2 instance with SSH/SSM transport
  2. `ami_builder.py`: AMI creation with identity clearing and state persistence
- Opt-in via `ROBOTOCORE_PACKER_TRANSPORT=1` environment variable
- Uses Docker containers to simulate EC2 instances (similar to Lambda docker executor pattern)
- Filesystem state persisted to `/opt/ami-state/` directory

**Identity clearing**: Implemented comprehensive identity clearing before AMI creation:
- `/etc/machine-id` and `/var/lib/dbus/machine-id`
- Hostname reset to `localhost`
- SSH host keys removed (regenerated on boot)
- Cloud-init logs and instance references cleared

**File upload semantics**: Implemented proper destination path handling:
- Directory destinations (ending with `/`) append source filename
- Existing directories in container get filename appended
- File destinations overwrite existing files
- This addresses the documented real failure mode

**Out of scope documented**: Added explicit comments and docstrings noting that GPU/driver fidelity is not implemented and remains a real-AWS test.

## Files changed

- `src/robotocore/services/ec2/packer/__init__.py` - Module exports
- `src/robotocore/services/ec2/packer/instance_transport.py` - Container-backed instance transport
- `src/robotocore/services/ec2/packer/ami_builder.py` - AMI creation with identity clearing
- `src/robotocore/services/ec2/provider.py` - Integrated CreateImage handler
- `tests/integration/test_packer_transport.py` - Comprehensive integration tests
- `CHANGELOG.md` - Added entry for 2026.8.24

## Testing

Created comprehensive integration tests covering:
- Instance transport lifecycle (start/stop)
- File upload with file destination
- File upload with directory destination (the real failure mode)
- File upload rejection of file-as-directory
- Shell command execution
- Shell with environment variables
- Shell with working directory
- Provisioner execution order
- AMI creation from instance
- AMI identity clearing
- AMI filesystem persistence
- Launch from AMI with fresh identity (key test)
- AMI list and delete operations
- End-to-end Packer scenarios (file provisioner, shell provisioner, complete build)

## Verification status

- Real Packer binary not exercised in tests — tests verify the underlying transport/provisioning primitives directly
- Tests require Docker and `ROBOTOCORE_PACKER_TRANSPORT=1` to run
- All new code follows project patterns (similar to Lambda docker executor)
