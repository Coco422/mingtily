use anyhow::{Context, Result};
use chrono::Local;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_dialog::DialogExt;

pub const LOG_FILE_SIZE_BYTES: u128 = 5 * 1024 * 1024;
pub const LOG_ARCHIVES_TO_KEEP: usize = 4;

#[derive(Debug, Serialize)]
pub struct DiagnosticExportResult {
    pub path: String,
    pub files_included: usize,
}

fn list_log_files(log_dir: &Path) -> Result<Vec<PathBuf>> {
    if !log_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = std::fs::read_dir(log_dir)
        .with_context(|| format!("Unable to read log directory: {}", log_dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("log"))
        .collect::<Vec<_>>();

    files.sort_by_key(|path| {
        std::fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .ok()
    });
    Ok(files)
}

fn sanitize_log_content(content: &str, home_dir: Option<&Path>) -> String {
    let home = home_dir.and_then(Path::to_str);
    content
        .lines()
        .map(|line| {
            let lowercase = line.to_ascii_lowercase();
            if lowercase.contains("authorization")
                || lowercase.contains("api_key")
                || lowercase.contains("api-key")
                || lowercase.contains("api key")
                || lowercase.contains("apikey")
                || lowercase.contains("x-api-key")
                || lowercase.contains("bearer ")
                || lowercase.contains("client_secret")
                || lowercase.contains("access_token")
                || lowercase.contains("auth token")
            {
                return "[redacted sensitive log line]".to_string();
            }

            match home {
                Some(home) if !home.is_empty() => line.replace(home, "$HOME"),
                _ => line.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_diagnostic_export(
    log_dir: &Path,
    app_version: &str,
    home_dir: Option<&Path>,
) -> Result<(String, usize)> {
    let log_files = list_log_files(log_dir)?;
    let mut output = format!(
        "Mingtily diagnostics\n\
         Generated: {}\n\
         Version: {}\n\
         Platform: {} {}\n\
         Privacy: created locally after an explicit user action; never uploaded by Mingtily.\n\
         Redaction: home-directory paths and obvious credential-bearing lines are removed.\n\n",
        Local::now().to_rfc3339(),
        app_version,
        std::env::consts::OS,
        std::env::consts::ARCH,
    );

    for path in &log_files {
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("unknown.log");
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Unable to read log file: {}", path.display()))?;
        output.push_str(&format!("===== {} =====\n", file_name));
        output.push_str(&sanitize_log_content(&content, home_dir));
        output.push_str("\n\n");
    }

    Ok((output, log_files.len()))
}

#[tauri::command]
pub async fn export_diagnostic_logs<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Option<DiagnosticExportResult>, String> {
    let log_dir = app
        .path()
        .app_log_dir()
        .map_err(|error| format!("Unable to resolve log directory: {error}"))?;
    let app_version = app.package_info().version.to_string();
    let home_dir = dirs::home_dir();
    let (content, files_included) = tokio::task::spawn_blocking(move || {
        build_diagnostic_export(&log_dir, &app_version, home_dir.as_deref())
    })
    .await
    .map_err(|error| format!("Diagnostic log collection failed: {error}"))?
    .map_err(|error| error.to_string())?;
    let default_name = format!(
        "mingtily-diagnostics-{}.txt",
        Local::now().format("%Y%m%d-%H%M%S")
    );

    let app_for_dialog = app.clone();
    let destination = tokio::task::spawn_blocking(move || {
        app_for_dialog
            .dialog()
            .file()
            .add_filter("Text", &["txt"])
            .set_file_name(default_name)
            .blocking_save_file()
    })
    .await
    .map_err(|error| format!("Diagnostic export dialog failed: {error}"))?;

    let Some(destination) = destination else {
        return Ok(None);
    };
    let destination = destination
        .into_path()
        .map_err(|error| format!("Invalid diagnostic export path: {error}"))?;
    let destination_for_write = destination.clone();
    tokio::task::spawn_blocking(move || std::fs::write(&destination_for_write, content))
        .await
        .map_err(|error| format!("Diagnostic export write task failed: {error}"))?
        .map_err(|error| format!("Unable to write diagnostic export: {error}"))?;

    Ok(Some(DiagnosticExportResult {
        path: destination.to_string_lossy().to_string(),
        files_included,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_export_redacts_home_and_obvious_credentials() {
        let content = "model path: /Users/example/models/a.onnx\nAuthorization: Bearer secret\napiKey=another-secret\nready";
        let sanitized = sanitize_log_content(content, Some(Path::new("/Users/example")));

        assert!(sanitized.contains("$HOME/models/a.onnx"));
        assert!(sanitized.contains("[redacted sensitive log line]"));
        assert!(!sanitized.contains("secret"));
    }

    #[test]
    fn diagnostic_export_orders_and_combines_log_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Mingtily.log"), "current").unwrap();
        std::fs::write(dir.path().join("ignored.txt"), "ignored").unwrap();

        let (export, count) = build_diagnostic_export(dir.path(), "0.5.2", None).unwrap();

        assert_eq!(count, 1);
        assert!(export.contains("Version: 0.5.2"));
        assert!(export.contains("===== Mingtily.log ====="));
        assert!(export.contains("current"));
        assert!(!export.contains("ignored"));
    }
}
