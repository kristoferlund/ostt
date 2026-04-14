# Phase 5 — Implementation Plan

**Scope:** Spec 5.1 (Process Subcommand), Spec 5.2 (Process Flag on Record, Transcribe, Retry)
**Target codebase:** `/home/kristoferlund/gh/ostt`
**Status:** Complete (all sections done)

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
Phase 4 (complete)
  4.1 Action Picker TUI
       |
       v
Phase 5
  5.1 Process Subcommand ──────────┐
       |                           |
       v                           v
  5.2 Process Flag on Record/Transcribe/Retry
```

**Rationale:**

- **5.1 first:** The `process` subcommand introduces `get_transcription_by_index` on `HistoryManager`, the `commands::process` module, and the `Process` command variant in `app.rs`. These are all new, isolated additions with no existing code modifications beyond registration.
- **5.2 second:** Adding the `-p` flag to `record`, `transcribe`, and `retry` modifies existing command handlers and routing. It depends on 5.1 being complete because the processing flow pattern (load config, validate actions, show picker or look up action, execute, save, output) is established in 5.1's `handle_process` and reused in 5.2.

Execution order: **5.1 → 5.2**

---

## Tasks

### Spec 5.1 — Process Subcommand

#### 5.1.A — History lookup and process command handler

- [x] **5.1.1** Add `get_transcription_by_index` method to `HistoryManager` in `src/history/storage.rs`: takes `index: usize` (1-indexed), computes `offset = index.saturating_sub(1)`, queries `SELECT id, text, created_at FROM transcriptions ORDER BY created_at DESC LIMIT 1 OFFSET ?1`, returns `Result<Option<TranscriptionEntry>>`. Use the same row-parsing pattern as `get_transcription`.
- [x] **5.1.2** Create `src/commands/process.rs` with the `handle_process` async function signature: `pub async fn handle_process(index: Option<usize>, action_id: Option<String>, list: bool, clipboard: bool, output_file: Option<String>) -> Result<(), anyhow::Error>`. Add necessary imports (`crate::config`, `crate::history::HistoryManager`, `crate::keywords::KeywordsManager`, `crate::process`, `crate::clipboard::copy_to_clipboard`, `dirs`).
- [x] **5.1.3** Implement the `--list` mode branch in `handle_process`: load config via `OsttConfig::load()`, if `config.process.actions` is empty print `"No process actions configured. Add actions to ~/.config/ostt/ostt.toml"` and return Ok, otherwise print each action as `"{id} — {name}"` (one per line) and return Ok.
- [x] **5.1.4** Implement the normal mode flow in `handle_process`: load config, validate at least one action exists (error if none: `"No process actions configured. Add actions to ~/.config/ostt/ostt.toml"`), load transcription via `HistoryManager::get_transcription_by_index(index.unwrap_or(1))` (error if None: `"No transcription found at index {N}. Use 'ostt history' to see available transcriptions."`).
- [x] **5.1.5** Continue the normal mode flow: if `action_id` is given, look up via `config.process.get_action(id)` (error if not found: `"Unknown action '{id}'. Use 'ostt process --list' to see available actions."`). If `action_id` is not given, call `process::picker::show_action_picker(&config.process.actions)` — if cancelled, return Ok (exit cleanly). Then look up the selected action from config.
- [x] **5.1.6** Complete the normal mode flow: load keywords via `KeywordsManager`, execute the action via `process::execute_action(&action, &transcription.text, &keywords)`, save the result to history via `history_manager.save_transcription(&result)`, then output using the file > clipboard > stdout priority pattern (matching retry.rs/transcribe.rs).
- [x] **5.1.7** Register the module: add `pub mod process;` and `pub use process::handle_process;` to `src/commands/mod.rs`.
- [x] **5.1.8** Add the `Process` variant to the `Commands` enum in `src/app.rs` with all fields (`index: Option<usize>`, `action: Option<String>`, `list: bool`, `clipboard: bool`, `output: Option<String>`) and the `#[command(visible_alias = "p")]` attribute, matching the spec's CLI definition exactly.
- [x] **5.1.9** Add the routing match arm in `src/app.rs` `run()` function: `Some(Commands::Process { index, action, list, clipboard, output }) => { commands::handle_process(index, action, list, clipboard, output).await?; }`.
- [x] **5.1.10** Verify: `cargo check` and `cargo clippy -- -D warnings` and `cargo test` all pass.

### Spec 5.2 — Process Flag on Record, Transcribe, Retry

#### 5.2.A — CLI changes and routing updates

