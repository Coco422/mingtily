use crate::parakeet_engine::model::ParakeetModel;
use anyhow::{anyhow, Result};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tar::Archive;
use tokio::fs;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::RwLock;
use tokio::time::timeout;

const PARAKEET_V3_MODEL_NAME: &str = "parakeet-tdt-0.6b-v3-int8";
const PARAKEET_V2_MODEL_NAME: &str = "parakeet-tdt-0.6b-v2-int8";
const PARAKEET_V2_REVISION: &str = "0bbb45a3365852604aef28b538a8f066f4ccaa85";
const PARAKEET_V3_REVISION: &str = "8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce";
const PARAKEET_V3_REPOSITORY: &str = "istupakov/parakeet-tdt-0.6b-v3-onnx";
#[cfg(test)]
const PARAKEET_V3_MODELSCOPE_REVISION: &str = "9e59b3be0ae24ee75558bf82f60ce7de24199a3a";
const PARAKEET_V3_MODELSCOPE_ARCHIVE_URL: &str = "https://www.modelscope.cn/api/v1/models/TonyWong/parakeet-v3-int8/repo?Revision=9e59b3be0ae24ee75558bf82f60ce7de24199a3a&FilePath=parakeet-v3-int8.tar.gz";
const PARAKEET_V3_MODELSCOPE_ARCHIVE_SIZE: u64 = 478_517_071;
const PARAKEET_V3_MODELSCOPE_ARCHIVE_SHA256: &str =
    "43d37191602727524a7d8c6da0eef11c4ba24320f5b4730f1a2497befc2efa77";

#[derive(Clone, Copy)]
struct ModelFileSpec {
    name: &'static str,
    size: u64,
    sha256: &'static str,
}

const PARAKEET_V3_INT8_FILES: [ModelFileSpec; 4] = [
    ModelFileSpec {
        name: "encoder-model.int8.onnx",
        size: 652_183_999,
        sha256: "6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09",
    },
    ModelFileSpec {
        name: "decoder_joint-model.int8.onnx",
        size: 18_202_004,
        sha256: "eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70",
    },
    ModelFileSpec {
        name: "nemo128.onnx",
        size: 139_764,
        sha256: "a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f",
    },
    ModelFileSpec {
        name: "vocab.txt",
        size: 93_939,
        sha256: "d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d",
    },
];
const PARAKEET_V2_INT8_FILES: [ModelFileSpec; 4] = [
    ModelFileSpec {
        name: "encoder-model.int8.onnx",
        size: 652_184_014,
        sha256: "3e0581fda6ab843888b51e56d7ee78b6d5bc3237ec113af1f732d1d5286aa155",
    },
    ModelFileSpec {
        name: "decoder_joint-model.int8.onnx",
        size: 8_998_286,
        sha256: "a449f49acd68979d418651dd2dcb737cc0f1bf0225e009e29ee326354edbf7d3",
    },
    ModelFileSpec {
        name: "nemo128.onnx",
        size: 139_764,
        sha256: "a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f",
    },
    ModelFileSpec {
        name: "vocab.txt",
        size: 9_384,
        sha256: "ec182b70dd42113aff6c5372c75cac58c952443eb22322f57bbd7f53977d497d",
    },
];

fn parakeet_v3_base_url() -> String {
    format!(
        "https://huggingface.co/{}/resolve/{}",
        PARAKEET_V3_REPOSITORY, PARAKEET_V3_REVISION
    )
}

fn exact_file_specs(model_name: &str) -> Option<&'static [ModelFileSpec]> {
    match model_name {
        PARAKEET_V3_MODEL_NAME => Some(&PARAKEET_V3_INT8_FILES),
        PARAKEET_V2_MODEL_NAME => Some(&PARAKEET_V2_INT8_FILES),
        _ => None,
    }
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| anyhow!("Failed to open {} for checksum: {}", path.display(), e))?;
    let mut hasher = Sha256::new();
    // Keep the read buffer small: this can run on the Windows main thread
    // (sync Tauri commands) where only 1 MB of stack is available.
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| anyhow!("Failed to read {} for checksum: {}", path.display(), e))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn extract_modelscope_v3_archive(archive_path: &Path, staging: &Path) -> Result<()> {
    std::fs::create_dir_all(staging)?;
    let decoder = GzDecoder::new(std::fs::File::open(archive_path)?);
    let mut archive = Archive::new(decoder);
    let expected = PARAKEET_V3_INT8_FILES
        .iter()
        .map(|spec| spec.name)
        .collect::<HashSet<_>>();
    let mut extracted = HashSet::new();
    for entry in archive.entries()? {
        let mut entry = entry?;
        let entry_path = entry.path()?.to_path_buf();
        let Some(file_name) = entry_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        if !expected.contains(file_name.as_str()) {
            continue;
        }
        let destination = staging.join(&file_name);
        let mut output = std::fs::File::create(&destination)?;
        std::io::copy(&mut entry, &mut output)?;
        extracted.insert(file_name);
    }
    if extracted.len() != expected.len() {
        return Err(anyhow!(
            "ModelScope Parakeet archive contained {}/{} required files",
            extracted.len(),
            expected.len()
        ));
    }
    for spec in PARAKEET_V3_INT8_FILES {
        let path = staging.join(spec.name);
        let metadata = std::fs::metadata(&path)?;
        if metadata.len() != spec.size || sha256_file(&path)? != spec.sha256 {
            return Err(anyhow!(
                "ModelScope Parakeet archive failed integrity verification for {}",
                spec.name
            ));
        }
    }
    Ok(())
}

async fn verify_exact_file(path: &Path, spec: ModelFileSpec) -> Result<()> {
    let metadata = fs::metadata(path)
        .await
        .map_err(|e| anyhow!("Failed to read {} metadata: {}", spec.name, e))?;
    if metadata.len() != spec.size {
        return Err(anyhow!(
            "{} has unexpected size: {} bytes (expected {} bytes)",
            spec.name,
            metadata.len(),
            spec.size
        ));
    }

    let checksum_path = path.to_path_buf();
    let actual_sha256 = tokio::task::spawn_blocking(move || sha256_file(&checksum_path))
        .await
        .map_err(|e| anyhow!("Checksum task failed for {}: {}", spec.name, e))??;

    if actual_sha256 != spec.sha256 {
        if let Err(error) = fs::remove_file(path).await {
            log::warn!(
                "Failed to remove corrupted Parakeet model file {}: {}",
                path.display(),
                error
            );
        }
        return Err(anyhow!(
            "{} checksum mismatch: expected {}, got {}; corrupted file removed",
            spec.name,
            spec.sha256,
            actual_sha256
        ));
    }

    Ok(())
}

