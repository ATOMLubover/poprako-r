---
name: test-spec
description: Apply PopRaKo Rust, PostgreSQL, and HTTP integration test conventions when changing or reviewing tests or externally visible behavior.
---

# Test conventions

- Keep Rust tests in a sibling `tests.rs` or adjacent `tests/` subtree;
  do not append inline test modules after production items.
- Exercise server use-case behavior through
  `part_impl::repo::mock_impl::Mock` and module-local fixtures where practical.
  Other workspace crates use their own contracts and local helpers.
- Add RDB tests for correctness depending on queries, PostgreSQL constraints,
  locking, rollback, or generated schema behavior.
- Cover success and relevant failure classes: invalid input, missing data,
  permission denial, stale state, concurrency, and rollback.
- Use descriptive names and the checked source-comment and layout rules.
- For changed HTTP behavior, update the affected Deno suite and its case
  documentation under [tests/AGENTS.md](../../../tests/AGENTS.md).
- For changed API contracts, regenerate and verify OpenAPI under
  [docs/AGENTS.md](../../../docs/AGENTS.md).

## Validation

Run from the repository root unless indicated. Start with affected tests,
then broaden for changed shared ports, transactions, API, or error handling.
Root AGENTS specifies mandatory Rust checks and the full pre-commit gate.

| Scope | Command |
| --- | --- |
| Focused server tests | `cargo test -p poprako-server <test-filter>` |
| Server tests | `cargo test -p poprako-server` |
| Other crate tests | `cargo test -p <affected-crate>` |
| Workspace tests | `sh scripts/ci-test.sh` |
| Workspace compilation | `cargo check --workspace --all-targets --all-features` |
| Deno static checks | `deno task check` from `tests/integration-tests/` |
| OpenAPI consistency | `sh scripts/ci-openapi-check.sh` |

Feature-gated RDB tests require their relevant features and database fixtures;
a default-feature test pass is not evidence that those tests ran. Inspect the
touched module's `cfg` and setup before choosing the test command.

Database-backed HTTP runs use `sh scripts/api-integration-test.sh`. Follow
the linked tests instructions for its disposable target and root approval
rules. Migration validation uses the dedicated CI database and the full
apply → revert-all → apply cycle through `scripts/ci-migration-check.sh`.
