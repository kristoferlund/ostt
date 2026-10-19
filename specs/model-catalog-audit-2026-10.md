# Transcription model catalog audit — 2026-10-20

Scope: all eight built-in cloud providers and the Whisper/Parakeet downloadable
registry. Only models usable by OSTT's completed-recording workflow are offered;
this refresh does not add providers, streaming transports or local architectures.
Provider claims below are documentation, not independently measured accuracy.

## Cloud providers

| Provider | Decision | Primary sources |
| --- | --- | --- |
| OpenAI | Add `gpt-transcribe`, now recommended for files, with native `keywords[]`, `languages[]` and contextual `prompt`. Keep GPT-4o, Mini, Diarize and Whisper. Remove Diarize's unsupported prompt; do not forward saved keywords to it. Default its chunking to `auto`, required beyond 30 seconds. | [File transcription](https://platform.openai.com/docs/guides/speech-to-text), [model](https://platform.openai.com/docs/models/gpt-transcribe), [request reference](https://platform.openai.com/docs/api-reference/audio/createTranscription) |
| Deepgram | Keep Nova 3/2; add Nova 3 Medical and Pharma. Nova 3 variants use `keyterm`, not legacy `keywords`. Pharma is English; Medical also supports multilingual code-switching. | [Models and languages](https://developers.deepgram.com/docs/models-languages-overview), [keyterms](https://developers.deepgram.com/docs/keyterm) |
| Groq | No additions: the current file STT catalog remains `whisper-large-v3` and `whisper-large-v3-turbo`. Existing JSON/verbose JSON, prompt, language, temperature and timestamp handling matches the file API. | [Speech to text](https://console.groq.com/docs/speech-to-text) |
| DeepInfra | Add `Qwen/Qwen3-ASR-0.6B`, `Qwen/Qwen3-ASR-1.7B` and `nvidia/Nemotron-3.5-ASR-Streaming-Multilingual-0.6b`. All have documented multipart file-inference requests and a `text` response; Nemotron's name does not restrict it to WebSockets. Keep Whisper Large v3/Turbo and the two Voxtral models. Remove five old Whisper entries absent from the current catalog whose model pages return HTTP 404. | [Current ASR catalog](https://deepinfra.com/models/automatic-speech-recognition), [Qwen 0.6B API](https://deepinfra.com/Qwen/Qwen3-ASR-0.6B/api), [Qwen 1.7B API](https://deepinfra.com/Qwen/Qwen3-ASR-1.7B/api), [Nemotron file API](https://deepinfra.com/nvidia/Nemotron-3.5-ASR-Streaming-Multilingual-0.6b/api) |
| AssemblyAI | Replace `universal-3-pro` with `universal-3-5-pro` and add `universal-2`. Send the documented priority list `["universal-3-5-pro", "universal-2"]` for 99-language coverage, not just the 18 native Pro languages. Contextual prompt/temperature controls are Pro-specific. An explicit language code disables detection by default. | [Models](https://www.assemblyai.com/docs/getting-started/models), [model selection](https://www.assemblyai.com/docs/pre-recorded-audio/select-the-speech-model), [request reference](https://www.assemblyai.com/docs/api-reference/transcripts/submit) |
| Berget | No new distinct STT models. The public catalog lists KB Whisper, NB Whisper, Pianissimo and `Systran/faster-whisper-large-v3`. Keep the supported `openai/whisper-large-v3` alias to avoid breaking existing configurations or duplicating the same deployment. Existing Whisper/Pianissimo transport and parameter split stays intact. | [Live catalog](https://api.berget.ai/v1/models), [OpenAPI](https://api.berget.ai/openapi.json) |
| ElevenLabs | Add `scribe_v2_medical`, documented with the same API/features/languages as v2. Keep v2 and mark v1 legacy: deprecated does not mean unavailable. Replace stale exact language-count labels with the current 90+ wording. | [Models and deprecations](https://elevenlabs.io/docs/overview/models), [file STT reference](https://elevenlabs.io/docs/api-reference/speech-to-text/convert) |
| Mistral | No new file model found. Retain `voxtral-mini-latest` and pinned `voxtral-mini-2602`, labeled as the same Transcribe V2 family rather than implying the pinned version is newer. Existing language/timestamp exclusion remains documented. | [Speech transcription](https://docs.mistral.ai/studio/audio/speech_to_text), [offline API guide](https://docs.mistral.ai/studio/audio/speech_to_text/offline_transcription) |

DeepInfra's model pages embed provider-owned input/output schemas in their
`__NEXT_DATA__`/`static_info`. The three new models expose the existing ASR fields
(`audio`, `language`, `initial_prompt`, `task`, `temperature`, `chunk_level`,
`chunk_length_s`) and top-level `text`, so no speculative adapter was added.

### Intentionally excluded

- OpenAI live-transcription models, Deepgram Flux, ElevenLabs Scribe Realtime,
  AssemblyAI streaming/sync/dictation APIs and Voxtral Realtime need other
  protocols/endpoints. Listing them behind an existing batch adapter would fail.
- DeepInfra Nemotron Diarization identifies speakers but is not a replacement
  speech-to-text model. It is not offered as a transcription choice.
- Historical aliases/snapshots and every older Nova 2 specialization are not
  duplicated in the general-purpose picker. The currently supported rolling
  OpenAI aliases and existing pinned Mistral model remain available.
- No optional paid enrichment features, webhook jobs, new output formats or
  transcript-editing instructions are introduced by this catalog refresh.
  OSTT still returns transcript text, not a speaker/timestamp report.

## Downloadable models

The companion [ostt-models](https://github.com/kristoferlund/ostt-models) change
expands the catalog from 20 to 32 entries without changing existing IDs:

- English Whisper Tiny/Base/Small/Medium and English Distil-Whisper Large v3.5.
- Small Q5_1, Medium/Large v3/Turbo Q5_0, Turbo Q8_0.
- Official KB/NB Whisper Large Q5_0, giving Nordic users a smaller download.
- Existing full-precision Whisper and four Parakeet choices remain available.

Every file URL is pinned to a commit and every SHA-256/size is checked against
Hugging Face LFS metadata at that revision. Sizes consistently use rounded-up
MiB. These are artifact integrity checks, not benchmark or quality guarantees.
Quantization can change accuracy and does not necessarily improve latency.

The current `models-v2.json` contains Whisper and Parakeet; `models.json` contains
exactly the same Whisper entries for older clients. Both retain document version
1. Automated checks enforce parity so a new engine cannot break old clients.

Sources: [whisper.cpp artifacts](https://huggingface.co/ggerganov/whisper.cpp),
[Distil v3.5 GGML](https://huggingface.co/distil-whisper/distil-large-v3.5-ggml),
[KBLab](https://huggingface.co/KBLab), [NbAiLab](https://huggingface.co/NbAiLab),
[Parakeet GGUF publisher](https://huggingface.co/handy-computer).

Other local architectures (Qwen, Moonshine, Canary, Voxtral) are excluded even
when GGUF weights exist: OSTT currently exposes only Whisper and Parakeet.
Older Whisper Large v1/v2, older Distil releases, every regional quantization and
every English/quantization combination are omitted to keep choices useful.

## Compatibility / migration

- Existing supported selections and default provider choice do not change.
- Replace `assemblyai/universal-3-pro` with `assemblyai/universal-3-5-pro`,
  including model-specific TOML section names.
- Replace removed DeepInfra `openai/whisper-large`, `openai/whisper-medium`,
  `openai/whisper-small`, `openai/whisper-base` or
  `openai/whisper-timestamped-medium` with Large v3, Turbo or Qwen3 ASR. Model
  IDs are not silently rewritten to a differently priced/behaving model.
- Remove explicit OpenAI Diarize `prompt`. Use `keyterm`, not `keywords`, for
  Deepgram Nova 3. Saved OSTT keywords are routed correctly automatically.
- GPT Transcribe uses plural `languages`, not `language`; put model-specific
  fields under `[openai.gpt-transcribe.params]`, not shared `[openai.params]`.
  Its initial schema deliberately exposes only the documented context controls;
  it does not inherit GPT-4o-only `include=logprobs`.
- Local English models require quoted TOML names such as
  `[whisper."tiny.en".params]` because their IDs contain a dot.
- Website follow-up: refresh the provider reference and local-model guide at
  ostt.ai from these catalogs; the website repository is outside this PR.

## Verification

Regression tests cover catalog/schema consistency, new multipart request fields
and JSON responses, native versus saved keyword precedence, invalid keyword
rejection, long-audio diarization defaults, Nova 3 specialty keyterms, AssemblyAI
model fallback/language handling, and legacy/current registry parity. They do
not establish live model availability for every account or measure accuracy.

See the pull requests for commands run, live inference results and platform/API
limitations for this audit.
