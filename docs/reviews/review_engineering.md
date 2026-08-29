# Engineering Review — robotocore Rust Port

**Angle:** Engineering practice and risk
**Reviewed:** `Cargo.toml`, `src/bin/robotocore-rust.rs`, `scripts/harness/parity.py`, git history (1,516 commits; 89 Rust-port commits since 2026-08-16), `.github/workflows/`
**Date:** 2026-08-29

## Executive summary

The port has a real measurement loop (parity runner + fidelity %) and a sane
crate-per-service structure, which puts it ahead of most "rewrite" efforts. But
the timeouts and monitors exist **only in the test harness**, not in the server.
The server itself has zero per-request timeouts, no liveness/monitoring beyond a
passive health endpoint, no Rust CI (no `cargo test`/`clippy` in any workflow),
and only ~40 unit tests across 18 crates. The "stubs" pattern is actively
polluting the fidelity metric. None of this is fatal, but all of it is the kind
of thing that becomes expensive to fix after the port is declared "finished".

---

## 1. Are the timeouts and monitors sufficient for production?

**What exists**

| Layer | Timeout / monitor | Verdict |
|---|---|---|
| Parity harness (`parity.py`) | `PER_SUITE_TIMEOUT=300s` per suite/endpoint via `subprocess.run(timeout=...)`; `TOTAL_TIMEOUT=1800s` via `signal.alarm` | Real, but fragile (see below) |
| Bridge probe (`_bridge_ready`) | curl `--max-time 10`/`15` | OK as a probe |
| `MotoProxy` (`proxy.rs`) | `reqwest` client `.timeout(30s)` | **The only production timeout in the entire Rust binary** |
| Server (`server.rs`, `robotocore-rust.rs`) | None. `axum::serve` with no request timeout, no tower timeout layer, no per-operation deadline | Missing |
| Monitors | `GET /_robotocore/health` (passive). No request latency, no in-flight gauge, no panic hook, no watchdog, no deadman switch | Missing |
| Sidecar liveness | `ensure_rust_server` polls sidecar health and restarts it once — inside the harness only | Harness-only |

**What's missing (by priority)**

1. **Per-request timeout in the server.** A hang in any native handler (deadlock
   on `parking_lot::RwLock`, an infinite loop in a parser) now blocks that
   worker forever; the process still answers `/health`, so no monitor will ever
   notice. Add `tower`'s `TimeoutLayer` (or a per-route `tokio::time::timeout`)
   with a configurable ceiling (e.g. 10–30s) returning a 500/503. This is the
   single most important gap given the goal "catch runaway tasks".
2. **Runaway-task detection for spawned tasks.** Nothing is currently spawned
   (`axum::serve` runs in `main`), so `tokio::task::spawn` + `JoinHandle` +
   `tokio::time::timeout` isn't even in the picture yet. But `Lambda invoke`,
   SNS→SQS fanout, and Step Functions will spawn work. Establish the pattern
   *before* those land: every spawned task gets a budget and a timeout that
   logs and abandons, never one that silently leaks.
3. **Liveness vs. liveness-of-work.** The health endpoint is a static JSON
   blob; it can never fail. Production tooling that "catches runaway tasks"
   needs: (a) request count + in-flight count + max latency in `/health`,
   (b) a `panic::set_hook` that records the thread/task (Rust panics in `async`
   context abort the task but nothing is logged beyond a default line), (c)
   optionally a heartbeat/deadman switch for CI and Docker.
4. **Harness timeout implementation bugs:**
   - `signal.alarm` is main-thread-only and unreliable in Python when the
     main thread is inside a C call (`subprocess.run` wait). A long `cargo
     build --release` (up to 600s) or a stuck pytest can swallow it. Use
     `asyncio`/thread-based timeouts or `timeout` in shell.
   - On `TimeoutExpired`, `subprocess.run` kills the child **without killing
     pytest's workers** (no process-group kill: `start_new_session=True` +
     `os.killpg`). A "timed out" suite can leave orphan pytest processes
     holding port state, corrupting the *next* run's results.
   - Partial results on timeout are silently folded into fidelity %, dragging
     the metric down without flagging that the number is garbage. A timed-out
     suite should be marked `status: "timeout"` and excluded from the metric.
   - Hardcoded absolute paths (`/Users/jackdanger/www/robotocore`,
     `RUST_BIN = target/release/robotocore-rust`) make the harness
     un-runnable outside this machine — which means the *only* timeout-protected
     loop currently runs on one laptop. It must run in CI to be a real safety net.
5. **No timeout on `cargo build`** in the harness is fine (600s), but the
   build happens *inside* the 1800s total budget, so a slow CI machine eats 1/3
   of the run budget before tests start.

