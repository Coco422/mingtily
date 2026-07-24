use crate::model_assets::{
    self, DirectDownloadFileSpec, LicenseFileSpec, ModelFileSpec, ModelInstallSource,
    ModelInstallSpec,
};
use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};

pub const PROVIDER_ID: &str = "sherpa-onnx";
pub const SENSEVOICE_MODEL_ID: &str = "sensevoice-small-int8";
pub const QWEN3_ASR_MODEL_ID: &str = "qwen3-asr-0.6b-int8";
pub const PARAFORMER_SMALL_MODEL_ID: &str = "paraformer-zh-small-int8";

const FUNASR_LICENSE: &str = include_str!("../../resources/licenses/FUNASR_MODEL_LICENSE.txt");
const APACHE_LICENSE: &str = include_str!("../../resources/licenses/APACHE-2.0.txt");

static FUNASR_LICENSE_FILES: &[LicenseFileSpec] = &[LicenseFileSpec {
    install_path: "licenses/FUNASR_MODEL_LICENSE.txt",
    contents: FUNASR_LICENSE,
}];

static APACHE_LICENSE_FILES: &[LicenseFileSpec] = &[LicenseFileSpec {
    install_path: "licenses/APACHE-2.0.txt",
    contents: APACHE_LICENSE,
}];

const SENSEVOICE_ARCHIVE_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2025-09-09.tar.bz2";
const SENSEVOICE_ARCHIVE_SHA256: &str =
    "7305f7905bfcf77fa0b39388a313f3da35c68d971661a65475b56fb2162c8e63";
static SENSEVOICE_FILES: &[ModelFileSpec] = &[
    ModelFileSpec {
        source_path: concat!(
            "sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2025-09-09",
            "/model.int8.onnx"
        ),
        install_path: "model.int8.onnx",
        size: 237_115_547,
        sha256: "12ca1a2ae7ecf3e0019ef2822307ee0b5cadc9196569e379b4c4026f8205276d",
    },
    ModelFileSpec {
        source_path: concat!(
            "sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2025-09-09",
            "/tokens.txt"
        ),
        install_path: "tokens.txt",
        size: 315_894,
        sha256: "f449eb28dc567533d7fa59be34e2abca8784f771850c78a47fb731a31429a1dc",
    },
];

const QWEN3_ARCHIVE_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25.tar.bz2";
const QWEN3_ARCHIVE_SHA256: &str =
    "393f8a14e2f5fb96746aaab342997a40641001fbd5bf9592a080a8329178ee96";
static QWEN3_FILES: &[ModelFileSpec] = &[
    ModelFileSpec {
        source_path: "sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/conv_frontend.onnx",
        install_path: "conv_frontend.onnx",
        size: 44_148_281,
        sha256: "d22dc4423e0940e49884e903d2ea2f7e5567c14fc1aed97e4e26d6b8f208ef9e",
    },
    ModelFileSpec {
        source_path: "sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/encoder.int8.onnx",
        install_path: "encoder.int8.onnx",
        size: 182_491_662,
        sha256: "60748d3e6744a57c9c91e1b17424a6c2990567e8adceb0783940c03ed98fa9d9",
    },
    ModelFileSpec {
        source_path: "sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/decoder.int8.onnx",
        install_path: "decoder.int8.onnx",
        size: 755_914_231,
        sha256: "4f6885be5959ae26af3089d38ee7972c5fafbeeb1cf8d5e76eab6d8b61ca5771",
    },
    ModelFileSpec {
        source_path: "sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/tokenizer/merges.txt",
        install_path: "tokenizer/merges.txt",
        size: 1_671_853,
        sha256: "8831e4f1a044471340f7c0a83d7bd71306a5b867e95fd870f74d0c5308a904d5",
    },
    ModelFileSpec {
        source_path: "sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/tokenizer/tokenizer_config.json",
        install_path: "tokenizer/tokenizer_config.json",
        size: 12_487,
        sha256: "4942d005604266809309cabc9f4e9cb89ce855d59b14681fdc0e1cc62ea26c4c",
    },
    ModelFileSpec {
        source_path: "sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/tokenizer/vocab.json",
        install_path: "tokenizer/vocab.json",
        size: 2_776_833,
        sha256: "ca10d7e9fb3ed18575dd1e277a2579c16d108e32f27439684afa0e10b1440910",
    },
];

