use anyhow::{anyhow, Context, Result};
use bzip2::read::BzDecoder;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use tar::Archive;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Copy)]
pub struct ModelFileSpec {
    pub source_path: &'static str,
    pub install_path: &'static str,
    pub size: u64,
    pub sha256: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct DirectDownloadFileSpec {
    pub url: &'static str,
    pub install_path: &'static str,
    pub size: u64,
    pub sha256: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct LicenseFileSpec {
    pub install_path: &'static str,
    pub contents: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub enum ModelInstallSource {
    Archive {
        url: &'static str,
        sha256: &'static str,
        files: &'static [ModelFileSpec],
    },
    DirectFiles {
        files: &'static [DirectDownloadFileSpec],
    },
}

#[derive(Debug, Clone, Copy)]
pub struct ModelInstallSpec {
    pub id: &'static str,
    pub provider: &'static str,
    pub backend: &'static str,
    pub source: ModelInstallSource,
    pub download_size: u64,
    pub installed_size: u64,
    pub licenses: &'static [LicenseFileSpec],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledFileManifest {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelAssetManifest {
    pub schema_version: u32,
    pub id: String,
    pub provider: String,
    pub backend: String,
    pub source: String,
    pub source_sha256: Option<String>,
    pub download_size: u64,
    pub installed_size: u64,
    pub files: Vec<InstalledFileManifest>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelDownloadProgress {
    pub model_id: String,
    pub progress: u8,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub downloaded_mb: f64,
    pub total_mb: f64,
    pub status: String,
}

pub fn validate_installation(root: &Path, spec: &ModelInstallSpec) -> Result<()> {
    let manifest_path = root.join("manifest.json");
    let manifest: ModelAssetManifest = serde_json::from_slice(
        &std::fs::read(&manifest_path)
            .with_context(|| format!("Unable to read {}", manifest_path.display()))?,
    )?;

    if manifest.schema_version != 1
        || manifest.id != spec.id
        || manifest.provider != spec.provider
        || manifest.backend != spec.backend
        || manifest.download_size != spec.download_size
        || manifest.installed_size != spec.installed_size
    {
        return Err(anyhow!("Unsupported or stale model manifest"));
    }

    let expected_files = expected_files(spec);
    if manifest.files.len() != expected_files.len() {
        return Err(anyhow!("Model manifest file list is incomplete"));
    }

    for expected in expected_files {
        let manifest_file = manifest
            .files
            .iter()
            .find(|file| file.path == expected.install_path)
            .ok_or_else(|| anyhow!("Manifest is missing {}", expected.install_path))?;
        if manifest_file.size != expected.size || manifest_file.sha256 != expected.sha256 {
            return Err(anyhow!(
                "Manifest integrity data is stale for {}",
                expected.install_path
            ));
        }
        verify_file(
            &root.join(expected.install_path),
            expected.size,
            expected.sha256,
        )?;
    }

    for license in spec.licenses {
        if !root.join(license.install_path).is_file() {
            return Err(anyhow!(
                "Installed model is missing {}",
                license.install_path
            ));
        }
    }

    Ok(())
}

pub async fn install_model<R: Runtime>(
    app: &AppHandle<R>,
    final_root: &Path,
    spec: &ModelInstallSpec,
    event_prefix: &str,
) -> Result<()> {
    let parent = final_root
        .parent()
        .ok_or_else(|| anyhow!("Invalid model directory"))?
        .to_path_buf();
    tokio::fs::create_dir_all(&parent).await?;

    let staging = parent.join(format!(".{}.download", spec.id));
    let backup = parent.join(format!(".{}.backup", spec.id));
    if staging.exists() {
        tokio::fs::remove_dir_all(&staging).await?;
    }
    if backup.exists() {
        if final_root.exists() {
            tokio::fs::remove_dir_all(&backup).await?;
        } else {
            tokio::fs::rename(&backup, final_root).await?;
        }
    }
    tokio::fs::create_dir_all(&staging).await?;

    let result = async {
        match spec.source {
            ModelInstallSource::Archive { url, sha256, files } => {
                let archive_part = staging.join("model.tar.bz2.part");
                download_file(
                    app,
                    spec.id,
                    url,
                    &archive_part,
                    0,
                    spec.download_size,
                    event_prefix,
                )
                .await?;
                verify_sha256(&archive_part, sha256)?;

                let archive_for_extract = archive_part.clone();
                let staging_for_extract = staging.clone();
                tokio::task::spawn_blocking(move || {
                    extract_selected_archive_files(
                        &archive_for_extract,
                        &staging_for_extract,
                        files,
                    )
                })
                .await
                .map_err(|error| anyhow!("Model extraction task failed: {error}"))??;
                tokio::fs::remove_file(archive_part).await?;
            }
            ModelInstallSource::DirectFiles { files } => {
                let mut completed = 0u64;
                for file in files {
                    let final_path = staging.join(file.install_path);
                    if let Some(parent) = final_path.parent() {
                        tokio::fs::create_dir_all(parent).await?;
                    }
                    let part_path = final_path.with_extension(
                        final_path
                            .extension()
                            .map(|extension| format!("{}.part", extension.to_string_lossy()))
                            .unwrap_or_else(|| "part".to_string()),
                    );
                    download_file(
                        app,
                        spec.id,
                        file.url,
                        &part_path,
                        completed,
                        spec.download_size,
                        event_prefix,
                    )
                    .await?;
                    verify_file(&part_path, file.size, file.sha256)?;
                    tokio::fs::rename(&part_path, &final_path).await?;
                    completed = completed.saturating_add(file.size);
                }
            }
        }

        for license in spec.licenses {
            let path = staging.join(license.install_path);
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(path, license.contents.as_bytes()).await?;
        }

        let source = match spec.source {
            ModelInstallSource::Archive { url, .. } => url.to_string(),
            ModelInstallSource::DirectFiles { files } => files
                .first()
                .map(|file| file.url.to_string())
                .unwrap_or_default(),
        };
        let source_sha256 = match spec.source {
            ModelInstallSource::Archive { sha256, .. } => Some(sha256.to_string()),
            ModelInstallSource::DirectFiles { .. } => None,
        };
        let files = expected_files(spec)
            .into_iter()
            .map(|file| InstalledFileManifest {
                path: file.install_path.to_string(),
                size: file.size,
                sha256: file.sha256.to_string(),
            })
            .collect();
        let manifest = ModelAssetManifest {
            schema_version: 1,
            id: spec.id.to_string(),
            provider: spec.provider.to_string(),
            backend: spec.backend.to_string(),
            source,
            source_sha256,
            download_size: spec.download_size,
            installed_size: spec.installed_size,
            files,
        };
        tokio::fs::write(
            staging.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest)?,
        )
        .await?;

        validate_installation(&staging, spec)?;

        if final_root.exists() {
            tokio::fs::rename(final_root, &backup).await?;
        }
        if let Err(error) = tokio::fs::rename(&staging, final_root).await {
            if backup.exists() {
                let _ = tokio::fs::rename(&backup, final_root).await;
            }
            return Err(error.into());
        }
        if backup.exists() {
            if let Err(error) = tokio::fs::remove_dir_all(&backup).await {
                log::warn!(
                    "Unable to remove model backup {} after successful install: {}",
                    backup.display(),
                    error
                );
            }
        }

        emit_progress(
            app,
            spec.id,
            spec.download_size,
            spec.download_size,
            "complete",
            event_prefix,
        );
        Ok::<(), anyhow::Error>(())
    }
    .await;

    if let Err(error) = &result {
        let _ = tokio::fs::remove_dir_all(&staging).await;
        let event = format!("{event_prefix}-error");
        let _ = app.emit(
            &event,
            serde_json::json!({ "model_id": spec.id, "error": error.to_string() }),
        );
    } else {
        let event = format!("{event_prefix}-complete");
        let _ = app.emit(&event, serde_json::json!({ "model_id": spec.id }));
    }

    result
}

pub async fn delete_model(root: &Path) -> Result<()> {
    if root.exists() {
        tokio::fs::remove_dir_all(root).await?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct ExpectedFile {
    install_path: &'static str,
    size: u64,
    sha256: &'static str,
}

fn expected_files(spec: &ModelInstallSpec) -> Vec<ExpectedFile> {
    match spec.source {
        ModelInstallSource::Archive { files, .. } => files
            .iter()
            .map(|file| ExpectedFile {
                install_path: file.install_path,
                size: file.size,
                sha256: file.sha256,
            })
            .collect(),
        ModelInstallSource::DirectFiles { files } => files
            .iter()
            .map(|file| ExpectedFile {
                install_path: file.install_path,
                size: file.size,
                sha256: file.sha256,
            })
            .collect(),
    }
}

async fn download_file<R: Runtime>(
    app: &AppHandle<R>,
    model_id: &str,
    url: &str,
    destination: &Path,
    completed_before: u64,
    total_bytes: u64,
    event_prefix: &str,
) -> Result<()> {
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .timeout(std::time::Duration::from_secs(3600))
        .build()?;
    let response = client.get(url).send().await?.error_for_status()?;
    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(destination).await?;
    let mut downloaded = 0u64;
    let mut last_progress = u8::MAX;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded = downloaded.saturating_add(chunk.len() as u64);
        let total_downloaded = completed_before.saturating_add(downloaded);
        let progress = if total_bytes == 0 {
            0
        } else {
            ((total_downloaded.saturating_mul(100) / total_bytes).min(100)) as u8
        };
        if progress != last_progress {
            emit_progress(
                app,
                model_id,
                total_downloaded,
                total_bytes,
                "downloading",
                event_prefix,
            );
            last_progress = progress;
        }
    }
    file.flush().await?;
    Ok(())
}

fn emit_progress<R: Runtime>(
    app: &AppHandle<R>,
    model_id: &str,
    downloaded_bytes: u64,
    total_bytes: u64,
    status: &str,
    event_prefix: &str,
) {
    let progress = if total_bytes == 0 {
        0
    } else {
        ((downloaded_bytes.saturating_mul(100) / total_bytes).min(100)) as u8
    };
    let event = format!("{event_prefix}-progress");
    let _ = app.emit(
        &event,
        ModelDownloadProgress {
            model_id: model_id.to_string(),
            progress,
            downloaded_bytes,
            total_bytes,
            downloaded_mb: downloaded_bytes as f64 / 1_048_576.0,
            total_mb: total_bytes as f64 / 1_048_576.0,
            status: status.to_string(),
        },
    );
}

fn extract_selected_archive_files(
    archive_path: &Path,
    destination: &Path,
    files: &[ModelFileSpec],
) -> Result<()> {
    let expected: HashMap<&str, &ModelFileSpec> =
        files.iter().map(|file| (file.source_path, file)).collect();
    let file = File::open(archive_path)?;
    let decoder = BzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    let mut extracted = HashMap::<String, bool>::new();

    for entry in archive.entries()? {
        let mut entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = entry
            .path()?
            .to_str()
            .ok_or_else(|| anyhow!("Model archive contains a non-UTF-8 path"))?
            .to_string();
        let Some(spec) = expected.get(path.as_str()) else {
            continue;
        };
        let output = destination.join(spec.install_path);
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut output_file = File::create(&output)?;
        std::io::copy(&mut entry, &mut output_file)?;
        output_file.flush()?;
        extracted.insert(path, true);
    }

    for file in files {
        if !extracted.contains_key(file.source_path) {
            return Err(anyhow!("Model archive is missing {}", file.source_path));
        }
        verify_file(&destination.join(file.install_path), file.size, file.sha256)?;
    }
    Ok(())
}

fn verify_file(path: &Path, expected_size: u64, expected_sha256: &str) -> Result<()> {
    let metadata =
        std::fs::metadata(path).with_context(|| format!("Unable to inspect {}", path.display()))?;
    if metadata.len() != expected_size {
        return Err(anyhow!(
            "{} has {} bytes; expected {}",
            path.display(),
            metadata.len(),
            expected_size
        ));
    }
    verify_sha256(path, expected_sha256)
}

fn verify_sha256(path: &Path, expected: &str) -> Result<()> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    // Keep the read buffer small: this can run on the Windows main thread
    // (sync Tauri commands) where only 1 MB of stack is available.
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let actual = format!("{:x}", hasher.finalize());
    if actual != expected {
        return Err(anyhow!(
            "SHA256 mismatch for {}: expected {}, got {}",
            path.display(),
            expected,
            actual
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn expected_files_preserve_install_paths() {
        static FILES: &[DirectDownloadFileSpec] = &[DirectDownloadFileSpec {
            url: "https://example.invalid/model",
            install_path: "nested/model.onnx",
            size: 1,
            sha256: "00",
        }];
        static LICENSES: &[LicenseFileSpec] = &[];
        let spec = ModelInstallSpec {
            id: "test",
            provider: "test",
            backend: "test",
            source: ModelInstallSource::DirectFiles { files: FILES },
            download_size: 1,
            installed_size: 1,
            licenses: LICENSES,
        };
        let files = expected_files(&spec);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].install_path, "nested/model.onnx");
    }

    #[test]
    fn validate_installation_rejects_a_changed_model_file() {
        static FILES: &[DirectDownloadFileSpec] = &[DirectDownloadFileSpec {
            url: "https://example.invalid/model",
            install_path: "model.bin",
            size: 3,
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        }];
        static LICENSES: &[LicenseFileSpec] = &[];
        let spec = ModelInstallSpec {
            id: "test",
            provider: "test-provider",
            backend: "test-backend",
            source: ModelInstallSource::DirectFiles { files: FILES },
            download_size: 3,
            installed_size: 3,
            licenses: LICENSES,
        };
        let directory = tempdir().unwrap();
        std::fs::write(directory.path().join("model.bin"), b"abc").unwrap();
        let manifest = ModelAssetManifest {
            schema_version: 1,
            id: spec.id.to_string(),
            provider: spec.provider.to_string(),
            backend: spec.backend.to_string(),
            source: FILES[0].url.to_string(),
            source_sha256: None,
            download_size: spec.download_size,
            installed_size: spec.installed_size,
            files: vec![InstalledFileManifest {
                path: FILES[0].install_path.to_string(),
                size: FILES[0].size,
                sha256: FILES[0].sha256.to_string(),
            }],
        };
        std::fs::write(
            directory.path().join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();

        validate_installation(directory.path(), &spec).unwrap();
        std::fs::write(directory.path().join("model.bin"), b"abd").unwrap();
        assert!(validate_installation(directory.path(), &spec).is_err());
    }

    #[test]
    fn validate_installation_rejects_stale_manifest_metadata() {
        static FILES: &[DirectDownloadFileSpec] = &[DirectDownloadFileSpec {
            url: "https://example.invalid/model",
            install_path: "model.bin",
            size: 3,
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        }];
        static LICENSES: &[LicenseFileSpec] = &[];
        let spec = ModelInstallSpec {
            id: "test",
            provider: "test-provider",
            backend: "test-backend",
            source: ModelInstallSource::DirectFiles { files: FILES },
            download_size: 3,
            installed_size: 3,
            licenses: LICENSES,
        };
        let directory = tempdir().unwrap();
        std::fs::write(directory.path().join("model.bin"), b"abc").unwrap();
        let manifest = ModelAssetManifest {
            schema_version: 1,
            id: "different-model".to_string(),
            provider: spec.provider.to_string(),
            backend: spec.backend.to_string(),
            source: FILES[0].url.to_string(),
            source_sha256: None,
            download_size: spec.download_size,
            installed_size: spec.installed_size,
            files: vec![InstalledFileManifest {
                path: FILES[0].install_path.to_string(),
                size: FILES[0].size,
                sha256: FILES[0].sha256.to_string(),
            }],
        };
        std::fs::write(
            directory.path().join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();

        assert!(validate_installation(directory.path(), &spec).is_err());
    }
}
