# robotocore-rust — Architecture & Structure Review

**Reviewer angle:** architecture and structure (not per-op behavior)
**Scope:** `src/core/server.rs`, `src/core/protocol.rs`, `src/core/proxy.rs`, `src/router.rs`, `src/core/spec.rs`, `crates/s3`, `crates/iam`, `crates/lambda`, `crates/stepfunctions`, `crates/sqs`, `contracts/`, `scripts/harness/parity.py`

---

## Verdict

The architecture is **sound in its skeleton** — the catch-all + service-router + per-crate handlers + Moto sidecar bridge is a workable shape for a multi-protocol AWS mock, and it is the same shape the Python original uses. It will not need to be torn down to reach 90%.

However, it will **not** reach 90% if it evolves the way it currently does, because the structure pushes every protocol-specific detail into the handler crates as copy-pasted string handling. Three structural problems will block progress:

1. **REST operation resolution is hand-rolled string-matching in core (`protocol.rs`), duplicated in the S3 crate, and never spec-driven.** Every REST service (S3, Lambda, API Gateway, …) needs this, and today each one gets a bespoke branch.
2. **The `ParsedRequest`/`ParsedResponse` boundary is leaky and duplicated across 18 crates** (18 near-identical `AwsRequest`/`AwsResponse` types + 17 near-identical adapters in `server.rs`). Every protocol improvement has to be made 18 times.
3. **A spec/contract layer exists (`spec.rs`, `contracts/*.json`) but is completely unused at runtime** — so response shapes are typed by hand, and the parity diff can never drive generation.

None of these require re-architecting; all three are "finish the abstraction" fixes.

---

## 1. Is `ParsedRequest`/`ParsedResponse` a good fit for REST (S3, Lambda)?

**Partially. The shape is fine; the content and the duplication are not.**

What works:
- One struct carries method, path, query string, headers, raw body, account, region — exactly what REST ops need. S3/Lambda adapters pass all of it through.
- The `raw: Option<String>` on the response lets native crates own their own serialization (XML, binary, custom JSON) — this is the right escape hatch.

What doesn't:
- **Two parallel worlds.** Each crate defines its own `protocol::AwsRequest`/`AwsResponse` (18 copies, each slightly different: S3 adds `bucket`/`key`/`query_params`, IAM adds `query`, Lambda adds `method`/`path`/`headers`…). The core `ParsedRequest` fields are then re-copied into them by hand in `server.rs`. `ParsedRequest` itself is a superset of all of them already — the per-crate types are a redundant indirection, and they have drifted (IAM's `body: Bytes` is unused; Lambda's `params: Value` duplicates `body`).
- **17 copy-paste adapters in `server.rs`.** `SnsServiceHandler`, `SmServiceHandler`, `KmsServiceHandler`, `SsmServiceHandler`, `Kms…` are 12–16 lines each doing the exact same `serde_json::to_value(&req.params)` + header-map copy. This is the single biggest source of repeated work in the tree. A single generic `impl Bridge<H> where H: Handle<AwsRequest>` (or just making the crates accept the core types) deletes ~500 lines and, more importantly, means a protocol fix (e.g. better query parsing) lands in one place.
- **`params` semantics differ per protocol and are decided by one brittle rule in `server.rs`:** `content-type contains "json" ? parse_json : parse_form`. For query-protocol services (SQS, IAM, STS) params come from the form body; for JSON-protocol from JSON; for REST (S3/Lambda) from method+path+query — and the core `params` is nearly meaningless there. That one `if` is where subtle bugs live (e.g. IAM `GetAccessKeyLastUsed` relies on the form body being parsed *and* on `req.query` being a fallback — two code paths for the same data).
- **`ParsedResponse.body: Value` is a dead field for every native crate** (they all set `raw`). Keeping both `body` and `raw` in one struct encodes "sometimes JSON, sometimes raw string, sometimes XML" in the type system, which is why `response_from_parsed` needs the `service == "sts"` special case to pick the XML serializer.

Recommended shape (small, not a rewrite):
- One shared `AwsRequest`/`AwsResponse` in a `robotocore-core` crate (the superset that already exists). Crates implement `trait Service { fn handle(&self, req: &AwsRequest) -> AwsResponse; }` and register directly — no adapters, no per-crate protocol modules, no `handle_sync`/`Box<dyn Error>`.
- Make `params` a `&Value` computed once by a per-service **protocol codec** (see below), and drop the parallel `body: Value` on responses; `raw` is the only body representation.

## 2. Is the single `catch_all_handler` the right pattern?

