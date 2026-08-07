use super::{ParakeetProvider, TranscriptionProvider, WhisperProvider};
use crate::config::{DEFAULT_PARAKEET_MODEL, DEFAULT_WHISPER_MODEL};
use log::{info, warn};
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime};

pub const WHISPER_PROVIDER_ID: &str = "localWhisper";
pub const PARAKEET_PROVIDER_ID: &str = "parakeet";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptionSelection {
    pub provider: String,
    pub model: String,
}

pub async fn validate_transcription_model_ready<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<(), String> {
    let selection = resolve_transcription_selection(app, None, None).await;
    if selection.provider == crate::sherpa_asr::PROVIDER_ID
        && crate::sherpa_asr::is_online_model(&selection.model)
    {
        crate::sherpa_asr::installed_model(app, &selection.model)
            .map_err(|error| error.to_string())?
            .map(|_| ())
            .ok_or_else(|| {
                format!(
                    "Sherpa ONNX model '{}' is missing or damaged. Download or repair it in Models.",
                    selection.model
                )
            })?;
    } else {
        let provider = load_transcription_provider(app, None, None).await?;
        if !provider.is_model_loaded().await {
            return Err("The selected transcription model is not ready".to_string());
        }
    }

    resolve_configured_streaming_model(app).await?;
    Ok(())
}

pub async fn resolve_configured_streaming_model<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<Option<crate::sherpa_asr::models::InstalledSherpaModel>, String> {
    let selection = resolve_transcription_selection(app, None, None).await;
    match crate::sherpa_asr::streaming_config::load_config_if_present(app)
        .map_err(|error| error.to_string())?
    {
        Some(config) => {
            if !config.enabled {
                return Ok(None);
            }
            if selection.provider == config.provider && selection.model == config.model {
                return Err(
                    "Beta live transcription requires a separate finalized model. Select an offline finalized model in Services."
                        .to_string(),
                );
            }
            crate::sherpa_asr::installed_model(app, &config.model)
                .map_err(|error| error.to_string())?
                .map(Some)
                .ok_or_else(|| {
                    format!(
                        "Streaming transcription model '{}' is missing or damaged. Download or repair it in Models.",
                        config.model
                    )
                })
        }
        None => {
            // v0.6 compatibility: an Online Paraformer selected as the only ASR model
            // keeps its previous live + finalized behavior until the new strategy is saved.
            if selection.provider != crate::sherpa_asr::PROVIDER_ID
                || !crate::sherpa_asr::is_online_model(&selection.model)
            {
                return Ok(None);
            }
            crate::sherpa_asr::installed_model(app, &selection.model)
                .map_err(|error| error.to_string())
        }
    }
}

pub async fn get_or_init_transcription_engine<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<Arc<dyn TranscriptionProvider>, String> {
    load_transcription_provider(app, None, None).await
}

pub async fn load_transcription_provider<R: Runtime>(
    app: &AppHandle<R>,
    requested_provider: Option<&str>,
    requested_model: Option<&str>,
) -> Result<Arc<dyn TranscriptionProvider>, String> {
    let selection = resolve_transcription_selection(app, requested_provider, requested_model).await;
    info!(
        "Loading transcription provider '{}' with model '{}'",
        selection.provider, selection.model
    );

    match selection.provider.as_str() {
        WHISPER_PROVIDER_ID | "whisper" => {
            let engine = load_whisper_model(&selection.model).await?;
            Ok(Arc::new(WhisperProvider::new(engine)))
        }
        PARAKEET_PROVIDER_ID => {
            let engine = load_parakeet_model(&selection.model).await?;
            Ok(Arc::new(ParakeetProvider::new(engine)))
        }
        crate::sherpa_asr::PROVIDER_ID => {
            let installed = crate::sherpa_asr::installed_model(app, &selection.model)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| {
                    format!(
                        "Sherpa ONNX model '{}' is missing or damaged. Download or repair it in Models.",
                        selection.model
                    )
                })?;
            let enhancements = match crate::sherpa_asr::enhancement::resolve_runtime(app) {
                Ok(enhancements) => enhancements,
                Err(error) => {
                    warn!(
                        "Unable to load Sherpa ASR enhancement settings; continuing without terminology enhancement: {}",
                        error
                    );
                    crate::sherpa_asr::RuntimeEnhancements::default()
                }
            };
            let provider: Arc<dyn TranscriptionProvider> =
                if crate::sherpa_asr::is_online_model(&selection.model) {
                    Arc::new(crate::sherpa_asr::SherpaOnlineAsrProvider::new(installed))
                } else {
                    Arc::new(crate::sherpa_asr::SherpaOfflineAsrProvider::new(
                        installed,
                        enhancements,
                    ))
                };
            if selection.model == crate::sherpa_asr::models::SENSEVOICE_MODEL_ID {
                Ok(crate::punctuation::wrap_if_available(app, provider))
            } else {
                Ok(provider)
            }
        }
        other => Err(format!(
            "Provider '{other}' is not supported for local transcription. Select Whisper, Parakeet, or Sherpa ONNX."
        )),
    }
}

