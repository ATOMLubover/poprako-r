# Integration tests

- `integration-tests/` is a standalone Deno/TypeScript workspace. Run
  `deno task check` there for formatting, lint, and type checks; run
  `deno task api` there against an already-running API server.
- When required variables exist only in the project `.env`, run
  `deno task --env-file=../../.env api` from `integration-tests/`.
- Changes to integration test cases under `integration-tests/src/` must be
  reflected in `integration-tests/TESTCASES.md` in the same change. Preserve
  stable case IDs unless removing a case; update additions, removals, renames,
  and changed assertions or coverage.
- From the repository root, `sh scripts/api-integration-test.sh` runs the
  isolated database-backed suite. It creates and drops its target database:
  `INTEGRATION_DATABASE_URL` must name a confirmed disposable database whose
  name ends in `_integration`. Apply the root database authorization rules.
