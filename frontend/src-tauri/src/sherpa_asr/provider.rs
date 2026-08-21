use super::models::{InstalledSherpaModel, SherpaAsrBackend};
use super::RuntimeEnhancements;
use crate::audio::transcription::{TranscriptResult, TranscriptionError, TranscriptionProvider};
use async_trait::async_trait;
use log::warn;
use sherpa_onnx::{
    OfflineFunASRNanoModelConfig, OfflineParaformerModelConfig, OfflineQwen3ASRModelConfig,
    OfflineRecognizer, OfflineRecognizerConfig, OfflineSenseVoiceModelConfig,
};
use std::path::Path;
use std::sync::{Arc, Mutex};

const SAMPLE_RATE: i32 = 16_000;
const MINIMUM_SAMPLES: usize = 1_600;

struct CachedRecognizer {
    key: String,
    recognizer: OfflineRecognizer,
}

#[derive(Clone)]
pub struct SherpaOfflineAsrProvider {
    model: InstalledSherpaModel,
    enhancements: RuntimeEnhancements,
    num_threads: usize,
    recognizer: Arc<Mutex<Option<CachedRecognizer>>>,
}

impl SherpaOfflineAsrProvider {
    pub fn new(model: InstalledSherpaModel, enhancements: RuntimeEnhancements) -> Self {
        Self::new_with_threads(model, enhancements, 2)
    }

    pub fn new_with_threads(
        model: InstalledSherpaModel,
        enhancements: RuntimeEnhancements,
        num_threads: usize,
    ) -> Self {
        Self {
            model,
            enhancements,
            num_threads: num_threads.max(1),
            recognizer: Arc::new(Mutex::new(None)),
        }
    }

    fn transcribe_blocking(
        model: &InstalledSherpaModel,
        enhancements: &RuntimeEnhancements,
        num_threads: usize,
        cache: &Mutex<Option<CachedRecognizer>>,
        audio: &[f32],
        language: Option<&str>,
    ) -> Result<String, TranscriptionError> {
        let normalized_language = normalize_language(model.backend, language)?;
        let cache_key = recognizer_cache_key(
            model.backend,
            &normalized_language,
            enhancements.cache_signature(),
        );
        let mut guard = cache
            .lock()
            .map_err(|_| TranscriptionError::EngineFailed("Sherpa ASR lock poisoned".into()))?;

        if guard.as_ref().is_none_or(|cached| cached.key != cache_key) {
            let recognizer = create_recognizer_with_threads(
                model,
                &normalized_language,
                enhancements,
                num_threads,
            )?;
            *guard = Some(CachedRecognizer {
                key: cache_key,
                recognizer,
            });
        }

        let recognizer = &guard
            .as_ref()
            .ok_or(TranscriptionError::ModelNotLoaded)?
            .recognizer;
        let stream = recognizer.create_stream();

        if model.backend == SherpaAsrBackend::Qwen3Asr {
            if let Some(language) = qwen3_language_name(&normalized_language) {
                stream.set_option("language", language);
            }
        }

        stream.accept_waveform(SAMPLE_RATE, audio);
        recognizer.decode(&stream);
        stream
            .get_result()
            .map(|result| result.text.trim().to_string())
            .ok_or_else(|| {
                TranscriptionError::EngineFailed(
                    "Sherpa ONNX returned no transcription result".into(),
                )
            })
    }
}

#[async_trait]
impl TranscriptionProvider for SherpaOfflineAsrProvider {
    async fn transcribe(
        &self,
        audio: Vec<f32>,
        language: Option<String>,
    ) -> Result<TranscriptResult, TranscriptionError> {
        if audio.len() < MINIMUM_SAMPLES {
            return Err(TranscriptionError::AudioTooShort {
                samples: audio.len(),
                minimum: MINIMUM_SAMPLES,
            });
        }

        let model = self.model.clone();
        let enhancements = self.enhancements.clone();
        let num_threads = self.num_threads;
        let recognizer = self.recognizer.clone();
        let result = tokio::task::spawn_blocking(move || {
            Self::transcribe_blocking(
                &model,
                &enhancements,
                num_threads,
                &recognizer,
                &audio,
                language.as_deref(),
            )
        })
        .await
        .map_err(|error| {
            TranscriptionError::EngineFailed(format!("Sherpa ASR task failed: {error}"))
        })??;

        Ok(TranscriptResult {
            text: result,
            confidence: None,
            is_partial: false,
        })
    }