/// Quantization type for Parakeet models
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QuantizationType {
    FP32, // Full precision
    Int8, // 8-bit integer quantization (faster)
}

impl Default for QuantizationType {
    fn default() -> Self {
        QuantizationType::Int8 // Default to int8 for best performance
    }
}

/// Model status for Parakeet models
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

/// Detailed download progress info (MB-based with speed)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    /// Bytes downloaded so far
    pub downloaded_bytes: u64,
    /// Total file size in bytes
    pub total_bytes: u64,
    /// Downloaded in MB (for display)
    pub downloaded_mb: f64,
    /// Total size in MB (for display)
    pub total_mb: f64,
    /// Download speed in MB/s
    pub speed_mbps: f64,
    /// Percentage complete (0-100)
    pub percent: u8,
}

impl DownloadProgress {
    pub fn new(downloaded: u64, total: u64, speed_mbps: f64) -> Self {
        let percent = if total > 0 {
            ((downloaded as f64 / total as f64) * 100.0).min(100.0) as u8
        } else {
            0
        };
        Self {
            downloaded_bytes: downloaded,
            total_bytes: total,
            downloaded_mb: downloaded as f64 / (1024.0 * 1024.0),
            total_mb: total as f64 / (1024.0 * 1024.0),
            speed_mbps,
            percent,
        }
    }
}

/// Information about a Parakeet model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub path: PathBuf,
    pub size_mb: u32,
    pub quantization: QuantizationType,
    pub speed: String, // Performance description
    pub status: ModelStatus,
    pub description: String,
}

#[derive(Debug)]
pub enum ParakeetEngineError {
    ModelNotLoaded,
    ModelNotFound(String),
    TranscriptionFailed(String),
    DownloadFailed(String),
    IoError(std::io::Error),
    Other(String),
}

impl std::fmt::Display for ParakeetEngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParakeetEngineError::ModelNotLoaded => write!(f, "No Parakeet model loaded"),
            ParakeetEngineError::ModelNotFound(name) => write!(f, "Model '{}' not found", name),
            ParakeetEngineError::TranscriptionFailed(err) => {
                write!(f, "Transcription failed: {}", err)
            }
            ParakeetEngineError::DownloadFailed(err) => write!(f, "Download failed: {}", err),
            ParakeetEngineError::IoError(err) => write!(f, "IO error: {}", err),
            ParakeetEngineError::Other(err) => write!(f, "Error: {}", err),
        }
    }
}

impl std::error::Error for ParakeetEngineError {}

impl From<std::io::Error> for ParakeetEngineError {
    fn from(err: std::io::Error) -> Self {
        ParakeetEngineError::IoError(err)
    }
}

pub struct ParakeetEngine {
    models_dir: PathBuf,
    current_model: Arc<RwLock<Option<ParakeetModel>>>,
    current_model_name: Arc<RwLock<Option<String>>>,
    current_model_threads: Arc<RwLock<Option<usize>>>,
    pub(crate) available_models: Arc<RwLock<HashMap<String, ModelInfo>>>,
    cancel_download_flag: Arc<RwLock<Option<String>>>, // Model name being cancelled
    // Active downloads tracking to prevent concurrent downloads
    pub(crate) active_downloads: Arc<RwLock<HashSet<String>>>, // Set of models currently being downloaded
}

impl ParakeetEngine {
    /// Create a new Parakeet engine with optional custom models directory
    pub fn new_with_models_dir(models_dir: Option<PathBuf>) -> Result<Self> {
        let models_dir = if let Some(dir) = models_dir {
            dir.join("parakeet") // Parakeet models in subdirectory
        } else {
            // Fallback to default location
            let current_dir = std::env::current_dir()
                .map_err(|e| anyhow!("Failed to get current directory: {}", e))?;

            if cfg!(debug_assertions) {
                // Development mode
                current_dir.join("models").join("parakeet")
            } else {
                // Production mode
                dirs::data_dir()
                    .or_else(|| dirs::home_dir())
                    .ok_or_else(|| anyhow!("Could not find system data directory"))?
                    .join("Mingtily")
                    .join("models")
                    .join("parakeet")
            }
        };

        log::info!(
            "ParakeetEngine using models directory: {}",
            models_dir.display()
        );

        // Create directory if it doesn't exist
        if !models_dir.exists() {
            std::fs::create_dir_all(&models_dir)?;
        }

        Ok(Self {
            models_dir,
            current_model: Arc::new(RwLock::new(None)),
            current_model_name: Arc::new(RwLock::new(None)),
            current_model_threads: Arc::new(RwLock::new(None)),
            available_models: Arc::new(RwLock::new(HashMap::new())),
            cancel_download_flag: Arc::new(RwLock::new(None)),
            // Initialize active downloads tracking
            active_downloads: Arc::new(RwLock::new(HashSet::new())),
        })
    }