#[cfg(test)]
const PARAFORMER_REVISION: &str = "63ddc3cd0f2810b68289a7b3876e62ef5d53d6df";
static PARAFORMER_FILES: &[DirectDownloadFileSpec] = &[
    DirectDownloadFileSpec {
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-paraformer-zh-small-2024-03-09/resolve/63ddc3cd0f2810b68289a7b3876e62ef5d53d6df/model.int8.onnx",
        install_path: "model.int8.onnx",
        size: 81_828_675,
        sha256: "3ef6c19369b912f7caf3cef8e545c5ccd1a33d9d7ec792a46668dc41c4b229ec",
    },
    DirectDownloadFileSpec {
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-paraformer-zh-small-2024-03-09/resolve/63ddc3cd0f2810b68289a7b3876e62ef5d53d6df/tokens.txt",
        install_path: "tokens.txt",
        size: 75_352,
        sha256: "4b2d964e18b9cf139b473003b6698fb2ed9a2a5ec55b93daa677b28f578897aa",
    },
];

static SENSEVOICE_SPEC: ModelInstallSpec = ModelInstallSpec {
    id: SENSEVOICE_MODEL_ID,
    provider: PROVIDER_ID,
    backend: "sense-voice",
    source: ModelInstallSource::Archive {
        url: SENSEVOICE_ARCHIVE_URL,
        sha256: SENSEVOICE_ARCHIVE_SHA256,
        files: SENSEVOICE_FILES,
    },
    download_size: 165_783_878,
    installed_size: 237_431_441,
    licenses: FUNASR_LICENSE_FILES,
};

static QWEN3_SPEC: ModelInstallSpec = ModelInstallSpec {
    id: QWEN3_ASR_MODEL_ID,
    provider: PROVIDER_ID,
    backend: "qwen3-asr",
    source: ModelInstallSource::Archive {
        url: QWEN3_ARCHIVE_URL,
        sha256: QWEN3_ARCHIVE_SHA256,
        files: QWEN3_FILES,
    },
    download_size: 878_702_423,
    installed_size: 987_015_347,
    licenses: APACHE_LICENSE_FILES,
};

