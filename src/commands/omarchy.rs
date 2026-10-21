//! `ostt omarchy setup|remove`: register OSTT as an Omarchy dictation backend.
//!
//! Omarchy 4.5 lets third-party programs provide dictation through an
//! `omarchy-dictation-<backend>` adapter and an `omarchy-install-dictation-<backend>`
//! installer on PATH. Setup writes both (and a remover) to `~/.local/bin`, a
//! cancel key to `~/.config/ostt/shortcuts.lua`, and an OSTT row under
//! Setup > Defaults > Dictation in the user's menu extension. It never edits
//! Omarchy's own files, and never changes the selected backend: selecting
//! OSTT in the menu runs the installer, which selects it once setup worked.

use anyhow::Context;
use std::path::{Path, PathBuf};

const MANAGED: &str = "ostt:managed";
const MENU_BEGIN: &str = "// ostt:begin - added by `ostt omarchy setup`";
const MENU_END: &str = "// ostt:end";
const MENU_ENTRY: &str = r#""setup.default.dictation.ostt": {"icon":"󰍬","label":"OSTT","checked":"[[ \"$(omarchy-default-dictation)\" == \"ostt\" ]]","action":"omarchy-default-dictation ostt"},"#;

const SCRIPTS: [(&str, &str); 3] = [
    (
        "omarchy-dictation-ostt",
        include_str!("../../contrib/omarchy/omarchy-dictation-ostt"),
    ),
    (
        "omarchy-install-dictation-ostt",
        include_str!("../../contrib/omarchy/omarchy-install-dictation-ostt"),
    ),
    (
        "omarchy-remove-dictation-ostt",
        include_str!("../../contrib/omarchy/omarchy-remove-dictation-ostt"),
    ),
];
const SHORTCUTS: &str = include_str!("../../contrib/omarchy/shortcuts.lua");

pub(crate) fn handle_setup() -> anyhow::Result<()> {
    if !command_on_path("omarchy-dictation-use") {
        anyhow::bail!(
            "This Omarchy has no selectable dictation backends. OSTT dictation needs Omarchy 4.5 or later."
        );
    }
    let home = home_dir()?;
    let bin_dir = home.join(".local/bin");
    std::fs::create_dir_all(&bin_dir)
        .with_context(|| format!("failed to create {}", bin_dir.display()))?;
    for (name, content) in SCRIPTS {
        write_managed(&bin_dir.join(name), content, true)?;
    }
    write_managed(&shortcuts_path(), SHORTCUTS, false)?;

    let menu = menu_path(&home);
    let existing = read_optional(&menu)?;
    let updated = add_menu_entry(existing.as_deref())?;
    if existing.as_deref() != Some(updated.as_str()) {
        if let Some(parent) = menu.parent() {
            std::fs::create_dir_all(parent)?;
        }
        write_atomically(&menu, &updated, false)?;
    }

    println!("OSTT is available as an Omarchy dictation backend.");
    println!("Select it in Setup > Defaults > Dictation, or run: omarchy default dictation ostt");
    Ok(())
}

pub(crate) async fn handle_remove() -> anyhow::Result<()> {
    let home = home_dir()?;
    // A recording in progress belongs to the backend being removed.
    if let Err(err) =
        crate::dictation::control::cancel(&crate::dictation::state::Paths::default_location()).await
    {
        tracing::warn!("omarchy remove: could not cancel dictation: {err:#}");
    }

    let selection = selection_path();
    if read_optional(&selection)?.is_some_and(|backend| backend.trim() == "ostt") {
        std::fs::remove_file(&selection)
            .with_context(|| format!("failed to remove {}", selection.display()))?;
        println!("OSTT is no longer the Omarchy dictation backend.");
    }

    for (name, _) in SCRIPTS {
        remove_managed(&home.join(".local/bin").join(name))?;
    }
    remove_managed(&shortcuts_path())?;
    let menu = menu_path(&home);
    if let Some(existing) = read_optional(&menu)? {
        let updated = remove_menu_entry(&existing);
        if updated != existing {
            write_atomically(&menu, &updated, false)?;
        }
    }
    if command_on_path("hyprctl") {
        let _ = std::process::Command::new("hyprctl")
            .arg("reload")
            .stdout(std::process::Stdio::null())
            .status();
    }
    println!("Removed the OSTT Omarchy integration. OSTT itself is still installed.");
    Ok(())
}

/// Adds the OSTT row to the menu extension. Existing entries and comments are
/// kept; the row goes right after the opening brace with a trailing comma,
/// which the menu parser accepts whatever follows.
fn add_menu_entry(existing: Option<&str>) -> anyhow::Result<String> {
    let block = format!("  {MENU_BEGIN}\n  {MENU_ENTRY}\n  {MENU_END}\n");
    let content = remove_menu_entry(existing.unwrap_or(""));
    if content.trim().is_empty() {
        return Ok(format!("{{\n{block}}}\n"));
    }
    let mut out = String::with_capacity(content.len() + block.len());
    let mut inserted = false;
    for line in content.split_inclusive('\n') {
        out.push_str(line);
        if !inserted && line.trim() == "{" {
            if !line.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(&block);
            inserted = true;
        }
    }
    if !inserted {
        anyhow::bail!(
            "could not find the opening brace on its own line in the Omarchy menu extension; add this entry by hand:\n  {MENU_ENTRY}"
        );
    }
    Ok(out)
}

