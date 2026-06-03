use crate::clipboard::copy_to_clipboard;
use crate::config::PasteConfig;

pub(crate) fn write_text(
    output_text: &str,
    output_file: Option<String>,
    clipboard: bool,
    paste: bool,
    paste_config: &PasteConfig,
    label: &str,
) -> anyhow::Result<()> {
    if let Some(file_path) = output_file {
        std::fs::write(&file_path, output_text).map_err(|err| {
            tracing::warn!("Failed to write to file '{file_path}': {err}");
            anyhow::anyhow!("Failed to write to file '{file_path}': {err}")
        })?;
        tracing::debug!("{label} written to file: {file_path}");
    } else if clipboard {
        match copy_to_clipboard(output_text) {
            Ok(()) => tracing::debug!("{label} copied to clipboard"),
            Err(err) => tracing::warn!("Failed to copy to clipboard: {err}"),
        }
    } else if paste {
        crate::paste::paste_text(output_text, paste_config)?;
        tracing::debug!("{label} pasted to focused app");
    } else {
        println!("{output_text}");
        tracing::debug!("{label} printed to stdout");
    }

    Ok(())
}
