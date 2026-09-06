# Diesel Usage Audit

This is the active audit record for production Diesel access and the use cases
that determine its query shape. Update or remove resolved findings so the
document continues to describe the checked-in implementation.

- Reviewed: 2026-09-04
- Repository revision: `0a9cc927`
- Scope: 55 production Diesel-related Rust files, approximately 221 query
  execution sites, 56 coordinated transaction paths, and 35 explicit
  `FOR UPDATE` sites
- Method: static call-path review; tests and generated `schema.rs` were not
  treated as production query paths

Runtime query plans must still be verified against representative data with
`EXPLAIN (ANALYZE, BUFFERS)` before adding or changing indexes.

## Priority findings

### Resolved: Unit editor reads and writes are bounded

`ApplyUnitEdits` builds one linear edit plan from the complete persisted Unit
chain. Creates use at most one typed batch insert; Saves, Deletes, and changed
persisted successors use at most one typed Diesel `CASE` update. The number of
write statements therefore no longer grows with the edit count or the number
of changed successors. Every update remains scoped by both Page and Unit ID,
and an unexpected affected-row count rejects the complete transaction.

The complete `(id, next_id, hidden_at)` graph is keyset-chunked at 512 consumed
rows, using a 513th sentinel to avoid an extra empty query at exact chunk
boundaries. This bounds each result set; total read and memory cost remains
`O(history)` because business semantics require every restorable tombstone and
anchor. Chapter and Page locks serialize every production Unit mutation, so
the graph read no longer locks all historical tombstones; concrete writes lock
only changed rows.

Post-write counters are one typed aggregate row. The translated predicate uses
the explicit Unicode whitespace set shared with Page diff semantics, so the
business text rule is pushed down without raw SQL or semantic weakening.
Public Page Unit listing uses a short repeatable-read snapshot: the same
snapshot supplies Page counters, chunked links, and full fields for only the
at-most-100 visible Unit IDs. A corrupt chain, a 101st visible Unit, a hydrate
mismatch, or a counter mismatch is an unrecoverable invariant failure rather
than a truncated response.

Evidence:

- [`src/complex/unit/edit.rs`](../src/complex/unit/edit.rs)
- [`src/part_impl/repo/rdb_impl/unit/edit.rs`](../src/part_impl/repo/rdb_impl/unit/edit.rs)
- [`src/part_impl/repo/rdb_impl/unit/sequence.rs`](../src/part_impl/repo/rdb_impl/unit/sequence.rs)
- [`src/usecase/unit.rs`](../src/usecase/unit.rs)

No Unit index is changed without representative
`EXPLAIN (ANALYZE, BUFFERS)` evidence. The existing Page-only index remains
until the history keyset query demonstrates an explicit sort or substantial
cross-Page filtering; that evidence would justify replacing it with
`(f_page_id, f_id)`. A visible partial index remains a separate measurement.

### P0 pinned: Chapter and Comic lock order is globally inconsistent

Unit and stage paths acquire Chapter before updating Comic activity, while
some Chapter creation and subtree-deletion paths acquire Comic before Chapter.
That inverse order can deadlock across otherwise valid transactions. PostgreSQL
`40P01` currently reaches Diesel as an unknown database error and is therefore
classified as unrecoverable instead of the public retryable conflict.

This is pinned for a separate cross-domain correction. Moving the Comic lock
into the Unit transaction prefix would serialize every Chapter save belonging
to that Comic and is not an acceptable local optimization without a broader
business and contention review.

### Resolved: Chapter Unit search no longer fans out by Page

Search now uses a bounded, typed three-stage read instead of introducing a
search-specific god object:

1. PostgreSQL applies Chapter scope, visibility, selected-field literal
   matching, and `LIMIT 101`, returning Unit IDs only.
2. For 1--100 matches, the existing `UnitInfo` projection is loaded for those
   IDs. This deliberately reuses the established domain projection rather than
   creating another wide `SearchInfo` type.
3. Only the matching Pages' `(page_id, id, next_id)` links are loaded. The
   existing chain validator derives Unit rank, and results are sorted by Page
   index and chain order.

