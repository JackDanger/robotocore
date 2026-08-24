---
session: "53e94e"
timestamp: "2026-08-17T21:15:00Z"
model: claude-sonnet-5
pr: 312
tools: [subagent-review]
---

## Human

(Context: found while building an unrelated Terraform module that deploys a
Python Lambda against robotocore as a local twin for testing.) The deployed
Lambda's handler read a configured environment variable at module scope —
`FORWARDER_ADDRESS = os.environ["FORWARDER_ADDRESS"]` above the handler
function — and every invocation failed with `KeyError: 'FORWARDER_ADDRESS'`,
even though `get_function` reported `Environment.Variables` correctly.

Open a PR with fixes to RobotoCore.

Later, once the PR was open:

Have Fable review that PR and then attack it from every different angle
until it's completely polished. Once you can't make it any better, merge it.

## Assistant

## Key decisions

**Root cause**: `execute_python_handler` imported/executed the handler
module (`spec.loader.exec_module`) in the dispatcher thread, but only
installed the invocation's thread-local `os.environ` override
(`_thread_local.env`) inside the separate worker thread that later calls the
handler function. Module-level `os.environ[...]` reads therefore ran before
the thread-local environment existed, falling through to the real process
environment instead. `get_function`/`get_function_configuration` reported
the configured variables correctly the whole time — only what the module's
own code could see at import time was wrong, which is what made this
confusing to isolate.

**Fix**: moved module lookup/import/exec inside `_run_handler` (the worker
thread), after `_thread_local.env` is set, so import-time reads resolve
identically to invocation-time ones — matching real Lambda, where env vars
are present on the execution environment before any code runs.

**Review process**: ran three adversarial review passes (Claude Fable model,
via subagent, each with instructions to attack the diff from a different
angle rather than confirm the obvious story) before merging, rather than
merging on the first green test run.

- Round 1 found: two stale comments describing code that no longer ran
  where the comment claimed; `except Exception` (not `BaseException`) in the
  worker thread meant a handler module calling `sys.exit()` at import time
  fell through every check to a silent `(None, None, logs)` success instead
  of being reported as a failed invocation; post-timeout branch ordering;
  and — the most consequential finding — that this fix's own design exposed
  a *new* staleness bug: module-scope config resolved once at import time
  had no way to notice `UpdateFunctionConfiguration` changing
  `Environment.Variables`. Pre-fix this crashed loudly (the KeyError); post-fix
  it would have silently kept serving the old value forever. Fixed by having
  the config-update handler invalidate the code cache (forcing a re-import)
  when `Environment`/`Handler` changes. Also added test coverage for
  concurrent module-scope reads, the warm/cached-module path, module-scope
  `print()`, module-scope `sys.exit()`, and the fact that module import now
  counts against the function timeout (previously imports were
  un-timeout-able) — a real, previously-unobserved behavior change from
  moving import into the timeout-governed thread.
- Round 2 found the round-1 staleness fix was itself incomplete:
  `CodeCache.invalidate()` only cleared `_lambda_{fn}.`-prefixed
  `sys.modules` entries, so a multi-file function's plain-named helper
  modules (`import shared`) survived invalidation with stale state —
  reproduced empirically before fixing. Also: a `Layers`-only config update
  wasn't covered (layer bytes aren't part of the extraction cache key but
  are baked into the cached extraction directory), and the round-1 pinning
  test only called `get_code_cache().invalidate()` directly rather than
  exercising the real `UpdateFunctionConfiguration` HTTP handler. Fixed all
  three; verified the new HTTP-level test actually catches the regression by
  temporarily disabling the fix and confirming the test fails, against a
  Python-3.12 server matching the test function's declared runtime (a
  version-mismatched server silently routes through a subprocess dispatch
  path that sidesteps the in-process caching bug entirely and would have
  made the test a false positive).
- Round 3 re-verified rounds 1 and 2 hadn't introduced new problems (lock
  ordering, multi-key cache entries, whether `Layers` is actually a valid
  `UpdateFunctionConfiguration` field per the real botocore service model,
  xdist/CI scheduling hermeticity of the new compat test) and ran the full
  suite three times serially plus once under the CI's parallel xdist
  config. Found two more pre-existing gaps in the same staleness-bug class
  (LRU eviction and the `_pending_cleanup` path don't clear plain-named
  modules either) but confirmed both pre-date this PR and aren't made worse
  by it — filed as issue #313 rather than expanding this PR's scope into
  unrelated cache-eviction code.

**Merge**: all CI checks green (unit ×2 Python versions, compat, integration,
IaC parity, docker-build) before merging.
