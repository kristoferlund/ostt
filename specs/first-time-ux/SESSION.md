## Session 1: Spec 1.1 — Auth And Model Guidance Messages

Accomplished:
- Updated the no-selected-model error to exactly: `No transcription model selected. Run 'ostt auth' to add an API key, then 'ostt model' to choose a model.`
- Updated missing cloud API key guidance to mention both `ostt auth` and `ostt model`.
- Added unit coverage for the exact no-model guidance text and stable missing-key guidance substrings.
- Marked all Spec 1.1 tasks complete in `PLAN.md` as they were completed.

Decisions made:
- Kept changes message-only per Spec 1.1; no setup, preflight, or TUI behavior was added.
- Introduced a private `missing_api_key_guidance` formatter so the missing-key message can be tested without touching user credential files or mutating test environment variables.

Files changed:
- `src/transcription/context.rs`
- `src/transcription/mod.rs`
- `specs/first-time-ux/PLAN.md`
- `specs/first-time-ux/SESSION.md`

Helpers/APIs introduced or reused:
- Reused existing `resolve_selected_model` and `config_for_selected_model` paths.
- Introduced private `missing_api_key_guidance(provider_name: &str) -> String` in `src/transcription/mod.rs` for the cloud missing-key message.

Verification results:
- `cargo check` passed.
- `cargo test transcription::context` passed.
- Additional targeted test `cargo test transcription::tests::missing_api_key_guidance_mentions_auth_and_model_steps` passed because the listed context filter does not run the new `transcription::mod` unit test.

Constraints for later sessions:
- Later preflight/TUI work should reuse these exact/auth-model guidance messages rather than adding alternate no-model or missing-key wording.
- Spec 1.1 intentionally did not alter when these errors are surfaced.

Obstacles encountered:
- None.

Open questions:
- None.

Out-of-scope observations:
- `loop.sh` was already modified in the worktree and was left untouched/uncommitted.

## Session 2: Spec 1.2 — Record TUI Lifecycle Starts Immediately

Accomplished:
- Created the record TUI before `AudioRecorder::start_recording()` so audio startup failures have an active TUI surface.
- Reused the same `RecordingTui` instance for startup errors, waveform rendering, transcription animation, processing animation, and action picker.
- Added post-startup sample-rate configuration on the existing TUI instead of a second terminal takeover.
- Routed all `AudioRecorder::start_recording()` failures through the record TUI error screen.
- Added startup error formatting with primary error, cause details, and targeted next steps for no-input-device, configured-device-not-found, device config, stream create/start, and likely macOS microphone permission failures.
- Added hardware-free unit tests for startup error formatting/remediation.
- Marked all Spec 1.2 tasks complete in `PLAN.md` as they were completed.

Decisions made:
- Reused `RecordingTui` rather than adding a separate popup error runner, matching the spec preference.
- Kept startup error formatting narrow to audio startup; broader full-chain record error formatting remains for Spec 1.4.
- Did not modify `src/recording/audio.rs`; routing all `start_recording()` errors from `handle_record` was sufficient for this scope.

Files changed:
- `src/commands/record.rs`
- `src/recording/tui.rs`
- `specs/first-time-ux/PLAN.md`
- `specs/first-time-ux/SESSION.md`

Helpers/APIs introduced or reused:
- Reused existing `RecordingTui::show_error` for startup failures.
- Introduced `RecordingTui::new_pending_audio(config)` for early terminal takeover with a temporary 48 kHz sample rate.
- Introduced `RecordingTui::set_sample_rate(sample_rate)` to apply the actual device sample rate after startup succeeds.
- Introduced private `show_audio_startup_error`, `format_audio_startup_error`, and `audio_startup_next_step` helpers in `src/commands/record.rs`.

Verification results:
- `cargo check` passed.
- `cargo test recording::tui` passed; it currently runs 0 tests for that filter.
- `cargo test commands::record` passed with 3 tests.
- After `cargo fmt`, reran `cargo check && cargo test recording::tui && cargo test commands::record`; all passed.

