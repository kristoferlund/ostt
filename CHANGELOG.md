# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## 0.0.16 - 2026-06-01

### Added

- Added per-run transcription model selection with `-m, --model <PROVIDER/MODEL>` for `ostt`, `ostt record`, `ostt transcribe`, `ostt retry`, and `ostt launch`. The override applies only to the current invocation and does not change the saved default model.
- Added scriptable CLI actions for models, keywords, auth, history, config helpers, logs, and local model download/removal.
- Added per-run model option overrides with repeatable `--mo key=value`, validated against the selected provider/model option schema for supported transcription providers.
- Added `ostt model options [PROVIDER/MODEL] --format table|json` to list supported model option keys for scripting.
- Added OpenAI `gpt-4o-transcribe-diarize` and validated JSON-safe OpenAI transcription options for logprobs, Whisper timestamp metadata, and diarization requests.
- Added validated JSON-safe Groq transcription options for `response_format` and `timestamp_granularities` based on Groq's speech-to-text documentation.
- Added more DeepInfra speech-recognition models and validated DeepInfra-native options including `initial_prompt`, `task`, `chunk_level`, and `chunk_length_s`.
- Added validated Berget transcription options for JSON-safe response format, word alignment, diarization, speaker embeddings, chunk size, and batch size.
- Added validated ElevenLabs Scribe options for timestamps, file format, entity detection/redaction, diarization rules, and speaker-role detection rules.
- Added validated Mistral Voxtral transcription options for `language`, `temperature`, `timestamp_granularities`, `diarize`, and `context_bias`.

### Changed

- Cloud transcription models now use provider/model identities such as `deepgram/nova-3`, `deepinfra/openai/whisper-large-v3`, and `berget/KBLab/kb-whisper-large`, replacing older OSTT-invented model IDs.
- Transcription request options are now model-scoped under `[model_options."provider/model"]` instead of provider-scoped settings under `[providers.*]`.
- Local model UI and error messages now show full public IDs such as `local/turbo`.
- Cleaned up command syntax: `ostt keyword` replaces `ostt keywords`, `ostt config list-devices` replaces `ostt list-devices`, `ostt process list` replaces `ostt process --list`, and `ostt completions install <shell>` replaces `ostt completions <shell> --install`.
- Record-only options no longer appear in unrelated management command help.

### Removed

- Removed the misleading `audio.sample_rate` configuration setting. Recording now uses the input device sample rate, while local transcription compatibility is controlled by `audio.output_format = "pcm_s16le -ar 16000"`.

## 0.0.15 - 2026-05-27

### Fixed

- Fixed Kitty popup launch behavior by using cell-based dimensions, supported window flags, macOS app bundle detection, configured positioning, and automatic quit when the popup closes.

## 0.0.14 - 2026-05-22

### Fixed

- Fixed macOS local artifact builds by removing a Linux-only runtime directory fallback from shared code.

## 0.0.13 - 2026-05-22

### Changed

- `ostt model` now shows the currently selected provider/model before choosing between local and cloud models.
- Routine popup/recording lifecycle messages now log at debug level to keep normal logs quieter.

### Fixed

- `ostt launch` now targets only the active recorder process via a recorder-owned runtime PID file, preventing the hotkey from accidentally signaling the local model daemon.
- Model selection now uses only `[transcription]` in `ostt.toml`; the legacy `~/.local/share/ostt/model` file is ignored and no longer migrated.
- Avoided printing duplicate logos during authentication flows.
- Improved error screen text color for better readability.

## 0.0.12 - 2026-05-21

### Added

- **Local transcription** — Run transcription fully offline using whisper.cpp. Metal is enabled on all macOS builds (Apple Silicon ~180 ms). CUDA and Vulkan builds are available for Linux. The install script auto-selects the right GPU variant.
- **Local model management** — `ostt model` lets users browse, download, activate, and delete local models from a hosted registry. Custom models can be added via Hugging Face URLs or direct `.gguf`/`ggml-*.bin` links.
- **Local model daemon** — A persistent background process keeps the model loaded between transcriptions, eliminating per-call startup cost. Managed via `ostt daemon start|stop|restart|status|install|uninstall`. Installs as a launchd service (macOS) or systemd user unit (Linux). Daemon logs are merged into `ostt logs`.
- **Mistral provider** - New transcription provider with Voxtral Mini Transcribe and Voxtral Mini 2602 models. Supports optional language hints via `[providers.mistral].language`.

