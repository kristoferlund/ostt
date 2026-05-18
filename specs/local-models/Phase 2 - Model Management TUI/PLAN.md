# Phase 2 — Model Management TUI Implementation Plan

Scope: specs in `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 2 - Model Management TUI/`

Target codebase path: `/Users/kristoferlund/gh/ostt`

Status: Not started

## Dependency Order

Recommended order: `2.1.A -> 2.1.B -> 2.1.C -> 2.2.A -> 2.2.B -> 2.2.C -> 2.3.A -> 2.3.B -> 2.3.C`

Dependencies:

`2.1 Model Registry & Storage` has no Phase 2 dependency and establishes shared registry/state/types, filesystem discovery, activation, and deletion primitives.

`2.2 Model Download Engine` depends on `2.1` because downloads use `RegistryEntry`, `LocalModelState`, deterministic filenames, model directories, and custom model registration.

`2.3 Model Management TUI` depends on `2.1` and `2.2` because the UI builds its list from registry/state/filesystem data and calls activation, deletion, download, cancellation, and custom URL resolution functions.

Deferred: hardware recommendation integration from spec `4.1` is outside this Phase 2 plan. Preserve metadata fields such as `recommended_hardware`, but do not implement hardware recommendation logic here.

## Tasks

### 2.1.A — Registry Types, Paths, and State Persistence

Spec: `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 2 - Model Management TUI/2.1 - Model Registry & Storage.md`

- [x] **2.1.1** Update `src/transcription/local_models.rs` `RegistryEntry` to match the spec shape with serde serialize/deserialize support.
- [x] **2.1.2** Add `LocalModelState.version` and implement `Default` with version `1` and empty `custom_models`.
- [x] **2.1.3** Add helpers for `state_path()` and `model_files_dir()` using the existing `models_dir()` convention.
- [x] **2.1.4** Replace custom-only loading with `load_state()` that returns default when `models.json` is missing or corrupted.
- [x] **2.1.5** Add `save_state(&LocalModelState)` that creates parent directories and writes pretty JSON.
- [x] **2.1.6** Preserve a compatibility helper for loading custom entries through the new local state shape if existing callers need it.
- [x] **2.1.7** Add focused tests for default load, corrupted load, save/load round trip, and parent directory creation.
- [x] **2.1.8** Verify: run `cargo check`.
- [x] **2.1.9** Verify: run focused `cargo test` for local model state persistence tests.

### 2.1.B — Filename Derivation and Installed Discovery

Depends on: `2.1.A`

Spec: `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 2 - Model Management TUI/2.1 - Model Registry & Storage.md`

- [x] **2.1.10** Implement URL extension extraction so `model_filename(id, url)` derives `{id}.{ext}` and falls back to `{id}.bin`.
- [x] **2.1.11** Add safe model ID validation matching `^[a-z0-9._-]+$` for reuse by registry/custom flows.
- [x] **2.1.12** Add `InstalledModelView` with entry, path, size, modified time, and active flag.
- [x] **2.1.13** Implement `installed_models(registry, state, selected_model)` by merging registry and custom entries and checking `models/files/`.
- [x] **2.1.14** Update `resolve_installed_model_path()` to use `model_files_dir()` and derived filenames consistently.
- [x] **2.1.15** Ensure installed status is inferred from files and does not write registry models into `models.json`.
- [x] **2.1.16** Add focused tests for filename derivation, safe ID validation, installed discovery, and active marking.
- [x] **2.1.17** Verify: run `cargo check`.
- [x] **2.1.18** Verify: run focused `cargo test` for filename/discovery tests.

### 2.1.C — Activation and Deletion

Depends on: `2.1.B`

Spec: `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 2 - Model Management TUI/2.1 - Model Registry & Storage.md`

- [x] **2.1.19** Add or adapt selected-model integration so local selections preserve provider ID `local` and arbitrary model IDs.
- [x] **2.1.20** Implement model lookup across registry and custom state with clear missing/not-downloaded errors.
- [x] **2.1.21** Implement `activate_model(model_id)` that verifies the derived file exists before saving provider/model selection.
- [x] **2.1.22** Implement `deactivate_model()` through the selected-model clearing mechanism.
- [x] **2.1.23** Implement `delete_model(model_id)` that removes the derived installed file.
- [x] **2.1.24** Make `delete_model(model_id)` clear active selection when the deleted model was active.
- [x] **2.1.25** Ensure deleting a custom model keeps custom metadata unless a separate explicit metadata removal flow is added later.
- [x] **2.1.26** Add focused tests for activate, deactivate, delete, active clearing, and not-installed errors.
- [x] **2.1.27** Verify: run `cargo check`.
- [x] **2.1.28** Verify: run focused `cargo test` for activation/deletion tests.

### 2.2.A — Registry Fetch and Download Streaming

Depends on: `2.1.C`

Spec: `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 2 - Model Management TUI/2.2 - Model Download Engine.md`