fn remove_menu_entry(existing: &str) -> String {
    let mut out = String::with_capacity(existing.len());
    let mut inside = false;
    for line in existing.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed == MENU_BEGIN {
            inside = true;
        } else if inside && trimmed == MENU_END {
            inside = false;
        } else if !inside {
            out.push_str(line);
        }
    }
    out
}

/// Writes a file OSTT owns. A file at that path without the managed marker
/// is the user's own and is left alone.
fn write_managed(path: &Path, content: &str, executable: bool) -> anyhow::Result<()> {
    if let Some(existing) = read_optional(path)? {
        if !existing.contains(MANAGED) {
            eprintln!(
                "Leaving {} unchanged: it was not written by OSTT.",
                path.display()
            );
            return Ok(());
        }
        if existing == content {
            return Ok(());
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    write_atomically(path, content, executable)
}

fn remove_managed(path: &Path) -> anyhow::Result<()> {
    match read_optional(path)? {
        Some(existing) if existing.contains(MANAGED) => std::fs::remove_file(path)
            .with_context(|| format!("failed to remove {}", path.display())),
        _ => Ok(()),
    }
}

fn write_atomically(path: &Path, content: &str, executable: bool) -> anyhow::Result<()> {
    let temporary = path.with_extension(format!("ostt-tmp.{}", std::process::id()));
    std::fs::write(&temporary, content)
        .with_context(|| format!("failed to write {}", temporary.display()))?;
    #[cfg(unix)]
    if executable {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o755))?;
    }
    std::fs::rename(&temporary, path).with_context(|| format!("failed to write {}", path.display()))
}

fn read_optional(path: &Path) -> anyhow::Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(Some(content)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err).with_context(|| format!("failed to read {}", path.display())),
    }
}

fn home_dir() -> anyhow::Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set")
}

fn config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home_dir().unwrap_or_default().join(".config"))
}

/// Omarchy's menu reads the extension from `$HOME/.config`, not `XDG_CONFIG_HOME`.
fn menu_path(home: &Path) -> PathBuf {
    home.join(".config/omarchy/extensions/omarchy-menu.jsonc")
}

/// Where Omarchy looks for a backend's desktop integration.
fn shortcuts_path() -> PathBuf {
    config_home().join("ostt/shortcuts.lua")
}

/// The file holding Omarchy's selected dictation backend.
pub(crate) fn selection_path() -> PathBuf {
    config_home().join("omarchy/defaults/dictation")
}

fn command_on_path(command: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(command).is_file()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHIPPED_SAMPLE: &str = "{\n  // Extend the Quickshell Omarchy menu with JSONC.\n  // \"personal\": {\"icon\":\"\",\"label\":\"Personal\"},\n}\n";

    #[test]
    fn a_missing_menu_extension_is_created_with_only_the_ostt_row() {
        let menu = add_menu_entry(None).unwrap();
        assert!(menu.starts_with("{\n"));
        assert!(menu.contains(MENU_ENTRY));
        assert!(menu.trim_end().ends_with('}'));
    }

    /// The user's own rows must survive, and the result must still parse
    /// when the last user entry has no trailing comma.
    #[test]
    fn the_row_goes_after_the_opening_brace_and_keeps_user_entries() {
        let user =
            "{\n  // my notes\n  \"personal.notes\": {\"label\":\"Notes\",\"action\":\"x\"}\n}\n";
        let menu = add_menu_entry(Some(user)).unwrap();
        assert!(menu.contains("// my notes"));
        assert!(menu.contains("\"personal.notes\""));
        let entry_at = menu.find(MENU_ENTRY).unwrap();
        assert!(entry_at < menu.find("personal.notes").unwrap());

        let json: String = menu
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        serde_json::from_str::<serde_json::Value>(&json).unwrap();
    }

    #[test]
    fn adding_twice_leaves_one_row() {
        let once = add_menu_entry(Some(SHIPPED_SAMPLE)).unwrap();
        let twice = add_menu_entry(Some(&once)).unwrap();
        assert_eq!(once, twice);
        assert_eq!(twice.matches(MENU_ENTRY).count(), 1);
    }

    #[test]
    fn removing_restores_the_original_file() {
        let added = add_menu_entry(Some(SHIPPED_SAMPLE)).unwrap();
        assert_eq!(remove_menu_entry(&added), SHIPPED_SAMPLE);
    }

    #[test]
    fn a_one_line_menu_file_is_refused_rather_than_broken() {
        let err = add_menu_entry(Some("{\"a\": {\"label\": \"A\"}}")).unwrap_err();
        assert!(err.to_string().contains("add this entry by hand"));
    }

    #[test]
    fn files_without_the_marker_are_never_overwritten_or_removed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("omarchy-dictation-ostt");
        std::fs::write(&path, "#!/bin/bash\n# my own adapter\n").unwrap();

        write_managed(&path, "managed ostt:managed", true).unwrap();
        remove_managed(&path).unwrap();

        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "#!/bin/bash\n# my own adapter\n"
        );
    }

    #[test]
    fn managed_files_are_updated_and_removed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bin/omarchy-dictation-ostt");

        write_managed(&path, "v1 ostt:managed", true).unwrap();
        write_managed(&path, "v2 ostt:managed", true).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "v2 ostt:managed");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o111, 0o111);
        }

        remove_managed(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn shipped_files_carry_the_managed_marker() {
        for (name, content) in SCRIPTS {
            assert!(content.contains(MANAGED), "{name} lacks the marker");
        }
        assert!(SHORTCUTS.contains(MANAGED));
    }
}