static PARAFORMER_SPEC: ModelInstallSpec = ModelInstallSpec {
    id: PARAFORMER_SMALL_MODEL_ID,
    provider: PROVIDER_ID,
    backend: "paraformer-offline",
    source: ModelInstallSource::DirectFiles {
        files: PARAFORMER_FILES,
    },
    download_size: 81_904_027,
    installed_size: 81_904_027,
    licenses: FUNASR_LICENSE_FILES,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SherpaAsrBackend {
    SenseVoice,
    Qwen3Asr,
    ParaformerOffline,
}

#[derive(Debug, Clone)]
pub struct InstalledSherpaModel {
    pub id: String,
    pub backend: SherpaAsrBackend,
    pub root: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct SherpaAsrModelStatus {
    pub id: String,
    pub name: String,
    pub backend: String,
    pub status: String,
    pub download_size: u64,
    pub installed_size: u64,
    pub languages: Vec<String>,
    pub language_hint: String,
    pub streaming_mode: String,
    pub license: String,
    pub recommended: bool,
    pub beta: bool,
    pub path: String,
    pub error: Option<String>,
}

pub fn all_specs() -> [&'static ModelInstallSpec; 3] {
    [&SENSEVOICE_SPEC, &PARAFORMER_SPEC, &QWEN3_SPEC]
}

pub fn spec_for_model(model_id: &str) -> Option<&'static ModelInstallSpec> {
    all_specs().into_iter().find(|spec| spec.id == model_id)
}

pub fn model_root<R: Runtime>(app: &AppHandle<R>, model_id: &str) -> Result<PathBuf> {
    Ok(app
        .path()
        .app_data_dir()
        .context("Unable to resolve app data directory")?
        .join("models")
        .join("asr")
        .join(PROVIDER_ID)
        .join(model_id))
}

pub fn installed_model<R: Runtime>(
    app: &AppHandle<R>,
    model_id: &str,
) -> Result<Option<InstalledSherpaModel>> {
    let spec = spec_for_model(model_id).ok_or_else(|| anyhow!("Unknown Sherpa ASR model"))?;
    let root = model_root(app, model_id)?;
    if model_assets::validate_installation(&root, spec).is_err() {
        return Ok(None);
    }
    Ok(Some(InstalledSherpaModel {
        id: model_id.to_string(),
        backend: backend_for_model(model_id)?,
        root,
    }))
}

pub fn list_status<R: Runtime>(app: &AppHandle<R>) -> Result<Vec<SherpaAsrModelStatus>> {
    all_specs()
        .into_iter()
        .map(|spec| status_for_spec(app, spec))
        .collect()
}

pub async fn download_model<R: Runtime>(app: &AppHandle<R>, model_id: &str) -> Result<()> {
    let spec = spec_for_model(model_id).ok_or_else(|| anyhow!("Unknown Sherpa ASR model"))?;
    let root = model_root(app, model_id)?;
    model_assets::install_model(app, &root, spec, "sherpa-asr-model-download").await
}

pub async fn delete_model<R: Runtime>(app: &AppHandle<R>, model_id: &str) -> Result<()> {
    let _ = spec_for_model(model_id).ok_or_else(|| anyhow!("Unknown Sherpa ASR model"))?;
    model_assets::delete_model(&model_root(app, model_id)?).await
}

pub fn backend_for_model(model_id: &str) -> Result<SherpaAsrBackend> {
    match model_id {
        SENSEVOICE_MODEL_ID => Ok(SherpaAsrBackend::SenseVoice),
        QWEN3_ASR_MODEL_ID => Ok(SherpaAsrBackend::Qwen3Asr),
        PARAFORMER_SMALL_MODEL_ID => Ok(SherpaAsrBackend::ParaformerOffline),
        _ => Err(anyhow!("Unknown Sherpa ASR model: {model_id}")),
    }
}

fn status_for_spec<R: Runtime>(
    app: &AppHandle<R>,
    spec: &'static ModelInstallSpec,
) -> Result<SherpaAsrModelStatus> {
    let root = model_root(app, spec.id)?;
    let validation = model_assets::validate_installation(&root, spec);
    let status = if validation.is_ok() {
        "available"
    } else if root.exists() {
        "corrupt"
    } else {
        "missing"
    };
    let (name, languages, language_hint, streaming_mode, license, recommended, beta) = match spec.id
    {
        SENSEVOICE_MODEL_ID => (
            "SenseVoice Small int8",
            vec!["zh", "yue", "en", "ja", "ko"],
            "auto-or-fixed",
            "vad-segmented",
            "FunASR Model License 1.1",
            true,
            false,
        ),
        PARAFORMER_SMALL_MODEL_ID => (
            "Paraformer Small int8",
            vec!["zh", "en"],
            "auto-only",
            "vad-segmented",
            "FunASR Model License 1.1",
            false,
            false,
        ),
        QWEN3_ASR_MODEL_ID => (
            "Qwen3-ASR 0.6B int8",
            vec!["zh", "yue", "en", "ja", "ko", "de", "fr", "es", "pt", "ru"],
            "auto-or-fixed",
            "vad-segmented",
            "Apache-2.0",
            false,
            true,
        ),
        _ => unreachable!(),
    };

    Ok(SherpaAsrModelStatus {
        id: spec.id.to_string(),
        name: name.to_string(),
        backend: spec.backend.to_string(),
        status: status.to_string(),
        download_size: spec.download_size,
        installed_size: spec.installed_size,
        languages: languages.into_iter().map(str::to_string).collect(),
        language_hint: language_hint.to_string(),
        streaming_mode: streaming_mode.to_string(),
        license: license.to_string(),
        recommended,
        beta,
        path: root.to_string_lossy().to_string(),
        error: validation.err().map(|error| error.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_contains_unique_model_ids() {
        let specs = all_specs();
        assert_eq!(specs.len(), 3);
        assert_ne!(specs[0].id, specs[1].id);
        assert_ne!(specs[0].id, specs[2].id);
        assert_ne!(specs[1].id, specs[2].id);
    }

    #[test]
    fn paraformer_revision_is_pinned() {
        assert_eq!(PARAFORMER_REVISION.len(), 40);
        assert!(PARAFORMER_FILES
            .iter()
            .all(|file| file.url.contains(PARAFORMER_REVISION)));
    }
}
