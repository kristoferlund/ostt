# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Transcribe command** - Transcribe pre-recorded audio files without recording (`ostt transcribe <file>`). Supports the same output flags as `record` and `retry` (`-c` for clipboard, `-o` for file, stdout by default). Alias: `t`.

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

