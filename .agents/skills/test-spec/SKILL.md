---
name: test-spec
description: Current PopRaKo Rust, PostgreSQL, and HTTP integration test conventions. Use whenever adding, changing, moving, or reviewing tests or externally visible behavior.
---

# Test conventions

- Keep Rust tests in a `tests.rs` sibling or a `tests/` subtree beside the
  module they verify; do not add inline test modules after production items.
- Exercise public use-case behavior through
  `part_impl::repo::mock_impl::Mock` where practical.
- Add RDB tests when correctness depends on Diesel queries, PostgreSQL
  constraints, locking, transaction rollback, or generated schema behavior.
- Cover the successful path and every relevant failure class: invalid input,
  missing data, permission denial, stale state, concurrency, or rollback.
- Use descriptive test names and follow the checked comment and layout rules.

HTTP suites live under `tests/integration-tests/src/suites/`. Whenever suite
coverage changes, update `tests/integration-tests/TESTCASES.md` in the same
change and keep stable case identifiers unless a case is removed.

Run validation directly through portable commands. `just` may be used as a
local convenience but is not a CI dependency:

```sh
cargo test -p poprako-server <test-filter>
cargo test -p poprako-server
cd tests/integration-tests && deno task check
```

Run `scripts/api-integration-test.sh` only with
`INTEGRATION_DATABASE_URL` pointing to a dedicated disposable database. The
script creates and drops that database.
