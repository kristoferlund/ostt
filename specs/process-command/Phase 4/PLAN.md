# Phase 4 — Implementation Plan

**Scope:** Spec 4.1 (Action Picker TUI)
**Target codebase:** `/home/kristoferlund/gh/ostt`
**Status:** Complete

---

## Dependency Order

```
Phase 1 (complete)           Phase 2 (complete)
  1.1 Action Config Types      2.1 AI CLI Tool Executor
  1.2 Input Resolution              |
       |                             |
       v                             v
Phase 3 (complete)                   |
  3.1 Bash Action Executor           |
  3.2 Action Dispatcher  <───────────┘
       |
       v
Phase 4
  4.1 Action Picker TUI
```

**Rationale:** Spec 4.1 depends on Phase 1 only for the `ProcessAction` and `ProcessConfig` types (used as input to the picker). It has no dependency on Phases 2 or 3 (the execution pipeline) — the picker only selects an action, it does not execute it. However, since Phases 1–3 are all complete, there are no blockers.

Phase 4 is a single spec (4.1) with a single module (`src/process/picker.rs`) and a one-line registration in `src/process/mod.rs`. The scope is small enough for a single section.

Execution order: **4.1 only.**

---

## Tasks

### Spec 4.1 — Action Picker TUI

#### 4.1.A — Module implementation and tests

- [x] **4.1.1** Create `src/process/picker.rs` with module-level doc comment, imports (ratatui, crossterm, anyhow), color constants (`BG`, `FG`, `HIGHLIGHT_BG`, `HELP_FG` matching history/keywords), and the `PickerResult` enum (`Selected(String)` / `Cancelled`)
- [x] **4.1.2** Implement the `ActionPicker` struct with fields: `terminal: Terminal<CrosstermBackend<Stdout>>`, `actions: Vec<ProcessAction>`, `list_state: ListState`, and a `cleaned_up: bool` flag. Implement `ActionPicker::new(actions: Vec<ProcessAction>) -> Result<Self>` with terminal setup (enable_raw_mode, EnterAlternateScreen — no mouse capture needed) and initial list_state selection at index 0
- [x] **4.1.3** Implement the `cleanup(&mut self) -> Result<()>` method (disable_raw_mode, LeaveAlternateScreen, show_cursor, with `cleaned_up` guard) and `impl Drop for ActionPicker` that calls cleanup — following the `KeywordsViewer` pattern
- [x] **4.1.4** Implement the `draw(&mut self) -> Result<()>` method: outer padding block, main block, vertical layout split into header (Length(3)), list area (Min(0)), footer (Length(1)). Render the OSTT logo header, a bordered `List` widget titled `" Process action "` with `highlight_symbol("> ")` and `HIGHLIGHT_BG` highlight style, and the help footer text `"↑/↓ select, ↵ confirm, esc/q cancel"` centered in `HELP_FG`
- [x] **4.1.5** Implement the `handle_key(&mut self, key: KeyEvent) -> Option<PickerAction>` method: Up/k moves selection up, Down/j moves selection down, Enter returns `PickerAction::Select` with the selected action's ID, Esc/q returns `PickerAction::Exit`. Define a private `PickerAction` enum (`Exit` / `Select(String)`)
- [x] **4.1.6** Implement the `run(&mut self) -> Result<PickerResult>` event loop: loop calling `draw()`, then `event::read()` with key dispatch via `handle_key()`. On `PickerAction::Exit` break and return `Cancelled`. On `PickerAction::Select(id)` break and return `Selected(id)`. Call `cleanup()` before returning
- [x] **4.1.7** Implement the public entry point `pub fn show_action_picker(actions: &[ProcessAction]) -> anyhow::Result<PickerResult>`: handle edge cases first — return error if `actions` is empty ("No processing actions configured. Add actions to ~/.config/ostt/ostt.toml"), return `Selected(id)` directly if only one action. Otherwise construct `ActionPicker` and call `run()`
- [x] **4.1.8** Register the module: add `pub mod picker;` to `src/process/mod.rs`
- [x] **4.1.9** Verify: `cargo check` and `cargo clippy -- -D warnings` pass
- [x] **4.1.10** Verify: `cargo test` passes (no new tests needed — the picker is a TUI widget that requires a terminal; the edge cases in `show_action_picker` are simple enough that verification via cargo check + clippy is sufficient)

---

## Verification Protocol

### After section 4.1.A

Run all three commands in sequence. All must pass:

```bash
cargo check
cargo clippy -- -D warnings
cargo test
```

### After all Phase 4 tasks are complete

Phase 4 has only one section, so the post-section verification above is also the final verification. Run the full suite one more time to confirm:

```bash
cargo check
cargo clippy -- -D warnings
cargo test
```

---

## Session Boundaries

- **One section per session.** Phase 4 has a single section (4.1.A) with 10 tasks. The agent completes 4.1.A, commits, and stops.
- **Do not continue to the next phase.** After completing 4.1.A, commit and stop.
- **Stop early on repeated failure.** If a verification command fails and the fix attempt also fails (two consecutive failures on the same task), mark the task with `[!]`, commit all partial work, and stop the session.
- **Commit before stopping.** The agent must `git commit` all changes before ending every session.

---

## Session Prompt Template

```
Read the implementation plan at:
  /home/kristoferlund/gh/ostt/specs/process-command/Phase 4/PLAN.md

Read the session notes at:
  /home/kristoferlund/gh/ostt/specs/process-command/Phase 4/SESSION.md

Read every spec file in:
  /home/kristoferlund/gh/ostt/specs/process-command/Phase 4/

Find the next incomplete SECTION in PLAN.md (the first section that has
unchecked tasks, e.g., 4.1.A).
Read the corresponding spec file for context.

Study the relevant source files in the target codebase at /home/kristoferlund/gh/ostt/src/
before making any changes. Understand existing patterns, imports, and conventions.
Key reference files for this phase:
  - src/history/ui.rs (primary TUI pattern reference)
  - src/keywords/ui.rs (secondary TUI pattern reference)
  - src/process/mod.rs (module registration)
  - src/config/file.rs (ProcessAction, ProcessConfig types)

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
  is a group like "4.1.A" — identified by a #### heading in the plan.
- Do not skip tasks. Do not reorder tasks.
- Only modify files in the target codebase (/home/kristoferlund/gh/ostt/src/), PLAN.md,
  and SESSION.md. Do not create or modify any other files.
- If a verification step fails, fix the issue and retry. If it fails a second time on
  the same task, mark the task with `[!]` in PLAN.md, git commit all partial work, and
  STOP the session.
- After completing all tasks in the current section, git commit all changes.
- STOP after completing one section. Do NOT continue to the next section.

After stopping, APPEND a session summary to the end of:
  /home/kristoferlund/gh/ostt/specs/process-command/Phase 4/SESSION.md

Use the heading format: ## Session N: Spec 4.1.A — <title>
(Increment N based on how many sessions already exist in the file.
If the file does not exist, create it and start with Session 1.)

Include in the summary:
- What was accomplished
- Obstacles encountered
- Out-of-scope observations (things noticed but not acted on)
```
