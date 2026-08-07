use crate::model_assets::{
    self, DirectDownloadFileSpec, LicenseFileSpec, ModelInstallSource, ModelInstallSpec,
};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "provider-settings.json";
const STORE_VERSION: u64 = 1;
const STORE_VERSION_KEY: &str = "version";
const STORE_CONFIG_KEY: &str = "sherpaAsrEnhancements";

pub const HOMOPHONE_LEXICON_ID: &str = "homophone-lexicon-zh";
const HOMOPHONE_LEXICON_URL: &str =
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/hr-files/lexicon.txt";
const HOMOPHONE_LEXICON_SIZE: u64 = 1_366_297;
const HOMOPHONE_LEXICON_SHA256: &str =
    "978900e511bc481b8630cb6e4a573c12566fa092c366d5396e2c3823dec9dcb9";
const MAX_HOTWORDS: usize = 200;
const MAX_HOTWORD_CHARS: usize = 100;
const MAX_HOTWORDS_CHARS: usize = 4_000;
const MAX_RULE_FILE_SIZE: u64 = 64 * 1024 * 1024;

const APACHE_LICENSE: &str = include_str!("../../resources/licenses/APACHE-2.0.txt");

static HOMOPHONE_LEXICON_FILES: &[DirectDownloadFileSpec] = &[DirectDownloadFileSpec {
    url: HOMOPHONE_LEXICON_URL,
    install_path: "lexicon.txt",
    size: HOMOPHONE_LEXICON_SIZE,
    sha256: HOMOPHONE_LEXICON_SHA256,
}];

static HOMOPHONE_LEXICON_LICENSES: &[LicenseFileSpec] = &[LicenseFileSpec {
    install_path: "licenses/APACHE-2.0.txt",
    contents: APACHE_LICENSE,
}];