**Yes — keep it.** A catch-all is the correct top level for an AWS mock: service and operation are not in the URL in any uniform way (X-Amz-Target header, `Action=` form field, credential scope, virtual host, REST method+path). The Python original routes the same way, and the Rust `router.rs` port of it is genuinely good: well-tested, ordered heuristics, alias tables, SigV2/3/4 handling, ~60 focused unit tests.

Caveats worth fixing while it's there:
- **Operation resolution is split across three places with different behaviors:** (a) `extract_operation` in `protocol.rs` (Target header → `Action=` scan of raw body → `resolve_rest_operation`), (b) `S3Handler::detect_s3_operation` in the S3 crate, (c) the Lambda branch of `resolve_rest_operation` in `protocol.rs` (≈150 lines of `if/else` on path prefixes). The S3 crate also re-parses the query string a second time in the adapter. One request to S3 thus flows: router → `extract_operation` → `detect_s3_operation` → `to_s3_request` (query parsed again) → `handle`. The `x-robotocore-path` header trick (injecting the URI path as a header so a core function can resolve REST ops) is a code smell — `extract_operation` should take the `Uri`/path directly.
- **`resolve_rest_operation` is where the Lambda 12.1% will stay stuck.** It's a hand-maintained mapping of ~30 path patterns to ops, in core, for *one* service. The same problem will be re-solved for API Gateway, ECR, ECS, SESv2, etc. Every missed branch = a silent `operation=""` = a `ResourceNotFoundException` with a misleading message.
- The async STS handler wrapped in a hand-rolled `RawWaker` to poll an "async but never awaits" function (`StsFunctionHandler`) is fragile ceremony; if the core trait is `fn handle(&self, &AwsRequest) -> AwsResponse` (sync), STS just becomes a normal crate like the others and the wrapper disappears.
- Non-native services fall back to the Moto proxy **only when the router produces a service name it recognizes as non-native** — anything the router can't classify gets `400 Could not determine service` instead of a proxy attempt. That's fine for fidelity (the Python server would 400 too) but the error path should be audited against the Python behavior, because it's the one path that diverges structurally.

## 3. Missing abstractions causing repeated work

Ranked by impact on the fidelity numbers:

### 3.1 Spec-driven REST operation resolution (biggest gap)

`spec.rs` already loads botocore `service-2.json` (operations, shapes, `http` blocks: `method` + `requestUri`) and `contracts/*.json` already records per-op response keys, header keys, error formats, and key types. **Nothing in the Rust server reads them at runtime.** Consequences:

- REST op mapping is hand-written (see §2). Botocore's spec already contains the exact `method + requestUri` template for every op of every service. A 50-line table builder (`service-2.json` → `[(method, regex, op)]`, ordered longest-first) would *replace* `resolve_rest_operation` entirely and give every REST service (S3's 100+ ops, Lambda's 50, ECR, ECS, API GW…) correct op resolution for free — including the sub-resources the hand-written Lambda branch currently misses (the parity gap list shows alias routing configs, URL configs, event-invoke configs failing exactly this way).
- Response shapes are typed by hand per op (Lambda's `func_config` hardcodes 16 fields; IAM's `xml()` string-concatenates every list; StepFunctions stores whole resources as `serde_json::Value` and reprojects field-by-field in each op). The recorded contracts (`response_keys`, `key_types`, `header_keys`) are exactly the diff data the parity system produces — wire them into a codegen step and most of the "missing field" failures (e.g. `AccountLimit` absent from `GetAccountSettings`, `creationDate` absent from `CreateActivity`, `UserName` absent from `GetAccessKeyLastUsed`) become a generated fixture, not 200 hand-edited responses.

This is the one structural investment that compounds: it fixes op resolution *and* response fidelity *and* gives `parity.py` machine-readable output to close the loop.

### 3.2 No shared query-protocol (XML) toolkit

IAM (1,444-line handler) and the STS serializer are both hand-rolling XML:
- `iam::protocol::AwsResponse::xml` concatenates `<…Response><…Result>` strings; list ops manually emit `<member>…</member>` (which is actually *wrong* for IAM — moto/boto expect `<member>` only for some lists and plain nesting for others, and the current tests that pass do so only by luck of assertion); errors are string-formatted with no XML escaping (a policy document containing `&` or `<` will produce malformed XML → botocore parse crash, visible as `AttributeError` in the gap list).
- `protocol.rs::serialize_query_response` (STS) is hard-wired to the STS 2011-06-15 namespace and does flat element emission with no list/member support.
- S3 has a small `xml.rs`, but `list_objects` still post-processes with `body.replace("</ListBucketResult>", …)` to bolt on `KeyCount`/`EncodingType`.

