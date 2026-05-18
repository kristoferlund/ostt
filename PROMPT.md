Read `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 3 - Config & Auth/PLAN.md`, `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 3 - Config & Auth/SESSION.md` if it exists, and the spec files in `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 3 - Config & Auth/`.

Target codebase: `/Users/kristoferlund/gh/ostt`.

Find the next incomplete section or sub-section in `PLAN.md`: the first section or sub-section containing unchecked `- [ ]` tasks. That section or sub-section is the entire scope for this session. Do not work on any later section or sub-section.

Read the spec file named by that section and study the relevant source files in the target codebase. Also read notes from previous sessions in `SESSION.md` before changing code.

Implement tasks strictly in order. Do not skip tasks. Do not reorder tasks. Scope is one section or sub-section only.

Critical crash-recovery rule: update `PLAN.md` immediately after completing each task, changing that task from `- [ ]` to `- [x]`, before starting the next task. Do not batch these updates.

Run the verification command listed for each verification task when you reach it. If verification fails, fix and rerun once. If it fails a second time, mark the task with `[!]`, append notes to `SESSION.md`, commit partial work, and stop.

Restrict file modifications to the target codebase, `PLAN.md`, and `SESSION.md` only. Do not modify files outside `/Users/kristoferlund/gh/ostt`, except for `PLAN.md` and `SESSION.md` in the spec folder.

Before stopping, append a session summary to the end of `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 3 - Config & Auth/SESSION.md`. Do not overwrite existing notes. Use heading `## Session N: Spec X.Y - <title>` with `N` incremented from prior sessions. Include what was accomplished, obstacles encountered, and out-of-scope observations.

Git commit all changes before stopping. Stop after one section or sub-section, even if all tasks passed. Do not continue to the next section or sub-section.
