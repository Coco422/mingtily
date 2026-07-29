use super::models::{InstalledSherpaModel, SherpaAsrBackend};
use crate::audio::transcription::{TranscriptResult, TranscriptionError, TranscriptionProvider};
use async_trait::async_trait;
use log::warn;
use sherpa_onnx::{
    OfflineParaformerModelConfig, OfflineQwen3ASRModelConfig, OfflineRecognizer,
    OfflineRecognizerConfig, OfflineSenseVoiceModelConfig,
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
    recognizer: Arc<Mutex<Option<CachedRecognizer>>>,
}

impl SherpaOfflineAsrProvider {
    pub fn new(model: InstalledSherpaModel) -> Self {
        Self {
            model,
            recognizer: Arc::new(Mutex::new(None)),
        }
    }

    fn transcribe_blocking(
        model: &InstalledSherpaModel,
        cache: &Mutex<Option<CachedRecognizer>>,
        audio: &[f32],
        language: Option<&str>,
    ) -> Result<String, TranscriptionError> {
        let normalized_language = normalize_language(model.backend, language)?;
        let cache_key = recognizer_cache_key(model.backend, &normalized_language);
        let mut guard = cache
            .lock()
            .map_err(|_| TranscriptionError::EngineFailed("Sherpa ASR lock poisoned".into()))?;

        if guard.as_ref().is_none_or(|cached| cached.key != cache_key) {
            let recognizer = create_recognizer(model, &normalized_language)?;
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
        let recognizer = self.recognizer.clone();
        let result = tokio::task::spawn_blocking(move || {
            Self::transcribe_blocking(&model, &recognizer, &audio, language.as_deref())
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

fn create_recognizer(
    model: &InstalledSherpaModel,
    language: &str,
) -> Result<OfflineRecognizer, TranscriptionError> {
    let mut config = OfflineRecognizerConfig::default();
    config.model_config.num_threads = 2;
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
                ..Default::default()
            };
        }
        SherpaAsrBackend::ParaformerOnline => {
            return Err(TranscriptionError::EngineFailed(
                "The online Paraformer model requires the streaming provider".into(),
            ));
        }
    }

    OfflineRecognizer::create(&config).ok_or_else(|| {
        TranscriptionError::EngineFailed(format!(
            "Unable to initialize Sherpa ONNX model '{}'",
            model.id
        ))
    })
}

fn path_string(path: &Path) -> Result<String, TranscriptionError> {
    path.to_str().map(str::to_string).ok_or_else(|| {
        TranscriptionError::EngineFailed(format!(
            "Sherpa ASR model path is not valid UTF-8: {}",
            path.display()
        ))
    })
}

fn recognizer_cache_key(backend: SherpaAsrBackend, language: &str) -> String {
    match backend {
        SherpaAsrBackend::SenseVoice => language.to_string(),
        SherpaAsrBackend::ParaformerOffline
        | SherpaAsrBackend::ParaformerOnline
        | SherpaAsrBackend::Qwen3Asr => "shared".to_string(),
    }
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
        SherpaAsrBackend::ParaformerOffline | SherpaAsrBackend::ParaformerOnline => {
            Ok("auto".to_string())
        }
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
}
