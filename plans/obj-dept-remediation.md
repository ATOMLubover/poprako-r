# ObjDept Review Remediation Plan

## Purpose

> Implementation complete: all actionable issues in this plan passed the
> validation recorded in `obj-dept-review-issues.md`. Deferred, separate-track,
> and accepted-safety items retain their original dispositions.

This plan addresses the actionable quality and performance findings from the
current ObjDept review. It prepares the existing image object types for a
future Font object type without introducing runtime object-kind dispatch.

The durable finding inventory is
[obj-dept-review-issues.md](obj-dept-review-issues.md). Implementation work must
close issues by stable ID in that ledger; findings must not be silently removed
from this plan.

The implementation must preserve the static marker and manifest design:

```text
poprako-rdb-core <- poprako-obj-dept <- poprako-server
                                      <- poprako-obj-dept-macro
```

## Issue coverage

| Plan area | Ledger issues |
| --- | --- |
| Phase 0: macro splits | OBJ-Q-009 |
| Generated identifier cleanup | OBJ-Q-005 |
| Phase 1: URL contract | OBJ-Q-001, OBJ-Q-002, OBJ-Q-017, OBJ-P-006 |
| Phase 2: generated read projection | OBJ-Q-006, OBJ-Q-007, OBJ-Q-017 |
| Phase 3: object/task batches | OBJ-Q-018, OBJ-P-001, OBJ-P-003, OBJ-P-011, OBJ-P-012, OBJ-P-014, OBJ-P-015 |
| Phase 4: page manifest batch | OBJ-P-002, OBJ-P-003 |
| Phase 5: nested hydration | OBJ-P-004, OBJ-P-005, OBJ-P-011 |
| Phase 6: safety debt and cleanup | OBJ-Q-003, OBJ-Q-004, OBJ-Q-008, OBJ-Q-011, OBJ-Q-014, OBJ-Q-015, OBJ-Q-016, OBJ-Q-018, OBJ-Q-019, OBJ-Q-020, OBJ-Q-021, OBJ-P-007, OBJ-P-008, OBJ-P-009, OBJ-P-010, OBJ-P-013 |
| Phase 7: verification | OBJ-Q-002, OBJ-Q-009, OBJ-Q-010, OBJ-Q-016, OBJ-Q-019, OBJ-P-015 and every actionable issue's recheck |
| Separate tracks | OBJ-Q-012, OBJ-Q-013 |

## Locked decisions and non-goals

The following decisions are not reopened by this plan:

- Production data cutover, historical migration repair, R2 object movement,
  Docker release construction, and deployment workflow changes are handled by
  a separate process.
- `mark-uploaded` optimistically marks only the submitted exact current
  generation uploaded without a synchronous R2 lookup. The delayed Check
  corrects that same generation back to unuploaded when remote presence is
  absent and must never change a newer generation.
- ObjDept Check continues to treat remote object presence as sufficient
  evidence. It will not enforce or re-read the stored content hash in this
  change because doing so has an unacceptable upload-throughput cost.
- The globally serial ObjActor, reset cadence, claim algorithm, operator
  repair workflow, and completed-task retention are deferred. This plan only
  records one focused `FIXME` at the actor loop.
- No Font marker, table, endpoint, or use case is added in this change. The
  result must make a later Font registration mechanical.

## Required behavior after the repair

1. Every `GenObjUrls<B>` entry is a named `ObjUrls` value containing:

   ```rust
   pub struct ObjUrls {
       pub origin_url: Url,
       pub thumbnail_url: Option<Url>,
   }
   ```

2. R2 always returns `Some(thumbnail_url)` using the existing Cloudflare Image
   Resizing path. Other or future pools may deliberately return `None`.
3. Page, user, team, and comic response Views expose both the origin and
   thumbnail URL fields.
4. A top-level list request performs at most one metadata query per required
   object marker, regardless of result count or include depth.
5. `ClearObjs`, `DeleteObjs`, and page manifest reservation use true batch persistence. Their
   database round-trip count must be bounded independently of item count.
6. A future manifest entry receives total-department and read-view
   implementations without handwritten marker-specific forwarding code.
7. No ObjDept Rust identifier starts with `__`. Generated internals use
   responsibility-bearing names and ordinary Rust visibility instead of an
   opaque punctuation convention.
