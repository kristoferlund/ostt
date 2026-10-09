# Pianissimo native Rust compatibility spike

Standalone crate: does not change OSTT's production dependencies or providers.
No Python runtime or wrapper. Uses `parakeet-rs` pinned in `Cargo.toml` and
the official Klang INT8 ONNX export at revision
`63730c6021234f26b9bbae9a07a04fec39e7a52e`.
The downloader verifies files against that revision's SHA-256 manifest.

```sh
bash specs/spikes/pianissimo/download.sh /tmp/opencode/pianissimo-int8
cargo build --release --manifest-path specs/spikes/pianissimo/Cargo.toml
specs/spikes/pianissimo/target/release/pianissimo-spike /tmp/opencode/pianissimo-int8 audio.wav
```

Input must be 16 kHz mono PCM16 WAV. Convert other formats with:

```sh
ffmpeg -i input.ogg -ar 16000 -ac 1 -c:a pcm_s16le audio.wav
```

The harness reports model-load time and three sequential inference times, rejects
empty transcripts, and checks that repeated identical inputs produce identical
text. It deliberately uses CPU; it does not claim GPU acceleration.

## Public speech fixture

[Google FLEURS](https://huggingface.co/datasets/google/fleurs) Swedish test
utterance `4184612854874415460` (CC BY 4.0), converted to 16 kHz mono PCM16 WAV.

SHA-256: `82e5cefc3793e1352ae1f4cebd3848ab57e96f91799192b7d1b7d027151c5ff2`.

Reference: “Hongkongön ger Hongkongs territorium dess namn och är den plats som
många turister betraktar som huvudfokus.”

## Limits

- Host hardware: AMD Radeon 680M, no NVIDIA GPU. CUDA and VRAM measurements
  require another machine. No GPU runtime installation is performed.
- `parakeet-rs` performs its own feature extraction; downloading `nemo128.onnx`
  does not make it use Klang's ONNX preprocessor. Output agreement needs testing.
- A short fixture is not an accuracy benchmark or a long-file boundary test.
- The upstream GPU configuration permits CPU fallback. Production integration
  must make requested GPU registration fail loudly and verify node placement.
- Model weights: Klang AI AB, CC BY 4.0; see
  https://huggingface.co/KlangAI/pianissimo-sv-onnx .

## Measured results (2026-10-01)

AMD Ryzen 7 6800H, Linux x86_64, release build, CPU execution with four
intra-op threads. All five downloaded files passed SHA-256 verification.
`parakeet-rs` required no patches or model-file renaming.

| Measurement | Seconds |
| --- | ---: |
| Audio duration | 7.560 |
| Model load | 2.713 |
| First inference | 0.534 |
| Second inference | 0.519 |
| Third inference | 0.599 |

All three transcripts matched the fixture reference exactly, including punctuation.
This is approximately 12.6–14.6 times realtime on this one short recording;
it does not establish corpus-level accuracy or general throughput.

Offline inference also succeeded in a separate network namespace:

```sh
unshare --user --map-root-user --net \
  specs/spikes/pianissimo/target/release/pianissimo-spike \
  /tmp/opencode/pianissimo-int8 audio.wav
```

Release build, Clippy with `-D warnings`, Rust formatting, and shell syntax checks
passed. GPU execution, GPU memory, corpus accuracy, reference-runtime comparison,
long-file segmentation/boundary accuracy, and OSTT integration remain untested.
Recommendation: proceed to a CPU/CUDA integration prototype, but do not advertise
GPU support until tested on suitable hardware with actual provider placement verified.

## OSTT integration prototype

OSTT now has an optional native `parakeet` backend, with model-picker/CLI bundle
downloads, selection, deletion, in-process inference, and daemon reuse. The
`parakeet-cuda` feature requires successful CUDA encoder registration; the
decoder/joint runs on CPU. Daemon identity includes the backend so CUDA requests
cannot silently reuse a CPU Parakeet daemon.

Reproduce the Linux CLI checks after building with `--features parakeet`:

```sh
bash specs/spikes/pianissimo/ostt-smoke.sh \
  target/release/ostt /tmp/opencode/pianissimo-int8 \
  path/to/swedish-fleurs-hongkong.wav
```

The script uses fresh isolated XDG directories and leaves artifacts under
`/tmp/opencode` for inspection. It copies the previously downloaded files and
lets the native downloader verify them and publish its completion marker; this
tests cached/resumed installation, not another complete network transfer.

Verified on 2026-10-01:

- 246 default-build tests and 247 Parakeet-build tests passed, none ignored.
- Production Clippy (`--lib --bins -- -D warnings`), formatting, and CUDA
  type-checking passed. Strict all-target Clippy also found pre-existing
  `await_holding_lock` and `useless_vec` warnings in tests; these were left untouched.
- The actual OSTT CLI matched the Swedish reference in-process and via the daemon.
- A 60.48-second fixture made from eight repetitions matched all eight sentences
  exactly across the 30-second overlapping window boundaries.
- In-process transcription worked with networking isolated using `unshare`.
- Whisper-only params were rejected; model removal deleted the bundle and cleared
  the active selection. Test daemons were stopped.

NVIDIA execution, GPU node placement/performance/memory, FP16 runtime inference,
broader accuracy, interactive TUI use, microphone capture, cross-platform builds,
and release packaging remain unverified. Existing release packages are unchanged.

## Apple Silicon WebGPU extension

The CPU build and daemon have subsequently been reported working smoothly on an
M4 Pro. `--features parakeet-webgpu` adds an experimental native WebGPU/Metal
encoder path on Apple Silicon. FP16 models select it automatically; INT8 models
remain on CPU, and both Whisper and Parakeet remain in the same application and
model picker. The decoder/joint stays on CPU. Provider registration must succeed,
and daemon backend identity prevents reusing a CPU daemon for WebGPU requests.

GPU correctness, actual node placement, and performance still require testing on
the M4 Pro. Mac CI checks are added, not locally verified. See the main README and
PR build/run instructions for the same-model CPU/FP16 versus GPU comparison.
Standard release inclusion remains pending, including runtime packaging and the
lack of a pinned ONNX Runtime prebuilt for Intel Macs.
