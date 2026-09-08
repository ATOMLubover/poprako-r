---
name: data-dto-boundaries
description: Define and review Instr, Val, and View roles, fields, conversions, serialization, and dependencies under src/data.
---

# Data DTO boundaries

`View` is the smallest reusable, request/response-independent presentation
unit. It must not encode request intent, mutation instructions, or
endpoint-specific response meaning.

`Instr` is request-only input and may contain request validation and intent.
`Val` is the direct response-only result of a use case or endpoint. It owns
aggregation, list metadata, positional alignment, operation results, and
upload reservations.

## Dependencies

Arrows below mean imports or contained types, not conversion data flow.

| Dependency | Allowed |
| --- | --- |
| `Val -> View` | Yes; normal response composition |
| `View -> View` | Yes; each fragment must remain independently meaningful |
| `View -> Instr / Val` | No |
| `Instr -> View / Val` | No |
| `Val -> Instr` | No |

A Val may contain only scalars, or aggregate Views, optional Views, collections,
counters, and identifiers. A View may convert from a model projection or shared
value object without acquiring request/response-specific meaning.

Do not use Instr as a response or reusable presentation fragment. Do not rename
a response aggregate to View merely because most of its fields are Views.

## Placement and review

- Request types live in `data::instr` and end in `Instr`.
- Direct results live in `data::val` and end in `Val`.
- Reusable fragments live in `data::view` and normally end in `View`.
  Exposed model `*Info` projections end in `InfoView`.
- Check the type's meaning, module, suffix, imports, fields, conversions, and
  serialization together. Keep endpoint-specific aggregation on Val.
- Read nearby DTOs and `src/data.rs` before introducing a category or
  dependency pattern. Follow
  [struct-field-blank-lines](../struct-field-blank-lines/SKILL.md) for grouping.

For example, `ListComicInfosVal { comics: Vec<ComicInfoView>, ... }` is valid
response composition; `UserInfoView { result: CreateUserVal }` and
`CreateUserInstr { preview: UserInfoView }` violate the matrix above.
