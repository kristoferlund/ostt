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
- Multiple transcription providers (OpenAI, Deepgram, Local Parakeet)
- **Offline transcription** with local Parakeet models (no API required)
- Browsable transcription history
- Keyword management for improved accuracy
- Cross-platform: Linux and macOS support

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

**Authentication:** ostt supports both cloud providers (bring-your-own-API-key) and local offline models. For cloud providers, you need an API key from either OpenAI or Deepgram. For local models, no API key is required.

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
- Show available providers (OpenAI, Deepgram, Parakeet Local)
- Let you select a model
- Prompt for your API key (if using cloud provider)
- Save everything securely

**Security Note:** API keys are stored separately in `~/.local/share/ostt/credentials` with restricted permissions (0600).

### Local Offline Transcription

ostt supports **offline transcription** using NVIDIA's Parakeet TDT models via sherpa-onnx! This means:
- ✅ **No API costs** - completely free after model download
- ✅ **Full privacy** - audio never leaves your machine
- ✅ **No internet required** - works 100% offline
- ✅ **CPU inference** - no GPU needed (though GPU is supported)
- ✅ **Multilingual support** - 25 European languages with v3

#### Available Local Models

| Model | Languages | Size | Transcription Speed | Best For |
|-------|-----------|------|---------------------|----------|
| **Parakeet TDT v2** | English only | ~640MB | ~3-8 sec for 10s audio | English-only users, faster inference |
| **Parakeet TDT v3** | 25 European languages + auto-detection | ~660MB | ~5-15 sec for 10s audio | Multilingual users, supports auto language detection |

**Supported Languages (v3):** English, German, French, Spanish, Italian, Portuguese, Polish, Dutch, Czech, Romanian, Hungarian, Swedish, Danish, Finnish, Norwegian, Slovak, Croatian, Bulgarian, Slovenian, Lithuanian, Latvian, Estonian, Irish, Maltese, Greek

#### Setting Up Local Models

**1. Select Parakeet as your provider:**
```bash
ostt auth
# Select "Parakeet (Local)" from the provider list
# Select your preferred model (v2 for English, v3 for multilingual)
# No API key needed - just press through the prompts
```

**2. Download the model files:**

**For v3 (Multilingual):**
```bash
# Create model directory
mkdir -p ~/.config/ostt/models/parakeet-tdt-v3
cd ~/.config/ostt/models/parakeet-tdt-v3

# Download from sherpa-onnx (pre-exported, ready to use)
wget https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2

# Extract files to current directory (--strip-components=1 is important!)
tar -xjf sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2 --strip-components=1

# Clean up
rm sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2

# Verify files exist
ls -lh  # Should show: encoder.int8.onnx, decoder.int8.onnx, joiner.int8.onnx, tokens.txt
```

**For v2 (English-only, faster):**
```bash
# Create model directory
mkdir -p ~/.config/ostt/models/parakeet-tdt-v2
cd ~/.config/ostt/models/parakeet-tdt-v2

# Download from sherpa-onnx
wget https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8.tar.bz2

# Extract files to current directory (--strip-components=1 is important!)
tar -xjf sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8.tar.bz2 --strip-components=1

# Clean up
rm sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8.tar.bz2

# Verify files exist
ls -lh  # Should show: encoder.int8.onnx, decoder.int8.onnx, joiner.int8.onnx, tokens.txt
```

**3. Start recording with local transcription:**
```bash
ostt record
# Press Enter - transcription happens locally with no API call!
```

#### Model Files Structure

The local model directory must contain these files at the top level:
```
~/.config/ostt/models/parakeet-tdt-v3/
├── encoder.int8.onnx      # Main encoder model (~652MB for v3)
├── decoder.int8.onnx      # Decoder model (~12MB)
├── joiner.int8.onnx       # Joiner model (~6MB)
└── tokens.txt             # Vocabulary/tokens (~94KB)
```

