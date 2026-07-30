use super::models::{self, SherpaAsrModelStatus, PROVIDER_ID};
use super::streaming_config::{self, StreamingTranscriptionConfig};
use tauri::{AppHandle, Manager, Runtime};

#[tauri::command]
pub async fn sherpa_asr_get_streaming_config<R: Runtime>(
    app: AppHandle<R>,
) -> Result<StreamingTranscriptionConfig, String> {
    match streaming_config::load_config_if_present(&app).map_err(|error| error.to_string())? {
        Some(config) => Ok(config),
        None => {
            let mut config = StreamingTranscriptionConfig::default();
            if let Ok(Some(transcript_config)) =
                crate::api::api::api_get_transcript_config(app.clone(), app.clone().state(), None)
                    .await
            {
                if transcript_config.provider == PROVIDER_ID
                    && super::online::is_online_model(&transcript_config.model)
                {
                    config.enabled = true;
                    config.model = transcript_config.model;
                }
            }
            Ok(config)
        }
    }
}

#[tauri::command]
pub fn sherpa_asr_save_streaming_config<R: Runtime>(
    app: AppHandle<R>,
    config: StreamingTranscriptionConfig,
) -> Result<(), String> {
    streaming_config::save_config(&app, &config).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn sherpa_asr_list_models<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Vec<SherpaAsrModelStatus>, String> {
    models::list_status(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn sherpa_asr_download_model<R: Runtime>(
    app: AppHandle<R>,
    model_id: String,
) -> Result<(), String> {
    models::download_model(&app, &model_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn sherpa_asr_delete_model<R: Runtime>(
    app: AppHandle<R>,
    model_id: String,
) -> Result<(), String> {
    if let Ok(Some(config)) =
        crate::api::api::api_get_transcript_config(app.clone(), app.clone().state(), None).await
    {
        if config.provider == PROVIDER_ID && config.model == model_id {
            return Err(
                "This model is currently selected for transcription. Select another model before deleting it."
                    .to_string(),
            );
        }
    }

    if let Ok(Some(config)) = streaming_config::load_config_if_present(&app) {
        if config.enabled && config.provider == PROVIDER_ID && config.model == model_id {
            return Err(
                "This model is currently selected for Beta live transcription. Disable the mode or select another streaming model before deleting it."
                    .to_string(),
            );
        }
    }

    models::delete_model(&app, &model_id)
        .await
        .map_err(|error| error.to_string())
}
