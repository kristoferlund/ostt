# Phase 3 - Config & Auth Implementation Plan

Scope: specs in `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 3 - Config & Auth/`

Target codebase path: `/Users/kristoferlund/gh/ostt`

Status: Not started

## Dependency Order

Recommended order: `3.1.A -> 3.1.B -> 3.2.A -> 3.2.B -> 3.2.C`

Dependencies:

`3.1 Local Provider Config Types` should run first because it adds local provider config defaults, validation, and re-exports used by the auth/model-selection work.

`3.2 Auth Flow & First-Time Setup` depends on `3.1` for local behavior config and depends on Phase 2 local model management (`src/commands/models_tui.rs`) for the management entry point and local download flow.

Rationale: start with isolated config data types and validation, then update auth credential commands, then build the broader model-selection flow that composes cloud credentials, selected-model persistence, local registry data, and the Phase 2 local management TUI.

Deferred:

- Model size comparison table in the auth picker is deferred by spec.
- Download resume is deferred by spec; interrupted downloads should re-download from scratch.

## Tasks

### 3.1.A - Local Config Types and Defaults

Spec: `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 3 - Config & Auth/3.1 - Local Provider Config Types.md`

- [x] **3.1.1** Add `LocalTranscriptionConfig`, `LocalModelOverride`, and `EffectiveLocalConfig` types to `src/config/file.rs`.
- [x] **3.1.2** Implement `Default` for `LocalTranscriptionConfig` with language `auto`, whisper defaults, `models_path = None`, daemon enabled, and 300 second timeout.
- [x] **3.1.3** Add `#[serde(default)]` behavior for local config and per-model override maps.
- [x] **3.1.4** Extend `ProvidersConfig` with `local: LocalTranscriptionConfig`.
- [x] **3.1.5** Implement `effective_for_model(model_id)` merging global local config with per-model overrides.
- [x] **3.1.6** Add config parsing tests for missing `[providers.local]`, full `[providers.local]`, optional `models_path`, and default daemon/language values.
- [x] **3.1.7** Add tests for per-model override deserialization and effective config merging.
- [x] **3.1.8** Verify: run `cargo check`.
- [x] **3.1.9** Verify: run focused `cargo test config::file`.

### 3.1.B - Local Config Validation and Integration

Depends on: `3.1.A`

Spec: `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 3 - Config & Auth/3.1 - Local Provider Config Types.md`

- [x] **3.1.10** Implement local config validation for temperature, entropy threshold, no-speech threshold, models path existence, and daemon idle timeout.
- [x] **3.1.11** Implement per-model override validation using the same value rules where override values are present.
- [x] **3.1.12** Validate per-model override keys as safe local model IDs without validating the active selected model ID.
- [x] **3.1.13** Call local provider validation from the existing config load validation path.
- [x] **3.1.14** Re-export `LocalTranscriptionConfig`, `LocalModelOverride`, and `EffectiveLocalConfig` from `src/config/mod.rs`.
- [x] **3.1.15** Add tests for invalid local values, valid values, missing models path, safe override keys, and active model ID non-validation.
- [x] **3.1.16** Add an integration helper or access pattern so local transcription callers can obtain local config only when the selected provider is local.
- [x] **3.1.17** Verify: run `cargo check`.
- [x] **3.1.18** Verify: run `cargo clippy -- -D warnings`.
- [x] **3.1.19** Verify: run focused `cargo test config::file`.

### 3.2.A - Auth Login and Logout Credential Commands

Depends on: `3.1.B`

Spec: `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 3 - Config & Auth/3.2 - Auth Flow & First-Time Setup.md`

- [x] **3.2.1** Refactor `src/commands/auth.rs` so `ostt auth login` manages credentials only and does not select a model.
- [x] **3.2.2** Ensure auth login lists only cloud providers and excludes Local.
- [x] **3.2.3** Preserve existing provider config and unrelated credentials when saving a selected provider API key.
- [x] **3.2.4** Add Cliclack completion guidance telling the user to run `ostt model` after login.
- [x] **3.2.5** Implement auth logout provider selection from currently authorized cloud providers only.
- [x] **3.2.6** Add logout confirmation before deleting a provider credential.
- [x] **3.2.7** Clear selected-model state when logout removes the provider used by the active model.
- [x] **3.2.8** Add focused tests for provider filtering, login credential preservation, logout clearing active selection, and Local exclusion where practical.
- [x] **3.2.9** Verify: run `cargo check`.
- [x] **3.2.10** Verify: run focused `cargo test commands::auth` or the nearest focused auth/config test target.

### 3.2.B - Model Selection Data and Recovery Errors

Depends on: `3.2.A`

Spec: `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 3 - Config & Auth/3.2 - Auth Flow & First-Time Setup.md`

