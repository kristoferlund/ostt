# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Transcribed text no longer includes leading/trailing whitespace added by transcription models
- Code quality improvements: fixed format string linting issues

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

