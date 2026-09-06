# Organization Transfer Readiness Checklist

This checklist tracks the work required before moving `poprako-server` from
the personal repository into the official PopRaKo organization. Complete the
blocking items before the transfer, then verify the organization-side settings
immediately after it.

## 1. Repository identity and public entry points

- [x] Decide the final organization and repository name.
  - Final URL: `https://github.com/poprako-dev/poprako-server`.
  - Confirm that the target organization does not already contain a repository
    or fork with the same name.
- [x] Add a root `README.md`.
  - Describe the product and current project status.
  - Document prerequisites, local configuration, database setup, migrations,
    development commands, tests, and production build commands.
  - Link to the API specification and contribution guide.
- [x] Add the full MIT license text as root `LICENSE`.
  - Keep `license = "MIT"` in Cargo package metadata.
- [ ] Add a repository description, website, and topics on GitHub.
- [x] Use protected `main` as the sole long-lived integration branch.
  - Create short-lived feature and fix branches from `main` and open their pull
    requests directly against `main`.
  - Do not maintain a separate development or release-integration branch.

### Acceptance

- [x] A new contributor can understand and start the project from the README.
- [ ] GitHub recognizes and displays the repository license.
- [x] All Cargo package repository URLs use the final organization URL.

## 2. Community and governance files

- [x] Add `CONTRIBUTING.md`.
  - Explain the branch and pull-request workflow.
  - List the required Rust, custom-format, and integration-test commands.
  - Distinguish standard Rust practices from PopRaKo-specific style rules.
  - Explain when `tests/integration-tests/TESTCASES.md` must be updated.
- [x] Add `SECURITY.md` with a private vulnerability-reporting channel.
- [x] Add a repository `CODE_OF_CONDUCT.md`.
- [x] Add issue forms and a pull-request template, either locally or through
  the organization's public `.github` repository.
- [x] Keep review ownership in repository rules instead of maintaining a
  `CODEOWNERS` file for the current small team.
- [x] Keep support and governance policies in this repository
  or the organization's shared `.github` repository.

### Acceptance

- [ ] Opening an issue or pull request presents the expected templates and
  contribution guidance.
- [x] Security reports have a documented non-public path.
- [x] Sensitive infrastructure changes remain subject to the repository's PR
  approval rule without requiring a dedicated reviewer team.

## 3. Clean stale and personal repository state

- [x] Remove `.agents/settings.local.json`.
  - It currently contains personal absolute paths and an obsolete
    `poprako-r` path.
  - Shared Agent configuration must not depend on one maintainer's home
    directory.
- [x] Refresh `src/AGENTS.md`.
  - Remove references to deleted `.bak` files, `forward_ref.rs`, and `domain/`.
  - Replace the generated layout with the authoritative active module graph.
- [x] Refresh `docs/AGENTS.md`.
  - Remove the reference to the missing `how-to-implement-api-http.md`.
  - Reconcile the instruction to remove stale plans with the files currently
    kept under `docs/plans/`.
- [x] Review all `NOTE.md`, `CHECK-LIST.md`, `docs/todos.md`, and
  `docs/plans/` files.
  - Keep only active references with a clear owner.
  - Delete completed migration notes and obsolete design plans.
- [x] Remove the tracked zero-byte root `openapi.json`.
- [x] Remove `docs/swagger.previous.json`; no active workflow needs
  it. It currently duplicates `docs/swagger.json` exactly.
- [x] Keep `app_config.toml` at the root because the active local
  startup path reads that exact filename; production uses the deploy copy.
- [x] Remove legacy `poprako-sr` deployment names and
  `deploy/poprako-sr/`.
  - Review defaults in `scripts/docker-run-prod.sh`,
    `scripts/local-run-release.sh`, and `scripts/local-stop-release.sh`.
  - Review the legacy `/opt/poprako-s/shared/.env` production dependency.

### Acceptance

- [x] `rg 'poprako-r|poprako-sr|/Users/|/home/'` returns only documented,
  intentional compatibility references.