The overflow path executes one Unit query; a successful non-empty search
executes three, independent of Chapter Page count. The former 20-query
concurrency fan-out and worst-case 201 Unit/Page queries are gone. All three
reads share a short repeatable-read transaction; phrase validation and
authorization remain outside it. NUL remains an authorized empty result
because PostgreSQL text cannot contain or bind it.

No migration or speculative index was added. Single-character matching is a
business requirement, so representative `EXPLAIN (ANALYZE, BUFFERS)` evidence
is still required before considering a partial visible-Unit index.

Verified behavior includes both text fields, literal `%`, `_`, and backslash,
case sensitivity, Unicode trimming, hidden chain nodes, stable Page/Unit order,
NUL, and the exact 100/101 boundary.

Evidence:

- [`src/usecase/unit.rs`](../src/usecase/unit.rs#L142)
- [`src/part_impl/repo/rdb_impl/unit/sequence.rs`](../src/part_impl/repo/rdb_impl/unit/sequence.rs#L227)
- [`src/usecase/unit/tests/search.rs`](../src/usecase/unit/tests/search.rs#L1)
- [`tests/integration-tests/src/suites/it_05_unit_save_order_count.ts`](../tests/integration-tests/src/suites/it_05_unit_save_order_count.ts#L102)

### Resolved P0: Hierarchical deletion

Team, Workset, and Comic deletion now atomically marks the selected hierarchy
with one timestamp. Marked Team, Workset, Comic, and Chapter rows are excluded
from normal reads and guarded mutations immediately. Direct Chapter deletion
remains synchronous because Chapter is the cleanup unit and its Page count is
frozen while deletion is in progress.

Two scheduler workers repeatedly attempt explicit levels with `FOR UPDATE SKIP
LOCKED`, in the order Chapter, childless Comic, childless Workset, then childless
Team. A successful level restarts polling from Chapter; an empty or failed level
falls through to the next one. Chapter and Team claims contain one row, while
Comic and Workset claims contain at most 64 rows ordered by `(deleted_at, id)`.
Each claim is one transaction that inserts all object deletion tasks before
batch-deleting its direct dependants and roots, so a rollback leaves the whole
claim available for a later sweep. Prom itself keeps its existing unordered
semantics.

The shared RDB pool is capped at four connections for the 2C2G production host;
the deletion scheduler uses two workers. Permanent-failure quarantine remains a
focused FIXME. No ordered task table or application-side copy of an entire
hierarchy was introduced.

The repository contract and every production execution boundary were also
audited in reverse: all declared domain `Oper`s have direct use-case consumers,
and no HTTP, scheduler, effect, or Prom business handler executes one outside
`src/usecase`. Thirteen obsolete operations and their RDB/mock implementations
were removed. Domain-event and deferred-task business orchestration now lives in
business-domain use cases; their actors only provide queueing, dispatch, and
retry lifecycles. This caller-independent boundary is codified in
[`usecase-boundaries`](../.agents/skills/usecase-boundaries/SKILL.md).

Tombstone follow-up guards cover adjacent lifecycle paths: user deletion
batch-removes memberships even when their teams are already hidden; archive
snapshot and commit both require an active Comic; and normal Page identity
reads reject Pages whose Chapter is tombstoned, preventing late upload
confirmation.

Evidence:

- [`src/part_impl/repo/rdb_impl/subtree_delete.rs`](../src/part_impl/repo/rdb_impl/subtree_delete.rs)
- [`src/part_impl/repo/rdb_impl/subtree_delete/mark.rs`](../src/part_impl/repo/rdb_impl/subtree_delete/mark.rs)
- [`src/part_impl/repo/rdb_impl/subtree_delete/sweep.rs`](../src/part_impl/repo/rdb_impl/subtree_delete/sweep.rs)
- [`src/usecase/subtree_delete.rs`](../src/usecase/subtree_delete.rs)
- [`src/usecase/internal/subtree_delete.rs`](../src/usecase/internal/subtree_delete.rs)
- [`src/extra/sched/subtree_delete.rs`](../src/extra/sched/subtree_delete.rs)
- [`src/part_impl/repo/rdb_impl/subtree_delete/tests.rs`](../src/part_impl/repo/rdb_impl/subtree_delete/tests.rs)

### P0 plan: Bound ordinary lists without truncating complete aggregates

The fifteen ordinary public lists, Page/Unit editor reads, and archive export
have different business contracts and must not share one pagination rule.

| Read class | Public contract | Database bound |
| --- | --- | --- |
| Ordinary lists | Require `1 <= limit <= 200`; use an opaque cursor for complete traversal | Fetch at most `limit + 1`, then hydrate includes only for the returned IDs |
| Chapter Pages | Always return the complete ordered manifest | Domain maximum 200 Pages; read at most 201 to detect persisted invariant violations |
| Page Units | Always return the complete visible edit sequence and counters | Domain maximum 100 visible Units; do not load full tombstone rows |
| Comic archives | Always export every selected retained payload | Stream one ordered database chunk at a time; never truncate by count or bytes |

The ordinary-list limit is a business rule. Represent it as
`ListLimit<const N: u32 = 20>`, where `N` is the compile-time maximum, and use
`ListLimit<200>` for ordinary public lists. Its custom deserializer rejects an
invalid query during HTTP extraction, so `0` and `201` receive `400 Bad
Request` before a handler or use case runs. Non-HTTP callers must use the
checked constructor. Repository operations accept the proven value rather
than a raw `u32`, and typed Diesel queries unwrap it only at `.limit(...)`.

This bounded-limit foundation is implemented for all existing ordinary list
inputs, repository specs, RDB adapters, and mock adapters. The OpenAPI schema
derives its inclusive `1..=N` range from the same const generic.

`limit + offset <= 200` is not the final pagination rule: by itself it would
make every row after the first 200 unreachable. Use it only for the legacy
offset compatibility path after cursor pagination is available. The rollout is:

1. Add optional cursor input and optional `next_cursor` response metadata while
   leaving each existing `data` payload shape unchanged. Preserve the current
   sort as the primary order and append ID as the final unique order key.
2. Fetch 201 base rows for a requested limit of 200, remove the sentinel before
   include/object hydration, and derive a versioned cursor bound to the list
   scope and normalized filters. Malformed, mismatched, or cursor-plus-offset
   requests are `422 / Args`.
3. After callers use cursors, constrain the legacy path to
   `offset + limit <= 200`; then remove offset in a later API revision. Cursor
   pages remain able to traverse an arbitrarily long static result set.

Page and Unit endpoints remain intentionally unpaginated. Consolidate the two
existing Chapter Page limits into one Page-domain constant, separate from the
ordinary-list limit even though both equal 200. Page list and edited-diff
queries must return all Pages in index order and treat a 201st row as corrupt
persisted state, never as a partial response.

Page hardening is implemented: allocation and both import formats share the
Page-domain maximum; manifest, locked-manifest, and edited-diff reads use a
typed 201-row sentinel with `(index, id)` ordering. Edited-diff selects only
`(id, has_diff)` until the Page-count invariant is proven, then filters the
response without loading full Page rows.

Unit listing is hardened without changing its complete-response contract.
Hidden nodes remain in the structural graph because restoring one at its
stored position is existing business behavior. A short repeatable-read
snapshot now reads the complete link graph in minimal chunks, validates and
orders it, and loads full fields only for the at-most-100 visible IDs. A 101st
visible Unit, a counter mismatch, or a corrupt chain is an unrecoverable
invariant failure rather than a partial response.

Archive export remains complete. Push the exact selected month intervals into
one typed SQL predicate, order by `(created_at, id)`, and stream rows into a
temporary export artifact with bounded buffering before the HTTP adapter sends
it. This preserves the current JSON envelope and month grouping without
holding all payloads in memory or holding a database connection for the
client's transfer time. Limit concurrent export artifact creation separately;
resource exhaustion is an infrastructure error, not permission to return a
partial archive.

Opaque cursor pagination and archive streaming are intentionally deferred and
recorded as actionable TODOs in their shared implementation boundaries. The
current bounded list limit and complete archive response semantics remain in
force until those TODOs are implemented.

Implementation order:

1. Add the bounded list value and HTTP extraction tests; then add the cursor
   codec and optional response metadata.
2. Convert all fifteen ordinary list specs and typed Diesel queries; add the ID
   tie-breakers and only add matching indexes after representative
   `EXPLAIN (ANALYZE, BUFFERS)` evidence.
3. Complete: harden the Page and Unit reads without adding pagination to their
   public endpoints.
4. Replace archive materialization and range filtering with typed interval
   filtering plus the bounded export artifact path.
5. Update OpenAPI and the HTTP integration suite. Cover `0/1/200/201`, legacy
   window overflow, cursor/filter mismatch, duplicate-free static traversal,
   the exact Page/Unit aggregate limits, non-contiguous archive months, and
   bounded-memory multi-chunk export.

Evidence:

- [`src/data/instr/comic.rs`](../src/data/instr/comic.rs#L94)
- [`src/usecase/page/alloc/validation.rs`](../src/usecase/page/alloc/validation.rs#L13)
- [`src/value/unit.rs`](../src/value/unit.rs#L7)
- [`src/part_impl/repo/rdb_impl/unit/sequence.rs`](../src/part_impl/repo/rdb_impl/unit/sequence.rs#L323)
- [`src/part_impl/repo/rdb_impl/comic_archive/payload.rs`](../src/part_impl/repo/rdb_impl/comic_archive/payload.rs#L44)
- [`src/usecase/comic_archive.rs`](../src/usecase/comic_archive.rs#L79)

### Resolved P1: Comic stage filtering is scoped and correlated

Stage-filtered Comic listing now keeps the complete pinned, non-deleted Chapter
predicate in a typed correlated `EXISTS` under the Workset-scoped Comic query.
It does not materialize global Comic IDs or issue a second filtering query.
Masks that ignore every stage add no predicate, while an unsupported Active
phase returns an empty result before database access.

Evidence:

- [`src/part_impl/repo/rdb_impl/comic/stage_filter.rs`](../src/part_impl/repo/rdb_impl/comic/stage_filter.rs#L41)
- [`src/part_impl/repo/rdb_impl/comic/step_impl.rs`](../src/part_impl/repo/rdb_impl/comic/step_impl.rs#L84)

### Resolved P1: Database pool waiting is bounded

The RDB pool remains capped at four connections and now uses the Tokio runtime
with a five-second wait timeout. Exhausted requests queue at the pool and map a
wait timeout to HTTP 503/code 9 without exposing driver details. Connection
acquisition duration and failures by bounded reason are recorded at the pool
boundary, which is also the single logging site for pool driver errors.

Evidence:

- [`poprako-rdb-core/src/rdb.rs`](../poprako-rdb-core/src/rdb.rs#L92)
- [`src/api/http/middleware/rate_limit.rs`](../src/api/http/middleware/rate_limit.rs#L15)

Residual risk: there is deliberately no HTTP in-flight concurrency limit and
no reserved connection capacity for background consumers. The five-second
bound prevents indefinite waiting but does not guarantee background fairness.
Serialization failures remain retryable conflicts and transaction isolation is
unchanged.

### Resolved P1: Object-task claiming and maintenance poll independently

The object actor now supervises an immediate claim loop and an immediate
maintenance loop with shared cancellation. Each loop waits 30 seconds after an
empty cycle, while the claim loop continues immediately when it finds work.
Maintenance retains its three explicit repair updates, limiting idle
maintenance to six updates per minute per application instance.

Claiming is one typed `UPDATE ... RETURNING` whose ordered candidate subquery
uses `FOR UPDATE SKIP LOCKED`. Pending and visibility checks, lease increment
and overflow quarantine, attempt timeout, and finalization fencing are retained.

Evidence:

- [`poprako-obj-dept/src/actor.rs`](../poprako-obj-dept/src/actor.rs#L174)
- [`poprako-obj-dept-macro/src/rdb_obj_prom.rs`](../poprako-obj-dept-macro/src/rdb_obj_prom.rs#L187)

### Resolved P1: Term import updates are batched and scoped

Term import now bulk-inserts new Terms and applies all replacements in one typed
`CASE` update scoped by both Termbase ID and Term IDs. Duplicate IDs, invalid
insert scope, and unexpected insert or update affected-row counts are
unrecoverable invariants that roll back the import transaction.

Administrator-retention work remains deferred P1: User deletion still loads
and locks the complete roster of every Team the user belongs to, while
Assignment and Member role changes load full collections to evaluate
administrator retention.

Evidence:

- [`src/part_impl/repo/rdb_impl/term.rs`](../src/part_impl/repo/rdb_impl/term.rs#L375)
- [`src/usecase/user/delete.rs`](../src/usecase/user/delete.rs#L72)
- [`src/usecase/assignment/update_roles.rs`](../src/usecase/assignment/update_roles.rs#L58)
- [`src/usecase/member.rs`](../src/usecase/member.rs#L340)

Deferred action: use purpose-specific aggregate queries that return only
offending Team or Chapter IDs. The invariant that a Team or Chapter retains an
administrator must remain transactionally safe.

## Secondary improvements

- Add purpose-specific model projections for cascade IDs, Page object IDs,
  import-target lookup, and latest-Chapter selection. Repository operations
  must continue to return models or projections, not request/response `View` or
  `Val` DTOs.
- Return only IDs from create operations whose callers consume only IDs,
  instead of returning complete `Info` rows.
- Push Unit counters, administrator existence checks, exact Termbase-name
  lookup, and latest-Chapter selection into typed aggregate, existence, or
  `LIMIT 1` queries.
- Give multi-row `FOR UPDATE` queries a deterministic lock order. Configure
  database statement and lock timeouts so large cascades cannot wait forever.
- Comic update reads the same Comic in the use case and again inside the
  adapter. Reuse one transactionally valid projection containing the index
  needed to compose the title.

## Business and transaction safeguards

- Serializable or repeatable-read transactions protect business invariants;
  optimize work inside them before considering a weaker isolation level.
- Comic archive preparation serializes a potentially large JSON payload while
  descendant rows remain locked. Moving it outside the transaction without a
  version check or two-phase protocol would break snapshot consistency.
- Chapter translation export correctly loads all Pages and Units in batches.
  If the exported document must represent one database version, those reads
  should use a short repeatable-read snapshot rather than independent pooled
  connections.
- Destructive Team, Workset, Comic, and Chapter permission checks currently
  occur before their deletion transaction. A cascade rewrite must revalidate
  the authorization evidence transactionally instead of moving more checks
  outside the transaction.
- Complete import and export operations must not be truncated or made
  partially successful for performance reasons.

## Existing patterns to retain

- Include hydration batches foreign-key IDs with `eq_any` rather than issuing
  one query per result:
  [`src/part_impl/repo/rdb_impl/incl/macros.rs`](../src/part_impl/repo/rdb_impl/incl/macros.rs#L20).
- Chapter translation export fetches all Units for all Page IDs in one query:
  [`src/usecase/chapter_port/export_translation.rs`](../src/usecase/chapter_port/export_translation.rs#L96).
- Edited-diff Page detection uses a correlated `EXISTS` predicate:
  [`src/part_impl/repo/rdb_impl/page/step_impl.rs`](../src/part_impl/repo/rdb_impl/page/step_impl.rs#L141).
- System-mail read marking validates a projected batch and updates the batch in
  one statement:
  [`src/part_impl/repo/rdb_impl/system_mail.rs`](../src/part_impl/repo/rdb_impl/system_mail.rs#L103).

## Recommended implementation order

1. Batch Chapter import and Unit edits.
2. Replace Chapter Unit search fan-out and establish concurrency limits.
3. Implement purpose-built subtree deletion.
4. Bound ordinary lists and rewrite archive and stage filters.
5. Batch Term and membership writes, then introduce smaller projections and
   database aggregates.

## Usecase naming lint

No violations were found. A total of 265 public functions were scanned in the
usecase and complex layers; all names follow the current convention.
