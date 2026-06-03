use indexmap::IndexMap;
use regex::Regex;

pub(crate) fn apply_replace(
    text: &str,
    replace_rules: &IndexMap<String, String>,
) -> anyhow::Result<String> {
    if replace_rules.is_empty() {
        return Ok(text.to_string());
    }

    let rules = replace_rules
        .iter()
        .map(|(source, target)| {
            if source.trim().is_empty() {
                anyhow::bail!("Text replace source must not be empty.");
            }
            Ok((
                Regex::new(&format!(r"(?i)^{}", regex::escape(source)))?,
                target.as_str(),
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

        if let Some((target, end)) = find_replace(text, index, &rules) {
            output.push_str(target);
            index = end;
            continue;
        }

        let next = next_char_boundary(text, index);
        output.push_str(&text[index..next]);
        index = next;
    }

    Ok(output)
}

fn find_replace<'a>(
    text: &str,
    index: usize,
    rules: &'a [(Regex, &'a str)],
) -> Option<(&'a str, usize)> {
    let rest = &text[index..];
    for (matcher, target) in rules {
        let Some(found) = matcher.find(rest) else {
            continue;
        };
        let end = index + found.end();
        if is_right_boundary(text, end) {
            return Some((*target, end));
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

    fn replace_rules(entries: &[(&str, &str)]) -> IndexMap<String, String> {
        entries
            .iter()
            .map(|(from, to)| (from.to_string(), to.to_string()))
            .collect()
    }

    #[test]
    fn applies_literal_case_insensitive_replace() {
        let result = apply_replace(
            "open ai api is useful",
            &replace_rules(&[("api", "API"), ("open ai", "OpenAI")]),
        )
        .unwrap();

        assert_eq!(result, "OpenAI API is useful");
    }

    #[test]
    fn avoids_replacing_inside_larger_words() {
        let result = apply_replace("capital api", &replace_rules(&[("api", "API")])).unwrap();

        assert_eq!(result, "capital API");
    }

    #[test]
    fn applies_phrases_before_punctuation() {
        let result =
            apply_replace("Vox type.", &replace_rules(&[("vox type", "Voxtype")])).unwrap();

        assert_eq!(result, "Voxtype.");
    }

    #[test]
    fn uses_config_order_for_overlapping_replace_rules() {
        let result = apply_replace(
            "open ai api",
            &replace_rules(&[("open ai", "OpenAI"), ("open ai api", "OpenAI API")]),
        )
        .unwrap();

        assert_eq!(result, "OpenAI api");
    }

    #[test]
    fn does_not_reprocess_replace_output() {
        let result =
            apply_replace("ostt", &replace_rules(&[("ostt", "api"), ("api", "API")])).unwrap();

        assert_eq!(result, "api");
    }

    #[test]
    fn uses_unicode_aware_boundaries() {
        let result = apply_replace("räksmörgås api", &replace_rules(&[("api", "API")])).unwrap();

        assert_eq!(result, "räksmörgås API");
    }
}