- [x] Every path named by an `AGENTS.md` file exists or is clearly presented as
  an example.
- [x] There is exactly one checked-in generated OpenAPI document.

## 4. Cargo workspace metadata and toolchain

- [x] Correct the repository URL in `poprako-util/Cargo.toml`.
  - Remove the unrelated `https://github.com/anomalyco/poprako-server` value.
- [x] Add a workspace-level package policy in the root `Cargo.toml`.
  - Inherit the version, edition, Rust version, license, repository, and
    publishing policy where appropriate.
- [x] Centralize shared dependency versions with `[workspace.dependencies]`.
- [x] Centralize shared lint policy with `[workspace.lints]` and inherit it in
  every member crate.
- [x] Select and document Rust 1.95 as the supported version.
  - Add `rust-version` to workspace package metadata.
  - Add a pinned `rust-toolchain.toml`.
  - Use the same exact Rust version in the Docker builder image.
- [x] Retain the existing Cargo feature names and combinations.
  - Confirm that `rdb`, `repo_impl`, and `prom_impl` are still the intended
    names and combinations.
  - Document test-only feature combinations used by RDB tests.
- [x] Make root Cargo commands default to the server package
  or the entire workspace.
  - CI commands must always specify `--workspace` explicitly.

### Acceptance

- [x] `cargo metadata --no-deps --format-version 1` reports the correct
  repository and Rust version for all members.
- [x] Local, CI, and Docker builds use the same Rust 1.95 toolchain policy.
- [x] No workspace member silently misses the shared lint policy.

## 5. Make repository rules enforceable

- [x] Resolve the contradictory formatting instruction in root `AGENTS.md`.
  - Read-only verification should use `cargo fmt --all --check`.
  - Code-changing workflows may run `cargo fmt --all` before verification.
- [x] Enforce the 600-line Rust file limit for new files and prevent growth of
  the 15 explicitly grandfathered files.
  - If mandatory, add an automated check.
  - Split or explicitly waive the 15 files currently at or above 600 lines.
  - The current largest file is `src/complex/chapter_port/import.rs` at 770
    lines.
- [x] Keep the project-specific guard/match preference mandatory for changed
  code and document it in `CONTRIBUTING.md`.
  - In particular, decide whether `if ... else` is truly forbidden or merely
    discouraged when guards or `match` are clearer.
  - Keep project-specific rules in `CONTRIBUTING.md` and automate every hard
    requirement.
- [x] Add a portable `scripts/ci-check.sh` as the validation entry point.
  - Use `cargo fmt --all --check` instead of the mutating formatter.
  - Ensure a custom checker failure cannot be masked by a later command.
  - Check the entire workspace and all targets.
  - Treat Clippy warnings as errors.
  - Keep `just fmt-check` only as an optional local wrapper around the script.
- [x] Fix the current strict Clippy failures.
  - Remove four unused `super::*` imports in RDB test modules.
  - Resolve the unused `find_chapter` helper.
  - Replace the needless `.last()` traversal in the chapter-port import test.

### Acceptance

The following commands all succeed from a clean checkout:

```sh
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
sh fmt/run-check.sh
```

## 6. Continuous integration

- [x] Add a pull-request CI workflow under `.github/workflows/`.
  - Run CI for pull requests targeting `main`.
  - Run branch CI after changes land on `main` so production deployment uses
    the exact checked and merged commit.
- [x] Invoke `sh scripts/ci-check.sh`; do not install or require `just` in CI.
- [x] Run `cargo test --workspace` through `sh scripts/ci-test.sh`.
  - Separate fast unit tests from Docker/PostgreSQL-backed tests if needed.
  - Make required services and feature flags explicit in the workflow.
- [x] Validate Diesel migrations against a disposable PostgreSQL 18 Alpine
  database, matching the Rust RDB testcontainers environment.
  - Run every migration up, revert all migrations, and run them all again.
  - Reject non-CI database names before the destructive rollback check.
- [x] Validate TypeScript integration-test sources through
  `sh scripts/ci-typecheck.sh`.

