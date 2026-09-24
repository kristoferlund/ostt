//! Berget API implementation.
//!
//! Handles transcription requests to Berget's OpenAI-compatible Whisper API using multipart form data.

use serde::Deserialize;
use std::path::Path;
use std::time::Duration;

use super::TranscriptionConfig;
use crate::config::ModelOptionValue;
use crate::transcription::model::{ModelOptionKind, ModelOptionSchema, ModelOptionSpec, ModelSpec};
use indexmap::IndexMap;

pub(super) const MODELS: &[ModelSpec] = &[
    ModelSpec {
        provider_id: "berget",
        model_id: "klang/pianissimo",
        endpoint: "wss://api.berget.ai/v1/realtime?intent=transcription",
        display_name: "Klang Pianissimo (Swedish optimized)",
        description: "Klang AI's Swedish Parakeet-based speech recognition model (KlangAI/pianissimo-sv), hosted through Berget's realtime API. Keyword boosting and prompts are not supported.",
        languages: &["Swedish"],
    },
    ModelSpec {
        provider_id: "berget",
        model_id: "KBLab/kb-whisper-large",
        endpoint: "https://api.berget.ai/v1/audio/transcriptions",
        display_name: "KBLab KB Whisper Large (Swedish optimized)",
        description: "KBLab's Swedish-optimized Whisper Large model from the National Library of Sweden, trained on more than 50,000 hours of Swedish speech. KBLab reports substantially lower Swedish WER than OpenAI Whisper Large V3 across FLEURS, CommonVoice, and NST evaluations.",
        languages: &["Swedish"],
    },
    ModelSpec {
        provider_id: "berget",
        model_id: "NbAiLab/nb-whisper-large",
        endpoint: "https://api.berget.ai/v1/audio/transcriptions",
        display_name: "NbAiLab NB Whisper Large (Norwegian optimized)",
        description: "NbAiLab's Norwegian NB-Whisper Large model from the National Library of Norway. It is trained on about 66,000 hours of speech and targets Norwegian ASR, including Bokmal, Nynorsk, English, and varied regional Norwegian speech.",
        languages: &["Norwegian", "Bokmal", "Nynorsk", "English"],
    },
    ModelSpec {
        provider_id: "berget",
        model_id: "openai/whisper-large-v3",
        endpoint: "https://api.berget.ai/v1/audio/transcriptions",
        display_name: "OpenAI Whisper Large V3 (general-purpose)",
        description: "General-purpose OpenAI Whisper Large V3 hosted through Berget for multilingual transcription and translation when no Swedish- or Norwegian-specialized model is preferred.",
        languages: &["Multilingual"],
    },
];

const OPTIONS: &[ModelOptionSpec] = &[
    ModelOptionSpec {
        name: "hotwords",
        kind: ModelOptionKind::StringList,
    },
    ModelOptionSpec {
        name: "language",
        kind: ModelOptionKind::String,
    },
    ModelOptionSpec {
        name: "prompt",
        kind: ModelOptionKind::String,
    },
    ModelOptionSpec {
        name: "temperature",
        kind: ModelOptionKind::Number,
    },
    ModelOptionSpec {
        name: "response_format",
        kind: ModelOptionKind::String,
    },
    ModelOptionSpec {
        name: "timestamp_granularities",
        kind: ModelOptionKind::StringList,
    },
    ModelOptionSpec {
        name: "align",
        kind: ModelOptionKind::Bool,
    },
    ModelOptionSpec {
        name: "diarize",
        kind: ModelOptionKind::Bool,
    },
    ModelOptionSpec {
        name: "speaker_embeddings",
        kind: ModelOptionKind::Bool,
    },
    ModelOptionSpec {
        name: "chunk_size",
        kind: ModelOptionKind::Integer,
    },
    ModelOptionSpec {
        name: "batch_size",
        kind: ModelOptionKind::Integer,
    },
];

pub(super) fn option_schema(model_id: &str) -> Option<ModelOptionSchema> {
    if model_id == "klang/pianissimo" {
        return Some(super::berget_realtime::option_schema());
    }
    Some(ModelOptionSchema::new(OPTIONS))
}

