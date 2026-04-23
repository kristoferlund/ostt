# Phase 2 — Session Notes

## Session 1: Spec 2.1.A — Config type changes in `src/config/file.rs`

### What was accomplished

All 9 tasks in sub-section 2.1.A completed successfully:

- Added `AiTool` enum with four variants (`OpenCode`, `ClaudeCode`, `GeminiCli`, `CodexCli`) using `#[serde(rename_all = "kebab-case")]` in `src/config/file.rs`
- Added `default_binary()` method returning `"opencode"`, `"claude"`, `"gemini"`, `"codex"` respectively
- Extended `ActionDetails::Ai` with three new fields: `tool: AiTool`, `tool_binary: Option<String>` (serde default), `tool_args: Option<Vec<String>>` (serde default)
- Verified `ProcessAction::validate()` still works unchanged (uses `..` pattern to ignore new fields)
- Exported `AiTool` from `src/config/mod.rs`
- Updated 6 existing tests to include the required `tool` field: `valid_ai_action`, `valid_mixed_actions`, `invalid_ai_missing_model`, `invalid_ai_missing_inputs`, `invalid_ai_missing_model_and_inputs`, `empty_inputs_deserializes_but_fails_validate`
- All verification passed: `cargo check`, `cargo clippy -- -D warnings`, `cargo test` (36/36 tests pass)

### Obstacles encountered

None. All tasks completed on first attempt.

### Out-of-scope observations

- The `valid_ai_action` test's match pattern was updated from `ActionDetails::Ai { model, inputs }` to `ActionDetails::Ai { tool, model, inputs, .. }` to destructure the new `tool` field and use `..` for remaining optional fields. This pattern should be used in future tests as well.
- The `invalid_ai_missing_model_and_inputs` test now includes `tool` — it tests that `model` and `inputs` are still required independently of `tool`.

## Session 2: Spec 2.1.B — Config type tests

### What was accomplished

All 9 tasks in sub-section 2.1.B completed successfully:

- Added `AiToolWrapper` helper struct and `parse_ai_tool()` helper function for deserializing bare `AiTool` values in tests
- Added test `ai_tool_deserializes_all_kebab_case_variants`: verifies all four kebab-case variants (`open-code`, `claude-code`, `gemini-cli`, `codex-cli`) deserialize to the correct enum variant
- Added test `ai_tool_unknown_variant_fails_deserialization`: verifies `tool = "vim"` fails deserialization
- Added test `ai_action_missing_tool_fails_deserialization`: verifies `ActionDetails::Ai` without `tool` field fails
- Added test `ai_action_optional_fields_default_to_none`: verifies `tool_binary` and `tool_args` default to `None` when omitted
- Added test `ai_action_tool_binary_and_tool_args_deserialize_correctly`: verifies `tool_binary = "/custom/path"` and `tool_args = ["--flag", "value"]` deserialize correctly
- Added test `ai_tool_default_binary_returns_expected_names`: verifies `default_binary()` returns `"opencode"`, `"claude"`, `"gemini"`, `"codex"` for each variant
- All verification passed: `cargo check`, `cargo clippy -- -D warnings`, `cargo test` (42/42 tests pass — 36 existing + 6 new)

### Obstacles encountered

None. All tasks completed on first attempt.

### Out-of-scope observations

- None.

## Session 3: Spec 2.1.C — AI executor module (`src/process/ai.rs`)

### What was accomplished

All 10 tasks in sub-section 2.1.C completed successfully:

- Created `src/process/ai.rs` with the `execute_ai_action` async function signature taking `&AiTool`, `&str` model, `&[ResolvedMessage]`, `Option<&str>` tool_binary, `Option<&[String]>` tool_args, returning `anyhow::Result<String>`
- Implemented `build_prompts()` helper that splits `ResolvedMessage` list into system prompt and user prompt, concatenating same-role messages with `"\n\n"` separators
- Added `build_required_args()` method to `AiTool` in `src/config/file.rs` returning tool-specific CLI args: OpenCode `["run", "--model", model]`, Claude Code `["-p", "--system-prompt", system, "--model", model]`, Gemini CLI `["-p", system, "-m", model]`, Codex CLI `["exec", system, "--model", model]`
- Implemented `build_stdin_content()` helper: OpenCode gets `[System]\n{system}\n\n[User]\n{user}` format; all other tools get user prompt only
- Implemented `tokio::process::Command` invocation with piped stdin/stdout/stderr, writes prompt to stdin, closes stdin, calls `wait_with_output()`
- Added 120-second timeout using `tokio::time::timeout`
- Implemented error handling: `ErrorKind::NotFound` on spawn, non-zero exit with stderr, timeout, empty stdout
- Registered module: added `pub mod ai;` to `src/process/mod.rs`
- All verification passed: `cargo check`, `cargo clippy -- -D warnings`, `cargo test` (42/42 tests pass — no new tests in this sub-section)

### Obstacles encountered

None. All tasks completed on first attempt.

### Out-of-scope observations

- The `build_prompts()` and `build_stdin_content()` functions are marked `pub(crate)` rather than private, to allow the upcoming 2.1.D test sub-section to test them directly without spawning real processes.
- The timeout implementation uses `tokio::time::timeout` wrapping `child.wait_with_output()`. When a timeout occurs, the child process is dropped (which sends SIGKILL on Unix), but the error is returned before we can confirm the kill. This is acceptable for the current use case.

## Session 4: Spec 2.1.D — AI executor tests

### What was accomplished

All 12 tasks in sub-section 2.1.D completed successfully:

- Added test `prompt_construction_separates_system_and_user`: verifies `build_prompts()` correctly splits system and user messages
- Added test `multiple_same_role_messages_concatenated_with_blank_line`: verifies multiple messages of the same role are joined with `"\n\n"`
- Added test `build_required_args_returns_correct_args_for_each_tool`: verifies all four tool variants return the expected CLI args
- Added test `opencode_stdin_includes_system_and_user_labels`: verifies OpenCode stdin format includes `[System]` and `[User]` role labels
- Added test `non_opencode_tools_stdin_contains_only_user_prompt`: verifies ClaudeCode, GeminiCli, and CodexCli stdin contains only user prompt (no system content)
- Added test `tool_binary_override_changes_binary_used`: verifies custom binary path is used in error messages (spawns nonexistent binary)
- Added test `tool_args_appended_after_required_args`: verifies extra args are appended after `build_required_args()` output
- Added test `missing_cli_tool_returns_error_with_tool_name`: verifies "not found" error includes the tool binary name
- Added test `non_zero_exit_returns_error_with_stderr`: verifies non-zero exit returns error containing stderr content (uses a temp shell script that writes to stderr and exits 1)
- All verification passed: `cargo check`, `cargo clippy -- -D warnings`, `cargo test` (51/51 tests pass — 42 existing + 9 new)

### Obstacles encountered

- The `non_zero_exit_returns_error_with_stderr` test initially failed because using `sh` as `tool_binary` with extra `-c` args didn't work — the required args from `build_required_args()` are prepended before `tool_args`, so `sh` tried to execute `run` (the first OpenCode required arg) as a script. Fixed by creating a temp executable shell script that writes to stderr and exits 1, then using that script path as `tool_binary`.

### Out-of-scope observations

- Session 3 (2.1.C) did not commit its changes. This session's commit includes both 2.1.C and 2.1.D changes.
- Phase 2 (Spec 2.1) is now fully complete — all 40 tasks across sub-sections A, B, C, and D are done.
