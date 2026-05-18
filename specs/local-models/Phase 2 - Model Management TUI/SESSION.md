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
