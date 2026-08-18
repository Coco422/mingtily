use anyhow::{anyhow, Context, Result};
use bzip2::read::BzDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tar::Archive;
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub const MODEL_ID: &str = "sherpa-v1";
const BACKEND_ID: &str = "sherpa-pyannote3-eres2net";
const SEGMENTATION_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2";
const SEGMENTATION_MODELSCOPE_URL: &str = "https://www.modelscope.cn/api/v1/models/pengzhendong/sherpa-onnx-pyannote-segmentation-3-0/repo?Revision=103d397e9706dbb03f458fad62430ee8e9ae2bb4&FilePath=model.int8.onnx";
const SEGMENTATION_SHA256: &str =
    "24615ee884c897d9d2ba09bb4d30da6bb1b15e685065962db5b02e76e4996488";
const SEGMENTATION_MODEL_SHA256: &str =
    "d582f4b4c6b48205de7e0643c57df0df5615a3c176189be3fc461e9d18827b5d";
const SEGMENTATION_DOWNLOAD_SIZE: u64 = 6_958_444;
const EMBEDDING_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/3dspeaker_speech_eres2net_base_sv_zh-cn_3dspeaker_16k.onnx";
const EMBEDDING_MODELSCOPE_URL: &str = "https://www.modelscope.cn/api/v1/models/liaowenbin/3dspeaker_speech_eres2net_base_sv_zh-cn_3dspeaker_16k/repo?Revision=38dbd263d67cf31fa0bb4c1184a31289f9fd94a8&FilePath=3dspeaker_speech_eres2net_base_sv_zh-cn_3dspeaker_16k.onnx";
const EMBEDDING_SHA256: &str = "1a331345f04805badbb495c775a6ddffcdd1a732567d5ec8b3d5749e3c7a5e4b";
const EMBEDDING_DOWNLOAD_SIZE: u64 = 39_593_761;
const MODELSCOPE_DOWNLOAD_SIZE: u64 = 1_540_506 + EMBEDDING_DOWNLOAD_SIZE;
const TOTAL_DOWNLOAD_SIZE: u64 = MODELSCOPE_DOWNLOAD_SIZE;

