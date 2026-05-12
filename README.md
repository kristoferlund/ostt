# OSTT - Open Speech-to-Text

**Open source voice-to-text for Linux. And macOS.**

OSTT is a terminal-native speech-to-text tool. Record from a hotkey, transcribe with your chosen provider, then send the result to your clipboard, a file, stdout, an AI prompt, or any shell command. It does not assume one vendor, one subscription, or one app-specific workflow: bring your own API key and choose from OpenAI, Deepgram, Groq, DeepInfra, AssemblyAI, Berget, and ElevenLabs.

OSTT is built for people who treat the terminal as a normal place for voice input to land. You can print to stdout, copy to the clipboard, write to files, retry the same recording with another model, transcribe existing audio, and post-process text with AI prompts or shell commands. Voice becomes text that can move through the same tools as everything else.

> [!TIP]
> Bind `Alt+Space` to `ostt launch -c` for a global hotkey popup. Press once to start recording, press again to stop and transcribe. Use `Alt+Ctrl+Space` with `ostt launch -c -p` for a popup with an action picker.

<video src="https://github.com/user-attachments/assets/a4124692-9d70-4d36-a4de-613b2209d81f" controls width="600">
  Your browser does not support the video tag.
</video>

## Features

- **Linux-first voice input** - Global hotkey setup for Omarchy/Hyprland, GNOME, KDE, and other Linux desktops, with macOS support too.
- **Provider choice** - Bring your own API key and switch between OpenAI, Deepgram, Groq, DeepInfra, AssemblyAI, Berget, and ElevenLabs.
- **Terminal-native workflow** - Use stdout, clipboard, files, aliases, shell completions, logs, and pipes.
- **Scriptable post-processing** - Transform transcripts with AI prompts or bash commands using `ostt -p` and `ostt process`.
- **Retry without re-recording** - Save recordings locally, then re-transcribe them with a different provider or model.
- **File transcription and replay** - Transcribe existing audio files and replay saved recordings from history.
- **Keywords and custom vocabulary** - Improve recognition for names, technical terms, and project-specific language.
- **Open source, no subscription** - Public code, local configuration, and no vendor lock-in beyond the providers you choose.

## Documentation

Full documentation is available at **https://ostt.ai**.

Start here:

- [Getting Started](https://ostt.ai/guide/getting-started)
- [Installation](https://ostt.ai/guide/installation)
- [Commands](https://ostt.ai/guide/commands)
- [Processing Actions](https://ostt.ai/guide/processing)
- [Configuration](https://ostt.ai/guide/configuration)
- [Why OSTT?](https://ostt.ai/guide/why-ostt)
- [Platform Setup](https://ostt.ai/guide/platforms)
- [Providers and Models](https://ostt.ai/reference/providers)

## Install

```bash
curl -fsSL https://ostt.ai/install | bash
```

The installer detects your platform, installs supported runtime dependencies, downloads the latest release, verifies its checksum, and installs the `ostt` CLI.

If you prefer platform package managers, see the docs for Homebrew, AUR, `.deb`, and `.rpm` options.

## Quick Start

```bash
ostt auth           # Choose provider/model and save API key
ostt                # Record, transcribe, print to stdout
ostt -c             # Record, transcribe, copy to clipboard
ostt launch -c      # Popup workflow for global hotkeys
```

By default, press `Enter` to stop and transcribe, `Space` to pause/resume, and `Esc`, `q`, or `Ctrl+C` to cancel.

## Processing

Processing actions transform transcriptions after recording or from history.

```bash
ostt -p clean -c              # Record, transcribe, clean, copy
ostt launch -c -p clean       # Popup hotkey workflow with processing
ostt process                  # Process most recent history item, show picker
ostt process clean            # Process most recent history item with clean action
ostt process 3 clean -c       # Process history item #3 with clean action
ostt process --list           # List configured actions
```

Actions are configured in `~/.config/ostt/ostt.toml` and can run either bash commands or AI CLI tools. See [Processing Actions](https://ostt.ai/guide/processing) for examples.

## Common Commands

```bash
ostt                         # Record audio, print transcription
ostt -c                      # Record audio, copy transcription
ostt -o notes.txt            # Record audio, write transcription to file
ostt launch -c               # Open popup recorder
ostt transcribe file.mp3     # Transcribe existing audio
ostt retry 2 -c              # Re-transcribe recording #2 and copy
ostt replay                  # Play most recent recording
ostt history                 # Browse transcription history
ostt keywords                # Manage transcription keywords
ostt config                  # Open config file
ostt list-devices            # List audio input devices
ostt logs                    # View recent logs
ostt completions zsh         # Generate shell completions
ostt --version               # Show version
ostt --help                  # Show help
```

Common aliases: `r` for `record`, `t` for `transcribe`, `l` for `launch`, `p` for `process`, `a` for `auth`, `h` for `history`, `k` for `keywords`, `c` for `config`, and `rp` for `replay`.

## Providers

OSTT is bring-your-own-API-key and currently supports OpenAI, Deepgram, DeepInfra, Groq, AssemblyAI, Berget, and ElevenLabs transcription models.

Run `ostt auth` to select your provider/model and save credentials securely.

## Platform Setup

Suggested default keybindings:

| Hotkey | Command | Action |
| --- | --- | --- |
| `Alt+Space` | `ostt launch -c` | Popup recorder, clipboard output |
| `Alt+Ctrl+Space` | `ostt launch -c -p` | Popup with action picker |

Platform-specific setup notes are available in the docs:

- [macOS](https://ostt.ai/guide/platforms/macos)
- [Omarchy / Hyprland](https://ostt.ai/guide/platforms/hyprland)
- [GNOME](https://ostt.ai/guide/platforms/gnome)
- [KDE Plasma](https://ostt.ai/guide/platforms/kde)

## Development

```bash
git clone https://github.com/kristoferlund/ostt.git
cd ostt
cargo build
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features
```

Release builds use the dist profile:

```bash
cargo build --profile dist --locked
```

### Contributing

Contributions are welcome. Please open an issue or submit a pull request.

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
