use anyhow::{anyhow, Context, Result};
use bzip2::read::BzDecoder;
use futures_util::StreamExt;
use reqwest::{
    header::{CONTENT_RANGE, RANGE},
    StatusCode,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    TarBz2,
    Zip,
}

#[derive(Debug, Clone, Copy)]
pub struct ArchiveSourceSpec {
    pub label: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
    pub download_size: u64,
    pub format: ArchiveFormat,
    pub files: &'static [ModelFileSpec],
}

#[derive(Debug, Clone, Copy)]
pub struct DirectSourceSpec {
    pub label: &'static str,
    pub download_size: u64,
    pub files: &'static [DirectDownloadFileSpec],
}

#[derive(Debug, Clone, Copy)]
pub enum ModelInstallSource {
    Archive {
        url: &'static str,
        sha256: &'static str,
        files: &'static [ModelFileSpec],
    },
    ArchiveVariants {
        sources: &'static [ArchiveSourceSpec],
    },
    DirectFiles {
        files: &'static [DirectDownloadFileSpec],
    },
    DirectFileVariants {
        sources: &'static [DirectSourceSpec],
    },
    HybridVariants {
        direct_sources: &'static [DirectSourceSpec],
        archive_sources: &'static [ArchiveSourceSpec],
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
    {
        return Err(anyhow!("Unsupported or stale model manifest"));
    }

    let expected_files = expected_files_for_manifest(spec, &manifest)
        .ok_or_else(|| anyhow!("Model manifest does not match a supported artifact version"))?;

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
    let cache_root = parent.join(".downloads").join(spec.id);
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
    tokio::fs::create_dir_all(&cache_root).await?;

    log::info!(
        "Starting verified model installation: model={}, destination={}",
        spec.id,
        final_root.display()
    );

    let result = async {
        let selected = match spec.source {
            ModelInstallSource::Archive { url, sha256, files } => {
                install_archive_source(
                    app,
                    &staging,
                    &cache_root,
                    ArchiveSourceSpec {
                        label: "upstream",
                        url,
                        sha256,
                        download_size: spec.download_size,
                        format: ArchiveFormat::TarBz2,
                        files,
                    },
                    spec.id,
                    event_prefix,
                )
                .await?
            }
            ModelInstallSource::ArchiveVariants { sources } => {
                let mut failures = Vec::new();
                let mut installed = None;
                for source in sources {
                    if staging.exists() {
                        tokio::fs::remove_dir_all(&staging).await?;
                    }
                    tokio::fs::create_dir_all(&staging).await?;
                    match install_archive_source(
                        app,
                        &staging,
                        &cache_root,
                        *source,
                        spec.id,
                        event_prefix,
                    )
                    .await
                    {
                        Ok(selected) => {
                            installed = Some(selected);
                            break;
                        }
                        Err(error) => {
                            log::warn!(
                                "Model source '{}' failed for {}: {:#}",
                                source.label,
                                spec.id,
                                error
                            );
                            failures.push(format!("{}: {error:#}", source.label));
                        }
                    }
                }
                installed.ok_or_else(|| {
                    anyhow!(
                        "All model download sources failed for {}: {}",
                        spec.id,
                        failures.join(" | ")
                    )
                })?
            }
            ModelInstallSource::DirectFiles { files } => {
                install_direct_source(
                    app,
                    &staging,
                    &cache_root,
                    DirectSourceSpec {
                        label: "upstream",
                        download_size: spec.download_size,
                        files,
                    },
                    spec.id,
                    event_prefix,
                )
                .await?
            }
            ModelInstallSource::DirectFileVariants { sources } => {
                let mut failures = Vec::new();
                let mut installed = None;
                for source in sources {
                    if staging.exists() {
                        tokio::fs::remove_dir_all(&staging).await?;
                    }
                    tokio::fs::create_dir_all(&staging).await?;
                    match install_direct_source(
                        app,
                        &staging,
                        &cache_root,
                        *source,
                        spec.id,
                        event_prefix,
                    )
                    .await
                    {
                        Ok(selected) => {
                            installed = Some(selected);
                            break;
                        }
                        Err(error) => {
                            log::warn!(
                                "Model source '{}' failed for {}: {:#}",
                                source.label,
                                spec.id,
                                error
                            );
                            failures.push(format!("{}: {error:#}", source.label));
                        }
                    }
                }
                installed.ok_or_else(|| {
                    anyhow!(
                        "All model download sources failed for {}: {}",
                        spec.id,
                        failures.join(" | ")
                    )
                })?
            }
            ModelInstallSource::HybridVariants {
                direct_sources,
                archive_sources,
            } => {
                let mut failures = Vec::new();
                let mut installed = None;
                for source in direct_sources {
                    if staging.exists() {
                        tokio::fs::remove_dir_all(&staging).await?;
                    }
                    tokio::fs::create_dir_all(&staging).await?;
                    match install_direct_source(
                        app,
                        &staging,
                        &cache_root,
                        *source,
                        spec.id,
                        event_prefix,
                    )
                    .await
                    {
                        Ok(selected) => {
                            installed = Some(selected);
                            break;
                        }
                        Err(error) => {
                            log::warn!(
                                "Model source '{}' failed for {}: {:#}",
                                source.label,
                                spec.id,
                                error
                            );
                            failures.push(format!("{}: {error:#}", source.label));
                        }
                    }
                }
                if installed.is_none() {
                    for source in archive_sources {
                        if staging.exists() {
                            tokio::fs::remove_dir_all(&staging).await?;
                        }
                        tokio::fs::create_dir_all(&staging).await?;
                        match install_archive_source(
                            app,
                            &staging,
                            &cache_root,
                            *source,
                            spec.id,
                            event_prefix,
                        )
                        .await
                        {
                            Ok(selected) => {
                                installed = Some(selected);
                                break;
                            }
                            Err(error) => {
                                log::warn!(
                                    "Model source '{}' failed for {}: {:#}",
                                    source.label,
                                    spec.id,
                                    error
                                );
                                failures.push(format!("{}: {error:#}", source.label));
                            }
                        }
                    }
                }
                installed.ok_or_else(|| {
                    anyhow!(
                        "All model download sources failed for {}: {}",
                        spec.id,
                        failures.join(" | ")
                    )
                })?
            }
        };

        let completed_bytes = selected.download_size;
        finalize_installation(&staging, &backup, final_root, spec, selected).await?;

        emit_progress(
            app,
            spec.id,
            completed_bytes,
            completed_bytes,
            "complete",
            event_prefix,
        );
        let _ = tokio::fs::remove_dir_all(&cache_root).await;
        log::info!("Verified model installation completed: {}", spec.id);
        Ok::<(), anyhow::Error>(())
    }
    .await;

    if let Err(error) = &result {
        let _ = tokio::fs::remove_dir_all(&staging).await;
        log::error!(
            "Verified model installation failed for {}: {error:#}",
            spec.id
        );
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

pub fn recognizes_archive(path: &Path, spec: &ModelInstallSpec) -> Result<bool> {
    Ok(matching_archive_source(path, spec)?.is_some())
}

pub fn recognizes_directory(path: &Path, spec: &ModelInstallSpec) -> Result<bool> {
    Ok(matching_directory_source(path, spec)?.is_some())
}

pub async fn import_model_archive(
    final_root: &Path,
    spec: &ModelInstallSpec,
    archive_path: &Path,
) -> Result<()> {
    let archive_path = archive_path.to_path_buf();
    let archive_for_match = archive_path.clone();
    let spec_copy = *spec;
    let source = tokio::task::spawn_blocking(move || {
        matching_archive_source(&archive_for_match, &spec_copy)
    })
    .await
    .map_err(|error| anyhow!("Offline archive inspection failed: {error}"))??
    .ok_or_else(|| {
        anyhow!(
            "The selected archive is not an exact supported artifact for {}",
            spec.id
        )
    })?;

    let (staging, backup) = create_import_transaction(final_root, spec.id).await?;
    let staging_for_extract = staging.clone();
    let result = async {
        tokio::task::spawn_blocking(move || {
            extract_selected_archive_files(
                &archive_path,
                &staging_for_extract,
                source.files,
                source.format,
            )
        })
        .await
        .map_err(|error| anyhow!("Offline model extraction failed: {error}"))??;

        finalize_installation(
            &staging,
            &backup,
            final_root,
            spec,
            SelectedArtifact::from_archive(source, "offline-archive"),
        )
        .await
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_dir_all(&staging).await;
        if backup.exists() && !final_root.exists() {
            let _ = tokio::fs::rename(&backup, final_root).await;
        }
    }
    result
}

pub async fn import_model_directory(
    final_root: &Path,
    spec: &ModelInstallSpec,
    source_root: &Path,
) -> Result<()> {
    let source_root = source_root.to_path_buf();
    let spec_copy = *spec;
    let (matched_root, selected) =
        tokio::task::spawn_blocking(move || matching_directory_source(&source_root, &spec_copy))
            .await
            .map_err(|error| anyhow!("Offline model directory inspection failed: {error}"))??
            .ok_or_else(|| {
                anyhow!(
                    "No exact supported {} model was found in the selected directory",
                    spec.id
                )
            })?;

    let (staging, backup) = create_import_transaction(final_root, spec.id).await?;
    let staging_for_copy = staging.clone();
    let selected_for_copy = selected.clone();
    let result = async {
        tokio::task::spawn_blocking(move || {
            copy_selected_directory_files(
                &matched_root,
                &staging_for_copy,
                &selected_for_copy.files,
            )
        })
        .await
        .map_err(|error| anyhow!("Offline model copy failed: {error}"))??;

        finalize_installation(&staging, &backup, final_root, spec, selected).await
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_dir_all(&staging).await;
        if backup.exists() && !final_root.exists() {
            let _ = tokio::fs::rename(&backup, final_root).await;
        }
    }
    result
}

async fn create_import_transaction(
    final_root: &Path,
    model_id: &str,
) -> Result<(PathBuf, PathBuf)> {
    let parent = final_root
        .parent()
        .ok_or_else(|| anyhow!("Invalid model directory"))?;
    tokio::fs::create_dir_all(parent).await?;
    let transaction = uuid::Uuid::new_v4();
    let staging = parent.join(format!(".{model_id}.import-{transaction}"));
    let backup = parent.join(format!(".{model_id}.backup-{transaction}"));
    tokio::fs::create_dir_all(&staging).await?;
    Ok((staging, backup))
}

#[derive(Debug, Clone)]
struct SelectedArtifact {
    source: String,
    source_sha256: Option<String>,
    download_size: u64,
    installed_size: u64,
    files: Vec<ModelFileSpec>,
}

impl SelectedArtifact {
    fn from_archive(source: ArchiveSourceSpec, source_kind: &str) -> Self {
        Self {
            source: format!("{source_kind}:{}", source.label),
            source_sha256: Some(source.sha256.to_string()),
            download_size: source.download_size,
            installed_size: source.files.iter().map(|file| file.size).sum(),
            files: source.files.to_vec(),
        }
    }
}

fn matching_archive_source(
    archive_path: &Path,
    spec: &ModelInstallSpec,
) -> Result<Option<ArchiveSourceSpec>> {
    let archive_size = std::fs::metadata(archive_path)
        .with_context(|| format!("Unable to inspect {}", archive_path.display()))?
        .len();
    let candidates = archive_sources(spec);
    let same_size: Vec<_> = candidates
        .into_iter()
        .filter(|source| source.download_size == archive_size)
        .collect();
    if same_size.is_empty() {
        return Ok(None);
    }
    let sha256 = file_sha256(archive_path)?;
    Ok(same_size.into_iter().find(|source| source.sha256 == sha256))
}

fn matching_directory_source(
    selected_root: &Path,
    spec: &ModelInstallSpec,
) -> Result<Option<(PathBuf, SelectedArtifact)>> {
    for root in candidate_model_roots(selected_root, 2, 256)? {
        for selected in file_variants(spec) {
            if directory_matches_files(&root, &selected.files)? {
                return Ok(Some((root, selected)));
            }
        }
    }
    Ok(None)
}

fn archive_sources(spec: &ModelInstallSpec) -> Vec<ArchiveSourceSpec> {
    match spec.source {
        ModelInstallSource::Archive { url, sha256, files } => vec![ArchiveSourceSpec {
            label: "upstream",
            url,
            sha256,
            download_size: spec.download_size,
            format: ArchiveFormat::TarBz2,
            files,
        }],
        ModelInstallSource::ArchiveVariants { sources } => sources.to_vec(),
        ModelInstallSource::HybridVariants {
            archive_sources, ..
        } => archive_sources.to_vec(),
        ModelInstallSource::DirectFiles { .. } | ModelInstallSource::DirectFileVariants { .. } => {
            Vec::new()
        }
    }
}

fn file_variants(spec: &ModelInstallSpec) -> Vec<SelectedArtifact> {
    match spec.source {
        ModelInstallSource::Archive { .. } | ModelInstallSource::ArchiveVariants { .. } => {
            archive_sources(spec)
                .into_iter()
                .map(|source| SelectedArtifact::from_archive(source, "offline-directory"))
                .collect()
        }
        ModelInstallSource::DirectFiles { files } => vec![SelectedArtifact {
            source: "offline-directory:verified-files".to_string(),
            source_sha256: None,
            download_size: spec.download_size,
            installed_size: files.iter().map(|file| file.size).sum(),
            files: direct_model_files(files),
        }],
        ModelInstallSource::DirectFileVariants { sources } => sources
            .iter()
            .map(|source| SelectedArtifact {
                source: format!("offline-directory:{}", source.label),
                source_sha256: None,
                download_size: source.download_size,
                installed_size: source.files.iter().map(|file| file.size).sum(),
                files: direct_model_files(source.files),
            })
            .collect(),
        ModelInstallSource::HybridVariants {
            direct_sources,
            archive_sources,
        } => direct_sources
            .iter()
            .map(|source| SelectedArtifact {
                source: format!("offline-directory:{}", source.label),
                source_sha256: None,
                download_size: source.download_size,
                installed_size: source.files.iter().map(|file| file.size).sum(),
                files: direct_model_files(source.files),
            })
            .chain(
                archive_sources
                    .iter()
                    .map(|source| SelectedArtifact::from_archive(*source, "offline-directory")),
            )
            .collect(),
    }
}

fn candidate_model_roots(root: &Path, max_depth: usize, max_dirs: usize) -> Result<Vec<PathBuf>> {
    if !root.is_dir() {
        return Err(anyhow!("{} is not a directory", root.display()));
    }
    let mut roots = vec![root.to_path_buf()];
    let mut cursor = 0;
    while cursor < roots.len() && roots.len() < max_dirs {
        let current = roots[cursor].clone();
        cursor += 1;
        let depth = current
            .strip_prefix(root)
            .map(|relative| relative.components().count())
            .unwrap_or(max_depth);
        if depth >= max_depth {
            continue;
        }
        let entries = match std::fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(error) if current != root => {
                log::debug!(
                    "Skipping unreadable offline model subdirectory {}: {}",
                    current.display(),
                    error
                );
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() && !file_type.is_symlink() {
                roots.push(entry.path());
                if roots.len() >= max_dirs {
                    break;
                }
            }
        }
    }
    Ok(roots)
}

fn directory_matches_files(root: &Path, files: &[ModelFileSpec]) -> Result<bool> {
    for file in files {
        let path = root.join(file.install_path);
        let metadata = match std::fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        if !metadata.is_file() || metadata.len() != file.size {
            return Ok(false);
        }
    }
    for file in files {
        if file_sha256(&root.join(file.install_path))? != file.sha256 {
            return Ok(false);
        }
    }
    Ok(true)
}

fn copy_selected_directory_files(
    source_root: &Path,
    destination: &Path,
    files: &[ModelFileSpec],
) -> Result<()> {
    for file in files {
        let output = destination.join(file.install_path);
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(source_root.join(file.install_path), &output)?;
        verify_file(&output, file.size, file.sha256)?;
    }
    Ok(())
}

async fn finalize_installation(
    staging: &Path,
    backup: &Path,
    final_root: &Path,
    spec: &ModelInstallSpec,
    selected: SelectedArtifact,
) -> Result<()> {
    for license in spec.licenses {
        let path = staging.join(license.install_path);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(path, license.contents.as_bytes()).await?;
    }

    let manifest = ModelAssetManifest {
        schema_version: 1,
        id: spec.id.to_string(),
        provider: spec.provider.to_string(),
        backend: spec.backend.to_string(),
        source: selected.source,
        source_sha256: selected.source_sha256,
        download_size: selected.download_size,
        installed_size: selected.installed_size,
        files: selected
            .files
            .into_iter()
            .map(|file| InstalledFileManifest {
                path: file.install_path.to_string(),
                size: file.size,
                sha256: file.sha256.to_string(),
            })
            .collect(),
    };
    tokio::fs::write(
        staging.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )
    .await?;

    validate_installation(staging, spec)?;

    if final_root.exists() {
        tokio::fs::rename(final_root, backup).await?;
    }
    if let Err(error) = tokio::fs::rename(staging, final_root).await {
        if backup.exists() {
            let _ = tokio::fs::rename(backup, final_root).await;
        }
        return Err(error.into());
    }
    if backup.exists() {
        if let Err(error) = tokio::fs::remove_dir_all(backup).await {
            log::warn!(
                "Unable to remove model backup {} after successful install: {}",
                backup.display(),
                error
            );
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct ExpectedFile {
    install_path: &'static str,
    size: u64,
    sha256: &'static str,
}

fn expected_files_for_manifest(
    spec: &ModelInstallSpec,
    manifest: &ModelAssetManifest,
) -> Option<Vec<ExpectedFile>> {
    let candidates: Vec<(&[ModelFileSpec], u64, u64, Option<&str>)> = match spec.source {
        ModelInstallSource::Archive { sha256, files, .. } => {
            vec![(files, spec.download_size, spec.installed_size, Some(sha256))]
        }
        ModelInstallSource::ArchiveVariants { sources } => sources
            .iter()
            .map(|source| {
                (
                    source.files,
                    source.download_size,
                    source.files.iter().map(|file| file.size).sum(),
                    Some(source.sha256),
                )
            })
            .collect(),
        ModelInstallSource::DirectFiles { files } => {
            let model_files = direct_model_files(files);
            return manifest_matches_files(
                manifest,
                &model_files,
                spec.download_size,
                spec.installed_size,
                None,
            )
            .then(|| expected_from_model_files(&model_files));
        }
        ModelInstallSource::DirectFileVariants { sources } => {
            for source in sources {
                let model_files = direct_model_files(source.files);
                if manifest_matches_files(
                    manifest,
                    &model_files,
                    source.download_size,
                    source.files.iter().map(|file| file.size).sum(),
                    None,
                ) {
                    return Some(expected_from_model_files(&model_files));
                }
            }
            return None;
        }
        ModelInstallSource::HybridVariants {
            direct_sources,
            archive_sources,
        } => {
            for source in direct_sources {
                let model_files = direct_model_files(source.files);
                if manifest_matches_files(
                    manifest,
                    &model_files,
                    source.download_size,
                    source.files.iter().map(|file| file.size).sum(),
                    None,
                ) {
                    return Some(expected_from_model_files(&model_files));
                }
            }
            archive_sources
                .iter()
                .map(|source| {
                    (
                        source.files,
                        source.download_size,
                        source.files.iter().map(|file| file.size).sum(),
                        Some(source.sha256),
                    )
                })
                .collect()
        }
    };

    candidates
        .into_iter()
        .find_map(|(files, download_size, installed_size, source_sha256)| {
            manifest_matches_files(
                manifest,
                files,
                download_size,
                installed_size,
                source_sha256,
            )
            .then(|| expected_from_model_files(files))
        })
}

fn manifest_matches_files(
    manifest: &ModelAssetManifest,
    files: &[ModelFileSpec],
    download_size: u64,
    installed_size: u64,
    source_sha256: Option<&str>,
) -> bool {
    manifest.download_size == download_size
        && manifest.installed_size == installed_size
        && manifest.source_sha256.as_deref() == source_sha256
        && manifest.files.len() == files.len()
        && files.iter().all(|expected| {
            manifest.files.iter().any(|actual| {
                actual.path == expected.install_path
                    && actual.size == expected.size
                    && actual.sha256 == expected.sha256
            })
        })
}

fn expected_from_model_files(files: &[ModelFileSpec]) -> Vec<ExpectedFile> {
    files
        .iter()
        .map(|file| ExpectedFile {
            install_path: file.install_path,
            size: file.size,
            sha256: file.sha256,
        })
        .collect()
}

async fn install_archive_source<R: Runtime>(
    app: &AppHandle<R>,
    staging: &Path,
    cache_root: &Path,
    source: ArchiveSourceSpec,
    model_id: &str,
    event_prefix: &str,
) -> Result<SelectedArtifact> {
    log::info!(
        "Trying model source '{}' for {} ({} bytes)",
        source.label,
        model_id,
        source.download_size
    );
    let archive_part = cache_root.join(format!("{}.part", source.sha256));
    download_with_retries(
        app,
        model_id,
        source.url,
        &archive_part,
        source.download_size,
        0,
        source.download_size,
        event_prefix,
    )
    .await?;
    if let Err(error) = verify_sha256(&archive_part, source.sha256) {
        let _ = tokio::fs::remove_file(&archive_part).await;
        return Err(error.context("Downloaded archive failed SHA256 verification"));
    }

    let archive_for_extract = archive_part.clone();
    let staging_for_extract = staging.to_path_buf();
    tokio::task::spawn_blocking(move || {
        extract_selected_archive_files(
            &archive_for_extract,
            &staging_for_extract,
            source.files,
            source.format,
        )
    })
    .await
    .map_err(|error| anyhow!("Model extraction task failed: {error}"))??;

    Ok(SelectedArtifact {
        source: source.url.to_string(),
        source_sha256: Some(source.sha256.to_string()),
        download_size: source.download_size,
        installed_size: source.files.iter().map(|file| file.size).sum(),
        files: source.files.to_vec(),
    })
}

async fn install_direct_source<R: Runtime>(
    app: &AppHandle<R>,
    staging: &Path,
    cache_root: &Path,
    source: DirectSourceSpec,
    model_id: &str,
    event_prefix: &str,
) -> Result<SelectedArtifact> {
    log::info!(
        "Trying model source '{}' for {} ({} bytes)",
        source.label,
        model_id,
        source.download_size
    );
    let mut completed = 0u64;
    for file in source.files {
        let final_path = staging.join(file.install_path);
        if let Some(parent) = final_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let part_path = cache_root.join(format!("{}.part", file.sha256));
        download_with_retries(
            app,
            model_id,
            file.url,
            &part_path,
            file.size,
            completed,
            source.download_size,
            event_prefix,
        )
        .await?;
        if let Err(error) = verify_file(&part_path, file.size, file.sha256) {
            let _ = tokio::fs::remove_file(&part_path).await;
            return Err(error.context("Downloaded model file failed SHA256 verification"));
        }
        tokio::fs::copy(&part_path, &final_path).await?;
        completed = completed.saturating_add(file.size);
    }
    Ok(SelectedArtifact {
        source: format!("direct:{}", source.label),
        source_sha256: None,
        download_size: source.download_size,
        installed_size: source.files.iter().map(|file| file.size).sum(),
        files: direct_model_files(source.files),
    })
}

fn direct_model_files(files: &[DirectDownloadFileSpec]) -> Vec<ModelFileSpec> {
    files
        .iter()
        .map(|file| ModelFileSpec {
            source_path: file.install_path,
            install_path: file.install_path,
            size: file.size,
            sha256: file.sha256,
        })
        .collect()
}

async fn download_with_retries<R: Runtime>(
    app: &AppHandle<R>,
    model_id: &str,
    url: &str,
    destination: &Path,
    expected_file_size: u64,
    completed_before: u64,
    total_bytes: u64,
    event_prefix: &str,
) -> Result<()> {
    let mut last_error = None;
    for attempt in 1..=3 {
        match download_file(
            app,
            model_id,
            url,
            destination,
            expected_file_size,
            completed_before,
            total_bytes,
            event_prefix,
        )
        .await
        {
            Ok(()) => return Ok(()),
            Err(error) => {
                log::warn!("Model download attempt {attempt}/3 failed for {model_id}: {error:#}");
                last_error = Some(error);
            }
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow!("Model download failed")))
}

async fn download_file<R: Runtime>(
    app: &AppHandle<R>,
    model_id: &str,
    url: &str,
    destination: &Path,
    expected_file_size: u64,
    completed_before: u64,
    total_bytes: u64,
    event_prefix: &str,
) -> Result<()> {
    let mut last_progress = None;
    download_file_from_source(
        model_id,
        url,
        destination,
        expected_file_size,
        |downloaded| {
            let total_downloaded = completed_before.saturating_add(downloaded);
            let progress = if total_bytes == 0 {
                0
            } else {
                ((total_downloaded.saturating_mul(100) / total_bytes).min(100)) as u8
            };
            if last_progress != Some(progress) {
                emit_progress(
                    app,
                    model_id,
                    total_downloaded,
                    total_bytes,
                    "downloading",
                    event_prefix,
                );
                last_progress = Some(progress);
            }
            Ok(())
        },
    )
    .await
}

async fn download_file_from_source<F>(
    model_id: &str,
    url: &str,
    destination: &Path,
    expected_file_size: u64,
    mut on_progress: F,
) -> Result<()>
where
    F: FnMut(u64) -> Result<()>,
{
    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .timeout(std::time::Duration::from_secs(3600))
        .build()?;

    let mut existing_size = match tokio::fs::metadata(destination).await {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
        Err(error) => return Err(error.into()),
    };
    if existing_size == expected_file_size {
        log::info!(
            "Reusing completed model download for {} ({} bytes)",
            model_id,
            existing_size
        );
        return Ok(());
    }
    if existing_size > expected_file_size {
        log::warn!(
            "Discarding oversized partial download for {} ({} > {})",
            model_id,
            existing_size,
            expected_file_size
        );
        tokio::fs::remove_file(destination).await?;
        existing_size = 0;
    }

    for restart in 0..=1 {
        let mut request = client.get(url);
        if existing_size > 0 {
            request = request.header(RANGE, format!("bytes={existing_size}-"));
            log::info!(
                "Resuming model download for {} from byte {}",
                model_id,
                existing_size
            );
        } else {
            log::info!("Starting model download for {} from {}", model_id, url);
        }

        let response = request.send().await?;
        if response.status() == StatusCode::RANGE_NOT_SATISFIABLE && existing_size > 0 {
            if restart == 0 {
                log::warn!(
                    "Saved model range was rejected; restarting {} from zero",
                    model_id
                );
                tokio::fs::remove_file(destination).await?;
                existing_size = 0;
                continue;
            }
        }
        if !response.status().is_success() {
            return Err(anyhow!(
                "Model download returned HTTP {}",
                response.status()
            ));
        }

        let partial_response = response.status() == StatusCode::PARTIAL_CONTENT;
        let valid_range = partial_response
            && response
                .headers()
                .get(CONTENT_RANGE)
                .and_then(|value| value.to_str().ok())
                .and_then(parse_content_range_start)
                == Some(existing_size);
        let append = existing_size > 0 && valid_range;
        if existing_size > 0 && !append {
            log::warn!(
                "Model source ignored or changed the requested range; restarting {} locally",
                model_id
            );
            existing_size = 0;
        }

        let mut options = tokio::fs::OpenOptions::new();
        options.create(true).write(true);
        if append {
            options.append(true);
        } else {
            options.truncate(true);
        }
        let mut file = options.open(destination).await?;
        let mut downloaded = existing_size;
        let mut stream = response.bytes_stream();

        loop {
            let next = tokio::time::timeout(std::time::Duration::from_secs(60), stream.next())
                .await
                .map_err(|_| anyhow!("Model download stalled for 60 seconds"))?;
            let Some(chunk) = next else { break };
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            downloaded = downloaded.saturating_add(chunk.len() as u64);
            on_progress(downloaded)?;
        }
        file.flush().await?;
        drop(file);

        let actual_size = tokio::fs::metadata(destination).await?.len();
        if actual_size != expected_file_size {
            return Err(anyhow!(
                "Downloaded file has {actual_size} bytes; expected {expected_file_size}"
            ));
        }
        return Ok(());
    }

    Err(anyhow!("Unable to restart model download"))
}

/// Download one pinned artifact with Range resume and exact size/SHA verification.
/// Callers with their own installation layout can reuse the same robust transport
/// without adopting the model manifest format used by this module.
pub async fn download_verified_artifact<F>(
    model_id: &str,
    url: &str,
    destination: &Path,
    expected_file_size: u64,
    expected_sha256: &str,
    mut on_progress: F,
) -> Result<()>
where
    F: FnMut(u64) -> Result<()>,
{
    let mut last_error = None;
    for attempt in 1..=3 {
        match download_file_from_source(
            model_id,
            url,
            destination,
            expected_file_size,
            &mut on_progress,
        )
        .await
        {
            Ok(()) => {
                let verify_path = destination.to_path_buf();
                let expected_sha256 = expected_sha256.to_string();
                let verification = tokio::task::spawn_blocking(move || {
                    verify_file(&verify_path, expected_file_size, &expected_sha256)
                })
                .await
                .map_err(|error| anyhow!("Artifact checksum task failed: {error}"))?;
                match verification {
                    Ok(()) => return Ok(()),
                    Err(error) => {
                        let _ = tokio::fs::remove_file(destination).await;
                        log::warn!(
                            "Artifact integrity check failed on attempt {attempt}/3 for {model_id}: {error:#}"
                        );
                        last_error = Some(error);
                    }
                }
            }
            Err(error) => {
                if error.to_string().to_ascii_lowercase().contains("cancelled") {
                    return Err(error);
                }
                log::warn!(
                    "Artifact download attempt {attempt}/3 failed for {model_id}: {error:#}"
                );
                last_error = Some(error);
            }
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow!("Artifact download failed")))
}

fn parse_content_range_start(value: &str) -> Option<u64> {
    value
        .strip_prefix("bytes ")?
        .split_once('-')?
        .0
        .parse()
        .ok()
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
    format: ArchiveFormat,
) -> Result<()> {
    match format {
        ArchiveFormat::TarBz2 => extract_selected_tar_bz2_files(archive_path, destination, files),
        ArchiveFormat::Zip => extract_selected_zip_files(archive_path, destination, files),
    }
}

fn extract_selected_tar_bz2_files(
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

fn extract_selected_zip_files(
    archive_path: &Path,
    destination: &Path,
    files: &[ModelFileSpec],
) -> Result<()> {
    let expected: HashMap<&str, &ModelFileSpec> =
        files.iter().map(|file| (file.source_path, file)).collect();
    let file = File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut extracted = HashMap::<String, bool>::new();

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if !entry.is_file() {
            continue;
        }
        let path = entry.name().replace('\\', "/");
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
    let actual = file_sha256(path)?;
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

fn file_sha256(path: &Path) -> Result<String> {
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
    Ok(format!("{:x}", hasher.finalize()))
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
        let variants = file_variants(&spec);
        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0].files[0].install_path, "nested/model.onnx");
    }

    #[test]
    fn parses_resumable_content_range_start() {
        assert_eq!(parse_content_range_start("bytes 123-999/1000"), Some(123));
        assert_eq!(parse_content_range_start("bytes */1000"), None);
        assert_eq!(parse_content_range_start("items 123-999/1000"), None);
    }

    #[tokio::test]
    async fn resumes_a_partial_download_with_http_range() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt as _};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0u8; 4096];
            let read = socket.read(&mut request).await.unwrap();
            let request = String::from_utf8_lossy(&request[..read]);
            assert!(request.contains("range: bytes=3-") || request.contains("Range: bytes=3-"));
            socket
                .write_all(
                    b"HTTP/1.1 206 Partial Content\r\nContent-Length: 3\r\nContent-Range: bytes 3-5/6\r\nConnection: close\r\n\r\ndef",
                )
                .await
                .unwrap();
        });

        let directory = tempdir().unwrap();
        let destination = directory.path().join("model.part");
        tokio::fs::write(&destination, b"abc").await.unwrap();
        download_file_from_source(
            "test-model",
            &format!("http://{address}/model"),
            &destination,
            6,
            |_| Ok(()),
        )
        .await
        .unwrap();
        server.await.unwrap();
        assert_eq!(tokio::fs::read(destination).await.unwrap(), b"abcdef");
    }