    async fn is_model_loaded(&self) -> bool {
        self.model.root.is_dir()
    }

    async fn get_current_model(&self) -> Option<String> {
        Some(self.model.id.clone())
    }

    fn provider_name(&self) -> &'static str {
        "Sherpa ONNX"
    }
}

#[cfg(test)]
fn create_recognizer(
    model: &InstalledSherpaModel,
    language: &str,
    enhancements: &RuntimeEnhancements,
) -> Result<OfflineRecognizer, TranscriptionError> {
    create_recognizer_with_threads(model, language, enhancements, 2)
}

fn create_recognizer_with_threads(
    model: &InstalledSherpaModel,
    language: &str,
    enhancements: &RuntimeEnhancements,
    num_threads: usize,
) -> Result<OfflineRecognizer, TranscriptionError> {
    let config = build_recognizer_config_with_threads(model, language, enhancements, num_threads)?;
    OfflineRecognizer::create(&config).ok_or_else(|| {
        TranscriptionError::EngineFailed(format!(
            "Unable to initialize Sherpa ONNX model '{}'",
            model.id
        ))
    })
}

#[cfg(test)]
fn build_recognizer_config(
    model: &InstalledSherpaModel,
    language: &str,
    enhancements: &RuntimeEnhancements,
) -> Result<OfflineRecognizerConfig, TranscriptionError> {
    build_recognizer_config_with_threads(model, language, enhancements, 2)
}

fn build_recognizer_config_with_threads(
    model: &InstalledSherpaModel,
    language: &str,
    enhancements: &RuntimeEnhancements,
    num_threads: usize,
) -> Result<OfflineRecognizerConfig, TranscriptionError> {
    let mut config = OfflineRecognizerConfig::default();
    config.model_config.num_threads = num_threads.max(1) as i32;
    config.model_config.provider = Some("cpu".to_string());

    match model.backend {
        SherpaAsrBackend::SenseVoice => {
            config.model_config.tokens = Some(path_string(&model.root.join("tokens.txt"))?);
            config.model_config.sense_voice = OfflineSenseVoiceModelConfig {
                model: Some(path_string(&model.root.join("model.int8.onnx"))?),
                language: Some(language.to_string()),
                use_itn: true,
            };
        }
        SherpaAsrBackend::ParaformerOffline => {
            config.model_config.tokens = Some(path_string(&model.root.join("tokens.txt"))?);
            config.model_config.paraformer = OfflineParaformerModelConfig {
                model: Some(path_string(&model.root.join("model.int8.onnx"))?),
            };
        }
        SherpaAsrBackend::Qwen3Asr => {
            config.feat_config.feature_dim = 128;
            config.model_config.qwen3_asr = OfflineQwen3ASRModelConfig {
                conv_frontend: Some(path_string(&model.root.join("conv_frontend.onnx"))?),
                encoder: Some(path_string(&model.root.join("encoder.int8.onnx"))?),
                decoder: Some(path_string(&model.root.join("decoder.int8.onnx"))?),
                tokenizer: Some(path_string(&model.root.join("tokenizer"))?),
                max_new_tokens: 512,
                hotwords: enhancements.hotwords.clone(),
                ..Default::default()
            };
        }
        SherpaAsrBackend::FunAsrNano => {
            config.model_config.funasr_nano = OfflineFunASRNanoModelConfig {
                encoder_adaptor: Some(path_string(&model.root.join("encoder_adaptor.int8.onnx"))?),
                llm: Some(path_string(&model.root.join("llm.int8.onnx"))?),
                embedding: Some(path_string(&model.root.join("embedding.int8.onnx"))?),
                tokenizer: Some(path_string(&model.root.join("Qwen3-0.6B"))?),
                system_prompt: Some("You are a helpful assistant.".to_string()),
                user_prompt: Some("语音转写:".to_string()),
                max_new_tokens: 512,
                temperature: 1e-6,
                top_p: 0.8,
                seed: 42,
                language: None,
                itn: 1,
                hotwords: enhancements.hotwords.clone(),
            };
        }
        SherpaAsrBackend::ParaformerOnline => {
            return Err(TranscriptionError::EngineFailed(
                "The online Paraformer model requires the streaming provider".into(),
            ));
        }
    }

    if let (Some(lexicon), Some(rule_fsts)) = (
        enhancements.homophone_lexicon.clone(),
        enhancements.homophone_rule_fsts.clone(),
    ) {
        config.hr.lexicon = Some(lexicon);
        config.hr.rule_fsts = Some(rule_fsts);
    }

    Ok(config)
}

