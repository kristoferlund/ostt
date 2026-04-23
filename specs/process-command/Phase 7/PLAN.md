# Phase 7 — Implementation Plan

**Scope:** Specs 7.1, 7.2, 7.3
**Target codebase:** `/home/kristoferlund/gh/ostt`
**Status:** Complete

---

## Dependency Order

```
7.2 — Miscellaneous Fixes (no dependencies, independent fixes)
  |
  ├─ Fix 4 adds mouse support to ActionPicker
  |
  v
7.1 — Unified TUI Lifecycle for Record Flow
  |   (depends on Phase 4, 5, 6 — all complete)
  |   (7.1 extracts render_picker_frame from picker.rs;
  |    7.2 Fix 4 adds mouse capture to picker.rs —
  |    do 7.2 first to avoid merge conflicts in picker.rs)
  |
  v
7.3 — Mouse Hover Highlight
      (depends on 7.2 Fix 4 for picker mouse support)
      (touches picker.rs after 7.1's render_picker_frame extraction)
```

**Rationale:**

- **7.2 first** — All four fixes are independent and small. Fix 4 (mouse scroll in picker) must be in place before 7.3. Doing 7.2 before 7.1 also means the picker.rs mouse imports are already present when 7.1 extracts the render function.
- **7.1 second** — The big refactor of `handle_record`. It extracts `render_picker_frame` from `picker.rs` and adds `render_action_picker` to `ui.rs`. This is the largest spec and is split into three sub-sections (A, B, C).
- **7.3 last** — Adds hover highlight to history viewer and action picker. Depends on 7.2's mouse support and builds on 7.1's refactored picker code.

Execution order: **7.2 → 7.1.A → 7.1.B → 7.1.C → 7.3**

---

## Tasks

### Spec 7.2 — Miscellaneous Fixes

#### 7.2

- [x] **7.2.1** In `src/config/mod.rs`, add `ProvidersConfig` to the re-export line so it reads: `pub use file::{AudioConfig, OsttConfig, ProvidersConfig, VisualizationType};`
- [x] **7.2.2** In `src/history/ui.rs`, add a `cleaned_up: bool` field to the `HistoryViewer` struct, initialize it to `false` in `new()`. In `cleanup()`, add an early-return guard: `if self.cleaned_up { return Ok(()); }` and set `self.cleaned_up = true;` before the existing cleanup code.
- [x] **7.2.3** In `src/process/bash.rs`, in `execute_bash_action`, after the `if !output.status.success()` check, add a special case for exit code 127: if `output.status.code() == Some(127)`, bail with `"Command not found. Make sure the command is installed.\nShell output: {stderr}"`. Keep the existing generic error for other codes.
- [x] **7.2.4** Update the `nonexistent_command_returns_clear_error` test in `src/process/bash.rs` to assert the error contains `"Command not found"` instead of `"Command exited with status"`.
- [x] **7.2.5** In `src/process/picker.rs`, add mouse capture to `ActionPicker`: import `EnableMouseCapture`, `DisableMouseCapture`, `MouseEventKind`; add `EnableMouseCapture` to the `execute!` call in `new()`; add `DisableMouseCapture` to the `execute!` call in `cleanup()`.
- [x] **7.2.6** In `src/process/picker.rs`, in `ActionPicker::run()`, change `event::read()` handling to match on `Event::Key` and `Event::Mouse`. For `Event::Mouse(mouse)`, handle `MouseEventKind::ScrollUp` → `self.list_state.select_previous()` and `MouseEventKind::ScrollDown` → `self.list_state.select_next()`.
- [x] **7.2.7** Verify: `cargo check` and `cargo clippy -- -D warnings` and `cargo test` all pass.

---

### Spec 7.1 — Unified TUI Lifecycle for Record Flow

#### 7.1.A — Extract render_picker_frame and add PickerEvent/render_action_picker

Depends on: 7.2 (picker mouse support already in place)

- [x] **7.1.1** In `src/process/picker.rs`, add a public standalone function `render_picker_frame(frame: &mut Frame, area: Rect, actions: &[ProcessAction], list_state: &mut ListState)` that contains the rendering logic currently inside `ActionPicker::draw()` (the padding block, main block, layout split, header, list items, list widget, help footer). This function takes a frame and area and renders into it.
- [x] **7.1.2** Refactor `ActionPicker::draw()` to call `render_picker_frame(frame, area, &self.actions, &mut self.list_state)` inside its `self.terminal.draw(...)` closure, removing the duplicated rendering code.
- [x] **7.1.3** In `src/recording/ui.rs`, add a `PickerEvent` enum: `pub enum PickerEvent { Selected(String), Cancelled }`.
- [x] **7.1.4** In `src/recording/ui.rs`, add a `pub fn render_action_picker(&mut self, actions: &[ProcessAction], list_state: &mut ListState) -> Result<Option<PickerEvent>, Box<dyn Error>>` method to `OsttTui`. It should: (1) render one frame by calling `render_picker_frame` through `self.terminal.draw(...)`, (2) poll for input with 50ms timeout, (3) handle Up/Down/k/j for navigation, Enter for selection (return `Some(PickerEvent::Selected(id))`), Esc/q for cancel (return `Some(PickerEvent::Cancelled)`), Ctrl+C for cancel, mouse scroll up/down for navigation, (4) return `Ok(None)` if no actionable input.
- [x] **7.1.5** Add necessary imports to `src/recording/ui.rs`: `ProcessAction` from config, `ListState` from ratatui, `render_picker_frame` from `crate::process::picker`.
- [x] **7.1.6** Verify: `cargo check` and `cargo clippy -- -D warnings` and `cargo test` all pass.

