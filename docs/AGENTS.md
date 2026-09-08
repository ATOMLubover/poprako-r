# Documentation

- Keep source paths, commands, and API contracts aligned with active code.
- Before removing a plan, checklist, or generated artifact, check its purpose,
  references, and maintenance or regeneration path within the requested scope.
  Mark historical status clearly when the document still has a purpose.
- Regenerate `swagger.json` from the repository root with
  `cargo run -p poprako-swagger > docs/swagger.json` when the API contract
  changes. Verify it with `sh scripts/ci-openapi-check.sh`.
- Integration-test documentation lives beside its Deno suite under
  `tests/integration-tests/`; follow `tests/AGENTS.md` for synchronization.
