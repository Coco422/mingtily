use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "app-settings.json";
const LOCALE_KEY: &str = "uiLocale";
pub const EN_US: &str = "en-US";
pub const ZH_CN: &str = "zh-CN";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UiLocaleConfig {
    pub locale: String,
}

fn validate_locale(locale: &str) -> Result<()> {
    match locale {
        EN_US | ZH_CN => Ok(()),
        _ => Err(anyhow!("Unsupported UI locale: {locale}")),
    }
}

pub fn load_ui_locale<R: Runtime>(app: &AppHandle<R>) -> String {
    let Ok(store) = app.store(STORE_FILE) else {
        return EN_US.to_string();
    };
    store
        .get(LOCALE_KEY)
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .filter(|locale| validate_locale(locale).is_ok())
        .unwrap_or_else(|| EN_US.to_string())
}

pub fn is_zh_cn<R: Runtime>(app: &AppHandle<R>) -> bool {
    load_ui_locale(app) == ZH_CN
}

#[tauri::command]
pub fn get_ui_locale<R: Runtime>(app: AppHandle<R>) -> UiLocaleConfig {
    UiLocaleConfig {
        locale: load_ui_locale(&app),
    }
}

#[tauri::command]
pub fn set_ui_locale<R: Runtime>(app: AppHandle<R>, locale: String) -> Result<(), String> {
    validate_locale(&locale).map_err(|error| error.to_string())?;
    let store = app
        .store(STORE_FILE)
        .map_err(|error| format!("Failed to access app settings: {error}"))?;
    store.set(LOCALE_KEY, serde_json::json!(locale));
    store
        .save()
        .map_err(|error| format!("Failed to save UI locale: {error}"))?;
    crate::tray::update_tray_menu(&app);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_supported_locales_are_accepted() {
        assert!(validate_locale(EN_US).is_ok());
        assert!(validate_locale(ZH_CN).is_ok());
        assert!(validate_locale("zh").is_err());
    }
}