#[derive(Debug, Clone)]
pub struct SpeakerModelPaths {
    pub root: PathBuf,
    pub segmentation: PathBuf,
    pub embedding: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerModelManifest {
    pub id: String,
    pub version: u32,
    pub backend: String,
    pub segmentation_source: String,
    pub segmentation_sha256: String,
    pub segmentation_model_sha256: String,
    pub embedding_source: String,
    pub embedding_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpeakerModelStatus {
    pub id: String,
    pub status: String,
    pub size_mb: f64,
    pub path: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct DownloadProgressEvent {
    model_id: String,
    progress: u8,
    downloaded_mb: f64,
    total_mb: f64,
    status: String,
}

pub fn model_root<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf> {
    Ok(app
        .path()
        .app_data_dir()
        .context("Unable to resolve app data directory")?
        .join("models")
        .join("speaker-diarization")
        .join(MODEL_ID))
}

pub fn paths_for_root(root: PathBuf) -> SpeakerModelPaths {
    SpeakerModelPaths {
        segmentation: root.join("segmentation").join("model.int8.onnx"),
        embedding: root.join("embedding").join("3dspeaker_eres2net.onnx"),
        root,
    }
}

pub fn installed_model_paths<R: Runtime>(app: &AppHandle<R>) -> Result<Option<SpeakerModelPaths>> {
    let paths = paths_for_root(model_root(app)?);
    if validate_installation(&paths).is_ok() {
        Ok(Some(paths))
    } else {
        Ok(None)
    }
}

pub fn get_status<R: Runtime>(app: &AppHandle<R>) -> Result<SpeakerModelStatus> {
    let paths = paths_for_root(model_root(app)?);
    let validation = validate_installation(&paths);
    let status = if validation.is_ok() {
        "available"
    } else if paths.root.exists() {
        "corrupt"
    } else {
        "missing"
    };

    Ok(SpeakerModelStatus {
        id: MODEL_ID.to_string(),
        status: status.to_string(),
        size_mb: TOTAL_DOWNLOAD_SIZE as f64 / 1_000_000.0,
        path: paths.root.to_string_lossy().to_string(),
        error: validation.err().map(|error| error.to_string()),
    })
}

pub async fn download_model<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    let final_root = model_root(&app)?;
    let parent = final_root
        .parent()
        .ok_or_else(|| anyhow!("Invalid speaker model directory"))?
        .to_path_buf();
    tokio::fs::create_dir_all(&parent).await?;

    let staging = parent.join(format!(".{}.download", MODEL_ID));
    if staging.exists() {
        tokio::fs::remove_dir_all(&staging).await?;
    }
    tokio::fs::create_dir_all(&staging).await?;

    let result = async {
        let cache_root = parent.join(".downloads").join(MODEL_ID);
        tokio::fs::create_dir_all(&cache_root).await?;
        let segmentation_dir = staging.join("segmentation");
        let embedding_dir = staging.join("embedding");
        tokio::fs::create_dir_all(&segmentation_dir).await?;
        tokio::fs::create_dir_all(&embedding_dir).await?;
        let ms_segmentation = cache_root.join("segmentation-modelscope.part");
        let ms_embedding = cache_root.join("embedding-modelscope.part");
        let modelscope_result = async {
            crate::model_assets::download_verified_artifact(
                MODEL_ID,
                SEGMENTATION_MODELSCOPE_URL,
                &ms_segmentation,
                1_540_506,
                SEGMENTATION_MODEL_SHA256,
                |downloaded| {
                    emit_download_progress(&app, downloaded, MODELSCOPE_DOWNLOAD_SIZE);
                    Ok(())
                },
            )
            .await?;
            crate::model_assets::download_verified_artifact(
                MODEL_ID,
                EMBEDDING_MODELSCOPE_URL,
                &ms_embedding,
                EMBEDDING_DOWNLOAD_SIZE,
                EMBEDDING_SHA256,
                |downloaded| {
                    emit_download_progress(
                        &app,
                        1_540_506u64.saturating_add(downloaded),
                        MODELSCOPE_DOWNLOAD_SIZE,
                    );
                    Ok(())
                },
            )
            .await?;
            tokio::fs::copy(
                &ms_segmentation,
                segmentation_dir.join("model.int8.onnx"),
            )
            .await?;
            tokio::fs::copy(
                &ms_embedding,
                embedding_dir.join("3dspeaker_eres2net.onnx"),
            )
            .await?;
            write_speaker_license_files(&staging)?;
            Ok::<(), anyhow::Error>(())
        }
        .await;

        let (segmentation_source, segmentation_sha256, embedding_source) =
            if let Err(modelscope_error) = modelscope_result {
                log::warn!(
                    "ModelScope speaker model source failed; falling back to sherpa-onnx GitHub Release: {modelscope_error:#}"
                );
                let archive_part = cache_root.join("segmentation-github.part");
                crate::model_assets::download_verified_artifact(
                    MODEL_ID,
                    SEGMENTATION_URL,
                    &archive_part,
                    SEGMENTATION_DOWNLOAD_SIZE,
                    SEGMENTATION_SHA256,
                    |downloaded| {
                        emit_download_progress(
                            &app,
                            downloaded,
                            SEGMENTATION_DOWNLOAD_SIZE + EMBEDDING_DOWNLOAD_SIZE,
                        );
                        Ok(())
                    },
                )
                .await?;
                let embedding_part = cache_root.join("embedding-github.part");
                crate::model_assets::download_verified_artifact(
                    MODEL_ID,
                    EMBEDDING_URL,
                    &embedding_part,
                    EMBEDDING_DOWNLOAD_SIZE,
                    EMBEDDING_SHA256,
                    |downloaded| {
                        emit_download_progress(
                            &app,
                            SEGMENTATION_DOWNLOAD_SIZE.saturating_add(downloaded),
                            SEGMENTATION_DOWNLOAD_SIZE + EMBEDDING_DOWNLOAD_SIZE,
                        );
                        Ok(())
                    },
                )
                .await?;
                tokio::fs::copy(
                    &embedding_part,
                    embedding_dir.join("3dspeaker_eres2net.onnx"),
                )
                .await?;
                let staging_for_extract = staging.clone();
                let archive_for_extract = archive_part.clone();
                tokio::task::spawn_blocking(move || {
                    extract_segmentation_archive(&archive_for_extract, &staging_for_extract)
                })
                .await
                .map_err(|error| anyhow!("Segmentation extraction task failed: {error}"))??;
                (
                    SEGMENTATION_URL,
                    SEGMENTATION_SHA256,
                    EMBEDDING_URL,
                )
            } else {
                (
                    SEGMENTATION_MODELSCOPE_URL,
                    SEGMENTATION_MODEL_SHA256,
                    EMBEDDING_MODELSCOPE_URL,
                )
            };

        let manifest = SpeakerModelManifest {
            id: MODEL_ID.to_string(),
            version: 1,
            backend: BACKEND_ID.to_string(),
            segmentation_source: segmentation_source.to_string(),
            segmentation_sha256: segmentation_sha256.to_string(),
            segmentation_model_sha256: SEGMENTATION_MODEL_SHA256.to_string(),
            embedding_source: embedding_source.to_string(),
            embedding_sha256: EMBEDDING_SHA256.to_string(),
        };
        tokio::fs::write(
            staging.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest)?,
        )
        .await?;

        let staging_paths = paths_for_root(staging.clone());
        validate_installation(&staging_paths)?;

        let backup_root = parent.join(format!(".{}.backup", MODEL_ID));
        if backup_root.exists() {
            tokio::fs::remove_dir_all(&backup_root).await?;
        }
        let _ = tokio::fs::remove_dir_all(&cache_root).await;
        if final_root.exists() {
            tokio::fs::rename(&final_root, &backup_root).await?;
        }
        if let Err(error) = tokio::fs::rename(&staging, &final_root).await {
            if backup_root.exists() {
                let _ = tokio::fs::rename(&backup_root, &final_root).await;
            }
            return Err(error.into());
        }
        if backup_root.exists() {
            tokio::fs::remove_dir_all(&backup_root).await?;
        }
        Ok::<(), anyhow::Error>(())
    }
    .await;

    match result {
        Ok(()) => {
            let _ = app.emit(
                "speaker-diarization-model-download-complete",
                serde_json::json!({ "model_id": MODEL_ID }),
            );
            Ok(())
        }
        Err(error) => {
            let _ = tokio::fs::remove_dir_all(&staging).await;
            let _ = app.emit(
                "speaker-diarization-model-download-error",
                serde_json::json!({ "model_id": MODEL_ID, "error": error.to_string() }),
            );
            Err(error)
        }
    }
}

pub async fn delete_model<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    let root = model_root(app)?;
    if root.exists() {
        tokio::fs::remove_dir_all(root).await?;
    }
    Ok(())
}

