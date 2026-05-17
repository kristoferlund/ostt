## Session 1: Spec 1.1 - Local Provider Variant + Standard Build

Accomplished:
- Added `TranscriptionProvider::Local` and wired its `id()`, `name()`, `from_id()`, and `all()` behavior.
- Added a local model ID escape hatch to `TranscriptionConfig` via `provider`, `model_id`, and `new_local(...)` while preserving enum-backed cloud model behavior.
- Added the local provider stub module and routed `TranscriptionProvider::Local` through API dispatch.
- Verified no static `TranscriptionModel` variants were added for individual local models.

Obstacles encountered:
- `cargo check` failed twice while building `whisper-rs-sys` before OSTT code compiled. The build script reported WASI header diagnostics from `/opt/wasi-sdk/...` and then failed to execute `cmake` with `No such file or directory`, indicating missing native build tooling/environment configuration outside this section's code scope.
- Per failure protocol, task 1.1.8 was marked `[!]`; tasks 1.1.9 and 1.1.10 were not run.

Out-of-scope observations:
- Current auth/selection persistence stores only a model ID and infers provider from `TranscriptionModel`; future local selection work will need provider-aware persisted state or another local-specific selection path.
