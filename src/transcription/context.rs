use crate::config::{self, OsttConfig, SelectedModel};
use indexmap::IndexMap;

use super::TranscriptionConfig;

pub(crate) struct TranscriptionContext {
    pub(crate) selected_model: SelectedModel,
    pub(crate) config: TranscriptionConfig,
    pub(crate) keywords: Vec<String>,
}

pub(crate) fn build_context(
    ostt_config: &OsttConfig,
    model_override: Option<SelectedModel>,
    param_overrides: &[String],
) -> anyhow::Result<TranscriptionContext> {
    let selected_model = resolve_selected_model(ostt_config, model_override)?;
    let keywords = crate::keywords::load_keywords()?;
    let transcription_config = config_for_selected_model(
        ostt_config,
        &selected_model,
        keywords.clone(),
        param_overrides,
    )?;

    Ok(TranscriptionContext {
        selected_model,
        config: transcription_config,
        keywords,
    })
}

fn resolve_selected_model(
    ostt_config: &OsttConfig,
    model_override: Option<SelectedModel>,
) -> anyhow::Result<SelectedModel> {
    if let Some(selected_model) = model_override {
        return Ok(selected_model);
    }

    match (
        ostt_config.transcription.provider.as_deref(),
        ostt_config.transcription.model.as_deref(),
    ) {
        (Some(provider_id), Some(model_id)) => Ok(SelectedModel {
            provider_id: provider_id.to_string(),
            model_id: model_id.to_string(),
        }),
        _ => Err(anyhow::anyhow!(
            "No model selected. Please run 'ostt auth' to select a transcription model"
        )),
    }
}

fn config_for_selected_model(
    ostt_config: &OsttConfig,
    selected_model: &SelectedModel,
    keywords: Vec<String>,
    param_overrides: &[String],
) -> anyhow::Result<TranscriptionConfig> {
    let api_key = config::get_api_key(&selected_model.provider_id)?;
    let mut params = ostt_config
        .provider_configs
        .get(&selected_model.provider_id)
        .map(|provider| provider.params.clone())
        .unwrap_or_default();
    if let Some(model_config) = ostt_config
        .provider_configs
        .get(&selected_model.provider_id)
        .and_then(|provider| provider.models.get(&selected_model.model_id))
    {
        params.extend(model_config.params.clone());
    }
    params.extend(parse_param_overrides(selected_model, param_overrides)?);
    config::file::validate_params_for_model(
        &selected_model.provider_id,
        &selected_model.model_id,
        &params,
    )?;

    super::config_for_selected_model(selected_model, api_key, keywords, params)
}

fn parse_param_overrides(
    selected_model: &SelectedModel,
    overrides: &[String],
) -> anyhow::Result<IndexMap<String, config::ModelOptionValue>> {
    let schema = super::api::option_schema(&selected_model.provider_id, &selected_model.model_id)
        .ok_or_else(|| anyhow::anyhow!("No params are supported for this model"))?;
    let full_model_id = format!("{}/{}", selected_model.provider_id, selected_model.model_id);
    let mut parsed = IndexMap::new();

    for override_value in overrides {
        let (name, raw_value) = override_value.split_once('=').ok_or_else(|| {
            anyhow::anyhow!("Invalid --param '{}'. Use key=value.", override_value)
        })?;
        if parsed.contains_key(name) {
            anyhow::bail!("Duplicate --param '{}'.", name);
        }
        let Some(spec) = schema.option(name) else {
            anyhow::bail!(
                "Invalid param '{}' for '{}'. Supported params: {}.",
                name,
                full_model_id,
                schema.option_names().join(", ")
            );
        };
        parsed.insert(
            name.to_string(),
            parse_model_option_value(raw_value, spec.kind)?,
        );
    }

    Ok(parsed)
}

fn parse_model_option_value(
    raw_value: &str,
    kind: super::model::ModelOptionKind,
) -> anyhow::Result<config::ModelOptionValue> {
    Ok(match kind {
        super::model::ModelOptionKind::Bool => {
            config::ModelOptionValue::Bool(raw_value.parse::<bool>()?)
        }
        super::model::ModelOptionKind::BoolOrString => match raw_value.parse::<bool>() {
            Ok(value) => config::ModelOptionValue::Bool(value),
            Err(_) => config::ModelOptionValue::String(raw_value.to_string()),
        },
        super::model::ModelOptionKind::BoolOrStringList => match raw_value.parse::<bool>() {
            Ok(value) => config::ModelOptionValue::Bool(value),
            Err(_) => config::ModelOptionValue::StringList(
                raw_value
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string)
                    .collect(),
            ),
        },
        super::model::ModelOptionKind::Integer => {
            config::ModelOptionValue::Integer(raw_value.parse::<i64>()?)
        }
        super::model::ModelOptionKind::Number => {
            config::ModelOptionValue::Number(raw_value.parse::<f64>()?)
        }
        super::model::ModelOptionKind::String => {
            if raw_value.is_empty() {
                anyhow::bail!("Value must not be empty for string option");
            }
            config::ModelOptionValue::String(raw_value.to_string())
        }
        super::model::ModelOptionKind::StringOrStringList => {
            if raw_value.is_empty() {
                anyhow::bail!("Value must not be empty for string option");
            }
            if raw_value.contains(',') {
                config::ModelOptionValue::StringList(parse_comma_separated_strings(raw_value))
            } else {
                config::ModelOptionValue::String(raw_value.to_string())
            }
        }
        super::model::ModelOptionKind::StringList => {
            config::ModelOptionValue::StringList(parse_comma_separated_strings(raw_value))
        }
    })
}

