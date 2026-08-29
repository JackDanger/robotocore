# Fidelity Parity System

The single source of truth for "how close is the Rust port to the Python/moto
server?" and "what should I work on next?"

## Run it (full)
    python3 scripts/harness/parity.py
Runs the 18 native + 4 bridge compat suites against BOTH servers, writes
state, prints the fidelity map + the next work item. Takes ~8 min.

## Quick check (periodic, <1s)
    python3 scripts/harness/parity.py --next
Reads the last state and prints the next work item. Use this to re-orient
without re-running the full suite.

## How to read the map
- `fid%` = rust_pass / py_pass. 100% = full fidelity for that service.
- `gap` = tests that pass on Python but fail on Rust = the actual fidelity loss.
- `bridge 0%` = the moto sidecar isn't routing that service (infra issue, not a
  porting task).

## State
.parity/state.json  — last full run (services, fidelity, next_work)
.parity/xml/        — raw JUnit XML per service/endpoint

## The loop
1. Run full parity -> read NEXT WORK.
2. Do that work (fix a crate, or fix the bridge).
3. Re-run -> confirm the gap shrank.
4. Repeat.
