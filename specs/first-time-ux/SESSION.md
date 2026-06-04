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