- [x] **3.2.11** Create `src/commands/model.rs` for the canonical `ostt model` Ratatui selection flow.
- [x] **3.2.12** Add data types for grouped model-selection entries covering authenticated cloud providers, local registry/custom entries, and the local management row.
- [x] **3.2.13** Build cloud model sections from providers that currently have credentials.
- [x] **3.2.14** Build the Local section from Phase 2 registry/custom state and filesystem downloaded status.
- [x] **3.2.15** Mark the active provider/model using provider-aware selected-model state.
- [x] **3.2.16** Add first-run and recovery error helpers for no selected model, missing cloud credentials, and missing local model files.
- [x] **3.2.17** Add tests for grouped model data, authenticated-provider filtering, Local entries, active marking, and actionable recovery messages.
- [x] **3.2.18** Verify: run `cargo check`.
- [x] **3.2.19** Verify: run focused `cargo test commands::model` or the nearest focused model-selection test target.

### 3.2.C - Ratatui Model Wizard and Local Management Integration

Depends on: `3.2.B`

Spec: `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 3 - Config & Auth/3.2 - Auth Flow & First-Time Setup.md`

- [ ] **3.2.20** Implement Ratatui rendering for grouped cloud and Local model selection following existing OSTT TUI style.
- [ ] **3.2.21** Implement navigation, quit, and back behavior: arrow keys, Enter, `m`, `Esc`, and `q`.
- [ ] **3.2.22** Save selected downloaded local models through provider-aware selected-model state as `provider = local` and string model ID.
- [ ] **3.2.23** Save selected cloud models through provider-aware selected-model state with the selected cloud provider/model.
- [ ] **3.2.24** Route `Manage local models...` and `[m]` into `src/commands/models_tui.rs` without adding `ostt local` or `ostt models`.
- [ ] **3.2.25** For missing local models, show download confirmation/progress and require explicit activation after download.
- [ ] **3.2.26** Add Ctrl+C cancellation behavior for local downloads in the model-selection flow.
- [ ] **3.2.27** Add the local audio compatibility warning or offer when current audio output is incompatible with local transcription.
- [ ] **3.2.28** Add focused tests for selection save behavior, local management routing, missing-local download flow state, and quit/back behavior where practical.
- [ ] **3.2.29** Verify: run `cargo check` and `cargo clippy -- -D warnings`.
- [ ] **3.2.30** Verify: run `cargo test`.

## Verification Protocol

After each section or sub-section is complete, run the verification tasks listed at the end of that section.

After each full spec is complete, run:

- `cargo check`
- `cargo clippy -- -D warnings`
- `cargo test`

After all Phase 3 specs are complete, run:

- `cargo check`
- `cargo clippy -- -D warnings`
- `cargo test`

If a command cannot be run, record the reason in `SESSION.md` and do not claim it passed.

## Session Boundaries

The unit of work per session is one section or sub-section, each containing at most 10 tasks. If a spec is split into sub-sections such as `3.1.A` and `3.1.B`, each sub-section is a separate session. The agent completes one section or sub-section, commits, and stops.

Stop early if verification for the current task fails twice after attempted fixes. Mark the failed task with `[!]`, commit the partial work and notes, and stop.

The agent must git commit before stopping every time, including failed or partial sessions. Do not continue to the next section or sub-section in the same session.

## Session Prompt Template

```text
Read `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 3 - Config & Auth/PLAN.md`, `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 3 - Config & Auth/SESSION.md` if it exists, and the spec files in `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 3 - Config & Auth/`.

Target codebase: `/Users/kristoferlund/gh/ostt`.

Find the next incomplete section or sub-section in `PLAN.md`: the first section or sub-section containing unchecked `- [ ]` tasks. That section or sub-section is the entire scope for this session. Do not work on any later section or sub-section.

Read the spec file named by that section and study the relevant source files in the target codebase. Also read notes from previous sessions in `SESSION.md` before changing code.

Implement tasks strictly in order. Do not skip tasks. Do not reorder tasks. Scope is one section or sub-section only.

Critical crash-recovery rule: update `PLAN.md` immediately after completing each task, changing that task from `- [ ]` to `- [x]`, before starting the next task. Do not batch these updates.

Run the verification command listed for each verification task when you reach it. If verification fails, fix and rerun once. If it fails a second time, mark the task with `[!]`, append notes to `SESSION.md`, commit partial work, and stop.

Restrict file modifications to the target codebase, `PLAN.md`, and `SESSION.md` only. Do not modify files outside `/Users/kristoferlund/gh/ostt`, except for `PLAN.md` and `SESSION.md` in the spec folder.

Before stopping, append a session summary to the end of `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 3 - Config & Auth/SESSION.md`. Do not overwrite existing notes. Use heading `## Session N: Spec X.Y - <title>` with `N` incremented from prior sessions. Include what was accomplished, obstacles encountered, and out-of-scope observations.

Git commit all changes before stopping. Stop after one section or sub-section, even if all tasks passed. Do not continue to the next section or sub-section.
```
