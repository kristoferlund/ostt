# Phase 2 — Implementation Plan

**Scope:** Spec 2.1 (AI CLI Tool Executor)
**Target codebase:** `/home/kristoferlund/gh/ostt`
**Status:** Not started

---

## Dependency Order

```
Phase 1 (complete)
  1.1 Action Config Types
  1.2 Input Resolution
        |
        v
Phase 2
  2.1 AI CLI Tool Executor
```

**Rationale:** Spec 2.1 depends on Phase 1 being complete because:
- The `ActionDetails::Ai` variant (defined in 1.1) is extended with new fields (`tool`, `tool_binary`, `tool_args`).
- The `ResolvedMessage` type and `resolve_inputs()` function (from 1.2) are consumed by the new `execute_ai_action()` function.
- Existing Phase 1 tests reference the current `ActionDetails::Ai` shape and must be updated.

Phase 1 is already complete, so 2.1 can proceed immediately.

Phase 2 contains only one spec (2.1), so there are no intra-phase dependencies.

---

## Tasks

### Spec 2.1 — AI CLI Tool Executor

#### 2.1.A — Config type changes in `src/config/file.rs`

- [x] **2.1.1** Add `AiTool` enum with variants `OpenCode`, `ClaudeCode`, `GeminiCli`, `CodexCli`, deriving `Debug, Clone, Serialize, Deserialize` with `#[serde(rename_all = "kebab-case")]`
- [x] **2.1.2** Add `default_binary()` method to `AiTool` returning the standard binary name for each variant (`"opencode"`, `"claude"`, `"gemini"`, `"codex"`)
- [x] **2.1.3** Extend `ActionDetails::Ai` to include: `tool: AiTool`, `#[serde(default)] tool_binary: Option<String>`, `#[serde(default)] tool_args: Option<Vec<String>>`
- [x] **2.1.4** Update `ProcessAction::validate()` if needed (the existing validation rejects AI actions with empty `inputs` — verify this still works with the new fields)
- [x] **2.1.5** Export `AiTool` from `src/config/mod.rs`
- [x] **2.1.6** Update all existing tests in `src/config/file.rs` that construct or parse `ActionDetails::Ai` to include the required `tool` field (tests: `valid_ai_action`, `valid_mixed_actions`, `invalid_ai_missing_model`, `invalid_ai_missing_inputs`, `invalid_ai_missing_model_and_inputs`, `empty_inputs_deserializes_but_fails_validate`)
- [x] **2.1.7** Verify: `cargo check` passes
- [x] **2.1.8** Verify: `cargo clippy -- -D warnings` passes
- [x] **2.1.9** Verify: `cargo test` passes (all 36 existing tests still pass)

#### 2.1.B — Config type tests

- [ ] **2.1.10** Add test: `AiTool` deserializes all four kebab-case variants (`open-code`, `claude-code`, `gemini-cli`, `codex-cli`)
- [ ] **2.1.11** Add test: unknown tool value (e.g., `tool = "vim"`) fails deserialization
- [ ] **2.1.12** Add test: `ActionDetails::Ai` without `tool` field fails deserialization
- [ ] **2.1.13** Add test: `tool_binary` and `tool_args` are optional — omitting them deserializes to `None`
- [ ] **2.1.14** Add test: `tool_binary = "/custom/path"` and `tool_args = ["--flag", "value"]` deserialize correctly
- [ ] **2.1.15** Add test: `AiTool::default_binary()` returns the expected binary name for each variant
- [ ] **2.1.16** Verify: `cargo check` passes
- [ ] **2.1.17** Verify: `cargo clippy -- -D warnings` passes
- [ ] **2.1.18** Verify: `cargo test` passes

#### 2.1.C — AI executor module (`src/process/ai.rs`)

- [ ] **2.1.19** Create `src/process/ai.rs` with the `execute_ai_action` async function signature (taking `&AiTool`, `&str` model, `&[ResolvedMessage]`, `Option<&str>` tool_binary, `Option<&[String]>` tool_args, returning `anyhow::Result<String>`)
- [ ] **2.1.20** Implement prompt construction: separate `ResolvedMessage` list into system prompt (system messages concatenated with `"\n\n"`) and user prompt (user messages concatenated with `"\n\n"`)
- [ ] **2.1.21** Add `build_required_args()` method to `AiTool` that returns the tool-specific CLI args given model and system prompt: OpenCode `["run", "--model", model]`; Claude Code `["-p", "--system-prompt", system, "--model", model]`; Gemini CLI `["-p", system, "-m", model]`; Codex CLI `["exec", system, "--model", model]`
- [ ] **2.1.22** Implement the stdin content logic: OpenCode gets concatenated `[System]\n{system}\n\n[User]\n{user}` format; all other tools get user prompt only
- [ ] **2.1.23** Implement the `tokio::process::Command` invocation: spawn with piped stdin/stdout/stderr, write prompt to stdin, close stdin, `wait_with_output()`
- [ ] **2.1.24** Add 120-second timeout using `tokio::time::timeout`, killing the child process on timeout
- [ ] **2.1.25** Implement error handling: tool not found (check `ErrorKind::NotFound` on spawn), non-zero exit (include stderr), timeout, empty stdout
- [ ] **2.1.26** Register the module: add `pub mod ai;` to `src/process/mod.rs`
- [ ] **2.1.27** Verify: `cargo check` passes
- [ ] **2.1.28** Verify: `cargo clippy -- -D warnings` passes

