# poprako-server

Rust 2024 backend for manga translation project management. `Cargo.toml`
defines the workspace; `src/lib.rs` defines the server module graph;
`src/main.rs` composes production adapters and starts the Axum server.

## Mandatory guardrails

- Preserve user-authored changes; never overwrite unrelated work.
- Never modify linters unless the user explicitly requests it.
- Never hand-edit generated `schema.rs` or `docs/swagger.json`. Locally,
  regenerate schema only with `just mgr-schema`; see the fullchain skill for
  migrations and `docs/AGENTS.md` for OpenAPI generation.
- Releases and production deployments must run through GitHub Actions.
  Production SSH is allowed only from the protected Actions environment
  through the dedicated deployment account. Maintainer machines must never
  execute production deployment scripts.
- Do not run local image/release builds, release helpers, or deployment
  scripts unless the user explicitly requests that exact operation. Use the
  checked-in CI validation scripts for deployment preparation.
- Never apply database migrations during application startup.
- Destructive migration verification requires explicit approval and a
  confirmed disposable database. Exception: `db_poprako_ci` and its
  `poprako-ci-pg` container are already authorized disposable CI resources;
  their validation must complete apply → revert-all → apply. Starting this
  CI database container is permitted and is not a release image build.
- Never bypass or disable commit hooks, including with `--no-verify`.
  Every commit must pass the full pre-commit CI chain.
- CI/CD must invoke checked-in POSIX `sh` scripts directly and must not
  require `just`. Apart from local schema regeneration, `just` is optional.

## Work and validation

- For non-trivial work: plan → implement → review with targeted validation.
  Read nearby active code and business references before choosing contracts.
- Load the relevant skills below before editing their areas. Keep changes
  limited to the layers the requested behavior needs.
- After editing Rust, run `cargo fmt --all --check` and
  `cargo check --all-features`, plus tests appropriate to the affected code.
- Run custom repository linters only when requested or when validating a CI
  entry point. Basic CI checks: `sh scripts/ci-check.sh`. Full pre-commit CI:
  `sh scripts/ci-local.sh`. A basic check is not a substitute for the full gate.
- Keep Rust files strictly below 600 lines. The current CI script rejects
  only files above 600; passing it does not relax the authoring rule.
- Keep comments in English and blank lines between Rust statements. Do not
  use `if ... else` or create `mod.rs`; use guards, `match`, and `let ... else`.

## Skill routing

Skills live in `.agents/skills/<name>/SKILL.md`. Read only those applicable
to the task; follow their links when the referenced area is affected.

| Task | Skill |
| --- | --- |
| Rust source creation, modification, moves, or review | `general-conventions` |
| Backend behavior changes across layers | `implement-fullchain-spec` |
| Use cases, repository contracts, or delivery boundaries | `usecase-boundaries` |
| Instr, Val, View types and dependencies | `data-dto-boundaries` |
| Harn, AppHarn, startup or test composition | `harness-spec` |
| Error classification, conversion, or propagation | `error-handling-spec` |
| Instrumentation, logging, or error-conversion events | `tracing-usage-spec` |
| Tests or externally visible behavior | `test-spec` |
| Rust struct fields in any workspace crate | `struct-field-blank-lines` |
| Module extraction for the file-length limit | `module-splitting-conventions` |

Server-specific domain, DTO, and error rules apply to the server crate.
Do not impose its HTTP types or domain dependencies on infrastructure crates.
