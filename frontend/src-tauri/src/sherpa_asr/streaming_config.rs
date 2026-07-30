use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "provider-settings.json";
const STORE_VERSION: u64 = 1;
const STORE_VERSION_KEY: &str = "version";
const STORE_CONFIG_KEY: &str = "streamingTranscription";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StreamingTranscriptionConfig {
    pub enabled: bool,
    pub provider: String,
    pub model: String,
}

impl Default for StreamingTranscriptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: super::models::PROVIDER_ID.to_string(),
            model: super::models::PARAFORMER_ONLINE_MODEL_ID.to_string(),
        }
    }
}

impl StreamingTranscriptionConfig {
    fn validate(&self) -> Result<()> {
        if self.provider != super::models::PROVIDER_ID {
            return Err(anyhow!(
                "Unsupported streaming transcription provider: {}",
                self.provider
            ));
        }
        if !super::online::is_online_model(&self.model) {
            return Err(anyhow!(
                "Model '{}' does not support continuous streaming transcription",
                self.model
            ));
        }
        Ok(())
    }
}

pub fn load_config_if_present<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<Option<StreamingTranscriptionConfig>> {
    let store = app
        .store(STORE_FILE)
        .map_err(|error| anyhow!("Failed to access provider settings: {error}"))?;

    let Some(value) = store.get(STORE_CONFIG_KEY) else {
        return Ok(None);
    };

    let config = serde_json::from_value::<StreamingTranscriptionConfig>(value.clone())
        .map_err(|error| anyhow!("Failed to read streaming transcription settings: {error}"))?;
    config.validate()?;
    Ok(Some(config))
}

pub fn load_config<R: Runtime>(app: &AppHandle<R>) -> Result<StreamingTranscriptionConfig> {
    Ok(load_config_if_present(app)?.unwrap_or_default())
}

pub fn save_config<R: Runtime>(
    app: &AppHandle<R>,
    config: &StreamingTranscriptionConfig,
) -> Result<()> {
    config.validate()?;
    if config.enabled && super::models::installed_model(app, &config.model)?.is_none() {
        return Err(anyhow!(
            "Streaming transcription model '{}' is missing or damaged",
            config.model
        ));
    }

    let store = app
        .store(STORE_FILE)
        .map_err(|error| anyhow!("Failed to access provider settings: {error}"))?;
    let config_value = serde_json::to_value(config).map_err(|error| {
        anyhow!("Failed to serialize streaming transcription settings: {error}")
    })?;

    store.set(STORE_VERSION_KEY, serde_json::json!(STORE_VERSION));
    store.set(STORE_CONFIG_KEY, config_value);
    store
        .save()
        .map_err(|error| anyhow!("Failed to save provider settings: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_is_opt_in_by_default() {
        assert_eq!(
            StreamingTranscriptionConfig::default(),
            StreamingTranscriptionConfig {
                enabled: false,
                provider: "sherpa-onnx".to_string(),
                model: "paraformer-online-zh-en-int8".to_string(),
            }
        );
    }

    #[test]
    fn config_uses_frontend_camel_case_contract() {
        let value = serde_json::to_value(StreamingTranscriptionConfig::default()).unwrap();
        assert_eq!(value["enabled"], false);
        assert_eq!(value["provider"], "sherpa-onnx");
        assert_eq!(value["model"], "paraformer-online-zh-en-int8");
    }

    #[test]
    fn non_streaming_models_are_rejected() {
        let mut config = StreamingTranscriptionConfig::default();
        config.model = super::super::models::SENSEVOICE_MODEL_ID.to_string();
        assert!(config.validate().is_err());
    }
}
