# OSTT — Project Overview

An interactive TUI tool for audio recording with real-time visualization and speech-to-text transcription via multiple AI providers (OpenAI, Deepgram, DeepInfra, Groq, AssemblyAI, Berget). Cross-platform: Linux and macOS.

## What It Does

- **Audio recording** with real-time waveform visualization in the terminal
- **Multi-provider transcription** — OpenAI, Deepgram, DeepInfra, Groq, AssemblyAI, Berget
- **History** — SQLite-backed recording and transcription history
- **Clipboard integration** — copy transcriptions directly
- **Keywords** — custom keyword lists for transcription accuracy
- **Configuration** — TOML-based config with secrets management

## Repos

- **Source**: [kristoferlund/ostt](https://github.com/kristoferlund/ostt)

## Evolution Roadmap — Post-Processing Pipeline

### Phase 1 — Config Types & Input Resolution
Define the data model for processing actions in TOML config and implement runtime input resolution.

| # | Item | Priority | Effort |
|---|------|----------|--------|
| 1.1 | [Action Config Types](Phase%201/1.1%20—%20Action%20Config%20Types.md) | Critical | Medium |
| 1.2 | [Input Resolution](Phase%201/1.2%20—%20Input%20Resolution.md) | Critical | Small |

Implementation plan: [Phase 1 Plan](Phase%201/PLAN.md)

### Phase 2 — Chat Completions Client
API client for sending messages to OpenAI-compatible chat endpoints.

| # | Item | Priority | Effort |
|---|------|----------|--------|
| 2.1 | [Chat Completions Client](Phase%202/2.1%20—%20Chat%20Completions%20Client.md) | Critical | Medium |

Implementation plan: [Phase 2 Plan](Phase%202/PLAN.md)

### Phase 3 — Action Execution
Bash command executor and unified action dispatcher.

| # | Item | Priority | Effort |
|---|------|----------|--------|
| 3.1 | [Bash Action Executor](Phase%203/3.1%20—%20Bash%20Action%20Executor.md) | High | Small |
| 3.2 | [Action Dispatcher](Phase%203/3.2%20—%20Action%20Dispatcher.md) | High | Small |

Implementation plan: [Phase 3 Plan](Phase%203/PLAN.md)

### Phase 4 — Action Picker TUI
Interactive action selection UI using ratatui.

| # | Item | Priority | Effort |
|---|------|----------|--------|
| 4.1 | [Action Picker TUI](Phase%204/4.1%20—%20Action%20Picker%20TUI.md) | High | Medium |

Implementation plan: [Phase 4 Plan](Phase%204/PLAN.md)

### Phase 5 — CLI Integration
The `process` subcommand and `-p` flag on record/transcribe/retry.

| # | Item | Priority | Effort |
|---|------|----------|--------|
| 5.1 | [Process Subcommand](Phase%205/5.1%20—%20Process%20Subcommand.md) | Critical | Medium |
| 5.2 | [Process Flag on Record, Transcribe, Retry](Phase%205/5.2%20—%20Process%20Flag%20on%20Record%2C%20Transcribe%2C%20Retry.md) | Critical | Large |

Implementation plan: [Phase 5 Plan](Phase%205/PLAN.md)

### Phase 6 — Processing Animation
Add status label to the logo animation for visual phase distinction.

| # | Item | Priority | Effort |
|---|------|----------|--------|
| 6.1 | [Processing Animation with Status Label](Phase%206/6.1%20—%20Processing%20Animation%20with%20Status%20Label.md) | Medium | Small |

Implementation plan: [Phase 6 Plan](Phase%206/PLAN.md)
