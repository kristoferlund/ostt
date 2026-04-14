Read the implementation plan at:
  /home/kristoferlund/gh/ostt/specs/Phase 6/PLAN.md

Read the session notes at:
  /home/kristoferlund/gh/ostt/specs/Phase 6/SESSION.md

Read the spec file:
  /home/kristoferlund/gh/ostt/specs/Phase 6/6.1 — Processing Animation with Status Label.md

Find the next incomplete SECTION in PLAN.md (the first section that has
unchecked tasks, e.g., 6.1.A or 6.1.B).
Read the spec file for context.

Study the relevant source files in the target codebase at /home/kristoferlund/gh/ostt/src/
before making any changes. Understand existing patterns, imports, and conventions.
Key reference files for this phase:
  - src/transcription/animation.rs (TranscriptionAnimation struct, draw method)
  - src/commands/record.rs (transcribe_recording_with_animation, handle_record with processing flow)
  - src/process/execute.rs (execute_action dispatcher — add execute_action_with_animation here)
  - src/process/mod.rs (module re-exports)
  - src/process/picker.rs (ActionPicker — reference for Drop-based terminal cleanup guard pattern)
  - src/commands/process.rs (handle_process — caller to update)
  - src/commands/retry.rs (handle_retry — caller to update)
  - src/commands/transcribe.rs (handle_transcribe — caller to update)

Read the session notes from previous sessions in SESSION.md (if the file exists) to
understand what has already been accomplished and any obstacles encountered.

Implement tasks in the exact order listed in PLAN.md. Do not skip or reorder tasks.

CRITICAL — Update PLAN.md after EVERY completed task:
  After finishing each task, IMMEDIATELY edit PLAN.md to change `- [ ]` to `- [x]`
  for that task BEFORE starting the next task. This is essential for crash recovery —
  if the session is interrupted, the plan must reflect what has already been done.
  Do NOT batch these updates. Do NOT wait until the end of the section.

After each verification task (cargo check, cargo clippy, cargo test), confirm it passes
before moving on.

Rules:
- SCOPE: Complete only ONE section per session (at most 10 tasks). A section
  is a group like "6.1.A" or "6.1.B" — identified by a #### heading in the plan.
- Do not skip tasks. Do not reorder tasks.
- Only modify files in the target codebase (/home/kristoferlund/gh/ostt/src/), PLAN.md,
  and SESSION.md. Do not create or modify any other files.
- If a verification step fails, fix the issue and retry. If it fails a second time on
  the same task, mark the task with `[!]` in PLAN.md, git commit all partial work, and
  STOP the session.
- After completing all tasks in the current section, git commit all changes.
- STOP after completing one section. Do NOT continue to the next section.

After stopping, APPEND a session summary to the end of:
  /home/kristoferlund/gh/ostt/specs/Phase 6/SESSION.md

Use the heading format: ## Session N: Spec X.Y.Z — <title>
(Increment N based on how many sessions already exist in the file.
If the file does not exist, create it and start with Session 1.)

Include in the summary:
- What was accomplished
- Obstacles encountered
- Out-of-scope observations (things noticed but not acted on)

