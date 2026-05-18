## Session 1: Spec 2.1.A — Registry Types, Paths, and State Persistence

Accomplished:
- Updated `RegistryEntry` to the full registry/custom model shape with serde serialize/deserialize support.
- Added `LocalModelState.version`, default state, state/model file path helpers, resilient `load_state()`, and pretty JSON `save_state()`.
- Preserved `load_custom_model_entries()` as a compatibility helper backed by the new local state shape.
- Added focused persistence tests for missing state, corrupted state, save/load round trip, and parent directory creation.
- Verified with `cargo check` and `cargo test transcription::local_models`.

Obstacles encountered:
- `SESSION.md` did not exist at session start, so this file was created for the first session summary.

Out-of-scope observations:
- `model_filename()` still uses the older filename-from-URL behavior; this is covered by the next scoped section, `2.1.B`.

## Session 2: Spec 2.1.B — Filename Derivation and Installed Discovery

Accomplished:
- Updated `model_filename()` to derive local filenames from model ID plus URL extension, with `.bin` fallback.
- Added shared safe model ID validation for lowercase ASCII IDs with dots, dashes, and underscores.
- Added `InstalledModelView` and `installed_models()` to merge registry/custom entries and infer installed/active status from files.
- Kept registry model installation inference filesystem-only so registry entries are not written into `models.json`.
- Added focused tests for filename derivation, ID validation, installed discovery, active marking, and registry no-persist behavior.
- Verified with `cargo check` and `cargo test transcription::local_models`.

Obstacles encountered:
- The first focused test run failed because test isolation used `XDG_DATA_HOME`, which `dirs::data_dir()` does not consistently use on macOS. Added an explicit `OSTT_MODELS_DIR` override for isolated tests and reran successfully.

Out-of-scope observations:
- Active local model selection still only compares raw model IDs in this section. Provider-aware selected-model persistence remains scoped to `2.1.C`.

## Session 3: Spec 2.1.C — Activation and Deletion

Accomplished:
- Added provider-aware selected-model persistence with `SelectedModel`, JSON storage for new selections, legacy plain model ID reading, and selected-model clearing.
- Implemented local model lookup across custom state and available registry entries, with missing and not-downloaded errors separated.
- Added `activate_model()`, `deactivate_model()`, and `delete_model()` with derived file existence checks, local provider selection, active selection clearing, and custom metadata preservation.
- Updated installed-model active detection to require provider ID `local` and matching model ID.
- Added focused activation/deletion tests covering activation, deactivation, delete, active clearing, custom metadata retention, missing models, and not-installed errors.
- Verified with `cargo check` and `cargo test transcription::local_models`.

Obstacles encountered:
- No blocking obstacles. Tests needed isolated `HOME` handling because selected-model state is stored under the user data directory.

Out-of-scope observations:
- Official registry activation remains limited by the current placeholder registry loader; remote registry fetching is scoped to `2.2.A`.

## Session 4: Spec 2.2.A — Registry Fetch and Download Streaming

Accomplished:
- Added minimal streaming support through `reqwest`'s `stream` feature and `futures-util`.
- Added the remote registry URL, `fetch_registry()`, remote registry fetching, and clear network/HTTP/parse errors.
- Added `DownloadProgressCallback` and `download_model()` streaming to a `.tmp` file, reporting bytes/total/MBps, syncing, and renaming only after completion.
- Added focused local HTTP tests for registry parsing, HTTP errors, download progress, final file contents, and temp-file cleanup on success.
- Verified with `cargo check` and `cargo test transcription::local_models`.

Obstacles encountered:
- The first focused test run failed because the local HTTP test helper moved borrowed `&str` values into a spawned thread. Converted them to owned strings and reran successfully.

Out-of-scope observations:
- Failed partial download cleanup and cancellation remain scoped to `2.2.C`.
- Download registration, custom model registration, and validation remain scoped to `2.2.B`.

## Session 5: Spec 2.2.B — Download Registration and Validation

Accomplished:
- Added deterministic `model_destination()`, registry download marking without state writes, and custom model registration with replace-by-ID semantics.
- Added post-download validation using SHA-256 when available and size checks when checksum metadata is absent.
- Kept download helpers install-only: re-download replaces the file via the temp-file path and does not change the selected local model.
- Added focused tests for registry no-op registration, missing-file validation, custom duplicate replacement, checksum/size validation, replacement download behavior, and non-activation.
- Verified with `cargo check` and `cargo test transcription::local_models`.

Obstacles encountered:
- SHA-256 validation required adding the minimal `sha2` dependency and updating `Cargo.lock`.
- The replacement/non-activation async test initially did not keep isolated environment state through the full test; tightened it and reran focused tests successfully.