- [x] **5.2.1** Add the `process` field to the `Record` variant in `Commands` enum in `src/app.rs`: `#[arg(short = 'p', long = "process", value_name = "ACTION", num_args = 0..=1, default_missing_value = "")] process: Option<String>`.
- [x] **5.2.2** Add the same `process` field to the `Retry` variant in `Commands` enum in `src/app.rs`.
- [x] **5.2.3** Add the same `process` field to the `Transcribe` variant in `Commands` enum in `src/app.rs`.
- [x] **5.2.4** Add the same `process` field to the top-level `Cli` struct in `src/app.rs` (for the default record command, no explicit subcommand).
- [x] **5.2.5** Update the `Record` routing match arm in `run()` to extract and pass `process`: destructure `process` alongside `clipboard` and `output`, merge from `cli.process` when `None` (no subcommand), pass to `commands::handle_record(clipboard, output, process)`.
- [x] **5.2.6** Update the `Retry` routing match arm in `run()` to extract and pass `process` to `commands::handle_retry(index, clipboard, output, process)`.
- [x] **5.2.7** Update the `Transcribe` routing match arm in `run()` to extract and pass `process` to `commands::handle_transcribe(file, clipboard, output, process)`.
- [x] **5.2.8** Verify: `cargo check` passes (handler signatures don't match yet — this will fail; proceed to 5.2.B to update handlers).

Note: Task 5.2.8 is expected to fail at `cargo check` because the handler signatures haven't been updated yet. Mark it as complete once you've confirmed the CLI/routing changes compile in isolation (or skip verification and proceed to 5.2.B if `cargo check` fails solely due to handler arity mismatches).

#### 5.2.B — Handler updates for handle_transcribe and handle_retry

- [x] **5.2.9** Update `handle_transcribe` signature in `src/commands/transcribe.rs` to accept `process: Option<String>` as the fourth parameter.
- [x] **5.2.10** Implement the processing flow in `handle_transcribe` after transcription succeeds and `trimmed_text` is available: if `process` is `None`, proceed to output as before. If `process` is `Some("")`, load config, call `process::picker::show_action_picker` — if cancelled, fall through to normal output. If `process` is `Some(id)`, load config, look up action (error if not found). Then load keywords, execute action, save both raw transcription AND processed result to history (two `save_transcription` calls), replace output text with processed result.
- [x] **5.2.11** Update `handle_retry` signature in `src/commands/retry.rs` to accept `process: Option<String>` as the fourth parameter.
- [x] **5.2.12** Implement the same processing flow in `handle_retry` after transcription succeeds: same pattern as `handle_transcribe` — check `process`, show picker or look up action, execute, save both to history, output processed result.
- [x] **5.2.13** Verify: `cargo check` and `cargo clippy -- -D warnings` pass.

#### 5.2.C — Handler update for handle_record and final verification

- [x] **5.2.14** Update `handle_record` signature in `src/commands/record.rs` to accept `process: Option<String>` as the third parameter.
- [x] **5.2.15** Implement the processing flow in `handle_record` after transcription succeeds and TUI is cleaned up: in the output section where `transcription_text` is `Some(text)`, before outputting, check `process`. If `None`, output as before. If `Some("")`, load config, show action picker (new TUI lifecycle — picker sets up its own terminal). If cancelled, fall through to output raw text. If `Some(id)`, load config, look up action. Then load keywords, execute action via `process::execute_action`, save both raw and processed to history, replace output text with processed result. Ensure the recording TUI is always cleaned up before any processing TUI starts.
- [x] **5.2.16** Verify: `cargo check` passes.
- [x] **5.2.17** Verify: `cargo clippy -- -D warnings` passes.
- [x] **5.2.18** Verify: `cargo test` passes.

---

## Verification Protocol

### After each section

Run all three commands in sequence. All must pass:

```bash
cargo check
cargo clippy -- -D warnings
cargo test
```

Exception: Section 5.2.A verification (task 5.2.8) may fail at `cargo check` due to handler signature mismatches — this is expected and the agent should proceed to 5.2.B.

### After all Phase 5 tasks are complete

Run the full suite one final time to confirm:

```bash
cargo check
cargo clippy -- -D warnings
cargo test
```

---

## Session Boundaries

- **One section per session.** Each section (5.1.A, 5.2.A, 5.2.B, 5.2.C) is a separate session with at most 10 tasks. The agent completes one section, commits, and stops.
- **Do not continue to the next section.** After completing a section, commit and stop.
- **Stop early on repeated failure.** If a verification command fails and the fix attempt also fails (two consecutive failures on the same task), mark the task with `[!]` in PLAN.md, commit all partial work, and stop the session.
- **Commit before stopping.** The agent must `git commit` all changes before ending every session.

---

## Session Prompt Template

```
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
```