fn parse_comma_separated_strings(raw_value: &str) -> Vec<String> {
    raw_value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selected_model(provider_id: &str, model_id: &str) -> SelectedModel {
        SelectedModel {
            provider_id: provider_id.to_string(),
            model_id: model_id.to_string(),
        }
    }

    #[test]
    fn param_overrides_reject_duplicate_keys() {
        let model = selected_model("deepgram", "nova-3");
        let err = parse_param_overrides(
            &model,
            &["language=sv".to_string(), "language=en".to_string()],
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("Duplicate --param 'language'"));
    }

    #[test]
    fn param_overrides_reject_missing_equals() {
        let model = selected_model("deepgram", "nova-3");
        let err = parse_param_overrides(&model, &["diarize".to_string()])
            .unwrap_err()
            .to_string();

        assert!(err.contains("Invalid --param 'diarize'"));
        assert!(err.contains("key=value"));
    }

    #[test]
    fn param_overrides_reject_unknown_param() {
        let model = selected_model("openai", "gpt-4o-transcribe");
        let err = parse_param_overrides(&model, &["smart_format=true".to_string()])
            .unwrap_err()
            .to_string();

        assert!(err.contains("Invalid param 'smart_format'"));
        assert!(err.contains("Supported params"));
    }

    #[test]
    fn param_overrides_reject_invalid_bool_and_number_values() {
        let bool_model = selected_model("deepgram", "nova-3");
        let err = parse_param_overrides(&bool_model, &["diarize=yes".to_string()])
            .unwrap_err()
            .to_string();
        assert!(err.contains("provided string was not `true` or `false`"));

        let number_model = selected_model("openai", "gpt-4o-transcribe");
        let err = parse_param_overrides(&number_model, &["temperature=hot".to_string()])
            .unwrap_err()
            .to_string();
        assert!(err.contains("invalid float literal"));
    }

    #[test]
    fn param_overrides_reject_empty_string_for_string_typed_option() {
        let model = selected_model("openai", "gpt-4o-transcribe");
        let err = parse_param_overrides(&model, &["prompt=".to_string()])
            .unwrap_err()
            .to_string();
        assert!(err.contains("must not be empty"));
    }

    #[test]
    fn param_overrides_parse_whisper_params() {
        let model = selected_model("whisper", "turbo");
        let options = parse_param_overrides(
            &model,
            &[
                "language=sv".to_string(),
                "no_context=false".to_string(),
                "temperature=0.2".to_string(),
            ],
        )
        .unwrap();

        assert_eq!(
            options.get("language"),
            Some(&config::ModelOptionValue::String("sv".to_string()))
        );
        assert_eq!(
            options.get("no_context"),
            Some(&config::ModelOptionValue::Bool(false))
        );
        assert_eq!(
            options.get("temperature"),
            Some(&config::ModelOptionValue::Number(0.2))
        );
    }

    #[test]
    fn param_overrides_parse_bool_or_string_list_bool() {
        let model = selected_model("deepgram", "nova-3");
        let options =
            parse_param_overrides(&model, &["detect_language=false".to_string()]).unwrap();

        assert_eq!(
            options.get("detect_language"),
            Some(&config::ModelOptionValue::Bool(false))
        );
    }

    #[test]
    fn param_overrides_parse_string_lists() {
        let model = selected_model("deepgram", "nova-3");
        let options = parse_param_overrides(&model, &["keyterm=OSTT,whisper".to_string()]).unwrap();

        assert_eq!(
            options.get("keyterm"),
            Some(&config::ModelOptionValue::StringList(vec![
                "OSTT".to_string(),
                "whisper".to_string()
            ]))
        );
    }

    #[test]
    fn param_overrides_parse_bool_or_string_lists() {
        let model = selected_model("deepgram", "nova-3");
        let options =
            parse_param_overrides(&model, &["detect_language=sv,en".to_string()]).unwrap();

        assert_eq!(
            options.get("detect_language"),
            Some(&config::ModelOptionValue::StringList(vec![
                "sv".to_string(),
                "en".to_string()
            ]))
        );
    }
}