fn path_string(path: &Path) -> Result<String, TranscriptionError> {
    path.to_str().map(str::to_string).ok_or_else(|| {
        TranscriptionError::EngineFailed(format!(
            "Sherpa ASR model path is not valid UTF-8: {}",
            path.display()
        ))
    })
}

fn recognizer_cache_key(
    backend: SherpaAsrBackend,
    language: &str,
    enhancement_signature: &str,
) -> String {
    let language_key = match backend {
        SherpaAsrBackend::SenseVoice => language,
        SherpaAsrBackend::ParaformerOffline
        | SherpaAsrBackend::ParaformerOnline
        | SherpaAsrBackend::Qwen3Asr
        | SherpaAsrBackend::FunAsrNano => "shared",
    };
    format!("{language_key}:{enhancement_signature}")
}

fn normalize_language(
    backend: SherpaAsrBackend,
    language: Option<&str>,
) -> Result<String, TranscriptionError> {
    let language = language.unwrap_or("auto").trim().to_ascii_lowercase();
    let language = match language.as_str() {
        "" | "auto" => "auto",
        "auto-translate" => {
            warn!(
                "Sherpa ONNX offline models do not translate to English; using automatic transcription"
            );
            "auto"
        }
        other => other,
    };

    match backend {
        SherpaAsrBackend::SenseVoice => match language {
            "auto" | "zh" | "yue" | "en" | "ja" | "ko" => Ok(language.to_string()),
            unsupported => Err(TranscriptionError::UnsupportedLanguage(
                unsupported.to_string(),
            )),
        },
        SherpaAsrBackend::ParaformerOffline
        | SherpaAsrBackend::ParaformerOnline
        | SherpaAsrBackend::FunAsrNano => Ok("auto".to_string()),
        SherpaAsrBackend::Qwen3Asr => {
            if language == "auto" || qwen3_language_name(language).is_some() {
                Ok(language.to_string())
            } else {
                Err(TranscriptionError::UnsupportedLanguage(
                    language.to_string(),
                ))
            }
        }
    }
}

