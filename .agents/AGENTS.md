# Agent instructions and skills

- Each skill requires valid YAML frontmatter with `name` and `description`;
  the name must match its directory. Describe concrete task triggers.
- Keep global guardrails in the root `AGENTS.md`, subtree-only guidance in
  that subtree, and specialized rules in one owning skill. Link to the owner
  instead of duplicating its detailed rules.
- Use current code and checked-in scripts to verify factual descriptions.
  An implementation deviation does not authorize weakening a requirement.
- Preserve explicit constraints when shortening text. Keep only examples
  that clarify a decision; remove obsolete paths and scaffold boilerplate.
- Resolve skill-relative links from the containing skill directory. Mark
  repository-relative commands and paths where that distinction matters.
- Keep reusable evaluations and scripts with their skill. Do not keep
  one-off evaluation output directories in the skill discovery tree.
- After edits, validate skill frontmatter, referenced paths, and the diff
  for lost requirements, conflicting instructions, or changes outside scope.
