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
