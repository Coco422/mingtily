use super::models::{self, PunctuationModelStatus};
use tauri::{AppHandle, Runtime};

#[tauri::command]
pub fn punctuation_get_status<R: Runtime>(
    app: AppHandle<R>,
) -> Result<PunctuationModelStatus, String> {
    models::status(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn punctuation_download_model<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    models::download_model(&app)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn punctuation_delete_model<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    models::delete_model(&app)
        .await
        .map_err(|error| error.to_string())
}