pub(super) fn validate_options(
    full_model_id: &str,
    options: &IndexMap<String, ModelOptionValue>,
) -> anyhow::Result<()> {
    if full_model_id == "berget/klang/pianissimo" {
        return super::validate_number_range(
            full_model_id,
            options,
            "chunk_seconds",
            f64::MIN_POSITIVE..=f64::MAX,
        );
    }
    super::validate_number_range(full_model_id, options, "temperature", 0.0..=1.0)?;

    if let Some(value) = options.get("response_format") {
        super::validate_string_value(
            full_model_id,
            "response_format",
            value,
            &["json", "verbose_json"],
        )?;
    }

    if let Some(value) = options.get("timestamp_granularities") {
        super::validate_string_list_values(
            full_model_id,
            "timestamp_granularities",
            value,
            &["word", "segment"],
        )?;
    }

    super::validate_integer_range(full_model_id, options, "chunk_size", 1..=60)?;
    super::validate_integer_range(full_model_id, options, "batch_size", 1..=32)?;

    Ok(())
}

/// Berget API response wrapper
#[derive(Debug, Deserialize)]
struct BergetResponse {
    text: String,
}

/// Transcribes an audio file using Berget's Whisper API.
///
/// Uses multipart form data with bearer token authentication.
/// Berget provides an OpenAI-compatible API endpoint.
///
/// Keywords are passed as the `hotwords` parameter (Berget's dedicated keyword boosting)
/// and as the `prompt` parameter (Whisper-compatible context hint).
/// Whisper requests have a 10-second connection deadline and a 30-minute total
/// upload/processing/response deadline, overridden by `config.timeout_secs`.
pub(super) async fn transcribe(
    config: &TranscriptionConfig,
    audio_path: &Path,
) -> anyhow::Result<String> {
    if config.model_id == "klang/pianissimo" {
        return super::berget_realtime::transcribe(config, audio_path).await;
    }
    let audio_data =
        std::fs::read(audio_path).map_err(|e| anyhow::anyhow!("Failed to read audio file: {e}"))?;

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(config.timeout_secs.unwrap_or(30 * 60)))
        .build()?;

    let file_name = audio_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let file_part = reqwest::multipart::Part::bytes(audio_data)
        .file_name(file_name)
        .mime_str(upload_mime_type(audio_path))
        .map_err(|e| anyhow::anyhow!("Failed to create file part for upload: {e}"))?;

    let mut form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", config.model_id.clone());

    // Debug log: Log the API call details (without the audio data)
    let mut debug_params = vec![format!("model={}", config.model_id)];

    if let Some(language) = config.option_string("language") {
        if !language.is_empty() {
            form = form.text("language", language.to_string());
            debug_params.push(format!("language={language}"));
        }
    }

    if let Some(temperature) = config.option_number("temperature") {
        form = form.text("temperature", temperature.to_string());
        debug_params.push(format!("temperature={temperature}"));
    }

    let hotwords = config
        .option_string_list("hotwords")
        .map(|values| values.join(", "))
        .or_else(|| (!config.keywords.is_empty()).then(|| config.keywords.join(", ")));
    if let Some(hotwords) = hotwords {
        form = form.text("hotwords", hotwords.clone());
        debug_params.push(format!("hotwords={hotwords}"));
    }
    let prompt = config
        .option_string("prompt")
        .map(ToString::to_string)
        .or_else(|| (!config.keywords.is_empty()).then(|| config.keywords.join(", ")));
    if let Some(prompt) = prompt {
        form = form.text("prompt", prompt.clone());
        debug_params.push(format!("prompt={prompt}"));
        tracing::debug!("Prompt used for Berget model: {prompt}");
    }

    if let Some(response_format) = response_format(config) {
        form = form.text("response_format", response_format.to_string());
        debug_params.push(format!("response_format={response_format}"));
    }

    if let Some(granularities) = config.option_string_list("timestamp_granularities") {
        if !granularities.is_empty() {
            let granularities = granularities.join(",");
            form = form.text("timestamp_granularities", granularities.clone());
            debug_params.push(format!("timestamp_granularities={granularities}"));
        }
    }

    for option_name in ["align", "diarize", "speaker_embeddings"] {
        if let Some(value) = config.option_bool(option_name) {
            form = form.text(option_name.to_string(), value.to_string());
            debug_params.push(format!("{option_name}={value}"));
        }
    }

    for option_name in ["chunk_size", "batch_size"] {
        if let Some(value) = config.option_integer(option_name) {
            form = form.text(option_name.to_string(), value.to_string());
            debug_params.push(format!("{option_name}={value}"));
        }
    }

    let endpoint = &config.endpoint;

    tracing::debug!(
        "Berget API Call:\n  URL: {}\n  Method: POST\n  Headers:\n    Authorization: Bearer <redacted>\n    Content-Type: multipart/form-data\n  Body parameters: {}",
        endpoint,
        debug_params.join("\n    ")
    );

    let response = match client
        .post(endpoint)
        .bearer_auth(&config.api_key)
        .multipart(form)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            let error_msg = if e.is_connect() {
                "Failed to connect to Berget API server. Check your internet connection."
                    .to_string()
            } else if e.is_timeout() {
                "Request to Berget timed out. The API server is not responding.".to_string()
            } else if e.to_string().contains("builder") {
                format!(
                    "Failed to build Berget API request: {e}. This may be a configuration error."
                )
            } else {
                format!("Berget network error: {e}")
            };
            return Err(anyhow::anyhow!(error_msg));
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read Berget error response: {e}"))?;

        let human_readable = match status.as_u16() {
            401 => "Berget API key is invalid or expired. Please run 'ostt auth' to update your API key.".to_string(),
            403 => "You don't have permission to use Berget's API. Check your API key and account status.".to_string(),
            429 => "Too many requests to Berget. You've hit the API rate limit. Please wait and try again.".to_string(),
            500 | 502 | 503 | 504 => "Berget API server is experiencing issues. Please try again later.".to_string(),
            _ => format!("Berget API error (status {status}): {error_body}"),
        };

        return Err(anyhow::anyhow!(human_readable));
    }

    let berget_response: BergetResponse = response.json().await.map_err(|e| {
        if e.is_timeout() {
            anyhow::anyhow!("Request to Berget timed out while reading the response.")
        } else {
            anyhow::anyhow!("Failed to parse Berget response: {e}")
        }
    })?;

    // Debug log: Log the full response for debugging
    tracing::debug!(
        "Berget API Response:\n  Status: Success\n  Transcription length: {} characters\n  Full response: {:#?}",
        berget_response.text.len(),
        berget_response
    );

    Ok(berget_response.text.trim().to_string())
}

