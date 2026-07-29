use super::models::{InstalledSherpaModel, SherpaAsrBackend, PARAFORMER_ONLINE_MODEL_ID};
use crate::audio::transcription::{TranscriptResult, TranscriptionError, TranscriptionProvider};
use crate::audio::AudioChunk;
use async_trait::async_trait;
use log::{error, info, warn};
use once_cell::sync::Lazy;
use serde::Serialize;
use sherpa_onnx::{
    OnlineParaformerModelConfig, OnlineRecognizer, OnlineRecognizerConfig, OnlineStream,
};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use tauri::{AppHandle, Emitter, Runtime};

const SAMPLE_RATE: i32 = 16_000;
const MINIMUM_SAMPLES: usize = 1_600;
const TAIL_PADDING_SAMPLES: usize = 4_800;
const MAX_DECODE_STEPS: usize = 100_000;

type SharedRecognizer = Arc<Mutex<OnlineRecognizer>>;

struct CachedOnlineRecognizer {
    key: String,
    recognizer: Weak<Mutex<OnlineRecognizer>>,
}

static ONLINE_RECOGNIZER_CACHE: Lazy<Mutex<Option<CachedOnlineRecognizer>>> =
    Lazy::new(|| Mutex::new(None));

#[derive(Clone)]
pub struct SherpaOnlineAsrProvider {
    model: InstalledSherpaModel,
}

impl SherpaOnlineAsrProvider {
    pub fn new(model: InstalledSherpaModel) -> Self {
        Self { model }
    }
}

