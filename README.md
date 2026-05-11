# OSTT - Open Speech-to-Text

**OSTT** is an interactive terminal-based audio recording and speech-to-text transcription tool. Record audio with real-time waveform visualization, automatically transcribe using multiple AI providers and models, and maintain a browsable history of all your transcriptions. Built with Rust for performance and minimal dependencies, OSTT works seamlessly on **Linux and macOS**.

> [!TIP]
> **Use OSTT as a global hotkey popup!** Works on all supported platforms, both Linux and macOS. Run `ostt launch -c` from a keyboard shortcut to record and transcribe from any app. See [Platform Setup](#platform-specific-setup) below.

<video src="https://github.com/user-attachments/assets/a4124692-9d70-4d36-a4de-613b2209d81f" controls width="600">
  Your browser does not support the video tag.
</video>

## Features

- **Real-time audio visualization** - Frequency spectrum (default) or time-domain waveform, optimized for human voice recording
- **Noise gating** - Automatic suppression of background noise in spectrum mode
- **dBFS-based volume metering** (industry standard)
- **Configurable reference level** for clipping detection
- **Audio clipping detection** with pause/resume support
- **Audio compression** for fast API calls
- **Multiple transcription providers and models**
- **Browsable transcription history**
- **Keyword management** for improved accuracy
- **Cross-platform support** - Linux and macOS

## Supported Providers & Models

OSTT supports multiple AI transcription providers. Bring your own API key and choose from the following:

### OpenAI
- **gpt-4o-transcribe** - Latest model with best accuracy
- **gpt-4o-mini-transcribe** - Faster, lighter model
- **whisper-1** - Legacy Whisper model

### Deepgram
- **nova-3** - Latest generation, fastest processing
- **nova-2** - Previous generation model

### DeepInfra
- **deepinfra-whisper-large-v3** - High accuracy Whisper model
- **deepinfra-whisper-base** - Fast, lightweight model

### Groq
- **groq-whisper-large-v3** - High accuracy processing
- **groq-whisper-large-v3-turbo** - Fastest transcription speed

### AssemblyAI
- **assemblyai-universal-3-pro** - Best accuracy, latest model

### Berget

Berget is a Swedish cloud provider guaranteeing that data never leaves Sweden. All models are hosted within Swedish borders.

- **berget-whisper-kb-large** - KB Whisper Large, developed by the National Library of Sweden. Trained on 50,000+ hours of Swedish speech, reduces WER by 47% compared to OpenAI's whisper-large-v3 on Swedish.
- **berget-whisper-nb-large** - NB Whisper Large, developed by the National Library of Norway. Trained on 66,000 hours of Norwegian speech, optimized for Norwegian ASR.
- **berget-whisper-large-v3** - OpenAI Whisper Large V3, general-purpose multilingual model hosted on Berget infrastructure.

### ElevenLabs

- **elevenlabs-scribe-v2** - Scribe v2, highest accuracy with support for 99 languages
- **elevenlabs-scribe-v1** - Scribe v1, previous generation model

Configure your preferred provider and model using `ostt auth`.

## Installation

### Recommended

Install OSTT and required runtime dependencies with the website installer:

```bash
curl -fsSL https://ostt.ai/install | bash
```

The installer detects your platform, installs missing dependencies, downloads the latest OSTT release, verifies its checksum, and installs the `ostt` CLI.

### Alternative Methods

**Homebrew:**

```bash
brew install kristoferlund/ostt/ostt
```

**Arch Linux (AUR):**

```bash
paru -S ostt
# or
yay -S ostt
```

`pacman` is the default package manager on Arch Linux, but AUR packages require an AUR helper such as `paru` or `yay` unless you build them manually.

### Dependencies

The recommended installer handles runtime dependencies for supported platforms. If you use an alternative install method, make sure `ffmpeg` is available. Linux clipboard output also requires `wl-clipboard` on Wayland or `xclip` on X11.

**Optional (Recommended for better audio playback):**
```bash
mpv  # Recommended for best audio playback experience with ostt replay
```

> **Note on Audio Playback:** For the best experience when replaying recordings with `ostt replay`, we recommend installing `mpv`. It will be used as the primary audio player if available. Fallbacks include `vlc`, `ffplay`, and `paplay`. If none are installed, the system default application will be used.

## Quick Start

After installation, set up authentication and start recording:

**Authentication:** OSTT is a bring-your-own-API-key application. Authenticate once with your preferred provider, then freely switch between available models.

```bash
# Configure your transcription provider
ostt auth

# Start recording (press Enter to transcribe, Esc to cancel)
ostt record

# Or just run ostt (defaults to recording)
ostt
```

The app will create a default configuration file on first run at `~/.config/ostt/ostt.toml`.

## Platform-Specific Setup

For the best experience, configure OSTT to run as a floating popup window tied to a global hotkey. This allows you to:

1. Press a hotkey from any application
2. Record your speech in a popup window
3. Have it automatically transcribed
4. Paste the result directly into your current app

The `ostt launch` command handles terminal detection, window configuration, and toggle behavior (press the hotkey again to finish recording). Bind `ostt launch -c` to a system keyboard shortcut on any platform.

Platform-specific setup instructions:

- **[macOS Setup](environments/macOS/README.md)** - Uses Shortcuts.app (built-in, no third-party tools)
- **[Hyprland / Omarchy Setup](environments/hyprland/README.md)** - Tiling window manager integration
- **[GNOME Setup](environments/gnome/README.md)** - Ubuntu, Fedora, and other GNOME desktops
- **[KDE Plasma Setup](environments/kde/README.md)** - Kubuntu, Fedora KDE, openSUSE, and other KDE desktops

### Other Platforms

OSTT works on all Linux distributions and macOS without additional setup. Simply use `ostt` or `ostt record` from your terminal. For popup integration on other Linux desktops (XFCE, Sway, Cinnamon), bind `ostt launch -c` to a hotkey in your desktop environment's keyboard shortcut settings.

## Commands

```bash
ostt                 # Record audio with real-time visualization (default)
ostt record          # Record audio with real-time visualization
                     # Output to stdout by default
ostt -c              # Record and copy to clipboard (shorthand)
ostt record -c       # Record and copy to clipboard (explicit)
ostt -o file         # Record and write to file (shorthand)
ostt record -o file  # Record and write to file (explicit)
ostt launch -c       # Launch popup terminal, record, copy to clipboard
ostt launch -c -p clean  # Launch popup, record, process with "clean", copy
ostt transcribe file # Transcribe a pre-recorded audio file
ostt transcribe f -c # Transcribe and copy to clipboard
ostt transcribe f -o out.txt # Transcribe and write to file
ostt retry [N]       # Re-transcribe recording #N (1=most recent)
ostt retry -c        # Re-transcribe and copy to clipboard
ostt replay [N]      # Play back recording #N
ostt auth            # Configure transcription provider and API key
ostt history         # Browse transcription history
ostt keywords        # Manage keywords for improved accuracy
ostt config          # Open configuration file in editor
ostt list-devices    # List available audio input devices
ostt logs            # View recent application logs
ostt version         # Show version information
ostt help            # Show all commands
ostt -h              # Quick help
ostt --help          # Detailed help with examples
```

**Command Aliases:** Most commands have short aliases for faster typing: `r` (record), `t` (transcribe), `l` (launch), `a` (auth), `h` (history), `k` (keywords), `c` (config), `rp` (replay).

```bash
ostt r -c            # Same as: ostt record -c
ostt l -c            # Same as: ostt launch -c
ostt a               # Same as: ostt auth
```

**Transcribe:** The `transcribe` command enables use of OSTT's transcription pipeline for pre-recorded audio files, without interactive recording. This is useful for non-interactive workflows such as CI pipelines, GitHub Actions, or agentic scripts where you have an existing audio file and want to leverage OSTT's multi-provider transcription infrastructure.

```bash
ostt transcribe recording.ogg              # Transcribe to stdout
ostt transcribe voice-memo.mp3 -c          # Transcribe and copy to clipboard
ostt transcribe meeting.wav -o transcript.txt  # Transcribe and write to file
ostt transcribe audio.ogg | grep keyword   # Pipe to other commands
```

**Record Options:** The `-c` and `-o` flags can be used without explicitly saying `record` since it's the default command:

```bash
ostt -c              # Same as: ostt record -c
ostt -o file.txt     # Same as: ostt record -o file.txt
```

## Shell Completions

OSTT can generate completion scripts for your shell to enable tab completion of commands and options.

**Bash:**
```bash
ostt completions bash > ostt.bash
sudo cp ostt.bash /etc/bash_completion.d/
```

**Zsh:**
```bash
ostt completions zsh > _ostt
# Copy to your zsh completions directory (location varies by system)
sudo cp _ostt /usr/local/share/zsh/site-functions/
```

**Fish:**
```bash
ostt completions fish > ostt.fish
cp ostt.fish ~/.config/fish/completions/
```

**PowerShell:**
```powershell
ostt completions powershell > ostt.ps1
# Add to your PowerShell profile
```

After installation, restart your shell or source the completion file to enable completions.

## Configuration

OSTT uses a TOML configuration file at `~/.config/ostt/ostt.toml`.

### Audio Device Configuration

List available devices:

```bash
ostt list-devices
```

Example output:
```
Available audio input devices:

  ID: 0
    Name: default [DEFAULT]
    Config: (44100Hz, 2 channels)

  ID: 2
    Name: USB Microphone
    Config: (48000Hz, 1 channels)
```

Edit `~/.config/ostt/ostt.toml`:

```toml
[audio]
# Use device by ID, name, or "default"
device = "2"                    # or "USB Microphone" or "default"
sample_rate = 16000             # 16kHz recommended for speech
peak_volume_threshold = 90      # Warning threshold (0-100%)
reference_level_db = -20        # dBFS reference for 100% meter
output_format = "mp3 -ab 16k -ar 12000"  # Compressed audio format
visualization = "spectrum"      # "spectrum" (default) or "waveform"
```

**Visualization Types:**

- `spectrum` (default) - Shows frequency spectrum with energy distribution across frequencies optimized for human voice (100-1500 Hz range).
- `waveform` - Shows time-domain waveform with amplitude over time. Classic oscilloscope-style display showing raw audio envelope.

### Popup Configuration

`ostt launch` opens OSTT in a popup terminal and auto-detects a terminal emulator in this order: Ghostty, kitty, Alacritty, foot, Konsole, GNOME Terminal, then Xfce Terminal.

Configure popup behavior in `~/.config/ostt/ostt.toml`:

```toml
[popup]
# Optional. If unset, OSTT auto-detects a supported terminal.
terminal = "ghostty"

# Window position in pixels. Some compositors ignore this.
x = 630
y = 790

# Window size in terminal columns and rows.
width = 90
height = 15

# Font size for the popup terminal.
font_size = 6

# Hide window decorations when supported by the terminal/compositor.
borderless = true
```

Platform-specific setup guides explain any compositor-specific behavior, such as Omarchy/Hyprland window rules or GNOME Wayland placement limitations.

### Transcription Setup

Configure your AI provider:

```bash
ostt auth
```

This will:
- Show available providers and models
- Let you select your preferred model
- Prompt for your API key
- Save everything securely

**Security Note:** API keys are stored separately in `~/.local/share/ostt/credentials` with restricted permissions (0600).

### Example Configuration

```toml
[audio]
device = "default"
sample_rate = 16000
peak_volume_threshold = 90
reference_level_db = -20
output_format = "mp3 -ab 16k -ar 12000"
visualization = "spectrum"  # "spectrum" for frequency display, "waveform" for amplitude display

[providers.deepgram]
punctuate = true
smart_format = false
filler_words = false
detect_language = true  # Automatic language detection (default: true)
# detect_language_codes = ["en", "es"]  # Restrict to specific languages only

[providers.assemblyai]
format_text = true        # Punctuation, casing, and numeral formatting
disfluencies = false      # Include filler words (uh, um)
filter_profanity = false  # Filter profanity from transcript
language_detection = true  # Automatic language detection

[providers.elevenlabs]
# Optional ISO-639-1 or ISO-639-3 language code, e.g. "en" or "eng".
# Leave unset to auto-detect the spoken language.
# language_code = "eng"
```

For detailed configuration options, see the config file comments or run `ostt config` to edit.

## Recording Controls

| Key | Action |
|-----|--------|
| `Enter` | Stop recording and transcribe |
| `Space` | Pause/resume recording |
| `Esc`, `q`, `Ctrl+C` | Cancel without saving |

**Display Elements:**

- **Visualization**: Real-time audio display (spectrum or waveform, configurable)
  - **Spectrum mode**: Frequency distribution across the voice range
  - **Waveform mode**: Amplitude envelope over time
- **Vol %**: Current volume level
- **Peak %**: Maximum volume in last 3 seconds
- **Red indicator**: Clipping warning

## File Locations

```
~/.config/ostt/
└── ostt.toml              # Main configuration

~/.local/share/ostt/
└── credentials            # API keys (0600 permissions)

~/.local/state/ostt/
└── ostt.log.*             # Daily-rotated logs (kept for 7 days, auto-cleanup on startup)
```

## Troubleshooting

### Logging

OSTT logs all activity to `~/.local/state/ostt/ostt.log.*` with daily rotation and automatic cleanup. Log files are kept for the 7 most recent days and older logs are automatically deleted on startup. By default, logs are set to `info` level.

**View recent logs:**
```bash
ostt logs
```

**Enable debug logging for detailed troubleshooting:**
```bash
RUST_LOG=debug ostt record
```

**Available log levels:** `error`, `warn`, `info` (default), `debug`, `trace`

### No Audio Input Detected

```bash
# List available devices
ostt list-devices

# Update config with correct device
ostt config
```

### Volume Meter Not Reaching 100%

The reference level may be set too high/low for your audio card. Run `ostt`, maximize your microphone gain, note the peak dBFS value, and update `reference_level_db` in your config.

### Transcription Not Working

```bash
# Verify authentication
ostt auth

# Check logs with debug output
RUST_LOG=debug ostt record
```

### Hyprland Window Not Appearing

```bash
# Test launch directly
ostt launch -c

# Verify Hyprland config loaded
hyprctl reload
```

For more troubleshooting, see `ostt logs` or check `~/.local/state/ostt/ostt.log.*`.

## Development

### Building from Source

```bash
git clone https://github.com/kristoferlund/ostt.git
cd ostt

# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run directly
cargo run
```

### Project Structure

```
ostt/
├── src/
│   ├── commands/         # Command handlers
│   ├── config/           # Configuration management
│   ├── recording/        # Audio capture and UI
│   ├── transcription/    # API integrations
│   ├── history/          # History storage and UI
│   └── ui/               # Shared UI components
├── environments/         # Platform-specific integrations
└── Cargo.toml
```

### Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## Contributors

<!-- readme: collaborators,contributors -start -->
<table>
	<tbody>
		<tr>
            <td align="center">
                <a href="https://github.com/kristoferlund">
                    <img src="https://avatars.githubusercontent.com/u/9698363?v=4" width="100;" alt="kristoferlund"/>
                    <br />
                    <sub><b>Kristofer</b></sub>
                </a>
            </td>
            <td align="center">
                <a href="https://github.com/claw-fmckl">
                    <img src="https://avatars.githubusercontent.com/u/260451250?v=4" width="100;" alt="claw-fmckl"/>
                    <br />
                    <sub><b>Kristofer Claw</b></sub>
                </a>
            </td>
            <td align="center">
                <a href="https://github.com/andrepadez">
                    <img src="https://avatars.githubusercontent.com/u/1013997?v=4" width="100;" alt="andrepadez"/>
                    <br />
                    <sub><b>Pastilhas</b></sub>
                </a>
            </td>
            <td align="center">
                <a href="https://github.com/kristofernoaccess">
                    <img src="https://avatars.githubusercontent.com/u/46928173?v=4" width="100;" alt="kristofernoaccess"/>
                    <br />
                    <sub><b>kristofernoaccess</b></sub>
                </a>
            </td>
            <td align="center">
                <a href="https://github.com/axo-bot">
                    <img src="https://avatars.githubusercontent.com/u/142847116?v=4" width="100;" alt="axo-bot"/>
                    <br />
                    <sub><b>axo bot</b></sub>
                </a>
            </td>
		</tr>
	<tbody>
</table>
<!-- readme: collaborators,contributors -end -->

## License

MIT