There is one quick-xml dependency sitting in `Cargo.toml` that barely any crate uses. A single `query-xml` module (request param extraction with `member.N.`/`TagKey.N` flattening, response builder from `Value` with namespace + `ResponseMetadata`, proper escaping) would remove the largest class of IAM/STS/SQS failures — and it's the only way IAM's 26.6% is realistic, because IAM is 200+ ops all in this protocol.

### 3.3 No shared pagination/next-token utility

Every list op re-implements (or fakes) pagination: S3 `is_truncated = false` hardcoded, StepFunctions echoes `nextToken` back unmodified, IAM/SQS/ECR lists ignore `MaxItems`/`NextToken` entirely (the StepFunctions gap list has `test_list_activities_pagination` failing). A tiny `Pager` (opaque token = base64 JSON of (table, offset/last-key)) in core would clear an entire test category across all 18 services.

### 3.4 No cross-service state/coordination layer

SQS↔SNS fanout, EventBridge→Lambda targets, Step Functions→Lambda invocations, and IAM roles referenced by Lambda all require two services to see each other's state. Today each crate owns its own `RwLock<HashMap<(u64,String), X>>`, and `core/state.rs` (a generic `ResourceTable` store) exists but is unused. Expect a wall at ~70–80% fidelity where the remaining failures are *integration* behaviors (SNS→SQS fanout, Step Functions task success/failure actually driving executions). A shared `World`/`Bus` (per-account-region registry + event queue) is the second structural investment that compounds.

### 3.5 Smaller items

- **`/`_robotocore/*` endpoints are stubs** (health reports only STS, uptime 0, audit empty, no per-service status). The Python server's health/services/audit are part of its contract (the parity harness itself uses them). Cheap to fill, and the audit log is invaluable for the next debug cycle.
- **Account/region extraction only from the SigV4 `Credential=` scope.** `X-Robotocore-Account` exists but presigned-URL and SigV2 paths (S3) fall to the default account; also `parse_account_from_key` requires exactly-12-digit keys or the "testing" key routes to the default account — the compat tests use `aws_access_key_id="testing"`, which is why everything currently runs in one account. Fine for parity, wrong for the "multi-account isolation" claim in AGENTS.md.
- **Proxy header allow-list is lossy** (`x-amz-` prefix + a few exact names): `content-md5`, `x-amz-copy-source`, `x-amz-meta-*`, `x-amz-acl`, etc. are dropped on the way to Moto. S3 metadata behaviors proxied through the bridge will silently fail.
- **`ServiceRegistry::new()` hardcodes the 18 services** and the binary hardcodes the native list *again* — two lists that must stay in sync (the parity `NATIVE` map is a third). Derive the list from the registry.
- **Duplicate match arms** (StepFunctions `"ListStateMachines"` appears twice) — no exhaustiveness checking against the spec; a codegen check (`spec ops` ⊇ `match arms`) would catch dead/unimplemented ops automatically.

---

## What will actually move the numbers

| Fidelity | Service | Structural blocker |
|---|---|---|
| 1.4% | stepfunctions | Hand-written field re-projection from raw `Value`; no pagination; no ASL execution model (that last is feature work, not structure). |
| 12.1% | lambda | Op resolution via hand-matched path table in core; hardcoded response shapes; no `AccountLimit`/limits model. |
| 26.6% | iam | No shared query-XML toolkit; 200+ ops each hand-concatenating XML; no escaping; pagination absent. |
| 51.4% | s3 | Mostly feature-complete for the common ops; remaining gaps (multipart edge cases, versioning, events, list truncation) are behavior, not structure — but the `body.replace()` XML post-processing will keep producing subtle wire bugs. |
| 53.7% | sqs | Batch ops, message attributes — behavior work; structure is fine (though the endpoint-URL format hardcodes port 4566 in queue URLs, which breaks when the Rust server runs on 4567 — verify this doesn't already cost parity points). |

**Sequencing recommendation:** (1) extract one core `AwsRequest`/`Service` trait and delete the adapters (~1 day, zero behavior change); (2) spec-driven REST op resolution + codegen from `contracts/` (1–2 weeks, directly raises S3/Lambda/SF/IAM); (3) shared query-XML + pagination utilities (1 week, directly raises IAM/STS/SQS/SNS); (4) shared state bus when cross-service tests start dominating.

The architecture does not need to change to hit 90%; the *abstractions that were scaffolded but never connected* (spec, contracts, state store, protocol codecs) need to be finished.
