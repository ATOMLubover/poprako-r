---
name: harness-spec
description: Keep Harn storage and application-port wiring aligned when changing Harn, AppHarn, main startup composition, or test composition.
---

# Harness storage and composition

`src/harn.rs` defines `Harn<N, R, O, P, A, D>`, which stores already-composed
parts in an `Arc`-backed inner value. It contains no business logic, actor
handler, background task, or composition factory.

- Production application adapters and actors are instantiated and injected
  only in `main`; tests and benchmarks have their own composition roots.
  Adapter constructors may allocate their own SDK clients and internal
  channels, but must not choose or recreate other application ports.
- Aggregate independent constructions at each dependency level with tuple
  bindings. Use domain-specific local names and `send` / `recv` for channel
  halves; keep shutdown steps sequential when one worker produces for another.
- Construct producers independently of consumers. Actor constructors do not
  spawn tasks; `run_detach(self)` returns each actor's non-cloneable descriptor.
  `Harn` stores producers, while `main` owns descriptors and calls `cancel`
  followed by consuming `join`. Dropping a producer clone never cancels an actor.
- Clone the application `HybRepo` to share both its database core and online
  user state. Do not construct a new repository inside a consumer.
- Prom and scheduler detached execution use explicitly injected `RdbNucl`
  coordinators: Orchestra's generic `Nucl::coord` does not promise a `Send`
  future. Business dispatch remains generic for mock composition.
- After HTTP finishes, cancel scheduler and Prom together and join them with
  `tokio::join!`. Then cancel and join effect and object workers together.
  Preserve these dependency levels so downstream consumers remain available
  until their producers finish. Continue cleanup after a join failure and
  report it at the owner.

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
