use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "pipeline-settings.json";
const CONFIG_KEY: &str = "pipelineConfig";
const BETA_KEY: &str = "betaFeatures";
const BETA_MIGRATION_KEY: &str = "betaSelectionMigrationV1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PipelinePreset {
    Fast,
    Balanced,
    Quality,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LiveMode {
    Off,
    VadSegmented,
    ContinuousPreview,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PostMeetingPolicy {
    Off,
    Manual,
    Auto,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SpeakerRefinementPolicy {
    Off,
    Manual,
    BackgroundAuto,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ResourceMode {
    Eco,
    Balanced,
    Fast,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LiveConfig {
    pub mode: LiveMode,
    pub streaming_provider: Option<String>,
    pub streaming_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FinalizedConfig {
    pub provider: String,
    pub model: String,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PostMeetingAsrConfig {
    pub policy: PostMeetingPolicy,
    pub provider: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PipelineSpeakerConfig {
    pub live_enabled: bool,
    pub refinement: SpeakerRefinementPolicy,
    pub speaker_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnhancementConfig {
    pub punctuation: String,
    pub terminology: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PipelineResourceConfig {
    pub mode: ResourceMode,
    #[serde(rename = "memoryLimitMiB")]
    pub memory_limit_mib: Option<u64>,
    pub run_automatic_jobs_on_battery: bool,
    pub pause_automatic_jobs_during_recording: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PipelineConfig {
    pub version: u32,
    pub preset: PipelinePreset,
    pub live: LiveConfig,
    pub finalized: FinalizedConfig,
    pub post_meeting_asr: PostMeetingAsrConfig,
    pub speaker: PipelineSpeakerConfig,
    pub enhancements: EnhancementConfig,
    pub resources: PipelineResourceConfig,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            version: 1,
            preset: PipelinePreset::Balanced,
            live: LiveConfig {
                mode: LiveMode::VadSegmented,
                streaming_provider: None,
                streaming_model: None,
            },
            finalized: FinalizedConfig {
                provider: crate::sherpa_asr::PROVIDER_ID.into(),
                model: crate::sherpa_asr::models::SENSEVOICE_MODEL_ID.into(),
                language: "auto".into(),
            },
            post_meeting_asr: PostMeetingAsrConfig {
                policy: PostMeetingPolicy::Off,
                provider: None,
                model: None,
            },
            speaker: PipelineSpeakerConfig {
                live_enabled: true,
                refinement: SpeakerRefinementPolicy::BackgroundAuto,
                speaker_count: None,
            },
            enhancements: EnhancementConfig {
                punctuation: "auto".into(),
                terminology: "auto".into(),
            },
            resources: PipelineResourceConfig {
                mode: ResourceMode::Balanced,
                memory_limit_mib: None,
                run_automatic_jobs_on_battery: false,
                pause_automatic_jobs_during_recording: true,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BetaFeatures {
    pub import_and_retranscribe: bool,
    pub custom_transcription_pipelines: bool,
    pub experimental_asr_models: bool,
}

impl Default for BetaFeatures {
    fn default() -> Self {
        Self {
            import_and_retranscribe: true,
            custom_transcription_pipelines: true,
            experimental_asr_models: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelCapabilities {
    pub provider: String,
    pub model: String,
    pub input_mode: String,
    pub outputs: Vec<String>,
    pub languages: Vec<String>,
    pub supports_hotwords: bool,
    pub built_in_punctuation: bool,
    pub recommended_threads: usize,
    #[serde(rename = "fixedMemoryMiB")]
    pub fixed_memory_mib: u64,
    #[serde(rename = "workerMemoryMiB")]
    pub worker_memory_mib: u64,
    pub max_parallelism: usize,
    pub max_audio_seconds: Option<u64>,
    pub supported_platforms: Vec<String>,
    pub beta_gate: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedPipeline {
    /// The user-authored configuration kept for editing and Gate restoration.
    pub config: PipelineConfig,
    /// The normalized configuration used by recording and background jobs.
    #[serde(default)]
    pub effective_config: Option<PipelineConfig>,
    pub live_capabilities: Option<ModelCapabilities>,
    pub finalized_capabilities: ModelCapabilities,
    pub post_meeting_capabilities: Option<ModelCapabilities>,
    #[serde(default)]
    pub speaker_capabilities: Option<ModelCapabilities>,
    pub punctuation_enabled: bool,
    pub speaker_refinement_enabled: bool,
    #[serde(rename = "estimatedMemoryMiB")]
    pub estimated_memory_mib: u64,
    pub worker_count: usize,
    #[serde(default = "default_thread_count")]
    pub thread_count: usize,
    pub decisions: Vec<String>,
}

fn default_thread_count() -> usize {
    1
}

impl ResolvedPipeline {
    pub fn runtime_config(&self) -> &PipelineConfig {
        self.effective_config.as_ref().unwrap_or(&self.config)
    }
}

pub(crate) fn is_experimental_model(model: &str) -> bool {
    matches!(
        model,
        crate::sherpa_asr::models::PARAFORMER_ONLINE_MODEL_ID
            | crate::sherpa_asr::models::QWEN3_ASR_MODEL_ID
            | crate::sherpa_asr::models::FUNASR_NANO_MODEL_ID
    )
}

fn preserve_selected_beta_gates(config: &PipelineConfig, beta: &mut BetaFeatures) {
    if config.preset != PipelinePreset::Balanced {
        beta.custom_transcription_pipelines = true;
    }
    if is_experimental_model(&config.finalized.model)
        || config
            .live
            .streaming_model
            .as_deref()
            .is_some_and(is_experimental_model)
        || config
            .post_meeting_asr
            .model
            .as_deref()
            .is_some_and(is_experimental_model)
    {
        beta.experimental_asr_models = true;
    }
}

pub fn capabilities(provider: &str, model: &str) -> ModelCapabilities {
    let (input_mode, languages, hotwords, punctuation, fixed, worker, max_parallelism, beta) =
        match (provider, model) {
            (crate::sherpa_asr::PROVIDER_ID, crate::sherpa_asr::models::SENSEVOICE_MODEL_ID) => (
                "vad-segmented",
                vec!["zh", "yue", "en", "ja", "ko"],
                false,
                false,
                384,
                96,
                1,
                None,
            ),
            (
                crate::sherpa_asr::PROVIDER_ID,
                crate::sherpa_asr::models::PARAFORMER_ONLINE_MODEL_ID,
            ) => (
                "continuous",
                vec!["zh", "en"],
                false,
                false,
                256,
                64,
                1,
                Some("experimentalAsrModels"),
            ),
            (crate::sherpa_asr::PROVIDER_ID, crate::sherpa_asr::models::QWEN3_ASR_MODEL_ID) => (
                "vad-segmented",
                vec!["zh", "yue", "en", "ja", "ko", "de", "fr", "es", "pt", "ru"],
                true,
                true,
                1200,
                256,
                1,
                Some("experimentalAsrModels"),
            ),
            (crate::sherpa_asr::PROVIDER_ID, crate::sherpa_asr::models::FUNASR_NANO_MODEL_ID) => (
                "vad-segmented",
                vec![
                    "zh", "yue", "en", "ja", "ko", "vi", "id", "th", "ms", "tl", "ar", "hi",
                ],
                true,
                true,
                1400,
                256,
                1,
                Some("experimentalAsrModels"),
            ),
            (crate::sherpa_asr::PROVIDER_ID, _) => (
                "vad-segmented",
                vec!["zh", "en"],
                false,
                false,
                320,
                96,
                1,
                None,
            ),
            ("parakeet", _) => ("vad-segmented", vec!["en"], false, true, 900, 192, 1, None),
            ("localWhisper" | "whisper", _) => {
                ("vad-segmented", Vec::new(), false, true, 768, 256, 1, None)
            }
            _ => ("vad-segmented", Vec::new(), false, false, 512, 192, 1, None),
        };
    let (fixed_memory_mib, worker_memory_mib) = if matches!(provider, "localWhisper" | "whisper") {
        crate::whisper_engine::registered_whisper_memory_mib(model).unwrap_or((fixed, worker))
    } else {
        (fixed, worker)
    };
    ModelCapabilities {
        provider: provider.into(),
        model: model.into(),
        input_mode: input_mode.into(),
        outputs: vec!["text".into()],
        languages: languages.into_iter().map(str::to_string).collect(),
        supports_hotwords: hotwords,
        built_in_punctuation: punctuation,
        recommended_threads: match provider {
            "localWhisper" | "whisper" => 4,
            "parakeet" => 2,
            _ => 2,
        },
        fixed_memory_mib,
        worker_memory_mib,
        max_parallelism,
        max_audio_seconds: None,
        supported_platforms: vec!["macos".into(), "windows".into(), "linux".into()],
        beta_gate: beta.map(str::to_string),
    }
}

fn should_run_speaker_refinement(config: &PipelineConfig, active_asr: &ModelCapabilities) -> bool {
    if config.speaker.refinement == SpeakerRefinementPolicy::Off {
        return false;
    }
    let asr_outputs_speakers = active_asr.outputs.iter().any(|output| output == "speakers");
    !asr_outputs_speakers || config.preset == PipelinePreset::Custom
}

fn normalize_preset(mut config: PipelineConfig) -> PipelineConfig {
    match config.preset {
        PipelinePreset::Fast => {
            config.live.mode = LiveMode::VadSegmented;
            config.post_meeting_asr.policy = PostMeetingPolicy::Off;
            config.speaker.refinement = SpeakerRefinementPolicy::Off;
            config.resources.mode = ResourceMode::Eco;
        }
        PipelinePreset::Balanced => {
            if config.live.mode == LiveMode::Off {
                config.live.mode = LiveMode::VadSegmented;
            }
            config.post_meeting_asr.policy = PostMeetingPolicy::Off;
            config.speaker.refinement = SpeakerRefinementPolicy::BackgroundAuto;
            config.resources.mode = ResourceMode::Balanced;
        }
        PipelinePreset::Quality => {
            if config.live.streaming_model.is_some() {
                config.live.mode = LiveMode::ContinuousPreview;
            }
            config.post_meeting_asr.policy = PostMeetingPolicy::Auto;
            if config.post_meeting_asr.provider.is_none() {
                config.post_meeting_asr.provider = Some(config.finalized.provider.clone());
            }
            if config.post_meeting_asr.model.is_none() {
                config.post_meeting_asr.model = Some(config.finalized.model.clone());
            }
            config.speaker.refinement = SpeakerRefinementPolicy::BackgroundAuto;
            config.resources.mode = ResourceMode::Fast;
        }
        PipelinePreset::Custom => {}
    }
    config.version = 1;
    // Recording always wins over automatic background work. This is a fixed
    // safety invariant rather than a user-tunable resource preference.
    config.resources.pause_automatic_jobs_during_recording = true;
    config
}

pub fn resolve(config: PipelineConfig, beta: &BetaFeatures) -> Result<ResolvedPipeline> {
    let requested_config = config;
    let mut config = normalize_preset(requested_config.clone());
    if config
        .speaker
        .speaker_count
        .is_some_and(|count| !(1..=10).contains(&count))
    {
        return Err(anyhow!("Speaker count must be between 1 and 10"));
    }
    if !matches!(config.enhancements.punctuation.as_str(), "auto" | "off")
        || !matches!(config.enhancements.terminology.as_str(), "auto" | "off")
    {
        return Err(anyhow!("Enhancement policies must be 'auto' or 'off'"));
    }
    let mut decisions = Vec::new();
    let mut used_stable_fallback = false;
    if is_experimental_model(&config.finalized.model) && !beta.experimental_asr_models {
        used_stable_fallback = true;
        let selected_provider = config.finalized.provider.clone();
        let selected_model = config.finalized.model.clone();
        config.finalized.provider = crate::sherpa_asr::PROVIDER_ID.into();
        config.finalized.model = crate::sherpa_asr::models::SENSEVOICE_MODEL_ID.into();
        if config.post_meeting_asr.provider.as_deref() == Some(selected_provider.as_str())
            && config.post_meeting_asr.model.as_deref() == Some(selected_model.as_str())
        {
            config.post_meeting_asr.provider = Some(crate::sherpa_asr::PROVIDER_ID.into());
            config.post_meeting_asr.model =
                Some(crate::sherpa_asr::models::SENSEVOICE_MODEL_ID.into());
        }
    }
    let finalized = capabilities(&config.finalized.provider, &config.finalized.model);
    if !finalized
        .supported_platforms
        .iter()
        .any(|platform| platform == std::env::consts::OS)
    {
        return Err(anyhow!(
            "The finalized model is not supported on this platform"
        ));
    }
    if finalized.input_mode == "continuous" {
        return Err(anyhow!(
            "A continuous model cannot be used for finalized VAD transcription"
        ));
    }
    if config.live.mode == LiveMode::ContinuousPreview && !beta.experimental_asr_models {
        config.live.mode = LiveMode::VadSegmented;
        decisions.push("streamingStableFallback".into());
    }
    if config.live.mode == LiveMode::ContinuousPreview {
        let streaming_model = config
            .live
            .streaming_model
            .as_deref()
            .ok_or_else(|| anyhow!("A streaming model is required"))?;
        let streaming_provider = config
            .live
            .streaming_provider
            .as_deref()
            .unwrap_or(crate::sherpa_asr::PROVIDER_ID);
        if streaming_provider == config.finalized.provider
            && streaming_model == config.finalized.model
        {
            return Err(anyhow!("Streaming and finalized models must be different"));
        }
        if capabilities(streaming_provider, streaming_model).input_mode != "continuous" {
            return Err(anyhow!("The selected preview model is not continuous"));
        }
    }
    let finalized = capabilities(&config.finalized.provider, &config.finalized.model);
    if used_stable_fallback {
        decisions.push("stableFallback".into());
    }
    if config.finalized.language != "auto"
        && !finalized.languages.is_empty()
        && !finalized
            .languages
            .iter()
            .any(|language| language == &config.finalized.language)
    {
        return Err(anyhow!(
            "Language '{}' is not supported by {}/{}",
            config.finalized.language,
            config.finalized.provider,
            config.finalized.model
        ));
    }
    let punctuation_language_supported = matches!(
        config.finalized.language.as_str(),
        "auto" | "zh" | "yue" | "en"
    );
    let punctuation_enabled = config.enhancements.punctuation == "auto"
        && !finalized.built_in_punctuation
        && punctuation_language_supported;
    if !punctuation_enabled {
        decisions.push("punctuationDisabled".into());
    }
    let post_selection = (
        config.post_meeting_asr.provider.clone(),
        config.post_meeting_asr.model.clone(),
    );
    let post = match config.post_meeting_asr.policy {
        PostMeetingPolicy::Off => None,
        _ => match post_selection {
            (Some(provider), Some(model)) => {
                let mut capability = capabilities(&provider, &model);
                if capability.input_mode == "continuous" {
                    return Err(anyhow!(
                        "A continuous model can only be used for the live preview path"
                    ));
                }
                if capability.beta_gate.as_deref() == Some("experimentalAsrModels")
                    && !beta.experimental_asr_models
                {
                    decisions.push("postMeetingStableFallback".into());
                    config.post_meeting_asr.provider = Some(crate::sherpa_asr::PROVIDER_ID.into());
                    config.post_meeting_asr.model =
                        Some(crate::sherpa_asr::models::SENSEVOICE_MODEL_ID.into());
                    capability = capabilities(
                        crate::sherpa_asr::PROVIDER_ID,
                        crate::sherpa_asr::models::SENSEVOICE_MODEL_ID,
                    );
                }
                if !capability
                    .supported_platforms
                    .iter()
                    .any(|platform| platform == std::env::consts::OS)
                {
                    return Err(anyhow!(
                        "The post-meeting model is not supported on this platform"
                    ));
                }
                if config.finalized.language != "auto"
                    && !capability.languages.is_empty()
                    && !capability
                        .languages
                        .iter()
                        .any(|language| language == &config.finalized.language)
                {
                    return Err(anyhow!(
                        "Language '{}' is not supported by the post-meeting model {}/{}",
                        config.finalized.language,
                        provider,
                        model
                    ));
                }
                Some(capability)
            }
            _ => return Err(anyhow!("Post-meeting ASR requires a provider and model")),
        },
    };
    let active_asr = post.as_ref().unwrap_or(&finalized);
    let speaker_refinement_enabled = should_run_speaker_refinement(&config, active_asr);
    if !speaker_refinement_enabled
        && config.speaker.refinement != SpeakerRefinementPolicy::Off
        && active_asr.outputs.iter().any(|output| output == "speakers")
    {
        decisions.push("nativeSpeakerOutput".into());
    }
    let speaker = speaker_refinement_enabled.then(|| ModelCapabilities {
        provider: "speaker-diarization".into(),
        model: "pyannote-segmentation-3dspeaker-eres2net".into(),
        input_mode: "whole-file".into(),
        outputs: vec!["speakers".into()],
        languages: Vec::new(),
        supports_hotwords: false,
        built_in_punctuation: false,
        recommended_threads: 2,
        fixed_memory_mib: 512,
        worker_memory_mib: 128,
        max_parallelism: 1,
        max_audio_seconds: None,
        supported_platforms: vec!["macos".into(), "windows".into(), "linux".into()],
        beta_gate: None,
    });
    let configured_resource_cap =
        config
            .resources
            .memory_limit_mib
            .unwrap_or(match config.resources.mode {
                ResourceMode::Eco => 1024,
                ResourceMode::Balanced => 2048,
                ResourceMode::Fast => 4096,
            });
    // Saving a pipeline must be deterministic. Transient memory pressure is enforced by the
    // processing scheduler's RSS budget and pause behavior, not treated as a permanent config
    // incompatibility.
    let resource_cap = configured_resource_cap;
    let live = if config.live.mode == LiveMode::ContinuousPreview {
        Some(capabilities(
            config
                .live
                .streaming_provider
                .as_deref()
                .unwrap_or(crate::sherpa_asr::PROVIDER_ID),
            config.live.streaming_model.as_deref().unwrap_or_default(),
        ))
    } else {
        None
    };
    let active = post.as_ref().unwrap_or(&finalized);
    let per_worker = active.worker_memory_mib.max(1);
    let memory_workers =
        ((resource_cap.saturating_sub(active.fixed_memory_mib)) / per_worker) as usize;
    let available_threads = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    let thread_count = active
        .recommended_threads
        .max(1)
        .min(match config.resources.mode {
            ResourceMode::Eco => (available_threads / 2).max(1),
            ResourceMode::Balanced | ResourceMode::Fast => available_threads,
        });
    let cpu_workers = available_threads / thread_count;
    let worker_count = memory_workers
        .min(cpu_workers.max(1))
        .min(active.max_parallelism.max(1))
        .max(1);
    let asr_background_memory_mib =
        active.fixed_memory_mib + per_worker * worker_count as u64 + 128;
    let speaker_background_memory_mib = speaker.as_ref().map_or(0, |capability| {
        capability.fixed_memory_mib + capability.worker_memory_mib + 128
    });
    let background_memory_mib = asr_background_memory_mib.max(speaker_background_memory_mib);
    let recording_memory_mib = finalized.fixed_memory_mib
        + finalized.worker_memory_mib
        + live.as_ref().map_or(0, |capability| {
            capability.fixed_memory_mib + capability.worker_memory_mib
        })
        + 128;
    let estimated_memory_mib = background_memory_mib.max(recording_memory_mib);
    if estimated_memory_mib > resource_cap {
        return Err(anyhow!("The selected pipeline is estimated to need {estimated_memory_mib} MiB, above the {resource_cap} MiB limit"));
    }
    Ok(ResolvedPipeline {
        config: requested_config,
        effective_config: Some(config),
        live_capabilities: live,
        finalized_capabilities: finalized,
        post_meeting_capabilities: post,
        speaker_capabilities: speaker,
        punctuation_enabled,
        speaker_refinement_enabled,
        estimated_memory_mib,
        worker_count,
        thread_count,
        decisions,
    })
}

async fn validate_model_assets<R: Runtime>(
    app: &AppHandle<R>,
    capability: &ModelCapabilities,
) -> Result<()> {
    match capability.provider.as_str() {
        crate::sherpa_asr::PROVIDER_ID => {
            crate::sherpa_asr::installed_model(app, &capability.model)
                .map_err(|error| anyhow!(error))?
                .map(|_| ())
                .ok_or_else(|| {
                    anyhow!(
                        "Model {}/{} is missing or damaged",
                        capability.provider,
                        capability.model
                    )
                })
        }
        "localWhisper" | "whisper" => {
            let models = crate::whisper_engine::commands::whisper_get_available_models()
                .await
                .map_err(anyhow::Error::msg)?;
            models
                .into_iter()
                .find(|model| model.name == capability.model)
                .filter(|model| {
                    matches!(model.status, crate::whisper_engine::ModelStatus::Available)
                })
                .map(|_| ())
                .ok_or_else(|| {
                    anyhow!(
                        "Model {}/{} is missing or damaged",
                        capability.provider,
                        capability.model
                    )
                })
        }
        "parakeet" => {
            crate::parakeet_engine::commands::parakeet_init()
                .await
                .map_err(anyhow::Error::msg)?;
            let models = crate::parakeet_engine::commands::parakeet_get_available_models()
                .await
                .map_err(anyhow::Error::msg)?;
            models
                .into_iter()
                .find(|model| model.name == capability.model)
                .filter(|model| {
                    matches!(model.status, crate::parakeet_engine::ModelStatus::Available)
                })
                .map(|_| ())
                .ok_or_else(|| {
                    anyhow!(
                        "Model {}/{} is missing or damaged",
                        capability.provider,
                        capability.model
                    )
                })
        }
        provider => Err(anyhow!("Unknown transcription Provider '{provider}'")),
    }
}

pub async fn resolve_for_app<R: Runtime>(
    app: &AppHandle<R>,
    config: PipelineConfig,
) -> Result<ResolvedPipeline> {
    let beta = load_beta(app)?;
    resolve_for_app_with_beta(app, config, &beta).await
}

pub(crate) async fn resolve_for_app_with_beta<R: Runtime>(
    app: &AppHandle<R>,
    config: PipelineConfig,
    beta: &BetaFeatures,
) -> Result<ResolvedPipeline> {
    let mut resolved = resolve(config, beta)?;
    validate_model_assets(app, &resolved.finalized_capabilities).await?;
    if let Some(capability) = &resolved.live_capabilities {
        validate_model_assets(app, capability).await?;
    }
    if let Some(capability) = &resolved.post_meeting_capabilities {
        validate_model_assets(app, capability).await?;
    }
    if resolved.speaker_refinement_enabled
        && crate::speaker_diarization::installed_model_paths(app)
            .ok()
            .flatten()
            .is_none()
    {
        resolved.speaker_refinement_enabled = false;
        resolved.speaker_capabilities = None;
        resolved.decisions.push("speakerModelUnavailable".into());
    }
    Ok(resolved)
}

pub async fn resolve_for_app_with_fallback<R: Runtime>(
    app: &AppHandle<R>,
    config: PipelineConfig,
) -> Result<ResolvedPipeline> {
    let requested = config.clone();
    let beta = load_beta(app)?;
    let mut effective = config;
    let mut fallback_decisions = Vec::<String>::new();
    loop {
        let mut resolved = resolve(effective.clone(), &beta)?;
        if let Err(error) = validate_model_assets(app, &resolved.finalized_capabilities).await {
            let stable_provider = crate::sherpa_asr::PROVIDER_ID;
            let stable_model = crate::sherpa_asr::models::SENSEVOICE_MODEL_ID;
            if resolved.finalized_capabilities.provider == stable_provider
                && resolved.finalized_capabilities.model == stable_model
            {
                return Err(error);
            }
            effective.finalized.provider = stable_provider.into();
            effective.finalized.model = stable_model.into();
            fallback_decisions.push("damagedFinalizedModelFallback".into());
            continue;
        }
        if let Some(capability) = &resolved.live_capabilities {
            if validate_model_assets(app, capability).await.is_err() {
                effective.live.mode = LiveMode::VadSegmented;
                effective.live.streaming_provider = None;
                effective.live.streaming_model = None;
                fallback_decisions.push("damagedStreamingModelFallback".into());
                continue;
            }
        }
        if let Some(capability) = &resolved.post_meeting_capabilities {
            if let Err(error) = validate_model_assets(app, capability).await {
                let stable_provider = crate::sherpa_asr::PROVIDER_ID;
                let stable_model = crate::sherpa_asr::models::SENSEVOICE_MODEL_ID;
                if capability.provider == stable_provider && capability.model == stable_model {
                    return Err(error);
                }
                effective.post_meeting_asr.provider = Some(stable_provider.into());
                effective.post_meeting_asr.model = Some(stable_model.into());
                fallback_decisions.push("damagedPostMeetingModelFallback".into());
                continue;
            }
        }
        if resolved.speaker_refinement_enabled
            && crate::speaker_diarization::installed_model_paths(app)
                .ok()
                .flatten()
                .is_none()
        {
            resolved.speaker_refinement_enabled = false;
            resolved.speaker_capabilities = None;
            resolved.decisions.push("speakerModelUnavailable".into());
        }
        resolved.config = requested;
        resolved.decisions.extend(fallback_decisions);
        return Ok(resolved);
    }
}

pub async fn resolve_loaded<R: Runtime>(app: &AppHandle<R>) -> Result<ResolvedPipeline> {
    resolve_for_app_with_fallback(app, initialize_from_legacy(app).await?).await
}

fn read_value<R: Runtime, T: for<'de> Deserialize<'de>>(
    app: &AppHandle<R>,
    key: &str,
) -> Result<Option<T>> {
    let store = app
        .store(STORE_FILE)
        .map_err(|error| anyhow!("Unable to access pipeline settings: {error}"))?;
    store
        .get(key)
        .map(|value| serde_json::from_value(value.clone()).map_err(Into::into))
        .transpose()
}

fn write_value<R: Runtime, T: Serialize>(app: &AppHandle<R>, key: &str, value: &T) -> Result<()> {
    let store = app
        .store(STORE_FILE)
        .map_err(|error| anyhow!("Unable to access pipeline settings: {error}"))?;
    store.set(key, serde_json::to_value(value)?);
    store
        .save()
        .map_err(|error| anyhow!("Unable to save pipeline settings: {error}"))
}

pub fn load_config<R: Runtime>(app: &AppHandle<R>) -> Result<PipelineConfig> {
    Ok(read_value(app, CONFIG_KEY)?.unwrap_or_default())
}

pub(crate) fn load_config_if_present<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<Option<PipelineConfig>> {
    read_value(app, CONFIG_KEY)
}

pub fn load_beta<R: Runtime>(app: &AppHandle<R>) -> Result<BetaFeatures> {
    let mut features: BetaFeatures = read_value(app, BETA_KEY)?.unwrap_or_default();
    features.custom_transcription_pipelines = true;
    Ok(features)
}

pub(crate) fn sync_legacy_finalized<R: Runtime>(
    app: &AppHandle<R>,
    provider: String,
    model: String,
) -> Result<()> {
    let mut config = load_config(app)?;
    config.finalized.provider = provider;
    config.finalized.model = model;
    write_value(app, CONFIG_KEY, &config)
}

pub(crate) fn sync_legacy_streaming<R: Runtime>(
    app: &AppHandle<R>,
    enabled: bool,
    provider: String,
    model: String,
) -> Result<()> {
    let mut config = load_config(app)?;
    config.live.mode = if enabled {
        LiveMode::ContinuousPreview
    } else {
        LiveMode::VadSegmented
    };
    config.live.streaming_provider = enabled.then_some(provider);
    config.live.streaming_model = enabled.then_some(model);
    write_value(app, CONFIG_KEY, &config)
}

pub(crate) fn sync_legacy_speaker<R: Runtime>(
    app: &AppHandle<R>,
    enabled: bool,
    speaker_count: Option<usize>,
) -> Result<()> {
    let mut config = load_config(app)?;
    config.speaker.live_enabled = enabled;
    config.speaker.speaker_count = speaker_count;
    write_value(app, CONFIG_KEY, &config)
}

pub async fn initialize_from_legacy<R: Runtime>(app: &AppHandle<R>) -> Result<PipelineConfig> {
    if let Some(config) = read_value::<R, PipelineConfig>(app, CONFIG_KEY)? {
        if !read_value::<R, bool>(app, BETA_MIGRATION_KEY)?.unwrap_or(false) {
            let mut beta = load_beta(app)?;
            preserve_selected_beta_gates(&config, &mut beta);
            write_value(app, BETA_KEY, &beta)?;
            write_value(app, BETA_MIGRATION_KEY, &true)?;
        }
        return Ok(config);
    }
    let mut config = PipelineConfig::default();
    if let Ok(Some(legacy)) =
        crate::api::api::api_get_transcript_config(app.clone(), app.clone().state(), None).await
    {
        config.finalized.provider = legacy.provider;
        config.finalized.model = legacy.model;
    }
    if let Ok(Some(streaming)) = crate::sherpa_asr::streaming_config::load_config_if_present(app) {
        if streaming.enabled {
            config.live.mode = LiveMode::ContinuousPreview;
            config.live.streaming_provider = Some(streaming.provider);
            config.live.streaming_model = Some(streaming.model);
        }
    }
    if let Ok(speaker) = crate::speaker_diarization::configuration::load_config(app) {
        config.speaker.live_enabled = speaker.enabled;
        config.speaker.speaker_count = speaker.speaker_count;
        if !speaker.enabled {
            config.speaker.refinement = SpeakerRefinementPolicy::Off;
        }
    }
    let mut beta = load_beta(app)?;
    preserve_selected_beta_gates(&config, &mut beta);
    write_value(app, BETA_KEY, &beta)?;
    write_value(app, CONFIG_KEY, &config)?;
    write_value(app, BETA_MIGRATION_KEY, &true)?;
    Ok(config)
}

#[tauri::command]
pub async fn pipeline_get_config<R: Runtime>(app: AppHandle<R>) -> Result<PipelineConfig, String> {
    initialize_from_legacy(&app)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn pipeline_resolve_config<R: Runtime>(
    app: AppHandle<R>,
    config: PipelineConfig,
) -> Result<ResolvedPipeline, String> {
    resolve_for_app_with_fallback(&app, config)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn pipeline_save_config<R: Runtime>(
    app: AppHandle<R>,
    config: PipelineConfig,
) -> Result<ResolvedPipeline, String> {
    let resolved = resolve_for_app(&app, config)
        .await
        .map_err(|error| error.to_string())?;
    write_value(&app, CONFIG_KEY, &resolved.config).map_err(|error| error.to_string())?;
    Ok(resolved)
}

#[tauri::command]
pub async fn pipeline_get_beta_features<R: Runtime>(
    app: AppHandle<R>,
) -> Result<BetaFeatures, String> {
    initialize_from_legacy(&app)
        .await
        .map_err(|error| error.to_string())?;
    load_beta(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn pipeline_migrate_legacy_beta_features<R: Runtime>(
    app: AppHandle<R>,
    legacy_import_and_retranscribe: Option<bool>,
) -> Result<BetaFeatures, String> {
    if read_value::<R, BetaFeatures>(&app, BETA_KEY)
        .map_err(|error| error.to_string())?
        .is_none()
    {
        let mut features = BetaFeatures::default();
        if let Some(enabled) = legacy_import_and_retranscribe {
            features.import_and_retranscribe = enabled;
        }
        write_value(&app, BETA_KEY, &features).map_err(|error| error.to_string())?;
    }
    initialize_from_legacy(&app)
        .await
        .map_err(|error| error.to_string())?;
    load_beta(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn pipeline_save_beta_features<R: Runtime>(
    app: AppHandle<R>,
    mut features: BetaFeatures,
) -> Result<(), String> {
    features.custom_transcription_pipelines = true;
    write_value(&app, BETA_MIGRATION_KEY, &true).map_err(|error| error.to_string())?;
    write_value(&app, BETA_KEY, &features).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balanced_disables_post_meeting_asr_and_keeps_background_speakers() {
        let resolved = resolve(PipelineConfig::default(), &BetaFeatures::default()).unwrap();
        assert_eq!(
            resolved.config.post_meeting_asr.policy,
            PostMeetingPolicy::Off
        );
        assert!(resolved.speaker_refinement_enabled);
    }

    #[test]
    fn pipeline_profiles_are_core_even_for_an_old_disabled_beta_value() {
        let mut config = PipelineConfig::default();
        config.preset = PipelinePreset::Fast;
        let beta = BetaFeatures {
            custom_transcription_pipelines: false,
            ..Default::default()
        };

        let resolved = resolve(config, &beta).unwrap();
        assert_eq!(resolved.runtime_config().preset, PipelinePreset::Fast);
        assert!(!resolved
            .decisions
            .iter()
            .any(|decision| decision == "customPipelineFallback"));
    }

    #[test]
    fn built_in_punctuation_avoids_external_model() {
        let mut config = PipelineConfig::default();
        config.preset = PipelinePreset::Custom;
        config.finalized.provider = "parakeet".into();
        config.finalized.model = "parakeet-tdt-0.6b-v3-int8".into();
        // Keep this capability test independent from the host's currently
        // available memory by excluding the unrelated speaker model budget.
        config.speaker.live_enabled = false;
        config.speaker.refinement = SpeakerRefinementPolicy::Off;
        let beta = BetaFeatures {
            custom_transcription_pipelines: true,
            ..Default::default()
        };
        assert!(!resolve(config, &beta).unwrap().punctuation_enabled);
    }

    #[test]
    fn continuous_model_is_rejected_as_finalized() {
        let mut config = PipelineConfig::default();
        config.preset = PipelinePreset::Custom;
        config.finalized.model = crate::sherpa_asr::models::PARAFORMER_ONLINE_MODEL_ID.into();
        let beta = BetaFeatures {
            custom_transcription_pipelines: true,
            experimental_asr_models: true,
            ..Default::default()
        };
        assert!(resolve(config, &beta).is_err());
    }

    #[test]
    fn gated_experimental_selection_keeps_choice_and_uses_stable_fallback() {
        let mut config = PipelineConfig::default();
        config.finalized.model = crate::sherpa_asr::models::QWEN3_ASR_MODEL_ID.into();
        let resolved = resolve(config.clone(), &BetaFeatures::default()).unwrap();
        assert_eq!(resolved.config.finalized.model, config.finalized.model);
        assert_eq!(
            resolved.runtime_config().finalized.model,
            crate::sherpa_asr::models::SENSEVOICE_MODEL_ID
        );
        assert!(resolved
            .decisions
            .iter()
            .any(|item| item == "stableFallback"));
    }

    #[test]
    fn gated_continuous_final_selection_falls_back_before_input_validation() {
        let mut config = PipelineConfig::default();
        config.finalized.model = crate::sherpa_asr::models::PARAFORMER_ONLINE_MODEL_ID.into();
        assert!(resolve(config, &BetaFeatures::default()).is_ok());
    }

    #[test]
    fn streaming_and_finalized_models_must_be_distinct() {
        let mut config = PipelineConfig::default();
        config.preset = PipelinePreset::Custom;
        config.live.mode = LiveMode::ContinuousPreview;
        config.live.streaming_provider = Some(config.finalized.provider.clone());
        config.live.streaming_model = Some(config.finalized.model.clone());
        let beta = BetaFeatures {
            custom_transcription_pipelines: true,
            experimental_asr_models: true,
            ..Default::default()
        };
        assert!(resolve(config, &beta)
            .unwrap_err()
            .to_string()
            .contains("must be different"));
    }

    #[test]
    fn unsupported_fixed_language_is_rejected() {
        let mut config = PipelineConfig::default();
        config.finalized.language = "de".into();
        assert!(resolve(config, &BetaFeatures::default())
            .unwrap_err()
            .to_string()
            .contains("not supported"));
    }

    #[test]
    fn memory_limit_rejects_an_oversized_pipeline() {
        let mut config = PipelineConfig::default();
        config.preset = PipelinePreset::Custom;
        config.resources.memory_limit_mib = Some(512);
        let beta = BetaFeatures {
            custom_transcription_pipelines: true,
            ..Default::default()
        };
        assert!(resolve(config, &beta)
            .unwrap_err()
            .to_string()
            .contains("above the 512 MiB limit"));
    }

    #[test]
    fn external_punctuation_is_skipped_for_unsupported_language() {
        let mut config = PipelineConfig::default();
        config.finalized.language = "ja".into();
        let resolved = resolve(config, &BetaFeatures::default()).unwrap();
        assert!(!resolved.punctuation_enabled);
    }

    #[test]
    fn background_speaker_memory_is_included_in_the_budget() {
        let resolved = resolve(PipelineConfig::default(), &BetaFeatures::default()).unwrap();
        assert!(resolved.speaker_capabilities.is_some());
        assert!(resolved.estimated_memory_mib >= 768);
    }

    #[test]
    fn native_speaker_output_suppresses_implicit_refinement() {
        let config = PipelineConfig::default();
        let mut capability = capabilities("test", "speaker-aware");
        capability.outputs.push("speakers".into());
        assert!(!should_run_speaker_refinement(&config, &capability));
    }

    #[test]
    fn custom_pipeline_can_explicitly_refine_native_speaker_output() {
        let mut config = PipelineConfig::default();
        config.preset = PipelinePreset::Custom;
        config.speaker.refinement = SpeakerRefinementPolicy::Manual;
        let mut capability = capabilities("test", "speaker-aware");
        capability.outputs.push("speakers".into());
        assert!(should_run_speaker_refinement(&config, &capability));
    }

    #[test]
    fn existing_beta_pipeline_selections_enable_their_gates_during_migration() {
        let mut config = PipelineConfig::default();
        config.preset = PipelinePreset::Quality;
        config.post_meeting_asr.model = Some(crate::sherpa_asr::models::QWEN3_ASR_MODEL_ID.into());
        let mut beta = BetaFeatures::default();
        preserve_selected_beta_gates(&config, &mut beta);
        assert!(beta.custom_transcription_pipelines);
        assert!(beta.experimental_asr_models);
    }

    #[test]
    fn post_meeting_model_must_support_the_transcription_language() {
        let mut config = PipelineConfig::default();
        config.preset = PipelinePreset::Custom;
        config.finalized.provider = "localWhisper".into();
        config.finalized.model = "ggml-base.bin".into();
        config.finalized.language = "zh".into();
        config.post_meeting_asr.policy = PostMeetingPolicy::Manual;
        config.post_meeting_asr.provider = Some("parakeet".into());
        config.post_meeting_asr.model = Some("parakeet-tdt-0.6b-v3-int8".into());
        let beta = BetaFeatures {
            custom_transcription_pipelines: true,
            ..Default::default()
        };
        assert!(resolve(config, &beta)
            .unwrap_err()
            .to_string()
            .contains("post-meeting model"));
    }

    #[test]
    fn recording_priority_is_always_enabled_in_the_effective_config() {
        let mut config = PipelineConfig::default();
        config.resources.pause_automatic_jobs_during_recording = false;
        let resolved = resolve(config, &BetaFeatures::default()).unwrap();
        assert!(
            resolved
                .runtime_config()
                .resources
                .pause_automatic_jobs_during_recording
        );
    }

    #[test]
    fn pipeline_json_contract_preserves_mib_acronym_spelling() {
        let mut config = PipelineConfig::default();
        config.resources.memory_limit_mib = Some(2_048);
        let config_json = serde_json::to_value(&config).unwrap();
        assert_eq!(config_json["resources"]["memoryLimitMiB"], 2_048);
        assert!(config_json["resources"].get("memoryLimitMib").is_none());
        let round_trip: PipelineConfig = serde_json::from_value(config_json).unwrap();
        assert_eq!(round_trip.resources.memory_limit_mib, Some(2_048));

        let resolved = resolve(round_trip, &BetaFeatures::default()).unwrap();
        let resolved_json = serde_json::to_value(resolved).unwrap();
        assert!(resolved_json.get("estimatedMemoryMiB").is_some());
        assert!(resolved_json.get("estimatedMemoryMib").is_none());
        assert!(resolved_json["finalizedCapabilities"]
            .get("fixedMemoryMiB")
            .is_some());
        assert!(resolved_json["finalizedCapabilities"]
            .get("workerMemoryMiB")
            .is_some());
    }

    #[test]
    fn whisper_memory_budget_uses_the_registered_model_size_and_runtime_workspace() {
        let capability = capabilities("localWhisper", "large-v3-q5_0");
        assert!(capability.fixed_memory_mib >= 1_031);
        assert!(capability.worker_memory_mib >= 640);

        let single_worker_estimate =
            capability.fixed_memory_mib + capability.worker_memory_mib + 128;
        assert!(single_worker_estimate >= 1_799);
    }

    #[test]
    fn transient_system_memory_pressure_does_not_invalidate_a_saveable_pipeline() {
        let mut config = PipelineConfig::default();
        config.finalized.provider = "localWhisper".into();
        config.finalized.model = "large-v3-q5_0".into();

        let resolved = resolve(config, &BetaFeatures::default()).unwrap();
        assert_eq!(resolved.estimated_memory_mib, 1_800);
        assert_eq!(resolved.worker_count, 1);
    }
}
