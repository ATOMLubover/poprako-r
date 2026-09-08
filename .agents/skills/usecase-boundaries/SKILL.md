---
name: usecase-boundaries
description: Enforce domain-oriented use cases, transaction ownership, loaders, and repository contracts when changing or reviewing use cases or HTTP, effect, Prom, scheduler, and actor delivery boundaries.
---

# Use-case boundaries

## Core rule

Organize use cases by business capability, never by the mechanism that calls
them. A use case must remain unaware of whether its caller is HTTP, an effect
consumer, a Prom worker, a scheduler, a test, or another delivery mechanism.

Use this direction:

```text
delivery adapter -> domain-oriented use case -> ports/Oper -> adapter
```

Do not introduce caller-oriented use-case modules such as `usecase::effect`,
`usecase::prom`, `usecase::http`, `usecase::scheduler`, or
`usecase::task_handler`.

## Place behavior by domain

- Put entity lifecycle behavior in that entity's use-case module.
- Put cross-entity behavior under the domain capability that owns the business
  outcome, not under the transport that initiated it.
- Accept domain inputs such as identifiers, values, and instructions. Do not
  accept queue messages or effect event payloads merely because one current
  caller uses them.
- Keep pure computation and permission helpers in `complex`; keep port
  orchestration, transaction ownership, and permission enforcement in `usecase`.
- Pure rules receive already-loaded concrete models. They must not import
  Orchestra, repository traits/operations, or Prom operations/tasks, or drive
  transactions. Do not pass `Option<Model>` into permission helpers.
- Keep business permission checks out of HTTP handlers and RDB adapters;
  use cases enforce them through pure domain helpers where appropriate.

For example, a background message that attempts a chapter stage transition
must call a chapter-stage use case. An event that creates system mail must call
a system-mail use case. The message or event type is decoded by its delivery
adapter before the use case is called.

## Transactions and reusable reads

- A use case owns its transaction through `Nucl::coord`. Use `.run_on(repo)`
  for independent operations and `.step_on(repo, context)` inside that
  transaction, with the required context and isolation-level capabilities.
- Bind transaction output before returning or converting it. For unit output,
  await the transaction, then return `accept(())` or the nearby equivalent.
- Put reusable chains of at least two repository reads behind associated
  methods on `*Loader` under private `usecase::internal`. Do not wrap a single
  operation in a loader. Model-returning methods include the model suffix,
  such as `load_info_from_*` or `load_infos_from_*`.
- Pass the repository independently from `LoadMode`. Use `LoadMode::Run`
  only when every operation supports Run, and `LoadMode::Step` with the
  caller-owned context when every operation supports Step.
- Pass loaded models instead of repeating their identifiers. A loader must
  classify a missing required model as an error before invoking pure rules.

## Delivery-adapter responsibilities

Effect consumers, Prom workers, schedulers, and HTTP handlers may:

- receive, decode, and validate their transport envelope;
- select the appropriate domain use case;
- pass domain-oriented arguments and ports;
- apply delivery policy such as acknowledgement, retry, wait, or dead-letter;
- log intentionally consumed failures.

They must not query or mutate domain entities through repository `Oper`s,
assemble domain write models, enforce permissions, or own business transaction
boundaries.

Prom queue lifecycle operations such as claim, retry scheduling, acknowledgement,
and dead-lettering remain infrastructure operations and do not need domain use
cases. Repository adapter implementations likewise implement ports rather than
call use cases.

## Repository contract rule

Declare a domain repository `Oper` only when production use-case code consumes
it. A test-only or adapter-only reference does not justify a public operation.
When removing an Oper, also remove its capability bound, RDB implementation,
mock implementation, dead helper code, and tests that exist only for that
obsolete contract.

Do not satisfy this rule with a pass-through use case whose only purpose is to
create a reference. Move the complete business operation to its proper domain
use case and keep delivery policy at the caller boundary.

## Review checklist

1. Search for `run_on` and `step_on` outside `src/usecase` and repository
   adapters. Classify every result.
2. Search for imports from `part::repo::oper` outside `src/usecase`,
   `src/part`, and repository adapter implementations.
3. Confirm HTTP, effect, Prom, actor, and scheduler code calls domain-named use
   cases rather than composing repository operations.
4. Reverse-check every declared domain Oper against non-test production
   use-case references.
5. Reject use-case module or function names that describe a caller or delivery
   mechanism instead of a business capability.
6. Preserve retry/acknowledgement semantics in the delivery adapter and
   transaction/business semantics in the use case.

Report infrastructure exceptions explicitly so an empty search result is not
mistaken for an unexamined boundary.