#### 7.1.B — Refactor handle_record to keep TUI alive through processing

Depends on: 7.1.A

- [x] **7.1.7** In `src/commands/record.rs`, in `handle_record`, remove the second `config::OsttConfig::load()` call in the `Some("")` (picker) branch and the `Some(id)` branch. Instead, reuse the `config_data` already loaded at the top of `handle_record`. Update references from `process_config` to `config_data`.
- [x] **7.1.8** In `src/commands/record.rs`, in `transcribe_recording_with_animation`, replace the manual keywords file reading (lines ~388-402, the `config_dir`, `keywords_file`, `if keywords_file.exists()` block) with `KeywordsManager`: use `dirs::config_dir()` to get the config directory, create a `KeywordsManager::new(&config_dir)?`, and call `keywords_manager.load_keywords()?`.
- [x] **7.1.9** In `src/commands/record.rs`, move `tui.cleanup()` from its current position (after transcription, before processing) to after the entire processing flow is complete — just before the output section (`if let Some(file_path) = output_file`). Remove the existing `tui.cleanup()` call at line ~204. The TUI should stay alive through recording, transcription, picker, and processing animation.
- [x] **7.1.10** Verify: `cargo check` and `cargo clippy -- -D warnings` and `cargo test` all pass.

#### 7.1.C — Use OsttTui for picker and processing animation in handle_record

Depends on: 7.1.B

- [x] **7.1.11** In `src/commands/record.rs`, in the `Some("")` (picker) branch: replace `process::picker::show_action_picker(...)` with an inline loop using `tui.render_action_picker(&config_data.process.actions, &mut list_state)`. Create `list_state` with `ListState::default()` and select index 0. Loop until `PickerEvent::Selected(id)` or `PickerEvent::Cancelled` is returned. Handle the single-action shortcut (if only one action, skip picker and use it directly) before entering the loop.
- [x] **7.1.12** In `src/commands/record.rs`, in the `Some("")` branch after picker selection: replace `process::execute_action_with_animation(...)` with an inline animation loop that reuses `tui.render_transcription_animation(...)`. Create a new `TranscriptionAnimation::new(80)` with `set_status_label("Processing...")`, spawn `process::execute_action(...)` as a tokio task, and run the same render-poll-cancel loop pattern used in `transcribe_recording_with_animation`. Return the result or raw text on cancel.
- [x] **7.1.13** In `src/commands/record.rs`, in the `Some(id)` (direct action) branch: apply the same pattern as 7.1.12 — replace `process::execute_action_with_animation(...)` with an inline animation loop through OsttTui.
- [x] **7.1.14** Ensure all error paths in `handle_record` call `tui.cleanup()` before returning. Check the transcription error paths in `transcribe_recording_with_animation` — where `tui.cleanup().ok()` is called before showing ErrorScreen — these should still work correctly since the TUI is now cleaned up later. Adjust if needed so cleanup happens exactly once on every exit path.
- [x] **7.1.15** Verify: `cargo check` and `cargo clippy -- -D warnings` and `cargo test` all pass.

---

### Spec 7.3 — Mouse Hover Highlight

#### 7.3

Depends on: 7.2 (mouse support), 7.1 (render_picker_frame extraction)