Out-of-scope observations:
- Custom URL resolution, collision detection, cancellation, and failed partial download cleanup remain scoped to `2.2.C`.

## Session 6: Spec 2.2.C — Custom URL Resolution and Cancellation

Accomplished:
- Added custom model URL resolution for direct model file URLs and Hugging Face model page URLs.
- Added conservative whisper.cpp-compatible file classification/selection, safe custom ID derivation, custom filename collision checks, `DownloadHandle` cancellation, and `.tmp` cleanup on cancellation/failure.
- Added focused tests for URL classification, invalid inputs, direct URL resolution, Hugging Face metadata resolution, unsafe ID/collision rejection, and cancellation cleanup.
- Verified `cargo check` passed.

Obstacles encountered:
- `cargo test transcription::local_models` failed twice. The first failure was due the test HTTP helper writing a body for `HEAD` requests and poisoning the test env mutex. After fixing the helper, the second run still failed because direct URL size resolution returned `0` instead of `1`, which again poisoned the shared test env mutex and caused many dependent test failures.
- Per the crash-recovery rule, `2.2.29` was marked `[!]` and work stopped after committing partial changes.

Out-of-scope observations:
- The focused test failure appears isolated to test HTTP `HEAD`/`Content-Length` behavior and test env lock poisoning, not to `cargo check` or cancellation cleanup.

## Session 7: Spec 2.3.A — TUI Module, Entry Point, and Model List

Accomplished:
- Added `src/commands/models_tui.rs` and exported it from `src/commands/mod.rs`.
- Added the TUI model entry data model, TUI mode/state skeleton, model-list builder, and disk usage calculation.
- Wired the singular `ostt model` command to the local model TUI handler without adding `ostt models`.
- Added focused tests for registry/custom merging, downloaded/active status, and disk usage.
- Verified with `cargo check` and `cargo test commands::models_tui`.

Obstacles encountered:
- No existing `ostt model` command or local model management row was present in the current codebase, so this session added the singular command as the minimal entry point for later TUI rendering work.

Out-of-scope observations:
- The TUI handler currently builds state and reports availability; full terminal setup/rendering and interactive browse behavior remain scoped to `2.3.B` and later.

## Session 8: Spec 2.3.B — Browse, Info, Activation, and Deletion UI

Accomplished:
- Implemented ratatui/crossterm terminal setup with cleanup on quit or error.
- Added browse rendering with active model, downloaded/available sections, disk usage, and key hints.
- Added bounds-safe arrow navigation, Enter activation, info view, Escape back behavior, delete confirmation, confirmed deletion, list refresh, and status messages.
- Added focused tests for navigation bounds, info/back behavior, activation requirements, and deletion clearing active selection.
- Verified with `cargo check` and `cargo test commands::models_tui`.

Obstacles encountered:
- Existing `activate_model()`/`delete_model()` can only resolve persisted custom entries because `load_registry_entries()` is still unavailable. The TUI now uses entry-aware activation/deletion for registry entries, while still calling `delete_model()` first for custom entries.

Out-of-scope observations:
- Download action, progress rendering, cancellation, custom URL input, and final integration remain scoped to `2.3.C`.

## Session 9: Spec 2.3.C — Download Progress, Custom Flow, and Final Integration

Accomplished:
- Added `[d]` download handling for selected registry models using the download engine.
- Added download progress rendering with a ratatui `Gauge`, bytes, speed, ETA, and status text.
- Added `Tab` cancellation signaling for in-progress downloads.
- Refreshes the model list after successful downloads so completed models move into the downloaded section.
- Added `[c]` custom URL input using `tui-input`, URL resolution through `resolve_custom_model()`, custom confirmation metadata, and custom download/registration.
- Shows a registry/network fallback message when the remote registry cannot be loaded while keeping custom URL entry available.
- Verified `cargo check` passed after adding a `Sync` bound to the download progress callback.
- Verified `cargo clippy -- -D warnings` passed after applying minimal clippy fixes in custom URL helpers.

Obstacles encountered:
- `cargo test` failed twice. The first run exposed parallel test environment races between model TUI tests and local model tests, plus direct URL size resolution returning `0` instead of `1`.
- A shared test env lock and direct `Content-Length` header parsing were added, but the second full `cargo test` still failed because `direct_model_file_url_resolves_to_custom_entry` continued to report size `0`, poisoning the shared test env lock and cascading into dependent local model tests.
- Per the crash-recovery rule, `2.3.30` was marked `[!]` and work stopped after committing partial changes.

Out-of-scope observations:
- The remaining failure appears isolated to test HTTP `HEAD`/`Content-Length` behavior in direct custom URL resolution and resulting lock poisoning, not to `cargo check`, clippy, or the TUI compile path.