fn emit_download_progress<R: Runtime>(app: &AppHandle<R>, downloaded: u64, total: u64) {
    let progress = if total == 0 {
        0
    } else {
        ((downloaded.saturating_mul(100) / total).min(99)) as u8
    };
    let _ = app.emit(
        "speaker-diarization-model-download-progress",
        DownloadProgressEvent {
            model_id: MODEL_ID.to_string(),
            progress,
            downloaded_mb: downloaded as f64 / 1_000_000.0,
            total_mb: total as f64 / 1_000_000.0,
            status: "downloading".to_string(),
        },
    );
}

fn verify_sha256(path: &Path, expected: &str) -> Result<()> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 128 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    let actual = format!("{:x}", hasher.finalize());
    if actual != expected {
        return Err(anyhow!(
            "Checksum mismatch for {}: expected {}, got {}",
            path.display(),
            expected,
            actual
        ));
    }
    Ok(())
}

fn extract_segmentation_archive(archive_path: &Path, staging: &Path) -> Result<()> {
    let segmentation_dir = staging.join("segmentation");
    let licenses_dir = staging.join("licenses");
    std::fs::create_dir_all(&segmentation_dir)?;
    std::fs::create_dir_all(&licenses_dir)?;

    let decoder = BzDecoder::new(File::open(archive_path)?);
    let mut archive = Archive::new(decoder);
    let mut model_found = false;
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_path_buf();
        let filename = path.file_name().and_then(|name| name.to_str());
        let destination = match filename {
            Some("model.int8.onnx") => {
                model_found = true;
                Some(segmentation_dir.join("model.int8.onnx"))
            }
            Some("LICENSE") => Some(licenses_dir.join("pyannote-segmentation-MIT.txt")),
            Some("README.md") => Some(licenses_dir.join("pyannote-segmentation-README.md")),
            _ => None,
        };
        if let Some(destination) = destination {
            let mut output = File::create(destination)?;
            std::io::copy(&mut entry, &mut output)?;
            output.flush()?;
        }
    }
    if !model_found {
        return Err(anyhow!(
            "Segmentation archive did not contain model.int8.onnx"
        ));
    }

    write_speaker_license_files(staging)?;
    Ok(())
}