- [x] **7.3.1** In `src/history/ui.rs`, add `const HOVER_BG: Color = Color::Rgb(10, 10, 10);`. Add `hovered_index: Option<usize>` and `list_area: Rect` fields to `HistoryViewer`. Initialize `hovered_index` to `None` and `list_area` to `Rect::default()` in `new()`.
- [x] **7.3.2** In `src/history/ui.rs`, in `draw()`, store the computed `list_area` rect on `self` (e.g., `self.list_area = list_area;` after the layout split). When building `ListItem`s, check if the item index matches `self.hovered_index` and is not the currently selected item — if so, apply `Style::default().bg(HOVER_BG)` to that item.
- [x] **7.3.3** In `src/history/ui.rs`, in `handle_mouse()`, add a `MouseEventKind::Moved` arm. Implement hit-testing: compute `inner_top = self.list_area.y + 1` (border), `inner_bottom = self.list_area.y + self.list_area.height - 1` (border). History items are 2 lines tall (timestamp + text). If `mouse.row` is in range, compute `visible_index = (mouse.row - inner_top) / 2`, `actual_index = visible_index + self.list_state.offset()`. Set `self.hovered_index = Some(actual_index)` if valid, else `None`.
- [x] **7.3.4** In `src/process/picker.rs`, add `const HOVER_BG: Color = Color::Rgb(10, 10, 10);`. Add `hovered_index: Option<usize>` and `list_area: Rect` fields to `ActionPicker`. Initialize them in `new()`.
- [x] **7.3.5** In `src/process/picker.rs`, update `render_picker_frame` to accept a `hovered_index: Option<usize>` parameter. When building `ListItem`s, apply `Style::default().bg(HOVER_BG)` to items whose index matches `hovered_index` (and is not the selected index). Update both callers (`ActionPicker::draw` and `OsttTui::render_action_picker`) to pass the appropriate `hovered_index` value.
- [x] **7.3.6** In `src/process/picker.rs`, in the `run()` method's mouse event handling, add a `MouseEventKind::Moved` arm. Implement hit-testing using `self.list_area`: picker items are 1 line tall. Compute `visible_index = mouse.row - inner_top`, `actual_index = visible_index + self.list_state.offset()`. Set `self.hovered_index` accordingly.
- [x] **7.3.7** In `src/process/picker.rs`, in `draw()`, store the computed `list_area` on `self` (similar to history viewer).
- [x] **7.3.8** Verify: `cargo check` and `cargo clippy -- -D warnings` and `cargo test` all pass.

---

## Verification Protocol

### After each section

Run all three commands in sequence. All must pass:

```bash
cargo check
cargo clippy -- -D warnings
cargo test
```

### After all Phase 7 tasks are complete

Run the full suite one final time to confirm:

```bash
cargo check
cargo clippy -- -D warnings
cargo test
```

---

## Session Boundaries

- **One section per session.** Each section (7.2, 7.1.A, 7.1.B, 7.1.C, 7.3) is a separate session with at most 10 tasks. The agent completes one section, commits, and stops.
- **Do not continue to the next section.** After completing a section, commit and stop.
- **Stop early on repeated failure.** If a verification command fails and the fix attempt also fails (two consecutive failures on the same task), mark the task with `[!]` in PLAN.md, commit all partial work, and stop the session.
- **Commit before stopping.** The agent must `git commit` all changes before ending every session.

---

## Session Prompt Template

```
Read the implementation plan at:
  /home/kristoferlund/gh/ostt/specs/process-command/Phase 7/PLAN.md

Read the session notes at:
  /home/kristoferlund/gh/ostt/specs/process-command/Phase 7/SESSION.md

Read the spec files:
  /home/kristoferlund/gh/ostt/specs/process-command/Phase 7/7.1 — Unified TUI Lifecycle for Record Flow.md
  /home/kristoferlund/gh/ostt/specs/process-command/Phase 7/7.2 — Miscellaneous Fixes.md
  /home/kristoferlund/gh/ostt/specs/process-command/Phase 7/7.3 — Mouse Hover Highlight.md

Find the next incomplete SECTION in PLAN.md (the first section that has
unchecked tasks, e.g., 7.2, 7.1.A, 7.1.B, 7.1.C, or 7.3).
Read the corresponding spec file for context.

Study the relevant source files in the target codebase at /home/kristoferlund/gh/ostt/src/
before making any changes. Understand existing patterns, imports, and conventions.
Key reference files for this phase:
  - src/recording/ui.rs (OsttTui struct — add render_action_picker, PickerEvent)
  - src/commands/record.rs (handle_record, transcribe_recording_with_animation — main refactor target)
  - src/process/picker.rs (ActionPicker — extract render_picker_frame, add mouse support, hover)
  - src/process/execute.rs (execute_action_with_animation — unchanged, reference only)
  - src/process/bash.rs (execute_bash_action — error message fix)
  - src/config/mod.rs (re-exports — add ProvidersConfig)
  - src/history/ui.rs (HistoryViewer — add cleaned_up guard, hover highlight)
  - src/process/mod.rs (module re-exports — may need to export render_picker_frame)

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
  is a group like "7.2", "7.1.A", "7.1.B", "7.1.C", or "7.3" — identified by
  a #### heading in the plan.
- Do not skip tasks. Do not reorder tasks.
- Only modify files in the target codebase (/home/kristoferlund/gh/ostt/src/), PLAN.md,
  and SESSION.md. Do not create or modify any other files.
- If a verification step fails, fix the issue and retry. If it fails a second time on
  the same task, mark the task with `[!]` in PLAN.md, git commit all partial work, and
  STOP the session.
- After completing all tasks in the current section, git commit all changes.
- STOP after completing one section. Do NOT continue to the next section.

After stopping, APPEND a session summary to the end of:
  /home/kristoferlund/gh/ostt/specs/process-command/Phase 7/SESSION.md

Use the heading format: ## Session N: Spec X.Y.Z — <title>
(Increment N based on how many sessions already exist in the file.
If the file does not exist, create it and start with Session 1.)

Include in the summary:
- What was accomplished
- Obstacles encountered
- Out-of-scope observations (things noticed but not acted on)
```
