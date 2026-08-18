use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "provider-settings.json";
const STORE_VERSION: u64 = 1;
const STORE_VERSION_KEY: &str = "version";
const STORE_CONFIG_KEY: &str = "summaryRuntime";

pub const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30 * 60;
pub const MIN_REQUEST_TIMEOUT_SECS: u64 = 5 * 60;
pub const MAX_REQUEST_TIMEOUT_SECS: u64 = 24 * 60 * 60;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SummaryRuntimeConfig {
    pub request_timeout_secs: u64,
}

impl Default for SummaryRuntimeConfig {
    fn default() -> Self {
        Self {
            request_timeout_secs: DEFAULT_REQUEST_TIMEOUT_SECS,
        }
    }
}

impl SummaryRuntimeConfig {
    pub fn validate(&self) -> Result<()> {
        if !(MIN_REQUEST_TIMEOUT_SECS..=MAX_REQUEST_TIMEOUT_SECS)
            .contains(&self.request_timeout_secs)
            || self.request_timeout_secs % 60 != 0
        {
            return Err(anyhow!(
                "Summary request timeout must be a whole number of minutes between {} and {} seconds",
                MIN_REQUEST_TIMEOUT_SECS,
                MAX_REQUEST_TIMEOUT_SECS
            ));
        }
        Ok(())
    }

    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request_timeout_secs)
    }
}

pub fn load_config<R: Runtime>(app: &AppHandle<R>) -> Result<SummaryRuntimeConfig> {
    let store = app
        .store(STORE_FILE)
        .map_err(|error| anyhow!("Failed to access provider settings: {error}"))?;
    let Some(value) = store.get(STORE_CONFIG_KEY) else {
        return Ok(SummaryRuntimeConfig::default());
    };

    let config = serde_json::from_value::<SummaryRuntimeConfig>(value.clone())
        .map_err(|error| anyhow!("Failed to read summary runtime settings: {error}"))?;
    config.validate()?;
    Ok(config)
}

pub fn save_config<R: Runtime>(
    app: &AppHandle<R>,
    config: SummaryRuntimeConfig,
) -> Result<SummaryRuntimeConfig> {
    config.validate()?;
    let store = app
        .store(STORE_FILE)
        .map_err(|error| anyhow!("Failed to access provider settings: {error}"))?;
    store.set(STORE_VERSION_KEY, serde_json::json!(STORE_VERSION));
    store.set(
        STORE_CONFIG_KEY,
        serde_json::to_value(config)
            .map_err(|error| anyhow!("Failed to serialize summary runtime settings: {error}"))?,
    );
    store
        .save()
        .map_err(|error| anyhow!("Failed to save provider settings: {error}"))?;
    Ok(config)
}

#[tauri::command]
pub fn api_get_summary_runtime_config<R: Runtime>(
    app: AppHandle<R>,
) -> Result<SummaryRuntimeConfig, String> {
    load_config(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn api_save_summary_runtime_config<R: Runtime>(
    app: AppHandle<R>,
    request_timeout_secs: u64,
) -> Result<SummaryRuntimeConfig, String> {
    save_config(
        &app,
        SummaryRuntimeConfig {
            request_timeout_secs,
        },
    )
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_allows_long_prefill() {
        assert_eq!(SummaryRuntimeConfig::default().request_timeout_secs, 1800);
    }

    #[test]
    fn config_uses_frontend_camel_case_contract() {
        let value = serde_json::to_value(SummaryRuntimeConfig::default()).unwrap();
        assert_eq!(value["requestTimeoutSecs"], 1800);
    }

    #[test]
    fn timeout_bounds_are_enforced() {
        assert!(SummaryRuntimeConfig {
            request_timeout_secs: MIN_REQUEST_TIMEOUT_SECS
        }
        .validate()
        .is_ok());
        assert!(SummaryRuntimeConfig {
            request_timeout_secs: MAX_REQUEST_TIMEOUT_SECS
        }
        .validate()
        .is_ok());
        assert!(SummaryRuntimeConfig {
            request_timeout_secs: MIN_REQUEST_TIMEOUT_SECS - 1
        }
        .validate()
        .is_err());
        assert!(SummaryRuntimeConfig {
            request_timeout_secs: MAX_REQUEST_TIMEOUT_SECS + 1
        }
        .validate()
        .is_err());
        assert!(SummaryRuntimeConfig {
            request_timeout_secs: MIN_REQUEST_TIMEOUT_SECS + 1
        }
        .validate()
        .is_err());
    }
}
