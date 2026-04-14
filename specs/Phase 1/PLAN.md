# Phase 1 — Implementation Plan

**Scope:** Specs 1.1 (Action Config Types) and 1.2 (Input Resolution)
**Target codebase:** `/home/kristoferlund/gh/ostt`
**Status:** Not started

---

## Dependency Order

```
1.1 Action Config Types
        |
        v
1.2 Input Resolution
```

**Rationale:** Spec 1.2 depends on 1.1 because `resolve_inputs` takes `&[ActionInput]` and returns `ResolvedMessage` values containing `InputRole` — both types defined in 1.1. The `InputContent` and `InputSource` enums that drive the resolution match arms are also from 1.1.

Execution order: **1.1 first, then 1.2.**

---

## Tasks

### Spec 1.1 — Action Config Types

#### 1.1.A — New types in `src/config/file.rs`

- [x] **1.1.1** Add `InputRole` enum (`System`, `User`) with `Debug, Clone, Serialize, Deserialize` and `#[serde(rename_all = "lowercase")]` to `src/config/file.rs`
- [x] **1.1.2** Add `InputSource` enum (`Transcription`, `Keywords`) with `Debug, Clone, Serialize, Deserialize` and `#[serde(rename_all = "lowercase")]` to `src/config/file.rs`
- [x] **1.1.3** Add `InputContent` enum (`Source`, `File`, `Literal`) with `#[serde(untagged)]` and correct variant order (Source > File > Literal) to `src/config/file.rs`
- [x] **1.1.4** Add `ActionInput` struct with `role: InputRole` and `#[serde(flatten)] input_content: InputContent` to `src/config/file.rs`
- [x] **1.1.5** Add `ActionDetails` enum (`Bash { command }`, `Ai { model, inputs }`) with `#[serde(tag = "type", rename_all = "lowercase")]` to `src/config/file.rs`
- [x] **1.1.6** Add `ProcessAction` struct (`id`, `name`, `#[serde(flatten)] details: ActionDetails`) to `src/config/file.rs`
- [x] **1.1.7** Add `ProcessConfig` struct with `#[serde(default)] actions: Vec<ProcessAction>` and derive `Default` to `src/config/file.rs`
- [x] **1.1.8** Verify: `cargo check` passes

#### 1.1.B — Integration, validation, and re-exports

- [x] **1.1.9** Add `#[serde(default)] pub process: ProcessConfig` field to `OsttConfig` and update `OsttConfig::default()` to include `process: ProcessConfig::default()`
- [x] **1.1.10** Add `get_action(&self, id: &str) -> Option<&ProcessAction>` method to `ProcessConfig`
- [x] **1.1.11** Add `validate()` method to `ProcessAction` that returns an error if an AI action has empty `inputs`; call validation during config load in `OsttConfig::load()`
- [x] **1.1.12** Add re-exports to `src/config/mod.rs`: `ProcessConfig`, `ProcessAction`, `ActionDetails`, `ActionInput`, `InputContent`, `InputRole`, `InputSource`
- [x] **1.1.13** Verify: `cargo check` passes

#### 1.1.C — Tests

- [x] **1.1.14** Add `#[cfg(test)] mod tests` in `src/config/file.rs` with tests for valid configurations: bash action, AI action, mixed actions, missing `[process]` section, all input variants (`content`, `source = "transcription"`, `source = "keywords"`, `file`, roles `system`/`user`)
- [x] **1.1.15** Add tests for invalid `ProcessAction` configurations: missing `type`, unknown `type`, missing `id`, missing `name`, bash missing `command`, AI missing `model`, AI missing `inputs`, AI missing both `model` and `inputs`
- [x] **1.1.16** Add tests for invalid `ActionInput` configurations: missing `role`, unknown `role`, no content field, unknown `source`
- [x] **1.1.17** Add tests for edge cases: empty `inputs = []` passes deserialization but fails `validate()`; input with multiple content fields uses highest-precedence field
- [x] **1.1.18** Add tests for `get_action`: returns matching action by id, returns `None` for nonexistent id
- [x] **1.1.19** Verify: `cargo check` passes
- [x] **1.1.20** Verify: `cargo clippy -- -D warnings` passes
- [x] **1.1.21** Verify: `cargo test` passes

