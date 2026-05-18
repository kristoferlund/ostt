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
