# ostt - Open Speech-to-Text

**ostt** is an interactive terminal-based audio recording and speech-to-text transcription tool. Record audio with real-time waveform visualization, automatically transcribe using multiple AI providers and models, and maintain a browsable history of all your transcriptions. Built with Rust for performance and minimal dependencies, ostt works seamlessly on **Linux and macOS**.

> [!TIP]
> **Omarchy and Hyprland users!** Configure ostt to run as a floating popup window to record and transcribe in any app. 

<video src="https://github.com/user-attachments/assets/488e16b1-5d9a-4ccb-9aef-1a42d1204018" controls width="600">
  Your browser does not support the video tag.
</video>

## Features

- Real-time waveform visualization with sparkline graphs
- dBFS-based volume metering (industry standard)
- Configurable reference level for clipping detection
- Audio clipping detection with pause/resume support
- Audio compression for fast API calls
- Multiple transcription providers and models
- Browsable transcription history
- Keyword management for improved accuracy
- Cross-platform: Linux and macOS support

## Supported Providers & Models

ostt supports multiple AI transcription providers. Bring your own API key and choose from the following:

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

Configure your preferred provider and model using `ostt auth`.

## Installation

### macOS

**Homebrew (Recommended):**
```bash
brew install kristoferlund/ostt/ostt
```

**Shell Installer:**
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/kristoferlund/ostt/releases/latest/download/ostt-installer.sh | sh
```

### Linux

**Arch Linux (AUR):**
```bash
yay -S ostt
```

**Shell Installer (All Distributions):**
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/kristoferlund/ostt/releases/latest/download/ostt-installer.sh | sh
```

### Dependencies

Dependencies need only to be installed manually if you used the shell installer. `yay` and `brew` installs the dependencies automatically.

**macOS:**
```bash
ffmpeg
```

**Linux:**
```bash
ffmpeg wl-clipboard  # For Wayland
# OR
ffmpeg xclip         # For X11
```

## Quick Start

After installation, set up authentication and start recording:

**Authentication:** ostt is a bring-your-own-API-key application. Authenticate once with your preferred provider, then freely switch between available models.

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

For the best experience, configure ostt to run as a floating popup window tied to a global hotkey. This allows you to:

1. Press a hotkey from any application
2. Record your speech in a popup window
3. Have it automatically transcribed
4. Paste the result directly into your current app

Platform-specific setup instructions:

- **[Hyprland / Omarchy Setup](environments/hyprland/README.md)** - Tiling window manager integration (recommended)
- **[macOS Setup](environments/macOS/README.md)** - Hammerspoon-based popup configuration

### Other Platforms

ostt works on all Linux distributions and macOS without additional setup. Simply use `ostt` or `ostt record` from your terminal.

## Commands

```bash
ostt record          # Record audio with real-time visualization
ostt auth            # Configure transcription provider and API key
ostt history         # Browse transcription history
ostt keywords        # Manage keywords for improved accuracy
ostt config          # Open configuration file in editor
ostt list-devices    # List available audio input devices
ostt logs            # View recent application logs
ostt version         # Show version information
ostt help            # Show all commands
```

## Configuration

ostt uses a TOML configuration file at `~/.config/ostt/ostt.toml`.

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
```

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

[providers.deepgram]
punctuate = true
smart_format = false
filler_words = false
```

For detailed configuration options, see the config file comments or run `ostt config` to edit.

## Usage

### Recording

```bash
ostt record
```

**Keyboard Controls:**

| Key | Action |
|-----|--------|
| `Enter` | Stop recording and transcribe |
| `Space` | Pause/resume recording |
| `Esc`, `q`, `Ctrl+C` | Cancel without saving |

**Display Elements:**

- **Waveform**: Real-time audio visualization
- **Vol %**: Current volume level
- **Peak %**: Maximum volume in last 3 seconds
- **Red indicator**: Clipping warning

### History

Browse your transcription history:

```bash
ostt history
```

Use arrow keys to navigate, Enter to copy selected transcription to clipboard, and Esc to exit.

### Keywords

Manage keywords for improved transcription accuracy:

```bash
ostt keywords
```

Add technical terms, names, or domain-specific vocabulary to help the AI transcribe more accurately.

## File Locations

```
~/.config/ostt/
├── ostt.toml              # Main configuration
└── hyprland/              # Hyprland integration (if set up)
    ├── ostt-float.sh
    └── alacritty-float.toml

~/.local/share/ostt/
└── credentials            # API keys (0600 permissions)

~/.local/state/ostt/
└── ostt.log.*             # Daily-rotated logs
```

## Troubleshooting

### Logging

ostt logs all activity to `~/.local/state/ostt/ostt.log.*` with daily rotation. By default, logs are set to `info` level.

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

The reference level may be set too high/low for your audio card. Run ostt, maximize your microphone gain, note the peak dBFS value, and update `reference_level_db` in your config.

### Transcription Not Working

```bash
# Verify authentication
ostt auth

# Check logs with debug output
RUST_LOG=debug ostt record
```

### Hyprland Window Not Appearing

```bash
# Test the script directly
bash ~/.local/bin/ostt-float

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
                <a href="https://github.com/andrepadez">
                    <img src="https://avatars.githubusercontent.com/u/1013997?v=4" width="100;" alt="andrepadez"/>
                    <br />
                    <sub><b>Pastilhas</b></sub>
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