**Important:** The files must be directly in the `parakeet-tdt-v3` directory, not in a subdirectory! The `--strip-components=1` flag in the tar command ensures this.

**Note:** The `.int8.onnx` files are quantized models (4x smaller, faster) while `.onnx` files are full precision (slightly better quality, larger). Both are supported.

#### Performance Tips

**Speed up inference:**
1. **Use v2 for English** - It's ~3x faster than v3
2. **Build in release mode** - `cargo build --release` then use `./target/release/ostt`
3. **More CPU cores help** - The model automatically uses all available threads
4. **First transcription is slower** - Model loading takes 1-3 seconds

**Typical performance (on modern CPU):**
- First transcription: 6-18 seconds (includes model loading)
- Subsequent: 3-12 seconds (model stays loaded)
- 10 seconds of audio typically takes 5-10 seconds to transcribe on CPU

**Memory usage:**
- v2: ~1.5GB RAM
- v3: ~2GB RAM

#### Switching Between Cloud and Local Models

You can easily switch between cloud and local models:

```bash
ostt auth
# Select different provider/model
# Immediately takes effect on next recording
```

Local models save as WAV (no compression), cloud models use MP3 (faster upload). This happens automatically!

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
├── keywords.txt           # Custom keywords for better accuracy
├── models/                # Local model storage
│   ├── parakeet-tdt-v2/  # English-only model (if downloaded)
│   └── parakeet-tdt-v3/  # Multilingual model (if downloaded)
└── hyprland/              # Hyprland integration (if set up)
    ├── ostt-float.sh
    └── alacritty-float.toml

~/.local/share/ostt/
├── credentials            # API keys (0600 permissions)
├── model                  # Currently selected model ID
└── ostt.db                # Transcription history database

~/.local/state/ostt/
└── ostt.log.*             # Daily-rotated logs
```

## Troubleshooting

### Local Model Issues

#### "Model directory not found"

The model files aren't in the expected location:

```bash
# Check what models exist
ls -la ~/.config/ostt/models/

# For v3, you should see:
ls -la ~/.config/ostt/models/parakeet-tdt-v3/
# Should contain: encoder.int8.onnx, decoder.int8.onnx, joiner.int8.onnx, tokens.txt

# If missing, download following instructions in "Setting Up Local Models" section
```

**Common mistake:** Files are in a subdirectory instead of at the top level. Make sure to use `--strip-components=1` when extracting!

#### "Failed to read audio file" or "sample rate must be 16000"

This has been fixed in the latest version. If you see this:
1. Make sure you selected the Parakeet model via `ostt auth`
2. Rebuild: `cargo build`
3. The app now automatically saves at 16kHz for local models

#### Local transcription is very slow

Performance tips:
1. **Use v2 for English** - Much faster than v3
2. **Build in release mode**: `cargo build --release && ./target/release/ostt`
3. **First run is always slower** - Model loading takes time
4. **Check CPU usage** - Should use multiple cores during transcription

Expected speed on modern CPU:
- v2: 3-8 seconds for 10s of audio
- v3: 5-15 seconds for 10s of audio

#### "Failed to create recognizer" or ONNX errors

File corruption or incomplete download:

```bash
cd ~/.config/ostt/models/parakeet-tdt-v3  # or v2

# Check file sizes
ls -lh

# Re-download if sizes don't match expected:
# encoder.int8.onnx: ~652MB (v3) or ~520MB (v2)
# decoder.int8.onnx: ~12MB (v3) or ~7MB (v2)
# joiner.int8.onnx: ~6MB (v3) or ~2MB (v2)
# tokens.txt: ~94KB (v3) or ~9KB (v2)
```

#### Transcription accuracy is poor

1. Try the non-quantized models (larger but more accurate)
2. For v3: Ensure you're speaking one of the 25 supported European languages
3. Add keywords: `ostt keywords` to improve accuracy for specific terms
4. Check audio quality - local models need clear audio

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
