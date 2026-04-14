Read the implementation plan at:
  /home/kristoferlund/gh/ostt/specs/Phase 5/PLAN.md

Read the session notes at:
  /home/kristoferlund/gh/ostt/specs/Phase 5/SESSION.md

Read every spec file in:
  /home/kristoferlund/gh/ostt/specs/Phase 5/

Find the next incomplete SECTION in PLAN.md (the first section that has
unchecked tasks, e.g., 5.1.A, 5.2.A, 5.2.B, or 5.2.C).
Read the corresponding spec file for context.

Study the relevant source files in the target codebase at /home/kristoferlund/gh/ostt/src/
before making any changes. Understand existing patterns, imports, and conventions.
Key reference files for this phase:
  - src/app.rs (CLI definition, command routing)
  - src/commands/mod.rs (module registration)
  - src/commands/record.rs (record handler — output pattern, TUI lifecycle)
  - src/commands/retry.rs (retry handler — output pattern, keywords loading)
  - src/commands/transcribe.rs (transcribe handler — output pattern)
  - src/history/storage.rs (HistoryManager, get_transcription pattern)
  - src/process/mod.rs (process module exports)
  - src/process/execute.rs (execute_action dispatcher)
  - src/process/picker.rs (show_action_picker, PickerResult)
  - src/config/file.rs (ProcessConfig, ProcessAction, get_action)
  - src/keywords/mod.rs (KeywordsManager)
  - src/clipboard.rs (copy_to_clipboard)

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
  is a group like "5.1.A" or "5.2.B" — identified by a #### heading in the plan.
- Do not skip tasks. Do not reorder tasks.
- Only modify files in the target codebase (/home/kristoferlund/gh/ostt/src/), PLAN.md,
  and SESSION.md. Do not create or modify any other files.
- If a verification step fails, fix the issue and retry. If it fails a second time on
  the same task, mark the task with `[!]` in PLAN.md, git commit all partial work, and
  STOP the session.
- After completing all tasks in the current section, git commit all changes.
- STOP after completing one section. Do NOT continue to the next section.

After stopping, APPEND a session summary to the end of:
  /home/kristoferlund/gh/ostt/specs/Phase 5/SESSION.md

Use the heading format: ## Session N: Spec X.Y.Z — <title>
(Increment N based on how many sessions already exist in the file.
If the file does not exist, create it and start with Session 1.)

Include in the summary:
- What was accomplished
- Obstacles encountered
- Out-of-scope observations (things noticed but not acted on)