---

### Spec 1.2 — Input Resolution

**Depends on: 1.1**

#### 1.2.A — Module and function

- [x] **1.2.1** Create `src/process/mod.rs` with `pub mod input;`
- [x] **1.2.2** Create `src/process/input.rs` with `ResolvedMessage` struct and `resolve_inputs` function signature (returning `anyhow::Result<Vec<ResolvedMessage>>`)
- [x] **1.2.3** Implement resolution logic: `Literal` uses content as-is, `Source::Transcription` uses the transcription arg, `Source::Keywords` joins with newlines (skip if empty), `File` reads file with `~` expansion
- [x] **1.2.4** Add `pub mod process;` to `src/lib.rs`
- [x] **1.2.5** Verify: `cargo check` passes

#### 1.2.B — Tests

- [x] **1.2.6** Add `#[cfg(test)] mod tests` in `src/process/input.rs` with tests: literal content resolves correctly, transcription source resolves, keywords source resolves to newline-joined string, empty keywords list is skipped
- [x] **1.2.7** Add tests for file resolution: valid file reads correctly, `~` path expansion works, missing file returns error
- [x] **1.2.8** Verify: `cargo check` passes
- [x] **1.2.9** Verify: `cargo clippy -- -D warnings` passes
- [x] **1.2.10** Verify: `cargo test` passes

---

## Verification Protocol

### After each spec group

Run all three commands in sequence. All must pass before the spec is considered complete:

```bash
cargo check
cargo clippy -- -D warnings
cargo test
```

### After all Phase 1 specs are complete

Run the full verification suite one final time:

```bash
cargo check
cargo clippy -- -D warnings
cargo test
```

---

## Session Boundaries

- **One spec group per session.** Each session implements exactly one spec group (1.1 or 1.2), then stops. Do not continue to the next spec.
- **Stop early on repeated failure.** If a verification command fails and the fix attempt also fails (two consecutive failures on the same task), mark the task with `[!]`, commit all partial work, and stop the session.
- **Commit before stopping.** The agent must `git commit` all changes before ending every session.

---

## Session Prompt Template

```
Read the implementation plan at:
  /home/kristoferlund/gh/ostt/specs/Phase 1/PLAN.md

Read the session notes at:
  /home/kristoferlund/gh/ostt/specs/Phase 1/SESSION.md

Read every spec file in:
  /home/kristoferlund/gh/ostt/specs/Phase 1/

Find the next incomplete spec group in PLAN.md (the first group that has unchecked tasks).
Read the corresponding spec file for that group.

Study the relevant source files in the target codebase at /home/kristoferlund/gh/ostt/src/
before making any changes. Understand existing patterns, imports, and conventions.

Read the session notes from previous sessions in SESSION.md (if the file exists) to
understand what has already been accomplished and any obstacles encountered.

Implement tasks in the exact order listed in PLAN.md. Do not skip or reorder tasks.

After each verification task (cargo check, cargo clippy, cargo test), confirm it passes
before moving on.

Mark each task complete in PLAN.md by changing `- [ ]` to `- [x]` as you finish it.

Rules:
- Do not skip tasks. Do not reorder tasks.
- Only modify files in the target codebase (/home/kristoferlund/gh/ostt/src/), PLAN.md,
  and SESSION.md. Do not create or modify any other files.
- If a verification step fails, fix the issue and retry. If it fails a second time on
  the same task, mark the task with `[!]` in PLAN.md, git commit all partial work, and
  STOP the session.
- After completing all tasks in the current spec group, git commit all changes.
- STOP after completing one spec group. Do not continue to the next spec group.

After stopping, APPEND a session summary to the end of:
  /home/kristoferlund/gh/ostt/specs/Phase 1/SESSION.md

Use the heading format: ## Session N: Spec X.Y — <title>
(Increment N based on how many sessions already exist in the file.)

Include in the summary:
- What was accomplished
- Obstacles encountered
- Out-of-scope observations (things noticed but not acted on)
```
