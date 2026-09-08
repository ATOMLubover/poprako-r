# Server module map

Use `src/lib.rs` for the authoritative module graph. HTTP delivery lives in
`api/http`, domain orchestration in `usecase`, pure rules in `complex`,
persisted models in `model`, DTOs in `data`, ports in `part`, adapters in
`part_impl`, background scheduling in `extra`, and shared domain types in
`value`. `harn.rs` stores the already-composed application parts.

Read the root skill routing for the affected layer. Workspace utility and
infrastructure crates have their own module graphs in their crate roots.