### Changed

- Updated model picker UI, dialogs, and toasts to use terminal ANSI colors and consistent local/cloud model screens.

## 0.0.11 - 2026-05-13

### Fixed

- Preserved user `ostt.toml` changes during version updates and added validation for the embedded default config template.

### Added

- Added process-level default AI tool and model settings for processing actions.
- **Shell completions** - `ostt completions <shell> --install` auto-detects the shell and writes completions to the standard system directory. Native packages (`.deb`, `.rpm`) ship completion files pre-installed. The AUR PKGBUILD generates them at build time.

## 0.0.10 - 2026-05-12

### Fixed

- Fixed invalid TOML in the embedded default config template for AssemblyAI language detection options.
- Suppressed VLC GUI and DBus noise during `ostt replay` on Linux by using terminal-friendly playback commands.
- Added ARM64 Linux package builds so Ubuntu arm64 users can install a matching `.deb` package.

## 0.0.9 - 2026-05-12

### Added

- **Process command** - New `ostt process` subcommand for running processing actions on history items. New `-p`/`--process` flag on `record`, `transcribe`, and `retry` commands for post-transcription processing. Includes AI tool execution (OpenCode, Claude Code, Gemini CLI, Codex CLI), bash command execution, and an action picker TUI for selecting which action to run. Actions are configured under `[process.actions]` in `~/.config/ostt/ostt.toml`.
- **Launch command** - New `ostt launch` subcommand for cross-platform popup recording. Opens a terminal popup that starts recording immediately, with toggle support (press hotkey again to stop). Auto-detects ghostty, kitty, alacritty, foot, konsole, gnome-terminal, and xfce4-terminal. Replaces the old Hyprland-specific float script.
- **ElevenLabs provider** - New transcription provider with Scribe v2 and Scribe v1 models. Supports optional language hints via `[providers.elevenlabs].language_code`.
- `.deb` package for Debian/Ubuntu/Mint installation via `cargo-deb`
- `.rpm` package for Fedora/RHEL/openSUSE installation via `cargo-generate-rpm`
- Both packages are automatically built and uploaded to GitHub Releases via CI
- **GNOME setup guide** - Platform-specific README in `environments/gnome/`
- **KDE Plasma setup guide** - Platform-specific README in `environments/kde/`

### Removed

- **Hyprland float script** - Removed legacy `ostt-float.sh` and `alacritty-float.toml` generation. Superseded by the new `ostt launch` command.
- **macOS Hammerspoon config** - Removed `init.lua` generation. Superseded by the new `ostt launch` command.

## 0.0.8 - 2026-03-31

### Added

- **Transcribe command** - Transcribe pre-recorded audio files without recording (`ostt transcribe <file>`). Enables use of OSTT's transcription pipeline in non-interactive workflows such as CI pipelines, GitHub Actions, or agentic scripts. Supports the same output flags as `record` and `retry` (`-c` for clipboard, `-o` for file, stdout by default). Alias: `t`.
- **Deepgram language detection** - Enable automatic language detection with `detect_language` option (default: true). Previously Deepgram defaulted to English only.
- **Deepgram language detection restriction** - Restrict detectable languages with `detect_language_codes` option. For example, `detect_language_codes = ["en", "es"]` will only detect English or Spanish.
- **AssemblyAI provider** - New transcription provider with the `universal-3-pro` model. Configurable via `[providers.assemblyai]` in `ostt.toml`.
- **Berget provider** - New transcription provider with 3 models: KB Whisper Large (Swedish optimized), NB Whisper Large (Norwegian optimized), and Whisper Large V3 (general-purpose). All data stays within Sweden.

### Fixed

- **Transcription cancel support** - Users can now press Escape, q, or Ctrl+C to cancel during transcription. Previously the UI was stuck until the API responded.
- **ErrorScreen reserved for TUI commands** - Non-TUI commands (retry, transcribe) now use standard error output instead of launching a full-screen error display.
- **macOS Hammerspoon popup** - Fixed terminal noise in Ghostty popup by launching `ostt` via `clear; exec` through a login shell.

## 0.0.7 - 2026-02-05

### Added

- **Output mode configuration** - Control transcription output destination with CLI flags:
  - Default: outputs to stdout for piping to other commands
  - `-c` flag: copy to clipboard
  - `-o <file>` flag: write to file
