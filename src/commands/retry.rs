//! Retry the last recording with the same transcription model.

pub async fn handle_retry() -> Result<(), anyhow::Error> {
    tracing::info!("=== ostt Retry Command ===");
    Err(anyhow::anyhow!("Retry command not yet implemented"))
}
