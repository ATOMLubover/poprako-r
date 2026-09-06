# ObjDept Review Issue Ledger

## Purpose

This file is the durable inventory for every issue found by the 2026-08-30
review of the ObjDept work. It is intentionally separate from the implementation
plan so later reviews can recheck findings without reconstructing them from chat
history.

- Branch: feat/obj-dept
- Review range: origin/main through the current working tree
- Remediation plan: [obj-dept-remediation.md](obj-dept-remediation.md)
- Production-only track:
  [obj-dept-production-migration.md](obj-dept-production-migration.md)

Evidence paths and symbol names are authoritative. Line numbers are only
snapshot hints and may move while fixes are implemented.

## Terminology guard

These names describe different responsibilities and must not be conflated:

- ObjPool is the physical object-storage capability. R2ObjPool is its current
  R2 adapter.
- page_image_rdb_impl is the planned readable name for the generated,
  PageImage-specific typed Diesel/RDB entry module around t_page_image. It owns
  FullRow, ObjRdbEntry, and the generated load/write/state-transition helpers.
- ObjProm is durable Check/Delete task persistence.

There is no ObjStore abstraction in this design. In particular,
__obj_dept_page_image is an RDB-entry implementation module, not an ObjPool and
not physical storage.

## Ledger rules

- Issue IDs are stable and must never be reused or deleted.
- OPEN means the current remediation must resolve the issue.
- ACCEPTED-SAFETY means behavior is deliberately retained for now and must be
  documented with a SAFETY comment.
- IMPLEMENTED-AWAITING-VALIDATION means a candidate repair exists in the
  working tree, but the issue remains unclosed until its required validation
  evidence is recorded.
- DEFERRED-FIXME means implementation is outside this repair and must remain
  visible through the required FIXME and this ledger.
- SEPARATE-TRACK means the issue is real but belongs to an explicitly separate
  plan.
- RESOLVED may be set only after recording the implementing commit or diff,
  the relevant validation result, and any remaining constraint.
- If a finding is disproved, retain it as CLOSED-NOT-REPRODUCED with the
  evidence that disproved it.

## Resolution evidence

Every issue marked RESOLVED below is implemented in the current working-tree
diff and was rechecked together on 2026-08-30. Shared validation evidence:

- `cargo fmt --all --check`: passed;
- `cargo check --all-features`: passed without warnings;
- `cargo test -p poprako-server`: 353 passed;
- `cargo test -p poprako-obj-dept -p poprako-obj-dept-macro`: passed,
  including unit, generated-manifest, RDB expansion, and doc-test targets;
- `tests/integration-tests/deno task check`: passed;
- `scripts/ci-openapi-check.sh`: passed after regenerating
  `docs/swagger.json`;
- `git diff --check`: passed;
- the ObjDept Rust `__` identifier scan is empty;
- the non-RDB `f_` identifier scan is empty; remaining matches are Diesel
  columns, RDB rows, or tests that explicitly exercise those RDB forms;
- every Rust file is below 600 lines.

The implementation-specific evidence and remaining constraints stay in each
issue's Evidence, Required disposition, and Recheck fields. In particular,
accepted safety debt, deferred actor work, separate production work, and the
no-business-ID-reuse invariant are not erased by these test results.

## Summary

| ID | Severity | Status | Finding |
| --- | --- | --- | --- |
| OBJ-Q-001 | Blocker | RESOLVED | GenObjUrls has an incomplete unnamed URL result |
| OBJ-Q-002 | Blocker | RESOLVED | Thumbnail response contract was removed |
| OBJ-Q-003 | High | RESOLVED | Optimistic mark and exact-generation correction required validation evidence |
| OBJ-Q-004 | High | ACCEPTED-SAFETY | ObjActor Check does not enforce the stored hash |
| OBJ-Q-005 | High | RESOLVED | Generated double-underscore identifiers are opaque |
| OBJ-Q-006 | High | RESOLVED | Read projection is handwritten for PageImage only |
| OBJ-Q-007 | Medium | RESOLVED | General Prom read dependency leaks raw ObjPoolView |
| OBJ-Q-008 | Medium | RESOLVED | A duplicate legacy ObjPool implementation remains |
| OBJ-Q-009 | High | RESOLVED | Macro files are at the line limit and hide long units |
| OBJ-Q-010 | High | RESOLVED | High-value tests were removed or are missing |
| OBJ-Q-011 | High | RESOLVED | Plans, specifications, and review records contradict code |
| OBJ-Q-012 | Medium | SEPARATE-TRACK | Linter changes are mixed into the branch without ObjDept ownership |
| OBJ-Q-013 | Blocker | SEPARATE-TRACK | Production cutover and image construction are incomplete |
| OBJ-Q-014 | Medium | DEFERRED-FIXME | Operator repair has no recovery operation |
| OBJ-Q-015 | Low | RESOLVED | New comments contain duplicate or stale intent |
| OBJ-Q-016 | High | RESOLVED | Actor HEAD can overwrite a newer same-generation client mark |
| OBJ-Q-017 | High | RESOLVED | Object markers lack a URL rendition profile for future Font support |
| OBJ-Q-018 | High | RESOLVED | Remove permits business-ID reuse ABA against old keys and tasks |
| OBJ-Q-019 | High | RESOLVED | Database `f_` names leak into domain models and ordinary locals |
| OBJ-Q-020 | High | RESOLVED | Lifecycle APIs used ambiguous availability and retirement vocabulary |
| OBJ-Q-021 | High | RESOLVED | obj_inst! hides operation types from Rust LSP navigation |
| OBJ-Q-022 | High | RESOLVED | Object cleanup API leaks persistence strategies into use cases |
| OBJ-P-001 | Blocker | RESOLVED | DelObjs is a per-ID loop disguised as a batch API |
| OBJ-P-002 | Blocker | RESOLVED | Page manifest persistence is N+1 and lookup is quadratic |
| OBJ-P-003 | Blocker | RESOLVED | Page image validation/reservation is N+1 |
| OBJ-P-004 | Blocker | RESOLVED | Nested View construction performs recursive N+1 reads |
| OBJ-P-005 | High | RESOLVED | collect_bounded only parallelizes the N+1 |
| OBJ-P-006 | Medium | RESOLVED | URL generation serially awaits every object |
| OBJ-P-007 | High | DEFERRED-FIXME | ObjActor processes all object topics globally serially |
| OBJ-P-008 | High | DEFERRED-FIXME | Reset maintenance runs before every claim |
| OBJ-P-009 | Medium | DEFERRED-FIXME | Claim is SELECT plus CAS rather than an atomic claim |
| OBJ-P-010 | Low | DEFERRED-FIXME | Actor dispatch clones task and adapters per attempt |
| OBJ-P-011 | Medium | RESOLVED | Hot read/delete paths perform avoidable cloning |
| OBJ-P-012 | Medium | RESOLVED | Metadata reads fetch and decode discarded timestamps |
| OBJ-P-013 | High | DEFERRED-FIXME | Completed durable tasks have no retention policy |
| OBJ-P-014 | Blocker | RESOLVED | ObjPromDefer exposes singleton task creation only |
| OBJ-P-015 | Blocker | RESOLVED | Separate anchor creation and locking permits an interleaving window |