**Bottom line:** the *measurement* timeouts are a good start but are
harness-local, partially broken, and machine-specific. The *server* — the thing
that users actually point SDKs at — has exactly one timeout (the proxy's 30s)
and no monitors at all. As it stands, a runaway task in a native crate is
invisible and unbounded.

---

## 2. Code-quality trajectory: maintainable, or tech debt?

**Evidence**

- **Rust unit tests: ~40 total across 18 crates** (`#[test]` count = 40).
  `sqs` carries the load (tests.rs, 696 lines); most newer crates
  (ecr, ecs, stepfunctions, cloudwatch) have near-zero.
- **No Rust CI.** `ci.yml`, `nightly.yml`, `parity.yml` all run Python tooling
  (`uv sync`, `ruff`, `mypy`, pytest). There is no `cargo test`, no `cargo
  clippy`, no `cargo fmt --check` anywhere in `.github/workflows/`. The Rust
  code is effectively uncompiled by CI.
- **Stubs as a committed strategy.** `crates/dynamodb/src/handler.rs` has
  `json_empty` / `json_stub` / `json_stub_list` helpers used by 22 operations
  that return `{}` with a 200. This is *plausible but wrong*: a stub returning
  HTTP 200 + empty JSON **passes** compat tests that only check status code,
  and *inflates* fidelity %; operations that should error (`ResourceNotFound`)
  now "work". The DynamoDB revert commit proves the metric is sensitive to
  this: adding stubs dropped fidelity 21.9% → 21.3% (they broke passing tests
  that expected real errors). That's a metric that doesn't agree with itself —
  the exact property you need from your North Star.
- **State churn in git.** 18 of the last 150 commits touch `.parity/`
  (state.json + 21×2 JUnit XMLs, ~1,800-line files committed per run). This is
  harness *output* being versioned as *state*. It bloats history, makes diffs
  unreadable (the DynamoDB revert shows a 2,400-line diff that is 95% XML
  noise), and invites merge conflicts. It belongs in gitignore + CI artifacts.
- **Commit style.** ~89 Rust commits in 14 days ≈ 6–7/day, each mixing
  "add stubs" + "run parity" + "commit state.json". Messages are honest and
  carry metric deltas (good: `36.9%->44.6%`), but the cadence is
  *harness-echoing* rather than *review-graded*: nothing between these commits
  has been reviewed, linted by Rust tooling, or compiled by a second machine.
- **Architecture is sound.** 18 small crates with a uniform
  `handler/protocol/models` split, a central `StateStore` (parking_lot
  RwLocks), a protocol layer, and a spec-driven `gen_crate` tool is a
  maintainable shape. The PyO3 `extension-module` feature and cdylib/rlib
  crate-type hint at a still-open decision about how Python and Rust coexist —
  that ambiguity itself is risk (see §3).

**Verdict:** The *architecture* is trending maintainable; the *process* is
trending toward tech debt. The debt is not (yet) in the code — it's in the
absence of Rust CI, the 40-test baseline, the stubs-as-fidelity strategy, and
the state-in-git habit. Fix the process now; the code is cheap to keep clean,
the process is expensive to retrofit.

---

## 3. Top 3 engineering risks that could derail the port

### Risk 1 — The fidelity metric is not a trustworthy definition of "finished"
The project's own completion signal is `rust_pass / py_pass` per service,
currently ~28% average. Three problems make it unsafe to build a "done"
decision on it:
- Stubs returning 200 inflate pass counts (proven by the DynamoDB
  revert). A service can hit 100% fidelity while half its operations return
  garbage.
- The denominator (Python/moto pass count) is a *moto* pass count, not an AWS
  pass count. "100% parity with moto" is a different, lower bar than the
  product promise ("behaves like AWS"). The probes system (checked-in
  `probes/*.json`) is the honest AWS-side signal and it's not wired into the
  metric.
- Partial/timeout results are silently averaged in.
**Mitigation:** classify stub-backed operations separately and exclude them
from fidelity (or count them as `stub`, not `pass`); add a gate that a service
is "finished" only when: fidelity ≥ X% **and** 0 active stubs **and** the
probes file shows no 500s **and** the compat suite passed without timeout.
Make "finished" a compound, falsifiable state — not a single percentage.

### Risk 2 — The Rust half of the project has no quality gate at all
No CI compiles the Rust code. No clippy. 40 unit tests for 18 crates and a
server. Every "fix: ..." commit since 2026-08-16 was verified only by the
parity run on one machine. Consequences: (a) a refactor that breaks `cargo
test` on Linux (the CI runner) will be discovered late; (b) performance
regressions in a server whose whole pitch is "faster / no Python GIL" are
never measured — there is no benchmark, no load test, no p99 tracking; (c)
when "done" arrives, the Rust codebase will have zero independent validation,
which contradicts a "production-grade" claim.
**Mitigation (cheap, high leverage):** add one CI job —
`cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, and a 5-minute
`cargo bench`/`criterion` smoke — on every PR that touches `crates/` or `src/`.
This is a day of work and removes the single biggest structural weakness.

### Risk 3 — The bridge is a load-bearing dependency with no SLO
18 services are native; everything else (~195 moto services per AGENTS.md) is
routed through a **Python sidecar process** started by a shell script,
restarted by a polling loop in the harness, and talked to over localhost with
a 30s client timeout. The Rust server's "finished" state therefore depends on
a Python process that (a) can die silently — nothing in the *server* checks
sidecar liveness; a dead sidecar turns every bridge call into a 30s hang-then-500,
not a fast failure; (b) is not in CI; (c) has no restart policy (no
supervisor, no Docker healthcheck wiring shown for the Rust binary). Also the
`MotoProxy` parses every proxied response through `serde_json` even when it
only needs to pass bytes through — for XML/query-protocol services the
`unwrap_or(Null)` path means the server can silently return an empty body
instead of surfacing a parse failure.
**Mitigation:** health-check the sidecar from the server (mark bridge
"degraded" in `/health`, fail fast with a 503 + clear error after 2–3s, not
30s); make the Rust binary *the* supervisor of the sidecar (spawn + restart +
deadman), or ship them in one Docker compose with healthchecks; proxy bytes
through without re-parsing when content-type is XML.

**Honorable mentions:** (4) single-machine harness = no reproducible
regression signal; (5) state is in-memory only in Rust while Python has
`ROBOTOCORE_STATE_DIR` persistence — a parity claim users will test immediately
(restart the server, lose everything); (6) the PyO3 `python` feature's purpose
is unclear from the repo — the coexistence model (replace? interpose? embed?)
needs a written decision.

---

## 4. Is the commit frequency and approach sustainable?

**Current approach:** small commits, 6–7/day, each = (implement ops/stubs) +
(run full parity) + (commit `.parity/` state). The *metric-in-message* habit is
genuinely good — it makes progress auditable. The cadence itself is not the
problem; **what the cadence is optimizing for is**.

- **Sustainable:** ~89 commits over 2 weeks for a 18-crate port is normal
  velocity for one focused engineer (or engineer + agent). No need to slow
  down.
- **Not sustainable as-is:**
  1. *Commits of harness output.* Stop committing `.parity/state.json` and the
     JUnit XMLs. Gitignore them; upload as CI artifacts; keep only the
     *derived* artifact (a one-line-per-service fidelity table in a
     `docs/fidelity.md` or the next-work file). This alone would halve diff
     noise and remove a whole class of merge conflicts.
  2. *No gate between commit and commit.* Right now "commit" is the only
     quality event. Insert: Rust CI job (Risk 2) as required status, and
     treat a fidelity regression (any service drops > 2 points) as a merge
     blocker — the revert commit shows regressions are already happening and
     being caught *by the metric, after the fact*.
  3. *Batching by service is right; batching by "stubs + real ops mixed" is
     wrong.* Keep commits atomic per concern: "add stubs for X (22 ops)"
     should not be the same commit type as "fix S3 XML wrapper". The stub
     commits should be marked as such (e.g. `stub:` prefix) and excluded from
     fidelity until they're replaced by real handlers — otherwise the history
     permanently records fake progress.
  4. *The harness is a single point of failure for the workflow.* If
     `parity.py` breaks (it has machine-specific paths and a duplicate
     `_bridge_ready`/`ensure_rust_server` definition — the second silently
     overrides the first), the whole next-work engine stops. Give the harness
     its own tests and pin it in CI.

**Recommended workflow change (minimal):** keep the cadence and the metric,
add (a) a required Rust CI job, (b) gitignored parity state, (c) a
stub-vs-real distinction in the metric, (d) parity harness in CI on main
(nightly is fine) so the timeout-protected loop runs where failures are
visible, not on one laptop. That's the difference between "an engineer is
making progress" and "a project is converging".

---

## Concrete checklist (ordered by leverage)

1. Add `cargo fmt --check` + `clippy -D warnings` + `cargo test` CI job. *(1 day)*
2. `tower` timeout layer on the Axum app; 30s default, configurable. *(half day)*
3. Stop committing `.parity/` (gitignore + artifacts). *(1 hour)*
4. Mark stub-backed ops in the parity metric; exclude from fidelity. *(1 day)*
5. Server-side sidecar health check + fail-fast proxy errors. *(half day)*
6. Panic hook + in-flight/max-latency counters in `/health`. *(half day)*
7. Move `parity.py` to CI (nightly) and fix its timeout handling
   (process-group kill, `status: "timeout"` exclusion, no hardcoded paths). *(1 day)*
8. Decide and document the Rust/Python coexistence model (PyO3 feature's purpose).
9. Write a "definition of done" per service (fidelity + 0 stubs + probes clean +
   no timeouts) — and enforce it in the next-work engine.
