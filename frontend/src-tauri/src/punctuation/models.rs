use crate::model_assets::{
    self, ArchiveFormat, ArchiveSourceSpec, DirectDownloadFileSpec, DirectSourceSpec,
    LicenseFileSpec, ModelFileSpec, ModelInstallSource, ModelInstallSpec,
};
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};

pub const MODEL_ID: &str = "punctuation-zh-en-int8";
const PROVIDER_ID: &str = "sherpa-onnx";
const BACKEND_ID: &str = "offline-punctuation";
const ARCHIVE_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/punctuation-models/sherpa-onnx-punct-ct-transformer-zh-en-vocab272727-2024-04-12-int8.tar.bz2";
const ARCHIVE_SHA256: &str = "c0d5aa5f8eeb686032345e180bedf39319dc2e0556781c6264bcadba8328a6e1";
const MODEL_SOURCE_PATH: &str = concat!(
    "sherpa-onnx-punct-ct-transformer-zh-en-vocab272727-2024-04-12-int8",
    "/model.int8.onnx"
);
const MODEL_SIZE: u64 = 75_519_198;
const MODEL_SHA256: &str = "65a3fb9f5ad7bfb96bf69e0dc4481df97f6ee60513c1d94ce981ba6effd524b1";
const DOWNLOAD_SIZE: u64 = 64_717_756;
#[cfg(test)]
const MODELSCOPE_REVISION: &str = "8177426a1240345bd35b21616475ddcf425d5288";

const APACHE_LICENSE: &str = include_str!("../../resources/licenses/APACHE-2.0.txt");

static MODEL_FILES: &[ModelFileSpec] = &[ModelFileSpec {
    source_path: MODEL_SOURCE_PATH,
    install_path: "model.int8.onnx",
    size: MODEL_SIZE,
    sha256: MODEL_SHA256,
}];
static MODELSCOPE_FILES: &[DirectDownloadFileSpec] = &[DirectDownloadFileSpec {
    url: "https://www.modelscope.cn/api/v1/models/ranger810/sherpa-onnx-punct-ct-transformer-zh-en-vocab272727-2024-04-12-int8/repo?Revision=8177426a1240345bd35b21616475ddcf425d5288&FilePath=model.int8.onnx",
    install_path: "model.int8.onnx",
    size: MODEL_SIZE,
    sha256: MODEL_SHA256,
}];
static DIRECT_SOURCES: &[DirectSourceSpec] = &[DirectSourceSpec {
    label: "ModelScope China",
    download_size: MODEL_SIZE,
    files: MODELSCOPE_FILES,
}];
static ARCHIVE_SOURCES: &[ArchiveSourceSpec] = &[ArchiveSourceSpec {
    label: "sherpa-onnx GitHub Release",
    url: ARCHIVE_URL,
    sha256: ARCHIVE_SHA256,
    download_size: DOWNLOAD_SIZE,
    format: ArchiveFormat::TarBz2,
    files: MODEL_FILES,
}];

static LICENSE_FILES: &[LicenseFileSpec] = &[LicenseFileSpec {
    install_path: "licenses/APACHE-2.0.txt",
    contents: APACHE_LICENSE,
}];

static MODEL_SPEC: ModelInstallSpec = ModelInstallSpec {
    id: MODEL_ID,
    provider: PROVIDER_ID,
    backend: BACKEND_ID,
    source: ModelInstallSource::HybridVariants {
        direct_sources: DIRECT_SOURCES,
        archive_sources: ARCHIVE_SOURCES,
    },
    download_size: MODEL_SIZE,
    installed_size: MODEL_SIZE,
    licenses: LICENSE_FILES,
};

#[derive(Debug, Clone, Serialize)]
pub struct PunctuationModelStatus {
    pub id: String,
    pub name: String,
    pub status: String,
    pub download_size: u64,
    pub installed_size: u64,
    pub languages: Vec<String>,
    pub license: String,
    pub path: String,
    pub error: Option<String>,
}

pub fn model_root<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf> {
    Ok(app
        .path()
        .app_data_dir()
        .context("Unable to resolve app data directory")?
        .join("models")
        .join("punctuation")
        .join(PROVIDER_ID)
        .join(MODEL_ID))
}

pub fn installed_model_path<R: Runtime>(app: &AppHandle<R>) -> Result<Option<PathBuf>> {
    let root = model_root(app)?;
    if model_assets::validate_installation(&root, &MODEL_SPEC).is_err() {
        return Ok(None);
    }
    Ok(Some(root.join("model.int8.onnx")))
}

pub fn status<R: Runtime>(app: &AppHandle<R>) -> Result<PunctuationModelStatus> {
    let root = model_root(app)?;
    let validation = model_assets::validate_installation(&root, &MODEL_SPEC);
    let status = if validation.is_ok() {
        "available"
    } else if root.exists() {
        "corrupt"
    } else {
        "missing"
    };

    Ok(PunctuationModelStatus {
        id: MODEL_ID.to_string(),
        name: "Chinese and English punctuation int8".to_string(),
        status: status.to_string(),
        download_size: MODEL_SPEC.download_size,
        installed_size: MODEL_SIZE,
        languages: vec!["zh".to_string(), "en".to_string()],
        license: "Apache-2.0".to_string(),
        path: root.to_string_lossy().to_string(),
        error: validation.err().map(|error| error.to_string()),
    })
}

pub async fn download_model<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    model_assets::install_model(
        app,
        &model_root(app)?,
        &MODEL_SPEC,
        "punctuation-model-download",
    )
    .await
}

pub async fn delete_model<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    model_assets::delete_model(&model_root(app)?).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_artifact_metadata_is_complete() {
        assert_eq!(MODEL_FILES.len(), 1);
        assert_eq!(ARCHIVE_SHA256.len(), 64);
        assert_eq!(MODEL_SHA256.len(), 64);
        assert_eq!(MODEL_SPEC.download_size, MODEL_SIZE);
        assert_eq!(MODEL_SPEC.installed_size, MODEL_SIZE);
        assert_eq!(MODELSCOPE_REVISION.len(), 40);
        assert_eq!(DIRECT_SOURCES[0].label, "ModelScope China");
        assert!(DIRECT_SOURCES[0].files[0].url.contains(MODELSCOPE_REVISION));
        assert_eq!(ARCHIVE_SOURCES[0].sha256, ARCHIVE_SHA256);
    }
}