## Quality, contract, and architecture issues

### OBJ-Q-001 — GenObjUrls has an incomplete unnamed URL result

- Severity: Blocker
- Status: RESOLVED
- Evidence:
  - poprako-obj-dept/src/oper.rs, GenObjUrls
  - poprako-obj-dept-macro/src/impl_obj_dept.rs, generated GenObjUrls Run
  - src/part_impl/obj_dept.rs, handwritten PageImage GenObjUrls Run
  - src/part_impl/obj_dept/r2_impl.rs, ObjPoolView implementation
- Finding: the operation returns HashMap<String, Url>. One bare URL cannot
  self-describe which rendition it represents and cannot carry the required
  thumbnail.
- Required disposition: add the named ObjUrls model with origin_url: Url and
  thumbnail_url: Option<Url>; return HashMap<String, ObjUrls>. R2 must currently
  return Some for the thumbnail while future pools may return None.
- Remediation: Phase 1.
- Recheck: compile-time operation-output test plus R2 and mock contract tests
  for Some, None, and URL-generation failure.

### OBJ-Q-002 — Thumbnail response contract was removed

- Severity: Blocker
- Status: RESOLVED
- Evidence:
  - src/data/view/page.rs lacks image_thumbnail_url
  - src/data/view/user.rs lacks avatar_thumbnail_url
  - src/data/view/team.rs lacks avatar_thumbnail_url
  - src/data/view/comic.rs lacks cover_thumbnail_url
  - tests/integration-tests/src/http/types.ts still declares all four fields
  - docs/plan.md says HTTP, OpenAPI, responses, and wire format do not change
- Finding: the Rust response DTOs no longer match the established TypeScript
  contract or the plan's stated no-wire-change constraint.
- Required disposition: restore all four optional thumbnail fields and update
  constructors, Swagger/OpenAPI, fixtures, and TypeScript types together.
- Remediation: Phase 1 and Phase 7.
- Recheck: serialized response tests for all four object kinds, generated
  OpenAPI diff, and integration typecheck.

### OBJ-Q-003 — Optimistic mark requires exact-generation async correction

- Severity: High
- Status: RESOLVED
- Evidence:
  - poprako-obj-dept/src/oper.rs defines MarkObjUploaded for one ObjKey
  - poprako-obj-dept-macro/src/impl_obj_dept/lifecycle.rs implements an
    exact-ID-and-version optimistic mark
  - poprako-obj-dept/src/actor/rdb_impl.rs checks both Unavailable and Available
    generations and writes the HEAD result through exact-version predicates
  - src/usecase/page.rs, src/usecase/comic/cover.rs, src/usecase/team.rs, and
    src/usecase/user.rs invoke MarkObjUploaded and preserve their established
    stale-version errors
- Finding: the intended contract is an optimistic client declaration, not a
  read-only generation check. A successful endpoint must immediately expose
  origin and supported thumbnail URLs for the exact current generation. The
  delayed actor must HEAD that same generation and reset it to unuploaded when
  absent. Neither transition may change a newer generation.
- Resolution note: the operation, four endpoint integrations, immediate URL
  visibility, exact-generation actor correction, and same-generation revision
  race are covered by the shared validation evidence above.
- Required disposition: validate all four endpoint integrations and retain the
  exact-generation actor correction. Keep an English SAFETY comment stating
  that the optimistic mark does not prove PUT success or content integrity.
- Remediation: Phase 6.
- Recheck: all four endpoints invoke MarkObjUploaded; stale versions fail;
  successful marks immediately produce origin and current-pool thumbnail URLs;
  a delayed absent Check resets only the same generation; a stale Check cannot
  modify a newer generation; no documentation claims hash verification.

### OBJ-Q-004 — ObjActor Check does not enforce the stored hash

