# Phase 1 — Provider & Plumbing Implementation Plan

Scope: `1.1 - Local Provider Variant + Feature Flag.md` and `1.2 - whisper-rs Integration & Model Loading.md`

Target codebase path: `/Users/kristoferlund/gh/ostt`

Status: Not started

Assumptions:
- Local transcription is part of the standard build; no feature flag gating is planned despite the folder name.
- `TranscriptionConfig` currently stores `TranscriptionModel`, so local model ID support must add a string escape hatch before full local inference can work.
- `SESSION.md` will be created by the first implementation session if it does not already exist.

Deferred:
- GPU acceleration is deferred to Phase 3.
- Parameter passthrough from config is deferred to spec 3.1; Phase 1 uses hardcoded defaults.
- Progress callbacks and streaming are deferred.

## Dependency Order

1.1 → 1.2

Spec 1.1 creates the standard dependency, provider variant, dispatch path, and local provider stub. Spec 1.2 depends on that module existing and replaces the stub with whisper-rs model loading, audio validation, inference, and filtering.

Recommended execution order:
1. Complete 1.1 first because it is smaller and establishes the compile-time provider plumbing.
2. Complete 1.2.A next because local model path resolution is needed before inference can locate a model.
3. Complete 1.2.B last because it depends on model resolution and adds the blocking whisper-rs runtime path.

## Tasks

### Spec 1.1 — Local Provider Variant + Standard Build

- [x] **1.1.1** Add `whisper-rs = "0.14"` to `/Users/kristoferlund/gh/ostt/Cargo.toml` standard dependencies.
- [x] **1.1.2** Add `TranscriptionProvider::Local` in `/Users/kristoferlund/gh/ostt/src/transcription/provider.rs`.
- [x] **1.1.3** Update `TranscriptionProvider::id()`, `name()`, `from_id()`, and `all()` for `local` / `Local (whisper.cpp)`.
- [x] **1.1.4** Add a local model ID string escape hatch to transcription request configuration so `provider = local` is not blocked by `TranscriptionModel::from_id()`.
- [x] **1.1.5** Create `/Users/kristoferlund/gh/ostt/src/transcription/api/local.rs` with the stub `transcribe(config, audio_path)` signature matching existing providers.
- [x] **1.1.6** Declare `mod local;` and route `TranscriptionProvider::Local` to `local::transcribe(config, audio_path).await` in `/Users/kristoferlund/gh/ostt/src/transcription/api/mod.rs`.
- [x] **1.1.7** Verify no static `TranscriptionModel` variants were added for individual local models.
- [!] **1.1.8** Verify: run `cargo check`.
- [!] **1.1.9** Verify: run `cargo clippy -- -D warnings`.
- [x] **1.1.10** Verify: run `cargo test`.

### Spec 1.2.A — Local Model Resolution

Depends on: 1.1

- [ ] **1.2.1** Add `thiserror` to `/Users/kristoferlund/gh/ostt/Cargo.toml` if it is still absent.
- [ ] **1.2.2** Create `/Users/kristoferlund/gh/ostt/src/transcription/local_models.rs` with `models_dir()` and `ModelError` variants from the spec.
- [ ] **1.2.3** Export `local_models` from `/Users/kristoferlund/gh/ostt/src/transcription/mod.rs`.
- [ ] **1.2.4** Add minimal registry/custom model state types needed to deserialize registry entries and `models.json.custom_models`.
- [ ] **1.2.5** Implement `model_filename(id, url)` for deriving installed whisper.cpp-compatible filenames.
- [ ] **1.2.6** Implement registry loading from the GitHub registry source already used or specified by the codebase; if no source exists, keep the lookup helper isolated and return clear errors.
- [ ] **1.2.7** Implement custom model entry loading from `models.json` under the OSTT models directory.
- [ ] **1.2.8** Implement installed model path resolution under `models/files/`, returning `ModelError::NotDownloaded` when the expected file is absent.
- [ ] **1.2.9** Verify: run `cargo check`.
- [ ] **1.2.10** Verify: run `cargo clippy -- -D warnings`.

### Spec 1.2.B — whisper-rs Transcription Runtime

Depends on: 1.2.A