- **Top-level record options** - `-c` and `-o` flags now available at CLI top level without explicit `record` command (e.g., `ostt -c` equivalent to `ostt record -c`)
- **Automatic log rotation** - Log files kept for 7 most recent days; older logs automatically deleted on startup
- **Version tracking and auto-updates** - Application version tracked in config; app-managed files (float script, Alacritty config) automatically updated on version changes
- **Retry command** - Re-transcribe previous recordings without re-recording audio (`ostt retry` or `ostt retry N`)
- **Replay command** - Playback previous recordings using system audio player (`ostt replay` or `ostt replay N`)
- **Recording history** - Maintains history of 10 most recent audio recordings with automatic rotation
- **Command aliases** - Short aliases for common commands: `r` (record), `a` (auth), `h` (history), `k` (keywords), `c` (config), `rp` (replay)
- **Rich help system** - Two-tier help with `-h` (short) and `--help` (long with examples)
- **Improved error messages** - Typo suggestions and better command-not-found errors
- **Shell completions** - Generate completion scripts for bash, zsh, fish, and PowerShell (`ostt completions <shell>`)

### Changed

- **CLI framework migration** - Migrated from manual argument parsing to clap for better UX and maintainability
- `ostt record` now outputs to stdout by default (enables shell piping) instead of clipboard
- **Audio player priority on Linux** - Replay command now prefers mpv for better user experience (falls back to vlc, ffplay, paplay, xdg-open)
- **Hyprland window rules syntax** - Updated to new Hyprland window rule syntax with dynamic expressions and `match:` patterns (BREAKING CHANGE)
- **Float script defaults to clipboard** - `ostt-float.sh` now defaults to `-c` (clipboard) if no arguments provided; existing Hyprland configs continue to work
- **BREAKING CHANGE for Hyprland/macOS popup users**: Default output changed to stdout. Update your integration scripts to add `-c` flag for clipboard output. See upgrade guides:
  - [Hyprland Upgrade Guide](environments/hyprland/README.md#upgrading-from-005)
  - [macOS Upgrade Guide](environments/macOS/README.md#upgrading-from-005)

### Fixed

- Transcribed text no longer includes leading/trailing whitespace added by transcription models
- Log rotation now properly removes old log files (previously accumulated indefinitely)

## [0.0.5] - 2025-12-27

### Added

- **Frequency spectrum visualization** - Real-time FFT-based audio spectrum display (new default visualization)

### Changed

- Error message centering now accounts for multi-line text, centering entire message block vertically

### Fixed

- Segmentation fault on macOS when listing audio devices with incompatible hardware
- Vertical centering of multi-line error messages

## [0.0.4] - 2025-12-05

### Added

- **DeepInfra provider** with 2 models: Whisper Large V3 and Whisper Base
- **Groq provider** with 2 models: Whisper Large V3 and Whisper Large V3 Turbo
- Logging configuration documentation in README with `RUST_LOG` environment variable support

### Changed

- Default log level changed from `debug` to `info` for cleaner log output
- Improved logging clarity: reduced redundant messages and moved verbose logs to DEBUG level
- Keywords UI input text now uses clean white color instead of yellow
- Help text on keywords and history screens now shows "esc/q exit" instead of "q quit"
- Suppressed redundant error message when canceling auth command

## [0.0.3] - 2025-12-02

### Fixed

- Fixed `ostt-float.sh` script to correctly locate ostt binary when installed via package managers (Homebrew, AUR, shell installer)
- Fixed Hyprland hotkey binding syntax in documentation - added missing description parameter for `bind` command

### Migration Notes

**Linux users upgrading from v0.0.2:**
- Update `~/.local/bin/ostt-float` script: Manually update using the [latest version](https://github.com/kristoferlund/ostt/blob/main/environments/hyprland/ostt-float.sh)
- Update Hyprland hotkey binding in `~/.config/hypr/hyprland.conf`:
  ```diff
  - bind = SUPER, R, exec, bash ~/.local/bin/ostt-float
  + bind = SUPER, R, ostt, exec, bash ~/.local/bin/ostt-float
  ```
- Reload Hyprland config: `hyprctl reload`

## [0.0.2] - 2025-11-28

### Added

- Initial public release
- Real-time audio recording with waveform visualization
- Speech-to-text transcription via OpenAI and Deepgram providers
- Transcription history browser with clipboard integration
- Keyword management for improved transcription accuracy
- Hyprland/Omarchy floating window integration
- Cross-platform support (Linux and macOS)
- Multiple installation methods (Homebrew, AUR, shell installer)