8. Every manifest marker declares a compile-time URL rendition profile, so an
   image marker may request a thumbnail while a future Font marker remains
   origin-only without handwritten runtime marker branching.
9. Domain models and ordinary Rust locals do not use the database-column
   `f_` prefix. Actual Diesel columns and typed RDB row mappings retain their
   physical names at the persistence boundary.

## Phase 0: approve the required macro module splits

The planned work must change all three macro implementation files, which are
already at the repository's 600-line boundary:

- `object.rs`: 598 lines;
- `rdb_obj_prom.rs`: 594 lines;
- `impl_obj_dept.rs`: 557 lines.

Before implementation, confirm these minimal responsibility-based splits:

### `object.rs`

- Keep manifest parsing, uniqueness validation, and per-marker module assembly
  in `object.rs` (projected 240-280 lines).
- Move generated table-bound `FullRow`/`ObjRdbEntry` definitions and their
  reads, writes, and state transitions to `object/rdb_entry.rs` (projected
  430-520 lines).

### `rdb_obj_prom.rs`

- Keep task row types and actor-side reset/claim/settlement expansion in
  `rdb_obj_prom.rs` (projected 430-480 lines).
- Move transaction-side single and batch task creation expansion to
  `rdb_obj_prom/defer.rs` (projected 170-230 lines).

### `impl_obj_dept.rs`

- Keep input parsing, manifest callback expansion, actor dispatch, and final
  token composition in `impl_obj_dept.rs` (projected 210-260 lines).
- Move generated metadata/URL operations and read-view implementations to
  `impl_obj_dept/read.rs` (projected 180-260 lines).
- Move generated slot reservation and object cleanup operations to
  `impl_obj_dept/lifecycle.rs` (projected 260-360 lines).

These are line-limit splits only. They must not change public behavior by
themselves. Run `scripts/audit_module_split.py` after extraction and keep every
file strictly below 600 lines.

## Cross-cutting requirement: remove opaque generated identifiers

The current double-underscore prefix was used as a collision-avoidance
convention for macro-generated items. It makes generated paths difficult to
read and is not an accepted naming mechanism for this implementation.

Remove every ObjDept-related Rust identifier beginning with `__`, including
generated modules, callback macros, hidden exports, test macros, imports, and
test references. Use this responsibility-based mapping:

| Current form | Required replacement |
| --- | --- |
| `__obj_dept_<marker>` | `<marker>_rdb_impl`, for example `page_image_rdb_impl` |
| `__obj_dept_rdb_obj_prom` | `obj_prom_rdb_impl` |
| `__obj_dept_norm_obj_dept` | `norm_obj_dept_rdb_lock` |
| `__obj_dept_unique` | `obj_manifest_uniqueness` |
| `__objs_manifest` | `for_each_obj` |
| `__impl_obj_dept_callback` | `implement_obj_dept_from_manifest` |
| `__impl_obj_dept_items` | `expand_obj_dept_items` |
| `__obj_handle` | `handle_obj_task` |
| `__impl_mock_obj_dept` | `implement_mock_obj_dept` |
| `__impl_mock_obj_dept_callback` | `implement_mock_obj_dept_from_manifest` |

Rules for the replacement:

1. Do not mechanically rename `__foo` to `_foo` or `foo`; the replacement must
   state the item's role.
2. Keep generated marker-specific RDB implementation modules private and use
   `#[doc(hidden)]` only for support macros that must cross a crate boundary.
3. Keep collision prevention structural: one total manifest per composition
   module, marker-qualified RDB modules, and department-qualified RDB locking.
4. If a future composition needs two manifests in one Rust module, extend the
   macro input with an explicit caller-selected namespace. Do not restore
   punctuation-based pseudo-hygiene.
5. Update proc-macro token tests to reject emitted identifiers containing a
   double-underscore prefix and compile-test the readable generated paths.
6. The final Rust-source scan must return no matches:

   ```text
   rg -n --pcre2 '\b__[A-Za-z][A-Za-z0-9_]*' \
     --glob '*.rs' src poprako-obj-dept poprako-obj-dept-macro poprako-rdb-core
   ```

The externally defined PostgreSQL table name `__diesel_schema_migrations` is
not a Rust identifier and is outside this naming rule.