fn upload_mime_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("mp3" | "mpga") => "audio/mpeg",
        Some("mp4") => "video/mp4",
        Some("mpeg") => "video/mpeg",
        Some("m4a") => "audio/mp4",
        Some("wav") => "audio/wav",
        Some("webm") => "audio/webm",
        _ => "application/octet-stream",
    }
}

fn response_format(config: &TranscriptionConfig) -> Option<&str> {
    if let Some(response_format) = config.option_string("response_format") {
        return Some(response_format);
    }

    if config
        .option_string_list("timestamp_granularities")
        .is_some_and(|granularities| !granularities.is_empty())
        || config.option_bool("align") == Some(true)
        || config.option_bool("diarize") == Some(true)
    {
        return Some("verbose_json");
    }

    Some("json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::provider::TranscriptionProvider;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    struct MockServer {
        config: TranscriptionConfig,
        audio_path: std::path::PathBuf,
        task: tokio::task::JoinHandle<String>,
    }

    impl Drop for MockServer {
        fn drop(&mut self) {
            self.task.abort();
            let _ = std::fs::remove_file(&self.audio_path);
        }
    }

    async fn mock_server(extension: &str, response: String, stall: bool) -> MockServer {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let audio_path = std::env::temp_dir().join(format!(
            "ostt-berget-{}-{}.{}",
            std::process::id(),
            address.port(),
            extension
        ));
        std::fs::write(&audio_path, b"audio fixture").unwrap();
        let mut config = TranscriptionConfig::new_cloud(
            TranscriptionProvider::Berget,
            "KBLab/kb-whisper-large".into(),
            format!("http://{address}/v1/audio/transcriptions"),
            "test-key".into(),
            Vec::new(),
            IndexMap::new(),
        );
        config.timeout_secs = Some(5);
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let count = socket.read(&mut buffer).await.unwrap();
                assert!(count > 0, "request closed before upload completed");
                request.extend_from_slice(&buffer[..count]);
                if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]);
                    let length: usize = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse().unwrap())
                        })
                        .expect("multipart request has content-length");
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            socket.write_all(response.as_bytes()).await.unwrap();
            if stall {
                std::future::pending::<()>().await;
            }
            String::from_utf8(request).unwrap()
        });
        MockServer {
            config,
            audio_path,
            task,
        }
    }

    fn json_response(status: &str, body: &str) -> String {
        format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
    }

    #[tokio::test]
    async fn sends_documented_timestamp_csv_and_reads_flat_verbose_response() {
        for model in [
            "KBLab/kb-whisper-large",
            "NbAiLab/nb-whisper-large",
            "openai/whisper-large-v3",
        ] {
            let mut server = mock_server("wav", json_response("200 OK", r#"{"text":" Hej Sverige! ","language":"sv","duration":1,"segments":[{"id":0,"start":0,"end":1,"text":"Hej Sverige!","speaker":"SPEAKER_00"}],"words":[{"word":"Hej","start":0,"end":0.3}]}"#), false).await;
            server.config.model_id = model.into();
            server.config.params.insert(
                "timestamp_granularities".into(),
                ModelOptionValue::StringList(vec!["word".into(), "segment".into()]),
            );
            assert_eq!(
                transcribe(&server.config, &server.audio_path)
                    .await
                    .unwrap(),
                "Hej Sverige!"
            );
            let request = (&mut server.task).await.unwrap();
            assert!(request.starts_with("POST /v1/audio/transcriptions HTTP/1.1\r\n"));
            assert!(request
                .to_ascii_lowercase()
                .contains("authorization: bearer test-key\r\n"));
            assert_eq!(
                request.matches("name=\"timestamp_granularities\"").count(),
                1
            );
            assert!(request.contains("name=\"timestamp_granularities\"\r\n\r\nword,segment\r\n"));
            assert!(!request.contains("timestamp_granularities[]"));
            assert!(request.contains("name=\"response_format\"\r\n\r\nverbose_json\r\n"));
            assert!(request.contains(&format!("name=\"model\"\r\n\r\n{model}\r\n")));
        }
    }

    #[tokio::test]
    async fn uploads_supported_containers_with_correct_mime_and_preserves_audio() {
        for (extension, mime) in [
            ("mp3", "audio/mpeg"),
            ("mpga", "audio/mpeg"),
            ("mp4", "video/mp4"),
            ("mpeg", "video/mpeg"),
            ("m4a", "audio/mp4"),
            ("WAV", "audio/wav"),
            ("webm", "audio/webm"),
            ("unknown", "application/octet-stream"),
        ] {
            let mut server = mock_server(
                extension,
                json_response("200 OK", r#"{"text":"Hej"}"#),
                false,
            )
            .await;
            assert_eq!(
                transcribe(&server.config, &server.audio_path)
                    .await
                    .unwrap(),
                "Hej"
            );
            let request = (&mut server.task).await.unwrap();
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains(&format!("content-type: {mime}\r\n")),
                "{extension}"
            );
            assert!(request.contains("audio fixture\r\n"));
            let filename = server.audio_path.file_name().unwrap().to_str().unwrap();
            assert!(request.contains(&format!("filename=\"{filename}\"")));
        }
        assert_eq!(
            upload_mime_type(Path::new("recording")),
            "application/octet-stream"
        );
    }

    #[tokio::test]
    async fn empty_timestamps_do_not_request_verbose_output() {
        let mut server = mock_server("wav", json_response("200 OK", r#"{"text":""}"#), false).await;
        server.config.params.insert(
            "timestamp_granularities".into(),
            ModelOptionValue::StringList(Vec::new()),
        );
        assert_eq!(
            transcribe(&server.config, &server.audio_path)
                .await
                .unwrap(),
            ""
        );
        let request = (&mut server.task).await.unwrap();
        assert!(!request.contains("timestamp_granularities"));
        assert!(request.contains("name=\"response_format\"\r\n\r\njson\r\n"));
    }

    #[tokio::test]
    async fn rejects_invalid_responses_and_preserves_api_errors() {
        for (status, body, expected) in [
            (
                "200 OK",
                r#"{"segments":[]}"#,
                "Failed to parse Berget response",
            ),
            ("200 OK", "not json", "Failed to parse Berget response"),
            (
                "400 Bad Request",
                r#"{"error":{"message":"invalid language"}}"#,
                "invalid language",
            ),
            ("401 Unauthorized", "{}", "API key is invalid"),
        ] {
            let server = mock_server("wav", json_response(status, body), false).await;
            let error = transcribe(&server.config, &server.audio_path)
                .await
                .unwrap_err();
            assert!(error.to_string().contains(expected), "{error}");
        }
    }

    #[tokio::test]
    async fn configured_deadline_covers_headers_and_response_body() {
        for response in [
            String::new(),
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 100\r\n\r\n"
                .into(),
        ] {
            let mut server = mock_server("wav", response, true).await;
            server.config.timeout_secs = Some(1);
            let error = tokio::time::timeout(
                Duration::from_secs(3),
                transcribe(&server.config, &server.audio_path),
            )
            .await
            .expect("configured deadline must terminate the request")
            .unwrap_err();
            assert!(error.to_string().contains("timed out"), "{error}");
        }
    }
}