- Severity: High
- Status: ACCEPTED-SAFETY
- Evidence:
  - src/part_impl/obj_dept/r2_impl.rs, gen_slot binds content type and length
    but no checksum header
  - poprako-obj-dept/src/actor/rdb_impl.rs, pending Check uses ObjPool::has
  - docs/image-consistency.md claims checksum binding and exact hash checking
- Finding: object presence is treated as upload evidence; the persisted hash is
  not compared with remote bytes or metadata. The current consistency document
  overstates the guarantee.
- Required disposition: do not add remote hash verification in this repair
  because of the accepted upload-throughput cost. Add one precise English
  SAFETY comment at the Check presence decision. Track the documentation
  mismatch without claiming integrity that is not implemented.
- Remediation: Phase 6.
- Recheck: behavior is unchanged, the SAFETY comment is present, and no updated
  documentation claims checksum enforcement.

### OBJ-Q-005 — Generated double-underscore identifiers are opaque

- Severity: High
- Status: RESOLVED
- Evidence:
  - poprako-obj-dept-macro/src/object.rs generates __obj_dept_<marker>,
    __obj_dept_unique, and __objs_manifest
  - poprako-obj-dept-macro/src/impl_obj_dept.rs generates callback, helper, and
    department module names with the same prefix
  - poprako-obj-dept-macro/src/rdb_obj_prom.rs generates another opaque module
  - poprako-obj-dept/src/actor/rdb_impl.rs exports __obj_handle
  - src/part_impl/obj_dept.rs and poprako-obj-dept/tests/rdb_obj.rs consume the
    generated names directly
- Finding: punctuation is being used as pseudo-hygiene, obscuring actual
  responsibility. The worst example, __obj_dept_page_image, is specifically
  the generated typed PageImage RDB-entry implementation, not an object pool.
- Required disposition: remove every ObjDept-related Rust identifier beginning
  with __. Rename the PageImage module to page_image_rdb_impl and apply the
  complete responsibility-based mapping in the remediation plan.
- Remediation: cross-cutting naming requirement.
- Recheck:

      rg -n --pcre2 '\b__[A-Za-z][A-Za-z0-9_]*' \
        --glob '*.rs' src poprako-obj-dept poprako-obj-dept-macro poprako-rdb-core

  The scan must be empty. PostgreSQL's external
  __diesel_schema_migrations table name is excluded.

### OBJ-Q-006 — Read projection is handwritten for PageImage only

- Severity: High
- Status: RESOLVED
- Evidence:
  - src/part_impl/obj_dept.rs implements Run and Step ListObjMetas<PageImage>
    and Run GenObjUrls<PageImage> manually
  - poprako-obj-dept-macro/src/impl_obj_dept.rs generates the same operations
    only for the total department
- Finding: adding Font would require discovering and copying PageImage-specific
  forwarding code. This defeats the manifest's promise that registering a new
  object marker is mechanical and creates two implementations that can drift.
- Required disposition: have impl_obj_dept generate read-projection operations
  for every manifest marker and remove the handwritten PageImage block.
- Remediation: Phase 2.
- Recheck: a two-marker macro expansion test proves both total-department and
  read-projection implementations exist without marker-specific forwarding.

### OBJ-Q-007 — General Prom read dependency leaks raw ObjPoolView

- Severity: Medium
- Status: RESOLVED
- Evidence: src/part_impl/prom/rdb_impl/actor/base.rs declares ObjView as
  ObjPoolView plus ObjDeptView<PageImage, RdbContext>.
- Finding: the general Prom actor needs resolved PageImage data, not raw
  physical-pool access. The extra supertrait exposes storage operations and
  couples a consumer to an implementation detail.
- Required disposition: reduce ObjView to the required ObjDeptView capability
  and remove public physical-pool forwarding from NormObjView.
- Remediation: Phase 2.
- Recheck: no general Prom module imports ObjPoolView and all actor tests compile.

### OBJ-Q-008 — A duplicate legacy ObjPool implementation remains

- Severity: Medium
- Status: RESOLVED
- Evidence:
  - active adapter: src/part_impl/obj_dept/r2_impl.rs
  - duplicate tree: src/part_impl/obj_pool.rs and
    src/part_impl/obj_pool/r2_impl.rs
  - src/part_impl.rs does not register the duplicate module
- Finding: two near-identical R2 implementations make it unclear which code is
  authoritative and create a high risk of fixing only one copy.
- Required disposition: after confirming zero active imports, remove the
  unregistered legacy tree.
- Remediation: Phase 6.
- Recheck: only the active ObjDept R2ObjPool definition remains and the full
  compile/test suite passes.

### OBJ-Q-009 — Macro files are at the line limit and hide long units

- Severity: High
- Status: RESOLVED
- Evidence:
  - poprako-obj-dept-macro/src/object.rs: 598 lines
  - poprako-obj-dept-macro/src/rdb_obj_prom.rs: 594 lines
  - poprako-obj-dept-macro/src/impl_obj_dept.rs: 557 lines
  - object.rs expand_load is already large enough to trigger Clippy's
    too_many_lines when warnings are denied
  - rdb_obj_prom.rs suppresses too_many_lines for its top-level expansion
- Finding: required batch and read-generation work cannot fit cleanly without
  crossing the 600-line project limit, and long expansion functions mix
  unrelated responsibilities.
- Required disposition: use only the exact responsibility-based module splits
  listed in Phase 0. Per module-splitting rules, obtain confirmation before
  implementation.
