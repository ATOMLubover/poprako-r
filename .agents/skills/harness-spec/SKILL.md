---
name: harness-spec
description: Keep Harn storage and application-port wiring aligned when changing Harn, AppHarn, main startup composition, or test composition.
---

# Harness storage and composition

`src/harn.rs` defines `Harn<N, R, O, P, A, D>`, which stores already-composed
parts in an `Arc`-backed inner value. It contains no business logic, actor
handler, background task, or composition factory.

- Keep `HarnInner`, the constructor, and accessors aligned:
  `config`, `nucl`, `repo`, `obj_dept`, `prom`, `auth`, `develop`.
- The constructor shape is
  `Harn::new(config, (nucl, repo, obj_dept, prom, auth, develop))`.
- Keep the concrete `AppHarn` alias in `src/api/http/state.rs` aligned with
  the production adapters composed in `src/main.rs`.
- Express transaction levels, context compatibility, and repository
  capabilities at the consuming port/use-case boundaries. Do not invent a
  context generic or port bounds on the storage-only harness.
- Add a harness parameter only for a required long-lived production port.
  Do not add a second harness or forwarding layer.
- Reuse current mock adapters and module-local fixtures for use-case tests;
  see [test-spec](../test-spec/SKILL.md) when changing test composition.
- Constructors, cloning, and accessors have no tracing spans; see
  [tracing-usage-spec](../tracing-usage-spec/SKILL.md) for I/O instrumentation.