static HOMOPHONE_LEXICON_SPEC: ModelInstallSpec = ModelInstallSpec {
    id: HOMOPHONE_LEXICON_ID,
    provider: super::models::PROVIDER_ID,
    backend: "homophone-replacer-lexicon",
    source: ModelInstallSource::DirectFiles {
        files: HOMOPHONE_LEXICON_FILES,
    },
    download_size: HOMOPHONE_LEXICON_SIZE,
    installed_size: HOMOPHONE_LEXICON_SIZE,
    licenses: HOMOPHONE_LEXICON_LICENSES,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct SherpaAsrEnhancementConfig {
    pub hotwords: Vec<String>,
    pub homophone_replacer_enabled: bool,
    pub homophone_rule_fsts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HomophoneRuleStatus {
    pub id: String,
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HomophoneReplacerStatus {
    pub id: String,
    pub name: String,
    pub status: String,
    pub download_size: u64,
    pub installed_size: u64,
    pub license: String,
    pub path: String,
    pub error: Option<String>,
    pub rules: Vec<HomophoneRuleStatus>,
}

#[derive(Debug, Clone)]
pub struct RuntimeEnhancements {
    pub hotwords: Option<String>,
    pub homophone_lexicon: Option<String>,
    pub homophone_rule_fsts: Option<String>,
    signature: String,
}

impl RuntimeEnhancements {
    pub(crate) fn from_parts(
        hotwords: Option<String>,
        homophone_lexicon: Option<String>,
        homophone_rule_fsts: Option<String>,
    ) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(hotwords.as_deref().unwrap_or_default().as_bytes());
        hasher.update([0]);
        hasher.update(homophone_lexicon.as_deref().unwrap_or_default().as_bytes());
        hasher.update([0]);
        hasher.update(
            homophone_rule_fsts
                .as_deref()
                .unwrap_or_default()
                .as_bytes(),
        );
        let signature = format!("{:x}", hasher.finalize());

        Self {
            hotwords,
            homophone_lexicon,
            homophone_rule_fsts,
            signature,
        }
    }

    pub fn cache_signature(&self) -> &str {
        &self.signature
    }
}

impl Default for RuntimeEnhancements {
    fn default() -> Self {
        Self::from_parts(None, None, None)
    }
}

pub fn load_config<R: Runtime>(app: &AppHandle<R>) -> Result<SherpaAsrEnhancementConfig> {
    let store = app
        .store(STORE_FILE)
        .map_err(|error| anyhow!("Failed to access provider settings: {error}"))?;
    let Some(value) = store.get(STORE_CONFIG_KEY) else {
        return Ok(SherpaAsrEnhancementConfig::default());
    };
    let config = serde_json::from_value::<SherpaAsrEnhancementConfig>(value.clone())
        .map_err(|error| anyhow!("Failed to read Sherpa ASR enhancement settings: {error}"))?;
    normalize_config(config)
}

pub fn save_config<R: Runtime>(
    app: &AppHandle<R>,
    config: SherpaAsrEnhancementConfig,
) -> Result<SherpaAsrEnhancementConfig> {
    let config = normalize_config(config)?;
    if config.homophone_replacer_enabled {
        if installed_lexicon_path(app)?.is_none() {
            return Err(anyhow!(
                "The homophone lexicon is missing or damaged. Download or repair it in Models."
            ));
        }
        if config.homophone_rule_fsts.is_empty() {
            return Err(anyhow!(
                "Select at least one imported homophone replacement rule"
            ));
        }
        let available = list_rules(app)?
            .into_iter()
            .map(|rule| rule.id)
            .collect::<HashSet<_>>();
        if let Some(missing) = config
            .homophone_rule_fsts
            .iter()
            .find(|rule_id| !available.contains(*rule_id))
        {
            return Err(anyhow!(
                "Homophone replacement rule '{missing}' is missing or damaged"
            ));
        }
    }

    let store = app
        .store(STORE_FILE)
        .map_err(|error| anyhow!("Failed to access provider settings: {error}"))?;
    store.set(STORE_VERSION_KEY, serde_json::json!(STORE_VERSION));
    store.set(
        STORE_CONFIG_KEY,
        serde_json::to_value(&config)
            .map_err(|error| anyhow!("Failed to serialize Sherpa ASR enhancements: {error}"))?,
    );
    store
        .save()
        .map_err(|error| anyhow!("Failed to save provider settings: {error}"))?;
    Ok(config)
}

pub fn resolve_runtime<R: Runtime>(app: &AppHandle<R>) -> Result<RuntimeEnhancements> {
    let config = load_config(app)?;
    let hotwords = (!config.hotwords.is_empty()).then(|| config.hotwords.join(","));

    let (homophone_lexicon, homophone_rule_fsts) = if config.homophone_replacer_enabled {
        let lexicon = installed_lexicon_path(app)?;
        let rules_by_id = list_rules(app)?
            .into_iter()
            .map(|rule| (rule.id.clone(), rule))
            .collect::<std::collections::HashMap<_, _>>();
        let selected_paths = config
            .homophone_rule_fsts
            .iter()
            .filter_map(|rule_id| rules_by_id.get(rule_id))
            .map(|rule| rules_root(app).map(|root| root.join(format!("{}.fst", rule.id))))
            .collect::<Result<Vec<_>>>()?;

        let lexicon = lexicon.and_then(|path| path.to_str().map(str::to_string));
        let selected_paths = selected_paths
            .iter()
            .map(|path| path.to_str())
            .collect::<Option<Vec<_>>>();

        match (lexicon, selected_paths) {
            (Some(lexicon), Some(selected_paths)) if !selected_paths.is_empty() => {
                (Some(lexicon), Some(selected_paths.join(",")))
            }
            _ => {
                log::warn!(
                    "Sherpa homophone replacement is enabled but its lexicon or rules are unavailable; continuing without replacement"
                );
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    Ok(RuntimeEnhancements::from_parts(
        hotwords,
        homophone_lexicon,
        homophone_rule_fsts,
    ))
}

pub fn status<R: Runtime>(app: &AppHandle<R>) -> Result<HomophoneReplacerStatus> {
    let root = lexicon_root(app)?;
    let validation = model_assets::validate_installation(&root, &HOMOPHONE_LEXICON_SPEC);
    let status = if validation.is_ok() {
        "available"
    } else if root.exists() {
        "corrupt"
    } else {
        "missing"
    };

    Ok(HomophoneReplacerStatus {
        id: HOMOPHONE_LEXICON_ID.to_string(),
        name: "Chinese homophone lexicon".to_string(),
        status: status.to_string(),
        download_size: HOMOPHONE_LEXICON_SIZE,
        installed_size: HOMOPHONE_LEXICON_SIZE,
        license: "Apache-2.0".to_string(),
        path: root.to_string_lossy().to_string(),
        error: validation.err().map(|error| error.to_string()),
        rules: list_rules(app)?,
    })
}

pub async fn download_lexicon<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    model_assets::install_model(
        app,
        &lexicon_root(app)?,
        &HOMOPHONE_LEXICON_SPEC,
        "sherpa-homophone-lexicon-download",
    )
    .await
}

pub async fn delete_lexicon<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    model_assets::delete_model(&lexicon_root(app)?).await?;
    let mut config = load_config(app)?;
    config.homophone_replacer_enabled = false;
    save_config(app, config)?;
    Ok(())
}

pub fn import_rule_files<R: Runtime>(
    app: &AppHandle<R>,
    source_paths: Vec<PathBuf>,
) -> Result<Vec<HomophoneRuleStatus>> {
    if source_paths.is_empty() {
        return list_rules(app);
    }

    let root = rules_root(app)?;
    std::fs::create_dir_all(&root)?;
    let mut metadata = load_rule_metadata(app)?;

    for source in source_paths {
        validate_rule_source(&source)?;
        let size = source.metadata()?.len();
        let id = sha256_file(&source)?;
        let destination = root.join(format!("{id}.fst"));
        if !destination.is_file() {
            let staging = root.join(format!(".{id}.fst.part"));
            std::fs::copy(&source, &staging)
                .with_context(|| format!("Unable to copy homophone rule {}", source.display()))?;
            let copied_hash = sha256_file(&staging)?;
            if copied_hash != id {
                let _ = std::fs::remove_file(&staging);
                return Err(anyhow!(
                    "Homophone rule changed while it was being imported: {}",
                    source.display()
                ));
            }
            std::fs::rename(&staging, &destination)?;
        }

        let name = source
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("homophone-rule.fst")
            .to_string();
        if let Some(existing) = metadata.iter_mut().find(|rule| rule.id == id) {
            existing.name = name;
            existing.size = size;
        } else {
            metadata.push(HomophoneRuleStatus { id, name, size });
        }
    }

    metadata.sort_by(|left, right| left.name.cmp(&right.name));
    save_rule_metadata(app, &metadata)?;
    list_rules(app)
}

pub fn delete_rule<R: Runtime>(
    app: &AppHandle<R>,
    rule_id: &str,
) -> Result<Vec<HomophoneRuleStatus>> {
    validate_rule_id(rule_id)?;
    let path = rules_root(app)?.join(format!("{rule_id}.fst"));
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    let mut metadata = load_rule_metadata(app)?;
    metadata.retain(|rule| rule.id != rule_id);
    save_rule_metadata(app, &metadata)?;

    let mut config = load_config(app)?;
    config.homophone_rule_fsts.retain(|id| id != rule_id);
    if config.homophone_rule_fsts.is_empty() {
        config.homophone_replacer_enabled = false;
    }
    let _ = save_config(app, config)?;
    list_rules(app)
}

fn normalize_config(mut config: SherpaAsrEnhancementConfig) -> Result<SherpaAsrEnhancementConfig> {
    let mut hotwords = Vec::new();
    let mut seen_hotwords = HashSet::new();
    for value in &config.hotwords {
        for term in value.split([',', '，', '\n', '\r']) {
            let term = term.trim();
            if term.is_empty() || !seen_hotwords.insert(term.to_string()) {
                continue;
            }
            if term.chars().count() > MAX_HOTWORD_CHARS {
                return Err(anyhow!(
                    "A hotword exceeds the {MAX_HOTWORD_CHARS}-character limit"
                ));
            }
            hotwords.push(term.to_string());
        }
    }
    if hotwords.len() > MAX_HOTWORDS {
        return Err(anyhow!("At most {MAX_HOTWORDS} hotwords are supported"));
    }
    if hotwords
        .iter()
        .map(|term| term.chars().count())
        .sum::<usize>()
        > MAX_HOTWORDS_CHARS
    {
        return Err(anyhow!(
            "Hotwords exceed the combined {MAX_HOTWORDS_CHARS}-character limit"
        ));
    }

    let mut seen_rules = HashSet::new();
    config
        .homophone_rule_fsts
        .retain(|rule_id| validate_rule_id(rule_id).is_ok() && seen_rules.insert(rule_id.clone()));
    config.hotwords = hotwords;
    Ok(config)
}

fn enhancements_root<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf> {
    Ok(app
        .path()
        .app_data_dir()
        .context("Unable to resolve app data directory")?
        .join("models")
        .join("asr")
        .join(super::models::PROVIDER_ID)
        .join("enhancements"))
}

fn lexicon_root<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf> {
    Ok(enhancements_root(app)?.join(HOMOPHONE_LEXICON_ID))
}

fn rules_root<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf> {
    Ok(enhancements_root(app)?.join("homophone-rules"))
}

fn installed_lexicon_path<R: Runtime>(app: &AppHandle<R>) -> Result<Option<PathBuf>> {
    let root = lexicon_root(app)?;
    if model_assets::validate_installation(&root, &HOMOPHONE_LEXICON_SPEC).is_err() {
        return Ok(None);
    }
    Ok(Some(root.join("lexicon.txt")))
}

fn metadata_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf> {
    Ok(rules_root(app)?.join("rules.json"))
}

fn load_rule_metadata<R: Runtime>(app: &AppHandle<R>) -> Result<Vec<HomophoneRuleStatus>> {
    let path = metadata_path(app)?;
    if !path.is_file() {
        return Ok(Vec::new());
    }
    serde_json::from_slice(&std::fs::read(&path)?)
        .with_context(|| format!("Unable to read {}", path.display()))
}

fn save_rule_metadata<R: Runtime>(app: &AppHandle<R>, rules: &[HomophoneRuleStatus]) -> Result<()> {
    let path = metadata_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let staging = path.with_extension("json.part");
    let backup = path.with_extension("json.backup");
    std::fs::write(&staging, serde_json::to_vec_pretty(rules)?)?;
    if backup.exists() {
        std::fs::remove_file(&backup)?;
    }
    if path.exists() {
        std::fs::rename(&path, &backup)?;
    }
    if let Err(error) = std::fs::rename(&staging, &path) {
        if backup.exists() {
            let _ = std::fs::rename(&backup, &path);
        }
        return Err(error.into());
    }
    if backup.exists() {
        std::fs::remove_file(backup)?;
    }
    Ok(())
}

fn list_rules<R: Runtime>(app: &AppHandle<R>) -> Result<Vec<HomophoneRuleStatus>> {
    let root = rules_root(app)?;
    let mut rules = load_rule_metadata(app)?;
    rules.retain(|rule| {
        if validate_rule_id(&rule.id).is_err() {
            return false;
        }
        let path = root.join(format!("{}.fst", rule.id));
        path.metadata()
            .is_ok_and(|metadata| metadata.is_file() && metadata.len() == rule.size)
            && sha256_file(&path).is_ok_and(|sha256| sha256 == rule.id)
    });
    rules.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(rules)
}

fn validate_rule_source(path: &Path) -> Result<()> {
    let metadata = path
        .metadata()
        .with_context(|| format!("Unable to inspect {}", path.display()))?;
    if !metadata.is_file() {
        return Err(anyhow!("Homophone replacement rules must be regular files"));
    }
    if metadata.len() == 0 || metadata.len() > MAX_RULE_FILE_SIZE {
        return Err(anyhow!(
            "Homophone replacement rules must be between 1 byte and 64 MiB"
        ));
    }
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("fst"))
    {
        return Err(anyhow!(
            "Only pre-generated .fst rule files can be imported"
        ));
    }
    Ok(())
}

fn validate_rule_id(rule_id: &str) -> Result<()> {
    if rule_id.len() != 64 || !rule_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(anyhow!("Invalid homophone rule identifier"));
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 1024 * 1024];
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
    fn hotwords_are_trimmed_split_and_deduplicated() {
        let config = normalize_config(SherpaAsrEnhancementConfig {
            hotwords: vec![" Mingtily，SenseVoice\nMingtily ".to_string()],
            ..Default::default()
        })
        .unwrap();
        assert_eq!(config.hotwords, vec!["Mingtily", "SenseVoice"]);
    }

    #[test]
    fn non_fst_rule_files_are_rejected() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("rules.txt");
        std::fs::write(&path, b"not an fst").unwrap();
        assert!(validate_rule_source(&path).is_err());
    }

    #[test]
    fn empty_runtime_has_stable_signature() {
        let runtime = RuntimeEnhancements::default();
        assert_eq!(runtime.cache_signature().len(), 64);
        assert_eq!(
            runtime.cache_signature(),
            RuntimeEnhancements::default().cache_signature()
        );
    }

    #[test]
    fn pinned_lexicon_metadata_is_complete() {
        assert_eq!(HOMOPHONE_LEXICON_SHA256.len(), 64);
        assert_eq!(HOMOPHONE_LEXICON_SPEC.download_size, HOMOPHONE_LEXICON_SIZE);
    }
}
