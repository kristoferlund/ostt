//! Small HTTP fixture for provider request/response regression tests.

use std::path::PathBuf;

use indexmap::IndexMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::TranscriptionConfig;
use crate::transcription::provider::TranscriptionProvider;

pub(super) struct MockServer {
    pub config: TranscriptionConfig,
    pub audio_path: PathBuf,
    pub task: tokio::task::JoinHandle<Vec<String>>,
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.task.abort();
        let _ = std::fs::remove_file(&self.audio_path);
    }
}

pub(super) async fn mock_server(
    provider: TranscriptionProvider,
    model: &str,
    responses: &[(u16, &str)],
) -> MockServer {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let audio_path = std::env::temp_dir().join(format!(
        "ostt-catalog-{}-{}.mp3",
        std::process::id(),
        address.port()
    ));
    std::fs::write(&audio_path, b"synthetic audio fixture").unwrap();
    let config = TranscriptionConfig::new_cloud(
        provider,
        model.into(),
        format!("http://{address}"),
        "test-key".into(),
        Vec::new(),
        IndexMap::new(),
    );
    let responses: Vec<_> = responses
        .iter()
        .map(|(status, body)| format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()))
        .collect();
    let task = tokio::spawn(async move {
        let mut requests = Vec::new();
        for response in responses {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let count = socket.read(&mut buffer).await.unwrap();
                assert!(count > 0, "request closed before upload completed");
                request.extend_from_slice(&buffer[..count]);
                if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            requests.push(String::from_utf8(request).unwrap());
            socket.write_all(response.as_bytes()).await.unwrap();
        }
        requests
    });
    MockServer {
        config,
        audio_path,
        task,
    }
}