pub async fn resolve_transcription_selection<R: Runtime>(
    app: &AppHandle<R>,
    requested_provider: Option<&str>,
    requested_model: Option<&str>,
) -> TranscriptionSelection {
    let configured = configured_transcription_selection(app).await;
    let provider = requested_provider
        .filter(|provider| !provider.trim().is_empty())
        .map(normalize_provider_id)
        .unwrap_or_else(|| configured.provider.clone());
    let model = requested_model
        .filter(|model| !model.trim().is_empty())
        .map(str::to_string)
        .or_else(|| {
            (provider == configured.provider && !configured.model.is_empty())
                .then_some(configured.model)
        })
        .unwrap_or_else(|| default_model_for_provider(&provider).to_string());

    TranscriptionSelection { provider, model }
}

async fn configured_transcription_selection<R: Runtime>(
    app: &AppHandle<R>,
) -> TranscriptionSelection {
    match crate::api::api::api_get_transcript_config(app.clone(), app.clone().state(), None).await {
        Ok(Some(config)) => TranscriptionSelection {
            provider: normalize_provider_id(&config.provider),
            model: config.model,
        },
        Ok(None) => {
            info!("No transcription configuration found; using the Parakeet default");
            default_selection()
        }
        Err(error) => {
            warn!(
                "Unable to read transcription configuration; using the Parakeet default: {}",
                error
            );
            default_selection()
        }
    }
}

fn default_selection() -> TranscriptionSelection {
    TranscriptionSelection {
        provider: PARAKEET_PROVIDER_ID.to_string(),
        model: DEFAULT_PARAKEET_MODEL.to_string(),
    }
}

fn normalize_provider_id(provider: &str) -> String {
    match provider {
        "whisper" => WHISPER_PROVIDER_ID.to_string(),
        other => other.to_string(),
    }
}

fn default_model_for_provider(provider: &str) -> &'static str {
    match provider {
        WHISPER_PROVIDER_ID | "whisper" => DEFAULT_WHISPER_MODEL,
        crate::sherpa_asr::PROVIDER_ID => crate::sherpa_asr::models::SENSEVOICE_MODEL_ID,
        _ => DEFAULT_PARAKEET_MODEL,
    }
}

async fn load_whisper_model(
    target_model: &str,
) -> Result<Arc<crate::whisper_engine::WhisperEngine>, String> {
    crate::whisper_engine::commands::whisper_init()
        .await
        .map_err(|error| format!("Failed to initialize Whisper: {error}"))?;
    let engine = {
        let guard = crate::whisper_engine::commands::WHISPER_ENGINE
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        guard
            .as_ref()
            .cloned()
            .ok_or_else(|| "Whisper engine was not initialized".to_string())?
    };

    if engine.get_current_model().await.as_deref() != Some(target_model) {
        engine
            .discover_models()
            .await
            .map_err(|error| format!("Failed to inspect Whisper models: {error}"))?;
        engine
            .load_model(target_model)
            .await
            .map_err(|error| format!("Failed to load Whisper model '{target_model}': {error}"))?;
    }
    Ok(engine)
}

async fn load_parakeet_model(
    target_model: &str,
) -> Result<Arc<crate::parakeet_engine::ParakeetEngine>, String> {
    crate::parakeet_engine::commands::parakeet_init()
        .await
        .map_err(|error| format!("Failed to initialize Parakeet: {error}"))?;
    let engine = {
        let guard = crate::parakeet_engine::commands::PARAKEET_ENGINE
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        guard
            .as_ref()
            .cloned()
            .ok_or_else(|| "Parakeet engine was not initialized".to_string())?
    };

    if engine.get_current_model().await.as_deref() != Some(target_model) {
        engine
            .discover_models()
            .await
            .map_err(|error| format!("Failed to inspect Parakeet models: {error}"))?;
        engine
            .load_model(target_model)
            .await
            .map_err(|error| format!("Failed to load Parakeet model '{target_model}': {error}"))?;
    }
    Ok(engine)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_aliases_are_normalized() {
        assert_eq!(normalize_provider_id("whisper"), WHISPER_PROVIDER_ID);
        assert_eq!(normalize_provider_id("sherpa-onnx"), "sherpa-onnx");
    }

    #[test]
    fn sherpa_defaults_to_sense_voice() {
        assert_eq!(
            default_model_for_provider(crate::sherpa_asr::PROVIDER_ID),
            crate::sherpa_asr::models::SENSEVOICE_MODEL_ID
        );
    }
}
