use super::configuration::{self, SpeakerDiarizationConfig};
use super::models::{self, SpeakerModelStatus};
use tauri::{AppHandle, Runtime};

#[tauri::command]
pub fn speaker_diarization_get_config<R: Runtime>(
    app: AppHandle<R>,
) -> Result<SpeakerDiarizationConfig, String> {
    configuration::load_config(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn speaker_diarization_save_config<R: Runtime>(
    app: AppHandle<R>,
    config: SpeakerDiarizationConfig,
) -> Result<(), String> {
    configuration::save_config(&app, &config).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn speaker_diarization_get_status<R: Runtime>(
    app: AppHandle<R>,
) -> Result<SpeakerModelStatus, String> {
    tokio::task::spawn_blocking(move || models::get_status(&app).map_err(|error| error.to_string()))
        .await
        .map_err(|error| format!("Speaker model scan failed: {error}"))?
}

#[tauri::command]
pub async fn speaker_diarization_download_model<R: Runtime>(
    app: AppHandle<R>,
) -> Result<(), String> {
    models::download_model(app)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn speaker_diarization_delete_model<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    if configuration::is_enabled(&app) {
        return Err(
            "Speaker diarization is enabled. Disable it before deleting the active model."
                .to_string(),
        );
    }
    models::delete_model(&app)
        .await
        .map_err(|error| error.to_string())
}
