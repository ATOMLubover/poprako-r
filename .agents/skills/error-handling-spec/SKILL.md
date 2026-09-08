---
name: error-handling-spec
description: Classify, convert, propagate, and review PopRaKo server application, adapter, transaction, and HTTP errors.
---

# Error handling

The server error surface is
`crate::result::{BaseError, BaseRest, ExpectedVariant, accept}`.
Infrastructure workspace crates keep their own errors; classify them at the
server adapter boundary rather than adding server dependencies to those crates.

## Classification

- Use `BaseError::Expected` for client-correctable argument, authentication,
  and permission conditions. Choose `ExpectedVariant::{Args, Auth, Perm}`
  and an established `poprako_util::i18n::trl` key for client-visible text.
- Use `BaseError::Unrecoverable` for infrastructure failures, corrupt
  persisted state, and violated internal invariants.
- Preserve an existing BaseError with `?` when no new classification is
  needed. Do not wrap an error merely to restate the function name.
- Use `accept(value)` for simple successful results where it is the nearby
  convention.

## Conversion and propagation

- Before converting external SDK, driver, or client errors, apply
  [tracing-usage-spec](../tracing-usage-spec/SKILL.md): the conversion leaf
  must record the original error once, with redaction. A `map_err`, `From`,
  or helper must not silently discard that diagnostic responsibility.
- `From<NuclError<...>> for BaseError` in `src/part/nucl.rs` converts
  backend and step errors from `Nucl::coord`.
- Server Diesel/pool failures use `crate::shared::result` helpers. Use
  `.optional()` when absence has business meaning, then classify `None`
  with the appropriate translated expected error.
- HTTP handlers return `HttpResult<T>`, propagate application errors with
  `?`, and use `Accept as _` or `no_content` for success. Only HTTP facts,
  such as mismatching path/body identifiers, are classified in handlers.
- `From<BaseError> for HttpError` owns HTTP mapping and must never expose
  unrecoverable details to clients.

Review classification at the narrowest informed boundary, preserve the
existing transaction mapper, and avoid duplicate business mapping in handlers.