Constraints for later sessions:
- Spec 1.3 preflight should run after `RecordingTui::new_pending_audio(config)` and before `AudioRecorder::start_recording()`.
- Spec 1.4 should either reuse or replace the narrow startup formatter deliberately when adding full record error-chain formatting; avoid creating another parallel TUI error formatter without consolidating.
- Keep using the same `RecordingTui` instance through record, transcription, processing, and action-picker states.

Obstacles encountered:
- Initial `cargo test recording::tui` emitted an unused-import warning from the new tests; removed the import and reran successfully.

Open questions:
- None.

Out-of-scope observations:
- `loop.sh` remains modified from prior work and was left untouched/uncommitted.

## Session 3: Spec 1.3 — Record Preflight Before Audio Recording

Accomplished:
- Added record preflight immediately after `RecordingTui::new_pending_audio(config)` and before `AudioRecorder::start_recording()`.
- Reused existing selected-model, model override, param override, cloud known-model, external profile, and API-key resolution logic for preflight.
- Added local Whisper model file availability preflight without loading the model or checking daemon state.
- Added ffmpeg availability preflight for the resolved recording output format.
- Routed preflight failures through the existing record TUI setup error screen before audio startup.
- Added fixture-based tests for no selected model, missing cloud API key, missing local model file, custom provider no-probe behavior, and missing ffmpeg.
- Marked all Spec 1.3 tasks complete in `PLAN.md` as they were completed.

Decisions made:
- Introduced a narrow transcription preflight context rather than broadening `build_context`, so preflight can validate model/config without loading keywords.
- Kept custom `command` and `http` preflight to existing config/profile resolution only; no command execution, endpoint reachability, network, or executable checks were added.
- Included local-model and ffmpeg preflight causes in the primary error string because the current TUI error display still renders `anyhow::Error::to_string()` until Spec 1.4.

Files changed:
- `src/commands/record.rs`
- `src/transcription/context.rs`
- `src/transcription/mod.rs`
- `specs/first-time-ux/PLAN.md`
- `specs/first-time-ux/SESSION.md`

Helpers/APIs introduced or reused:
- Reused `RecordingTui::new_pending_audio`, `RecordingTui::show_error`, and `show_recording_error` for preflight failures.
- Reused `resolve_selected_model`, `config_for_selected_model`, missing API-key guidance, `resolve_installed_model_path`, `resolve_recording_output_format`, and `ffmpeg::find_ffmpeg`.
- Introduced `build_preflight_context` and `TranscriptionPreflightContext` in `src/transcription/context.rs`.
- Introduced private `run_record_preflight` and `run_record_preflight_with_ffmpeg_check` in `src/commands/record.rs`; the latter exists only to test ffmpeg behavior without depending on host ffmpeg.

Verification results:
- `cargo check` passed.
- `cargo test transcription` passed with 63 tests.
- Additional targeted test `cargo test commands::record` passed with 8 tests.

Constraints for later sessions:
- Spec 1.4 should consolidate or replace the current `show_recording_error` top-level-only formatting with full useful error-chain formatting; do not add a parallel record TUI formatter without deciding how it relates to `format_audio_startup_error` and preflight messages.
- Runtime ffmpeg conversion errors still occur in `AudioRecorder::convert_with_ffmpeg`; Spec 1.4 must preserve that path even though preflight now checks availability.
- Later sessions should reuse `run_record_preflight` semantics for record setup failures rather than adding another pre-audio validation path.

Obstacles encountered:
- The first extra `cargo test commands::record` run failed because local-model and ffmpeg causes were only in the anyhow source chain while the existing TUI formatter displays only the top-level message. The preflight messages were adjusted to include those narrow causes in the primary string, then the test passed.

Open questions:
- None.

Out-of-scope observations:
- `loop.sh` remains modified from prior work and was left untouched/uncommitted.

## Session 4: Spec 1.4 — Error Dialog Detail And ffmpeg Messaging

Accomplished:
- Added shared record TUI error formatting that shows primary error, cause chain, root cause when nested, and known next steps.
- Routed record TUI error display paths, including transcription failure dialogs, through the shared full-chain formatter.
- Improved missing-ffmpeg remediation for macOS with Homebrew, macOS without Homebrew, Linux, and other platforms.
- Preserved runtime ffmpeg lookup/conversion error handling in `AudioRecorder::convert_with_ffmpeg` while keeping preflight checks.
- Added tests for nested recording error-chain formatting and ffmpeg remediation message generation.
- Marked all Spec 1.4 tasks complete in `PLAN.md` as they were completed.

