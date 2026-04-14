# Phase 6 — Implementation Plan

**Scope:** Spec 6.1 (Processing Animation with Status Label)
**Target codebase:** `/home/kristoferlund/gh/ostt`
**Status:** Not started

---

## Dependency Order

```
Phase 3 (complete)
  3.2 Action Dispatcher (execute_action)
       |
Phase 5 (complete)
  5.1 Process Subcommand (handle_process)
  5.2 Process Flag (handle_record, handle_retry, handle_transcribe)
       |
       v
Phase 6
  6.1 Processing Animation with Status Label
```

**Rationale:**

- **6.1** depends on Phase 3's `execute_action` dispatcher and Phase 5's command handlers. All four callers (`handle_process`, `handle_record`, `handle_retry`, `handle_transcribe`) already call `process::execute_action` directly — Phase 6 wraps those calls in an animation helper.
- This is a single spec with one core change (add status label to animation) and one significant addition (`execute_action_with_animation` helper), plus four caller updates. It fits naturally into two sub-sections.

Execution order: **6.1.A → 6.1.B**

---

## Tasks

### Spec 6.1 — Processing Animation with Status Label

#### 6.1.A — Status label on TranscriptionAnimation and execute_action_with_animation helper

- [ ] **6.1.1** Add `status_label: String` field to the `TranscriptionAnimation` struct in `src/transcription/animation.rs`. Initialize it to `"Transcribing...".to_string()` in `new()`.
- [ ] **6.1.2** Add `pub fn set_status_label(&mut self, label: &str)` method to `TranscriptionAnimation` that sets `self.status_label = label.to_string()`.
- [ ] **6.1.3** In the `draw()` method of `TranscriptionAnimation`, after the existing character rendering loop (`for anim_char in &self.chars` block), add label rendering: compute `label_x` centered horizontally, `label_y = center_y + 2`, render using `frame.buffer_mut().set_string(...)` with `Style::default().fg(Color::Rgb(128, 128, 128))`. Only render if `!self.status_label.is_empty() && label_y < height`.
- [ ] **6.1.4** In `src/commands/record.rs`, in the `transcribe_recording_with_animation` function, call `animation.set_status_label("Transcribing...");` after creating the animation (after `let mut animation = TranscriptionAnimation::new(80);`) for explicitness.
- [ ] **6.1.5** Add the `execute_action_with_animation` async function in `src/process/execute.rs`. Signature: `pub async fn execute_action_with_animation(action: &ProcessAction, transcription: &str, keywords: &[String]) -> anyhow::Result<Option<String>>`. It should: (1) set up terminal (raw mode, alternate screen, CrosstermBackend, Terminal::new), (2) create a `TranscriptionAnimation` and call `set_status_label("Processing...")`, (3) spawn `execute_action` as a tokio task, (4) run animation loop (render frame, poll for cancel input via Esc/q/Ctrl+C, sleep 50ms, check if task finished), (5) return `Ok(Some(result))` on success, `Ok(None)` on cancel, `Err` on failure, (6) restore terminal on exit. Use a `Drop`-based cleanup guard struct (same pattern as `ActionPicker` in `picker.rs`) to ensure terminal is restored even on panic/early return.
- [ ] **6.1.6** Re-export `execute_action_with_animation` from `src/process/mod.rs`.
- [ ] **6.1.7** Verify: `cargo check` and `cargo clippy -- -D warnings` and `cargo test` all pass.

#### 6.1.B — Update all callers to use execute_action_with_animation

- [ ] **6.1.8** Update `src/commands/process.rs` (`handle_process`): replace the direct `process::execute_action(&action, &transcription.text, &keywords).await?` call with `process::execute_action_with_animation(&action, &transcription.text, &keywords).await?`. Handle the `Option<String>` return: if `None` (cancelled), return `Ok(())` early; if `Some(result)`, use `result` for the rest of the output flow.
- [ ] **6.1.9** Update `src/commands/record.rs` (`handle_record`): in both processing branches (`Some("")` picker path and `Some(id)` direct path), replace `process::execute_action(...)` with `process::execute_action_with_animation(...)`. Handle `None` (cancelled) by falling through to output raw transcription text.
- [ ] **6.1.10** Update `src/commands/retry.rs` (`handle_retry`): in both processing branches (`Some("")` picker path and `Some(id)` direct path), replace `process::execute_action(...)` with `process::execute_action_with_animation(...)`. Handle `None` (cancelled) by falling through to output raw transcription `trimmed_text`.
- [ ] **6.1.11** Update `src/commands/transcribe.rs` (`handle_transcribe`): in both processing branches (`Some("")` picker path and `Some(id)` direct path), replace `process::execute_action(...)` with `process::execute_action_with_animation(...)`. Handle `None` (cancelled) by falling through to output raw `trimmed_text`.
- [ ] **6.1.12** Verify: `cargo check` and `cargo clippy -- -D warnings` and `cargo test` all pass.

---

## Verification Protocol

### After each section

Run all three commands in sequence. All must pass:

```bash
cargo check
cargo clippy -- -D warnings
cargo test
```

### After all Phase 6 tasks are complete

Run the full suite one final time to confirm:

```bash
cargo check
cargo clippy -- -D warnings
cargo test
```

---

## Session Boundaries

- **One section per session.** Each section (6.1.A, 6.1.B) is a separate session with at most 10 tasks. The agent completes one section, commits, and stops.
- **Do not continue to the next section.** After completing a section, commit and stop.
- **Stop early on repeated failure.** If a verification command fails and the fix attempt also fails (two consecutive failures on the same task), mark the task with `[!]` in PLAN.md, commit all partial work, and stop the session.
- **Commit before stopping.** The agent must `git commit` all changes before ending every session.

---

## Session Prompt Template

```
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
```