```sh
cd tests/integration-tests
deno ci
deno task check
```

- [x] Add a scheduled or manually triggered full HTTP integration-test job.
  - Provision an isolated non-production R2 bucket first; the suite performs
    real uploads through presigned URLs and must never target production.
- [x] Regenerate `docs/swagger.json` through a checked-in `sh` script into a
  temporary file and fail CI when the checked-in specification differs.
- [x] Add dependency security checks.
  - [x] Enable Dependabot for Cargo and GitHub Actions, targeting
    `main`.
  - [x] Run pinned `cargo-audit` through `sh scripts/ci-audit.sh`; mark this
    complete only after the first successful audit.
- [x] Pin third-party GitHub Actions to full commit SHAs.
- [x] Give every workflow an explicit minimal `permissions` block.
- [x] Configure CI concurrency so superseded branch runs are cancelled.

### Acceptance

- [ ] Required CI succeeds on the default branch.
- [ ] A deliberately malformed Rust file, stale OpenAPI file, or failing test
  prevents a pull request from merging.
- [x] CI does not require production secrets for untrusted pull requests.
  - PR #26 passed all four required checks while the production deployment job
    was skipped and no production secret was exposed to a runnable PR job.

## 7. GitHub Actions deployment and container hardening

- [x] Pin the Docker builder to the selected Rust version.
- [x] Run the application as a non-root runtime user.
- [x] Add an image-level `HEALTHCHECK` for `/api/health`.
- [x] Review the release image for unnecessary files and packages.
- [x] Use commit-qualified immutable image tags.
  - Build `poprako-server-prod:sha-<full-commit>` on the GitHub runner.
  - Upload the compressed image archive over authenticated SSH instead of
    publishing `latest` or relying on a maintainer-owned registry.
- [x] Add a `main`-only GitHub Actions deployment job.
  - Trigger deployment only after changes land on `main`; never deploy from
    a feature branch.
  - Run all required CI checks before building a release image.
  - Build and upload the image from GitHub Actions rather than a maintainer's
    machine.
  - Record the full source commit, image tag, and loaded image ID.
  - Use GitHub deployment environments for production approval and audit.
- [x] Store production secrets in a protected GitHub environment or an
  approved external secret manager.
  - Use `deploy/poprako-server/github-production-secrets.env.example` as the
    Environment-secret name and value-shape template.
  - Store all server-specific values as `production` environment secrets:
    `DEPLOY_HOST`, `DEPLOY_PORT`, `DEPLOY_USER`, `DEPLOY_SSH_PRIVATE_KEY`,
    `DEPLOY_KNOWN_HOSTS`, `DEPLOY_ROOT`, `DEPLOY_PUBLIC_PORT`,
    `DEPLOY_BIND_HOST`, `DEPLOY_DOCKER_NETWORK`, and
    `DEPLOY_POSTGRES_CONTAINER`.
  - Store every application runtime value as an independent `production`
    environment secret: `DEPLOY_DATABASE_URL`, `DEPLOY_JWT_SECRET`,
    `DEPLOY_JWT_EXPIRATION_HOURS`, `DEPLOY_R2_ACCOUNT_ID`,
    `DEPLOY_R2_ACCESS_KEY_ID`, `DEPLOY_R2_SECRET_ACCESS_KEY`,
    `DEPLOY_R2_BUCKET_NAME`, `DEPLOY_R2_REGION`,
    `DEPLOY_R2_CUSTOM_DOMAIN`, and
    `DEPLOY_POPRAKO_SNOWFLAKE_NODE_ID`.
  - The deployment script validates those values, streams them to the remote
    deployment script over SSH stdin, and passes them directly to `docker run`
    from a fixed allowlist. It does not create or upload a runtime dotenv.
    Never commit or source production runtime configuration from a maintainer
    machine.
  - Do not commit production IP addresses, SSH usernames, filesystem paths,
    Docker network names, credentials, or runtime configuration.