- Remediation: Phase 0.
- Recheck: every Rust file remains below 600 lines, the module-split audit
  passes, and no broad too_many_lines suppression remains for newly split code.

### OBJ-Q-010 — High-value tests were removed or are missing

- Severity: High
- Status: RESOLVED
- Evidence:
  - branch diff deletes large page, avatar, cover, mock-repository, and legacy
    image test suites while the replacement ObjDept crate has only a small
    operation/macro test set
  - no focused RDB tests cover multi-object locking, batch task collisions,
    rollback, actor restart, stale lease fencing, or operator state
  - tests/integration-tests/src/suites/it_11_comic_archive.ts still reads
    removed t_comic.f_cover_key and t_page.f_image_key columns
  - tests/integration-tests/TESTCASES.md was not updated for the behavioral
    replacement
- Finding: current passing unit counts do not cover the risk transferred into
  generated RDB code, object lifecycle state, or changed HTTP output.
- Required disposition: restore behavior coverage at the appropriate
  ObjDept/RDB/use-case/HTTP layers and repair the stale archive integration
  assertion.
- Remediation: Phase 7.
- Recheck: run the test matrix in Phase 7 and record explicit batch query-count,
  rollback, URL-contract, and nested hydration evidence.

### OBJ-Q-011 — Plans, specifications, and review records contradict code

- Severity: High
- Status: RESOLVED
- Evidence:
  - plans/obj-dept.md says existing Page traffic is not cut over
  - plans/obj-dept-review.md repeats that scope and records every check as
    passing with no blocker
  - the current branch routes Page, UserAvatar, TeamAvatar, and ComicCover
    through ObjDept
  - docs/plan.md forbids response/OpenAPI changes while thumbnail fields are
    absent
  - docs/image-consistency.md promises checksum behavior not implemented by
    the R2/Check path
- Finding: the historical review record cannot be used as current acceptance
  evidence, and implementation intent is no longer self-describing.
- Required disposition: mark obsolete records as superseded or update them only
  after new evidence exists. Keep accepted limitations explicit.
- Remediation: Phase 6 and Phase 7.
- Recheck: every active document agrees on current traffic, URL shape, hash
  guarantee, validation results, and deferred scope.

### OBJ-Q-012 — Linter changes are mixed into the branch without ObjDept ownership

- Severity: Medium
- Status: SEPARATE-TRACK
- Evidence: origin/main..feat/obj-dept includes rust-style-lint.toml, linters,
  linters-extra, and scripts/check-rust-lines.sh changes unrelated to the
  ObjDept runtime behavior.
- Finding: ownership of those changes is not established by the ObjDept repair,
  and repository instructions prohibit modifying linters without an explicit
  user request.
- Required disposition: preserve the changes, do not edit them during ObjDept
  remediation, and have their author decide whether they belong in this branch
  or a separate change.
- Remediation: outside the ObjDept repair.
- Recheck: record the ownership decision; do not silently revert user work.

### OBJ-Q-013 — Production cutover and image construction are incomplete

- Severity: Blocker for production only
- Status: SEPARATE-TRACK
- Evidence:
  - historical create-table migrations were edited without an independent
    production backfill/cutover
  - existing physical R2 keys require copying to the new namespace grammar
  - the production Dockerfile omits new path workspace members
  - plans/obj-dept-production-migration.md contains the detailed evidence and
    runbook
- Finding: the branch is not independently deployable to current production
  data, but the user explicitly excluded production switching from this
  remediation.
- Required disposition: no production, migration, Docker, release, or
  deployment work in the current repair. Track it exclusively in the
  production migration plan.
- Remediation: separate production track.
- Recheck: completion is governed by that plan and must not be inferred from
  ObjDept unit checks.

### OBJ-Q-014 — Operator repair has no recovery operation

- Severity: Medium
- Status: DEFERRED-FIXME
- Evidence: poprako-obj-dept/src/prom.rs exposes mark_task_operator but no
  inspected transition for repairing/requeueing an operator task.
- Finding: an operator-marked object task is durable but has no application
  recovery path.
- Required disposition: do not design or implement actor recovery in this
  repair. Include it in the single required ObjActor FIXME.
- Remediation: Phase 6 FIXME only.
- Recheck: FIXME explicitly names operator recovery; ledger remains deferred
  until a separate actor design is approved.

### OBJ-Q-015 — New comments contain duplicate or stale intent

- Severity: Low
- Status: RESOLVED
- Evidence:
  - src/usecase/page/reserve.rs repeats the apply_manifest summary twice
  - src/data/view/page.rs places two competing summaries above from_model
  - several active plans describe an earlier PageImage-only slice
- Finding: these comments make already complex orchestration harder to audit
  and reduce implementation-intent clarity.
- Required disposition: remove duplicate comments and keep one accurate
  responsibility statement per item while repairing the touched modules.
- Remediation: Phase 6 cleanup.
- Recheck: targeted source review finds no duplicated or obsolete summaries in
  the changed ObjDept paths.

### OBJ-Q-016 — Actor HEAD can overwrite a newer same-generation client mark

- Severity: High
- Status: RESOLVED
- Evidence:
  - poprako-obj-dept/src/actor/rdb_impl.rs performs remote HEAD after reading
    the current object state
  - poprako-obj-dept-macro/src/impl_obj_dept/lifecycle.rs allows a client to
    optimistically mark the same generation while that HEAD is in flight
  - poprako-obj-dept-macro/src/object/rdb_entry.rs contains the candidate
    revision-fenced state updates
