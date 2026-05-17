use std::path::Path;

use super::TranscriptionConfig;

pub async fn transcribe(
    _config: &TranscriptionConfig,
    _audio_path: &Path,
) -> anyhow::Result<String> {
    anyhow::bail!("local provider not yet implemented - this is a stub")
}