- [x] Define the GitHub Actions-to-runtime deployment mechanism.
  - GitHub Actions connects over SSH using a dedicated deployment account.
  - The account must already have access to Docker, the deployment root, and
    the pre-existing Docker network.
  - The remote PostgreSQL 18 container is managed independently and must
    already be healthy; application CD never creates, replaces, or restarts it.
  - Upload only the immutable image archive, deployment scripts, and
    migrations. Stream GitHub-secret-derived runtime values over the remote
    deployment command's stdin without writing a runtime dotenv.
- [x] Apply migrations as one CD-owned transaction.
  - Run it only against the independently managed, already-running PostgreSQL
    18 service; it must not manage the PostgreSQL container lifecycle.
  - Limit repeatable schema/bootstrap operations to `CREATE IF NOT EXISTS` and
    `INSERT ... ON CONFLICT` semantics.
  - `scripts/ga-apply-migrations.sh` concatenates every ordered `up.sql` between
    one `BEGIN` and `COMMIT`, then executes it with `psql` and
    `ON_ERROR_STOP=1` inside the existing PostgreSQL container.
  - Run the migration batch before replacing the application container. A
    migration failure stops deployment while the previous application
    container remains running.
  - The server binary never embeds, applies, or triggers migrations.
  - The three bootstrap inserts use primary-key `ON CONFLICT DO NOTHING` so
    repeated deployment preserves existing rows.
- [x] Remove fixed bootstrap administrator credentials from production
  migrations; keep deterministic bootstrap data only in the isolated HTTP
  integration-test fixture.
- [x] Remove the dependency on legacy production credentials at
  `/opt/poprako-s/shared/.env`.
- [x] Retire the legacy manual deployment implementation.
  - Removed `scripts/deploy-release.sh` and
    `scripts/remote-deploy-release.sh`.
  - Removed the manual release recipe from `justfile`.
  - Removed old `poprako-sr` deployment paths and names.
- [x] Make the container replacement rollback-safe.
  - Do not delete the healthy container before the replacement is healthy.
  - Preserve the previous image and runtime configuration.
  - Automatically restore the previous release when the new health check
    fails.
- [x] Define release retention and cleanup for loaded Docker images, uploaded
  image archives, and deployed revisions.
  - Remove the uploaded image archive immediately after it is loaded.
  - After a successful health check, retain only the current release and the
    previous release used for rollback.
  - Delete only older `poprako-server-prod:sha-<full-commit>` images and
    40-character commit-named release directories; never run a host-wide
    Docker prune.
- [x] Add log, metric, and alert checks to the post-deployment verification.
  - Require both the container health check and internal Prometheus counters
    before accepting the new release.
  - Reject startup logs containing an `ERROR`, panic, or fatal runtime error.
  - Roll back when verification fails, fail the deployment job, and emit a
    GitHub Actions error annotation containing only the deployed commit SHA.
- [x] Change the root Agent rule so releases must use GitHub Actions rather
  than `just deploy-release`.

### Acceptance

- [ ] A failed deployment leaves or restores a healthy previous version.
- [ ] Production deployment does not require a maintainer machine, a manually
  initiated SSH session, or a developer-owned plaintext secret file.
- [ ] A deployed commit can be mapped to its commit-qualified image ID and
  release record.

## 8. Release policy

- [x] Define the relationship between Cargo version, Git tags, container image
  tags, and GitHub Releases.
- [x] Choose Semantic Versioning for the production service.
- [x] Add `CHANGELOG.md` and generate release notes from conventional commits.
- [x] Define who may create and approve a production release.
- [x] Build release artifacts only after required CI succeeds.
- [x] Generate and retain checksums, a Cargo dependency inventory, and a build
  provenance manifest with every GitHub Release.
- [ ] Test the release process in a staging environment before the first
  organization-owned production release.

### Acceptance

- [ ] Every production deployment has a tag or release record, source commit,
  image digest, approver, and deployment result.
- [ ] The documented rollback procedure has been exercised successfully.

## 9. Transfer-day procedure

