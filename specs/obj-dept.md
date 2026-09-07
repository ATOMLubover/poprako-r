# ObjDept Reliable Object Lifecycle Spec

## Scope

ObjDept owns the relation between one business object and its physical OSS
objects. A business object has one stable `id` and at most one newest logical
version. Upload checks and deletion are asynchronous, so it may temporarily
have zero to many physical versions.

`id + version` forms the logical `ObjKey`. ObjDept alone encodes the physical
key; callers treat `ObjKey` as opaque. This slice supports only durable Check
and Delete work. Inventory scans and other recovery systems are out of scope.

The registered object kinds are `PageImage`, `UserAvatar`, `TeamAvatar`, and
`ComicCover`, and `ChapterArtwork`.

## Dependency direction

```text
poprako-rdb-core <- poprako-obj-dept <- poprako-server
```

`poprako-rdb-core` contains neutral RDB mechanics. `poprako-obj-dept` contains
the `ObjPool`, `ObjProm`, and `ObjPromDefer` support traits, values, operations,
actor, typed Diesel glue, and declaration macros. `poprako-server` supplies the
private R2 support implementation, generated schema, business markers, concrete object manifest, concrete
RDB prom declaration, and the total `NormObjDept` composition.

No image-specific pool abstraction or server application port sits between
ObjDept and `ObjPool`.

## Static declaration and dispatch

The server declares every supported object in one manifest:

```rust
objs_def! {
    PageImage {
        table: t_page_image,
        topic: "page_image",
        namespace: "page_image",
    },
}
```

`objs_def!` rejects duplicate marker, table, topic, or namespace values and
generates typed helpers against each concrete Diesel table. It also emits one
local callback manifest. `impl_obj_dept! { NormObjDept }` consumes that
manifest and generates every operation implementation plus the full static
topic dispatch. Its input never names a pool or prom implementation.

Adding a new object kind requires its marker, standard Diesel table, and one
manifest entry. `ObjPool` and `ObjProm` remain trait-constrained dependencies.
There is no runtime registry, erased RDB access, boxed future, or dynamic trait
dispatch in the object-kind path.

## Diesel safety

Migrations are the schema source and Diesel generates `schema.rs`. Generated
code directly references the concrete table and all required columns. A
missing table, missing column, incompatible SQL type, or wrong nullability
therefore fails compilation.

Object tables contain `f_id`, `f_version`, nullable `f_is_uploaded`, `f_hash`,
`f_ext`, and timestamps. `t_obj_prom_task` contains the durable task envelope,
including `f_topic`. These tables deliberately avoid business foreign keys and
business lifecycle checks; mutable policy stays in application code.

Raw SQL, runtime table names, `QueryableByName`, and schema-erasing RDB traits
are forbidden in ObjDept.

## Operations and values

`ObjDept<B, C>` drives:

- `ListObjMetas<B>` for newest persisted metadata;
- `GenObjUrls<B>` for origin and optional thumbnail URLs derived from
  supplied metadata versions;
- `GenObjSlot<B>` for allocation and a signed write capability;
- `GenObjSlots<B>` for batch allocation and signed write capabilities;
- `ClearObjs<B>` for clearing current files while their owning business
  entities remain active;
- `DeleteObjs<B>` for ending object lifecycles together with permanently
  deleted business entities.

Operation structures expose ordinary associated constructors and keep their
phantom marker fields private. Callers import and invoke the real operation
type directly so rust-analyzer navigation reaches its definition; no operation
construction macro is used. `MarkObjUploaded<B>` supports both independent Run and transaction-scoped Step. It returns `bool`, bound as
`marked` and handled at the immediate use-case boundary.

`ObjSlotSpec` carries `id`, `hash`, `ext`, `content_type`, and `byte_len`.
`ObjMeta` carries `key`, `is_available`, `hash`, and `ext`. Availability is an
optimistic read decision, not proof that remote bytes were verified. Component failures
use `ObjDeptError` and `ObjDeptRest`.
`ObjUrls` carries `origin_url` and `thumbnail_url: Option<Url>`. The current
R2 pool provides a thumbnail; the optional field permits a future pool or
object kind without that capability.

`ClearObjs` makes the current files unavailable and guarantees that later
reservations cannot reuse an earlier generation. `DeleteObjs` ends the object
lifecycle and is valid only when the owning business identifiers will never be
reused for that object kind. Both record Delete work in the caller-owned
transaction before business mutation commits; neither performs remote I/O in
that transaction. A future object kind that permits business-ID reuse must use
`ClearObjs` or introduce a monotonic identity epoch.

## Pool and durable tasks

`ObjPool` directly exposes `gen_slot`, `gen_urls`, `has`, and `del`. It is
payload-neutral. Callers select original, optimized, and thumbnail URLs through `ObjUrlSpec`. Artwork exports select only the original URL; the R2 adapter does not infer image renditions from the file extension.

`ObjProm` owns reset, global claim, completion, retry, and operator transitions.
`ObjPromDefer<C>` records single or batched Check and Delete work inside the
caller-owned transaction. The RDB adapter is generated by `rdb_obj_prom!`
against `t_obj_prom_task` with ordinary typed Diesel queries. A batch uses
one bulk insert and one bulk identity/status read rather than one operation per
object.