#### 2.1.D — AI executor tests

- [ ] **2.1.29** Add test: prompt construction correctly separates system and user messages
- [ ] **2.1.30** Add test: multiple messages of the same role are concatenated with blank line separators
- [ ] **2.1.31** Add test: `build_required_args()` returns correct args for each tool variant
- [ ] **2.1.32** Add test: OpenCode stdin content includes `[System]` and `[User]` role labels with both prompts
- [ ] **2.1.33** Add test: non-OpenCode tools stdin content contains only user prompt (no system content)
- [ ] **2.1.34** Add test: `tool_binary` override changes the binary name used (test via the binary selection logic, not by spawning)
- [ ] **2.1.35** Add test: `tool_args` are appended after required args
- [ ] **2.1.36** Add test: missing CLI tool returns error with tool name in message
- [ ] **2.1.37** Add test: non-zero exit returns error with stderr content (invoke a known binary like `false` or `sh -c "exit 1"` to simulate)
- [ ] **2.1.38** Verify: `cargo check` passes
- [ ] **2.1.39** Verify: `cargo clippy -- -D warnings` passes
- [ ] **2.1.40** Verify: `cargo test` passes

---

## Verification Protocol

### After each sub-section (2.1.A, 2.1.B, 2.1.C, 2.1.D)

Run all three commands in sequence. All must pass before moving to the next sub-section:

```bash
cargo check
cargo clippy -- -D warnings
cargo test
```

### After all Phase 2 tasks are complete

Run the full verification suite one final time:

```bash
cargo check
cargo clippy -- -D warnings
cargo test
```

---

## Session Boundaries

- **One sub-section per session.** Each session works on exactly ONE sub-section (e.g., 2.1.A, 2.1.B, 2.1.C, or 2.1.D), then commits and STOPS. Sub-sections have at most 10 tasks to keep each session focused and within context limits.
- **Do not continue to the next sub-section.** After completing all tasks in the current sub-section (including its verification tasks), commit and stop. The next sub-section is a separate session.
- **Stop early on repeated failure.** If a verification command fails and the fix attempt also fails (two consecutive failures on the same task), mark the task with `[!]`, commit all partial work, and stop the session.
- **Commit before stopping.** The agent must `git commit` all changes before ending every session.

---

## Session Prompt Template

```
Read the implementation plan at:
  /home/kristoferlund/gh/ostt/specs/Phase 2/PLAN.md

Read the session notes at:
  /home/kristoferlund/gh/ostt/specs/Phase 2/SESSION.md

Read every spec file in:
  /home/kristoferlund/gh/ostt/specs/Phase 2/

Find the next incomplete SUB-SECTION in PLAN.md (the first sub-section that has
unchecked tasks, e.g., 2.1.A, 2.1.B, 2.1.C, or 2.1.D).
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
  Do NOT batch these updates. Do NOT wait until the end of the sub-section.

After each verification task (cargo check, cargo clippy, cargo test), confirm it passes
before moving on.

Rules:
- SCOPE: Complete only ONE sub-section per session (at most 10 tasks). A sub-section
  is a group like "2.1.A", "2.1.B", etc. — identified by a #### heading in the plan.
- Do not skip tasks. Do not reorder tasks.
- Only modify files in the target codebase (/home/kristoferlund/gh/ostt/src/), PLAN.md,
  and SESSION.md. Do not create or modify any other files.
- If a verification step fails, fix the issue and retry. If it fails a second time on
  the same task, mark the task with `[!]` in PLAN.md, git commit all partial work, and
  STOP the session.
- After completing all tasks in the current sub-section, git commit all changes.
- STOP after completing one sub-section. Do NOT continue to the next sub-section.

After stopping, APPEND a session summary to the end of:
  /home/kristoferlund/gh/ostt/specs/Phase 2/SESSION.md

Use the heading format: ## Session N: Spec 2.1.X — <title>
(Increment N based on how many sessions already exist in the file.)

Include in the summary:
- What was accomplished
- Obstacles encountered
- Out-of-scope observations (things noticed but not acted on)
```