## Phase 1: restore the complete URL contract

### ObjDept model and operations

1. Add `poprako-obj-dept/src/model/url.rs` and define `ObjUrls` there.
2. Derive `Debug`, `Clone`, `PartialEq`, and `Eq`; do not derive transport
   serialization traits.
3. Change `GenObjUrls<B>` output from `HashMap<String, Url>` to
   `HashMap<String, ObjUrls>`, keyed by business-object ID as today.
4. Replace `ObjPoolView::gen_url` with `ObjPoolView::gen_urls`, generating both
   URLs for one physical key in one call. The pool returns `ObjUrls`;
   `thumbnail_url: None` means the capability is intentionally unavailable,
   while an error remains an error.
5. Keep unuploaded metadata absent from the result map. Do not turn an
   unuploaded entry into an `ObjUrls` value containing partial URLs.
6. Resolve independent physical keys with bounded concurrency inside
   `GenObjUrls`; do not create one task per returned API View.
7. Add an explicit marker-level URL rendition profile to the object manifest.
   The profile must at least distinguish image-thumbnail resolution from
   origin-only resolution, remain compile-time dispatched, and require no
   marker-name strings in R2 or use-case code.

### R2 and mock adapters

1. Restore the Cloudflare Image Resizing transform:

   ```text
   width=300,fit=scale-down,quality=80,format=auto,metadata=none
   ```

2. Build the origin and thumbnail URLs from the same normalized custom domain
   and physical key.
3. Return both URLs from R2 without performing remote I/O.
4. Let the mock configure `thumbnail_url` as `Some` or `None` so the optional
   contract is tested independently from current R2 behavior.
5. Propagate URL-construction failures. `Option` represents capability absence,
   not a swallowed adapter error.

### Response Views

Restore these response-neutral View fields:

- `PageInfoView::image_thumbnail_url`;
- `UserInfoView::avatar_thumbnail_url`;
- `TeamInfoView::avatar_thumbnail_url`;
- `ComicInfoView::cover_thumbnail_url`.

Keep them beside the corresponding origin URL and use the same optional
serialization policy. Update constructors, Swagger schemas, checked-in
OpenAPI output, TypeScript HTTP types, and affected fixtures together.

`ObjUrls` remains an ObjDept domain value. It must not be placed under
`src/data`; the API Views receive already-resolved optional strings.

## Phase 2: generate the read projection for every marker

1. Extend `impl_obj_dept!` with an explicit, self-describing declaration:

   ```rust
   impl_obj_dept! {
       dept: NormObjDept,
       view: NormObjView,
   }
   ```

2. For every manifest marker, generate the following for both the total
   department and its read projection:

   - `Run<ListObjMetas<B>>`;
   - `Step<ListObjMetas<B>, RdbContext<L>>`;
   - `Run<GenObjUrls<B>>`.

3. Remove the handwritten PageImage-only implementations from
   `src/part_impl/obj_dept.rs`.
4. Narrow the general Prom `ObjView` bound to
   `ObjDeptView<PageImage, RdbContext>`. Remove its unused `ObjPoolView`
   supertrait and the public pool forwarding implementation on `NormObjView`.
5. Add a macro expansion test with at least two markers and prove both markers
   receive read-view implementations. Give the second Font-like marker an
   origin-only URL profile while image markers retain their thumbnail profile.
   This is the compile-time extension test for a later Font marker.
6. Ensure generated implementations refer to readable paths such as
   `page_image_rdb_impl::load_many`, never synthesized `__obj_dept_*` paths.

## Phase 3: replace per-ID object persistence with true batches

### Shared normalization and locking

1. Normalize IDs/specs once at each operation boundary.
2. Sort and deduplicate deletion IDs. Reject duplicate slot specs because two
   specifications for one object ID are ambiguous.
3. Acquire existing and missing object identities atomically with one typed,
   bulk `INSERT ... ON CONFLICT DO UPDATE ... RETURNING` anchor statement over
   the normalized deterministic ID order. The conflict update may be a no-op,
   but it must acquire the row locks held through the caller transaction.
4. Do not split missing-anchor insertion from a later `FOR UPDATE` load and do
   not add a per-ID fallback. The one-statement candidate remains
   RESOLVED under OBJ-P-015 until concurrent RDB tests
   cover overlapping existing/missing sets, opposite input order, and rollback.

