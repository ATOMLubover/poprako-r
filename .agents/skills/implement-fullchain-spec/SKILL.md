---
name: implement-fullchain-spec
description: Implement new or changed backend behavior across the affected domain, repository, adapter, use-case, HTTP, migration, and test contracts.
---

# Vertical-slice workflow

Change only the layers needed by the behavior. Paths below are repository
relative and refer to the server crate unless stated otherwise.

## 1. Establish behavior

Read current domain code, business references, and nearby tests. Identify
permissions, transaction boundaries, side effects, uniqueness, locking,
response shape, and negative cases. Find an operation with a comparable
execution shape.

## 2. Domain and transport types

Put shared concepts under `value`; follow
[general-conventions](../general-conventions/SKILL.md) for persisted models.
Follow [usecase-boundaries](../usecase-boundaries/SKILL.md) for pure rules and
[data-dto-boundaries](../data-dto-boundaries/SKILL.md) for changed DTOs.
Convert timestamps to Unix milliseconds at the response boundary.

## 3. Repository operations and persistence

1. Define domain-qualified descriptors in `src/part/repo/oper/<domain>.rs`
   with `#[oper(output = ...)]`. Domain capability traits live directly
   under `part::repo`; only declare contracts consumed by production use cases.
2. Add descriptors to the domain `XxxRepo<C>` capability via `#[drive(...)]`:
   `run` for independent operations, `step` for caller-owned transactions.
3. Implement `Run<Oper>` or `Step<Oper, Context>` in the required RDB and
   mock adapters. Keep Orchestra implementations beside focused SQL helpers.
4. Keep server Diesel entities under `src/part_impl/repo/rdb_impl/entity`.
   Follow [error-handling-spec](../error-handling-spec/SKILL.md) for conversion.
5. For schema changes, update matching migration `up.sql` and `down.sql`,
   apply to the intended development database, regenerate using
   `just mgr-schema`, then compile. Follow root authorization rules for
   destructive operations and the generated schema guardrail.
6. Validate migrations through `sh scripts/ci-migration-check.sh` against
   the dedicated CI database; `sh scripts/ci-local.sh` prepares that target.
   Production migrations run only through Actions using
   `scripts/ga-apply-migrations.sh`: all up files in one transaction against
   an independently managed, already-running PostgreSQL 18 container.

## 4. Use cases and side effects

Implement public generic domain orchestration under `usecase`; use
[usecase-boundaries](../usecase-boundaries/SKILL.md) for transaction ownership,
repository execution, loaders, and delivery boundaries.

Persist deferred Prom work through `Defer` or `DeferBatch` in the owning
transaction. Respect at-least-once delivery and make handlers idempotent.
Make the relationship between immediate effects and transaction completion
explicit; do not emit a success-dependent effect before successful commit.

## 5. HTTP exposure

- Validate transport-only facts in handlers; propagate use-case errors with
  `?` and build success responses through `Accept as _` or `no_content`.
- Follow [tracing-usage-spec](../tracing-usage-spec/SKILL.md) for handler spans.
- Keep router paths, extractors, `#[utoipa::path]`, and schema registration
  aligned. Business routes use `/api/v1`; health uses `/api/health`.
- The `swagger` feature exposes `/api/swagger-ui` and `/api/openapi.json`;
  `--swagger` prints the specification. Regenerate changed API contracts
  according to [docs/AGENTS.md](../../../docs/AGENTS.md).

## 6. Validate affected contracts

Follow [test-spec](../test-spec/SKILL.md) for focused positive/negative tests,
RDB coverage, HTTP suites, documentation synchronization, and validation
commands. Review the final diff for omissions across changed contracts.
