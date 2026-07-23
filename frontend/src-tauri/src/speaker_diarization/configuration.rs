use anyhow::{anyhow, Result};
use log::warn;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "provider-settings.json";
const STORE_VERSION: u64 = 1;
const STORE_VERSION_KEY: &str = "version";
const STORE_CONFIG_KEY: &str = "speakerDiarization";

pub const PROVIDER_ID: &str = "sherpa-onnx";
pub const MODEL_ID: &str = super::models::MODEL_ID;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerDiarizationConfig {
    pub enabled: bool,
    pub provider: String,
    pub model: String,
}

impl Default for SpeakerDiarizationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            provider: PROVIDER_ID.to_string(),
            model: MODEL_ID.to_string(),
        }
    }
}

impl SpeakerDiarizationConfig {
    fn validate(&self) -> Result<()> {
        if self.provider != PROVIDER_ID {
            return Err(anyhow!(
                "Unsupported speaker diarization provider: {}",
                self.provider
            ));
        }
        if self.model != MODEL_ID {
            return Err(anyhow!(
                "Unsupported speaker diarization model: {}",
                self.model
            ));
        }
        Ok(())
    }
}

pub fn load_config<R: Runtime>(app: &AppHandle<R>) -> Result<SpeakerDiarizationConfig> {
    let store = app
        .store(STORE_FILE)
        .map_err(|error| anyhow!("Failed to access provider settings: {error}"))?;

    let Some(value) = store.get(STORE_CONFIG_KEY) else {
        return Ok(SpeakerDiarizationConfig::default());
    };

    let config = serde_json::from_value::<SpeakerDiarizationConfig>(value.clone())
        .map_err(|error| anyhow!("Failed to read speaker diarization settings: {error}"))?;
    config.validate()?;
    Ok(config)
}

pub fn save_config<R: Runtime>(
    app: &AppHandle<R>,
    config: &SpeakerDiarizationConfig,
) -> Result<()> {
    config.validate()?;

    let store = app
        .store(STORE_FILE)
        .map_err(|error| anyhow!("Failed to access provider settings: {error}"))?;
    let config_value = serde_json::to_value(config)
        .map_err(|error| anyhow!("Failed to serialize speaker diarization settings: {error}"))?;

    store.set(STORE_VERSION_KEY, serde_json::json!(STORE_VERSION));
    store.set(STORE_CONFIG_KEY, config_value);
    store
        .save()
        .map_err(|error| anyhow!("Failed to save provider settings: {error}"))?;
    Ok(())
}

/// Runtime checks fail open to preserve the pre-0.5 behavior when the store cannot be read.
pub fn is_enabled<R: Runtime>(app: &AppHandle<R>) -> bool {
    match load_config(app) {
        Ok(config) => config.enabled,
        Err(error) => {
            warn!(
                "Unable to read speaker diarization settings; keeping diarization enabled: {}",
                error
            );
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_preserve_existing_runtime_behavior() {
        assert_eq!(
            SpeakerDiarizationConfig::default(),
            SpeakerDiarizationConfig {
                enabled: true,
                provider: "sherpa-onnx".to_string(),
                model: "sherpa-v1".to_string(),
            }
        );
    }

    #[test]
    fn config_uses_frontend_camel_case_contract() {
        let value = serde_json::to_value(SpeakerDiarizationConfig::default()).unwrap();
        assert_eq!(value["enabled"], true);
        assert_eq!(value["provider"], "sherpa-onnx");
        assert_eq!(value["model"], "sherpa-v1");
    }

    #[test]
    fn unsupported_provider_or_model_is_rejected() {
        let mut config = SpeakerDiarizationConfig::default();
        config.provider = "unknown".to_string();
        assert!(config.validate().is_err());

        let mut config = SpeakerDiarizationConfig::default();
        config.model = "unknown".to_string();
        assert!(config.validate().is_err());
    }
}
