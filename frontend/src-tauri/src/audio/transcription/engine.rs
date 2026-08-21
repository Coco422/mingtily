use super::{ParakeetProvider, TranscriptionProvider, WhisperProvider};
use crate::config::{DEFAULT_PARAKEET_MODEL, DEFAULT_WHISPER_MODEL};
use log::{info, warn};
use std::sync::Arc;
use tauri::{AppHandle, Runtime};

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
    let selection = resolve_transcription_selection(app, None, None).await?;
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
    let resolved = crate::pipeline::resolve_loaded(app)
        .await
        .map_err(|error| error.to_string())?;
    let config = resolved.runtime_config();
    if config.live.mode != crate::pipeline::LiveMode::ContinuousPreview {
        return Ok(None);
    }
    let provider = config
        .live
        .streaming_provider
        .as_deref()
        .unwrap_or(crate::sherpa_asr::PROVIDER_ID);
    if provider != crate::sherpa_asr::PROVIDER_ID {
        return Err(format!(
            "Streaming Provider '{provider}' is not implemented by the current recording path"
        ));
    }
    let model = config
        .live
        .streaming_model
        .as_deref()
        .ok_or_else(|| "A streaming model is required".to_string())?;
    crate::sherpa_asr::installed_model(app, model)
        .map_err(|error| error.to_string())?
        .map(Some)
        .ok_or_else(|| {
            format!(
                "Streaming transcription model '{}' is missing or damaged. Download or repair it in Models.",
                model
            )
        })
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
    let resolved = resolve_requested_pipeline(app, requested_provider, requested_model).await?;
    load_resolved_transcription_provider(app, resolved).await
}

pub(crate) async fn load_transcription_provider_for_config<R: Runtime>(
    app: &AppHandle<R>,
    config: crate::pipeline::PipelineConfig,
    requested_provider: Option<&str>,
    requested_model: Option<&str>,
) -> Result<Arc<dyn TranscriptionProvider>, String> {
    let resolved =
        resolve_pipeline_request(app, config, requested_provider, requested_model).await?;
    load_resolved_transcription_provider(app, resolved).await
}

async fn load_resolved_transcription_provider<R: Runtime>(
    app: &AppHandle<R>,
    resolved: crate::pipeline::ResolvedPipeline,
) -> Result<Arc<dyn TranscriptionProvider>, String> {
    let runtime_config = resolved.runtime_config();
    let selection = TranscriptionSelection {
        provider: normalize_provider_id(&runtime_config.finalized.provider),
        model: runtime_config.finalized.model.clone(),
    };
    info!(
        "Loading transcription provider '{}' with model '{}'",
        selection.provider, selection.model
    );

    let mut terminology = crate::sherpa_asr::enhancement::load_terminology_config(app)
        .map_err(|error| format!("Failed to load terminology settings: {error}"))?;
    if runtime_config.enhancements.terminology == "off" {
        terminology = crate::sherpa_asr::enhancement::TerminologyConfig::default();
    }
    if selection.provider == crate::sherpa_asr::PROVIDER_ID
        && terminology.homophone_replacer_enabled
        && terminology.homophone_rule_fsts.len() > 1
    {
        return Err(
            "Multiple legacy homophone FST rules are selected. Open Custom terminology and choose one rule before transcribing."
                .to_string(),
        );
    }
    let provider: Arc<dyn TranscriptionProvider> = match selection.provider.as_str() {
        WHISPER_PROVIDER_ID | "whisper" => {
            let engine = load_whisper_model(&selection.model).await?;
            Arc::new(WhisperProvider::new(engine, terminology.terms.clone()))
        }
        PARAKEET_PROVIDER_ID => {
            let engine = load_parakeet_model(&selection.model, resolved.thread_count).await?;
            Arc::new(ParakeetProvider::new(engine))
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
                    Arc::new(crate::sherpa_asr::SherpaOfflineAsrProvider::new_with_threads(
                        installed,
                        enhancements,
                        resolved.thread_count,
                    ))
                };
            if resolved.punctuation_enabled {
                crate::punctuation::wrap_if_available(app, provider)
            } else {
                provider
            }
        }
        other => return Err(format!(
            "Provider '{other}' is not supported for local transcription. Select Whisper, Parakeet, or Sherpa ONNX."
        )),
    };
    Ok(super::terminology::TerminologyCorrectionProvider::wrap(
        provider,
        terminology.replacements,
    ))
}

pub async fn resolve_transcription_selection<R: Runtime>(
    app: &AppHandle<R>,
    requested_provider: Option<&str>,
    requested_model: Option<&str>,
) -> Result<TranscriptionSelection, String> {
    let resolved = resolve_requested_pipeline(app, requested_provider, requested_model).await?;
    let config = resolved.runtime_config();
    Ok(TranscriptionSelection {
        provider: normalize_provider_id(&config.finalized.provider),
        model: config.finalized.model.clone(),
    })
}

pub(crate) async fn resolve_requested_pipeline<R: Runtime>(
    app: &AppHandle<R>,
    requested_provider: Option<&str>,
    requested_model: Option<&str>,
) -> Result<crate::pipeline::ResolvedPipeline, String> {
    if requested_provider.is_none() && requested_model.is_none() {
        return crate::pipeline::resolve_loaded(app)
            .await
            .map_err(|error| error.to_string());
    }
    let config = crate::pipeline::initialize_from_legacy(app)
        .await
        .map_err(|error| error.to_string())?;
    resolve_pipeline_request(app, config, requested_provider, requested_model).await
}

async fn resolve_pipeline_request<R: Runtime>(
    app: &AppHandle<R>,
    mut config: crate::pipeline::PipelineConfig,
    requested_provider: Option<&str>,
    requested_model: Option<&str>,
) -> Result<crate::pipeline::ResolvedPipeline, String> {
    if let Some(provider) = requested_provider.filter(|provider| !provider.trim().is_empty()) {
        let provider = normalize_provider_id(provider);
        let provider_changed = provider != normalize_provider_id(&config.finalized.provider);
        config.finalized.provider = provider.clone();
        if provider_changed && requested_model.is_none() {
            config.finalized.model = default_model_for_provider(&provider).to_string();
        }
    }
    if let Some(model) = requested_model.filter(|model| !model.trim().is_empty()) {
        config.finalized.model = model.to_string();
    }
    crate::pipeline::resolve_for_app(app, config)
        .await
        .map_err(|error| error.to_string())
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
        PARAKEET_PROVIDER_ID => DEFAULT_PARAKEET_MODEL,
        crate::sherpa_asr::PROVIDER_ID => crate::sherpa_asr::models::SENSEVOICE_MODEL_ID,
        _ => crate::sherpa_asr::models::SENSEVOICE_MODEL_ID,
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
    num_threads: usize,
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
    }
    engine
        .load_model_with_threads(target_model, Some(num_threads))
        .await
        .map_err(|error| format!("Failed to load Parakeet model '{target_model}': {error}"))?;
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
