use super::models::{self, SherpaAsrModelStatus, PROVIDER_ID};
use tauri::{AppHandle, Manager, Runtime};

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

    models::delete_model(&app, &model_id)
        .await
        .map_err(|error| error.to_string())
}
