# Phase 1 — First-Time Popup UX Implementation Plan

Scope: `/Users/kristoferlund/gh/ostt/specs/first-time-ux/spec.md`

Target codebase path: `/Users/kristoferlund/gh/ostt`

Status: Not started

## Dependency Order

Execution order:

`1.1 -> 1.2 -> 1.3 -> 1.4 -> R1 -> 1.5.A -> 1.5.B -> 1.6 -> R2`

Rationale: start with isolated message changes, then make the record TUI available immediately so later preflight failures have a user-visible surface. Add preflight and richer record error details next, then run a record-flow review before moving to launch/paste surfaces. Launch notification/context and Ghostty behavior then prepare the no-popup surfaces used by explicit paste/clipboard failures, followed by a final coherence review.

Dependencies:

`1.1` has no dependencies.

`1.2` depends on `1.1` only for final message consistency.

`1.3` depends on `1.1` and `1.2`.

`1.4` depends on `1.2` and supports runtime failures left after `1.3` preflight.

`R1` depends on `1.2`, `1.3`, and `1.4`; it verifies record-flow coherence before launch/paste work starts.

`1.5.A` has no code dependency on record flow, but should follow record work so popup context semantics are clear.

`1.5.B` depends on `1.5.A` for launch command construction changes.

`1.6` depends on `1.5.A` because popup context and notification helpers are needed after the popup closes.

`R2` depends on all implementation sections; it verifies cross-flow coherence before the phase is considered complete.

Deferrals: installer changes, hotkey docs, `ostt doctor`, local daemon checks, runtime probes for custom command/http profiles, durable paste status files, and broad app-wide failure abstractions are excluded from this phase.

## Tasks

### Spec 1.1 — Auth And Model Guidance Messages

Files: `src/transcription/context.rs`, `src/transcription/mod.rs`

- [x] **1.1.1** Update the no-selected-model error to exactly: `No transcription model selected. Run 'ostt auth' to add an API key, then 'ostt model' to choose a model.`
- [x] **1.1.2** Update the missing cloud API key error to include both credential setup via `ostt auth` and model confirmation/selection via `ostt model`.
- [x] **1.1.3** Add or update unit tests covering the no-model guidance text.
- [x] **1.1.4** Add or update unit tests covering stable missing-API-key guidance substrings.
- [x] **1.1.5** Verify: `cargo check`
- [x] **1.1.6** Verify: `cargo test transcription::context`

### Spec 1.2 — Record TUI Lifecycle Starts Immediately

Depends on: `1.1`

Files: `src/commands/record.rs`, `src/recording/tui.rs`, `src/recording/audio.rs`

- [x] **1.2.1** Adjust `RecordingTui` or add a narrow TUI context so `handle_record` can take over the terminal before preflight/audio startup.
- [x] **1.2.2** Preserve reuse of the same TUI context for early errors, waveform rendering, transcription animation, processing animation, and action picker.
- [x] **1.2.3** Refactor `handle_record` so TUI initialization happens before `AudioRecorder::start_recording()`.
- [x] **1.2.4** Update sample-rate-dependent state after audio startup succeeds without requiring a second terminal takeover.
- [x] **1.2.5** Route no-input-device, configured-device-not-found, device-config, stream-create, and stream-start failures through the TUI error screen.
- [x] **1.2.6** Add macOS microphone permission remediation text for likely permission/device-denial startup errors.
- [x] **1.2.7** Add tests for startup-error formatting or routing without real audio hardware.
- [x] **1.2.8** Verify: `cargo check`
- [x] **1.2.9** Verify: `cargo test recording::tui`
- [x] **1.2.10** Verify: `cargo test commands::record`

### Spec 1.3 — Record Preflight Before Audio Recording

Depends on: `1.1`, `1.2`

Files: `src/commands/record.rs`, `src/transcription/context.rs`, `src/transcription/mod.rs`, `src/transcription/local_models.rs`, `src/recording/ffmpeg.rs`, `src/config/file.rs`

- [x] **1.3.1** Add a record preflight entry point that runs after TUI initialization and before audio startup.
- [x] **1.3.2** Preflight selected model resolution, including model override and param overrides.
- [x] **1.3.3** Preflight cloud providers by checking known provider/model and available API key.
- [x] **1.3.4** Preflight built-in Whisper provider by checking the selected model file exists/is downloaded without loading the model or checking daemon state.
- [x] **1.3.5** Ensure custom `command` and `http` providers do not perform runtime executable, endpoint, or network probes beyond existing config validation.
- [x] **1.3.6** Preflight ffmpeg availability for the current recording save path.
- [x] **1.3.7** Show every preflight failure immediately through the TUI error screen and keep the popup open until dismissed.
- [x] **1.3.8** Add fixture-based tests for no-model, missing cloud key, missing local model file, custom-provider no-probe behavior, and missing ffmpeg.
- [x] **1.3.9** Verify: `cargo check`
- [x] **1.3.10** Verify: `cargo test transcription`

### Spec 1.4 — Error Dialog Detail And ffmpeg Messaging

Depends on: `1.2`

