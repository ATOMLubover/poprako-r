---
name: struct-field-blank-lines
description: Apply semantic blank-line grouping when creating, changing, moving, or reviewing named-field Rust structs in any workspace crate; grouping is mandatory for five or more fields.
---

# Struct field grouping

Group named fields by logical concern, separating groups with one blank line.
This is mandatory for structs with five or more fields. Smaller structs may
omit grouping when their fields form one coherent concept.

- Keep the entity `id` alone, followed by a blank line when fields remain.
- Keep `created_at` and `updated_at` together in the final logical block,
  separated from preceding fields.
- DTOs mirror their model's grouping after omitted fields are removed.
- Preserve established domain order. Do not reorder fields solely to group
  them; if timestamp placement conflicts with existing order, retain order
  unless the task explicitly changes the contract.
- Keep documentation comments attached to their fields, not separated by the
  group boundary.
- Apply these rules across workspace crates, including macro implementation
  structs, test structs, and generated source templates. Review macro output;
  change its source template rather than hand-editing generated artifacts.
- Tuple/unit structs, struct literals, and enum variant fields are outside
  this rule.

For example, an entity may have these groups: `id`; parent identity and
position; image metadata; progress counters; lifecycle timestamps.

Inspect corresponding models, DTOs, conversions, and siblings when grouping
is ambiguous. Limit edits to necessary blank lines; report unresolved domain
ambiguity for review instead of inventing a grouping convention.