- [ ] Freeze merges and deployments for the transfer window.
- [x] Confirm the working tree is clean and all intended branches are pushed.
- [ ] Record current repository settings.
  - Default branch and merge methods.
  - Collaborators and permissions.
  - Branch protections or rulesets.
  - Webhooks, deploy keys, Actions secrets, environments, and Pages settings.
- [ ] Back up critical repository and deployment configuration.
- [x] Confirm the target organization permits repository creation and accepts
  the final repository name.
- [x] Transfer the repository through GitHub repository settings.
- [x] Do not recreate the old personal repository at the previous path; doing
  so can break GitHub's redirect.
- [x] Update local clones after transfer.

```sh
git remote set-url origin git@github.com:poprako-dev/poprako-server.git
git remote -v
```

- [ ] Update external integrations, badges, documentation links, registry
  paths, deployment automation, and bookmarks.

## 10. Organization-side verification

- [x] Keep direct organization ownership for the current single-member team;
  introduce organization teams only when multiple maintainers make them
  useful.
- [x] Apply least-privilege base permissions.
  - Organization members have read-only repository access by default.
- [x] Configure a ruleset for the default and release branches.
  - Require pull requests.
  - Require zero approvals while the organization has only one member; raise
    this when an independent reviewer joins.
  - Require resolved conversations and required CI checks.
  - Block force pushes and branch deletion.
  - Decide whether signed commits and linear history are required.
- [x] Configure protected deployment environments.
  - Restrict production deployments to approved branches or tags.
  - `production` accepts deployments only from `main`.
  - Add an independent reviewer and disable administrator bypass when another
    production-capable organization member is available.
- [ ] Re-audit webhooks, deploy keys, secrets, and GitHub App access inherited
  during transfer.
- [x] Enable the dependency graph, Dependabot alerts and updates, secret
  scanning, and push protection where available.
- [ ] Verify issue assignments, package links, branch protections, and all
  external integrations after transfer.
- [ ] Run CI and perform a staging deployment from the organization-owned
  repository.

### Acceptance

- [ ] Direct pushes to protected branches are rejected.
- [ ] A normal pull request can pass CI, receive review, and merge.
- [ ] A release can deploy through the protected environment without using a
  personal repository or personal registry namespace.

## 11. Final sign-off

- [ ] Repository owner sign-off.
- [ ] Application maintainer sign-off.
- [ ] Infrastructure/deployment owner sign-off.
- [ ] Security contact sign-off.
- [ ] First organization-owned release completed and verified.

## Current audit baseline

The repository-local checklist was re-audited on 2026-08-02.

- `cargo fmt --all --check`: passed.
- `cargo check --workspace --all-targets --all-features`: passed.
- Strict workspace/all-target Clippy with warnings denied: passed.
- `sh fmt/run-check.sh`: passed.
- `cargo test --workspace`: passed, 347 tests.
- OpenAPI and TypeScript checks: passed.
- The 39 PostgreSQL 18 migrations completed twice as a single CD transaction.
  The production migration path created no administrator or membership; the
  isolated integration fixture created both only after migrations completed.
- Pinned `cargo-audit` passed against 1,178 RustSec advisories. The audit also
  removed unused AWS configuration features, legacy Rustls, and the unused RSA
  JWT backend from the production dependency graph.
- Tracked Rust files: 501.
- Rust files above 600 lines: 15, all frozen at their current line count by
  `scripts/check-rust-lines.sh`.
- Migration `up.sql` files: 39.
- Migration `down.sql` files: 39.
- Git tags: 0.
- GitHub Actions workflows: 3.

## External execution blockers

Repository-local work is complete where marked above. Transfer-day and
organization-side checkboxes remain deliberately unchecked because they
represent external state, not files:

- GitHub CLI authentication, API access, and Git-over-SSH access to
  `poprako-dev/poprako-server` are operational.
- The organization repository, protected branches, `production` environment,
  and all required production secret names now exist. Staging deployment,
  production acceptance, and owner/security sign-offs remain outstanding.
