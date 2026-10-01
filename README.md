<p align="center">
  <img src="./ostt.png" width="160" alt="OSTT logo" />
</p>

<p align="center">
  <strong>Open source voice-to-text for Linux and macOS</strong>
</p>

<p align="center">
  <a href="https://github.com/kristoferlund/ostt/stargazers"><img src="https://img.shields.io/github/stars/kristoferlund/ostt?style=flat&color=yellow" alt="Stars"></a>
  <a href="https://github.com/kristoferlund/ostt/commits/main"><img src="https://img.shields.io/github/last-commit/kristoferlund/ostt?style=flat" alt="Last Commit"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/kristoferlund/ostt?style=flat" alt="License"></a>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#install">Install</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#processing">Processing</a> •
  <a href="https://ostt.ai">Docs</a>
</p>

---

OSTT is a terminal-native speech-to-text tool. Record from a hotkey, transcribe with local Whisper-compatible models or your chosen cloud provider, then send the result to your clipboard, a file, stdout, an AI prompt, or any shell command. Local transcription runs offline and supports GPU acceleration through Metal on macOS and CUDA or Vulkan on Linux.

OSTT is built for people who treat the terminal as a normal place for voice input to land. It does not assume one vendor, one subscription, or one app-specific workflow: use offline local models, bring your own API key for OpenAI, Deepgram, Groq, DeepInfra, AssemblyAI, Berget, ElevenLabs, or Mistral, and retry the same recording with another model when needed. Voice becomes text that can move through the same tools as everything else.

> [!TIP]
> Bind `Alt+Space` to `ostt launch -c` for a global hotkey popup. Press once to start recording, press again to stop and transcribe. Use `Alt+Ctrl+Space` with `ostt launch -c -p` for a popup with an action picker.

<video src="https://github.com/user-attachments/assets/8df8afb8-04f6-44d7-8cf3-8e40c4816d40" controls width="600">
  Your browser does not support the video tag.
</video>

## Features

