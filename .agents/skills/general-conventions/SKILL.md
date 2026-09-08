---
name: general-conventions
description: Apply PopRaKo Rust naming, construction, imports, and source conventions when creating, changing, moving, or reviewing Rust in any workspace crate. Server layer rules apply only to the server crate.
---

# Rust conventions

## Scope and sources

Read the affected crate root and nearby code before choosing names, visibility,
or imports. `Cargo.toml` defines workspace membership; `src/lib.rs` defines the
server module graph. Infrastructure crates keep their own contracts.

Mechanical rules live in repository-relative `linters-extra/*/FORMAT.md`;
the runner is `sh linters-extra/run-check.sh`. Follow the root AGENTS rules
for when to run it. Do not reproduce its checks in skills.

## Types and construction

- Server persisted projections and list specs live in `model::read`;
  entries, modifications, replacements, and reservations in `model::write`.
- For DTO roles and suffixes, read
  [data-dto-boundaries](../data-dto-boundaries/SKILL.md) when changing DTOs.
- Use specific domain locals such as `comic_info`, `chapter_entry`,
  `cover_reservation`, and `system_mail_infos`.
- Bind domain payloads, including DTOs and write models, before passing them
  to operations. Construct one-shot Orchestra descriptors inline in the
  consuming `run_on` or `step_on` call.
- For transaction ownership, pure rules, and reusable read chains, read
  [usecase-boundaries](../usecase-boundaries/SKILL.md) when touching those areas.
- Keep comments and public documentation in English and about current
  behavior; remove retired-design commentary from active code.
- Follow the root control-flow, statement spacing, and file-length rules.
  Use `foo.rs` plus `foo/` for modules. Read
  [module-splitting-conventions](../module-splitting-conventions/SKILL.md)
  before extracting modules for length compliance.
- Apply [struct-field-blank-lines](../struct-field-blank-lines/SKILL.md)
  when creating, changing, or reviewing named-field structs.

## Public contracts and imports

- Keep implementation details private; document every public contract.
- Import derive and attribute macros explicitly and use their bare names.
  Tracing event macros use `tracing::...!`; detailed instrumentation rules
  belong to [tracing-usage-spec](../tracing-usage-spec/SKILL.md).
- Import traits used only for method resolution as `as _`.
- Use generated Diesel table modules through their full local path, never
  through a `schema::` alias.
