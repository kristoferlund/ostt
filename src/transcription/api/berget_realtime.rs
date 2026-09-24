//! File transcription over Berget's realtime API, using one manually committed turn.

use std::{path::Path, time::Duration};

use anyhow::{bail, Context};
use base64::{engine::general_purpose::STANDARD, Engine};
use futures_util::{SinkExt, Stream, StreamExt};
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Error, Message};

use super::TranscriptionConfig;
use crate::transcription::model::{ModelOptionKind, ModelOptionSchema, ModelOptionSpec};

pub(super) fn option_schema() -> ModelOptionSchema {
    ModelOptionSchema::new(&[
        ModelOptionSpec {
            name: "language",
            kind: ModelOptionKind::String,
        },
        ModelOptionSpec {
            name: "chunk_seconds",
            kind: ModelOptionKind::Number,
        },
    ])
}

pub(super) async fn transcribe(
    config: &TranscriptionConfig,
    audio_path: &Path,
) -> anyhow::Result<String> {
    let output = tokio::process::Command::new(crate::recording::find_ffmpeg()?)
        .args(["-nostdin", "-v", "error", "-i"])
        .arg(audio_path)
        .args(["-vn", "-f", "s16le", "-ac", "1", "-ar", "24000", "pipe:1"])
        .kill_on_drop(true)
        .output()
        .await
        .context("Failed to decode audio for Berget realtime")?;
    if !output.status.success() {
        bail!(
            "Failed to decode audio for Berget realtime: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    if output.stdout.is_empty() {
        bail!("Audio file contains no samples");
    }

    let timeout = Duration::from_secs(config.timeout_secs.unwrap_or(120));
    let mut request = config.endpoint.as_str().into_client_request()?;
    let mut authorization = format!("Bearer {}", config.api_key)
        .parse::<tokio_tungstenite::tungstenite::http::HeaderValue>()?;
    authorization.set_sensitive(true);
    request.headers_mut().insert("Authorization", authorization);
    let (mut socket, _) = tokio::time::timeout(timeout, tokio_tungstenite::connect_async(request))
        .await
        .context("Berget realtime connection timed out")?
        .context("Failed to connect to Berget realtime API")?;
    socket
        .send(Message::Text(
            json!({
                "type": "session.update",
                "session": { "type": "transcription", "audio": { "input": {
                    "format": { "type": "audio/pcm", "rate": 24000 },
                    "transcription": {
                        "model": config.model_id,
                        "languages": [config.option_string("language").unwrap_or("sv")],
                        "chunk_seconds": config.option_number("chunk_seconds").unwrap_or(3.0)
                    },
                    "turn_detection": null
                }}}
            })
            .to_string()
            .into(),
        ))
        .await?;
    loop {
        let event = next_event(&mut socket, timeout).await?;
        if event["type"] == "session.updated" {
            break;
        }
    }

    let (mut sender, mut receiver) = socket.split();
    let upload = async {
        // 200 ms of PCM16 per message. Read responses concurrently to avoid backpressure.
        for chunk in output.stdout.chunks(9600) {
            sender
                .send(Message::Text(
                    json!({
                        "type": "input_audio_buffer.append", "audio": STANDARD.encode(chunk)
                    })
                    .to_string()
                    .into(),
                ))
                .await?;
        }
        sender
            .send(Message::Text(
                json!({"type": "input_audio_buffer.commit"})
                    .to_string()
                    .into(),
            ))
            .await?;
        Ok::<_, anyhow::Error>(())
    };
    let (_, transcript) = tokio::try_join!(
        async {
            tokio::time::timeout(timeout, upload)
                .await
                .context("Berget realtime upload timed out")?
        },
        final_transcript(&mut receiver, timeout)
    )?;
    let _ = tokio::time::timeout(timeout, sender.close()).await;
    Ok(transcript)
}

async fn next_event(
    stream: &mut (impl Stream<Item = Result<Message, Error>> + Unpin),
    timeout: Duration,
) -> anyhow::Result<Value> {
    tokio::time::timeout(timeout, async {
        while let Some(message) = stream.next().await {
            match message? {
                Message::Text(text) => {
                    let event: Value =
                        serde_json::from_str(&text).context("Invalid Berget realtime response")?;
                    if event["type"] == "error"
                        || event["type"] == "conversation.item.input_audio_transcription.failed"
                    {
                        bail!("Berget realtime transcription failed: {}", event["error"]);
                    }
                    return Ok(event);
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
        bail!("Berget realtime connection closed before transcription completed")
    })
    .await
    .context("Berget realtime response timed out")?
}

async fn final_transcript(
    stream: &mut (impl Stream<Item = Result<Message, Error>> + Unpin),
    timeout: Duration,
) -> anyhow::Result<String> {
    loop {
        let event = next_event(stream, timeout).await?;
        if event["type"] == "conversation.item.input_audio_transcription.completed" {
            return Ok(event["transcript"]
                .as_str()
                .context("Berget realtime response is missing transcript")?
                .trim()
                .to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn events(values: Vec<Value>) -> impl Stream<Item = Result<Message, Error>> + Unpin {
        futures_util::stream::iter(
            values
                .into_iter()
                .map(|v| Ok(Message::Text(v.to_string().into()))),
        )
    }

    #[tokio::test]
    async fn returns_authoritative_final_text_without_duplicating_deltas() {
        let mut stream = events(vec![
            json!({"type":"conversation.item.input_audio_transcription.delta", "delta":"Hej"}),
            json!({"type":"conversation.item.input_audio_transcription.completed", "transcript":" Hej Sverige! "}),
        ]);
        assert_eq!(
            final_transcript(&mut stream, Duration::from_secs(1))
                .await
                .unwrap(),
            "Hej Sverige!"
        );
    }

    #[tokio::test]
    async fn partial_transcripts_are_not_success_on_failure_or_disconnect() {
        for ending in [
            None,
            Some(json!({"type":"error", "error":{"message":"bad model"}})),
            Some(
                json!({"type":"conversation.item.input_audio_transcription.failed", "error":{"message":"inference failed"}}),
            ),
        ] {
            let mut values = vec![
                json!({"type":"conversation.item.input_audio_transcription.delta", "delta":"Hej"}),
            ];
            values.extend(ending);
            assert!(
                final_transcript(&mut events(values), Duration::from_secs(1))
                    .await
                    .is_err()
            );
        }
    }

    #[test]
    fn rejects_whisper_tuning_for_pianissimo() {
        for name in ["prompt", "hotwords", "temperature", "diarize"] {
            assert!(super::super::berget::option_schema("klang/pianissimo")
                .unwrap()
                .option(name)
                .is_none());
            let params = indexmap::IndexMap::from([(
                name.to_string(),
                crate::config::ModelOptionValue::String("test".into()),
            )]);
            assert!(crate::config::file::validate_params_for_model(
                "berget",
                "klang/pianissimo",
                &params
            )
            .is_err());
        }
    }

    #[test]
    fn chunk_duration_must_be_positive_and_finite() {
        for (value, valid) in [
            (3.0, true),
            (0.5, true),
            (0.0, false),
            (-1.0, false),
            (f64::NAN, false),
            (f64::INFINITY, false),
        ] {
            let params = indexmap::IndexMap::from([(
                "chunk_seconds".into(),
                crate::config::ModelOptionValue::Number(value),
            )]);
            assert_eq!(
                crate::config::file::validate_params_for_model(
                    "berget",
                    "klang/pianissimo",
                    &params
                )
                .is_ok(),
                valid
            );
        }
    }
}