- **Linux-first voice input** - Global hotkey setup for Omarchy/Hyprland, GNOME, KDE, and other Linux desktops, with macOS support too.
- **Provider choice** - Bring your own API key and switch between OpenAI, Deepgram, Groq, DeepInfra, AssemblyAI, Berget, ElevenLabs, Mistral, and local Whisper-compatible models.
- **Model-scoped params** - Tune supported provider/model request params persistently or per run with `--param key=value`.
- **Local transcription models** - Download curated local models or add custom Hugging Face/direct model files for offline transcription.
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
ostt auth           # Save cloud provider credentials
ostt model          # Choose cloud or local transcription model
ostt                # Record, transcribe, print to stdout
ostt -c             # Record, transcribe, copy to clipboard
ostt -m deepgram/nova-3 -c
ostt -m whisper/turbo --param language=sv -c
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
ostt process list             # List configured actions
```

Actions are configured in `~/.config/ostt/ostt.toml` and can run either bash commands or AI CLI tools. See [Processing Actions](https://ostt.ai/guide/processing) for examples.

## Common Commands

```bash
ostt                         # Record audio, print transcription
ostt -c                      # Record audio, copy transcription
ostt -o notes.txt            # Record audio, write transcription to file
ostt -m openai/whisper-1     # Override model for this run
ostt --param language=sv -c  # Override a transcription param for this run
ostt launch -c               # Open popup recorder
ostt transcribe file.mp3 -m deepinfra/openai/whisper-large-v3
ostt retry 2 -m groq/whisper-large-v3 -c
ostt replay                  # Play most recent recording
ostt model                   # Choose cloud or local transcription model
ostt model params whisper/turbo  # List supported params for a model
ostt history                 # Browse transcription history
ostt keyword                 # Manage transcription keywords
ostt config                  # Open config file
ostt config list-devices     # List audio input devices
ostt logs                    # View recent logs
ostt completions zsh         # Generate shell completions
ostt completions install bash    # Install completions system-wide
ostt --version               # Show version
ostt --help                  # Show help
```

Common aliases: `r` for `record`, `t` for `transcribe`, `l` for `launch`, `p` for `process`, `a` for `auth`, `h` for `history`, `k` for `keyword`, `c` for `config`, and `rp` for `replay`.

## Providers

OSTT is bring-your-own-API-key and currently supports OpenAI, Deepgram, DeepInfra, Groq, AssemblyAI, Berget, ElevenLabs, and Mistral transcription models.

Run `ostt auth` to select your provider/model and save credentials securely.

Run `ostt model` to switch between authenticated cloud models and local models. The local model screen can download curated models, activate downloaded models, delete local model files, and add custom models from Hugging Face model pages or direct `.gguf` / `ggml-*.bin` URLs.

Per-provider and per-model params are configured under `[provider.params]` and `[provider."model".params]`, or passed per run with `--param key=value`. See [Providers and Models](https://ostt.ai/reference/providers) and [Configuration](https://ostt.ai/guide/configuration) for supported params.

### Local Parakeet / Pianissimo (experimental source builds)

Native Rust Parakeet **TDT** inference is available through the optional
`parakeet` feature; no Python installation or wrapper is used. Existing release
packages are unchanged and do not yet include this backend.

```bash
cargo build --release --features parakeet
./target/release/ostt model local download parakeet/pianissimo-sv-int8
./target/release/ostt model select parakeet/pianissimo-sv-int8
./target/release/ostt daemon start  # Optional: keep the model loaded
./target/release/ostt -c
./target/release/ostt transcribe audio.wav
```

The model picker also supports downloading, selecting, and removing Pianissimo.
Downloads use pinned Klang exports and verify both the manifest and every file.
Cached model inference works offline. File transcription requires **16 kHz mono
PCM16 WAV**; recording uses that format automatically. Convert other files with
`ffmpeg -i input.mp3 -ar 16000 -ac 1 -c:a pcm_s16le audio.wav`.

Long recordings use 30-second windows with 4-second overlap and timestamp-based
whole-word ownership to avoid emitting overlap twice. Accuracy near boundaries
still needs broader evaluation. Whisper decoder params and keyword boosting are
not supported; text replacements continue to apply.

The same application includes Whisper and Parakeet: choose a model in `ostt model`,
without changing engine modes or installing an engine-specific application.
Build features below currently enable experimental capabilities for testing;
making Parakeet standard in distributed builds remains a release requirement,
including resolving the lack of a pinned ONNX Runtime prebuilt for Intel Macs.

On **Apple Silicon**, test native Metal-backed WebGPU acceleration with:

```bash
cargo build --locked --release --features parakeet-webgpu
./target/release/ostt model local download parakeet/pianissimo-sv-fp16
./target/release/ostt model select parakeet/pianissimo-sv-fp16
./target/release/ostt daemon restart
./target/release/ostt daemon status
./target/release/ostt transcribe audio.wav
```

The FP16 encoder requests WebGPU (`parakeet/webgpu` in daemon status); WebGPU uses
Metal natively, without a browser. The build places the Dawn runtime library beside
the executable; keep that companion library alongside `ostt` if moving the binary.
The decoder/joint stays on CPU. INT8 models
continue to use CPU in this same build, and Whisper retains its normal Metal
support. CPU-only builds run FP16 on CPU as well. GPU registration failures are
fatal, but registration/status alone does not prove GPU node placement. Compare
FP16 output and warm latency against the CPU build using the same audio before
relying on this experimental path. GPU execution/performance remain unverified.

Normal logs keep OSTT lifecycle messages and ONNX Runtime warnings/errors, but
omit per-kernel runtime INFO/debug chatter. `RUST_LOG` overrides these defaults;
use `RUST_LOG=info,ort=debug` when deliberately collecting runtime diagnostics,
or `RUST_LOG=debug,ort=warn` to debug OSTT without noisy runtime logs. Restart the
daemon after rebuilding or changing its logging environment.

For NVIDIA, build with `--features parakeet-cuda` and select
`parakeet/pianissimo-sv-fp16`. This requires an ONNX Runtime CUDA build matching
the installed CUDA/cuDNN runtime, including its provider shared libraries.
The encoder requests CUDA with registration errors treated as fatal; the
decoder/joint deliberately runs on CPU. **GPU execution, node placement, and
performance have not been validated on NVIDIA hardware.** This does not reuse
Whisper's CUDA/Vulkan backend. AMD and Intel acceleration are not enabled
for Parakeet in this prototype.

Custom TDT models can be added with `[c]` using a `manifest.json#VARIANT` URL.
The manifest must follow Klang's `model`, `builds`, and `files` schema, including
file sizes and SHA-256 hashes, and expose compatible encoder/decoder graphs,
`vocab.txt`, `config.json`, and `nemo128.onnx`. Resolving the URL pins its manifest
checksum. Arbitrary NeMo checkpoints, CTC models, and unrelated ONNX formats are
not supported.

Pianissimo weights are by [Klang AI AB](https://huggingface.co/KlangAI/pianissimo-sv-onnx),
licensed under **CC BY 4.0**, and fine-tuned from NVIDIA Parakeet v3. See
`specs/spikes/pianissimo/README.md` for the initial compatibility measurements.

### Berget Pianissimo (Swedish)

Select `berget/klang/pianissimo` with `ostt model`, or transcribe a file:

```bash
ostt transcribe audio.mp3 --model berget/klang/pianissimo
```

This is [KlangAI/pianissimo-sv](https://huggingface.co/KlangAI/pianissimo-sv), served through [Berget's realtime WebSocket API](https://api.berget.ai/#tag/audio/GET/v1/realtime). OSTT converts the recording to 24 kHz mono PCM using ffmpeg, sends it as one turn, and returns the final transcript.

Transcription starts **after recording stops**. OSTT does not send microphone audio or display partial transcripts during recording. See the [Pianissimo overview](https://ostt.ai/lp/klang-pianissimo-svenska) and [Berget reference](https://ostt.ai/reference/providers/berget#pianissimo-params) for setup and limitations. Available since OSTT 0.0.27.

Supported params are `language` (default `sv`) and positive `chunk_seconds` (default `3`, the server's target segment length). For example:

```toml
[berget."klang/pianissimo".params]
language = "sv"
chunk_seconds = 3
```

Pianissimo does not support keyword boosting, prompts, or temperature. OSTT does not send saved keywords to it and rejects unsupported params. Keep Whisper-specific params in model-specific sections rather than `[berget.params]` when switching between these models.

Deprecated config shapes such as `[providers]`, `[model_options]`, `[audio].sample_rate`, `[[process.actions]]`, and `provider = "local"` fail loudly. Update local model IDs from `local/<model>` to `whisper/<model>`.

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
