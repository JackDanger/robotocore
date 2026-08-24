---
session: 20260711
slug: cognito-secretsmanager-dropped-fields
type: fix
---

## Context

Part of a broad Bedrock-agent sweep for silent-correctness bugs across robotocore's native providers. This run also produced an IAM `conditions.py` "fix" for `StringLike`/`ArnLike` wildcard matching that was factually wrong (it claimed AWS IAM doesn't support `?` as a single-character wildcard, which it does per AWS's own docs, and rewrote pre-existing correct tests to assert the wrong behavior) — that change and its test were discarded entirely during review and are not part of this commit.

## Root cause

Two more instances of the "field accepted, silently dropped" bug class:

1. **Cognito `CreateUserPoolClient` never initialized several updatable fields** (`LogoutURLs`, `DefaultRedirectURI`, `ReadAttributes`, `WriteAttributes`, `SupportedIdentityProviders`, `AllowedOAuthFlowsUserPoolClient`, `TokenValidityUnits`, `AccessTokenValidity`, `IdTokenValidity`, `RefreshTokenValidity`) even though `_update_user_pool_client` explicitly updates all of them — a client created with these fields set would have them silently discarded at creation time.
2. **SecretsManager `RotateSecret` only copied `secret_string` into the new `AWSPENDING` version**, never `secret_binary` — rotating a binary secret would silently lose its value.

## Fix

- `_create_user_pool_client` now initializes all fields from `params` that `_update_user_pool_client` is able to update, matching the schema `_update_user_pool_client` already expects.
- `_rotate_secret` copies `secret_binary` alongside `secret_string` when present.

## Verification

- New tests: `TestCreateUserPoolClientInitializesAllFields` (cognito), `TestRotateSecretPreservesBinary` (secretsmanager).
- Full local suite: 8741 unit passed. `ruff`/`ruff format`/`mypy` clean on changed files.
