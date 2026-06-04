# First-Time Popup UX Specification

Status: Not started

Target codebase: `/Users/kristoferlund/gh/ostt`

Primary journey: a first-time user runs `ostt launch --paste` or `ostt launch -c` from a hotkey and expects either a successful transcription output or a visible, actionable explanation of why it failed.

## Goals

1. A popup record flow must not fail as "nothing happened" for predictable first-run failures.
2. Once the recorder process starts, record-flow errors should prefer an OSTT TUI error screen over stderr or OS notifications.
3. OS notifications or native alerts are allowed only for cases where no OSTT popup/TUI can reasonably be shown, or after the popup has already closed.
4. Preflight should catch predictable setup failures before the user records audio.
5. Explicit output modes (`--clipboard`, `--paste`, `--output`) are contractual: if the requested output cannot be performed, the user must see an error or notification.

## Non-Goals And Deferred Work

- Do not add an `ostt doctor` command in this phase.
- Do not change the installer or hotkey documentation in this phase.
- Do not build a broad app-wide error framework unless required by the scoped record, launch, and paste flows.
- Do not probe custom command/HTTP transcription backends at runtime. Users who configure `command` or `http` profiles are treated as advanced users; config validation is enough for this phase.
- Do not require manual window-manager verification to pass automated tests. Ghostty/macOS sizing must have automated argument tests, but actual visual placement is manual/out of scope for test automation.

## Files Modified

Implementation work is expected to touch only these source files unless a scoped task proves one adjacent module is required:

- `src/app.rs`
- `src/clipboard.rs`
- `src/commands/launch.rs`
- `src/commands/output.rs`
- `src/commands/record.rs`
- `src/config/file.rs`
- `src/paste.rs`
- `src/recording/audio.rs`
- `src/recording/ffmpeg.rs`
- `src/recording/storage.rs`
- `src/recording/tui.rs`
- `src/transcription/context.rs`
- `src/transcription/local_models.rs`
- `src/transcription/mod.rs`

## Specification

### 1.1 Auth And Model Guidance Messages

Current issue:

- No selected model currently says only to run `ostt auth`, even though model selection is now a separate flow.
- Missing cloud API key also points only to `ostt auth` and can leave first-time users unsure whether they also need `ostt model`.

Required behavior:

- If no transcription model is selected, return exactly this primary message: `No transcription model selected. Run 'ostt auth' to add an API key, then 'ostt model' to choose a model.`
- If a cloud model is selected but the provider has no API key, return an actionable message that includes both adding credentials and confirming/selecting a model. The wording may include the provider display name.
- These are message-only changes; do not introduce new setup flows in this section.

Acceptance criteria:

- A fresh config with no selected provider/model produces the new no-model message before recording starts once preflight is implemented.
- A selected cloud provider/model with no API key produces a message that mentions `ostt auth` and `ostt model`.
- Tests cover the message strings or stable substrings.

### 1.2 Record TUI Lifecycle Starts Immediately

Current issue:

- `AudioRecorder::start_recording()` can fail before `RecordingTui::new(...)` exists, so hotkey-launched users may never see the error.

Required behavior:

- `handle_record` must establish a TUI context immediately when the record command starts, before transcription preflight and before audio startup.
- The same TUI context should be reused for the record flow: early error screen, recording waveform, transcription animation, processing animation, and action picker.
- The implementation may adjust `RecordingTui` so it can be initialized before the actual device sample rate is known, then update or configure sample-rate-dependent state after `AudioRecorder::start_recording()` succeeds.
- Do not add a separate popup error runner for record startup unless the normal TUI cannot support early initialization after a focused attempt.
- In normal terminal usage, preserving a visible TUI error screen is acceptable; do not rely only on stderr once the TUI context exists.

Required error content:

- Microphone/device errors must be shown in the TUI error screen.
- Likely macOS microphone permission failures must include this remediation, or a close equivalent: `Grant microphone access to your terminal app in System Settings > Privacy & Security > Microphone, then retry.`
- The error screen should include the primary error, useful cause details, and a concrete next step when known.

Acceptance criteria:

- Audio startup failures no longer occur before a user-visible TUI error surface exists.
- No-input-device, configured-device-not-found, and stream startup failures are routed through the record TUI error screen.
- Tests cover the error formatting or routing logic without requiring real audio hardware.

### 1.3 Record Preflight Before Audio Recording

Current issue:

- Missing model selection, missing cloud credentials, unavailable local models, and missing ffmpeg are discovered after recording or during save/transcription.

Required behavior:

- Add record-mode preflight that runs after the TUI context exists and before `AudioRecorder::start_recording()`.
- Preflight must check that a transcription model is selected.
- For cloud providers, preflight must check that the selected model is known and an API key exists.
- For the built-in local Whisper provider, preflight must check that the selected local model file exists/is downloaded. Do not load the model and do not start or check a daemon in this phase.
- For custom `command` and `http` providers, do not perform runtime reachability/executable checks. Existing config validation for required `command` or `endpoint` fields is enough.
- Preflight must check ffmpeg availability for the current recording save path because current recording storage uses ffmpeg conversion.
- Preflight failures must be shown immediately in the record TUI error screen and keep the popup open until dismissed.

