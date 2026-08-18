// macOS audio permissions handling
use anyhow::Result;
use log::{error, info, warn};

#[cfg(target_os = "macos")]
use std::process::Command;
#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(target_os = "macos")]
static SYSTEM_AUDIO_VERIFIED: AtomicBool = AtomicBool::new(false);

/// Check if the app has Audio Capture permission (required for Core Audio taps on macOS 14.4+)
///
/// Note: Core Audio taps require NSAudioCaptureUsageDescription in Info.plist.
/// When the app first attempts to create a Core Audio tap, macOS will automatically
/// show a permission dialog to the user. If permission is denied, the tap will return
/// silence (all zeros).
///
/// macOS does not expose a public preflight API for the AudioCapture TCC service. This
/// returns true only after this process has successfully created and started a Core Audio
/// stream; creating a tap alone is not proof because denied taps may still be created.
#[cfg(target_os = "macos")]
pub fn check_screen_recording_permission() -> bool {
    SYSTEM_AUDIO_VERIFIED.load(Ordering::Acquire)
}

#[cfg(not(target_os = "macos"))]
pub fn check_screen_recording_permission() -> bool {
    true // Not required on other platforms
}

/// Request Audio Capture permission from the user
/// This will open System Settings to the Privacy & Security page
#[cfg(target_os = "macos")]
pub fn request_screen_recording_permission() -> Result<()> {
    info!("🔐 Opening System Settings for Audio Capture permission...");

    // AudioCapture is managed in the Screen & System Audio Recording privacy pane.
    let result = Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
        .spawn();

    match result {
        Ok(_) => {
            info!("✅ Opened Screen & System Audio Recording settings");
            info!("👉 Enable Mingtily under System Audio Recording Only, then retry");
            Ok(())
        }
        Err(e) => {
            error!("❌ Failed to open System Settings: {}", e);
            Err(anyhow::anyhow!("Failed to open System Settings: {}", e))
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn request_screen_recording_permission() -> Result<()> {
    Ok(()) // Not required on other platforms
}

/// Check and request Audio Capture permission if not granted
/// Returns true if permission is granted, false otherwise
pub fn ensure_screen_recording_permission() -> bool {
    if check_screen_recording_permission() {
        return true;
    }

    warn!("Audio Capture permission not granted - requesting...");

    if let Err(e) = request_screen_recording_permission() {
        error!("Failed to request Audio Capture permission: {}", e);
        return false;
    }

    false // Permission will be granted after restart
}

/// Tauri command to check Screen Recording permission
#[tauri::command]
pub async fn check_screen_recording_permission_command() -> bool {
    check_screen_recording_permission()
}

/// Tauri command to request Screen Recording permission
#[tauri::command]
pub async fn request_screen_recording_permission_command() -> Result<(), String> {
    request_screen_recording_permission().map_err(|e| e.to_string())
}

/// Trigger system audio permission and verify that a complete stream can start.
/// Creating only the process tap is insufficient: macOS may allow that while delivering
/// silence or blocking IO-proc creation when AudioCapture permission is missing.
#[cfg(target_os = "macos")]
pub fn trigger_system_audio_permission() -> Result<bool> {
    info!("🔐 Triggering Audio Capture permission request...");

    match crate::audio::capture::CoreAudioCapture::new().and_then(|capture| capture.stream()) {
        Ok(stream) => {
            drop(stream);
            info!("✅ Audio Capture permission verified with a started stream");
            Ok(true)
        }
        Err(e) => {
            let error_msg = e.to_string().to_lowercase();
            if error_msg.contains("permission") || error_msg.contains("denied") {
                info!("🔐 Audio Capture permission denied");
                info!("👉 Please grant Audio Capture permission in System Settings");
                return Ok(false);
            }
            warn!("⚠️ Failed to create Core Audio tap: {}", e);
            // If tap creation fails for other reasons, still return false
            // as we can't verify permission status
            Ok(false)
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn trigger_system_audio_permission() -> Result<bool> {
    // System audio permissions not required on other platforms
    info!("System audio permissions not required on this platform");
    Ok(true)
}

/// Tauri command to trigger system audio permission request
/// Returns true if permission was granted (stream created), false if denied
#[tauri::command]
pub async fn trigger_system_audio_permission_command() -> Result<bool, String> {
    // AudioDeviceCreateIOProcID may block for about a minute when permission is absent.
    // Bound onboarding to eight seconds; the detached native task owns and later drops
    // any resources it creates, while the user can grant access in System Settings.
    let task = tokio::task::spawn_blocking(trigger_system_audio_permission);
    match tokio::time::timeout(std::time::Duration::from_secs(8), task).await {
        Ok(joined) => {
            let granted = joined
                .map_err(|e| format!("Task join error: {}", e))?
                .map_err(|e| e.to_string())?;
            #[cfg(target_os = "macos")]
            SYSTEM_AUDIO_VERIFIED.store(granted, Ordering::Release);
            Ok(granted)
        }
        Err(_) => {
            #[cfg(target_os = "macos")]
            SYSTEM_AUDIO_VERIFIED.store(false, Ordering::Release);
            warn!("Audio Capture verification timed out; permission is not ready");
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_permission() {
        let has_permission = check_screen_recording_permission();
        println!("Has Screen Recording permission: {}", has_permission);
    }
}