fn qwen3_language_name(language: &str) -> Option<&'static str> {
    match language {
        "zh" => Some("Chinese"),
        "yue" => Some("Cantonese"),
        "en" => Some("English"),
        "ja" => Some("Japanese"),
        "ko" => Some("Korean"),
        "de" => Some("German"),
        "fr" => Some("French"),
        "es" => Some("Spanish"),
        "pt" => Some("Portuguese"),
        "ru" => Some("Russian"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_model(backend: SherpaAsrBackend) -> InstalledSherpaModel {
        InstalledSherpaModel {
            id: "test-model".to_string(),
            backend,
            root: PathBuf::from("/tmp/mingtily-test-model"),
        }
    }

    #[test]
    fn sense_voice_accepts_supported_language_hints() {
        for language in ["auto", "zh", "yue", "en", "ja", "ko"] {
            assert_eq!(
                normalize_language(SherpaAsrBackend::SenseVoice, Some(language)).unwrap(),
                language
            );
        }
        assert!(normalize_language(SherpaAsrBackend::SenseVoice, Some("fr")).is_err());
    }

    #[test]
    fn paraformer_always_uses_automatic_language_detection() {
        assert_eq!(
            normalize_language(SherpaAsrBackend::ParaformerOffline, Some("zh")).unwrap(),
            "auto"
        );
    }

    #[test]
    fn qwen3_maps_language_codes_to_prompt_names() {
        assert_eq!(qwen3_language_name("zh"), Some("Chinese"));
        assert_eq!(qwen3_language_name("yue"), Some("Cantonese"));
        assert_eq!(qwen3_language_name("en"), Some("English"));
    }

    #[test]
    fn enhancement_changes_recognizer_cache_key() {
        assert_ne!(
            recognizer_cache_key(SherpaAsrBackend::Qwen3Asr, "zh", "first"),
            recognizer_cache_key(SherpaAsrBackend::Qwen3Asr, "zh", "second")
        );
    }

    #[test]
    fn qwen3_receives_dynamic_hotwords() {
        let enhancements =
            RuntimeEnhancements::from_parts(Some("Mingtily,SenseVoice".to_string()), None, None);
        let config = build_recognizer_config(
            &test_model(SherpaAsrBackend::Qwen3Asr),
            "auto",
            &enhancements,
        )
        .unwrap();
        assert_eq!(
            config.model_config.qwen3_asr.hotwords.as_deref(),
            Some("Mingtily,SenseVoice")
        );
        assert_eq!(config.model_config.qwen3_asr.max_new_tokens, 512);
    }

    #[test]
    fn funasr_nano_receives_dynamic_hotwords_and_itn() {
        let enhancements =
            RuntimeEnhancements::from_parts(Some("Mingtily,FunASR".to_string()), None, None);
        let config = build_recognizer_config(
            &test_model(SherpaAsrBackend::FunAsrNano),
            "auto",
            &enhancements,
        )
        .unwrap();
        assert_eq!(
            config.model_config.funasr_nano.hotwords.as_deref(),
            Some("Mingtily,FunASR")
        );
        assert_eq!(config.model_config.funasr_nano.itn, 1);
        assert_eq!(config.model_config.funasr_nano.max_new_tokens, 512);
    }

    #[test]
    fn homophone_resources_are_applied_to_offline_recognizer() {
        let enhancements = RuntimeEnhancements::from_parts(
            None,
            Some("/tmp/lexicon.txt".to_string()),
            Some("/tmp/rule-a.fst,/tmp/rule-b.fst".to_string()),
        );
        let config = build_recognizer_config(
            &test_model(SherpaAsrBackend::SenseVoice),
            "zh",
            &enhancements,
        )
        .unwrap();
        assert_eq!(config.hr.lexicon.as_deref(), Some("/tmp/lexicon.txt"));
        assert_eq!(
            config.hr.rule_fsts.as_deref(),
            Some("/tmp/rule-a.fst,/tmp/rule-b.fst")
        );
    }

    #[test]
    fn resolved_thread_budget_is_applied_to_offline_recognizer() {
        let config = build_recognizer_config_with_threads(
            &test_model(SherpaAsrBackend::SenseVoice),
            "zh",
            &RuntimeEnhancements::default(),
            1,
        )
        .unwrap();
        assert_eq!(config.model_config.num_threads, 1);
    }

    #[test]
    fn funasr_nano_uses_automatic_language_detection() {
        assert_eq!(
            normalize_language(SherpaAsrBackend::FunAsrNano, Some("zh")).unwrap(),
            "auto"
        );
    }

    #[test]
    #[ignore = "requires a complete local Fun-ASR Nano model directory"]
    fn funasr_nano_fixture_initializes_native_recognizer() {
        let root = std::env::var("MINGTILY_FUNASR_MODEL_DIR")
            .expect("set MINGTILY_FUNASR_MODEL_DIR to the extracted model directory");
        let model = InstalledSherpaModel {
            id: "funasr-nano-int8".to_string(),
            backend: SherpaAsrBackend::FunAsrNano,
            root: PathBuf::from(root),
        };
        create_recognizer(
            &model,
            "auto",
            &RuntimeEnhancements::from_parts(None, None, None),
        )
        .expect("Fun-ASR Nano fixture should initialize");
    }
}