- Finding: an exact-version predicate alone does not order two writes to the
  same generation. Without an additional revision fence, a missing result
  from an earlier HEAD can reset availability after a later successful client
  mark, violating immediate availability for the newest declaration.
- Implementation note: the working tree captures the initial row revision and
  applies the actor result only when both generation and revision still match.
  Deterministic core tests cover a same-generation mark interleaving and a
  newer-generation replacement; both pass in the shared validation run.
- Required disposition: retain a same-generation revision/CAS fence around
  actor correction. A lost race must reload and classify state without
  overwriting the later mark.
- Remediation: Phase 6 and Phase 7.
- Recheck: deterministic tests pause HEAD, perform a client mark for the same
  generation, then release HEAD and prove the actor cannot revoke that mark;
  absent HEAD without an intervening write still resets availability.

### OBJ-Q-017 — Object markers lack a URL rendition profile for future Font support

- Severity: High
- Status: RESOLVED
- Evidence:
  - poprako-obj-dept/src/model/url.rs exposes one optional thumbnail slot
  - src/part_impl/obj_dept/r2_impl.rs applies the image thumbnail transform
    without a marker-specific rendition policy
  - the object manifest records marker, table, topic, and namespace but no URL
    profile
- Finding: all current markers are images, so one shared thumbnail rule appears
  sufficient. A future Font marker must not inherit image resizing merely by
  registering with ObjDept, and handwritten marker branching would defeat the
  mechanical extension goal.
- Required disposition: add an explicit compile-time URL/rendition profile to
  each manifest marker. The profile must distinguish image thumbnails from
  origin-only objects and reach generated URL resolution without runtime
  object-kind string dispatch.
- Remediation: Phase 1 and Phase 2.
- Recheck: a compile-test Font-like marker selects an origin-only profile while
  image markers retain optional thumbnails, with no handwritten marker branch.

### OBJ-Q-018 — Remove permits business-ID reuse ABA against old keys and tasks

- Severity: High
- Status: RESOLVED
- Evidence:
  - poprako-obj-dept/src/oper.rs, DeleteObjs ends the object identity and
    documents the no-ID-reuse invariant
  - generated slot allocation starts an absent row from its initial generation
  - physical keys and durable task identities include business ID and version
- Finding: deleting the row also deletes its generation watermark. Reusing the
  same business ID can allocate an old version again, allowing an earlier
  physical key or delayed durable task to become indistinguishable from the
  newly created logical object.
- Required disposition: publish and enforce the invariant that a business ID
  is never reused within one object topic. If every caller cannot enforce that
  invariant, Remove must retain a durable watermark or introduce a monotonic
  epoch so a reused ID cannot recreate an earlier logical/physical key.
- Remediation: Phase 3 and Phase 6.
- Recheck: public contracts state the invariant and every Remove caller proves
  non-reuse, or an RDB test proves retained watermark/epoch allocation remains
  monotonic after removal and recreation.

### OBJ-Q-019 — Database `f_` names leak into domain models and ordinary locals

- Severity: High
- Status: RESOLVED
- Evidence:
  - poprako-obj-dept/src/model/meta.rs previously exposed
    ObjMeta.f_is_uploaded
  - use cases, mock adapters, actor helpers, and proc-macro parsing helpers used
    ordinary identifiers such as f_marked, f_exists, and f_word_boundary
  - `f_` is the established physical-column convention in Diesel schema and
    RDB row mappings, not a domain naming convention
- Finding: leaking a storage prefix into the public metadata model and normal
  control-flow variables obscures responsibility and makes persistence naming
  appear to be a repository-wide semantic convention.
- Implementation note: the working tree uses ObjMeta.is_available and ordinary
  semantic local names while retaining `f_` only for actual Diesel columns and
  RDB row mappings. The targeted identifier scans, compile, and tests pass.
- Required disposition: keep domain and ordinary Rust identifiers free of the
  `f_` prefix. Preserve it only where the identifier directly represents an
  existing database column or typed RDB row field.
- Remediation: cross-cutting naming cleanup and Phase 7.
- Recheck: targeted source scans find no `f_` identifier outside schema/table
  tokens, generated RDB row mappings, and direct SQL integration row structs;
  format, compile, and tests pass.

### OBJ-Q-020 — Lifecycle APIs use ambiguous state and retirement vocabulary

- Severity: High
- Status: RESOLVED
- Evidence:
  - ObjKeyState::Verified names optimistic client availability as verification
  - DelObjs combined cleanup for active and deleted business entities
- Finding: the state and retirement names overstate guarantees and conceal
  materially different state transitions. The attempted
  `MarkObjUploadedOutcome` repair introduced a nonstandard `Outcome` type that
  did not improve this codebase's vocabulary.
- Required disposition: keep `MarkObjUploaded` returning `bool`, bind it to the
  semantic local `marked`, and match `true`/`false` at the immediate use-case
  boundary. Rename the optimistic state so it does not claim verification, and
  split cleanup operations by business lifecycle intent.
- Remediation: Phase 6.
- Recheck: no `Outcome` lifecycle type remains, no active identifier claims
  content verification, and cleanup call sites express business lifecycle
  intent.

### OBJ-Q-021 — obj_inst! hides operation types from Rust LSP navigation

- Severity: High
- Status: RESOLVED
- Evidence:
  - poprako-obj-dept/src/lib.rs defined `obj_inst!`, whose invocation tokens
    mentioned names such as `MarkObjUploaded` without resolving them as Rust
    paths in the calling module
  - use-case callers therefore had no import or navigable reference to the
    operation structure they were invoking