### Generated RDB-entry helpers

Generate typed helpers for each concrete object table:

- `load_many_for_update(ids)` using one `WHERE id = ANY(...) FOR UPDATE` query;
- `write_many(writes)` using one insert/upsert statement;
- `detach_many(ids)` using one update statement;
- `remove_many(ids)` using one delete statement.

Decode each row once and reuse the normalized ID map. Preserve deterministic
lock ordering and the existing latest-generation rules.

### Batch durable task creation

1. Extend `ObjPromDefer<C>` with batch Check and Delete creation.
2. Build deterministic task IDs in memory.
3. Insert all tasks with one typed Diesel insert and conflict handling.
4. Read all persisted identities/statuses with one query and validate every
   collision before success.
5. Keep the single-item methods as thin wrappers over the batch engine where
   that does not complicate borrowing.

### `ClearObjs` and `DeleteObjs`

Replace the current loop with this fixed-round-trip pipeline:

1. normalize and batch-lock all IDs;
2. load all current rows for update once;
3. derive all active keys in memory;
4. defer all Delete tasks as one batch;
5. apply the operation-specific state change with one statement.

Missing IDs remain idempotent. Any invalid state or task identity conflict
rolls back the entire caller transaction; partial cleanup is not allowed.

### Business-ID lifetime and removal

`DeleteObjs` ends the durable object identity, so its safety depends on a
public lifetime invariant rather than only transaction isolation:

1. Publish and enforce that a business ID is never reused within one object
   topic, including after the owning business row has been deleted.
2. Audit every row-removing caller against that invariant.
3. If the invariant cannot be enforced for every caller, retain the watermark
   or introduce a monotonic epoch. Do not permit recreation to reuse an old
   logical key, physical key, or durable-task identity.
4. Resolve OBJ-Q-018 only after one of those two dispositions has implementation
   and validation evidence.

### Slot reservation

1. Add `GenObjSlots<B>` with a slice of `ObjSlotSpec` and a result map keyed by
   business-object ID.
2. Add `GenObjSlots<B>` to the writable `ObjDept<B, C>` drive contract and
   implement it in generated RDB code and the mock adapter.
3. Batch-lock and batch-load all current states.
4. Compute new versions, old Delete keys, and new physical keys in memory.
5. Generate independent pool slots with bounded concurrency.
6. Batch-write latest rows and batch-defer old Delete/new Check tasks.
7. Implement `GenObjSlot<B>` through the same engine so singleton endpoints do
   not maintain a second state-transition implementation.

The transaction remains all-or-nothing. A generated presigned URL whose
transaction later rolls back is harmless and must not leave a durable Check
task.

## Phase 4: make page manifest reservation bounded

1. Convert `existing_page_infos` into an ID map once. Replace every repeated
   linear `.find(...)` with constant-time lookup.
2. Validate every retained page ID against that map before issuing writes.
3. Replace the per-page `UpdatePageManifest` plus separate `CreatePages` flow
   with one `ApplyPageManifest` repository operation:

   - accept all final page identities, chapter IDs, and indexes;
   - insert new pages and update retained indexes in one typed batch upsert;
   - preserve existing counters for retained pages;
   - return every resulting `PageInfo` and verify the returned count.

4. Split image specifications into retained and newly uploaded groups.
5. Fetch all retained `PageImage` metadata with one `ListObjMetas` step and
   validate it in memory.
6. Reserve all changed/new images with one `GenObjSlots<PageImage>` step.
7. Delete removed images through the batch `DeleteObjs` implementation.
8. Preserve manifest response order by joining result maps back to the input
   sequence in memory.

The 200-page maximum must not change the number of SQL statements in steps
3, 5, 6, or 7.

## Phase 5: eliminate nested View N+1

Follow the algorithm used by
`src/part_impl/repo/rdb_impl/incl/framework.rs`, without importing or depending
on the RDB implementation:

```text
walk the complete included model graph
    -> collect object IDs by marker
    -> deduplicate IDs
    -> batch-load one snapshot per marker
    -> render every View synchronously from those snapshots
```

### Collection

1. Add private use-case-side collection helpers for `UserInfo`, `TeamInfo`,
   `ComicInfo`, `ChapterInfo`, and `AssignmentInfo`.
