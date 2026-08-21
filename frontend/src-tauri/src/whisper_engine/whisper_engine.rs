// Commit name to recover the serial whisper engine processing for smaller meetings [Slower processing but dooes not fail] - "before parallel processing implementation"

use super::acceleration::{whisper_context_acceleration_for, WhisperCompiledBackend};
use crate::config::WHISPER_MODEL_CATALOG;
use anyhow::{anyhow, Result};
use reqwest::{
    header::{CONTENT_RANGE, RANGE},
    Client, StatusCode,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

const WHISPER_MODELSCOPE_REVISION: &str = "52d9452b318d8aa5ea7a8def34b6df7e7fa283a1";
const WHISPER_HUGGINGFACE_REVISION: &str = "5359861c739e955e79d9a303bcbc70fb988958b1";

#[derive(Clone, Copy)]
struct WhisperDownloadSpec {
    model_name: &'static str,
    file_name: &'static str,
    size: u64,
    sha256: &'static str,
}

const WHISPER_DOWNLOAD_SPECS: &[WhisperDownloadSpec] = &[
    WhisperDownloadSpec {
        model_name: "tiny",
        file_name: "ggml-tiny.bin",
        size: 77_691_713,
        sha256: "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
    },
    WhisperDownloadSpec {
        model_name: "base",
        file_name: "ggml-base.bin",
        size: 147_951_465,
        sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
    },
    WhisperDownloadSpec {
        model_name: "small",
        file_name: "ggml-small.bin",
        size: 487_601_967,
        sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
    },
    WhisperDownloadSpec {
        model_name: "medium",
        file_name: "ggml-medium.bin",
        size: 1_533_763_059,
        sha256: "6c14d5adee5f86394037b4e4e8b59f1673b6cee10e3cf0b11bbdbee79c156208",
    },
    WhisperDownloadSpec {
        model_name: "large-v3-turbo",
        file_name: "ggml-large-v3-turbo.bin",
        size: 1_624_555_275,
        sha256: "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69",
    },
    WhisperDownloadSpec {
        model_name: "large-v3",
        file_name: "ggml-large-v3.bin",
        size: 3_095_033_483,
        sha256: "64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2",
    },
    WhisperDownloadSpec {
        model_name: "tiny-q5_1",
        file_name: "ggml-tiny-q5_1.bin",
        size: 32_152_673,
        sha256: "818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7",
    },
    WhisperDownloadSpec {
        model_name: "base-q5_1",
        file_name: "ggml-base-q5_1.bin",
        size: 59_707_625,
        sha256: "422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898",
    },
    WhisperDownloadSpec {
        model_name: "small-q5_1",
        file_name: "ggml-small-q5_1.bin",
        size: 190_085_487,
        sha256: "ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb",
    },
    WhisperDownloadSpec {
        model_name: "medium-q5_0",
        file_name: "ggml-medium-q5_0.bin",
        size: 539_212_467,
        sha256: "19fea4b380c3a618ec4723c3eef2eb785ffba0d0538cf43f8f235e7b3b34220f",
    },
    WhisperDownloadSpec {
        model_name: "large-v3-turbo-q5_0",
        file_name: "ggml-large-v3-turbo-q5_0.bin",
        size: 574_041_195,
        sha256: "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
    },
    WhisperDownloadSpec {
        model_name: "large-v3-q5_0",
        file_name: "ggml-large-v3-q5_0.bin",
        size: 1_081_140_203,
        sha256: "d75795ecff3f83b5faa89d1900604ad8c780abd5739fae406de19f23ecd98ad1",
    },
];

fn whisper_download_spec(model_name: &str) -> Option<WhisperDownloadSpec> {
    WHISPER_DOWNLOAD_SPECS
        .iter()
        .find(|spec| spec.model_name == model_name)
        .copied()
}

pub fn registered_whisper_memory_mib(model_name: &str) -> Option<(u64, u64)> {
    let spec = whisper_download_spec(model_name)?;
    let fixed = spec.size.div_ceil(1024 * 1024);
    // Decoder/KV/compute buffers follow the model architecture rather than the
    // quantized file size. Values include headroom over observed whisper.cpp
    // allocations so the Pipeline budget is not just the weight-file size.
    let worker = if model_name.starts_with("tiny") {
        192
    } else if model_name.starts_with("base") {
        256
    } else if model_name.starts_with("small") {
        384
    } else if model_name.starts_with("medium") {
        512
    } else if model_name.contains("turbo") {
        512
    } else {
        640
    };
    Some((fixed, worker))
}

fn whisper_model_urls(spec: WhisperDownloadSpec) -> [String; 2] {
    [
        format!(
            "https://www.modelscope.cn/api/v1/models/iceCream2025/whisper.cpp/repo?Revision={WHISPER_MODELSCOPE_REVISION}&FilePath={}",
            spec.file_name
        ),
        format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/{WHISPER_HUGGINGFACE_REVISION}/{}",
            spec.file_name
        ),
    ]
}

fn verify_whisper_file(path: &Path, spec: WhisperDownloadSpec) -> Result<()> {
    verify_file_integrity(path, spec.file_name, spec.size, spec.sha256)
}

