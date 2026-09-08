---
name: module-splitting-conventions
description: Govern Rust module splitting only when a module has reached or will reach the project's 600-line file limit. Use before extracting submodules for line-limit compliance; do not use to reorganize modules that remain below the limit.
---

# Module splitting conventions

Use module extraction as a last-mile response to the 600-line limit, not as a
general tidiness technique. A cohesive module below the limit is preferable to
several small classification modules.

## Required workflow

Complete these steps in order and stop before editing until the user confirms
the proposed split:

1. Measure the unsplit module. Count the parent plus implementation that would
   return to it if its direct submodules were inlined. Run
   this skill's [audit script](scripts/audit_module_split.py) for an existing
   split. From the repository root, run
   `python3 .agents/skills/module-splitting-conventions/scripts/audit_module_split.py <parent.rs>`.
2. Inventory concrete responsibilities and map every relevant function or type
   to one responsibility.
3. Propose the fewest stable, business-named child modules needed to keep every
   Rust file strictly below 600 lines. Include the function/type mapping and
   the projected line counts.
4. Ask the user to confirm that exact proposal. Do not create, move, or edit
   modules before confirmation.
5. After confirmation, perform the extraction without changing public API,
   behavior, errors, transactions, or tracing.
6. Re-run the audit and the root AGENTS formatting/compilation checks, plus
   affected tests. Run repository linters only under the root CI/request rule.

## Boundary rules

- Keep the parent module as the architectural entry point for its layer. Trait
  implementations, public exports, dispatch, or format selection stay there
  when they define how the layer is entered.
- Give each child one complete, stable implementation responsibility. Prefer a
  domain capability such as `avatar`, `coordination`, or a complete format
  flow over a technical bucket.
- Extract only enough children to satisfy the limit. Do not create a child
  merely to make a directory look symmetrical.
- Never introduce catch-all names such as `helper`, `helpers`, `impls`,
  `operation`, `operations`, `orchestra`, or `statement` as module boundaries.
- Do not extract fragments that exist only to hold a few unrelated functions,
  re-exports, or forwarding wrappers. The child must own the implementation it
  names.
- If the measured unsplit module is below 600 lines and the planned change will
  keep it below 600, report that no line-limit split is warranted and stop.

Import style and forbidden-name enforcement belong to the repository linters;
do not recreate those checks in this skill.

The audit and authoring rule require files strictly below 600 lines. The CI
file-length script currently allows exactly 600; its pass does not replace
this stricter check. Reusable split scenarios live in
[evals/evals.json](evals/evals.json).
