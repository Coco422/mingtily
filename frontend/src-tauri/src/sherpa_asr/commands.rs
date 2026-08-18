use super::models::{self, SherpaAsrModelStatus, PROVIDER_ID};
use super::streaming_config::{self, StreamingTranscriptionConfig};
use super::{enhancement, SherpaAsrEnhancementConfig, TerminologyConfig};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_dialog::DialogExt;

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
pub fn sherpa_asr_get_enhancement_config<R: Runtime>(
    app: AppHandle<R>,
) -> Result<SherpaAsrEnhancementConfig, String> {
    enhancement::load_config(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn sherpa_asr_save_enhancement_config<R: Runtime>(
    app: AppHandle<R>,
    config: SherpaAsrEnhancementConfig,
) -> Result<SherpaAsrEnhancementConfig, String> {
    enhancement::save_config(&app, config).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn terminology_get_config<R: Runtime>(app: AppHandle<R>) -> Result<TerminologyConfig, String> {
    enhancement::load_terminology_config(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn terminology_save_config<R: Runtime>(
    app: AppHandle<R>,
    config: TerminologyConfig,
) -> Result<TerminologyConfig, String> {
    enhancement::save_terminology_config(&app, config).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn sherpa_asr_get_homophone_status<R: Runtime>(
    app: AppHandle<R>,
) -> Result<enhancement::HomophoneReplacerStatus, String> {
    enhancement::status(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn sherpa_asr_download_homophone_lexicon<R: Runtime>(
    app: AppHandle<R>,
) -> Result<(), String> {
    enhancement::download_lexicon(&app)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn sherpa_asr_delete_homophone_lexicon<R: Runtime>(
    app: AppHandle<R>,
) -> Result<(), String> {
    enhancement::delete_lexicon(&app)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn sherpa_asr_import_homophone_rules<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Vec<enhancement::HomophoneRuleStatus>, String> {
    let app_for_dialog = app.clone();
    let selected = tokio::task::spawn_blocking(move || {
        app_for_dialog
            .dialog()
            .file()
            .add_filter("Sherpa homophone rule", &["fst"])
            .blocking_pick_files()
    })
    .await
    .map_err(|error| format!("Homophone rule dialog failed: {error}"))?;

    let Some(selected) = selected else {
        return enhancement::status(&app)
            .map(|status| status.rules)
            .map_err(|error| error.to_string());
    };
    let paths = selected
        .into_iter()
        .map(|path| path.into_path().map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    enhancement::import_rule_files(&app, paths).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn sherpa_asr_delete_homophone_rule<R: Runtime>(
    app: AppHandle<R>,
    rule_id: String,
) -> Result<Vec<enhancement::HomophoneRuleStatus>, String> {
    enhancement::delete_rule(&app, &rule_id).map_err(|error| error.to_string())
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
pub async fn sherpa_asr_import_model_archive<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Option<models::ImportedSherpaModel>, String> {
    let app_for_dialog = app.clone();
    let selected = tokio::task::spawn_blocking(move || {
        app_for_dialog
            .dialog()
            .file()
            .add_filter("Model archive", &["zip", "bz2"])
            .blocking_pick_file()
    })
    .await
    .map_err(|error| format!("Model archive dialog failed: {error}"))?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected.into_path().map_err(|error| error.to_string())?;
    models::import_archive(&app, &path)
        .await
        .map(Some)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn sherpa_asr_import_model_directory<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Option<models::ImportedSherpaModel>, String> {
    let app_for_dialog = app.clone();
    let selected =
        tokio::task::spawn_blocking(move || app_for_dialog.dialog().file().blocking_pick_folder())
            .await
            .map_err(|error| format!("Model directory dialog failed: {error}"))?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected.into_path().map_err(|error| error.to_string())?;
    models::import_directory(&app, &path)
        .await
        .map(Some)
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