- Finding: the macro existed only to inject `PhantomData`, but made
  go-to-definition and ordinary type discovery fail at every call site.
- Required disposition: delete `obj_inst!`; make marker fields private; expose
  ordinary associated constructors on each operation; import and invoke the
  real operation types directly at every caller.
- Remediation: Phase 6.
- Recheck: repository scans contain no `obj_inst` reference; all operation call
  sites compile through `Type::<Marker>::new(...)`; format, compile, and tests
  pass.

### OBJ-Q-022 — Object cleanup API leaks persistence strategies into use cases

- Severity: High
- Status: RESOLVED
- Evidence:
  - `RetireObjs` grouped two business transitions into one mode enum
  - `PreserveWatermarks` and `RemoveRows` named RDB persistence choices at
    application call sites
- Finding: callers should decide whether the owning business entity remains
  active or has ended its lifecycle. They must not select how ObjDept persists
  generation state.
- Required disposition: replace the enum with independent `ClearObjs<B>` and
  `DeleteObjs<B>` operations. `ClearObjs` is used when the business entity stays
  active and may receive another file; `DeleteObjs` is used only when the
  owning entity is permanently deleted and its identifier cannot be reused.
  Keep detach/remove mechanics private to generated RDB code.
- Remediation: Phase 6.
- Recheck: Rust source contains no `RetireObjs`, `PreserveWatermarks`, or
  `RemoveRows`; both operations remain batched and transaction-scoped; compile
  and focused lifecycle tests pass.

## Performance issues

### OBJ-P-001 — DelObjs is a per-ID loop disguised as a batch API

- Severity: Blocker
- Status: RESOLVED
- Evidence: poprako-obj-dept-macro/src/impl_obj_dept.rs, generated DelObjs Step,
  copies and deduplicates IDs, then performs lock, load, task defer, and
  detach/remove separately for every ID.
- Finding: database round trips grow linearly with object count. Since one
  defer_task itself performs INSERT plus SELECT, one active ID costs at least
  several statements inside one long transaction.
- Required disposition: normalize once, acquire typed batch locks, load all
  rows for update once, batch-create tasks, and detach/remove with one
  statement.
- Remediation: Phase 3.
- Recheck: an adapter/query counter proves statement count is bounded
  independently of ID count, including duplicate and missing IDs.

### OBJ-P-002 — Page manifest persistence is N+1 and lookup is quadratic

- Severity: Blocker
- Status: RESOLVED
- Evidence:
  - src/usecase/page/reserve.rs loops over every page
  - upsert_page performs existing_page_infos.iter().find for each retained page
  - UpdatePageManifest is issued separately for each retained page
  - CreatePages is a separate path after the per-item updates
  - validation allows a manifest of up to 200 pages
- Finding: retained-page matching is O(N squared), and manifest writes perform
  O(N) SQL statements in a transaction.
- Required disposition: index existing pages once and introduce one typed
  ApplyPageManifest batch upsert that handles retained and new pages together.
- Remediation: Phase 4.
- Recheck: a mixed 200-page test has constant manifest SQL count and preserves
  order, counters, validation, and atomicity.

### OBJ-P-003 — Page image validation/reservation is N+1

- Severity: Blocker
- Status: RESOLVED
- Evidence: src/usecase/page/reserve.rs calls reserve_page_obj in a second
  per-page loop; retained pages call singleton ListObjMetas and changed/new
  pages call singleton GenObjSlot.
- Finding: a 200-page manifest can issue hundreds of object metadata, lock,
  latest-row, task, and write statements. This is a nested transaction-level
  N+1 and can exceed one thousand SQL statements in a worst-case mixed
  manifest.
- Required disposition: batch-load retained metadata once and add GenObjSlots
  for all changed/new images, using batch state and task persistence.
- Remediation: Phase 3 and Phase 4.
- Recheck: one metadata step and one slot-reservation step per PageImage marker,
  independent of page count.

### OBJ-P-004 — Nested View construction performs recursive N+1 reads

- Severity: Blocker
- Status: RESOLVED
- Evidence:
  - src/usecase/assignment.rs maps assignment_info_view per assignment
  - src/usecase/chapter.rs maps chapter_info_view per chapter
  - src/usecase/comic/list.rs resolves nested assignment, chapter, comic, team,
    and user Views through per-record async constructors
  - src/usecase/assignment/view.rs, chapter/view.rs, comic/view.rs,
    user/view.rs, and team/view.rs recursively invoke ObjDept
- Finding: one top-level list can repeat ListObjMetas and GenObjUrls for each
  root and nested record. Repeated users/teams are not reused.
- Required disposition: follow the algorithm in
  src/part_impl/repo/rdb_impl/incl/framework.rs: walk the full included graph,
  collect and deduplicate IDs, batch-load once per marker, then synchronously
  inject/render Views.
- Remediation: Phase 5.
- Recheck: ObjDept metadata query count is at most the number of distinct
  requested object markers, regardless of row count or include depth.

### OBJ-P-005 — collect_bounded only parallelizes the N+1

- Severity: High
- Status: RESOLVED
- Evidence:
  - src/usecase/internal/util.rs sets concurrency to 20
  - assignment.rs, chapter.rs, and comic/list.rs use it around per-record View
    construction
- Finding: limiting 20 simultaneous futures prevents unlimited fan-out but
  does not reduce query count. It can pressure the DB pool with 20 concurrent
  copies of the same logical lookup.
- Required disposition: remove per-record async View work after snapshot
  hydration. Concurrency is appropriate only across the small number of
  independent marker batches.
