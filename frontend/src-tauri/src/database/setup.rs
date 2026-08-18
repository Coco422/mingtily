use log::{error, info};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use super::manager::{missing_migration_version, DatabaseManager};
use crate::state::AppState;

fn startup_error_copy(error: &sqlx::Error, is_zh_cn: bool) -> (&'static str, String) {
    if let Some(version) = missing_migration_version(error) {
        if is_zh_cn {
            return (
                "需要更新 Mingtily",
                format!(
                    "无法使用当前版本打开这份数据。\n\n这份 Mingtily 数据已由更新版本升级，请安装最新版本后重试。你的会议数据没有被修改。\n\n数据库迁移版本：{version}"
                ),
            );
        }
        return (
            "Update Mingtily",
            format!(
                "This version cannot open the current data.\n\nThe Mingtily data was upgraded by a newer version. Install the latest version and try again. Your meeting data was not changed.\n\nDatabase migration version: {version}"
            ),
        );
    }

    if is_zh_cn {
        (
            "Mingtily 启动失败",
            "无法打开本地数据库。你的会议数据没有被删除或重置。请安装最新版本后重试；如果问题仍然存在，请导出诊断日志。"
                .to_string(),
        )
    } else {
        (
            "Mingtily could not start",
            "The local database could not be opened. Your meeting data was not deleted or reset. Install the latest version and try again; if the problem continues, export the diagnostic logs."
                .to_string(),
        )
    }
}

pub fn show_database_startup_error(app: &AppHandle, startup_error: &sqlx::Error) {
    error!("Database startup failed: {startup_error}");
    let (title, message) = startup_error_copy(startup_error, crate::localization::is_zh_cn(app));

    if let Some(window) = app.get_webview_window("main") {
        if let Err(hide_error) = window.hide() {
            error!("Failed to hide the main window after database startup error: {hide_error}");
        }
    }

    let app_for_exit = app.clone();
    app.dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Error)
        .show(move |_| app_for_exit.exit(1));
}

/// Initialize database on app startup
/// Handles first launch detection and conditional initialization
pub async fn initialize_database_on_startup(app: &AppHandle) -> sqlx::Result<()> {
    // Check if this is the first launch (no database exists yet)
    let is_first_launch = DatabaseManager::is_first_launch(app).await?;

    if is_first_launch {
        info!("First launch detected - will notify window when ready");

        // Delay event emission to ensure window is ready and React listeners are registered
        let app_handle = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            app_handle
                .emit("first-launch-detected", ())
                .expect("Failed to emit first-launch-detected event");
            info!("Emitted first-launch-detected after delay");
        });
    } else {
        // Normal flow - initialize database immediately
        let db_manager = DatabaseManager::new_from_app_handle(app).await?;

        app.manage(AppState { db_manager });
        info!("Database initialized successfully");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_database_copy_is_actionable_and_does_not_suggest_resetting_data() {
        let error = sqlx::Error::Migrate(Box::new(sqlx::migrate::MigrateError::VersionMissing(
            20991231000000,
        )));

        let (english_title, english_message) = startup_error_copy(&error, false);
        assert_eq!(english_title, "Update Mingtily");
        assert!(english_message.contains("Install the latest version"));
        assert!(english_message.contains("was not changed"));
        assert!(english_message.contains("20991231000000"));

        let (chinese_title, chinese_message) = startup_error_copy(&error, true);
        assert_eq!(chinese_title, "需要更新 Mingtily");
        assert!(chinese_message.contains("请安装最新版本"));
        assert!(chinese_message.contains("没有被修改"));
        assert!(chinese_message.contains("20991231000000"));
    }
}
