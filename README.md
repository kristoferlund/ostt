# ostt - Open Speech-to-Text

**ostt** is an interactive terminal-based audio recording and speech-to-text transcription tool. Record audio with real-time waveform visualization, automatically transcribe using multiple AI providers and models, and maintain a browsable history of all your transcriptions. Built with Rust for performance and minimal dependencies, ostt works seamlessly on **Linux and macOS**.

> [!TIP]
> **Omarchy and Hyprland users!** Configure ostt to run as a floating popup window to record and transcribe in any app. 

<video src="https://github.com/user-attachments/assets/488e16b1-5d9a-4ccb-9aef-1a42d1204018" controls width="600">
  Your browser does not support the video tag.
</video>

Configure any supported transcription model, authenticate with your preferred API provider, and access your transcription history anytime—all from an intuitive command-line interface.

## Features

- Real-time waveform visualization with sparkline graphs
- dBFS-based volume metering (industry standard)
- Configurable reference level for clipping detection
- Audio clipping detection with pause/resume support
- Audio compression for fast API calls
- Multiple transcription providers (OpenAI, Deepgram)
- Browsable transcription history
- Keyword management for improved accuracy
- Cross-platform: Linux and macOS support

## Installation

### macOS

**Homebrew (Recommended):**
```bash
brew tap kristoferlund/ostt
brew install ostt
```

**Shell Installer:**
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/kristoferlund/ostt/releases/latest/download/ostt-installer.sh | sh
```

### Linux

**Shell Installer (All Distributions):**
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/kristoferlund/ostt/releases/latest/download/ostt-installer.sh | sh
```

**Arch Linux (AUR):**
```bash
yay -S ostt
# or
git clone https://aur.archlinux.org/ostt.git
cd ostt
makepkg -si
```

### Dependencies

After installing ostt, install the required dependencies:

**macOS:**
```bash
brew install ffmpeg  # pbcopy is built-in
```

**Linux (Debian/Ubuntu):**
```bash
sudo apt install ffmpeg wl-clipboard  # For Wayland
# OR
sudo apt install ffmpeg xclip          # For X11
```

**Linux (Arch):**
```bash
sudo pacman -S ffmpeg wl-clipboard     # For Wayland
# OR
sudo pacman -S ffmpeg xclip            # For X11
```

**Linux (Fedora):**
```bash
sudo dnf install ffmpeg wl-clipboard   # For Wayland
# OR
sudo dnf install ffmpeg xclip          # For X11
```

### Install from Source

```bash
git clone https://github.com/kristoferlund/ostt.git
cd ostt
cargo build --profile dist
sudo cp target/dist/ostt /usr/local/bin/
```

**Build Prerequisites:**
- Rust 1.70+ ([Install Rust](https://rustup.rs/))

## Quick Start

After installation, set up authentication and start recording:

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

### Hyprland / Omarchy - Floating Window Integration

If you're using Hyprland (or Omarchy, which is built on Hyprland), ostt can work as a floating popup window for quick voice-to-text transcription from any application.

#### Setup (One-time)

On first run, ostt automatically detects Hyprland and creates the integration script at `~/.local/bin/ostt-float`.

Simply add this to your `~/.config/hypr/hyprland.conf`:

```hyprland
# ostt - Speech-to-Text hotkey
bindd = SUPER, R, exec, bash ~/.local/bin/ostt-float

# Window appearance (optional but recommended)
windowrule = float, title:ostt
windowrule = size 14% 8%, title:ostt
windowrule = move 43% 90%, title:ostt
```

Then reload your config:

```bash
hyprctl reload
```

That's it! 

#### Usage

- **Press `Super+R`**: Opens ostt in a floating window and starts recording
- **Speak your text**: Watch the waveform visualization
- **Press `Super+R` again** (or `Enter`): Transcribes and copies to clipboard
- **Paste with `Ctrl+V`**: Use the transcribed text anywhere

#### Window Customization

Adjust the window rules in your `hyprland.conf` to change size and position:

```hyprland
# Default: small window at bottom-center
windowrule = size 14% 8%, title:ostt
windowrule = move 43% 90%, title:ostt

# Example: larger centered window
windowrule = size 50% 30%, title:ostt
windowrule = move 25% 35%, title:ostt
```

The terminal appearance can be customized by editing `~/.config/ostt/alacritty-float.toml`.

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
- Show available providers (OpenAI, Deepgram)
- Let you select a model
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

# Check logs
ostt logs
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

## License

MIT