    #[test]
    fn directory_recognition_requires_exact_content() {
        static FILES: &[DirectDownloadFileSpec] = &[DirectDownloadFileSpec {
            url: "https://example.invalid/model",
            install_path: "nested/model.bin",
            size: 3,
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        }];
        let spec = ModelInstallSpec {
            id: "test",
            provider: "test",
            backend: "test",
            source: ModelInstallSource::DirectFiles { files: FILES },
            download_size: 3,
            installed_size: 3,
            licenses: &[],
        };
        let directory = tempdir().unwrap();
        let nested = directory.path().join("export/nested");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("model.bin"), b"abc").unwrap();
        assert!(recognizes_directory(directory.path(), &spec).unwrap());

        std::fs::write(nested.join("model.bin"), b"abd").unwrap();
        assert!(!recognizes_directory(directory.path(), &spec).unwrap());
    }

    #[test]
    fn zip_extraction_selects_only_verified_model_files() {
        let directory = tempdir().unwrap();
        let archive_path = directory.path().join("model.zip");
        let file = File::create(&archive_path).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file("bundle/model.bin", zip::write::SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"abc").unwrap();
        archive
            .start_file(
                "bundle/ignored.txt",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        archive.write_all(b"ignored").unwrap();
        archive.finish().unwrap();

        static FILES: &[ModelFileSpec] = &[ModelFileSpec {
            source_path: "bundle/model.bin",
            install_path: "model.bin",
            size: 3,
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        }];
        let output = directory.path().join("output");
        std::fs::create_dir_all(&output).unwrap();
        extract_selected_archive_files(&archive_path, &output, FILES, ArchiveFormat::Zip).unwrap();
        assert_eq!(std::fs::read(output.join("model.bin")).unwrap(), b"abc");
        assert!(!output.join("ignored.txt").exists());
    }

    #[test]
    fn hybrid_manifest_accepts_each_complete_source_contract() {
        static ARCHIVE_FILES: &[ModelFileSpec] = &[ModelFileSpec {
            source_path: "bundle/model.bin",
            install_path: "model.bin",
            size: 3,
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        }];
        static DIRECT_FILES: &[DirectDownloadFileSpec] = &[DirectDownloadFileSpec {
            url: "https://example.invalid/model.bin",
            install_path: "model.bin",
            size: 3,
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        }];
        static ARCHIVES: &[ArchiveSourceSpec] = &[ArchiveSourceSpec {
            label: "legacy",
            url: "https://example.invalid/model.zip",
            sha256: "archive-sha",
            download_size: 5,
            format: ArchiveFormat::Zip,
            files: ARCHIVE_FILES,
        }];
        static DIRECTS: &[DirectSourceSpec] = &[DirectSourceSpec {
            label: "preferred",
            download_size: 3,
            files: DIRECT_FILES,
        }];
        let spec = ModelInstallSpec {
            id: "test",
            provider: "test",
            backend: "test",
            source: ModelInstallSource::HybridVariants {
                direct_sources: DIRECTS,
                archive_sources: ARCHIVES,
            },
            download_size: 3,
            installed_size: 3,
            licenses: &[],
        };
        let mut manifest = ModelAssetManifest {
            schema_version: 1,
            id: "test".to_string(),
            provider: "test".to_string(),
            backend: "test".to_string(),
            source: "legacy".to_string(),
            source_sha256: Some("archive-sha".to_string()),
            download_size: 5,
            installed_size: 3,
            files: vec![InstalledFileManifest {
                path: "model.bin".to_string(),
                size: 3,
                sha256: ARCHIVE_FILES[0].sha256.to_string(),
            }],
        };
        assert!(expected_files_for_manifest(&spec, &manifest).is_some());

        manifest.source_sha256 = None;
        manifest.download_size = 3;
        assert!(expected_files_for_manifest(&spec, &manifest).is_some());

        manifest.files[0].sha256 = "different".to_string();
        assert!(expected_files_for_manifest(&spec, &manifest).is_none());
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
