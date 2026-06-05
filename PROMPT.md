You are working in target codebase `/Users/kristoferlund/gh/ostt`.

Read `/Users/kristoferlund/gh/ostt/specs/first-time-ux/PLAN.md`, `/Users/kristoferlund/gh/ostt/specs/first-time-ux/SESSION.md` if it exists, and the spec files folder `/Users/kristoferlund/gh/ostt/specs/first-time-ux`.

Find the next incomplete section or sub-section in PLAN.md. This is the first section or sub-section containing unchecked tasks. SCOPE is exactly that one section or sub-section only. Do not continue to the next section or sub-section.

Read `/Users/kristoferlund/gh/ostt/specs/first-time-ux/spec.md`, then study the source files listed for the scoped section in PLAN.md and the immediate callers/types/utilities needed to make safe changes. Do not do broad codebase exploration.

Before editing, read prior session notes and recent commits. Identify helpers/APIs/patterns already introduced for this feature and reuse them unless clearly unsuitable. Do not create parallel helpers for the same concern without documenting why.

Implement tasks in order. No skipping tasks. No reordering tasks. If the scoped spec or task is vague, conflicts with the clarified non-goals, or risks over-implementing beyond the listed files and acceptance criteria, stop and ask the user before editing further.

After completing each task, update PLAN.md IMMEDIATELY before starting the next task by changing that task from `- [ ]` to `- [x]`. Do not batch PLAN.md updates. This is required for crash recovery.

Run the verification task(s) listed in the section as they are reached. If verification fails, fix and retry once. If the same verification task fails twice, mark that task as `- [!]`, append the session summary to SESSION.md, git commit partial work, and stop.

Restrict file modifications to `/Users/kristoferlund/gh/ostt`, `/Users/kristoferlund/gh/ostt/specs/first-time-ux/PLAN.md`, and `/Users/kristoferlund/gh/ostt/specs/first-time-ux/SESSION.md` only.

Append, do not overwrite, a session summary to `/Users/kristoferlund/gh/ostt/specs/first-time-ux/SESSION.md`. Use heading `## Session N: Spec X.Y — <title>` with N incremented from existing sessions. Include what was accomplished, decisions made, files changed, helpers/APIs introduced or reused, verification results, constraints for later sessions, obstacles encountered, open questions, and out-of-scope observations.

Before stopping, git commit all changes. Commit only files changed for this scoped section or sub-section plus PLAN.md and SESSION.md. Stop after one section or sub-section even if more tasks remain elsewhere.
