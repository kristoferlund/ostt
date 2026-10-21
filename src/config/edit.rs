//! In-place edits of `ostt.toml`.
//!
//! OSTT changes only the keys it manages; everything else in the file, including
//! the user's comments and formatting, is written back as it was read.

use std::fs;
use std::path::Path;

use anyhow::Context;
use indexmap::IndexMap;
use toml_edit::{value, Array, DocumentMut, Item, Table, Value};

/// Applies `edit` to the config file and writes the result back.
pub(crate) fn edit_config_file(
    edit: impl FnOnce(&mut DocumentMut) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    edit_config_file_at(&crate::app_dirs::config_path()?, edit)
}

/// [`edit_config_file`] for a given path; a missing file starts empty.
pub(crate) fn edit_config_file_at(
    path: &Path,
    edit: impl FnOnce(&mut DocumentMut) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.into()),
    };
    let mut doc: DocumentMut = content
        .parse()
        .with_context(|| format!("{} is not valid TOML", path.display()))?;
    edit(&mut doc)?;
    fs::write(path, doc.to_string())?;
    Ok(())
}

/// Sets `[transcription]` provider and model, or removes them for `None`.
/// Other keys in the section are kept.
pub(crate) fn set_selected_model(
    doc: &mut DocumentMut,
    selection: Option<(&str, &str)>,
) -> anyhow::Result<()> {
    match selection {
        Some((provider_id, model_id)) => {
            let transcription = table_mut(doc, "transcription", false)?;
            set_string(transcription, "provider", provider_id);
            set_string(transcription, "model", model_id);
        }
        None => {
            if let Some(transcription) = existing_table_mut(doc, "transcription")? {
                transcription.remove("provider");
                transcription.remove("model");
            }
            remove_if_empty(doc, "transcription");
        }
    }
    Ok(())
}

/// Sets `[transcription].keywords`, one keyword per line so the list is easy to
/// edit by hand. An empty list removes the key.
pub(crate) fn set_keywords(doc: &mut DocumentMut, keywords: &[String]) -> anyhow::Result<()> {
    if keywords.is_empty() {
        if let Some(transcription) = existing_table_mut(doc, "transcription")? {
            transcription.remove("keywords");
        }
        remove_if_empty(doc, "transcription");
        return Ok(());
    }
    let mut array: Array = keywords.iter().map(String::as_str).collect();
    for keyword in array.iter_mut() {
        keyword.decor_mut().set_prefix("\n    ");
    }
    array.set_trailing_comma(true);
    array.set_trailing("\n");
    set_value(
        table_mut(doc, "transcription", false)?,
        "keywords",
        array.into(),
    );
    Ok(())
}

/// Makes `[text.replace]` hold exactly `rules`. Rules already in the file keep
/// their place and comments; new rules are appended in order.
pub(crate) fn set_replace_rules(
    doc: &mut DocumentMut,
    rules: &IndexMap<String, String>,
) -> anyhow::Result<()> {
    if rules.is_empty() {
        if let Some(text) = existing_table_mut(doc, "text")? {
            text.remove("replace");
        }
        remove_if_empty(doc, "text");
        return Ok(());
    }

    // An implicit [text] is not printed when it only holds [text.replace].
    let text = table_mut(doc, "text", true)?;
    let replace = text
        .entry("replace")
        .or_insert_with(|| Item::Table(Table::new()))
        .as_table_mut()
        .context("'text.replace' in ostt.toml must be a table; edit it by hand")?;
    let stale: Vec<String> = replace
        .iter()
        .map(|(source, _)| source.to_string())
        .filter(|source| !rules.contains_key(source))
        .collect();
    for source in stale {
        replace.remove(&source);
    }
    for (source, target) in rules {
        set_string(replace, source, target);
    }
    Ok(())
}

fn set_string(table: &mut Table, key: &str, new: &str) {
    set_value(table, key, new.into());
}