2. Traverse existing optional include paths, including:

   - assignment user;
   - assignment chapter;
   - chapter creator and comic;
   - comic cover, team, and creator.

3. Deduplicate business IDs before calling ObjDept. Use deterministic vectors
   for stable tests.

### Resolution

1. Add one generic helper that performs exactly one `ListObjMetas<B>` followed
   by one `GenObjUrls<B>` for a marker.
2. Resolve different marker snapshots with one bounded `tokio::try_join!` at
   the root request. Current nested graphs require at most ComicCover,
   TeamAvatar, and UserAvatar concurrently.
3. Do not call ObjDept from recursive or per-item View constructors.
4. Do not use `collect_bounded` to parallelize per-record database access. Once
   snapshots exist, all recursive conversion is synchronous.

### Injection and rendering

1. Keep the existing domain-specific View modules.
2. Give their internal render functions borrowed snapshot maps and make them
   pure.
3. Implement single-item public helpers through their corresponding batch
   path or a one-element snapshot without cloning the ID into a literal slice.
4. Reuse one resolved URL entry for repeated IDs. Clone strings only when two
   independently owned response fields require it.
5. Remove `collect_bounded` and its concurrency test if no non-View caller
   remains; retain `LoadMode` in `usecase::internal::util`.

The required query bound for one top-level request is:

```text
ObjDept metadata queries <= number of distinct object markers requested
```

It must not depend on root row count, include depth, or repeated related IDs.

## Phase 6: record accepted safety debt and remove ambiguity

### Required `SAFETY` comments

Add concise English `SAFETY` comments at:

- each `mark-uploaded` use case, stating that it validates only the current
  generation, optimistically exposes its URLs, and does not prove a successful
  PUT;
- the ObjDept Check presence test, stating that object existence is accepted
  as upload evidence and content-hash verification is deliberately deferred
  for upload-throughput reasons. The same comment must make the exact-version
  correction fence explicit so delayed absence cannot corrupt a newer
  generation.

These comments must not claim checksum integrity or change behavior.

### Same-generation actor correction fence

The actor must not apply an earlier HEAD result over a later optimistic client
mark for the same ID and generation:

1. Capture a row revision before starting remote HEAD.
2. Apply the presence result only when ID, generation, active-state columns,
   and the captured revision still match.
3. Treat a lost CAS as a race to reload/classify, never as authority to
   overwrite the later state.
4. Resolve OBJ-Q-016 only after deterministic interleaving tests prove both the
   lost-race and ordinary absent paths.

### Lifecycle vocabulary

1. Keep the bool output of `MarkObjUploaded`, bind it as `marked`, and handle
   `true`/`false` at the immediate use-case boundary. Do not introduce a generic
   `Outcome` type for this transition.
2. Rename `ObjKeyState::Verified`; client marking is optimistic availability,
   not remote or content verification.
3. Split cleanup into `ClearObjs` for active business entities and `DeleteObjs`
   for ended business entities. Do not expose persistence strategy as an enum
   or constructor name.
4. Keep the database-column `f_` prefix only on actual schema/table tokens and
   typed RDB row mappings. Domain state uses `is_available`; ordinary locals
   use direct semantic names such as `marked`, `exists`, and
   `include_thumbnail`.
5. Delete `obj_inst!`. Give each operation an ordinary associated constructor
   so rust-analyzer resolves the real type at every call site.
6. Resolve OBJ-Q-019 through OBJ-Q-021 only after the targeted naming scans,
   compile, and tests pass.

### Required ObjActor `FIXME`

Place one English `FIXME` beside the globally serial actor loop. It must record
that bounded parallel claim/processing, independent reset maintenance,
operator recovery, and completed-task retention need a separate design before
high-volume object topics are added. Do not modify actor behavior in this
change.

### Cleanup

1. Delete the unreferenced legacy `src/part_impl/obj_pool.rs` and
   `src/part_impl/obj_pool/` implementation after confirming there are no
   active imports.
2. Remove PageImage-only forwarding code replaced by macro generation.
3. Fix Clippy-proven unnecessary clones, including singleton ID slices.
4. Do not refactor clones inside the deferred ObjActor merely for style.
5. Remove duplicated or stale responsibility comments in the touched ObjDept
   View and page-reservation paths.