#[async_trait]
impl TranscriptionProvider for SherpaOnlineAsrProvider {
    async fn transcribe(
        &self,
        audio: Vec<f32>,
        _language: Option<String>,
    ) -> Result<TranscriptResult, TranscriptionError> {
        if audio.len() < MINIMUM_SAMPLES {
            return Err(TranscriptionError::AudioTooShort {
                samples: audio.len(),
                minimum: MINIMUM_SAMPLES,
            });
        }

        let model = self.model.clone();
        let text = tokio::task::spawn_blocking(move || transcribe_segment(&model, &audio))
            .await
            .map_err(|error| {
                TranscriptionError::EngineFailed(format!("Online Paraformer task failed: {error}"))
            })??;

        Ok(TranscriptResult {
            text,
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
        "Sherpa ONNX Online Paraformer"
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveTranscriptUpdate {
    pub utterance_id: u64,
    pub revision: u64,
    pub text: String,
    pub is_final: bool,
    pub audio_start_time: f64,
    pub audio_end_time: f64,
}

pub fn is_online_model(model_id: &str) -> bool {
    model_id == PARAFORMER_ONLINE_MODEL_ID
}

pub fn start_live_transcription_task<R: Runtime>(
    app: AppHandle<R>,
    receiver: tokio::sync::mpsc::UnboundedReceiver<AudioChunk>,
    model: InstalledSherpaModel,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let error_app = app.clone();
        match tokio::task::spawn_blocking(move || run_live_session(app, receiver, &model)).await {
            Ok(Ok(())) => info!("Online Paraformer live session completed"),
            Ok(Err(message)) => {
                error!("Online Paraformer live session failed: {message}");
                let _ = error_app.emit(
                    "transcription-error",
                    serde_json::json!({
                        "error": message,
                        "userMessage": "Live transcription stopped because the streaming model failed.",
                        "actionable": false
                    }),
                );
            }
            Err(error) => {
                error!("Online Paraformer live task failed: {error}");
                let _ = error_app.emit(
                    "transcription-error",
                    serde_json::json!({
                        "error": error.to_string(),
                        "userMessage": "Live transcription stopped unexpectedly.",
                        "actionable": false
                    }),
                );
            }
        }
    })
}

fn run_live_session<R: Runtime>(
    app: AppHandle<R>,
    mut receiver: tokio::sync::mpsc::UnboundedReceiver<AudioChunk>,
    model: &InstalledSherpaModel,
) -> Result<(), String> {
    let recognizer = shared_recognizer(model).map_err(|error| error.to_string())?;
    let stream = lock_recognizer(&recognizer)
        .map_err(|error| error.to_string())?
        .create_stream();

    let mut utterance_id = 0_u64;
    let mut revision = 0_u64;
    let mut utterance_start_time = 0.0_f64;
    let mut elapsed_time = 0.0_f64;
    let mut last_text = String::new();
    let mut last_sample_rate = SAMPLE_RATE;

    while let Some(chunk) = receiver.blocking_recv() {
        if chunk.data.is_empty() || chunk.sample_rate == 0 {
            continue;
        }

        last_sample_rate = chunk.sample_rate as i32;
        stream.accept_waveform(last_sample_rate, &chunk.data);
        elapsed_time += chunk.data.len() as f64 / chunk.sample_rate as f64;
        decode_until_drained(&recognizer, &stream).map_err(|error| error.to_string())?;

        let (result, is_endpoint) = {
            let recognizer = lock_recognizer(&recognizer).map_err(|error| error.to_string())?;
            (
                recognizer.get_result(&stream),
                recognizer.is_endpoint(&stream),
            )
        };

        if let Some(result) = result {
            let text = result.text.trim().to_string();
            let is_final = is_endpoint || result.is_final;
            if !text.is_empty() && (text != last_text || is_final) {
                revision += 1;
                emit_live_update(
                    &app,
                    LiveTranscriptUpdate {
                        utterance_id,
                        revision,
                        text: text.clone(),
                        is_final,
                        audio_start_time: utterance_start_time,
                        audio_end_time: elapsed_time,
                    },
                );
                last_text = text;
            }
        }

        if is_endpoint {
            lock_recognizer(&recognizer)
                .map_err(|error| error.to_string())?
                .reset(&stream);
            utterance_id += 1;
            revision = 0;
            utterance_start_time = elapsed_time;
            last_text.clear();
        }
    }

    let tail_padding = vec![0.0_f32; (last_sample_rate as usize * 3) / 10];
    stream.accept_waveform(last_sample_rate, &tail_padding);
    elapsed_time += tail_padding.len() as f64 / last_sample_rate as f64;
    stream.input_finished();
    decode_until_drained(&recognizer, &stream).map_err(|error| error.to_string())?;

    if let Some(result) = lock_recognizer(&recognizer)
        .map_err(|error| error.to_string())?
        .get_result(&stream)
    {
        let text = result.text.trim().to_string();
        if !text.is_empty() {
            revision += 1;
            emit_live_update(
                &app,
                LiveTranscriptUpdate {
                    utterance_id,
                    revision,
                    text,
                    is_final: true,
                    audio_start_time: utterance_start_time,
                    audio_end_time: elapsed_time,
                },
            );
        }
    }

    Ok(())
}

fn emit_live_update<R: Runtime>(app: &AppHandle<R>, update: LiveTranscriptUpdate) {
    if let Err(error) = app.emit("transcript-live-update", update) {
        warn!("Unable to emit live transcript update: {error}");
    }
}

fn transcribe_segment(
    model: &InstalledSherpaModel,
    audio: &[f32],
) -> Result<String, TranscriptionError> {
    let recognizer = shared_recognizer(model)?;
    let stream = lock_recognizer(&recognizer)?.create_stream();
    stream.accept_waveform(SAMPLE_RATE, audio);
    stream.accept_waveform(SAMPLE_RATE, &vec![0.0_f32; TAIL_PADDING_SAMPLES]);
    stream.input_finished();
    decode_until_drained(&recognizer, &stream)?;

    let result = {
        let recognizer = lock_recognizer(&recognizer)?;
        recognizer.get_result(&stream)
    };
    result
        .map(|result| result.text.trim().to_string())
        .ok_or_else(|| {
            TranscriptionError::EngineFailed(
                "Sherpa ONNX returned no online Paraformer result".into(),
            )
        })
}

fn shared_recognizer(model: &InstalledSherpaModel) -> Result<SharedRecognizer, TranscriptionError> {
    if model.backend != SherpaAsrBackend::ParaformerOnline {
        return Err(TranscriptionError::EngineFailed(format!(
            "Model '{}' is not an online Paraformer model",
            model.id
        )));
    }

    let key = model.root.to_string_lossy().to_string();
    let mut cache = ONLINE_RECOGNIZER_CACHE.lock().map_err(|_| {
        TranscriptionError::EngineFailed("Online Paraformer cache lock poisoned".into())
    })?;

    if let Some(cached) = cache.as_ref() {
        if cached.key == key {
            if let Some(recognizer) = cached.recognizer.upgrade() {
                return Ok(recognizer);
            }
        }
    }

    let recognizer = Arc::new(Mutex::new(create_recognizer(model)?));
    *cache = Some(CachedOnlineRecognizer {
        key,
        recognizer: Arc::downgrade(&recognizer),
    });
    Ok(recognizer)
}

fn create_recognizer(model: &InstalledSherpaModel) -> Result<OnlineRecognizer, TranscriptionError> {
    let mut config = OnlineRecognizerConfig::default();
    config.model_config.tokens = Some(path_string(&model.root.join("tokens.txt"))?);
    config.model_config.num_threads = 2;
    config.model_config.provider = Some("cpu".to_string());
    config.model_config.paraformer = OnlineParaformerModelConfig {
        encoder: Some(path_string(&model.root.join("encoder.int8.onnx"))?),
        decoder: Some(path_string(&model.root.join("decoder.int8.onnx"))?),
    };
    config.decoding_method = Some("greedy_search".to_string());
    config.enable_endpoint = true;
    config.rule1_min_trailing_silence = 2.4;
    config.rule2_min_trailing_silence = 1.2;
    config.rule3_min_utterance_length = 300.0;

    OnlineRecognizer::create(&config).ok_or_else(|| {
        TranscriptionError::EngineFailed(format!(
            "Unable to initialize online Paraformer model '{}'",
            model.id
        ))
    })
}

fn decode_until_drained(
    recognizer: &SharedRecognizer,
    stream: &OnlineStream,
) -> Result<(), TranscriptionError> {
    for step in 0..MAX_DECODE_STEPS {
        let is_ready = lock_recognizer(recognizer)?.is_ready(stream);
        if !is_ready {
            return Ok(());
        }
        lock_recognizer(recognizer)?.decode(stream);
        if step % 8 == 7 {
            std::thread::yield_now();
        }
    }

    Err(TranscriptionError::EngineFailed(
        "Online Paraformer exceeded the decode-step safety limit".into(),
    ))
}

fn lock_recognizer(
    recognizer: &SharedRecognizer,
) -> Result<MutexGuard<'_, OnlineRecognizer>, TranscriptionError> {
    recognizer.lock().map_err(|_| {
        TranscriptionError::EngineFailed("Online Paraformer recognizer lock poisoned".into())
    })
}

fn path_string(path: &Path) -> Result<String, TranscriptionError> {
    path.to_str().map(str::to_string).ok_or_else(|| {
        TranscriptionError::EngineFailed(format!(
            "Online Paraformer model path is not valid UTF-8: {}",
            path.display()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn online_model_id_is_explicit() {
        assert!(is_online_model(PARAFORMER_ONLINE_MODEL_ID));
        assert!(!is_online_model("paraformer-zh-small-int8"));
    }

    #[test]
    #[ignore = "requires PARAFORMER_ONLINE_MODEL_DIR and PARAFORMER_ONLINE_TEST_WAV"]
    fn transcribes_public_online_paraformer_fixture() {
        let model_root = PathBuf::from(
            std::env::var("PARAFORMER_ONLINE_MODEL_DIR")
                .expect("PARAFORMER_ONLINE_MODEL_DIR must be set"),
        );
        let wav_path = std::env::var("PARAFORMER_ONLINE_TEST_WAV")
            .expect("PARAFORMER_ONLINE_TEST_WAV must be set");
        let wave = sherpa_onnx::Wave::read(&wav_path).expect("test WAV should be readable");
        assert_eq!(wave.sample_rate(), SAMPLE_RATE);

        let model = InstalledSherpaModel {
            id: PARAFORMER_ONLINE_MODEL_ID.to_string(),
            backend: SherpaAsrBackend::ParaformerOnline,
            root: model_root,
        };
        let text = transcribe_segment(&model, wave.samples()).expect("transcription should work");
        assert!(!text.trim().is_empty());
    }

    #[test]
    #[ignore = "requires PARAFORMER_ONLINE_MODEL_DIR and PARAFORMER_ONLINE_TEST_WAV"]
    fn produces_partial_revisions_before_finalization() {
        let model_root = PathBuf::from(
            std::env::var("PARAFORMER_ONLINE_MODEL_DIR")
                .expect("PARAFORMER_ONLINE_MODEL_DIR must be set"),
        );
        let wav_path = std::env::var("PARAFORMER_ONLINE_TEST_WAV")
            .expect("PARAFORMER_ONLINE_TEST_WAV must be set");
        let wave = sherpa_onnx::Wave::read(&wav_path).expect("test WAV should be readable");
        let model = InstalledSherpaModel {
            id: PARAFORMER_ONLINE_MODEL_ID.to_string(),
            backend: SherpaAsrBackend::ParaformerOnline,
            root: model_root,
        };
        let recognizer = shared_recognizer(&model).expect("recognizer should initialize");
        let stream = lock_recognizer(&recognizer)
            .expect("recognizer lock should be available")
            .create_stream();
        let mut revisions = Vec::new();

        for chunk in wave.samples().chunks(3_200) {
            stream.accept_waveform(wave.sample_rate(), chunk);
            decode_until_drained(&recognizer, &stream).expect("stream should decode");
            if let Some(result) = lock_recognizer(&recognizer)
                .expect("recognizer lock should be available")
                .get_result(&stream)
            {
                let text = result.text.trim().to_string();
                if !text.is_empty() && revisions.last() != Some(&text) {
                    revisions.push(text);
                }
            }
        }

        stream.accept_waveform(SAMPLE_RATE, &vec![0.0_f32; TAIL_PADDING_SAMPLES]);
        stream.input_finished();
        decode_until_drained(&recognizer, &stream).expect("stream should flush");
        let final_text = lock_recognizer(&recognizer)
            .expect("recognizer lock should be available")
            .get_result(&stream)
            .expect("final result should exist")
            .text;

        assert!(!final_text.trim().is_empty());
        assert!(
            revisions.len() >= 2,
            "expected at least two distinct streaming hypotheses, got {revisions:?}"
        );
    }
}