fn write_speaker_license_files(staging: &Path) -> Result<()> {
    let licenses_dir = staging.join("licenses");
    std::fs::create_dir_all(&licenses_dir)?;
    std::fs::write(
        licenses_dir.join("3D-Speaker-Apache-2.0.txt"),
        "3D-Speaker is licensed under the Apache License 2.0.\nhttps://github.com/modelscope/3D-Speaker\n",
    )?;
    std::fs::write(
        licenses_dir.join("sherpa-onnx-Apache-2.0.txt"),
        "sherpa-onnx is licensed under the Apache License 2.0.\nhttps://github.com/k2-fsa/sherpa-onnx\n",
    )?;
    Ok(())
}

fn validate_installation(paths: &SpeakerModelPaths) -> Result<()> {
    if !paths.segmentation.is_file() {
        return Err(anyhow!("Segmentation model is missing"));
    }
    if !paths.embedding.is_file() {
        return Err(anyhow!("Speaker embedding model is missing"));
    }
    if std::fs::metadata(&paths.segmentation)?.len() < 1_000_000 {
        return Err(anyhow!("Segmentation model is incomplete"));
    }
    if std::fs::metadata(&paths.embedding)?.len() < 35_000_000 {
        return Err(anyhow!("Speaker embedding model is incomplete"));
    }
    verify_sha256(&paths.segmentation, SEGMENTATION_MODEL_SHA256)?;
    verify_sha256(&paths.embedding, EMBEDDING_SHA256)?;
    let manifest_path = paths.root.join("manifest.json");
    let manifest: SpeakerModelManifest = serde_json::from_slice(&std::fs::read(&manifest_path)?)?;
    let supported_modelscope = manifest.segmentation_source == SEGMENTATION_MODELSCOPE_URL
        && manifest.segmentation_sha256 == SEGMENTATION_MODEL_SHA256
        && manifest.embedding_source == EMBEDDING_MODELSCOPE_URL;
    let supported_legacy = manifest.segmentation_source == SEGMENTATION_URL
        && manifest.segmentation_sha256 == SEGMENTATION_SHA256
        && manifest.embedding_source == EMBEDDING_URL;
    if manifest.id != MODEL_ID
        || manifest.version != 1
        || manifest.backend != BACKEND_ID
        || (!supported_modelscope && !supported_legacy)
        || manifest.segmentation_model_sha256 != SEGMENTATION_MODEL_SHA256
        || manifest.embedding_sha256 != EMBEDDING_SHA256
    {
        return Err(anyhow!("Unsupported speaker model manifest"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_paths_are_stable() {
        let paths = paths_for_root(PathBuf::from("/tmp/speaker-model"));
        assert!(paths.segmentation.ends_with("segmentation/model.int8.onnx"));
        assert!(paths
            .embedding
            .ends_with("embedding/3dspeaker_eres2net.onnx"));
    }

    #[test]
    fn checksum_validation_rejects_corrupted_content() {
        let dir = tempfile::tempdir().unwrap();
        let model = dir.path().join("model.onnx");
        std::fs::write(&model, b"corrupted").unwrap();

        let error = verify_sha256(&model, SEGMENTATION_MODEL_SHA256).unwrap_err();
        assert!(error.to_string().contains("Checksum mismatch"));
    }

    #[test]
    fn installation_validation_fails_open_on_missing_assets() {
        let dir = tempfile::tempdir().unwrap();
        let paths = paths_for_root(dir.path().join("sherpa-v1"));

        let error = validate_installation(&paths).unwrap_err();
        assert!(error.to_string().contains("Segmentation model is missing"));
    }

    #[test]
    fn speaker_models_prefer_pinned_modelscope_and_keep_legacy_sources() {
        assert!(SEGMENTATION_MODELSCOPE_URL.contains("103d397e9706dbb03f458fad62430ee8e9ae2bb4"));
        assert!(EMBEDDING_MODELSCOPE_URL.contains("38dbd263d67cf31fa0bb4c1184a31289f9fd94a8"));
        assert!(SEGMENTATION_URL.contains("github.com"));
        assert!(EMBEDDING_URL.contains("github.com"));
    }
}