Task identity is deterministic. A repeated defer is accepted only when its
immutable obligation matches. Conflicting data is an error. Claims use fencing
leases; completion, retry, and operator transitions require the exact lease.
Expired processing work returns to pending. Remote failures have no attempt
limit.

## Single actor

The generic actor lifecycle is always compiled. Only its typed RDB handler
support lives under `actor::rdb_impl` and follows the `rdb_impl` feature.

`ObjActor::new` immediately spawns exactly one event loop and returns one
`ObjActorDesc`. The loop claims globally across every topic and currently
processes tasks serially. Topic dispatch is a generated static match whose
branches call their typed handlers. The actor owns whether future topic-level
parallelism is introduced; composition never creates one actor per topic.

Shutdown cancels the loop and `ObjActorDesc::join` waits for exit. Each attempt
has a deadline, and each remote call has a shorter timeout. Physical `has` and
`del` calls occur outside RDB transactions.

## Check and Delete behavior

A Check becomes visible after the write capability expires plus grace time.

- The exact current version runs `has`, including a generation optimistically
  marked uploaded by the client-facing endpoint.
- Presence leaves or marks that exact generation available.
- Absence resets that exact generation to unavailable and completes the Check.
- Missing, stale, or retired state deletes the physical key idempotently.
- Future version or invalid state moves to operator repair.

Every Check snapshots the row's update revision before remote I/O. Its final
write compares the object ID, version, and that revision. A concurrent mark of
the same generation therefore causes a retry instead of being overwritten;
a newer generation is likewise never changed. Every lost compare-and-set
reloads the typed row and classifies the new state before deciding whether to
complete, retry, delete, or require repair.

Object presence is deliberately accepted as upload evidence. Check does not
read or compare remote content hashes because that verification cost is
currently incompatible with the upload-throughput requirement.

Delete work removes a missing, stale, or retired physical version
idempotently. A task targeting a pending, verified, or future current state
requires operator repair. Unresolved deletion work is never discarded because
of retry count.

## Business integration

Page, User, Team, and Comic tables no longer carry OSS lifecycle fields. Their
use cases select a logical object by business row id plus compile-time marker.
Reserve flows validate policy, then run `GenObjSlot` or `GenObjSlots` in the
owner transaction.
Read flows combine business data with explicit ObjDept metadata or URLs.
Delete and cascade flows run `DeleteObjs` in the same transaction as
business-row mutation. Workflows that keep their business entities but clear
current files run `ClearObjs`.

The mark-uploaded endpoints optimistically set the submitted exact current
ObjDept generation to uploaded. Origin and supported thumbnail URLs are
therefore immediately readable after a successful mark. This client claim does
not prove that the PUT succeeded: the delayed Check performs `has` for the same
generation and resets its uploaded flag when the object is absent. Exact-version
compare-and-set prevents that correction from changing a newer generation.

Both page-image reservation paths defer the chapter raw-provide advancement
task in the same transaction. That handler loads the chapter's complete page
set and every `PageImage` metadata row. It advances the stage only when all
images are available; unavailable images leave the task available for a later
attempt without consuming a failure budget.

## Acceptance

- dependency direction remains `rdb-core -> obj-dept -> server`;
- main and Harn own one total `NormObjDept` and one actor descriptor;
- every supported object is registered only through `objs_def!`;
- generated Rust identifiers do not use a double-underscore prefix;
- generated code remains fully Diesel type checked;
- multi-object lifecycle and task persistence use bounded batch statements;
- Check/Delete work is isolated in `t_obj_prom_task`;
- no image-specific pool or old unified-prom image handler remains;
- all image and chapter artwork flows use ObjDept operations;
- all page-image upload paths can trigger automatic stage advancement;
- full formatting, all-feature compilation, tests, custom linters, and schema
  regeneration pass.

## Chapter artwork integration

Each chapter owns one `ChapterArtwork` slot in `t_chapter_artwork`, registered
in the same typed manifest and served by the existing object actor. Artwork
keys use `chapter_artwork/{chapter_id}-{version}.{ext}`. The frontend packs
and uploads the complete file directly; the API never transfers or parses its
bytes. Uploads accept safe file extensions and canonical Base64 SHA-256
identities, with an exact signed byte length bounded by
`[artwork].chapter_artwork_limit` (MiB, default 512).

`alloc_artwork` returns the current version even when deduplication produces
no PUT slot. New content replaces the current version immediately. Export
requires an available current object and returns its original public URL.
Artwork does not request optimized or thumbnail image URLs.

`mark_artwork_uploaded` locks the owning chapter and uses the transaction-scoped
`MarkObjUploaded` operation. Availability, completion of `TypesetRedraw`, and
its `ArtworkUpload` workflow record commit together. The workflow-completion
event is emitted after commit only on a real transition. Repeated confirms do
not repeat completion; a stale generation cannot confirm the current object.
The delayed existence check can revoke availability without reverting the
business stage.

Publishing clears the artwork alongside page images in the publication
transaction. Published chapters reject allocation and confirmation. Chapter
deletion, hierarchy sweeping, and comic archival delete artwork associations
and defer physical deletion through the existing durable task mechanism.
