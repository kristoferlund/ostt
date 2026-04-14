Read the implementation plan at:
  /home/kristoferlund/gh/ostt/specs/Phase 1/PLAN.md

Read the session notes at:
  /home/kristoferlund/gh/ostt/specs/Phase 1/SESSION.md

Read every spec file in:
  /home/kristoferlund/gh/ostt/specs/Phase 1/

Find the next incomplete spec group in PLAN.md (the first group that has unchecked tasks).
Read the corresponding spec file for that group.

Study the relevant source files in the target codebase at /home/kristoferlund/gh/ostt/src/
before making any changes. Understand existing patterns, imports, and conventions.

Read the session notes from previous sessions in SESSION.md (if the file exists) to
understand what has already been accomplished and any obstacles encountered.

Implement tasks in the exact order listed in PLAN.md. Do not skip or reorder tasks.

After each verification task (cargo check, cargo clippy, cargo test), confirm it passes
before moving on.

Mark each task complete in PLAN.md by changing `- [ ]` to `- [x]` as you finish it.

Rules:
- Do not skip tasks. Do not reorder tasks.
- Only modify files in the target codebase (/home/kristoferlund/gh/ostt/src/), PLAN.md,
  and SESSION.md. Do not create or modify any other files.
- If a verification step fails, fix the issue and retry. If it fails a second time on
  the same task, mark the task with `[!]` in PLAN.md, git commit all partial work, and
  STOP the session.
- After completing all tasks in the current spec group, git commit all changes.
- STOP after completing one spec group. Do not continue to the next spec group.

After stopping, APPEND a session summary to the end of:
  /home/kristoferlund/gh/ostt/specs/Phase 1/SESSION.md

Use the heading format: ## Session N: Spec X.Y — <title>
(Increment N based on how many sessions already exist in the file.)

Include in the summary:
- What was accomplished
- Obstacles encountered
- Out-of-scope observations (things noticed but not acted on)