- [ ] **2.2.1** Add any required streaming dependency or `reqwest` feature in `Cargo.toml` using the smallest compatible change.
- [ ] **2.2.2** Add `REMOTE_REGISTRY_URL` and `fetch_registry()` in `src/transcription/local_models.rs`.
- [ ] **2.2.3** Implement remote registry fetch and parse into `Vec<RegistryEntry>` with clear network/registry errors.
- [ ] **2.2.4** Add `DownloadProgressCallback` and download state primitives needed by the engine.
- [ ] **2.2.5** Implement `download_model(url, dest_path, progress)` streaming to `dest_path` via a `.tmp` file.
- [ ] **2.2.6** Report downloaded bytes, total bytes, and MB/s through the progress callback.
- [ ] **2.2.7** Rename `.tmp` to final path only after a successful complete download.
- [ ] **2.2.8** Add focused tests using mocked/local HTTP or isolated filesystem paths where practical.
- [ ] **2.2.9** Verify: run `cargo check`.
- [ ] **2.2.10** Verify: run focused `cargo test` for registry/download tests.

### 2.2.B — Download Registration and Validation

Depends on: `2.2.A`

Spec: `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 2 - Model Management TUI/2.2 - Model Download Engine.md`

- [ ] **2.2.11** Add `model_destination(entry)` using `model_files_dir()` and `model_filename()`.
- [ ] **2.2.12** Implement `mark_downloaded_registry_model(entry)` that validates the downloaded file exists without writing registry metadata.
- [ ] **2.2.13** Implement `register_custom_model(entry)` with replace-by-ID semantics and no duplicate custom state entries.
- [ ] **2.2.14** Add post-download validation using SHA256 when available and size checks otherwise where metadata exists.
- [ ] **2.2.15** Ensure re-downloading replaces the model file atomically through the temp-file path.
- [ ] **2.2.16** Ensure download helpers do not activate models; leave activation to calling flows.
- [ ] **2.2.17** Add focused tests for registry registration no-op, custom registration, duplicate replacement, and missing-file validation.
- [ ] **2.2.18** Verify: run `cargo check`.
- [ ] **2.2.19** Verify: run focused `cargo test` for registration/validation tests.

### 2.2.C — Custom URL Resolution and Cancellation

Depends on: `2.2.B`

Spec: `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 2 - Model Management TUI/2.2 - Model Download Engine.md`

- [ ] **2.2.20** Implement `resolve_custom_model(input)` that rejects non-URL inputs with a clear URL-required error.
- [ ] **2.2.21** Detect Hugging Face model page URLs separately from direct model file URLs.
- [ ] **2.2.22** Resolve direct model file URLs into registry-shaped custom entries with safe IDs, names, size metadata when available, URL, and `category = Some("custom")`.
- [ ] **2.2.23** Resolve Hugging Face model page URLs through the Hugging Face API and select a whisper.cpp-compatible file conservatively.
- [ ] **2.2.24** Validate custom IDs with the shared safe-ID helper and detect derived filename collisions before registration/download.
- [ ] **2.2.25** Add `DownloadHandle` or equivalent cancellation flag support and check it during chunk streaming.
- [ ] **2.2.26** Clean up `.tmp` files on cancellation or failed partial downloads.
- [ ] **2.2.27** Add focused tests for URL classification, invalid inputs, direct URL resolution, ID validation, collision detection, and cancellation cleanup.
- [ ] **2.2.28** Verify: run `cargo check`.
- [ ] **2.2.29** Verify: run focused `cargo test` for custom URL/cancellation tests.

### 2.3.A — TUI Module, Entry Point, and Model List

Depends on: `2.1.C`, `2.2.C`

Spec: `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 2 - Model Management TUI/2.3 - Model Management TUI.md`

- [ ] **2.3.1** Add `src/commands/models_tui.rs` and declare/export it from `src/commands/mod.rs` following existing command module conventions.
- [ ] **2.3.2** Add `TuiModelEntry` matching the spec data model.
- [ ] **2.3.3** Implement `build_model_list(local_state, registry, selected_model)` by merging registry and custom entries.
- [ ] **2.3.4** Compute downloaded and active status from `models/files/` and selected-model state.
- [ ] **2.3.5** Add disk usage summary calculation from downloaded model file metadata.
- [ ] **2.3.6** Add `ModelTui` and `TuiMode` state skeleton with browse, custom input, downloading, info, and confirmation states as needed.
- [ ] **2.3.7** Wire opening the TUI from the local model management entry in `ostt model` without adding a separate `ostt models` command.
- [ ] **2.3.8** Add focused tests for `build_model_list()` and disk usage calculation.
- [ ] **2.3.9** Verify: run `cargo check`.
- [ ] **2.3.10** Verify: run focused `cargo test` for TUI model-list tests.

### 2.3.B — Browse, Info, Activation, and Deletion UI

Depends on: `2.3.A`

Spec: `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 2 - Model Management TUI/2.3 - Model Management TUI.md`