Files: `src/commands/record.rs`, `src/recording/tui.rs`, `src/recording/ffmpeg.rs`, `src/recording/storage.rs`, `src/recording/audio.rs`

- [x] **1.4.1** Add narrow error formatting for record TUI dialogs that includes primary error, useful cause chain/root cause, and concrete next step when available.
- [x] **1.4.2** Update record error display paths to use the full formatted error instead of only `anyhow::Error::to_string()`.
- [x] **1.4.3** Improve ffmpeg-not-found remediation for macOS with Homebrew, macOS without Homebrew, and Linux.
- [x] **1.4.4** Preserve runtime ffmpeg conversion error handling even when preflight exists.
- [x] **1.4.5** Ensure nested save/encode errors expose the root cause text in the TUI error screen.
- [x] **1.4.6** Add tests for error-chain formatting.
- [x] **1.4.7** Add tests for ffmpeg remediation message generation.
- [x] **1.4.8** Verify: `cargo check`
- [x] **1.4.9** Verify: `cargo test recording`

### Review R1 — Record Flow Integration Review

Depends on: `1.2`, `1.3`, `1.4`

Files: `src/commands/record.rs`, `src/recording/tui.rs`, `src/recording/ffmpeg.rs`, `src/transcription/context.rs`, `src/transcription/local_models.rs`, `SESSION.md`

- [x] **R1.1** Read `SESSION.md` and recent commits; list the record-flow helpers/APIs that later sessions must reuse.
- [x] **R1.2** Review the record path from TUI initialization through preflight, audio startup, save, transcription, and error display for duplicate or conflicting patterns.
- [x] **R1.3** Make only small consistency fixes needed to align existing record-flow helpers; if a larger redesign seems needed, stop and ask.
- [x] **R1.4** Append a handoff note naming the established TUI, preflight, and error-formatting patterns for future sessions.
- [x] **R1.5** Verify: `cargo check`
- [x] **R1.6** Verify: `cargo test commands::record`
- [x] **R1.7** Verify: `cargo test recording`

### Spec 1.5.A — Launch Failure Notification And Popup Context

Files: `src/app.rs`, `src/commands/launch.rs`, `src/paste.rs`

- [x] **1.5.1** Add a narrow popup-safe notification/native-alert helper for no-popup surfaces: macOS `osascript`, Linux `notify-send` when available, stderr fallback.
- [x] **1.5.2** Route terminal detection failures in `ostt launch` through stderr plus the notification/native-alert helper.
- [x] **1.5.3** Route terminal spawn failures in `ostt launch` through stderr plus the notification/native-alert helper.
- [x] **1.5.4** Make missing/unsupported terminal errors mention installing Ghostty, kitty, or Alacritty, or setting `[popup].terminal`.
- [x] **1.5.5** Set popup context for spawned recorder processes, for example `OSTT_POPUP=1`.
- [x] **1.5.6** Add tests for actionable launch failure messages.
- [x] **1.5.7** Add tests that popup context is included in launch command/spawn setup without executing terminals.
- [x] **1.5.8** Verify: `cargo check`
- [x] **1.5.9** Verify: `cargo test commands::launch`

### Spec 1.5.B — Ghostty macOS Launch And Best-Effort Positioning

Depends on: `1.5.A`

Files: `src/commands/launch.rs`, `src/config/file.rs`

- [x] **1.5.10** Change macOS Ghostty command construction to use `open -na Ghostty.app --args ...` where appropriate.
- [x] **1.5.11** Preserve non-macOS Ghostty direct binary invocation behavior.
- [x] **1.5.12** Attempt screen-aware centering only if it can be implemented simply and tested deterministically without new heavy dependencies.
- [x] **1.5.13** If screen-aware centering is not implemented, record the deferral reason in `SESSION.md` and keep fixed defaults.
- [x] **1.5.14** Add tests for macOS Ghostty `open -na ... --args` command shape.
- [x] **1.5.15** Add tests for non-macOS Ghostty direct command shape.
- [x] **1.5.16** Add deterministic position-calculation tests if screen-aware centering is implemented.
- [x] **1.5.17** Verify: `cargo check`
- [x] **1.5.18** Verify: `cargo test commands::launch`

### Spec 1.6 — Explicit Clipboard And Paste Output Failures

Depends on: `1.5.A`

Files: `src/clipboard.rs`, `src/commands/output.rs`, `src/commands/record.rs`, `src/paste.rs`

- [x] **1.6.1** Change explicit `--clipboard`/`-c` output so clipboard backend failure is a returned user-visible error instead of warning-only success.
- [x] **1.6.2** Preserve best-effort clipboard behavior only for optional clipboard copies that are not the requested output mode.
- [x] **1.6.3** Change paste-key automation failure to leave transcription text in the clipboard and return/report an error.
- [x] **1.6.4** Add macOS Accessibility remediation text for AppleScript/System Events paste failures.
- [x] **1.6.5** Make detached paste-helper failures in popup context attempt OS notification/native alert because the popup has closed.
- [x] **1.6.6** Ensure record popup paste failures after TUI cleanup use the no-popup notification path rather than only stderr/logging.
- [x] **1.6.7** Add tests for explicit clipboard failure behavior.
- [x] **1.6.8** Add tests for paste-key failure behavior and remediation text.
- [x] **1.6.9** Verify: `cargo check`
- [x] **1.6.10** Verify: `cargo test paste`