- Remediation: Phase 5.
- Recheck: no list/View call site wraps per-record ObjDept work in
  collect_bounded.

### OBJ-P-006 — URL generation serially awaits every object

- Severity: Medium
- Status: RESOLVED
- Evidence:
  - generated GenObjUrls in
    poprako-obj-dept-macro/src/impl_obj_dept.rs awaits gen_url in a for loop
  - handwritten PageImage GenObjUrls in src/part_impl/obj_dept.rs duplicates
    the same loop
- Finding: current R2 URL construction is local, so immediate cost is small,
  but the generic pool contract is asynchronous and a future adapter can make
  the operation linearly latency-bound.
- Required disposition: resolve independent physical keys with bounded
  operation-level concurrency and retain deterministic error propagation.
- Remediation: Phase 1.
- Recheck: a delayed mock proves bounded parallelism and no per-View task
  spawning.

### OBJ-P-007 — ObjActor processes all object topics globally serially

- Severity: High
- Status: DEFERRED-FIXME
- Evidence: poprako-obj-dept/src/actor.rs, run_actor, claims one task and fully
  completes its remote attempt before claiming another.
- Finding: one slow Check/Delete causes head-of-line blocking for PageImage,
  UserAvatar, TeamAvatar, ComicCover, and a future Font topic.
- Required disposition: no behavior change in this repair, per user decision.
  Record bounded parallel claim/processing in the single actor FIXME.
- Remediation: Phase 6 FIXME only.
- Recheck: ledger remains deferred until a separately approved actor design.

### OBJ-P-008 — Reset maintenance runs before every claim

- Severity: High
- Status: DEFERRED-FIXME
- Evidence:
  - poprako-obj-dept/src/actor.rs calls reset_tasks before every claim
  - generated reset_tasks in
    poprako-obj-dept-macro/src/rdb_obj_prom.rs executes three UPDATE statements
  - idle polling repeats every five seconds
- Finding: an idle actor executes about 69,120 SQL statements per day from
  three reset UPDATEs plus one claim SELECT each poll. With backlog, the reset
  cost is also paid before every task without the idle delay.
- Required disposition: no actor behavior change in this repair. Record
  independent reset maintenance in the actor FIXME.
- Remediation: Phase 6 FIXME only.
- Recheck: deferred until reset cadence is designed separately.

### OBJ-P-009 — Claim is SELECT plus CAS rather than an atomic claim

- Severity: Medium
- Status: DEFERRED-FIXME
- Evidence: generated claim_task in
  poprako-obj-dept-macro/src/rdb_obj_prom.rs first selects the oldest row, then
  separately updates it with status/lease predicates; it does not use an
  atomic locked claim or SKIP LOCKED.
- Finding: additional actors would collide on the same oldest row. Losing
  claimers return no task even if backlog remains, which prevents clean
  horizontal scaling.
- Required disposition: no actor/claim change in this repair. Keep the scaling
  gap in the actor FIXME and this ledger.
- Remediation: Phase 6 FIXME only.
- Recheck: deferred until the actor concurrency model is approved.

### OBJ-P-010 — Actor dispatch clones task and adapters per attempt

- Severity: Low
- Status: DEFERRED-FIXME
- Evidence:
  - poprako-obj-dept/src/actor.rs passes task.clone() to the handler
  - src/part_impl/obj_dept.rs clones RdbCore and R2ObjPool inside the handler
    closure for every task
- Finding: cloning shared handles is cheap, but R2ObjPool also owns strings and
  the copies are unnecessary under a serial actor.
- Required disposition: do not refactor actor clones in this repair. Consider
  borrowed task dispatch or shared Arc state with the separate actor redesign.
- Remediation: deferred actor scope.
- Recheck: ledger remains deferred until actor work is authorized.

### OBJ-P-011 — Hot read/delete paths perform avoidable cloning

- Severity: Medium
- Status: RESOLVED
- Evidence:
  - src/usecase/team/view.rs clones team_id into a one-element slice
  - src/usecase/user/view.rs clones user_id into a one-element slice
  - generated DelObjs starts with ids.to_vec
  - generated load_many clones row.id and decode_row then owns another ID for
    ObjMeta.key
  - poprako-obj-dept/src/rdb_impl.rs next_version clones ObjRdbRow solely for
    validation
- Finding: some ownership is required for response maps, but the current path
  performs intermediate copies that can be removed or consumed once.
- Required disposition: use slice::from_ref for singleton reads, eliminate the
  deletion copy through batch normalization, and consume or borrow decoded row
  data without duplicate ID/row clones where practical.
- Resolution note: singleton view and cleanup calls now borrow their model
  IDs with `slice::from_ref`; page view assembly looks up by the borrowed model
  ID before consuming the model; bulk slot generation moves prior keys into
  Delete work with `Option::take` and keys its temporary pool-slot map by
  borrowed `&str`; the mock slot path moves the replaced row instead of cloning
  the whole record. Remaining lifecycle key copies create independent owned
  values required simultaneously by returned slots, durable Check tasks, or
  owned response maps. The actor's retry clone remains explicitly deferred by
  OBJ-P-010 and the required actor FIXME.
- Remediation: Phase 3, Phase 5, and Phase 6 cleanup.
- Recheck: Clippy passes with denied warnings and focused review records any
  clone that remains structurally required.

### OBJ-P-012 — Metadata reads fetch and decode discarded timestamps