Acceptance criteria:

- A user with no selected model sees the no-model TUI error before recording starts.
- A user with a selected cloud model but no API key sees the credentials/model TUI error before recording starts.
- A user with a selected Whisper model whose file is missing sees a local model availability TUI error before recording starts.
- A user without ffmpeg sees an install-remediation TUI error before recording starts.
- Tests cover cloud, no-model, local-file-missing, custom-provider-no-runtime-probe, and ffmpeg preflight branches using fixtures/mocks where practical.

### 1.4 Error Dialog Detail And ffmpeg Messaging

Current issue:

- Recording errors can be wrapped multiple times, and `anyhow::Error::to_string()` may show only the top-level context.
- Missing ffmpeg currently reports `ffmpeg not found. Please install ffmpeg.`, which is too generic for first-time users.

Required behavior:

- Record TUI error screens must render the full useful error chain, not only the top-level context.
- Error screens should include a concise title, primary error, cause chain or root cause, and a concrete next step when available.
- Missing ffmpeg remediation must be platform-aware:
  - macOS with Homebrew path likely available: mention `brew install ffmpeg`.
  - macOS without Homebrew detected: mention `https://brew.sh` and then `brew install ffmpeg`.
  - Linux: mention common package-manager examples or a clear install-docs fallback.
- Runtime ffmpeg conversion errors must still be handled even though preflight exists.

Acceptance criteria:

- A nested recording-save error displays the root cause text in the TUI error screen.
- Missing ffmpeg produces an actionable install message.
- Tests cover error-chain formatting and ffmpeg remediation message generation.

### 1.5 Popup Launch And Ghostty macOS Behavior

Current issue:

- If terminal detection or spawn fails, a hotkey-launched user may not see the error.
- Ghostty on macOS may not honor direct app-binary CLI invocation; `open -na Ghostty.app --args ...` is likely required.
- Fixed popup coordinates are brittle across screen sizes.

Required behavior:

- For `ostt launch`, terminal detection and spawn failures must print to stderr and also attempt an OS notification/native alert.
- macOS notification/alert may use `osascript`.
- Linux notification may use `notify-send` if available, with stderr fallback.
- Missing/unsupported terminal messages must say to install Ghostty, kitty, or Alacritty, or set `[popup].terminal` in `~/.config/ostt/ostt.toml`.
- When spawning the recorder from `launch`, set popup context for the child process, for example `OSTT_POPUP=1`.
- On macOS, Ghostty launch should try `open -na Ghostty.app --args ...` rather than invoking `/Applications/Ghostty.app/Contents/MacOS/ghostty` directly.
- Non-macOS Ghostty behavior should remain compatible with direct binary invocation.
- Screen-aware centering is a best-effort requirement: implement it only if it can be done simply and tested deterministically without new heavy dependencies. If it requires untestable window-manager probing or broad platform code, defer it and keep fixed defaults.

Acceptance criteria:

- Launch failures are actionable and notification-backed where supported.
- Spawned popup recorders receive popup context.
- Ghostty command construction has tests for macOS `open -na ... --args` and non-macOS direct invocation behavior.
- If screen-aware centering is implemented, tests cover deterministic position calculation. If deferred, `SESSION.md` must record why.

### 1.6 Explicit Clipboard And Paste Output Failures

Current issue:

- Explicit clipboard output may warn but still return success if no backend is available.
- Popup paste uses a detached helper; failures can happen after the popup closes and become invisible.
- Paste-key automation failures currently print a warning even though the popup terminal may be gone.

Required behavior:

- If the user explicitly requested `--clipboard` or `-c`, clipboard backend failure must be treated as a user-visible failure.
- In popup context, explicit clipboard failures should use the popup-safe error path available at that point. If the TUI is gone, use OS notification/native alert where supported.
- Optional clipboard behavior that is not the requested output mode may remain best-effort.
- If paste-key automation fails, leave the transcription text in the clipboard as fallback and return/report an error instead of silent success.
- In popup paste-helper context, paste failures must attempt OS notification/native alert because the popup has already closed.
- macOS paste automation failures should mention Accessibility permissions: `Grant accessibility permissions to your terminal app or OSTT launcher in System Settings > Privacy & Security > Accessibility.`

Acceptance criteria:

- Explicit `--clipboard` failure no longer exits as success without a visible error.
- Paste-key failure reports an error/notification and leaves text in the clipboard as fallback.
- Detached paste-helper failure attempts notification in popup context.
- Tests cover explicit clipboard error behavior, paste-key failure behavior, and macOS remediation text generation where practical.

## Dependency Notes

- `1.1` is independent and should be implemented first.
- `1.2` must precede record preflight because preflight failures need a TUI surface.
- `1.3` depends on `1.1` and `1.2`.
- `1.4` depends on `1.2` and supports `1.3` runtime errors.
- `1.5` is mostly independent, but popup context is useful for `1.6`.
- `1.6` depends on popup context and notification helpers from `1.5`.

## Verification Commands

Run targeted tests for each section where possible, then run:

- `cargo check`
- `cargo clippy -- -D warnings`
- `cargo test`
