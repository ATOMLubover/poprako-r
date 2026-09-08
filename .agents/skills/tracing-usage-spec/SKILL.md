---
name: tracing-usage-spec
description: Apply span, event, redaction, and single-error-log rules when adding or reviewing instrumentation, logs, or external error conversions.
---

# Tracing usage

Draw spans around observable operations, not pure data transformation.

| Location | Convention |
| --- | --- |
| Public fallible server use case | `#[instrument(level = "info", skip(...))]` |
| HTTP handler | `#[instrument(level = "info", skip_all)]` |
| RDB, object storage, JWT, Prom, effect, scheduler I/O | Useful operation boundaries, normally `skip_all` |
| `complex`, `model`, `data`, `value`, conversions | No span unless performing observable I/O |
| Constructors, clones, plain accessors, including Harn | No span |

## Fields and redaction

- Skip ports, contexts, and connection handles.
- Never emit credentials, JWTs, plaintext passwords, presigned URLs, or private
  payloads, including through an error's Debug output.
- Keep safe business instruction fields needed to reconstruct the operation
  in use-case spans. For large or secret-bearing DTOs, skip the whole value
  and record the necessary non-secret fields explicitly.
- Use stable structured fields such as `resource_id`, `operation`,
  `err_variant`, and `err_message`; keep event messages static.
- Import `tracing::instrument` and use its bare attribute name. Invoke event
  macros as `tracing::info!`, `tracing::warn!`, etc., without importing them.

## Events and errors

- Emit events for lifecycle state, retries, intentional error consumption,
  application error construction, and external error conversion.
- Before an adapter converts an external SDK, driver, or client error into an
  application error, emit exactly one event with the original error's Debug
  diagnostics, preserving variants/source information and redacting secrets.
  This conversion site is the error-production leaf.
- Returning `Err` through `?` is propagation. Do not emit another event
  for the same failure or configure `#[instrument]` to do so automatically.
- Use [error-handling-spec](../error-handling-spec/SKILL.md) when classifying
  or converting server application errors.