/// Sets a value. An existing key keeps its position, the comments above it and
/// any trailing comment; `Table::insert` would replace all of them.
fn set_value(table: &mut Table, key: &str, mut new: Value) {
    match table.get_mut(key).and_then(Item::as_value_mut) {
        Some(existing) => {
            *new.decor_mut() = existing.decor().clone();
            *existing = new;
        }
        None => {
            table.insert(key, value(new));
        }
    }
}

fn table_mut<'a>(
    doc: &'a mut DocumentMut,
    name: &str,
    implicit: bool,
) -> anyhow::Result<&'a mut Table> {
    doc.entry(name)
        .or_insert_with(|| {
            let mut table = Table::new();
            table.set_implicit(implicit);
            Item::Table(table)
        })
        .as_table_mut()
        .with_context(|| format!("'{name}' in ostt.toml must be a table; edit it by hand"))
}

fn existing_table_mut<'a>(
    doc: &'a mut DocumentMut,
    name: &str,
) -> anyhow::Result<Option<&'a mut Table>> {
    match doc.get_mut(name) {
        None => Ok(None),
        Some(item) => item
            .as_table_mut()
            .map(Some)
            .with_context(|| format!("'{name}' in ostt.toml must be a table; edit it by hand")),
    }
}

fn remove_if_empty(doc: &mut DocumentMut, name: &str) {
    if doc
        .get(name)
        .and_then(Item::as_table)
        .is_some_and(Table::is_empty)
    {
        doc.remove(name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edited(content: &str, edit: impl FnOnce(&mut DocumentMut)) -> String {
        let mut doc: DocumentMut = content.parse().unwrap();
        edit(&mut doc);
        doc.to_string()
    }

    fn rules(entries: &[(&str, &str)]) -> IndexMap<String, String> {
        entries
            .iter()
            .map(|(source, target)| (source.to_string(), target.to_string()))
            .collect()
    }

    const CONFIG: &str = r#"config_version = "0.0.29"
# My audio setup
[audio]
device = "default"

[transcription]
# Switched for meetings
provider = "deepgram"
model = "nova-3" # cheapest for long calls

[text.replace]
# Brand spelling
"ostt" = "OSTT"
"api" = "API"

[popup]
width = 90 # wide enough for the waveform
"#;

    /// Selecting a model used to rewrite the whole [transcription] section,
    /// deleting comments and any other keys the user kept there.
    #[test]
    fn selecting_a_model_changes_only_provider_and_model() {
        let updated = edited(CONFIG, |doc| {
            set_selected_model(doc, Some(("parakeet", "parakeet-ultra"))).unwrap()
        });
        assert_eq!(
            updated,
            CONFIG
                .replace("\"deepgram\"", "\"parakeet\"")
                .replace("\"nova-3\"", "\"parakeet-ultra\"")
        );
    }

    #[test]
    fn selecting_a_model_without_a_transcription_section_adds_one() {
        let updated = edited("[audio]\ndevice = \"default\"\n", |doc| {
            set_selected_model(doc, Some(("whisper", "base"))).unwrap()
        });
        let config: toml::Table = updated.parse().unwrap();
        assert_eq!(
            config["transcription"]["provider"].as_str(),
            Some("whisper")
        );
        assert_eq!(config["transcription"]["model"].as_str(), Some("base"));
        assert!(updated.starts_with("[audio]\ndevice = \"default\"\n"));
    }

    #[test]
    fn clearing_the_selection_keeps_other_transcription_keys() {
        let content =
            "[transcription]\nprovider = \"deepgram\"\nmodel = \"nova-3\"\nkeywords = [\"OSTT\"]\n";
        let updated = edited(content, |doc| set_selected_model(doc, None).unwrap());
        assert_eq!(updated, "[transcription]\nkeywords = [\"OSTT\"]\n");

        let updated = edited(
            "[transcription]\nprovider = \"a\"\nmodel = \"b\"\n",
            |doc| set_selected_model(doc, None).unwrap(),
        );
        assert_eq!(updated, "");
    }

    #[test]
    fn keywords_are_written_one_per_line_next_to_the_selection() {
        let updated = edited(CONFIG, |doc| {
            set_keywords(doc, &["OSTT".to_string(), "Rödeby".to_string()]).unwrap()
        });
        assert!(
            updated.contains(
                "model = \"nova-3\" # cheapest for long calls\nkeywords = [\n    \"OSTT\",\n    \"Rödeby\",\n]\n"
            ),
            "{updated}"
        );
        assert!(updated.contains("# Switched for meetings\nprovider = \"deepgram\""));

        // Selecting a model must not drop the keywords stored beside it.
        let reselected = edited(&updated, |doc| {
            set_selected_model(doc, Some(("whisper", "base"))).unwrap()
        });
        let config: toml::Table = reselected.parse().unwrap();
        assert_eq!(
            config["transcription"]["keywords"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn an_empty_keyword_list_removes_the_key_but_not_the_selection() {
        let content = "[transcription]\nprovider = \"a\"\nmodel = \"b\"\nkeywords = [\"x\"]\n";
        let updated = edited(content, |doc| set_keywords(doc, &[]).unwrap());
        assert_eq!(
            updated,
            "[transcription]\nprovider = \"a\"\nmodel = \"b\"\n"
        );
    }

    /// Saving rules used to delete all of [text] and rewrite it, losing the
    /// comments next to rules the user kept.
    #[test]
    fn saving_replace_rules_keeps_kept_rules_with_their_comments() {
        let updated = edited(CONFIG, |doc| {
            set_replace_rules(doc, &rules(&[("ostt", "OSTT"), ("k8s", "Kubernetes")])).unwrap()
        });
        assert!(
            updated.contains("# Brand spelling\n\"ostt\" = \"OSTT\"\n"),
            "{updated}"
        );
        assert!(updated.contains("k8s = \"Kubernetes\"\n"), "{updated}");
        assert!(!updated.contains("\"api\""), "{updated}");
        assert!(updated.contains("# Switched for meetings"));
        assert!(updated.contains("width = 90 # wide enough for the waveform"));
    }

    #[test]
    fn changing_a_rule_target_keeps_its_place() {
        let updated = edited(CONFIG, |doc| {
            set_replace_rules(doc, &rules(&[("ostt", "Ostt"), ("api", "API")])).unwrap()
        });
        assert!(updated.contains("# Brand spelling\n\"ostt\" = \"Ostt\"\n\"api\" = \"API\"\n"));
    }

    #[test]
    fn first_replace_rule_adds_a_text_replace_section_without_an_empty_text_header() {
        let updated = edited("[audio]\ndevice = \"default\"\n", |doc| {
            set_replace_rules(doc, &rules(&[("ostt", "OSTT")])).unwrap()
        });
        assert!(
            updated.contains("[text.replace]\nostt = \"OSTT\"\n"),
            "{updated}"
        );
        assert!(!updated.contains("[text]\n"), "{updated}");
    }

    #[test]
    fn removing_every_rule_removes_the_section_and_nothing_else() {
        let updated = edited(CONFIG, |doc| {
            set_replace_rules(doc, &IndexMap::new()).unwrap()
        });
        assert!(!updated.contains("[text"), "{updated}");
        assert!(updated.contains("[transcription]\n# Switched for meetings"));
        assert!(updated.contains("[popup]"));
    }

    #[test]
    fn rules_with_quotes_and_backslashes_round_trip() {
        let tricky = rules(&[("say \"hi\"", r"C:\path")]);
        let updated = edited("", |doc| set_replace_rules(doc, &tricky).unwrap());
        let config: crate::config::OsttConfig =
            toml::from_str(&format!("[audio]\ndevice = \"default\"\n{updated}")).unwrap();
        assert_eq!(config.text.replace, tricky);
    }
}
