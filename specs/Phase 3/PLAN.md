# Phase 3 — Implementation Plan

**Scope:** Specs 3.1 (Bash Action Executor) and 3.2 (Action Dispatcher)
**Target codebase:** `/home/kristoferlund/gh/ostt`
**Status:** Not started

---

## Dependency Order

```
Phase 1 (complete)           Phase 2 (complete)
  1.1 Action Config Types      2.1 AI CLI Tool Executor
  1.2 Input Resolution              |
       |                             |
       v                             v
Phase 3                              |
  3.1 Bash Action Executor           |
       |                             |
       v                             v
  3.2 Action Dispatcher  <───────────┘
```

**Rationale:** Spec 3.1 (Bash Action Executor) is a standalone new module with no intra-phase dependencies — it only uses `tokio::process::Command` and standard error handling. It is small and isolated, so it goes first.

Spec 3.2 (Action Dispatcher) depends on 3.1 because `execute_action` dispatches to `bash::execute_bash_action` for bash actions. It also depends on Phase 2's `ai::execute_ai_action` and Phase 1's `input::resolve_inputs`, both of which are already complete.

Execution order: **3.1 first, then 3.2.**

---

## Tasks

### Spec 3.1 — Bash Action Executor

#### 3.1.A — Module implementation and tests

- [ ] **3.1.1** Create `src/process/bash.rs` with the `execute_bash_action` async function signature (taking `&str` command and `&str` input, returning `anyhow::Result<String>`)
- [ ] **3.1.2** Implement the function body: spawn `sh -c <command>` using `tokio::process::Command` with stdin piped and stdout/stderr captured, write `input` to stdin, close stdin, wait for output
- [ ] **3.1.3** Add 30-second timeout using `tokio::time::timeout` — kill the child and return error "Command timed out after 30 seconds" on timeout
- [ ] **3.1.4** Implement error handling: command spawn failure ("Command failed to start: {error}. Make sure the command is installed."), non-zero exit ("Command exited with status {code}:\n{stderr}"), return trimmed stdout on success
- [ ] **3.1.5** Register the module: add `pub mod bash;` to `src/process/mod.rs`
- [ ] **3.1.6** Add test: `execute_bash_action("tr '[:lower:]' '[:upper:]'", "hello")` returns `"HELLO"`
- [ ] **3.1.7** Add test: `execute_bash_action("cat", "pass through")` returns `"pass through"`
- [ ] **3.1.8** Add test: a command that exits non-zero returns an error containing stderr (e.g., `sh -c "echo err >&2; exit 1"`)
- [ ] **3.1.9** Add test: a non-existent command (e.g., `nonexistent_command_xyz`) returns a clear error
- [ ] **3.1.10** Verify: `cargo check`, `cargo clippy -- -D warnings`, `cargo test` all pass

---

### Spec 3.2 — Action Dispatcher

**Depends on: 3.1**

#### 3.2.A — Module implementation and tests

- [ ] **3.2.1** Create `src/process/execute.rs` with the `execute_action` async function signature (taking `&ProcessAction`, `&str` transcription, `&[String]` keywords, returning `anyhow::Result<String>`)
- [ ] **3.2.2** Implement the dispatch logic: match on `action.details` — for `ActionDetails::Bash { command }` call `bash::execute_bash_action(command, transcription)`, for `ActionDetails::Ai { tool, model, inputs, tool_binary, tool_args }` call `input::resolve_inputs` then `ai::execute_ai_action`
- [ ] **3.2.3** Register the module and add re-exports: add `pub mod execute;` to `src/process/mod.rs`, add `pub use execute::execute_action;`
- [ ] **3.2.4** Add test: `execute_action` with a bash action (`cat`) returns the transcription text as-is
- [ ] **3.2.5** Add test: `execute_action` with a bash action that transforms text (e.g., `tr '[:lower:]' '[:upper:]'`) returns the transformed result
- [ ] **3.2.6** Add test: `execute_action` with a bash action that fails returns an error
- [ ] **3.2.7** Verify: `cargo check`, `cargo clippy -- -D warnings`, `cargo test` all pass

---

## Verification Protocol

### After each section (3.1.A, 3.2.A)

Run all three commands in sequence. All must pass before moving to the next section:

```bash
cargo check
cargo clippy -- -D warnings
cargo test
```

### After all Phase 3 tasks are complete

Run the full verification suite one final time:

```bash
cargo check
cargo clippy -- -D warnings
cargo test
```

---

## Session Boundaries

- **One section per session.** Each session works on exactly ONE section (e.g., 3.1.A or 3.2.A), then commits and STOPS. Sections have at most 10 tasks to keep each session focused and within context limits.
- **Do not continue to the next section.** After completing all tasks in the current section (including its verification tasks), commit and stop. The next section is a separate session.
- **Stop early on repeated failure.** If a verification command fails and the fix attempt also fails (two consecutive failures on the same task), mark the task with `[!]`, commit all partial work, and stop the session.
- **Commit before stopping.** The agent must `git commit` all changes before ending every session.

---

## Session Prompt Template

```
Read the implementation plan at:
  /home/kristoferlund/gh/ostt/specs/Phase 3/PLAN.md

Read the session notes at:
  /home/kristoferlund/gh/ostt/specs/Phase 3/SESSION.md

Read every spec file in:
  /home/kristoferlund/gh/ostt/specs/Phase 3/

Find the next incomplete SECTION in PLAN.md (the first section that has
unchecked tasks, e.g., 3.1.A or 3.2.A).
Read the corresponding spec file for context.

Study the relevant source files in the target codebase at /home/kristoferlund/gh/ostt/src/
before making any changes. Understand existing patterns, imports, and conventions.

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
  is a group like "3.1.A", "3.2.A", etc. — identified by a #### heading in the plan.
- Do not skip tasks. Do not reorder tasks.
- Only modify files in the target codebase (/home/kristoferlund/gh/ostt/src/), PLAN.md,
  and SESSION.md. Do not create or modify any other files.
- If a verification step fails, fix the issue and retry. If it fails a second time on
  the same task, mark the task with `[!]` in PLAN.md, git commit all partial work, and
  STOP the session.
- After completing all tasks in the current section, git commit all changes.
- STOP after completing one section. Do NOT continue to the next section.

After stopping, APPEND a session summary to the end of:
  /home/kristoferlund/gh/ostt/specs/Phase 3/SESSION.md

Use the heading format: ## Session N: Spec 3.X.Y — <title>
(Increment N based on how many sessions already exist in the file.)

Include in the summary:
- What was accomplished
- Obstacles encountered
- Out-of-scope observations (things noticed but not acted on)
```