- [ ] **2.3.11** Implement terminal setup/teardown so quitting or errors restore the terminal cleanly.
- [ ] **2.3.12** Render the browse screen with active model, downloaded section, available section, disk usage, and key hints.
- [ ] **2.3.13** Implement arrow-key navigation with bounds-safe selection behavior.
- [ ] **2.3.14** Implement Enter activation for downloaded models and a clear message for non-downloaded models.
- [ ] **2.3.15** Implement `i` info view with size, download date, path, URL, languages, and recommendation metadata.
- [ ] **2.3.16** Implement `Esc` behavior from info and other sub-views back to browse.
- [ ] **2.3.17** Implement `r` deletion confirmation and `y/N` handling.
- [ ] **2.3.18** Call `delete_model()` on confirmed deletion and refresh model list, including active-model clearing.
- [ ] **2.3.19** Verify: run `cargo check`.
- [ ] **2.3.20** Verify: run focused `cargo test` for browse/info/delete logic tests where practical.

### 2.3.C — Download Progress, Custom Flow, and Final Integration

Depends on: `2.3.B`

Spec: `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 2 - Model Management TUI/2.3 - Model Management TUI.md`

- [ ] **2.3.21** Implement `d` download action for selected available models using the download engine.
- [ ] **2.3.22** Render download progress with a ratatui `Gauge`, bytes, speed, ETA, and completion status.
- [ ] **2.3.23** Implement `Tab` cancellation for in-progress downloads and return to browse with a clear status message.
- [ ] **2.3.24** Refresh the model list after successful download so the model moves to the downloaded section.
- [ ] **2.3.25** Implement `c` custom URL input using the existing `tui-input` dependency.
- [ ] **2.3.26** Resolve custom URLs through `resolve_custom_model()`, show confirmation metadata, then download/register confirmed custom models.
- [ ] **2.3.27** Show clear errors for registry/network failures while still allowing custom model entry where possible.
- [ ] **2.3.28** Verify: run `cargo check`.
- [ ] **2.3.29** Verify: run `cargo clippy -- -D warnings`.
- [ ] **2.3.30** Verify: run `cargo test`.

## Verification Protocol

After each section or sub-section is complete, run the verification tasks listed at the end of that section. Prefer focused tests first when they exist, then `cargo check`.

After each full spec is complete, run:

- `cargo check`
- `cargo clippy -- -D warnings`
- `cargo test`

After all Phase 2 specs are complete, run:

- `cargo check`
- `cargo clippy -- -D warnings`
- `cargo test`

If a command cannot be run, record the reason in `SESSION.md` and do not claim it passed.

## Session Boundaries

The unit of work per session is one section or sub-section, each containing at most 10 tasks. If a spec is split into sub-sections such as `2.1.A`, `2.1.B`, and `2.1.C`, each sub-section is a separate session. The agent completes one section or sub-section, commits, and stops.

Stop early if verification for the current task fails twice after attempted fixes. Mark the failed task with `[!]`, commit the partial work and notes, and stop.

The agent must git commit before stopping every time, including failed or partial sessions. Do not continue to the next section or sub-section in the same session.

## Session Prompt Template

Read `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 2 - Model Management TUI/PLAN.md`, `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 2 - Model Management TUI/SESSION.md` if it exists, and the spec files in `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 2 - Model Management TUI/`.

Target codebase: `/Users/kristoferlund/gh/ostt`.

Find the next incomplete section or sub-section in `PLAN.md`: the first section or sub-section containing unchecked `- [ ]` tasks. That section or sub-section is the entire scope for this session. Do not work on any later section or sub-section.

Read the spec file named by that section and study only the relevant source files in the target codebase needed for that section. Also read notes from previous sessions in `SESSION.md` before changing code.

Implement tasks strictly in order. Do not skip tasks. Do not reorder tasks. Scope is one section or sub-section only.

Critical crash-recovery rule: update `PLAN.md` immediately after completing each task, changing that task from `- [ ]` to `- [x]`, before starting the next task. Do not batch these updates.

Run the verification command listed for each verification task when you reach it. If verification fails, fix and rerun once. If it fails a second time, mark the task with `[!]`, append notes to `SESSION.md`, commit partial work, and stop.

Restrict file modifications to the target codebase, `PLAN.md`, and `SESSION.md` only. Do not modify files outside `/Users/kristoferlund/gh/ostt`, except for `PLAN.md` and `SESSION.md` in the spec folder.

Before stopping, append a session summary to the end of `/Users/kristoferlund/gh/ostt/specs/local-models/Phase 2 - Model Management TUI/SESSION.md`. Do not overwrite existing notes. Use heading `## Session N: Spec X.Y — <title>` with `N` incremented from prior sessions. Include what was accomplished, obstacles encountered, and out-of-scope observations.

Git commit all changes before stopping. Stop after one section or sub-section, even if all tasks passed. Do not continue to the next section or sub-section.
