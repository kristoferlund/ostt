use crate::clipboard::copy_to_clipboard;
use crate::config::PasteConfig;
use anyhow::Context;

pub(crate) fn write_text(
    output_text: &str,
    output_file: Option<String>,
    clipboard: bool,
    paste: bool,
    paste_config: &PasteConfig,
    label: &str,
) -> anyhow::Result<()> {
    write_text_with_handlers(
        output_text,
        output_file,
        clipboard,
        paste,
        paste_config,
        label,
        copy_to_clipboard,
        crate::paste::paste_text,
    )
}

fn write_text_with_handlers<C, P>(
    output_text: &str,
    output_file: Option<String>,
    clipboard: bool,
    paste: bool,
    paste_config: &PasteConfig,
    label: &str,
    mut copy_to_clipboard: C,
    mut paste_text: P,
) -> anyhow::Result<()>
where
    C: FnMut(&str) -> anyhow::Result<()>,
    P: FnMut(&str, &PasteConfig) -> anyhow::Result<()>,
{
    if let Some(file_path) = output_file {
        std::fs::write(&file_path, output_text).map_err(|err| {
            tracing::warn!("Failed to write to file '{file_path}': {err}");
            anyhow::anyhow!("Failed to write to file '{file_path}': {err}")
        })?;
        tracing::debug!("{label} written to file: {file_path}");
    } else if clipboard {
        copy_to_clipboard(output_text)
            .with_context(|| format!("failed to copy {label} to clipboard"))
            .map_err(|err| {
                crate::paste::notify_no_popup_error_if_popup_context(
                    "Clipboard Failed",
                    &err.to_string(),
                );
                err
            })?;
        tracing::debug!("{label} copied to clipboard");
    } else if paste {
        paste_text(output_text, paste_config).map_err(|err| {
            crate::paste::notify_no_popup_error_if_popup_context("Paste Failed", &err.to_string());
            err
        })?;
        tracing::debug!("{label} pasted to focused app");
    } else {
        println!("{output_text}");
        tracing::debug!("{label} printed to stdout");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_clipboard_failure_returns_user_visible_error() {
        let config = PasteConfig::default();

        let err = write_text_with_handlers(
            "hello",
            None,
            true,
            false,
            &config,
            "Output text",
            |_| Err(anyhow::anyhow!("no clipboard backend")),
            |_, _| Ok(()),
        )
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("failed to copy Output text to clipboard"));
    }
}
