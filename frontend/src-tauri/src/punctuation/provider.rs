use super::models;
use crate::audio::transcription::{TranscriptResult, TranscriptionError, TranscriptionProvider};
use async_trait::async_trait;
use log::{info, warn};
use sherpa_onnx::{OfflinePunctuation, OfflinePunctuationConfig};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Runtime};

enum PunctuationState {
    Uninitialized,
    Ready(OfflinePunctuation),
    Failed,
}

struct PunctuationRuntime {
    model_path: PathBuf,
    state: Mutex<PunctuationState>,
}

impl PunctuationRuntime {
    fn new(model_path: PathBuf) -> Self {
        Self {
            model_path,
            state: Mutex::new(PunctuationState::Uninitialized),
        }
    }

    fn add_punctuation(&self, text: &str) -> Option<String> {
        let mut state = self.state.lock().ok()?;

        if matches!(*state, PunctuationState::Uninitialized) {
            let Some(model_path) = self.model_path.to_str().map(str::to_string) else {
                warn!("Punctuation model path is not valid UTF-8");
                *state = PunctuationState::Failed;
                return None;
            };
            let mut config = OfflinePunctuationConfig::default();
            config.model.ct_transformer = Some(model_path);
            config.model.num_threads = 1;

            *state = match OfflinePunctuation::create(&config) {
                Some(engine) => {
                    info!("Local punctuation model initialized");
                    PunctuationState::Ready(engine)
                }
                None => {
                    warn!("Unable to initialize the local punctuation model; using raw ASR text");
                    PunctuationState::Failed
                }
            };
        }

        match &*state {
            PunctuationState::Ready(engine) => engine.add_punctuation(text),
            PunctuationState::Uninitialized | PunctuationState::Failed => None,
        }
    }
}

pub struct PunctuatedTranscriptionProvider {
    inner: Arc<dyn TranscriptionProvider>,
    punctuation: Arc<PunctuationRuntime>,
}

impl PunctuatedTranscriptionProvider {
    fn new(inner: Arc<dyn TranscriptionProvider>, model_path: PathBuf) -> Self {
        Self {
            inner,
            punctuation: Arc::new(PunctuationRuntime::new(model_path)),
        }
    }
}

#[async_trait]
impl TranscriptionProvider for PunctuatedTranscriptionProvider {
    async fn transcribe(
        &self,
        audio: Vec<f32>,
        language: Option<String>,
    ) -> Result<TranscriptResult, TranscriptionError> {
        let punctuation_language = language.clone();
        let mut result = self.inner.transcribe(audio, language).await?;

        if result.is_partial
            || !should_restore_punctuation(punctuation_language.as_deref(), &result.text)
        {
            return Ok(result);
        }

        let punctuation = self.punctuation.clone();
        let raw_text = result.text.clone();
        let raw_text_for_style = raw_text.clone();
        match tokio::task::spawn_blocking(move || punctuation.add_punctuation(&raw_text)).await {
            Ok(Some(punctuated)) if !punctuated.trim().is_empty() => {
                result.text = normalize_punctuation_style(&raw_text_for_style, punctuated.trim());
            }
            Ok(_) => {
                warn!("Punctuation inference returned no text; using raw ASR text");
            }
            Err(error) => {
                warn!("Punctuation inference task failed; using raw ASR text: {error}");
            }
        }

        Ok(result)
    }

    async fn is_model_loaded(&self) -> bool {
        self.inner.is_model_loaded().await
    }

    async fn get_current_model(&self) -> Option<String> {
        self.inner.get_current_model().await
    }

    fn provider_name(&self) -> &'static str {
        "Sherpa ONNX + punctuation"
    }
}

pub fn wrap_if_available<R: Runtime>(
    app: &AppHandle<R>,
    provider: Arc<dyn TranscriptionProvider>,
) -> Arc<dyn TranscriptionProvider> {
    match models::installed_model_path(app) {
        Ok(Some(model_path)) => {
            Arc::new(PunctuatedTranscriptionProvider::new(provider, model_path))
        }
        Ok(None) => provider,
        Err(error) => {
            warn!("Unable to inspect the punctuation model; using raw ASR text: {error}");
            provider
        }
    }
}

