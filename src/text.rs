use indexmap::IndexMap;
use regex::Regex;

pub(crate) fn apply_replacements(
    text: &str,
    replacements: &IndexMap<String, String>,
) -> anyhow::Result<String> {
    if replacements.is_empty() {
        return Ok(text.to_string());
    }

    let rules = replacements
        .iter()
        .map(|(source, replacement)| {
            if source.trim().is_empty() {
                anyhow::bail!("Text replacement source must not be empty.");
            }
            Ok((
                Regex::new(&format!(r"(?i)^{}", regex::escape(source)))?,
                replacement.as_str(),
            ))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let mut output = String::with_capacity(text.len());
    let mut index = 0;
    while index < text.len() {
        if !is_left_boundary(text, index) {
            let next = next_char_boundary(text, index);
            output.push_str(&text[index..next]);
            index = next;
            continue;
        }

        if let Some((replacement, end)) = find_replacement(text, index, &rules) {
            output.push_str(replacement);
            index = end;
            continue;
        }

        let next = next_char_boundary(text, index);
        output.push_str(&text[index..next]);
        index = next;
    }

    Ok(output)
}

fn find_replacement<'a>(
    text: &str,
    index: usize,
    rules: &'a [(Regex, &'a str)],
) -> Option<(&'a str, usize)> {
    let rest = &text[index..];
    for (matcher, replacement) in rules {
        let Some(found) = matcher.find(rest) else {
            continue;
        };
        let end = index + found.end();
        if is_right_boundary(text, end) {
            return Some((*replacement, end));
        }
    }
    None
}

fn is_left_boundary(text: &str, index: usize) -> bool {
    index == 0 || !text[..index].chars().next_back().is_some_and(is_word_char)
}

fn is_right_boundary(text: &str, index: usize) -> bool {
    index == text.len() || !text[index..].chars().next().is_some_and(is_word_char)
}

fn is_word_char(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

fn next_char_boundary(text: &str, index: usize) -> usize {
    index + text[index..].chars().next().map_or(0, char::len_utf8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn replacements(entries: &[(&str, &str)]) -> IndexMap<String, String> {
        entries
            .iter()
            .map(|(from, to)| (from.to_string(), to.to_string()))
            .collect()
    }

    #[test]
    fn applies_literal_case_insensitive_replacements() {
        let result = apply_replacements(
            "open ai api is useful",
            &replacements(&[("api", "API"), ("open ai", "OpenAI")]),
        )
        .unwrap();

        assert_eq!(result, "OpenAI API is useful");
    }

    #[test]
    fn avoids_replacing_inside_larger_words() {
        let result = apply_replacements("capital api", &replacements(&[("api", "API")])).unwrap();

        assert_eq!(result, "capital API");
    }

    #[test]
    fn replaces_phrases_before_punctuation() {
        let result =
            apply_replacements("Vox type.", &replacements(&[("vox type", "Voxtype")])).unwrap();

        assert_eq!(result, "Voxtype.");
    }

    #[test]
    fn uses_config_order_for_overlapping_replacements() {
        let result = apply_replacements(
            "open ai api",
            &replacements(&[("open ai", "OpenAI"), ("open ai api", "OpenAI API")]),
        )
        .unwrap();

        assert_eq!(result, "OpenAI api");
    }

    #[test]
    fn does_not_reprocess_replacement_output() {
        let result =
            apply_replacements("ostt", &replacements(&[("ostt", "api"), ("api", "API")])).unwrap();

        assert_eq!(result, "api");
    }

    #[test]
    fn uses_unicode_aware_boundaries() {
        let result =
            apply_replacements("räksmörgås api", &replacements(&[("api", "API")])).unwrap();

        assert_eq!(result, "räksmörgås API");
    }
}
