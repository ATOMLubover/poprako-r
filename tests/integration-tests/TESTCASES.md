# HTTP Integration Test Inventory

This is the maintenance inventory for the TypeScript API suite. Whenever a file under `src/suites/` is added, removed,
renamed, or materially changes scope, update this document in the same change. The executable suite and its assertions
are the source of truth; do not keep separate implementation plans or status trackers here.

## Running the suite

```text
cd tests/integration-tests
deno task check
```

For an isolated end-to-end run, configure `INTEGRATION_DATABASE_URL` and run:

```text
scripts/api-integration-test.sh
```

The script creates and drops the dedicated integration database. Do not point it at a database that contains data you
need to preserve.

When running against an API server that is already available, use:

```text
deno task api
```

If the required variables only exist in the project `.env`, use:

```text
deno task --env-file=../../.env api
```

## Suite order

`src/main.ts` resets the database, runs these modules in order, then restores the seed-only state in its `finally`
block. Every current module exports `IMPLEMENTED = true`.

| Module | File                                        | Coverage                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| ------ | ------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| it_00  | `it_00_bootstrap_auth_default_seed.ts`      | Seed data, login, and unauthenticated access.                                                                                                                                                                                                                                                                                                                                                                                                                   |
| it_01  | `it_01_member_invitation_register_roles.ts` | Invitations, registration, member lists, and role perms.                                                                                                                                                                                                                                                                                                                                                                                                        |
| it_02  | `it_02_workset_comic_chapter_index.ts`      | Workset, comic, chapter indexes, dedicated pinning, profile updates, and positionally aligned comic/pinned-chapter list payloads.                                                                                                                                                                                                                                                                                                                               |
| it_03  | `it_03_page_reserve_image.ts`               | Authoritative hash-plus-extension manifests, optional `new_byte_len` retention, required upload lengths, duplicate-ID and count validation, object uploads, optimistic exact-generation mark semantics with immediate origin/optimized/thumbnail URLs, comic-cover fallback to the pinned chapter's first uploaded page across direct/list/nested responses, replacement, deletion, and page rebuilds.                                                          |
| it_04  | `it_04_assignment_invitation.ts`            | Assignment joins, invitations, role updates, self role removal, deletion, and comic-cover fallback in deeply nested owner-assignment responses.                                                                                                                                                                                                                                                                                                                 |
| it_05  | `it_05_unit_save_order_count.ts`            | Unit v2 create/next ordering and counters; chapter-scoped non-empty single-character text search with literal metacharacters, case and selected-field semantics, NUL handling, hidden-node ordering, and the 100-match limit; proofread-diff Page ID listing; non-cascading literal transform; atomic combined translation exports; required import modes, permission-aware PopRaKo overwrite round trips with changed-page counts, and keep-mode preservation. |
| it_06  | `it_06_unit_concurrency.ts`                 | Serializable same-Page writes with client-side 409/code 8 retries, same-anchor inserts, tombstone delete/Patch commit order, and linked-list completeness.                                                                                                                                                                                                                                                                                                      |
| it_07  | `it_07_workflow_sysmail.ts`                 | Snake_case-body workflow transitions, strongly typed immutable activity events without repository storage JSON or rendered text, pagination, and deferred system mail.                                                                                                                                                                                                                                                                                          |
| it_08  | `it_08_info_update_upload_mark.ts`          | Resource updates, runtime avatar-size validation, avatar/cover PUTs before optimistic exact-generation mark requests, immediate origin/thumbnail URLs, idempotent repeated allocations for identical available content, stale upload rejection, announcement create/update/delete, comments, and profiles.                                                                                                                                                      |
| it_09  | `it_09_cross_team_perm.ts`                  | Cross-team authorization isolation (including chapter activity records) and team-scoped online-user leases.                                                                                                                                                                                                                                                                                                                                                     |
| it_10  | `it_10_cascade_delete_cleanup.ts`           | Cascade deletion, serializable member-admin retention, last-admin member/user deletion rejection, and cleanup side effects.                                                                                                                                                                                                                                                                                                                                     |
| it_11  | `it_11_comic_archive.ts`                    | Permanent comic archive snapshots, lifecycle list filtering, and image-delete prom records.                                                                                                                                                                                                                                                                                                                                                                     |
| it_12  | `it_12_termbase_term.ts`                    | Termbase/term lifecycle, native JSON import/export and force merge, inherited lookup, fuzzy isolation, the 200-term capacity boundary, write perms, response contracts, and termbase/comic/team cascades.                                                                                                                                                                                                                                                       |

## Shared fixtures and invariants

- `src/db/seed.ts` owns reset, cleanup (including object metadata and durable object tasks), and seed-only assertions.
- `src/http/fixtures.ts` owns reusable API operations, deterministic SHA-256 image requests, and direct
  page/avatar/cover PUT uploads using every signed response header before mark-uploaded confirmation.
- `src/http/invariants.ts` owns counter, index, workflow, export, and mail consistency assertions.
- `src/state/runCtx.ts` is the shared state passed between modules.

When a test reveals a contract change, update the tested source, the relevant module assertion, and any affected active
API document together.