fn should_restore_punctuation(language: Option<&str>, text: &str) -> bool {
    if text.trim().is_empty() {
        return false;
    }

    let language = language
        .unwrap_or("auto")
        .trim()
        .to_ascii_lowercase()
        .replace('_', "-");
    match language.as_str() {
        "zh" | "zh-cn" | "zh-hans" | "yue" | "en" | "en-us" | "en-gb" => true,
        "auto" | "auto-translate" | "" => is_supported_auto_text(text),
        _ => false,
    }
}

fn is_supported_auto_text(text: &str) -> bool {
    let mut contains_supported_script = false;

    for character in text.chars() {
        if is_kana(character) || is_hangul(character) {
            return false;
        }
        if character.is_ascii_alphabetic() || is_cjk_ideograph(character) {
            contains_supported_script = true;
        }
    }

    contains_supported_script
}

fn is_kana(character: char) -> bool {
    matches!(character as u32, 0x3040..=0x30ff | 0x31f0..=0x31ff)
}

fn is_hangul(character: char) -> bool {
    matches!(character as u32, 0x1100..=0x11ff | 0x3130..=0x318f | 0xac00..=0xd7af)
}

fn is_cjk_ideograph(character: char) -> bool {
    matches!(character as u32, 0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xf900..=0xfaff)
}

fn normalize_punctuation_style(raw_text: &str, punctuated: &str) -> String {
    if raw_text.chars().any(is_cjk_ideograph)
        || !raw_text
            .chars()
            .any(|character| character.is_ascii_alphabetic())
    {
        return punctuated.to_string();
    }

    punctuated
        .replace('，', ", ")
        .replace('。', ". ")
        .replace('！', "! ")
        .replace('？', "? ")
        .replace('；', "; ")
        .replace('：', ": ")
        .replace('、', ", ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" ,", ",")
        .replace(" .", ".")
        .replace(" !", "!")
        .replace(" ?", "?")
        .replace(" ;", ";")
        .replace(" :", ":")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_chinese_and_english_are_supported() {
        assert!(should_restore_punctuation(Some("zh"), "我们开始吧"));
        assert!(should_restore_punctuation(Some("yue"), "我哋開始啦"));
        assert!(should_restore_punctuation(Some("en"), "let us begin"));
    }

    #[test]
    fn automatic_mode_accepts_chinese_and_english() {
        assert!(should_restore_punctuation(Some("auto"), "我们开始吧"));
        assert!(should_restore_punctuation(Some("auto"), "let us begin"));
        assert!(should_restore_punctuation(None, "中 English 混合"));
    }

    #[test]
    fn unsupported_scripts_fail_open_without_punctuation() {
        assert!(!should_restore_punctuation(Some("ja"), "会議を始めます"));
        assert!(!should_restore_punctuation(Some("ko"), "회의를 시작합니다"));
        assert!(!should_restore_punctuation(Some("auto"), "会議を始めます"));
        assert!(!should_restore_punctuation(
            Some("auto"),
            "회의를 시작합니다"
        ));
    }

    #[test]
    fn empty_or_unknown_text_is_skipped() {
        assert!(!should_restore_punctuation(Some("zh"), "  "));
        assert!(!should_restore_punctuation(Some("auto"), "12345"));
        assert!(!should_restore_punctuation(
            Some("fr"),
            "bonjour tout le monde"
        ));
    }

    #[test]
    fn pure_english_uses_ascii_punctuation() {
        assert_eq!(
            normalize_punctuation_style(
                "today is a good day how are you",
                "today is a good day，how are you？"
            ),
            "today is a good day, how are you?"
        );
    }

    #[test]
    fn chinese_and_mixed_text_keep_full_width_punctuation() {
        assert_eq!(
            normalize_punctuation_style("我们开始吧", "我们开始吧。"),
            "我们开始吧。"
        );
        assert_eq!(
            normalize_punctuation_style("今天 review 一下", "今天 review 一下。"),
            "今天 review 一下。"
        );
    }
}
