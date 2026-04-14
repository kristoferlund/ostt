
~~~
Read the implementation plan at:
  /Users/kristoferlund/gh/ostt/specs/Phase 1/PLAN.md

Find the first spec group that has incomplete tasks (unchecked `- [ ]` items).
Read the corresponding spec file in:
  /Users/kristoferlund/gh/ostt/specs/Phase 1/

Study the relevant source files in the target codebase at:
  /Users/kristoferlund/gh/ostt/

Read SESSION.md in the same directory as PLAN.md for notes from previous sessions.

Then implement the tasks for that ONE spec group, in order. Follow these rules:

1. Implement tasks sequentially — no skipping, no reordering.
2. After each task, run the verification command (`cargo check`, `cargo clippy -- -D warnings`, or `cargo test` as appropriate).
3. Mark each task complete in PLAN.md (`- [x]`) as you finish it.
4. If verification fails, fix the issue and retry. If it fails twice on the same task, mark it with `- [!]` in PLAN.md, git commit partial work, and STOP.
5. Only modify files in the target codebase (`/Users/kristoferlund/gh/ostt/`), PLAN.md, and SESSION.md. Do not modify the spec files.
6. When all tasks in the spec group are done, run the full verification protocol:
   ```
   cargo check && cargo clippy -- -D warnings && cargo test
   ```
7. Git commit all changes with a descriptive message.
8. APPEND a session summary to the END of SESSION.md (do NOT overwrite — read first, then add after the last line). Use heading `## Session N: Spec X.Y — <title>` (increment N). Include: what was accomplished, obstacles encountered, out-of-scope observations.
9. STOP. Do not continue to the next spec group.
~~~
