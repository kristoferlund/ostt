Read the implementation plan at:
  /home/kristoferlund/gh/ostt/specs/Phase 2/PLAN.md

Read the session notes at:
  /home/kristoferlund/gh/ostt/specs/Phase 2/SESSION.md

Read every spec file in:
  /home/kristoferlund/gh/ostt/specs/Phase 2/

Find the next incomplete SUB-SECTION in PLAN.md (the first sub-section that has
unchecked tasks, e.g., 2.1.A, 2.1.B, 2.1.C, or 2.1.D).
Read the corresponding spec file for context.

Study the relevant source files in the target codebase at /home/kristoferlund/gh/ostt/src/
before making any changes. Understand existing patterns, imports, and conventions.

Read the session notes from previous sessions in SESSION.md (if the file exists) to
understand what has already been accomplished and any obstacles encountered.

Implement tasks in the exact order listed in PLAN.md. Do not skip or reorder tasks.

After each verification task (cargo check, cargo clippy, cargo test), confirm it passes
before moving on.

Mark each task complete in PLAN.md by changing `- [ ]` to `- [x]` as you finish it.

Rules:
- SCOPE: Complete only ONE sub-section per session (at most 10 tasks). A sub-section
  is a group like "2.1.A", "2.1.B", etc. — identified by a #### heading in the plan.
- Do not skip tasks. Do not reorder tasks.
- Only modify files in the target codebase (/home/kristoferlund/gh/ostt/src/), PLAN.md,
  and SESSION.md. Do not create or modify any other files.
- If a verification step fails, fix the issue and retry. If it fails a second time on
  the same task, mark the task with `[!]` in PLAN.md, git commit all partial work, and
  STOP the session.
- After completing all tasks in the current sub-section, git commit all changes.
- STOP after completing one sub-section. Do NOT continue to the next sub-section.

After stopping, APPEND a session summary to the end of:
  /home/kristoferlund/gh/ostt/specs/Phase 2/SESSION.md

Use the heading format: ## Session N: Spec 2.1.X — <title>
(Increment N based on how many sessions already exist in the file.)

Include in the summary:
- What was accomplished
- Obstacles encountered
- Out-of-scope observations (things noticed but not acted on)
```