fn verify_file_integrity(
    path: &Path,
    display_name: &str,
    expected_size: u64,
    expected_sha256: &str,
) -> Result<()> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() != expected_size {
        return Err(anyhow!(
            "{} has {} bytes; expected {}",
            display_name,
            metadata.len(),
            expected_size
        ));
    }
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    let actual = format!("{:x}", hasher.finalize());
    if actual != expected_sha256 {
        return Err(anyhow!(
            "{} checksum mismatch: expected {}, got {}",
            display_name,
            expected_sha256,
            actual
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedWhisperModel {
    pub model_id: String,
    pub name: String,
    pub path: String,
}

pub async fn import_registered_whisper_file(
    models_dir: &Path,
    source: &Path,
) -> Result<ImportedWhisperModel> {
    let file_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("The selected Whisper model has an invalid file name"))?;
    let spec = WHISPER_DOWNLOAD_SPECS
        .iter()
        .find(|candidate| candidate.file_name == file_name)
        .copied()
        .ok_or_else(|| anyhow!("This file name does not match a registered Whisper model"))?;
    let source_for_validation = source.to_path_buf();
    tokio::task::spawn_blocking(move || verify_whisper_file(&source_for_validation, spec))
        .await
        .map_err(|error| anyhow!("Whisper model verification task failed: {error}"))??;

    tokio::fs::create_dir_all(models_dir).await?;
    let destination = models_dir.join(spec.file_name);
    if source == destination {
        return Ok(ImportedWhisperModel {
            model_id: spec.model_name.into(),
            name: spec.model_name.into(),
            path: destination.to_string_lossy().into_owned(),
        });
    }
    let transaction_id = uuid::Uuid::new_v4();
    let staging = models_dir.join(format!(".{}.import-{transaction_id}", spec.file_name));
    let backup = models_dir.join(format!(".{}.backup-{transaction_id}", spec.file_name));
    let result = async {
        tokio::fs::copy(source, &staging).await?;
        let staging_for_validation = staging.clone();
        tokio::task::spawn_blocking(move || verify_whisper_file(&staging_for_validation, spec))
            .await
            .map_err(|error| anyhow!("Whisper import verification task failed: {error}"))??;
        if destination.exists() {
            tokio::fs::rename(&destination, &backup).await?;
        }
        if let Err(error) = tokio::fs::rename(&staging, &destination).await {
            if backup.exists() {
                let _ = tokio::fs::rename(&backup, &destination).await;
            }
            return Err(error.into());
        }
        if backup.exists() {
            tokio::fs::remove_file(&backup).await?;
        }
        Ok::<(), anyhow::Error>(())
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&staging).await;
        if backup.exists() && !destination.exists() {
            let _ = tokio::fs::rename(&backup, &destination).await;
        }
    }
    result?;
    Ok(ImportedWhisperModel {
        model_id: spec.model_name.into(),
        name: spec.model_name.into(),
        path: destination.to_string_lossy().into_owned(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelStatus {
    Available,
    Missing,
    Downloading {
        progress: u8,
    },
    Error(String),
    Corrupted {
        file_size: u64,
        expected_min_size: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub path: PathBuf,
    pub size_mb: u32,
    pub accuracy: String,
    pub speed: String,
    pub status: ModelStatus,
    pub description: String,
}

pub struct WhisperEngine {
    models_dir: PathBuf,
    current_context: Arc<RwLock<Option<WhisperContext>>>,
    current_model: Arc<RwLock<Option<String>>>,
    available_models: Arc<RwLock<HashMap<String, ModelInfo>>>,
    // State tracking for smart logging
    last_transcription_was_short: Arc<RwLock<bool>>,
    short_audio_warning_logged: Arc<RwLock<bool>>,
    // Performance optimization: reduce logging frequency
    transcription_count: Arc<RwLock<u64>>,
    // Download cancellation tracking
    cancel_download_flag: Arc<RwLock<Option<String>>>, // Model name being cancelled
    // Active downloads tracking to prevent concurrent downloads
    active_downloads: Arc<RwLock<HashSet<String>>>, // Set of models currently being downloaded
}

impl WhisperEngine {
    /// Detect available GPU acceleration capabilities
    fn detect_gpu_acceleration() -> bool {
        match WhisperCompiledBackend::current() {
            WhisperCompiledBackend::Metal => {
                log::info!("macOS detected - attempting to enable Metal GPU acceleration");
                true
            }
            WhisperCompiledBackend::Cuda => {
                log::info!("CUDA feature enabled - attempting GPU acceleration");
                true
            }
            WhisperCompiledBackend::Vulkan => {
                log::info!("Vulkan feature enabled - attempting GPU acceleration");
                true
            }
            WhisperCompiledBackend::HipBlas => {
                log::info!("HIP BLAS feature enabled - attempting GPU acceleration");
                true
            }
            WhisperCompiledBackend::Cpu => {
                log::info!("No GPU acceleration features detected - using CPU processing");
                false
            }
        }
    }

    pub fn new() -> Result<Self> {
        Self::new_with_models_dir(None)
    }

    /// Create a new WhisperEngine with optional custom models directory
    /// If models_dir is None, uses default location (app data dir for production, local for dev)
    pub fn new_with_models_dir(models_dir: Option<PathBuf>) -> Result<Self> {
        // PERFORMANCE: Suppress verbose whisper.cpp and Metal logs
        // These C library logs bypass Rust logging and clutter output
        // Set environment variables to reduce C library verbosity
        std::env::set_var("GGML_METAL_LOG_LEVEL", "1"); // 0=off, 1=error, 2=warn, 3=info
        std::env::set_var("WHISPER_LOG_LEVEL", "1"); // Reduce whisper.cpp verbosity

        let models_dir = if let Some(dir) = models_dir {
            // Use provided directory (for production with app_data_dir)
            dir
        } else {
            // Fallback: determine based on debug/release mode
            let current_dir = std::env::current_dir()
                .map_err(|e| anyhow!("Failed to get current directory: {}", e))?;

            // Development: Use frontend/models or backend directories
            // Production: Use system directories (should be overridden by caller)
            if cfg!(debug_assertions) {
                // Development mode - try frontend and backend directories
                if current_dir.join("models").exists() {
                    current_dir.join("models")
                } else if current_dir.join("../models").exists() {
                    current_dir.join("../models")
                } else if current_dir
                    .join("backend/whisper-server-package/models")
                    .exists()
                {
                    current_dir.join("backend/whisper-server-package/models")
                } else if current_dir
                    .join("../backend/whisper-server-package/models")
                    .exists()
                {
                    current_dir.join("../backend/whisper-server-package/models")
                } else {
                    // Create models directory in current directory for development
                    current_dir.join("models")
                }
            } else {
                // Production mode fallback (shouldn't reach here, caller should provide path)
                log::warn!("WhisperEngine: No models directory provided, using fallback path");
                dirs::data_dir()
                    .or_else(|| dirs::home_dir())
                    .ok_or_else(|| anyhow!("Could not find system data directory"))?
                    .join("Mingtily")
                    .join("models")
            }
        };

        log::info!(
            "WhisperEngine using models directory: {}",
            models_dir.display()
        );
        log::info!("Debug mode: {}", cfg!(debug_assertions));

        // Log acceleration capabilities
        let gpu_support = Self::detect_gpu_acceleration();
        log::info!(
            "Hardware acceleration support: {}",
            if gpu_support { "enabled" } else { "disabled" }
        );

        #[cfg(feature = "metal")]
        log::info!("Apple Metal GPU support: enabled");

        #[cfg(feature = "openblas")]
        log::info!("OpenBLAS CPU optimization: enabled");

        #[cfg(feature = "coreml")]
        log::info!("Apple CoreML support: enabled");

        #[cfg(feature = "cuda")]
        log::info!("NVIDIA CUDA support: enabled");

        #[cfg(feature = "vulkan")]
        log::info!("Vulkan GPU support: enabled");

        #[cfg(feature = "openmp")]
        log::info!("OpenMP parallel processing: enabled");

        let engine = Self {
            models_dir,
            current_context: Arc::new(RwLock::new(None)),
            current_model: Arc::new(RwLock::new(None)),
            available_models: Arc::new(RwLock::new(HashMap::new())),
            // Initialize state tracking
            last_transcription_was_short: Arc::new(RwLock::new(false)),
            short_audio_warning_logged: Arc::new(RwLock::new(false)),
            // Performance optimization: reduce logging frequency
            transcription_count: Arc::new(RwLock::new(0)),
            // Initialize cancellation tracking
            cancel_download_flag: Arc::new(RwLock::new(None)),
            // Initialize active downloads tracking
            active_downloads: Arc::new(RwLock::new(HashSet::new())),
        };

        Ok(engine)
    }

    pub async fn discover_models(&self) -> Result<Vec<ModelInfo>> {
        let models_dir = &self.models_dir;
        let mut models = Vec::new();
        // Use centralized model catalog from config.rs
        let model_configs = WHISPER_MODEL_CATALOG;

        for &(name, filename, size_mb, accuracy, speed, description) in model_configs {
            let model_path = models_dir.join(filename);
            let status = if model_path.exists() {
                // Check if file size is reasonable (at least 1MB for a valid model)
                match std::fs::metadata(&model_path) {
                    Ok(metadata) => {
                        let file_size_bytes = metadata.len();
                        let file_size_mb = file_size_bytes / (1024 * 1024);
                        let expected_min_size_mb = (size_mb as f64 * 0.9) as u64; // Allow 90% of expected size as minimum for more accurate corruption detection

                        if file_size_mb >= expected_min_size_mb && file_size_mb > 1 {
                            // File size looks good, but let's also check if it's a valid GGML file
                            match self.validate_model_file(&model_path).await {
                                Ok(_) => ModelStatus::Available,
                                Err(_) => {
                                    log::warn!("Model file {} has correct size but appears corrupted (failed validation)",
                                             filename);
                                    ModelStatus::Corrupted {
                                        file_size: file_size_bytes,
                                        expected_min_size: (expected_min_size_mb * 1024 * 1024)
                                            as u64,
                                    }
                                }
                            }
                        } else if file_size_mb > 0 {
                            // File exists but is smaller than expected
                            // Check if this model is currently being downloaded
                            let models_guard = self.available_models.read().await;
                            if let Some(existing_model) = models_guard.get(name) {
                                match &existing_model.status {
                                    ModelStatus::Downloading { progress } => {
                                        log::debug!("Model {} appears to be downloading ({} MB so far, {}% complete)",
                                                  filename, file_size_mb, progress);
                                        ModelStatus::Downloading {
                                            progress: *progress,
                                        }
                                    }
                                    _ => {
                                        log::warn!("Model file {} exists but is corrupted ({} MB, expected ~{} MB)",
                                                 filename, file_size_mb, size_mb);
                                        ModelStatus::Corrupted {
                                            file_size: file_size_bytes,
                                            expected_min_size: (expected_min_size_mb * 1024 * 1024)
                                                as u64,
                                        }
                                    }
                                }
                            } else {
                                log::warn!("Model file {} exists but is corrupted ({} MB, expected ~{} MB)",
                                         filename, file_size_mb, size_mb);
                                ModelStatus::Corrupted {
                                    file_size: file_size_bytes,
                                    expected_min_size: (expected_min_size_mb * 1024 * 1024) as u64,
                                }
                            }
                        } else {
                            ModelStatus::Missing
                        }
                    }
                    Err(_) => ModelStatus::Missing,
                }
            } else {
                ModelStatus::Missing
            };

            let model_info = ModelInfo {
                name: name.to_string(),
                path: model_path,
                size_mb: size_mb as u32,
                accuracy: accuracy.to_string(),
                speed: speed.to_string(),
                status,
                description: description.to_string(),
            };

            models.push(model_info);
        }

        // Update internal cache
        let mut available_models = self.available_models.write().await;
        available_models.clear();
        for model in &models {
            available_models.insert(model.name.clone(), model.clone());
        }

        Ok(models)
    }

    pub async fn load_model(&self, model_name: &str) -> Result<()> {
        let models = self.available_models.read().await;
        let model_info = models
            .get(model_name)
            .ok_or_else(|| anyhow!("Model {} not found", model_name))?;

        match model_info.status {
            ModelStatus::Available => {
                // FIX 5: Check if this model is already loaded
                if let Some(current_model) = self.current_model.read().await.as_ref() {
                    if current_model == model_name {
                        log::info!("Model {} is already loaded, skipping reload", model_name);
                        return Ok(());
                    }

                    // FIX 5: Unload current model before loading new one
                    log::info!(
                        "Unloading current model '{}' before loading '{}'",
                        current_model,
                        model_name
                    );
                    self.unload_model().await;
                }

                log::info!("Loading model: {}", model_name);

                // PERFORMANCE OPTIMIZATION: Use comprehensive hardware profile for optimal GPU configuration
                let hardware_profile = crate::audio::HardwareProfile::detect();
                let adaptive_config = hardware_profile.get_whisper_config();
                let acceleration = whisper_context_acceleration_for(
                    WhisperCompiledBackend::current(),
                    hardware_profile.gpu_type,
                    hardware_profile.performance_tier,
                );

                let context_param = WhisperContextParameters {
                    use_gpu: acceleration.use_gpu,
                    gpu_device: acceleration.gpu_device,
                    flash_attn: acceleration.flash_attn,
                    ..Default::default()
                };

                log::info!(
                    "Whisper acceleration decision: compiled_backend={} runtime_detected_gpu={:?} use_gpu={} flash_attn={} gpu_device={}",
                    acceleration.compiled_backend.as_str(),
                    acceleration.runtime_detected_gpu,
                    acceleration.use_gpu,
                    acceleration.flash_attn,
                    acceleration.gpu_device,
                );

                // PERFORMANCE: Suppress verbose C library logs during model loading
                // This hides the excessive Metal/GGML initialization logs in release builds
                let ctx = {
                    // let _suppressor = crate::whisper_engine::StderrSuppressor::new();

                    // Load whisper context with hardware-optimized parameters
                    WhisperContext::new_with_params(
                        &model_info.path.to_string_lossy(),
                        context_param,
                    )
                    .map_err(|e| anyhow!("Failed to load model {}: {}", model_name, e))?
                    // Suppressor dropped here, stderr restored
                };

                // Update current context and model
                *self.current_context.write().await = Some(ctx);
                *self.current_model.write().await = Some(model_name.to_string());

                // Enhanced acceleration status reporting
                let acceleration_status = acceleration.status_label();

                log::info!("Successfully loaded model: {} with {} (Performance Tier: {:?}, Beam Size: {}, Threads: {:?})",
                          model_name, acceleration_status, hardware_profile.performance_tier,
                          adaptive_config.beam_size, adaptive_config.max_threads);
                Ok(())
            }
            ModelStatus::Missing => Err(anyhow!("Model {} is not downloaded", model_name)),
            ModelStatus::Downloading { .. } => {
                Err(anyhow!("Model {} is currently downloading", model_name))
            }
            ModelStatus::Error(ref err) => Err(anyhow!("Model {} has error: {}", model_name, err)),
            ModelStatus::Corrupted { .. } => Err(anyhow!(
                "Model {} is corrupted and cannot be loaded",
                model_name
            )),
        }
    }

    pub async fn unload_model(&self) -> bool {
        let mut ctx_guard = self.current_context.write().await;
        let unloaded = ctx_guard.take().is_some();
        if unloaded {
            log::info!("📉Whisper model unloaded");
        }

        let mut model_name_guard = self.current_model.write().await;
        model_name_guard.take();

        unloaded
    }

    pub async fn get_current_model(&self) -> Option<String> {
        self.current_model.read().await.clone()
    }

    pub async fn is_model_loaded(&self) -> bool {
        self.current_context.read().await.is_some()
    }

    // Enhanced function to clean repetitive text patterns and meaningless outputs
    fn clean_repetitive_text(text: &str) -> String {
        if text.is_empty() {
            return String::new();
        }

        // Check for obviously meaningless patterns first
        if Self::is_meaningless_output(text) {
            // Performance optimization: reduce meaningless output logging to debug level
            perf_debug!("Detected meaningless output, returning empty: '{}'", text);
            return String::new();
        }

        let words: Vec<&str> = text.split_whitespace().collect();
        if words.len() < 3 {
            return text.to_string();
        }

        // Enhanced repetition detection with sliding window
        let cleaned_words = Self::remove_word_repetitions(&words);

        // Remove phrase repetitions with more sophisticated detection
        let cleaned_words = Self::remove_phrase_repetitions(&cleaned_words);

        // Check for overall repetition ratio
        let final_text = cleaned_words.join(" ");
        if Self::calculate_repetition_ratio(&final_text) > 0.7 {
            // Performance optimization: reduce repetition ratio logging to debug level
            perf_debug!(
                "High repetition ratio detected, filtering out: '{}'",
                final_text
            );
            return String::new();
        }

        final_text
    }

    // Check for obviously meaningless patterns
    fn is_meaningless_output(text: &str) -> bool {
        let text_lower = text.to_lowercase();

        // Check for common meaningless patterns
        let meaningless_patterns = [
            "thank you for watching",
            "thanks for watching",
            "like and subscribe",
            "music playing",
            "applause",
            "laughter",
            "um um um",
            "uh uh uh",
            "ah ah ah",
        ];

        for pattern in &meaningless_patterns {
            if text_lower.contains(pattern) {
                return true;
            }
        }

        // Check if text is mostly the same character or very short repetitive patterns
        let unique_chars: HashSet<char> = text.chars().collect();
        if unique_chars.len() <= 3 && text.len() > 10 {
            return true;
        }

        false
    }

    // Enhanced word repetition removal
    fn remove_word_repetitions<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
        let mut cleaned_words = Vec::new();
        let mut i = 0;

        while i < words.len() {
            let current_word = words[i];
            let mut repeat_count = 1;

            // Count consecutive repetitions of the same word
            while i + repeat_count < words.len() && words[i + repeat_count] == current_word {
                repeat_count += 1;
            }

            // Be more aggressive: if word is repeated 2+ times, only keep one instance
            if repeat_count >= 2 {
                cleaned_words.push(current_word);
                i += repeat_count;
            } else {
                cleaned_words.push(current_word);
                i += 1;
            }
        }

        cleaned_words
    }

    // Enhanced phrase repetition removal with variable length detection
    fn remove_phrase_repetitions<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
        if words.len() < 4 {
            return words.to_vec();
        }

        let mut final_words = Vec::new();
        let mut i = 0;

        while i < words.len() {
            let mut phrase_found = false;

            // Check for 2-word to 5-word phrase repetitions
            for phrase_len in 2..=std::cmp::min(5, (words.len() - i) / 2) {
                if i + phrase_len * 2 <= words.len() {
                    let phrase1 = &words[i..i + phrase_len];
                    let phrase2 = &words[i + phrase_len..i + phrase_len * 2];

                    if phrase1 == phrase2 {
                        // Add the phrase once and skip the repetition
                        final_words.extend_from_slice(phrase1);
                        i += phrase_len * 2;
                        phrase_found = true;
                        break;
                    }
                }
            }

            if !phrase_found {
                final_words.push(words[i]);
                i += 1;
            }
        }

        final_words
    }

    // Calculate repetition ratio in text
    fn calculate_repetition_ratio(text: &str) -> f32 {
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.len() < 4 {
            return 0.0;
        }

        let mut word_counts = HashMap::new();
        for word in &words {
            *word_counts.entry(word.to_lowercase()).or_insert(0) += 1;
        }

        let total_words = words.len() as f32;
        let repeated_words: usize = word_counts
            .values()
            .map(|&count| if count > 1 { count - 1 } else { 0 })
            .sum();

        repeated_words as f32 / total_words
    }

    /// Transcribe audio with streaming support for partial results and adaptive quality
    pub async fn transcribe_audio_with_confidence(
        &self,
        audio_data: Vec<f32>,
        language: Option<String>,
    ) -> Result<(String, f32, bool)> {
        self.transcribe_audio_with_confidence_and_prompt(audio_data, language, None)
            .await
    }

    pub async fn transcribe_audio_with_confidence_and_prompt(
        &self,
        audio_data: Vec<f32>,
        language: Option<String>,
        initial_prompt: Option<&str>,
    ) -> Result<(String, f32, bool)> {
        let ctx_lock = self.current_context.read().await;
        let ctx = ctx_lock
            .as_ref()
            .ok_or_else(|| anyhow!("No model loaded. Please load a model first."))?;

        // Get adaptive configuration based on hardware
        let hardware_profile = crate::audio::HardwareProfile::detect();
        let adaptive_config = hardware_profile.get_whisper_config();

        // ADAPTIVE parameters - optimized for current hardware
        let mut params = FullParams::new(SamplingStrategy::BeamSearch {
            beam_size: adaptive_config.beam_size as i32,
            patience: 1.0,
        });

        // Configure with adaptive settings
        // If language is "auto" or None, use automatic language detection (pass None)
        // If language is "auto-translate", enable translation to English
        // Otherwise, use the specified language code
        let (language_code, should_translate) = match language.as_deref() {
            Some("auto") | None => (None, false),
            Some("auto-translate") => (None, true),
            Some(lang) => (Some(lang), false),
        };
        params.set_language(language_code);
        params.set_translate(should_translate);
        if let Some(prompt) = initial_prompt.filter(|prompt| !prompt.trim().is_empty()) {
            params.set_initial_prompt(prompt);
        }

        // CRITICAL: Disable timestamp tokens to prevent whisper.cpp chunking heuristics
        // The "single timestamp ending - skip entire chunk" optimization incorrectly discards
        // complete, valid transcriptions. Disabling timestamps forces whisper to return ALL text.
        params.set_no_timestamps(true); // Prevent timestamp-based segment skipping
        params.set_token_timestamps(true); // Keep for any timestamp-aware features

        // PERFORMANCE: Disable ALL whisper.cpp internal printing
        // This reduces C library log spam significantly
        params.set_print_special(false); // Don't print special tokens
        params.set_print_progress(false); // Don't print progress
        params.set_print_realtime(false); // Don't print realtime info
        params.set_print_timestamps(false); // Don't print timestamps

        // Additional suppression to reduce C library verbosity
        params.set_suppress_blank(true);
        params.set_suppress_non_speech_tokens(true);
        params.set_temperature(adaptive_config.temperature);
        params.set_max_initial_ts(1.0);
        params.set_entropy_thold(2.4);
        params.set_logprob_thold(-1.0);
        // BALANCED FIX: Lowered from 0.75 to 0.55 to allow quiet speech detection
        // Previous value was too aggressive and rejected valid quiet speech
        // 0.55 is balanced - prevents hallucinations while preserving quiet speech
        params.set_no_speech_thold(0.55);
        params.set_max_len(200);
        params.set_single_segment(false);

        // Set thread count based on hardware (if supported by whisper.cpp)
        if let Some(_max_threads) = adaptive_config.max_threads {
            // Note: whisper.cpp may or may not expose thread control through params
            // Removed debug log to reduce I/O overhead in transcription hot path
        }

        // PERFORMANCE: Suppress verbose C library logs during transcription
        // This hides whisper_full_with_state debug logs and beam search details
        let (num_segments, state) = {
            // let _suppressor = crate::whisper_engine::StderrSuppressor::new();

            let mut state = ctx.create_state()?;
            state.full(params, &audio_data)?;
            let num_segments = state.full_n_segments();

            (num_segments, state)
            // Suppressor dropped here, stderr restored
        };
        let mut result = String::new();
        let mut total_confidence = 0.0;
        let mut segment_count = 0;

        let num_segments = num_segments?;
        for i in 0..num_segments {
            let segment_text = match state.full_get_segment_text_lossy(i) {
                Ok(text) => text,
                Err(_) => continue,
            };

            // Calculate confidence based on segment length and duration (simplified approach)
            let segment_length = segment_text.len() as f32;
            let segment_confidence = if segment_length > 0.0 {
                (segment_length / 100.0).min(0.9) + 0.1 // 0.1 to 1.0 confidence based on text length
            } else {
                0.1
            };
            total_confidence += segment_confidence;
            segment_count += 1;

            let cleaned_text = segment_text.trim();
            if !cleaned_text.is_empty() {
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(cleaned_text);
            }
        }

        let final_result = result.trim().to_string();
        let cleaned_result = Self::clean_repetitive_text(&final_result);

        let avg_confidence = if segment_count > 0 {
            total_confidence / segment_count as f32
        } else {
            0.0
        };

        // Whisper runs once on an already-finalized VAD segment. Segment length
        // is not a provisional/final signal; only continuous streaming sessions
        // are allowed to emit provisional hypotheses.
        Ok((cleaned_result, avg_confidence, false))
    }

    pub async fn transcribe_audio(
        &self,
        audio_data: Vec<f32>,
        language: Option<String>,
    ) -> Result<String> {
        let ctx_lock = self.current_context.read().await;
        let ctx = ctx_lock
            .as_ref()
            .ok_or_else(|| anyhow!("No model loaded. Please load a model first."))?;

        // Get adaptive configuration based on hardware
        let hardware_profile = crate::audio::HardwareProfile::detect();
        let adaptive_config = hardware_profile.get_whisper_config();

        // ADAPTIVE parameters - optimized for current hardware
        let mut params = FullParams::new(SamplingStrategy::BeamSearch {
            beam_size: adaptive_config.beam_size as i32,
            patience: 1.0,
        });

        // Configure for good quality
        // If language is "auto" or None, use automatic language detection (pass None)
        // If language is "auto-translate", enable translation to English
        // Otherwise, use the specified language code
        let (language_code, should_translate) = match language.as_deref() {
            Some("auto") | None => (None, false),
            Some("auto-translate") => (None, true),
            Some(lang) => (Some(lang), false),
        };
        params.set_language(language_code);
        params.set_translate(should_translate);

        // CRITICAL: Disable timestamp tokens to prevent whisper.cpp chunking heuristics
        // The "single timestamp ending - skip entire chunk" optimization incorrectly discards
        // complete, valid transcriptions. Disabling timestamps forces whisper to return ALL text.
        params.set_no_timestamps(true); // Prevent timestamp-based segment skipping
        params.set_token_timestamps(true); // Keep for any timestamp-aware features

        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        // BALANCED settings - good quality with reasonable speed
        params.set_suppress_blank(true);
        params.set_suppress_non_speech_tokens(true);
        params.set_temperature(0.3); // Lower than 0.4 for consistency, higher than 0.0 for quality
        params.set_max_initial_ts(1.0);
        params.set_entropy_thold(2.4);
        params.set_logprob_thold(-1.0);
        // BALANCED FIX: Lowered from 0.75 to 0.55 to allow quiet speech detection
        // Previous value was too aggressive and rejected valid quiet speech
        // 0.55 is balanced - prevents hallucinations while preserving quiet speech
        params.set_no_speech_thold(0.55);

        // Reasonable length limits
        params.set_max_len(200); // Reasonable length
        params.set_single_segment(false); // Allow multiple segments for better accuracy

        // Note: compression_ratio_threshold would be ideal but not available in current whisper-rs
        // This would help detect repetitive outputs: params.set_compression_ratio_threshold(2.4);

        // Duration-based optimization is handled by beam search parameters
        let duration_seconds = audio_data.len() as f64 / 16000.0; // Assuming 16kHz
        let is_short_audio = duration_seconds < 1.0;

        // Smart logging based on audio duration and previous states
        let mut should_log_transcription = true;
        let mut should_log_short_warning = false;

        if is_short_audio {
            let last_was_short = *self.last_transcription_was_short.read().await;
            let warning_logged = *self.short_audio_warning_logged.read().await;

            if !warning_logged {
                should_log_short_warning = true;
                *self.short_audio_warning_logged.write().await = true;
            }

            // Only log transcription start if it's the first short audio or previous wasn't short
            should_log_transcription = !last_was_short;

            *self.last_transcription_was_short.write().await = true;
        } else {
            let last_was_short = *self.last_transcription_was_short.read().await;

            // Always log when transitioning from short to normal audio
            if last_was_short {
                log::info!("Audio duration normalized, resuming transcription");
                *self.short_audio_warning_logged.write().await = false;
            }

            *self.last_transcription_was_short.write().await = false;
        }

        if should_log_short_warning {
            log::warn!("Audio duration is short ({:.1}s < 1.0s). Consider padding the input audio with silence. Further short audio warnings will be suppressed.", duration_seconds);
        }

        // Performance optimization: reduce transcription start logging frequency
        let transcription_count = {
            let mut count = self.transcription_count.write().await;
            *count += 1;
            *count
        };

        // Only log every 10th transcription or significant audio (>10s) to reduce I/O overhead
        if should_log_transcription && (transcription_count % 10 == 0 || duration_seconds > 10.0) {
            log::info!(
                "Starting transcription #{} of {} samples ({:.1}s duration)",
                transcription_count,
                audio_data.len(),
                duration_seconds
            );
        }
        let mut state = ctx.create_state()?;
        state.full(params, &audio_data)?;

        // Extract text with improved segment handling
        let num_segments = state.full_n_segments()?;

        // Performance optimization: reduce segment completion logging
        // Only log for significant transcriptions to avoid I/O overhead
        if (should_log_transcription || num_segments > 0)
            && (num_segments > 3 || duration_seconds > 5.0)
        {
            perf_debug!(
                "Transcription #{} completed with {} segments ({:.1}s)",
                transcription_count,
                num_segments,
                duration_seconds
            );
        }
        let mut result = String::new();

        for i in 0..num_segments {
            let segment_text = match state.full_get_segment_text_lossy(i) {
                Ok(text) => text,
                Err(_) => continue,
            };

            let _start_time = state.full_get_segment_t0(i).unwrap_or(0);
            let _end_time = state.full_get_segment_t1(i).unwrap_or(0);

            // Performance optimization: remove per-segment debug logging
            // This was causing significant I/O overhead during transcription
            // Only log segments for very long audio (>30s) or when explicitly debugging
            if duration_seconds > 30.0 {
                perf_trace!(
                    "Segment {} ({:.2}s-{:.2}s): '{}'",
                    i,
                    _start_time as f64 / 100.0,
                    _end_time as f64 / 100.0,
                    segment_text
                );
            }

            // Clean and append segment text
            let cleaned_text = segment_text.trim();
            if !cleaned_text.is_empty() {
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(cleaned_text);
            }
        }

        let final_result = result.trim().to_string();

        // Check for repetition loops and clean them up
        let cleaned_result = Self::clean_repetitive_text(&final_result);

        // Performance optimization: smart logging for transcription results
        if cleaned_result.is_empty() {
            // Only log empty results occasionally to reduce spam
            if should_log_transcription && transcription_count % 20 == 0 {
                perf_debug!(
                    "Transcription #{} result is empty - no speech detected",
                    transcription_count
                );
            }
        } else {
            if cleaned_result != final_result {
                log::info!(
                    "Cleaned repetitive transcription #{}: {} chars -> {} chars",
                    transcription_count,
                    final_result.chars().count(),
                    cleaned_result.chars().count()
                );
            }
            // Reduce successful transcription logging frequency
            // Only log every 5th result or significant results (>50 chars) to reduce I/O overhead
            if transcription_count % 5 == 0 || cleaned_result.len() > 50 || duration_seconds > 10.0
            {
                log::info!(
                    "Transcription #{} completed: {} chars",
                    transcription_count,
                    cleaned_result.chars().count()
                );
            } else {
                perf_debug!(
                    "Transcription #{} completed: {} chars",
                    transcription_count,
                    cleaned_result.chars().count()
                );
            }
        }

        Ok(cleaned_result)
    }

    pub async fn get_models_directory(&self) -> PathBuf {
        self.models_dir.clone()
    }

    /// Validate if a model file is a valid GGML file by checking its header
    async fn validate_model_file(&self, model_path: &PathBuf) -> Result<()> {
        use tokio::io::AsyncReadExt;

        let mut file = fs::File::open(model_path)
            .await
            .map_err(|e| anyhow!("Failed to open model file: {}", e))?;

        // Read the first 8 bytes to check for GGML magic number
        let mut buffer = [0u8; 8];
        file.read_exact(&mut buffer)
            .await
            .map_err(|e| anyhow!("Failed to read model file header: {}", e))?;

        // Check for GGML magic number (various versions and endianness)
        if buffer.starts_with(b"ggml")
            || buffer.starts_with(b"GGUF")
            || buffer.starts_with(b"ggmf")
            || buffer.starts_with(b"lmgg")
            || buffer.starts_with(b"FUGU")
            || buffer.starts_with(b"fmgg")
        {
            Ok(())
        } else {
            Err(anyhow!(
                "Invalid model file: missing GGML/GGUF magic number. Found: {:?}",
                String::from_utf8_lossy(&buffer[..4])
            ))
        }
    }

    pub async fn delete_model(&self, model_name: &str) -> Result<String> {
        log::info!("Attempting to delete model: {}", model_name);

        // Get model info to find the file path
        let model_info = {
            let models = self.available_models.read().await;
            models.get(model_name).cloned()
        };

        let model_info = model_info.ok_or_else(|| anyhow!("Model '{}' not found", model_name))?;

        // Check if model is corrupted before allowing deletion
        log::info!("Model '{}' has status: {:?}", model_name, model_info.status);
        match &model_info.status {
            ModelStatus::Corrupted {
                file_size,
                expected_min_size,
            } => {
                log::info!(
                    "Deleting corrupted model '{}' (file size: {} bytes, expected min: {} bytes)",
                    model_name,
                    file_size,
                    expected_min_size
                );

                // Delete the file
                if model_info.path.exists() {
                    fs::remove_file(&model_info.path).await.map_err(|e| {
                        anyhow!(
                            "Failed to delete file '{}': {}",
                            model_info.path.display(),
                            e
                        )
                    })?;
                    log::info!(
                        "Successfully deleted corrupted file: {}",
                        model_info.path.display()
                    );
                } else {
                    log::warn!(
                        "File '{}' does not exist, nothing to delete",
                        model_info.path.display()
                    );
                }

                // Update model status to Missing
                {
                    let mut models = self.available_models.write().await;
                    if let Some(model) = models.get_mut(model_name) {
                        model.status = ModelStatus::Missing;
                    }
                }

                Ok(format!(
                    "Successfully deleted corrupted model '{}'",
                    model_name
                ))
            }
            ModelStatus::Available => {
                // Allow deletion of available models for testing/cleanup
                log::info!("Deleting available model '{}' (for cleanup)", model_name);

                if model_info.path.exists() {
                    fs::remove_file(&model_info.path).await.map_err(|e| {
                        anyhow!(
                            "Failed to delete file '{}': {}",
                            model_info.path.display(),
                            e
                        )
                    })?;
                    log::info!(
                        "Successfully deleted available model file: {}",
                        model_info.path.display()
                    );
                } else {
                    log::warn!(
                        "File '{}' does not exist, nothing to delete",
                        model_info.path.display()
                    );
                }

                // Update model status to Missing
                {
                    let mut models = self.available_models.write().await;
                    if let Some(model) = models.get_mut(model_name) {
                        model.status = ModelStatus::Missing;
                    }
                }

                Ok(format!("Successfully deleted model '{}'", model_name))
            }
            _ => Err(anyhow!(
                "Can only delete corrupted or available models. Model '{}' has status: {:?}",
                model_name,
                model_info.status
            )),
        }
    }

    pub async fn download_model(
        &self,
        model_name: &str,
        progress_callback: Option<Box<dyn Fn(u8) + Send + Sync>>,
    ) -> Result<()> {
        let spec = whisper_download_spec(model_name)
            .ok_or_else(|| anyhow!("Unsupported model: {}", model_name))?;
        let model_urls = whisper_model_urls(spec);
        self.download_model_from_urls(model_name, &model_urls, progress_callback)
            .await
    }

    async fn download_model_from_urls(
        &self,
        model_name: &str,
        model_urls: &[String],
        progress_callback: Option<Box<dyn Fn(u8) + Send + Sync>>,
    ) -> Result<()> {
        log::info!("Starting download for model: {}", model_name);

        // Check and register atomically so two simultaneous requests cannot both start.
        {
            let mut active = self.active_downloads.write().await;
            if !active.insert(model_name.to_string()) {
                log::warn!("Download already in progress for model: {}", model_name);
                return Err(anyhow!(
                    "Download already in progress for model: {}",
                    model_name
                ));
            }
        }

        {
            let mut cancel_flag = self.cancel_download_flag.write().await;
            if cancel_flag.as_deref() == Some(model_name) {
                *cancel_flag = None;
            }
        }

        self.update_model_status(model_name, ModelStatus::Downloading { progress: 0 })
            .await;

        let mut failures = Vec::new();
        let mut result = Err(anyhow!("No Whisper download sources configured"));
        for (index, model_url) in model_urls.iter().enumerate() {
            log::info!(
                "Trying Whisper model source {}/{} for {}: {}",
                index + 1,
                model_urls.len(),
                model_name,
                model_url
            );
            match self
                .download_model_inner(model_name, model_url, progress_callback.as_deref())
                .await
            {
                Ok(()) => {
                    result = Ok(());
                    break;
                }
                Err(error) if is_cancelled_download(&error) => {
                    result = Err(error);
                    break;
                }
                Err(error) => {
                    log::warn!("Whisper source failed for {}: {error:#}", model_name);
                    failures.push(error.to_string());
                    result = Err(anyhow!(
                        "All Whisper download sources failed: {}",
                        failures.join(" | ")
                    ));
                }
            }
        }

        // Every terminal path must release the registration. Previously network and disk
        // errors returned early and made Retry fail with "already in progress" forever.
        {
            let mut active = self.active_downloads.write().await;
            active.remove(model_name);
        }
        {
            let mut cancel_flag = self.cancel_download_flag.write().await;
            if cancel_flag.as_deref() == Some(model_name) {
                *cancel_flag = None;
            }
        }

        match &result {
            Ok(()) => {
                let file_path = self.models_dir.join(format!("ggml-{}.bin", model_name));
                let mut models = self.available_models.write().await;
                if let Some(model_info) = models.get_mut(model_name) {
                    model_info.status = ModelStatus::Available;
                    model_info.path = file_path;
                }
            }
            Err(error) if is_cancelled_download(error) => {
                self.update_model_status(model_name, ModelStatus::Missing)
                    .await;
            }
            Err(error) => {
                self.update_model_status(model_name, ModelStatus::Error(error.to_string()))
                    .await;
            }
        }

        result
    }

    #[cfg(test)]
    async fn download_model_from_url(
        &self,
        model_name: &str,
        model_url: &str,
        progress_callback: Option<Box<dyn Fn(u8) + Send + Sync>>,
    ) -> Result<()> {
        self.download_model_from_urls(model_name, &[model_url.to_string()], progress_callback)
            .await
    }

    async fn download_model_inner(
        &self,
        model_name: &str,
        model_url: &str,
        progress_callback: Option<&(dyn Fn(u8) + Send + Sync)>,
    ) -> Result<()> {
        log::info!("Model URL for {}: {}", model_name, model_url);

        // Generate correct filename - all models follow ggml-{model_name}.bin pattern
        let filename = format!("ggml-{}.bin", model_name);
        let file_path = self.models_dir.join(&filename);
        let partial_path = partial_download_path(&file_path);

        log::info!("Downloading to file path: {}", file_path.display());

        // Create models directory if it doesn't exist
        if !self.models_dir.exists() {
            fs::create_dir_all(&self.models_dir)
                .await
                .map_err(|e| anyhow!("Failed to create models directory: {}", e))?;
        }

        if file_path.exists() {
            let exact_file_is_valid = whisper_download_spec(model_name)
                .map(|spec| verify_whisper_file(&file_path, spec).is_ok())
                .unwrap_or_else(|| model_file_is_complete(model_name, &file_path));
            if self.validate_model_file(&file_path).await.is_ok() && exact_file_is_valid {
                log::info!(
                    "Model already exists and passed validation: {}",
                    file_path.display()
                );
                if let Some(callback) = progress_callback {
                    callback(100);
                }
                return Ok(());
            }

            // Releases before this fix wrote partial data to the final filename. Preserve a
            // valid GGML prefix as resumable data; discard unrelated/corrupt content.
            if !partial_path.exists() && self.validate_model_file(&file_path).await.is_ok() {
                fs::rename(&file_path, &partial_path)
                    .await
                    .map_err(|e| anyhow!("Failed to preserve partial download: {}", e))?;
            } else {
                fs::remove_file(&file_path)
                    .await
                    .map_err(|e| anyhow!("Failed to remove incomplete model file: {}", e))?;
            }
        }

        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| anyhow!("Failed to create download client: {}", e))?;

        let mut resume_offset = match fs::metadata(&partial_path).await {
            Ok(metadata) => metadata.len(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
            Err(error) => return Err(anyhow!("Failed to inspect partial download: {}", error)),
        };

        if self.is_download_cancelled(model_name).await {
            remove_file_if_exists(&partial_path).await?;
            return Err(anyhow!("Download cancelled by user"));
        }

        log::info!(
            "Sending GET request to {} (resume offset: {} bytes)",
            model_url,
            resume_offset
        );
        let mut request = client.get(model_url);
        if resume_offset > 0 {
            request = request.header(RANGE, format!("bytes={}-", resume_offset));
        }
        let mut response = request
            .send()
            .await
            .map_err(|e| anyhow!("Failed to start download: {}", e))?;

        if self.is_download_cancelled(model_name).await {
            remove_file_if_exists(&partial_path).await?;
            return Err(anyhow!("Download cancelled by user"));
        }

        // A stale or already-complete partial can produce 416. Restart cleanly once.
        if resume_offset > 0 && response.status() == StatusCode::RANGE_NOT_SATISFIABLE {
            log::warn!("Server rejected the saved download range; restarting from zero");
            resume_offset = 0;
            remove_file_if_exists(&partial_path).await?;
            response = client
                .get(model_url)
                .send()
                .await
                .map_err(|e| anyhow!("Failed to restart download: {}", e))?;
        }

        log::info!("Received response with status: {}", response.status());
        if !response.status().is_success() {
            return Err(anyhow!(
                "Download failed with status: {}",
                response.status()
            ));
        }

        let is_partial_response = response.status() == StatusCode::PARTIAL_CONTENT;
        if resume_offset > 0 && !is_partial_response {
            log::warn!("Server ignored the Range request; restarting the local file from zero");
            resume_offset = 0;
        }

        let response_size = response.content_length().unwrap_or(0);
        let total_size = if is_partial_response {
            response
                .headers()
                .get(CONTENT_RANGE)
                .and_then(|value| value.to_str().ok())
                .and_then(parse_content_range_total)
                .unwrap_or_else(|| resume_offset.saturating_add(response_size))
        } else {
            response_size
        };
        if let Some(expected_size) = whisper_download_spec(model_name).map(|spec| spec.size) {
            if total_size != 0 && total_size != expected_size {
                return Err(anyhow!(
                    "Download source reported {} bytes for {}; expected {}",
                    total_size,
                    model_name,
                    expected_size
                ));
            }
        }
        log::info!(
            "Response successful, content length: {} bytes ({:.1} MB)",
            total_size,
            total_size as f64 / (1024.0 * 1024.0)
        );

        if total_size == 0 {
            log::warn!("Content length is 0 or unknown - download may not show accurate progress");
        }

        let mut options = fs::OpenOptions::new();
        options.create(true).write(true);
        if resume_offset > 0 && is_partial_response {
            options.append(true);
        } else {
            options.truncate(true);
        }
        let mut file = options
            .open(&partial_path)
            .await
            .map_err(|e| anyhow!("Failed to open partial download: {}", e))?;

        log::info!("Partial download file ready at: {}", partial_path.display());

        // Stream download with real progress reporting
        log::info!("Starting streaming download...");
        log::info!(
            "Expected size: {:.1} MB",
            total_size as f64 / (1024.0 * 1024.0)
        );

        use futures_util::StreamExt;
        let mut stream = response.bytes_stream();
        let mut downloaded = resume_offset;
        let initial_progress = download_progress(downloaded, total_size);
        let mut last_progress_report = initial_progress;
        let mut last_report_time = std::time::Instant::now();

        if let Some(callback) = progress_callback {
            callback(initial_progress);
        }

        while let Some(chunk_result) = stream.next().await {
            // Check for cancellation before processing chunk
            if self.is_download_cancelled(model_name).await {
                log::info!("Download cancelled for {}", model_name);
                drop(file);
                remove_file_if_exists(&partial_path).await?;
                return Err(anyhow!("Download cancelled by user"));
            }

            let chunk = chunk_result.map_err(|e| anyhow!("Failed to read chunk: {}", e))?;

            file.write_all(&chunk)
                .await
                .map_err(|e| anyhow!("Failed to write chunk to file: {}", e))?;

            downloaded += chunk.len() as u64;

            // Calculate progress
            let progress = download_progress(downloaded, total_size);

            // Report progress every 1% or every 2 seconds for better UI responsiveness
            let time_since_last_report = last_report_time.elapsed().as_secs();
            if progress >= last_progress_report + 1
                || progress == 100
                || time_since_last_report >= 2
            {
                log::info!(
                    "Download progress: {}% ({:.1} MB / {:.1} MB)",
                    progress,
                    downloaded as f64 / (1024.0 * 1024.0),
                    total_size as f64 / (1024.0 * 1024.0)
                );

                // Update progress in model info
                self.update_model_status(model_name, ModelStatus::Downloading { progress })
                    .await;

                // Call progress callback
                if let Some(callback) = progress_callback {
                    callback(progress);
                }

                last_progress_report = progress;
                last_report_time = std::time::Instant::now();
            }
        }

        log::info!("Streaming download completed: {} bytes", downloaded);

        if self.is_download_cancelled(model_name).await {
            drop(file);
            remove_file_if_exists(&partial_path).await?;
            return Err(anyhow!("Download cancelled by user"));
        }

        if total_size > 0 && downloaded != total_size {
            return Err(anyhow!(
                "Download ended early: received {} of {} bytes",
                downloaded,
                total_size
            ));
        }

        file.flush()
            .await
            .map_err(|e| anyhow!("Failed to flush file: {}", e))?;
        file.sync_all()
            .await
            .map_err(|e| anyhow!("Failed to sync model file: {}", e))?;
        drop(file);

        if let Err(error) = self.validate_model_file(&partial_path).await {
            remove_file_if_exists(&partial_path).await?;
            return Err(error);
        }
        if let Some(spec) = whisper_download_spec(model_name) {
            let verification_path = partial_path.clone();
            if let Err(error) =
                tokio::task::spawn_blocking(move || verify_whisper_file(&verification_path, spec))
                    .await
                    .map_err(|error| anyhow!("Whisper checksum task failed: {error}"))?
            {
                remove_file_if_exists(&partial_path).await?;
                return Err(error);
            }
        }

        fs::rename(&partial_path, &file_path)
            .await
            .map_err(|e| anyhow!("Failed to finalize model download: {}", e))?;

        self.update_model_status(model_name, ModelStatus::Downloading { progress: 100 })
            .await;
        if let Some(callback) = progress_callback {
            callback(100);
        }

        log::info!("Download completed for model: {}", model_name);

        Ok(())
    }

    async fn is_download_cancelled(&self, model_name: &str) -> bool {
        self.cancel_download_flag.read().await.as_deref() == Some(model_name)
    }

    async fn update_model_status(&self, model_name: &str, status: ModelStatus) {
        let mut models = self.available_models.write().await;
        if let Some(model_info) = models.get_mut(model_name) {
            model_info.status = status;
        }
    }

    pub async fn cancel_download(&self, model_name: &str) -> Result<()> {
        log::info!("Cancelling download for model: {}", model_name);

        // Set cancellation flag to interrupt the download loop
        {
            let mut cancel_flag = self.cancel_download_flag.write().await;
            *cancel_flag = Some(model_name.to_string());
        }

        // Update model status to Missing (so it can be retried)
        self.update_model_status(model_name, ModelStatus::Missing)
            .await;

        // The download loop owns the open file and performs cleanup. Keeping the active
        // registration until it exits also prevents an immediate retry from racing it.

        Ok(())
    }
}

fn partial_download_path(file_path: &Path) -> PathBuf {
    let mut name = file_path.as_os_str().to_os_string();
    name.push(".part");
    PathBuf::from(name)
}

fn parse_content_range_total(value: &str) -> Option<u64> {
    value.rsplit_once('/')?.1.parse().ok()
}

fn download_progress(downloaded: u64, total_size: u64) -> u8 {
    if total_size == 0 {
        0
    } else {
        // Reserve 100% for validation + atomic rename so the UI never announces a
        // corrupted or not-yet-finalized model as complete.
        ((downloaded.saturating_mul(100) / total_size).min(99)) as u8
    }
}

fn model_file_is_complete(model_name: &str, file_path: &Path) -> bool {
    let Some((_, _, size_mb, _, _, _)) = WHISPER_MODEL_CATALOG
        .iter()
        .find(|(name, _, _, _, _, _)| *name == model_name)
    else {
        return true;
    };
    let expected_min = ((*size_mb as f64) * 0.9 * 1024.0 * 1024.0) as u64;
    std::fs::metadata(file_path)
        .map(|metadata| metadata.len() >= expected_min)
        .unwrap_or(false)
}

async fn remove_file_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(anyhow!(
            "Failed to remove partial download {}: {}",
            path.display(),
            error
        )),
    }
}

fn is_cancelled_download(error: &anyhow::Error) -> bool {
    error.to_string().contains("Download cancelled by user")
}

#[cfg(test)]
mod download_tests {
    use super::*;
    use std::sync::atomic::{AtomicU8, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[test]
    fn every_whisper_model_prefers_pinned_modelscope_with_exact_fallback() {
        assert_eq!(WHISPER_DOWNLOAD_SPECS.len(), 12);
        for spec in WHISPER_DOWNLOAD_SPECS {
            assert_eq!(spec.sha256.len(), 64);
            let urls = whisper_model_urls(*spec);
            assert!(urls[0].contains("modelscope.cn"));
            assert!(urls[0].contains(WHISPER_MODELSCOPE_REVISION));
            assert!(urls[1].contains("huggingface.co"));
            assert!(urls[1].contains(WHISPER_HUGGINGFACE_REVISION));
        }
    }

    #[tokio::test]
    async fn failed_download_releases_active_registration_for_retry() {
        let temp = tempfile::tempdir().unwrap();
        let engine = WhisperEngine::new_with_models_dir(Some(temp.path().to_path_buf())).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let url = format!("http://{address}/model.bin");

        let first = engine.download_model_from_url("test", &url, None).await;
        assert!(first.is_err());
        assert!(!engine.active_downloads.read().await.contains("test"));

        let second = engine.download_model_from_url("test", &url, None).await;
        assert!(second.is_err());
        assert!(!second
            .unwrap_err()
            .to_string()
            .contains("already in progress"));
    }

    #[test]
    fn transfer_progress_reserves_completion_for_finalization() {
        assert_eq!(download_progress(0, 100), 0);
        assert_eq!(download_progress(50, 100), 50);
        assert_eq!(download_progress(100, 100), 99);
        assert_eq!(download_progress(200, 100), 99);
        assert_eq!(download_progress(10, 0), 0);
    }

    #[tokio::test]
    async fn offline_import_rejects_unverified_content_without_replacing_the_installed_model() {
        let source_dir = tempfile::tempdir().unwrap();
        let models_dir = tempfile::tempdir().unwrap();
        let source = source_dir.path().join("ggml-tiny.bin");
        let installed = models_dir.path().join("ggml-tiny.bin");
        fs::write(&source, b"not a registered model").await.unwrap();
        fs::write(&installed, b"existing model remains intact")
            .await
            .unwrap();

        assert!(import_registered_whisper_file(models_dir.path(), &source)
            .await
            .is_err());
        assert_eq!(
            fs::read(&installed).await.unwrap(),
            b"existing model remains intact"
        );
    }

    #[test]
    #[ignore = "requires MINGTILY_WHISPER_MODEL_FILE pointing to a registered Whisper fixture"]
    fn registered_offline_whisper_fixture_passes_exact_integrity_verification() {
        let path = std::env::var("MINGTILY_WHISPER_MODEL_FILE").unwrap();
        let path = PathBuf::from(path);
        let file_name = path.file_name().and_then(|value| value.to_str()).unwrap();
        let spec = WHISPER_DOWNLOAD_SPECS
            .iter()
            .find(|candidate| candidate.file_name == file_name)
            .copied()
            .unwrap();
        verify_whisper_file(&path, spec).unwrap();
    }

    #[tokio::test]
    #[ignore = "requires MINGTILY_WHISPER_MODEL_FILE pointing to a registered Whisper fixture"]
    async fn registered_offline_whisper_fixture_loads_and_runs_inference() {
        let path = PathBuf::from(std::env::var("MINGTILY_WHISPER_MODEL_FILE").unwrap());
        let file_name = path.file_name().and_then(|value| value.to_str()).unwrap();
        let spec = WHISPER_DOWNLOAD_SPECS
            .iter()
            .find(|candidate| candidate.file_name == file_name)
            .copied()
            .unwrap();
        let models_dir = path.parent().unwrap().to_path_buf();
        let engine = WhisperEngine::new_with_models_dir(Some(models_dir)).unwrap();
        engine.discover_models().await.unwrap();
        engine.load_model(spec.model_name).await.unwrap();
        assert!(engine.is_model_loaded().await);
        let (text, confidence, partial) = engine
            .transcribe_audio_with_confidence(vec![0.0; 32_000], Some("en".into()))
            .await
            .unwrap();
        assert!(!partial);
        assert!(confidence.is_finite());
        assert!(text.len() < 10_000);
        assert!(engine.unload_model().await);
    }

    #[tokio::test]
    async fn resumes_partial_download_and_atomically_finalizes_it() {
        let temp = tempfile::tempdir().unwrap();
        let engine = WhisperEngine::new_with_models_dir(Some(temp.path().to_path_buf())).unwrap();
        let final_path = temp.path().join("ggml-test.bin");
        let partial_path = partial_download_path(&final_path);
        fs::write(&partial_path, b"ggmlPART").await.unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0_u8; 2048];
            let bytes_read = socket.read(&mut request).await.unwrap();
            let request = String::from_utf8_lossy(&request[..bytes_read]).to_string();
            let response = b"HTTP/1.1 206 Partial Content\r\nContent-Length: 4\r\nContent-Range: bytes 8-11/12\r\nConnection: close\r\n\r\ntail";
            socket.write_all(response).await.unwrap();
            request
        });

        let progress = Arc::new(AtomicU8::new(0));
        let callback_progress = progress.clone();
        engine
            .download_model_from_url(
                "test",
                &format!("http://{address}/model.bin"),
                Some(Box::new(move |value| {
                    callback_progress.store(value, Ordering::SeqCst);
                })),
            )
            .await
            .unwrap();

        let request = server.await.unwrap();
        assert!(request.to_ascii_lowercase().contains("range: bytes=8-"));
        assert_eq!(fs::read(&final_path).await.unwrap(), b"ggmlPARTtail");
        assert!(!partial_path.exists());
        assert_eq!(progress.load(Ordering::SeqCst), 100);
    }
}