Decisions made:
- Replaced the narrow startup formatter with `format_recording_error` instead of adding another parallel TUI formatter.
- Kept ffmpeg remediation generation inside `src/recording/ffmpeg.rs` so both preflight and runtime conversion use the same missing-ffmpeg message through `find_ffmpeg`.

Files changed:
- `src/commands/record.rs`
- `src/recording/ffmpeg.rs`
- `specs/first-time-ux/PLAN.md`
- `specs/first-time-ux/SESSION.md`

Helpers/APIs introduced or reused:
- Reused `RecordingTui::show_error`, `show_recording_error`, `show_audio_startup_error`, and `audio_startup_next_step`.
- Introduced private `format_recording_error` and `record_error_next_step` in `src/commands/record.rs`.
- Introduced private ffmpeg remediation helpers `missing_ffmpeg_message`, `missing_ffmpeg_message_for`, `current_os`, `homebrew_likely_available`, and `command_exists` in `src/recording/ffmpeg.rs`.

Verification results:
- `cargo check` passed after removing an unused wrapper warning found on the first run.
- `cargo test recording` passed with 5 tests.

Constraints for later sessions:
- Review R1 should treat `format_recording_error` as the established record TUI error-formatting path and avoid adding another record dialog formatter unless consolidating deliberately.
- Runtime ffmpeg conversion still calls `find_ffmpeg`; future changes should not rely on preflight as the only ffmpeg failure surface.

Obstacles encountered:
- Initial `cargo check` passed but warned that the old startup formatter wrapper was unused; removed the wrapper and reran successfully.

Open questions:
- None.

Out-of-scope observations:
- `loop.sh` remains modified from prior work and was left untouched/uncommitted.

## Session 5: Review R1 — Record Flow Integration Review

Accomplished:
- Reviewed the record path from TUI initialization through preflight, audio startup, save, transcription, processing, and error display.
- Confirmed no duplicate or conflicting record-flow helper pattern requires a source change in this review scope.
- Marked R1 review tasks complete in `PLAN.md` as they were completed.

Decisions made:
- Kept R1.3 as a no-op because the existing helpers already form one coherent record-flow path.
- Treated `format_recording_error` as the single record TUI dialog formatter for setup, recording, transcription, and processing failures.

Files changed:
- `specs/first-time-ux/PLAN.md`
- `specs/first-time-ux/SESSION.md`

Helpers/APIs introduced or reused:
- Reused `RecordingTui::new_pending_audio(config)` for terminal takeover before preflight/audio startup.
- Reused `RecordingTui::set_sample_rate(sample_rate)` after successful audio startup.
- Reused `run_record_preflight` and `build_preflight_context` for model, params, cloud key, local Whisper file, custom provider config-only, and ffmpeg availability checks.
- Reused `ffmpeg::find_ffmpeg` as both the preflight and runtime conversion ffmpeg discovery path.
- Reused `show_recording_error`, `show_audio_startup_error`, and `format_recording_error` as the established TUI error display path.

Verification results:
- `cargo check` passed.
- `cargo test commands::record` passed with 9 tests.
- `cargo test recording` passed with 5 tests.

Constraints for later sessions:
- Later record-related changes should keep one `RecordingTui` instance alive from `new_pending_audio` through recording, transcription animation, processing animation, and action picker.
- Later sessions should not add a parallel record preflight path; extend `run_record_preflight` only if the record preflight scope explicitly grows.
- Later sessions should not add another record TUI error formatter; consolidate through `format_recording_error` unless a deliberate replacement is made.
- Runtime ffmpeg conversion must continue to call `find_ffmpeg`; preflight is not the only ffmpeg failure surface.

Obstacles encountered:
- None.

Open questions:
- None.

Out-of-scope observations:
- `loop.sh` remains modified from prior work and was left untouched/uncommitted.