6. Update the ObjDept review record only after the new validation evidence
   exists; do not preserve obsolete passing checkboxes.

## Phase 7: verification

### Contract tests

- `ObjUrls` contains an origin and optional thumbnail URL.
- R2 origin URL uses the custom domain.
- R2 thumbnail URL uses the exact Cloudflare transform.
- A pool/mock returning `thumbnail_url: None` still produces a valid origin
  response.
- A Font-like compile-test marker selects origin-only URL resolution while
  image markers select the thumbnail profile without runtime marker matching.
- Unuploaded metadata produces no URL-map entry.
- URL-generation errors propagate rather than becoming `None`.
- Page, user, team, and comic Views serialize both URL fields correctly and
  keep Swagger/OpenAPI aligned.
- Page, user, team, and comic mark-uploaded endpoints optimistically expose
  origin and supported thumbnail URLs only for the submitted exact current
  generation; stale generations remain rejected.
- A delayed Check with absent remote state resets only its exact generation to
  unuploaded, while a stale Check cannot change a newer generation.
- An in-flight absent HEAD cannot revoke a later optimistic mark for the same
  generation; a revision-CAS loss reloads/classifies without overwriting it.
- Neither mark-uploaded nor Check claims server-side content-hash validation.

### Batch and transaction tests

- `ClearObjs` and `DeleteObjs` handle multiple, duplicate, and missing IDs without partial
  changes.
- Batch lock contention returns a retryable error and rolls back the complete
  operation.
- Atomic anchor acquisition covers overlapping existing/missing ID sets and
  opposite caller order without a create-then-lock interleaving window.
- Batch task conflicts and operator statuses roll back latest-state writes.
- `GenObjSlots` allocates correct next versions and schedules old Delete/new
  Check tasks for every item.
- Duplicate slot specifications fail before persistence.
- A mixed retained/new/deleted 200-page manifest preserves order, counters,
  and atomicity.
- Mock call counters prove one ObjDept metadata operation per marker, not per
  returned model.
- Repeated nested users/teams/comics are resolved once per marker snapshot.
- Row removal cannot permit an old logical/physical key or durable task to
  alias a recreated business ID: either non-reuse is enforced and audited or
  watermark/epoch monotonicity is tested.

Add focused RDB tests for query semantics, row locking, collision handling,
and rollback. Keep use-case behavior tests beside the affected modules. Update
the TypeScript integration suite and `TESTCASES.md` together; remove direct
queries of retired legacy object columns from the archive suite.

### Required checks

Run the narrow checks first, then the shared checks:

```text
cargo fmt --all --check
cargo check --all-features
cargo test -p poprako-obj-dept --all-features
cargo test -p poprako-obj-dept-macro
cargo test -p poprako-server --lib
cargo clippy --all-targets --all-features -- -D warnings
scripts/check-rust-lines.sh
cd tests/integration-tests && deno task check
```

Also scan touched Rust for database-prefix leakage. `f_` identifiers are valid
only when they directly name schema/table columns, generated typed RDB row
fields, or direct SQL integration row fields; they are invalid in domain
models, operation outputs, use cases, and ordinary locals.

Run the HTTP/database integration suite only against the explicitly disposable
CI database. Do not run Docker, release helpers, deployment scripts, or any
production cutover operation for this plan.

## Completion criteria

The repair is complete only when:

- every actionable issue in `obj-dept-review-issues.md` has implementation and
  validation evidence;
- all four current image markers return origin and optional thumbnail URLs;
- a Font-like marker proves the origin-only URL profile without image-specific
  handwritten code;
- a second compile-test marker receives generated read-view implementations;
- ObjDept source and emitted support identifiers contain no `__` prefix;
- no reviewed list or page-manifest path performs per-item SQL;
- nested View assembly has a marker-count query bound;
- public schemas and integration types agree;
- accepted hash/Actor debt is explicit in `SAFETY`/`FIXME` comments;
- actor correction is revision-fenced against same-generation marks, anchor
  acquisition is validated atomically, and the business-ID reuse disposition
  is explicit;
- lifecycle and cleanup operations have self-describing domain names,
  and database `f_` prefixes do not leak across the RDB boundary;
- standard format, compile, test, Clippy, and line-limit checks pass.