- [ ] **1.2.11** Replace the local provider stub with model ID lookup from the local string escape hatch or selected-model state.
- [ ] **1.2.12** Validate local input audio is WAV, signed 16-bit PCM, 16 kHz, mono before transcription and return actionable config guidance on mismatch.
- [ ] **1.2.13** Load compatible WAV samples into normalized `Vec<f32>` without conversion or resampling.
- [ ] **1.2.14** Run whisper-rs model loading and inference inside `tokio::task::spawn_blocking`.
- [ ] **1.2.15** Apply hardcoded MVP whisper parameters: no timestamps, no context, temperature `0.0`, entropy threshold `2.4`, no-speech threshold `0.6`.
- [ ] **1.2.16** Collect segment text into a trimmed transcription string.
- [ ] **1.2.17** Add anti-hallucination filtering for empty text, blank/silence tokens, music tokens, and low-alphanumeric output; return empty string when filtered.
- [ ] **1.2.18** Map missing model, invalid model, incompatible audio, and whisper-rs runtime failures to clear errors.
- [ ] **1.2.19** Verify: run `cargo clippy -- -D warnings`.
- [ ] **1.2.20** Verify: run `cargo test`.

## Verification Protocol

After each section or sub-section:
- Run the verification commands listed as tasks in that section.
- If a verification task fails, fix only issues in the current section scope and rerun the same command.
- If the same verification task fails twice, follow the failure protocol in the session prompt.

After all specs are complete:
- Run `cargo check`.
- Run `cargo clippy -- -D warnings`.
- Run `cargo test`.
- Optionally run `cargo build` when native whisper-rs build dependencies are available.

## Session Boundaries

The unit of work per session is one section or sub-section with at most 10 tasks. The available units are:
- Spec 1.1
- Spec 1.2.A
- Spec 1.2.B

Each session must complete one unit, update task checkboxes immediately after each completed task, commit all changes, append to `SESSION.md`, and stop. Do not continue into the next section or sub-section.

Stop early when:
- The same verification command fails twice after a focused fix attempt.
- The current task requires modifying files outside the target codebase, `PLAN.md`, or `SESSION.md`.
- The spec and current code conflict in a way that changes product behavior beyond the current section.

The agent must git commit before stopping every time, including partial work after the failure protocol.

## Session Prompt Template

```text
You are implementing Phase 1 local model support for OSTT.

Paths:
- Plan: /Users/kristoferlund/gh/ostt/specs/local-models/Phase 1 - Provider & Plumbing/PLAN.md
- Session notes: /Users/kristoferlund/gh/ostt/specs/local-models/Phase 1 - Provider & Plumbing/SESSION.md
- Spec folder: /Users/kristoferlund/gh/ostt/specs/local-models/Phase 1 - Provider & Plumbing
- Target codebase: /Users/kristoferlund/gh/ostt

Read PLAN.md first. Find the next incomplete section or sub-section: the first section/sub-section containing unchecked tasks. That section/sub-section is your entire scope for this session. STOP after completing that one section/sub-section. Do not continue to the next section/sub-section.

Read the spec file for the selected section:
- Spec 1.1: /Users/kristoferlund/gh/ostt/specs/local-models/Phase 1 - Provider & Plumbing/1.1 - Local Provider Variant + Feature Flag.md
- Spec 1.2: /Users/kristoferlund/gh/ostt/specs/local-models/Phase 1 - Provider & Plumbing/1.2 - whisper-rs Integration & Model Loading.md

Read SESSION.md if it exists. Use previous notes as context. Append to SESSION.md at the end; do not overwrite it.

Study the relevant source files in /Users/kristoferlund/gh/ostt before editing. Read exports, immediate callers, and shared utilities needed for the current section. Do not do unrelated refactors.

Implement tasks in exact checklist order. No skipping tasks. No reordering. SCOPE is one section/sub-section only.

CRITICAL crash-recovery rule: update PLAN.md IMMEDIATELY after completing each task by changing that task from `- [ ]` to `- [x]` before starting the next task. Do not batch PLAN.md updates.

Verification:
- Run the verification command tasks in order when reached.
- If a verification command fails, make a focused fix within the current section scope and rerun it once.
- If the same verification command fails twice, change that task marker in PLAN.md from `- [ ]` to `- [!]`, append a SESSION.md summary describing the failure, git commit partial work, and stop.

File modification restrictions:
- You may modify files only under /Users/kristoferlund/gh/ostt, PLAN.md, and SESSION.md.
- Do not modify files outside the target codebase except PLAN.md and SESSION.md.
- Do not implement deferred items: GPU acceleration, config-driven whisper parameters, progress callbacks, or streaming.

Session notes:
- Append a summary to /Users/kristoferlund/gh/ostt/specs/local-models/Phase 1 - Provider & Plumbing/SESSION.md.
- Use heading `## Session N: Spec X.Y — <title>` and increment N from existing headings.
- Include: what was accomplished, obstacles encountered, out-of-scope observations.

Git:
- Before stopping, git commit all changes from this session.
- Use a concise commit message matching repository style.
- Do not push.

Stop condition:
- Stop after one section/sub-section is complete and committed, or after the failure protocol is committed.
```