### Review R2 — Final Popup UX Coherence Review

Depends on: `1.1`, `1.2`, `1.3`, `1.4`, `R1`, `1.5.A`, `1.5.B`, `1.6`

Files: `src/commands/record.rs`, `src/commands/launch.rs`, `src/commands/output.rs`, `src/paste.rs`, `SESSION.md`

- [x] **R2.1** Read `SESSION.md` and recent commits; identify the final established patterns for TUI errors, no-popup notifications, preflight, and output failures.
- [x] **R2.2** Review launch, record, clipboard, and paste flows for duplicate helpers, inconsistent messages, or behavior that violates deferred/non-goal scope.
- [x] **R2.3** Make only small consistency fixes; if a larger redesign or extra scope is required, stop and ask.
- [x] **R2.4** Append a final handoff note summarizing implemented behavior, remaining deferrals, and any manual verification needed.
- [x] **R2.5** Verify: `cargo check`
- [x] **R2.6** Verify: `cargo clippy -- -D warnings`
- [x] **R2.7** Verify: `cargo test`

## Verification Protocol

After each section or sub-section is complete, run the verification tasks listed at the end of that section.

After all specs are complete, run:

- `cargo check`
- `cargo clippy -- -D warnings`
- `cargo test`

If a targeted test command is invalid for the current module layout, replace it with the narrowest valid `cargo test` filter that covers the changed code and record the substitution in `SESSION.md`.

## Session Boundaries

The unit of work per session is ONE section or sub-section with at most 10 tasks. If a spec is split into sub-sections, each sub-section is a separate session. The agent completes one section or sub-section, commits, and stops.

Stop early if the same verification task fails twice after attempted fixes. Mark the failing task with `[!]`, append a session summary, commit partial work, and stop.

If the scoped spec or task is vague, conflicts with the clarified non-goals, or risks over-implementing beyond the listed files and acceptance criteria, stop and ask the user before editing further.

The agent must git commit before stopping every time.

## Handoff Protocol

Each session must treat `SESSION.md` as the continuity layer between fresh agent runs.

Before editing, read prior session notes and recent commits, then reuse established helpers and patterns unless they are clearly wrong. Do not create parallel helpers for the same concern without documenting why the existing helper is unsuitable.

At the end of every session, append a handoff note to `SESSION.md` with: decisions made, files changed, helpers/APIs introduced or reused, verification results, constraints for later sessions, and open questions. Review sessions must also summarize the current architecture for the flows they reviewed.

## Session Prompt Template

```text
You are working in target codebase `/Users/kristoferlund/gh/ostt`.

Read `/Users/kristoferlund/gh/ostt/specs/first-time-ux/PLAN.md`, `/Users/kristoferlund/gh/ostt/specs/first-time-ux/SESSION.md` if it exists, and the spec files folder `/Users/kristoferlund/gh/ostt/specs/first-time-ux`.

Find the next incomplete section or sub-section in PLAN.md. This is the first section or sub-section containing unchecked tasks. SCOPE is exactly that one section or sub-section only. Do not continue to the next section or sub-section.

Read `/Users/kristoferlund/gh/ostt/specs/first-time-ux/spec.md`, then study the source files listed for the scoped section in PLAN.md and the immediate callers/types/utilities needed to make safe changes. Do not do broad codebase exploration.

Before editing, read prior session notes and recent commits. Identify helpers/APIs/patterns already introduced for this feature and reuse them unless clearly unsuitable. Do not create parallel helpers for the same concern without documenting why.

Implement tasks in order. No skipping tasks. No reordering tasks. If the scoped spec or task is vague, conflicts with the clarified non-goals, or risks over-implementing beyond the listed files and acceptance criteria, stop and ask the user before editing further.

After completing each task, update PLAN.md IMMEDIATELY before starting the next task by changing that task from `- [ ]` to `- [x]`. Do not batch PLAN.md updates. This is required for crash recovery.

Run the verification task(s) listed in the section as they are reached. If verification fails, fix and retry once. If the same verification task fails twice, mark that task as `- [!]`, append the session summary to SESSION.md, git commit partial work, and stop.

Restrict file modifications to `/Users/kristoferlund/gh/ostt`, `/Users/kristoferlund/gh/ostt/specs/first-time-ux/PLAN.md`, and `/Users/kristoferlund/gh/ostt/specs/first-time-ux/SESSION.md` only.

Append, do not overwrite, a session summary to `/Users/kristoferlund/gh/ostt/specs/first-time-ux/SESSION.md`. Use heading `## Session N: Spec X.Y — <title>` with N incremented from existing sessions. Include what was accomplished, decisions made, files changed, helpers/APIs introduced or reused, verification results, constraints for later sessions, obstacles encountered, open questions, and out-of-scope observations.

Before stopping, git commit all changes. Commit only files changed for this scoped section or sub-section plus PLAN.md and SESSION.md. Stop after one section or sub-section even if more tasks remain elsewhere.
```