    /// Discover available Parakeet models
    pub async fn discover_models(&self) -> Result<Vec<ModelInfo>> {
        let models_dir = &self.models_dir;
        let mut models = Vec::new();

        // Parakeet model configurations
        // Model name format: parakeet-tdt-0.6b-v{version}-{quantization}
        // Sizes match actual download sizes (encoder + decoder + preprocessor + vocab)
        let model_configs = [
            (
                "parakeet-tdt-0.6b-v3-int8",
                670,
                QuantizationType::Int8,
                "Ultra Fast (v3)",
                "Real time on M4 Max, latest version with int8 quantization",
            ),
            (
                "parakeet-tdt-0.6b-v2-int8",
                661,
                QuantizationType::Int8,
                "Fast (v2)",
                "Previous version with int8 quantization, good balance of speed and accuracy",
            ),
        ];

        // Get active downloads to override status
        let active_downloads = self.active_downloads.read().await;

        for (name, size_mb, quantization, speed, description) in model_configs {
            let model_path = models_dir.join(name);

            // Check if model is currently downloading
            let status = if active_downloads.contains(name) {
                // If downloading, preserve that status regardless of file system
                // We don't know the exact progress here without more state, but 0 is safe fallback
                // The progress events will update the UI
                ModelStatus::Downloading { progress: 0 }
            } else if model_path.exists() {
                // Check for required ONNX files
                let required_files = match quantization {
                    QuantizationType::Int8 => vec![
                        "encoder-model.int8.onnx",
                        "decoder_joint-model.int8.onnx",
                        "nemo128.onnx",
                        "vocab.txt",
                    ],
                    QuantizationType::FP32 => vec![
                        "encoder-model.onnx",
                        "decoder_joint-model.onnx",
                        "nemo128.onnx",
                        "vocab.txt",
                    ],
                };

                let all_files_exist = required_files
                    .iter()
                    .all(|file| model_path.join(file).exists());

                if all_files_exist {
                    // Validate model by checking file sizes
                    match self.validate_model_directory(&model_path).await {
                        Ok(_) => ModelStatus::Available,
                        Err(_) => {
                            log::warn!("Model directory {} appears corrupted", name);
                            // Calculate total size of existing files
                            let mut total_size = 0u64;
                            for file in required_files {
                                if let Ok(metadata) = std::fs::metadata(model_path.join(file)) {
                                    total_size += metadata.len();
                                }
                            }
                            ModelStatus::Corrupted {
                                file_size: total_size,
                                expected_min_size: (size_mb as u64) * 1024 * 1024,
                            }
                        }
                    }
                } else {
                    ModelStatus::Missing
                }
            } else {
                ModelStatus::Missing
            };

            let model_info = ModelInfo {
                name: name.to_string(),
                path: model_path,
                size_mb: size_mb as u32,
                quantization: quantization.clone(),
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

    /// Validate model directory by checking if all required files exist AND have valid sizes
    async fn validate_model_directory(&self, model_dir: &PathBuf) -> Result<()> {
        let model_name = model_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if let Some(file_specs) = exact_file_specs(model_name) {
            for spec in file_specs {
                verify_exact_file(&model_dir.join(spec.name), *spec).await?;
            }
            return Ok(());
        }

        // Check if vocab.txt exists and is readable
        let vocab_path = model_dir.join("vocab.txt");
        if !vocab_path.exists() {
            return Err(anyhow!("vocab.txt not found"));
        }

        // Determine which files to check based on what exists
        let is_int8 = model_dir.join("encoder-model.int8.onnx").exists();
        let is_fp32 = model_dir.join("encoder-model.onnx").exists();

        if !is_int8 && !is_fp32 {
            return Err(anyhow!("No ONNX model files found"));
        }

        // Check preprocessor
        if !model_dir.join("nemo128.onnx").exists() {
            return Err(anyhow!("Preprocessor (nemo128.onnx) not found"));
        }

        // Define minimum file sizes (90% of expected to allow some variance)
        // These are critical to catch partial downloads that would crash on load
        let expected_sizes: Vec<(&str, u64)> = if is_int8 {
            vec![
                ("encoder-model.int8.onnx", 580_000_000), // ~652 MB, min 580 MB (89%)
                ("decoder_joint-model.int8.onnx", 8_000_000), // ~18 MB, min 8 MB
                ("nemo128.onnx", 100_000),                // ~140 KB, min 100 KB
                ("vocab.txt", 5_000),                     // ~94 KB, min 5 KB
            ]
        } else {
            vec![
                ("encoder-model.onnx", 2_200_000_000), // ~2.44 GB, min 2.2 GB
                ("decoder_joint-model.onnx", 65_000_000), // ~72 MB, min 65 MB
                ("nemo128.onnx", 100_000),             // ~140 KB, min 100 KB
                ("vocab.txt", 5_000),                  // ~94 KB, min 5 KB
            ]
        };

        // Validate each file exists AND has sufficient size
        for (filename, min_size) in expected_sizes {
            let file_path = model_dir.join(filename);
            if !file_path.exists() {
                return Err(anyhow!("{} not found", filename));
            }

            match std::fs::metadata(&file_path) {
                Ok(metadata) => {
                    let actual_size = metadata.len();
                    if actual_size < min_size {
                        return Err(anyhow!(
                            "{} is incomplete: {} bytes (expected at least {} bytes)",
                            filename,
                            actual_size,
                            min_size
                        ));
                    }
                }
                Err(e) => {
                    return Err(anyhow!("Failed to read {} metadata: {}", filename, e));
                }
            }
        }

        Ok(())
    }

    /// Clean incomplete model directory before download
    /// Removes all files if directory exists but model is not Available
    async fn clean_incomplete_model_directory(&self, model_dir: &PathBuf) -> Result<()> {
        if !model_dir.exists() {
            return Ok(()); // Nothing to clean
        }

        // Validate the directory
        match self.validate_model_directory(model_dir).await {
            Ok(_) => {
                log::info!("Model directory is valid, no cleanup needed");
                return Ok(());
            }
            Err(validation_error) => {
                log::warn!(
                    "Model directory exists but is invalid: {}. Cleaning up...",
                    validation_error
                );

                // List and remove all files in the directory
                let mut entries = fs::read_dir(model_dir)
                    .await
                    .map_err(|e| anyhow!("Failed to read model directory: {}", e))?;

                let mut removed_count = 0;
                while let Some(entry) = entries
                    .next_entry()
                    .await
                    .map_err(|e| anyhow!("Failed to read directory entry: {}", e))?
                {
                    let path = entry.path();
                    if path.is_file() {
                        match fs::remove_file(&path).await {
                            Ok(_) => {
                                log::info!("Removed incomplete file: {:?}", path.file_name());
                                removed_count += 1;
                            }
                            Err(e) => {
                                log::warn!("Failed to remove file {:?}: {}", path, e);
                            }
                        }
                    }
                }

                log::info!(
                    "Cleaned {} incomplete files from model directory",
                    removed_count
                );
                Ok(())
            }
        }
    }

    /// Load a Parakeet model
    pub async fn load_model(&self, model_name: &str) -> Result<()> {
        self.load_model_with_threads(model_name, None).await
    }

    /// Load a Parakeet model with an explicit ONNX Runtime thread budget.
    pub async fn load_model_with_threads(
        &self,
        model_name: &str,
        num_threads: Option<usize>,
    ) -> Result<()> {
        let num_threads = num_threads.map(|threads| threads.max(1));
        let models = self.available_models.read().await;
        let model_info = models
            .get(model_name)
            .ok_or_else(|| anyhow!("Model {} not found", model_name))?;

        match model_info.status {
            ModelStatus::Available => {
                // Check if this model is already loaded
                if let Some(current_model) = self.current_model_name.read().await.as_ref() {
                    let current_threads = *self.current_model_threads.read().await;
                    if current_model == model_name && current_threads == num_threads {
                        log::info!(
                            "Parakeet model {} is already loaded, skipping reload",
                            model_name
                        );
                        return Ok(());
                    }

                    // Unload current model before loading new one
                    log::info!(
                        "Unloading current Parakeet model '{}' before loading '{}'",
                        current_model,
                        model_name
                    );
                    self.unload_model().await;
                }

                log::info!("Loading Parakeet model: {}", model_name);

                // Load model based on quantization type
                let quantized = model_info.quantization == QuantizationType::Int8;
                let model =
                    ParakeetModel::new_with_threads(&model_info.path, quantized, num_threads)
                        .map_err(|e| {
                            anyhow!("Failed to load Parakeet model {}: {}", model_name, e)
                        })?;

                // Update current model and model name
                *self.current_model.write().await = Some(model);
                *self.current_model_name.write().await = Some(model_name.to_string());
                *self.current_model_threads.write().await = num_threads;

                log::info!(
                    "Successfully loaded Parakeet model: {} ({})",
                    model_name,
                    if quantized { "Int8 quantized" } else { "FP32" }
                );
                Ok(())
            }
            ModelStatus::Missing => Err(anyhow!("Parakeet model {} is not downloaded", model_name)),
            ModelStatus::Downloading { .. } => Err(anyhow!(
                "Parakeet model {} is currently downloading",
                model_name
            )),
            ModelStatus::Error(ref err) => {
                Err(anyhow!("Parakeet model {} has error: {}", model_name, err))
            }
            ModelStatus::Corrupted { .. } => Err(anyhow!(
                "Parakeet model {} is corrupted and cannot be loaded",
                model_name
            )),
        }
    }

    /// Unload the current model
    pub async fn unload_model(&self) -> bool {
        let mut model_guard = self.current_model.write().await;
        let unloaded = model_guard.take().is_some();
        if unloaded {
            log::info!("Parakeet model unloaded");
        }

        let mut model_name_guard = self.current_model_name.write().await;
        model_name_guard.take();
        self.current_model_threads.write().await.take();

        unloaded
    }

    /// Get the currently loaded model name
    pub async fn get_current_model(&self) -> Option<String> {
        self.current_model_name.read().await.clone()
    }

    /// Check if a model is loaded
    pub async fn is_model_loaded(&self) -> bool {
        self.current_model.read().await.is_some()
    }

    /// Transcribe audio samples using the loaded Parakeet model
    pub async fn transcribe_audio(&self, audio_data: Vec<f32>) -> Result<String> {
        let mut model_guard = self.current_model.write().await;
        let model = model_guard
            .as_mut()
            .ok_or_else(|| anyhow!("No Parakeet model loaded. Please load a model first."))?;

        let duration_seconds = audio_data.len() as f64 / 16000.0; // Assuming 16kHz
        log::debug!(
            "Parakeet transcribing {} samples ({:.1}s duration)",
            audio_data.len(),
            duration_seconds
        );

        // Transcribe using Parakeet model
        let result = model
            .transcribe_samples(audio_data)
            .map_err(|e| anyhow!("Parakeet transcription failed: {}", e))?;

        log::debug!(
            "Parakeet transcription completed: {} chars",
            result.text.chars().count()
        );

        Ok(result.text)
    }

    /// Get the models directory path
    pub async fn get_models_directory(&self) -> PathBuf {
        self.models_dir.clone()
    }

    /// Delete a corrupted model
    pub async fn delete_model(&self, model_name: &str) -> Result<String> {
        log::info!("Attempting to delete Parakeet model: {}", model_name);

        // Get model info to find the directory path
        let model_info = {
            let models = self.available_models.read().await;
            models.get(model_name).cloned()
        };

        let model_info =
            model_info.ok_or_else(|| anyhow!("Parakeet model '{}' not found", model_name))?;

        log::info!(
            "Parakeet model '{}' has status: {:?}",
            model_name,
            model_info.status
        );

        // Allow deletion of corrupted or available models
        match &model_info.status {
            ModelStatus::Corrupted { .. } | ModelStatus::Available => {
                // Delete the entire model directory
                if model_info.path.exists() {
                    fs::remove_dir_all(&model_info.path).await
                        .map_err(|e| anyhow!("Failed to delete directory '{}': {}", model_info.path.display(), e))?;
                    log::info!("Successfully deleted Parakeet model directory: {}", model_info.path.display());
                } else {
                    log::warn!("Directory '{}' does not exist, nothing to delete", model_info.path.display());
                }

                // Update model status to Missing
                {
                    let mut models = self.available_models.write().await;
                    if let Some(model) = models.get_mut(model_name) {
                        model.status = ModelStatus::Missing;
                    }
                }

                Ok(format!("Successfully deleted Parakeet model '{}'", model_name))
            }
            _ => {
                Err(anyhow!(
                    "Can only delete corrupted or available Parakeet models. Model '{}' has status: {:?}",
                    model_name,
                    model_info.status
                ))
            }
        }
    }

    /// Download a Parakeet model from HuggingFace (backward-compatible wrapper)
    pub async fn download_model(
        &self,
        model_name: &str,
        progress_callback: Option<Box<dyn Fn(u8) + Send + Sync>>,
    ) -> Result<()> {
        // Wrap simple callback to use detailed version
        let detailed_callback: Option<Box<dyn Fn(DownloadProgress) + Send + Sync>> =
            progress_callback.map(|cb| {
                Box::new(move |p: DownloadProgress| cb(p.percent))
                    as Box<dyn Fn(DownloadProgress) + Send + Sync>
            });
        self.download_model_detailed(model_name, detailed_callback)
            .await
    }

    /// Download a Parakeet model with detailed progress (MB/speed/resume support)
    pub async fn download_model_detailed(
        &self,
        model_name: &str,
        progress_callback: Option<Box<dyn Fn(DownloadProgress) + Send + Sync>>,
    ) -> Result<()> {
        log::info!("Starting download for Parakeet model: {}", model_name);

        // Check if download is already in progress for this model
        {
            let active = self.active_downloads.read().await;
            if active.contains(model_name) {
                log::warn!(
                    "Download already in progress for Parakeet model: {}",
                    model_name
                );
                return Err(anyhow!(
                    "Download already in progress for model: {}",
                    model_name
                ));
            }
        }

        // Add to active downloads
        {
            let mut active = self.active_downloads.write().await;
            active.insert(model_name.to_string());
        }

        // Clear any previous cancellation flag for this model
        {
            let mut cancel_flag = self.cancel_download_flag.write().await;
            *cancel_flag = None;
        }

        // Get model info
        let model_info = {
            let models = self.available_models.read().await;
            match models.get(model_name).cloned() {
                Some(info) => info,
                None => {
                    // Remove from active downloads on error
                    let mut active = self.active_downloads.write().await;
                    active.remove(model_name);
                    return Err(anyhow!("Model {} not found", model_name));
                }
            }
        };

        // Update model status to downloading
        {
            let mut models = self.available_models.write().await;
            if let Some(model) = models.get_mut(model_name) {
                model.status = ModelStatus::Downloading { progress: 0 };
            }
        }

        // HuggingFace base URL for Parakeet models (version-specific)
        let base_url = if model_name.contains("-v2-") {
            format!(
                "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v2-onnx/resolve/{PARAKEET_V2_REVISION}"
            )
        } else {
            parakeet_v3_base_url()
        };

        // Determine which files to download based on quantization
        let files_to_download = match model_info.quantization {
            QuantizationType::Int8 => vec![
                "encoder-model.int8.onnx",
                "decoder_joint-model.int8.onnx",
                "nemo128.onnx",
                "vocab.txt",
            ],
            QuantizationType::FP32 => vec![
                "encoder-model.onnx",
                "decoder_joint-model.onnx",
                "nemo128.onnx",
                "vocab.txt",
            ],
        };

        // Create model directory
        let model_dir = &model_info.path;
        if !model_dir.exists() {
            if let Err(e) = fs::create_dir_all(model_dir).await {
                // Remove from active downloads on error
                let mut active = self.active_downloads.write().await;
                active.remove(model_name);
                return Err(anyhow!("Failed to create model directory: {}", e));
            }
        }

        // Clean up incomplete downloads before starting
        log::info!("Checking for incomplete model files to clean up...");
        if let Err(e) = self.clean_incomplete_model_directory(model_dir).await {
            log::warn!("Failed to clean incomplete model directory: {}", e);
            // Continue anyway - we'll handle errors during download
        }

        // The audited ModelScope v3 archive contains byte-for-byte identical files to
        // the pinned Hugging Face artifact. Prefer it in mainland China, while keeping
        // the existing per-file downloader as an automatic fallback.
        if model_name == PARAKEET_V3_MODEL_NAME && model_info.quantization == QuantizationType::Int8
        {
            let cache_dir = self.models_dir.join(".downloads");
            let archive_path = cache_dir.join("parakeet-v3-int8.tar.gz.part");
            let staging = self
                .models_dir
                .join(format!(".{PARAKEET_V3_MODEL_NAME}.modelscope"));
            let model_name_for_cancel = model_name.to_string();
            let modelscope_result = crate::model_assets::download_verified_artifact(
                model_name,
                PARAKEET_V3_MODELSCOPE_ARCHIVE_URL,
                &archive_path,
                PARAKEET_V3_MODELSCOPE_ARCHIVE_SIZE,
                PARAKEET_V3_MODELSCOPE_ARCHIVE_SHA256,
                |downloaded| {
                    if self
                        .cancel_download_flag
                        .try_read()
                        .ok()
                        .and_then(|flag| flag.clone())
                        .as_deref()
                        == Some(model_name_for_cancel.as_str())
                    {
                        return Err(anyhow!("Download cancelled by user"));
                    }
                    if let Some(ref callback) = progress_callback {
                        callback(DownloadProgress::new(
                            downloaded,
                            PARAKEET_V3_MODELSCOPE_ARCHIVE_SIZE,
                            0.0,
                        ));
                    }
                    Ok(())
                },
            )
            .await;

            let modelscope_result = match modelscope_result {
                Ok(()) => {
                    async {
                        if staging.exists() {
                            fs::remove_dir_all(&staging).await?;
                        }
                        let archive_for_extract = archive_path.clone();
                        let staging_for_extract = staging.clone();
                        tokio::task::spawn_blocking(move || {
                            extract_modelscope_v3_archive(
                                &archive_for_extract,
                                &staging_for_extract,
                            )
                        })
                        .await
                        .map_err(|error| anyhow!("Parakeet extraction task failed: {error}"))??;
                        Ok::<(), anyhow::Error>(())
                    }
                    .await
                }
                Err(error) => Err(error),
            };

            match modelscope_result {
                Ok(()) => {
                    for spec in PARAKEET_V3_INT8_FILES {
                        let destination = model_dir.join(spec.name);
                        if destination.exists() {
                            fs::remove_file(&destination).await?;
                        }
                        fs::rename(staging.join(spec.name), destination).await?;
                    }
                    let _ = fs::remove_dir_all(&staging).await;
                    let _ = fs::remove_file(&archive_path).await;
                    self.validate_model_directory(model_dir).await?;
                    if let Some(ref callback) = progress_callback {
                        callback(DownloadProgress::new(
                            PARAKEET_V3_MODELSCOPE_ARCHIVE_SIZE,
                            PARAKEET_V3_MODELSCOPE_ARCHIVE_SIZE,
                            0.0,
                        ));
                    }
                    if let Some(model) = self.available_models.write().await.get_mut(model_name) {
                        model.status = ModelStatus::Available;
                        model.path = model_dir.clone();
                    }
                    self.active_downloads.write().await.remove(model_name);
                    *self.cancel_download_flag.write().await = None;
                    log::info!("Downloaded Parakeet v3 from ModelScope China");
                    return Ok(());
                }
                Err(error) if error.to_string().contains("cancelled by user") => {
                    self.active_downloads.write().await.remove(model_name);
                    if let Some(model) = self.available_models.write().await.get_mut(model_name) {
                        model.status = ModelStatus::Missing;
                    }
                    return Err(error);
                }
                Err(error) => {
                    log::warn!(
                        "ModelScope Parakeet v3 source failed; falling back to pinned Hugging Face files: {error:#}"
                    );
                    let _ = fs::remove_dir_all(&staging).await;
                }
            }
        }

        // Optimized HTTP client for large file downloads
        let client = reqwest::Client::builder()
            .tcp_nodelay(true) // Disable Nagle's algorithm for better streaming
            .pool_max_idle_per_host(1) // Keep connection alive
            .timeout(Duration::from_secs(3600)) // 1 hour timeout for large files
            .connect_timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        let total_files = files_to_download.len();

        // Calculate total download size for weighted progress
        // Note: These are approximate sizes based on HuggingFace repo inspection
        let file_sizes: std::collections::HashMap<&str, u64> = match model_info.quantization {
            QuantizationType::Int8 => {
                if model_name.contains("-v2-") {
                    PARAKEET_V2_INT8_FILES
                        .iter()
                        .map(|spec| (spec.name, spec.size))
                        .collect()
                } else {
                    PARAKEET_V3_INT8_FILES
                        .iter()
                        .map(|spec| (spec.name, spec.size))
                        .collect()
                }
            }
            QuantizationType::FP32 => {
                // FP32 model sizes (encoder has .onnx + .onnx.data)
                [
                    ("encoder-model.onnx", 41_800_000u64 + 2_440_000_000u64), // 41.8 MB + 2.44 GB
                    ("decoder_joint-model.onnx", 72_500_000u64),              // 72.5 MB
                    ("nemo128.onnx", 140_000u64),                             // 140 KB
                    ("vocab.txt", 93_900u64),                                 // 93.9 KB
                ]
                .iter()
                .cloned()
                .collect()
            }
        };

        // Calculate total expected download size
        let total_size_bytes: u64 = files_to_download
            .iter()
            .filter_map(|f| file_sizes.get(*f))
            .copied()
            .sum();

        // Check for existing downloads (complete or partial) to calculate resume offset
        let mut already_downloaded: u64 = 0;
        for filename in &files_to_download {
            let file_path = model_dir.join(filename);
            if file_path.exists() {
                if let Ok(metadata) = fs::metadata(&file_path).await {
                    let file_size = metadata.len();
                    let expected_size = file_sizes.get(*filename).copied().unwrap_or(0);
                    // Count all existing bytes (complete files capped at expected size, partial as-is)
                    // This ensures progress starts from where we left off
                    already_downloaded += file_size.min(expected_size);
                }
            }
        }

        let mut total_downloaded: u64 = already_downloaded;

        // Timing for speed calculation
        let download_start_time = Instant::now();
        let mut last_report_time = Instant::now();
        let mut bytes_since_last_report: u64 = 0;
        let mut last_reported_progress: u8 = 0;

        log::info!(
            "Starting weighted download for {} files, total size: {:.2} MB (already downloaded: {:.2} MB)",
            total_files,
            total_size_bytes as f64 / 1_048_576.0,
            already_downloaded as f64 / 1_048_576.0
        );

        for (index, filename) in files_to_download.iter().enumerate() {
            let file_url = format!("{}/{}", base_url, filename);
            let file_path = model_dir.join(filename);

            // Check for existing partial file to resume
            let mut existing_size: u64 = if file_path.exists() {
                fs::metadata(&file_path).await.map(|m| m.len()).unwrap_or(0)
            } else {
                0
            };

            let expected_size = file_sizes.get(*filename).copied().unwrap_or(0);

            let exact_spec = exact_file_specs(model_name)
                .and_then(|specs| specs.iter().find(|spec| spec.name == *filename))
                .copied();

            // Reuse an existing file only after exact checksum validation for pinned v3 assets.
            let existing_file_is_complete = if let Some(spec) = exact_spec {
                if existing_size == spec.size && verify_exact_file(&file_path, spec).await.is_ok() {
                    true
                } else {
                    if file_path.exists() {
                        fs::remove_file(&file_path).await.map_err(|e| {
                            anyhow!("Failed to remove invalid file {}: {}", filename, e)
                        })?;
                    }
                    existing_size = 0;
                    false
                }
            } else {
                let size_tolerance = (expected_size as f64 * 0.99) as u64;
                existing_size >= size_tolerance && expected_size > 0
            };
            if existing_file_is_complete {
                log::info!(
                    "Skipping complete file: {} ({:.2} MB, expected: {:.2} MB)",
                    filename,
                    existing_size as f64 / 1_048_576.0,
                    expected_size as f64 / 1_048_576.0
                );
                continue;
            }

            log::info!(
                "Downloading file {}/{}: {} (resuming from {} bytes)",
                index + 1,
                total_files,
                filename,
                existing_size
            );

            // Build request with optional Range header for resume
            let mut request = client.get(&file_url);
            if existing_size > 0 {
                request = request.header("Range", format!("bytes={}-", existing_size));
                log::info!("Resuming download from byte {}", existing_size);
            }

            let mut response = request
                .send()
                .await
                .map_err(|e| anyhow!("Failed to start download for {}: {}", filename, e))?;

            // Handle response status
            let (file_total_size, resuming) =
                if response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
                    // Server supports resume, get remaining size
                    let remaining = response.content_length().unwrap_or(0);
                    log::info!("Server supports resume, remaining: {} bytes", remaining);
                    (existing_size + remaining, true)
                } else if response.status().is_success() {
                    // Fresh download or server doesn't support resume
                    if existing_size > 0 {
                        log::warn!(
                            "Server doesn't support resume for {}, starting fresh download",
                            filename
                        );
                    }
                    (response.content_length().unwrap_or(0), false)
                } else if response.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
                    // 416: Range not satisfiable - file complete or invalid range
                    log::warn!("Server returned 416 Range Not Satisfiable for {}", filename);

                    let size_tolerance = (expected_size as f64 * 0.99) as u64;
                    if existing_size >= size_tolerance && expected_size > 0 {
                        // File is complete - skip it
                        log::info!(
                            "File {} complete ({} bytes). Skipping.",
                            filename,
                            existing_size
                        );
                        continue;
                    } else {
                        // File incomplete but server won't accept range - delete and retry
                        log::warn!(
                            "File {} incomplete ({}/{} bytes). Deleting and retrying.",
                            filename,
                            existing_size,
                            expected_size
                        );

                        if let Err(e) = fs::remove_file(&file_path).await {
                            let mut active = self.active_downloads.write().await;
                            active.remove(model_name);
                            return Err(anyhow!(
                                "Failed to delete incomplete file {}: {}",
                                filename,
                                e
                            ));
                        }

                        // Retry without Range header
                        log::info!("Retrying {} without resume", filename);
                        response = client
                            .get(&file_url)
                            .send()
                            .await
                            .map_err(|e| anyhow!("Retry failed for {}: {}", filename, e))?;

                        if !response.status().is_success() {
                            let mut active = self.active_downloads.write().await;
                            active.remove(model_name);
                            return Err(anyhow!(
                                "Retry failed for {} with status: {}",
                                filename,
                                response.status()
                            ));
                        }

                        (response.content_length().unwrap_or(0), false)
                    }
                } else {
                    // Other errors
                    let mut active = self.active_downloads.write().await;
                    active.remove(model_name);
                    return Err(anyhow!(
                        "Download failed for {} with status: {}",
                        filename,
                        response.status()
                    ));
                };

            // Open file for writing (append if resuming, create new if not)
            let file = if resuming {
                fs::OpenOptions::new()
                    .append(true)
                    .open(&file_path)
                    .await
                    .map_err(|e| anyhow!("Failed to open file for resume {}: {}", filename, e))?
            } else {
                fs::File::create(&file_path)
                    .await
                    .map_err(|e| anyhow!("Failed to create file {}: {}", filename, e))?
            };

            // Use buffered writer for better I/O performance (8MB buffer)
            let mut writer = BufWriter::with_capacity(8 * 1024 * 1024, file);

            // Stream download
            use futures_util::StreamExt;
            let mut stream = response.bytes_stream();
            let mut file_downloaded = if resuming { existing_size } else { 0u64 };

            loop {
                // Check for cancellation before processing chunk
                {
                    let cancel_flag = self.cancel_download_flag.read().await;
                    if cancel_flag.as_ref() == Some(&model_name.to_string()) {
                        log::info!("Download cancelled for {}", model_name);
                        // Flush and keep partial file for resume on next attempt
                        let _ = writer.flush().await;
                        drop(writer);
                        // Remove from active downloads on cancellation
                        let mut active = self.active_downloads.write().await;
                        active.remove(model_name);
                        return Err(anyhow!("Download cancelled by user"));
                    }
                }

                // Add per-chunk timeout (30 seconds) to detect stalled connections
                let next_result = timeout(Duration::from_secs(30), stream.next()).await;

                let chunk = match next_result {
                    // Timeout - no data received for 30 seconds
                    Err(_) => {
                        log::warn!(
                            "Download timeout for {}: no data received for 30 seconds",
                            model_name
                        );
                        let _ = writer.flush().await;

                        // Remove from active downloads
                        {
                            let mut active = self.active_downloads.write().await;
                            active.remove(model_name);
                        }

                        // Update model status to Missing so retry can work
                        {
                            let mut models = self.available_models.write().await;
                            if let Some(model) = models.get_mut(model_name) {
                                model.status = ModelStatus::Missing;
                            }
                        }

                        return Err(anyhow!(
                            "Download timeout - No data received for 30 seconds"
                        ));
                    }
                    // Stream ended
                    Ok(None) => break,
                    // Got chunk result
                    Ok(Some(chunk_result)) => {
                        match chunk_result {
                            Ok(c) => c,
                            // Detect error type for better user feedback
                            Err(e) => {
                                log::error!("Download error for {}: {:?}", model_name, e);
                                let _ = writer.flush().await;

                                // Remove from active downloads
                                {
                                    let mut active = self.active_downloads.write().await;
                                    active.remove(model_name);
                                }

                                // Update model status to Missing so retry can work
                                {
                                    let mut models = self.available_models.write().await;
                                    if let Some(model) = models.get_mut(model_name) {
                                        model.status = ModelStatus::Missing;
                                    }
                                }

                                let error_msg = if e.is_timeout() {
                                    "Connection timeout - Check your internet"
                                } else if e.is_connect() {
                                    "Connection failed - Check your internet"
                                } else if e.is_body() {
                                    "Stream interrupted - Network unstable"
                                } else {
                                    "Download error"
                                };

                                return Err(anyhow!("{}: {}", error_msg, e));
                            }
                        }
                    }
                };

                if let Err(e) = writer.write_all(&chunk).await {
                    // Remove from active downloads on error
                    {
                        let mut active = self.active_downloads.write().await;
                        active.remove(model_name);
                    }

                    // Update model status to Missing so retry can work
                    {
                        let mut models = self.available_models.write().await;
                        if let Some(model) = models.get_mut(model_name) {
                            model.status = ModelStatus::Missing;
                        }
                    }

                    return Err(anyhow!("Failed to write chunk to file: {}", e));
                }

                let chunk_len = chunk.len() as u64;
                file_downloaded += chunk_len;
                total_downloaded += chunk_len;
                bytes_since_last_report += chunk_len;

                // Calculate weighted overall progress based on total bytes downloaded
                let overall_progress = if total_size_bytes > 0 {
                    ((total_downloaded as f64 / total_size_bytes as f64) * 100.0).min(99.0) as u8
                } else {
                    // Fallback to per-file progress if total size unknown
                    ((index as f64 + (file_downloaded as f64 / file_total_size.max(1) as f64))
                        / total_files as f64
                        * 100.0) as u8
                };

                // Report every 1% progress change OR every 500ms for smooth UI updates
                let elapsed_since_report = last_report_time.elapsed();
                let progress_changed = overall_progress > last_reported_progress;
                let time_threshold = elapsed_since_report >= Duration::from_millis(500);
                let is_complete = file_downloaded >= file_total_size;

                let should_report = progress_changed || time_threshold || is_complete;

                if should_report {
                    // Calculate download speed
                    let speed_mbps = if elapsed_since_report.as_secs_f64() >= 0.1 {
                        (bytes_since_last_report as f64 / (1024.0 * 1024.0))
                            / elapsed_since_report.as_secs_f64()
                    } else {
                        // Fallback to overall average speed
                        let total_elapsed = download_start_time.elapsed().as_secs_f64();
                        if total_elapsed > 0.0 {
                            ((total_downloaded - already_downloaded) as f64 / (1024.0 * 1024.0))
                                / total_elapsed
                        } else {
                            0.0
                        }
                    };

                    last_reported_progress = overall_progress;
                    last_report_time = Instant::now();
                    bytes_since_last_report = 0;

                    // Create detailed progress and report
                    let progress =
                        DownloadProgress::new(total_downloaded, total_size_bytes, speed_mbps);
                    if let Some(ref callback) = progress_callback {
                        callback(progress);
                    }

                    // Update model status
                    {
                        let mut models = self.available_models.write().await;
                        if let Some(model) = models.get_mut(model_name) {
                            model.status = ModelStatus::Downloading {
                                progress: overall_progress,
                            };
                        }
                    }
                }
            }

            // Flush the buffered writer
            if let Err(e) = writer.flush().await {
                // Remove from active downloads on error
                {
                    let mut active = self.active_downloads.write().await;
                    active.remove(model_name);
                }

                // Update model status to Missing so retry can work
                {
                    let mut models = self.available_models.write().await;
                    if let Some(model) = models.get_mut(model_name) {
                        model.status = ModelStatus::Missing;
                    }
                }

                return Err(anyhow!("Failed to flush file {}: {}", filename, e));
            }
            drop(writer);

            if let Some(spec) = exact_spec {
                if let Err(e) = verify_exact_file(&file_path, spec).await {
                    {
                        let mut active = self.active_downloads.write().await;
                        active.remove(model_name);
                    }
                    {
                        let mut models = self.available_models.write().await;
                        if let Some(model) = models.get_mut(model_name) {
                            model.status = ModelStatus::Corrupted {
                                file_size: file_downloaded,
                                expected_min_size: spec.size,
                            };
                        }
                    }
                    return Err(e);
                }
            }

            log::info!(
                "Completed download: {} ({:.2} MB, overall progress: {:.1}%)",
                filename,
                file_downloaded as f64 / 1_048_576.0,
                (total_downloaded as f64 / total_size_bytes as f64) * 100.0
            );
        }

        if let Err(e) = self.validate_model_directory(model_dir).await {
            {
                let mut active = self.active_downloads.write().await;
                active.remove(model_name);
            }
            {
                let mut models = self.available_models.write().await;
                if let Some(model) = models.get_mut(model_name) {
                    model.status = ModelStatus::Corrupted {
                        file_size: total_downloaded,
                        expected_min_size: total_size_bytes,
                    };
                }
            }
            return Err(anyhow!(
                "Downloaded model failed integrity verification: {}",
                e
            ));
        }

        // Report 100% progress with final speed
        let total_elapsed = download_start_time.elapsed().as_secs_f64();
        let final_speed = if total_elapsed > 0.0 {
            ((total_downloaded - already_downloaded) as f64 / (1024.0 * 1024.0)) / total_elapsed
        } else {
            0.0
        };
        let final_progress = DownloadProgress::new(total_size_bytes, total_size_bytes, final_speed);
        if let Some(ref callback) = progress_callback {
            callback(final_progress);
        }

        // Update model status to available
        {
            let mut models = self.available_models.write().await;
            if let Some(model) = models.get_mut(model_name) {
                model.status = ModelStatus::Available;
                model.path = model_dir.clone();
            }
        }

        // Remove from active downloads on completion
        {
            let mut active = self.active_downloads.write().await;
            active.remove(model_name);
        }

        // Clear cancellation flag on successful completion
        {
            let mut cancel_flag = self.cancel_download_flag.write().await;
            if cancel_flag.as_ref() == Some(&model_name.to_string()) {
                *cancel_flag = None;
            }
        }

        log::info!("Download completed for Parakeet model: {}", model_name);
        Ok(())
    }

    /// Cancel an ongoing model download
    pub async fn cancel_download(&self, model_name: &str) -> Result<()> {
        log::info!("Cancelling download for Parakeet model: {}", model_name);

        // Set cancellation flag to interrupt the download loop
        {
            let mut cancel_flag = self.cancel_download_flag.write().await;
            *cancel_flag = Some(model_name.to_string());
        }

        // Remove from active downloads
        {
            let mut active = self.active_downloads.write().await;
            active.remove(model_name);
        }

        // Update model status to Missing (so it can be retried)
        {
            let mut models = self.available_models.write().await;
            if let Some(model) = models.get_mut(model_name) {
                model.status = ModelStatus::Missing;
            }
        }

        // Clean up partially downloaded files
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await; // Brief delay to let download loop exit

        let model_path = self.models_dir.join(model_name);
        if model_path.exists() {
            if let Err(e) = fs::remove_dir_all(&model_path).await {
                log::warn!("Failed to clean up cancelled download directory: {}", e);
            } else {
                log::info!(
                    "Cleaned up cancelled download directory: {}",
                    model_path.display()
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v3_download_url_is_pinned_to_a_revision() {
        assert_eq!(
            parakeet_v3_base_url(),
            "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce"
        );
        assert_eq!(PARAKEET_V3_INT8_FILES.len(), 4);
        assert_eq!(PARAKEET_V3_MODELSCOPE_REVISION.len(), 40);
        assert!(PARAKEET_V3_MODELSCOPE_ARCHIVE_URL.contains("modelscope.cn"));
        assert!(PARAKEET_V3_MODELSCOPE_ARCHIVE_URL.contains(PARAKEET_V3_MODELSCOPE_REVISION));
        assert_eq!(PARAKEET_V3_MODELSCOPE_ARCHIVE_SHA256.len(), 64);
    }

    #[test]
    fn v2_fallback_is_pinned_with_exact_file_contract() {
        assert_eq!(PARAKEET_V2_REVISION.len(), 40);
        assert_eq!(PARAKEET_V2_INT8_FILES.len(), 4);
        assert!(PARAKEET_V2_INT8_FILES
            .iter()
            .all(|spec| spec.sha256.len() == 64));
    }

    #[test]
    #[ignore = "requires the audited ModelScope Parakeet v3 archive"]
    fn modelscope_v3_archive_extracts_to_exact_runtime_files() {
        let archive = std::env::var("MINGTILY_PARAKEET_V3_ARCHIVE")
            .expect("set MINGTILY_PARAKEET_V3_ARCHIVE to parakeet-v3-int8.tar.gz");
        let directory = tempfile::tempdir().unwrap();
        extract_modelscope_v3_archive(Path::new(&archive), directory.path()).unwrap();
        for spec in PARAKEET_V3_INT8_FILES {
            assert_eq!(
                std::fs::metadata(directory.path().join(spec.name))
                    .unwrap()
                    .len(),
                spec.size
            );
        }
    }

    #[tokio::test]
    async fn checksum_mismatch_removes_corrupted_file() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let file_path = temp_dir.path().join("model.onnx");
        std::fs::write(&file_path, b"hello").expect("write test model");
        let spec = ModelFileSpec {
            name: "model.onnx",
            size: 5,
            sha256: "0000000000000000000000000000000000000000000000000000000000000000",
        };

        let error = verify_exact_file(&file_path, spec)
            .await
            .expect_err("checksum mismatch should fail");

        assert!(error.to_string().contains("checksum mismatch"));
        assert!(!file_path.exists());
    }
}