- Severity: Medium
- Status: RESOLVED
- Evidence: poprako-obj-dept-macro/src/object.rs generated FullRow selects
  f_created_at and f_updated_at for load/load_many, destructures them, and
  immediately drops them.
- Finding: every hot metadata read transfers and decodes columns that do not
  contribute to ObjMeta or lifecycle decisions.
- Required disposition: use the narrowest typed projection required by
  metadata/state reads while retaining compile-time schema checks separately.
- Remediation: Phase 3 generated RDB-entry helpers.
- Recheck: generated read projection excludes unused timestamps and typed
  Diesel compilation/tests remain green.

### OBJ-P-013 — Completed durable tasks have no retention policy

- Severity: High
- Status: DEFERRED-FIXME
- Evidence: ObjProm supports completion but no purge/retention operation was
  found; completed rows remain in t_obj_prom_task and its indexes.
- Finding: task table and index size grow monotonically, increasing maintenance
  and lookup cost over time.
- Required disposition: no purge behavior in this repair. Include completed
  task retention in the single actor FIXME.
- Remediation: Phase 6 FIXME only.
- Recheck: deferred until retention, audit needs, and purge scheduling are
  designed together.

### OBJ-P-014 — ObjPromDefer exposes singleton task creation only

- Severity: Blocker
- Status: RESOLVED
- Evidence:
  - poprako-obj-dept/src/prom.rs exposes only defer_check and defer_delete
  - generated defer_task in
    poprako-obj-dept-macro/src/rdb_obj_prom.rs performs one INSERT and one
    identity SELECT for one task
- Finding: even if callers batch object-row work, the current port forces
  task-persistence N+1 and prevents a constant-round-trip GenObjSlots/DelObjs
  implementation.
- Required disposition: add typed batch Check/Delete deferral, use one bulk
  insert plus one bulk identity/status read, and keep singleton operations as
  thin wrappers where useful.
- Remediation: Phase 3.
- Recheck: task statement count is constant for one or many objects and all
  identity conflicts roll back the caller transaction.

### OBJ-P-015 — Separate anchor creation and locking permits an interleaving window

- Severity: Blocker
- Status: RESOLVED
- Evidence:
  - the initial batch-lock design proposed inserting missing anchor rows and
    then loading them through a separate `FOR UPDATE` statement
  - poprako-obj-dept-macro/src/object/rdb_entry.rs now contains the candidate
    one-statement conflict-update-and-returning anchor acquisition
  - poprako-obj-dept-macro/src/impl_obj_dept/lifecycle.rs consumes those rows
    as the reservation lock boundary
- Finding: creating missing anchors in one statement and locking them in a
  later statement is not an atomic lock acquisition. A concurrent transaction
  can interleave between those operations, so that design would not establish
  the required lifecycle serialization boundary.
- Implementation note: the working tree uses one typed bulk
  `INSERT ... ON CONFLICT DO UPDATE ... RETURNING` statement so existing and
  missing anchors are acquired through the same statement. Sorted inputs,
  one-statement expansion, concrete-table Diesel compilation, watermark
  retention, and removal of the second load are covered by macro/core tests
  and the all-features check.
- Required disposition: keep anchor creation and row-lock acquisition in one
  typed database statement, with normalized deterministic ID ordering and no
  separate insert-then-lock fallback.
- Remediation: Phase 3 and Phase 7.
- Recheck: concurrent RDB tests cover overlapping existing/missing ID sets,
  opposite caller order, rollback, and prove no partial lifecycle transition
  or ordering regression.

## Verified areas without an open issue

The review also checked these areas so later reviewers do not need to infer
whether they were omitted:

- The crate dependency direction remains downward:
  poprako-rdb-core to poprako-obj-dept to poprako-server, with the proc macro
  emitting paths rather than adding a runtime registry.
- No raw SQL or runtime object-kind handler registry was found in the ObjDept
  implementation.
- Current Instr, Val, and View category placement does not introduce a DTO
  dependency inversion. ObjUrls belongs in poprako-obj-dept model, not
  src/data.
- The active use-case function naming audit found no list/_by_ convention
  violation.
- Transaction ownership remains in use cases; complex code does not drive
  transactions.

## Review baseline

The following snapshot was observed before remediation:

- PASS: cargo fmt --all --check
- PASS: cargo check --all-features
- PASS: cargo test -p poprako-server --lib, 322 tests
- PASS: cargo test -p poprako-obj-dept --all-features, 7 tests
- PASS: cargo test -p poprako-obj-dept-macro, 1 test
- PASS: scripts/check-rust-lines.sh
- PASS: tests/integration-tests deno task check
- PASS: git diff --check
- PASS: use-case naming audit, 134 public functions and no violations
- FAIL: cargo clippy --all-targets --all-features reports avoidable singleton
  slice clones; denied warnings additionally expose the oversized expand_load
  unit
- NOT RUN: database/R2 integration behavior was not used as acceptance evidence
- NOT RUN: Docker, release, deployment, and production cutover operations are
  prohibited for this review scope

## Closure checklist

Before declaring this ledger resolved:

1. Update each OPEN issue independently with implementation and validation
   evidence.
2. Verify every ACCEPTED-SAFETY item has the exact required comment and no
   accidental behavior claim.
3. Verify every DEFERRED-FIXME item is represented by the one focused actor
   FIXME and remains visible here.
4. Keep production and linter ownership issues in their separate tracks.
5. Re-run the complete Phase 7 validation matrix.
6. Re-audit the final diff for new N+1 loops, per-item awaits, unnecessary
   clones, and new double-underscore identifiers.
