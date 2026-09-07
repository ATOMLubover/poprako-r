# poprako-server - Agent Context

MUST NOT GENERATE A RELEASE until the GitHub Actions release workflow is
established. After that workflow exists, releases and production deployments
MUST run through GitHub Actions. Production SSH is allowed only from the
protected GitHub Actions environment through the dedicated deployment account;
maintainer machines must not run production deployment scripts.

NEVER TOUCH schema.rs by hand!!!!!!!!!!!!!!!!!!!! The only way to change it is to use `just` scripts.

Deployment preparation on a maintainer machine is CI-only by default. Do not
run local Docker/image/release builds, local release helpers, or any deployment
script unless the user explicitly requests that exact operation. Run the
checked-in CI checks instead. Do not run destructive migration verification
(`migration revert`, reset, or equivalent) unless the user explicitly approves
it and the target is confirmed to be a disposable database.

Never bypass a commit hook with `git commit --no-verify`. A commit intended for
review, merge, release, or deployment must complete the repository pre-commit
CI chain successfully. `db_poprako_ci` and its `poprako-ci-pg` container are
explicitly disposable CI resources: migration validation must run its full
apply → revert-all → apply cycle there, and production/beta data-protection
rules do not apply to that CI database.

`just` is an optional local convenience only. CI/CD and release automation
MUST invoke checked-in POSIX `sh` scripts directly and MUST NOT require `just`.

`poprako-server` is the Rust 2024 backend for manga translation project
management. The executable is active: `src/main.rs` wires the production
harness and starts the Axum HTTP server.

After editing Rust, run `cargo fmt --all --check` and
`cargo check --all-features`. Run the custom `fmt/` checker suite only when the
user asks for it or when validating the CI entry point.

## Active Architecture

`src/lib.rs` is the authoritative module graph.

```text
src/
├── api/http/       # Axum handlers, middleware, router, OpenAPI, server
├── complex/        # Pure business rules and perm checks
├── data/           # Instr requests, Val responses, and API views
├── harn.rs         # Production application-harness composition
├── model/          # Persisted application models and input forms
├── part/           # Ports and repository operation descriptors
├── part_impl/      # RDB, R2, JWT, prom, effect, and test-mock adapters
├── usecase/        # Generic application orchestration
└── value/          # Shared value objects and enums
```

- `part::repo::oper` contains Orchestra operation descriptors. Repository
  capabilities declare `run` operations and transaction-scoped `step`
  operations with `#[drive(...)]`.
- `Nucl::coord` supplies the transaction context. A use case owns the
  transaction boundary; `complex` remains pure and never drives transactions.
- `Harn` composes production ports. Tests use `part_impl::repo::mock_impl` and
  the test helpers local to the touched module.
- RDB repository code and generated Diesel schema live under
  `src/part_impl/repo/rdb_impl/`. Never edit `schema.rs` directly or import it
  through a `schema::` alias. Change migrations, regenerate it with
  `diesel print-schema > src/part_impl/repo/rdb_impl/schema.rs`, then use the
  generated table module through its full local path.

## Implementation Rules

- For non-trivial work: plan, implement, then review with targeted validation.
- Read nearby active Rust code before choosing names, trait bounds, error
  handling, or test structure.
- Keep comments in English. Do not use `if ... else`; prefer `match`, guards,
  or `let ... else`.
- Keep Rust files under 600 lines and place blank lines between statements.
- Give typed local bindings specific domain names, such as `comic_info` or
  `cover_reservation`.
- Bind a data struct before passing it to a step factory; do not nest inline
  struct literals in step calls.
- Bind transaction output before returning from a use case. For unit output,
  await the transaction, then return `Ok(())`.
- Preserve user-authored working-tree changes. Do not overwrite unrelated edits.
- Never edit linters unless user demand you to do so.

## Active Development Slice

When a behavior changes, update only the layers it needs, in this order:

1. Read current Rust code and the corresponding business reference.
2. Add shared `value`, `model`, or pure `complex` logic where warranted.
3. Add `Instr`/`Val`/`View` DTOs, repository operation descriptors, and the
   required `#[drive(...)]` repository capability bounds.
4. Implement both RDB and mock adapters as needed.
5. Implement the use case, then the HTTP handler/router/OpenAPI surface when
   the behavior is exposed over HTTP.
6. Add focused Rust tests and keep `tests/integration-tests/TESTCASES.md` in
   sync with integration-suite changes.

## HTTP and Migrations

HTTP routes are active under `/api/v1`; health is `/api/health`. Handlers use
`Accept as _`, propagate use-case errors with `?`, and keep router and
`#[utoipa::path]` declarations aligned. The `swagger` Cargo feature exposes
`/api/swagger-ui` and `/api/openapi.json`; `--swagger` prints the generated
specification.

For schema changes, edit matching migration `up.sql` and `down.sql`, apply or
revert with `diesel migration run` / `diesel migration revert`, regenerate
with `diesel print-schema > src/part_impl/repo/rdb_impl/schema.rs`, then
compile. The generated file is
`src/part_impl/repo/rdb_impl/schema.rs`.

The application must never apply migrations during startup. Production CD
uses `scripts/ga-apply-migrations.sh` to apply all `up.sql` files in one
transaction against an independently managed, already-running PostgreSQL 18
container.

## Project Skills and Verification

Project-local Rust guidance lives in `.agents/skills/`. Load the relevant skill
before editing its area; the active skills cover conventions, errors, traits,
tracing, tests, imports, and formatting.

Run the narrowest useful check first, then broader checks when shared ports,
transactions, API, or error handling change:

```text
cargo fmt --check
cargo check
cargo test -p poprako-server
```
