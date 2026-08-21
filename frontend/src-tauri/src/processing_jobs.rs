use crate::audio::common::{
    create_transcript_segments, split_segment_at_silence, unload_engine_after_batch,
    write_transcripts_json,
};
use crate::audio::decoder::decode_audio_range_to_whisper_format;
use crate::audio::transcription::engine::load_transcription_provider_for_config;
use crate::audio::vad::get_speech_chunks;
use crate::pipeline::{PostMeetingPolicy, ResolvedPipeline, SpeakerRefinementPolicy};
use crate::speaker_diarization::engine::{
    diarize_audio_file_in_windows_resumable, DiarizationCheckpoint,
};
use crate::speaker_diarization::{installed_model_paths, refine_speaker_labels};
use crate::state::AppState;
use anyhow::{anyhow, Result};
use chrono::Utc;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use uuid::Uuid;

static RUNNING_JOBS: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));
static STOP_REQUESTED: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));
static DISPATCHER_RUNNING: AtomicBool = AtomicBool::new(false);
static RECORDING_PRIORITY_REQUESTED: AtomicBool = AtomicBool::new(false);
const RECOMPUTE_WINDOW_SECONDS: f64 = 300.0;
const RECOMPUTE_PREFETCH_CAPACITY: usize = 1;
const MAX_RESIDENT_AUDIO_WINDOWS: usize = RECOMPUTE_PREFETCH_CAPACITY + 1;
const MAX_SEGMENT_SAMPLES: usize = 25 * 16_000;

pub struct RecordingPriorityGuard;

impl Drop for RecordingPriorityGuard {
    fn drop(&mut self) {
        RECORDING_PRIORITY_REQUESTED.store(false, Ordering::SeqCst);
    }
}

pub async fn prepare_for_recording() -> Result<RecordingPriorityGuard, String> {
    RECORDING_PRIORITY_REQUESTED.store(true, Ordering::SeqCst);
    let running = RUNNING_JOBS
        .lock()
        .expect("running jobs lock poisoned")
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    STOP_REQUESTED
        .lock()
        .expect("stop requests lock poisoned")
        .extend(running);
    for _ in 0..60 {
        if RUNNING_JOBS
            .lock()
            .expect("running jobs lock poisoned")
            .is_empty()
        {
            return Ok(RecordingPriorityGuard);
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    RECORDING_PRIORITY_REQUESTED.store(false, Ordering::SeqCst);
    Err("A background meeting-processing window is still finishing. Recording was not started to avoid loading two heavy models at once; try again shortly.".into())
}

fn plan_recompute_windows(duration_seconds: f64) -> Vec<(f64, f64)> {
    let mut windows = Vec::new();
    let mut start = 0.0;
    while start < duration_seconds {
        let duration = RECOMPUTE_WINDOW_SECONDS.min(duration_seconds - start);
        windows.push((start, duration));
        start += duration;
    }
    windows
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AsrCheckpoint {
    next_window_start: f64,
    transcripts: Vec<(String, f64, f64, Option<String>)>,
    decode_seconds: f64,
    vad_seconds: f64,
    asr_seconds: f64,
    completed_windows: usize,
    peak_rss_mib: u64,
}

struct PreparedAsrWindow {
    start: f64,
    duration: f64,
    speech: Vec<crate::audio::vad::SpeechSegment>,
    decode_seconds: f64,
    vad_seconds: f64,
}

fn spawn_asr_window_producer(
    audio_path: PathBuf,
    first_window_start: f64,
    duration_seconds: f64,
    whole_file: bool,
) -> tokio::sync::mpsc::Receiver<Result<PreparedAsrWindow>> {
    let (sender, receiver) = tokio::sync::mpsc::channel(RECOMPUTE_PREFETCH_CAPACITY);
    tokio::spawn(async move {
        let mut window_start = first_window_start;
        while window_start < duration_seconds {
            if RECORDING_PRIORITY_REQUESTED.load(Ordering::SeqCst) {
                break;
            }
            let window_duration = RECOMPUTE_WINDOW_SECONDS.min(duration_seconds - window_start);
            let decode_path = audio_path.clone();
            let decode_started = Instant::now();
            let samples = match tokio::task::spawn_blocking(move || {
                decode_audio_range_to_whisper_format(&decode_path, window_start, window_duration)
            })
            .await
            {
                Ok(Ok(samples)) => samples,
                Ok(Err(error)) => {
                    let _ = sender.send(Err(error.into())).await;
                    break;
                }
                Err(error) => {
                    let _ = sender
                        .send(Err(anyhow!("Audio decode task failed: {error}")))
                        .await;
                    break;
                }
            };
            let decode_seconds = decode_started.elapsed().as_secs_f64();
            let vad_started = Instant::now();
            let speech = if whole_file {
                Ok(vec![crate::audio::vad::SpeechSegment {
                    end_timestamp_ms: samples.len() as f64 / 16.0,
                    start_timestamp_ms: 0.0,
                    confidence: 1.0,
                    samples,
                }])
            } else {
                tokio::task::spawn_blocking(move || get_speech_chunks(&samples, 2_000))
                    .await
                    .map_err(|error| anyhow!("VAD task failed: {error}"))
                    .and_then(|result| result.map_err(Into::into))
            };
            let prepared = speech.map(|speech| PreparedAsrWindow {
                start: window_start,
                duration: window_duration,
                speech,
                decode_seconds,
                vad_seconds: vad_started.elapsed().as_secs_f64(),
            });
            if sender.send(prepared).await.is_err() {
                break;
            }
            window_start = (window_start + window_duration).min(duration_seconds);
        }
    });
    receiver
}

fn current_process_rss_mib() -> u64 {
    let Ok(pid) = sysinfo::get_current_pid() else {
        return 0;
    };
    let mut system = sysinfo::System::new_all();
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
    system
        .process(pid)
        .map_or(0, |process| process.memory() / (1024 * 1024))
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct MeetingProcessingJob {
    pub id: String,
    pub meeting_id: String,
    pub kind: String,
    pub automatic: bool,
    pub status: String,
    pub progress: i64,
    pub config_snapshot: String,
    pub checkpoint: Option<String>,
    pub depends_on: Option<String>,
    pub error: Option<String>,
    pub metrics: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnqueueMeetingJobsRequest {
    pub meeting_id: String,
    pub kind: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub language: Option<String>,
    pub speaker_count: Option<usize>,
    pub speaker_refinement: Option<bool>,
    pub resource_mode: Option<crate::pipeline::ResourceMode>,
}

async fn insert_job(
    pool: &SqlitePool,
    meeting_id: &str,
    kind: &str,
    snapshot: &str,
    depends_on: Option<&str>,
    automatic: bool,
) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO meeting_processing_jobs
         (id, meeting_id, kind, automatic, status, progress, config_snapshot, depends_on, created_at, updated_at)
         VALUES (?, ?, ?, ?, 'pending', 0, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(meeting_id)
    .bind(kind)
    .bind(automatic)
    .bind(snapshot)
    .bind(depends_on)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(id)
}

async fn get_job(pool: &SqlitePool, id: &str) -> Result<MeetingProcessingJob> {
    sqlx::query_as("SELECT * FROM meeting_processing_jobs WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| anyhow!("Processing job not found"))
}

async fn set_state(
    pool: &SqlitePool,
    id: &str,
    status: &str,
    progress: i64,
    error: Option<&str>,
    metrics: Option<&str>,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE meeting_processing_jobs SET status = ?, progress = ?, error = ?, metrics = ?,
         updated_at = ?, started_at = CASE WHEN ? = 'processing' THEN COALESCE(started_at, ?) ELSE started_at END,
         completed_at = CASE
             WHEN ? IN ('completed', 'failed', 'cancelled') THEN ?
             WHEN ? = 'pending' THEN NULL
             ELSE completed_at
         END
         WHERE id = ?",
    )
    .bind(status)
    .bind(progress.clamp(0, 100))
    .bind(error)
    .bind(metrics)
    .bind(&now)
    .bind(status)
    .bind(&now)
    .bind(status)
    .bind(&now)
    .bind(status)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn emit_job<R: Runtime>(app: &AppHandle<R>, id: &str) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    if let Ok(job) = get_job(state.db_manager.pool(), id).await {
        let _ = app.emit("meeting-processing-job-updated", job);
    }
}

async fn meeting_audio_path(pool: &SqlitePool, meeting_id: &str) -> Result<PathBuf> {
    let folder: Option<String> =
        sqlx::query_scalar("SELECT folder_path FROM meetings WHERE id = ?")
            .bind(meeting_id)
            .fetch_optional(pool)
            .await?
            .flatten();
    let folder = folder.ok_or_else(|| anyhow!("Meeting has no recording folder"))?;
    crate::audio::retranscription::find_audio_file(Path::new(&folder))
}

async fn run_speaker_refinement<R: Runtime>(
    app: &AppHandle<R>,
    job: &MeetingProcessingJob,
    resolved: &ResolvedPipeline,
) -> Result<serde_json::Value> {
    let state = app.state::<AppState>();
    let pool = state.db_manager.pool();
    let rows: Vec<(String, Option<f64>, Option<f64>, Option<String>)> = sqlx::query_as(
        "SELECT id, audio_start_time, audio_end_time, speaker FROM transcripts
         WHERE meeting_id = ? ORDER BY audio_start_time ASC, id ASC",
    )
    .bind(&job.meeting_id)
    .fetch_all(pool)
    .await?;
    let indexed = rows
        .into_iter()
        .enumerate()
        .filter_map(|(index, (id, start, end, speaker))| {
            Some((id, index as u64, start?, end?, speaker))
        })
        .collect::<Vec<_>>();
    if indexed.is_empty() {
        return Ok(serde_json::json!({ "windows": 0, "reason": "no-timestamped-transcripts" }));
    }
    let ranges = indexed
        .iter()
        .map(|(_, _, start, end, _)| (*start, *end))
        .collect::<Vec<_>>();
    let segments = indexed
        .iter()
        .map(|(_, sequence, start, end, speaker)| (*sequence, *start, *end, speaker.clone()))
        .collect::<Vec<_>>();
    let paths = installed_model_paths(app)?
        .ok_or_else(|| anyhow!("Speaker diarization model is missing or damaged"))?;
    let audio_path = meeting_audio_path(pool, &job.meeting_id).await?;
    let progress_app = app.clone();
    let progress_job = job.id.clone();
    let stop_job = job.id.clone();
    let speaker_count = resolved.runtime_config().speaker.speaker_count;
    let started = Instant::now();
    let checkpoint = job
        .checkpoint
        .as_deref()
        .and_then(|value| serde_json::from_str::<DiarizationCheckpoint>(value).ok());
    let initial_completed = checkpoint
        .as_ref()
        .map_or(0, |value| value.completed_windows);
    let checkpoint_started = started;
    let turns = tokio::task::spawn_blocking(move || {
        let mut last_saved = initial_completed;
        diarize_audio_file_in_windows_resumable(
            &audio_path,
            &paths,
            &ranges,
            speaker_count,
            checkpoint,
            |window, checkpoint| {
                let should_continue = !STOP_REQUESTED
                    .lock()
                    .expect("stop requests lock poisoned")
                    .contains(&stop_job)
                    && !crate::audio::recording_commands::is_recording_now()
                    && !RECORDING_PRIORITY_REQUESTED.load(Ordering::SeqCst);
                let progress = if window.total_windows == 0 {
                    10
                } else {
                    10 + ((window.completed_windows * 80) / window.total_windows) as i64
                };
                if window.completed_windows > last_saved {
                    last_saved = window.completed_windows;
                    if let Ok(encoded) = serde_json::to_string(checkpoint) {
                        let app = progress_app.clone();
                        let id = progress_job.clone();
                        let elapsed = checkpoint_started.elapsed().as_secs_f64();
                        let completed = window.completed_windows;
                        let total = window.total_windows;
                        tauri::async_runtime::block_on(async move {
                            if let Some(state) = app.try_state::<AppState>() {
                                let estimated = if completed == 0 {
                                    0.0
                                } else {
                                    elapsed / completed as f64 * total.saturating_sub(completed) as f64
                                };
                                let metrics = serde_json::json!({
                                    "windows": completed,
                                    "diarizationSeconds": elapsed,
                                    "estimatedRemainingSeconds": estimated,
                                    "peakRssMiB": current_process_rss_mib(),
                                })
                                .to_string();
                                let _ = sqlx::query("UPDATE meeting_processing_jobs SET checkpoint = ?, progress = ?, metrics = ?, updated_at = ? WHERE id = ?")
                                    .bind(encoded).bind(progress.clamp(1, 95)).bind(metrics)
                                    .bind(Utc::now().to_rfc3339()).bind(&id)
                                    .execute(state.db_manager.pool()).await;
                                emit_job(&app, &id).await;
                            }
                        });
                    }
                }
                should_continue
            },
        )
    })
    .await
    .map_err(|error| anyhow!("Speaker refinement task failed: {error}"))??;

    let latest = get_job(pool, &job.id).await?;
    if matches!(latest.status.as_str(), "cancelled" | "paused")
        || crate::audio::recording_commands::is_recording_now()
        || STOP_REQUESTED
            .lock()
            .expect("stop requests lock poisoned")
            .contains(&job.id)
    {
        return Err(anyhow!("processing-interrupted"));
    }
    let updates = refine_speaker_labels(&segments, &turns);
    let id_by_sequence = indexed
        .iter()
        .map(|(id, sequence, _, _, _)| (*sequence, id.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    let mut overrides = std::collections::HashMap::new();
    for update in updates {
        if let Some(id) = id_by_sequence.get(&update.sequence_id) {
            if let Some(speaker) = update.speaker {
                overrides.insert(id.clone(), speaker);
            }
        }
    }
    let save_started = Instant::now();
    crate::speaker_mapping::save_speaker_overrides(pool, &job.meeting_id, overrides)
        .await
        .map_err(anyhow::Error::msg)?;
    Ok(serde_json::json!({
        "turns": turns.len(),
        "diarizationSeconds": started.elapsed().as_secs_f64(),
        "saveSeconds": save_started.elapsed().as_secs_f64(),
        "peakRssMiB": current_process_rss_mib()
    }))
}

async fn save_checkpoint(
    pool: &SqlitePool,
    job_id: &str,
    checkpoint: &AsrCheckpoint,
    progress: i64,
    duration_seconds: f64,
) -> Result<()> {
    let encoded = serde_json::to_string(checkpoint)?;
    let processing_seconds =
        checkpoint.decode_seconds + checkpoint.vad_seconds + checkpoint.asr_seconds;
    let rtf = processing_seconds / checkpoint.next_window_start.max(1.0);
    let metrics = serde_json::to_string(&serde_json::json!({
        "windows": checkpoint.completed_windows,
        "rtf": rtf,
        "estimatedRemainingSeconds": (duration_seconds - checkpoint.next_window_start).max(0.0) * rtf,
        "peakRssMiB": checkpoint.peak_rss_mib,
        "stages": {
            "decodeAndResampleSeconds": checkpoint.decode_seconds,
            "vadSeconds": checkpoint.vad_seconds,
            "asrAndEnhancementSeconds": checkpoint.asr_seconds,
        }
    }))?;
    sqlx::query("UPDATE meeting_processing_jobs SET checkpoint = ?, progress = ?, metrics = ?, updated_at = ? WHERE id = ?")
        .bind(encoded)
        .bind(progress.clamp(1, 95))
        .bind(metrics)
        .bind(Utc::now().to_rfc3339())
        .bind(job_id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn run_asr_recompute_inner<R: Runtime>(
    app: &AppHandle<R>,
    job: &MeetingProcessingJob,
    resolved: &ResolvedPipeline,
) -> Result<serde_json::Value> {
    let provider_name = resolved
        .runtime_config()
        .post_meeting_asr
        .provider
        .as_deref()
        .ok_or_else(|| anyhow!("Post-meeting ASR provider is not configured"))?;
    let model_name = resolved
        .runtime_config()
        .post_meeting_asr
        .model
        .as_deref()
        .ok_or_else(|| anyhow!("Post-meeting ASR model is not configured"))?;
    let state = app.state::<AppState>();
    let pool = state.db_manager.pool();
    let audio_path = meeting_audio_path(pool, &job.meeting_id).await?;
    let folder_path = audio_path
        .parent()
        .ok_or_else(|| anyhow!("Recording folder is invalid"))?
        .to_path_buf();
    let transcript_duration: f64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(audio_end_time), 0) FROM transcripts WHERE meeting_id = ?",
    )
    .bind(&job.meeting_id)
    .fetch_one(pool)
    .await?;
    let duration_seconds = crate::audio::import::extract_duration_from_metadata(&audio_path)
        .unwrap_or(transcript_duration);
    if duration_seconds <= 0.0 {
        return Err(anyhow!(
            "Meeting has no timestamped transcript to determine audio duration"
        ));
    }

    let mut checkpoint = job
        .checkpoint
        .as_deref()
        .and_then(|value| serde_json::from_str::<AsrCheckpoint>(value).ok())
        .unwrap_or_default();
    let mut provider_config = resolved.runtime_config().clone();
    provider_config.preset = crate::pipeline::PipelinePreset::Balanced;
    let provider = load_transcription_provider_for_config(
        app,
        provider_config,
        Some(provider_name),
        Some(model_name),
    )
    .await
    .map_err(|error| anyhow!(error))?;
    let language =
        Some(resolved.runtime_config().finalized.language.clone()).filter(|value| value != "auto");
    let total_windows = plan_recompute_windows(duration_seconds).len().max(1);
    let wall_started = Instant::now();

    let whole_file = resolved
        .post_meeting_capabilities
        .as_ref()
        .is_some_and(|capability| capability.input_mode == "whole-file");
    let mut prepared_windows = spawn_asr_window_producer(
        audio_path,
        checkpoint.next_window_start,
        duration_seconds,
        whole_file,
    );

    while let Some(prepared) = prepared_windows.recv().await {
        let latest = get_job(pool, &job.id).await?;
        if matches!(latest.status.as_str(), "paused" | "cancelled")
            || crate::audio::recording_commands::is_recording().await
            || RECORDING_PRIORITY_REQUESTED.load(Ordering::SeqCst)
        {
            return Err(anyhow!("processing-interrupted"));
        }
        let prepared = prepared?;
        let window_start = prepared.start;
        let window_duration = prepared.duration;
        checkpoint.decode_seconds += prepared.decode_seconds;
        checkpoint.vad_seconds += prepared.vad_seconds;

        for segment in prepared.speech {
            let segments = if !whole_file && segment.samples.len() > MAX_SEGMENT_SAMPLES {
                split_segment_at_silence(&segment, MAX_SEGMENT_SAMPLES)
            } else {
                vec![segment]
            };
            for segment in segments {
                if segment.samples.len() < 1_600 {
                    continue;
                }
                let latest = get_job(pool, &job.id).await?;
                if matches!(latest.status.as_str(), "paused" | "cancelled")
                    || RECORDING_PRIORITY_REQUESTED.load(Ordering::SeqCst)
                {
                    return Err(anyhow!("processing-interrupted"));
                }
                let asr_started = Instant::now();
                let result = provider
                    .transcribe(segment.samples, language.clone())
                    .await
                    .map_err(|error| anyhow!("Transcription failed: {error}"))?;
                checkpoint.asr_seconds += asr_started.elapsed().as_secs_f64();
                if !result.is_partial && !result.text.trim().is_empty() {
                    checkpoint.transcripts.push((
                        result.text,
                        (window_start * 1_000.0) + segment.start_timestamp_ms,
                        (window_start * 1_000.0) + segment.end_timestamp_ms,
                        None,
                    ));
                }
            }
        }
        checkpoint.next_window_start = (window_start + window_duration).min(duration_seconds);
        checkpoint.completed_windows += 1;
        checkpoint.peak_rss_mib = checkpoint.peak_rss_mib.max(current_process_rss_mib());
        let progress = 5 + ((checkpoint.completed_windows * 85) / total_windows) as i64;
        save_checkpoint(pool, &job.id, &checkpoint, progress, duration_seconds).await?;
        emit_job(app, &job.id).await;
        let memory_budget = resolved
            .runtime_config()
            .resources
            .memory_limit_mib
            .unwrap_or(match resolved.runtime_config().resources.mode {
                crate::pipeline::ResourceMode::Eco => 1_024,
                crate::pipeline::ResourceMode::Balanced => 2_048,
                crate::pipeline::ResourceMode::Fast => 4_096,
            });
        if checkpoint.peak_rss_mib > memory_budget {
            set_state(
                pool,
                &job.id,
                "paused",
                progress,
                Some("The processing memory budget was exceeded; reduce the resource mode or resume manually"),
                None,
            )
            .await?;
            emit_job(app, &job.id).await;
            return Err(anyhow!("processing-interrupted"));
        }
    }

    let segments = create_transcript_segments(&checkpoint.transcripts);
    let save_started = Instant::now();
    replace_transcript_rows(pool, &job.meeting_id, &segments).await?;
    if let Err(error) = write_transcripts_json(&folder_path, &segments) {
        log::warn!("Failed to update transcripts.json after background recompute: {error}");
    }
    let save_seconds = save_started.elapsed().as_secs_f64();
    Ok(serde_json::json!({
        "windows": checkpoint.completed_windows,
        "segments": segments.len(),
        "decodeSeconds": checkpoint.decode_seconds,
        "decodeAndResampleSeconds": checkpoint.decode_seconds,
        "vadSeconds": checkpoint.vad_seconds,
        "asrSeconds": checkpoint.asr_seconds,
        "asrAndEnhancementSeconds": checkpoint.asr_seconds,
        "saveSeconds": save_seconds,
        "wallSeconds": wall_started.elapsed().as_secs_f64(),
        "rtf": wall_started.elapsed().as_secs_f64() / duration_seconds,
        "windowSeconds": RECOMPUTE_WINDOW_SECONDS,
        "prefetchCapacity": RECOMPUTE_PREFETCH_CAPACITY,
        "maxResidentAudioWindows": MAX_RESIDENT_AUDIO_WINDOWS,
        "estimatedMemoryMiB": resolved.estimated_memory_mib,
        "peakRssMiB": checkpoint.peak_rss_mib,
        "workers": resolved.worker_count,
        "threadsPerWorker": resolved.thread_count
    }))
}

async fn replace_transcript_rows(
    pool: &SqlitePool,
    meeting_id: &str,
    segments: &[crate::api::TranscriptSegment],
) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM transcripts WHERE meeting_id = ?")
        .bind(meeting_id)
        .execute(&mut *tx)
        .await?;
    for segment in segments {
        sqlx::query("INSERT INTO transcripts (id, meeting_id, transcript, timestamp, audio_start_time, audio_end_time, duration, speaker) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(&segment.id).bind(meeting_id).bind(&segment.text).bind(&segment.timestamp)
            .bind(segment.audio_start_time).bind(segment.audio_end_time).bind(segment.duration).bind(&segment.speaker)
            .execute(&mut *tx).await?;
    }
    sqlx::query("DELETE FROM meeting_speaker_maps WHERE meeting_id = ?")
        .bind(meeting_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn run_asr_recompute<R: Runtime>(
    app: &AppHandle<R>,
    job: &MeetingProcessingJob,
    resolved: &ResolvedPipeline,
) -> Result<serde_json::Value> {
    let provider = resolved.runtime_config().post_meeting_asr.provider.clone();
    let result = run_asr_recompute_inner(app, job, resolved).await;
    unload_engine_after_batch(provider.as_deref()).await;
    result
}

async fn run_job<R: Runtime>(app: AppHandle<R>, id: String) {
    {
        let mut running = RUNNING_JOBS.lock().expect("running jobs lock poisoned");
        if !running.insert(id.clone()) {
            return;
        }
    }
    let result = async {
        let state = app.state::<AppState>();
        let pool = state.db_manager.pool();
        let job = get_job(pool, &id).await?;
        if job.status != "pending" {
            return Ok::<_, anyhow::Error>(());
        }
        if crate::audio::recording_commands::is_recording().await {
            return Ok(());
        }
        if let Some(dependency) = &job.depends_on {
            let status: Option<String> =
                sqlx::query_scalar("SELECT status FROM meeting_processing_jobs WHERE id = ?")
                    .bind(dependency)
                    .fetch_optional(pool)
                    .await?;
            if status.as_deref() != Some("completed") {
                return Ok(());
            }
        }
        set_state(pool, &id, "processing", 1, None, None).await?;
        emit_job(&app, &id).await;
        let resolved: ResolvedPipeline = serde_json::from_str(&job.config_snapshot)?;
        let metrics = match job.kind.as_str() {
            "speaker_refinement" => run_speaker_refinement(&app, &job, &resolved).await?,
            "asr_recompute" => run_asr_recompute(&app, &job, &resolved).await?,
            _ => return Err(anyhow!("Unknown processing job kind")),
        };
        let latest = get_job(pool, &id).await?;
        if latest.status != "cancelled" {
            let metrics = serde_json::to_string(&metrics)?;
            set_state(pool, &id, "completed", 100, None, Some(&metrics)).await?;
            emit_job(&app, &id).await;
            let _ = app.emit(
                "meeting-processing-complete",
                serde_json::json!({ "meetingId": job.meeting_id, "kind": job.kind }),
            );
        }
        Ok(())
    }
    .await;
    if let Err(error) = result {
        if let Some(state) = app.try_state::<AppState>() {
            let current = get_job(state.db_manager.pool(), &id).await.ok();
            if error.to_string() == "processing-interrupted"
                && current
                    .as_ref()
                    .is_some_and(|job| job.status == "processing")
            {
                let progress = current.as_ref().map_or(0, |job| job.progress);
                let _ = set_state(
                    state.db_manager.pool(),
                    &id,
                    "pending",
                    progress,
                    None,
                    None,
                )
                .await;
                emit_job(&app, &id).await;
            }
            if current
                .as_ref()
                .is_none_or(|job| !matches!(job.status.as_str(), "cancelled" | "paused"))
                && error.to_string() != "processing-interrupted"
            {
                let _ = set_state(
                    state.db_manager.pool(),
                    &id,
                    "failed",
                    current.map_or(0, |job| job.progress),
                    Some(&error.to_string()),
                    None,
                )
                .await;
                emit_job(&app, &id).await;
            }
        }
    }
    RUNNING_JOBS
        .lock()
        .expect("running jobs lock poisoned")
        .remove(&id);
    STOP_REQUESTED
        .lock()
        .expect("stop requests lock poisoned")
        .remove(&id);
}

pub async fn dispatch_pending<R: Runtime>(app: AppHandle<R>) {
    if DISPATCHER_RUNNING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    tauri::async_runtime::spawn(async move {
        loop {
            let Some(state) = app.try_state::<AppState>() else {
                break;
            };
            if crate::audio::recording_commands::is_recording().await {
                break;
            }
            if RECORDING_PRIORITY_REQUESTED.load(Ordering::SeqCst) {
                break;
            }
            for id in cancel_jobs_with_failed_dependencies(state.db_manager.pool())
                .await
                .unwrap_or_default()
            {
                emit_job(&app, &id).await;
            }
            let next: Option<String> = sqlx::query_scalar(
                "SELECT j.id FROM meeting_processing_jobs j LEFT JOIN meeting_processing_jobs d ON d.id = j.depends_on
                 WHERE j.status = 'pending' AND (j.depends_on IS NULL OR d.status = 'completed')
                 ORDER BY j.automatic ASC, j.created_at ASC LIMIT 1",
            ).fetch_optional(state.db_manager.pool()).await.ok().flatten();
            let Some(id) = next else {
                break;
            };
            if let Ok(job) = get_job(state.db_manager.pool(), &id).await {
                if job.automatic && running_on_battery() {
                    if let Ok(resolved) =
                        serde_json::from_str::<ResolvedPipeline>(&job.config_snapshot)
                    {
                        if !resolved
                            .runtime_config()
                            .resources
                            .run_automatic_jobs_on_battery
                        {
                            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                            continue;
                        }
                    }
                }
            }
            run_job(app.clone(), id).await;
        }
        DISPATCHER_RUNNING.store(false, Ordering::SeqCst);
    });
}

fn running_on_battery() -> bool {
    #[cfg(target_os = "macos")]
    {
        return std::process::Command::new("pmset")
            .args(["-g", "batt"])
            .output()
            .ok()
            .is_some_and(|output| {
                String::from_utf8_lossy(&output.stdout).contains("Battery Power")
            });
    }
    #[cfg(target_os = "linux")]
    {
        return std::fs::read_to_string("/sys/class/power_supply/AC/online")
            .ok()
            .is_some_and(|value| value.trim() == "0");
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    false
}

pub async fn recover_jobs<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    let Some(state) = app.try_state::<AppState>() else {
        return Ok(());
    };
    recover_interrupted_jobs(state.db_manager.pool()).await?;
    dispatch_pending(app).await;
    Ok(())
}

async fn recover_interrupted_jobs(pool: &SqlitePool) -> Result<u64> {
    let result = sqlx::query(
        "UPDATE meeting_processing_jobs SET status = 'pending', updated_at = ? WHERE status = 'processing'",
    )
    .bind(Utc::now().to_rfc3339())
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

async fn cancel_jobs_with_failed_dependencies(pool: &SqlitePool) -> Result<Vec<String>> {
    let blocked: Vec<String> = sqlx::query_scalar(
        "SELECT j.id FROM meeting_processing_jobs j
         JOIN meeting_processing_jobs d ON d.id = j.depends_on
         WHERE j.status = 'pending' AND d.status IN ('failed', 'cancelled')",
    )
    .fetch_all(pool)
    .await?;
    let mut cancelled = Vec::with_capacity(blocked.len());
    for id in blocked {
        let job = get_job(pool, &id).await?;
        set_state(
            pool,
            &id,
            "cancelled",
            job.progress,
            Some("A required processing job did not complete"),
            job.metrics.as_deref(),
        )
        .await?;
        cancelled.push(id);
    }
    Ok(cancelled)
}

#[tauri::command]
pub async fn processing_enqueue_meeting_jobs<R: Runtime>(
    app: AppHandle<R>,
    request: EnqueueMeetingJobsRequest,
) -> Result<Vec<MeetingProcessingJob>, String> {
    let state = app.state::<AppState>();
    let beta = crate::pipeline::load_beta(&app).map_err(|error| error.to_string())?;
    if request.kind.is_some() && !beta.import_and_retranscribe {
        return Err("Meeting reprocessing is disabled in Beta settings".into());
    }
    if request
        .model
        .as_deref()
        .is_some_and(crate::pipeline::is_experimental_model)
        && !beta.experimental_asr_models
    {
        return Err("Experimental ASR models are disabled in Beta settings".into());
    }
    let mut config = crate::pipeline::load_config(&app).map_err(|error| error.to_string())?;
    if request.kind.as_deref() == Some("asr_recompute") {
        config.preset = crate::pipeline::PipelinePreset::Custom;
        config.post_meeting_asr.policy = PostMeetingPolicy::Manual;
        config.post_meeting_asr.provider = request
            .provider
            .clone()
            .or(config.post_meeting_asr.provider);
        config.post_meeting_asr.model = request.model.clone().or(config.post_meeting_asr.model);
        if let Some(language) = request.language.clone() {
            config.finalized.language = language;
        }
        if request.speaker_count.is_some() {
            config.speaker.speaker_count = request.speaker_count;
        }
        if let Some(enabled) = request.speaker_refinement {
            config.speaker.refinement = if enabled {
                SpeakerRefinementPolicy::Manual
            } else {
                SpeakerRefinementPolicy::Off
            };
        }
        if let Some(mode) = request.resource_mode {
            config.resources.mode = mode;
        }
    }
    let mut request_beta = beta.clone();
    if request.kind.is_some() {
        request_beta.custom_transcription_pipelines = true;
    }
    let resolved = crate::pipeline::resolve_for_app_with_beta(&app, config, &request_beta)
        .await
        .map_err(|error| error.to_string())?;
    if request.kind.as_deref() == Some("speaker_refinement") && !resolved.speaker_refinement_enabled
    {
        return Err("Speaker refinement is unavailable for this configuration".into());
    }
    let snapshot = serde_json::to_string(&resolved).map_err(|error| error.to_string())?;
    let mut ids = Vec::new();
    let force = request.kind.as_deref();
    if force == Some("asr_recompute")
        || (force.is_none()
            && resolved.runtime_config().post_meeting_asr.policy == PostMeetingPolicy::Auto)
    {
        ids.push(
            insert_job(
                state.db_manager.pool(),
                &request.meeting_id,
                "asr_recompute",
                &snapshot,
                None,
                force.is_none(),
            )
            .await
            .map_err(|error| error.to_string())?,
        );
    }
    if force == Some("speaker_refinement")
        || (force == Some("asr_recompute") && resolved.speaker_refinement_enabled)
        || (force.is_none()
            && resolved.runtime_config().speaker.refinement
                == SpeakerRefinementPolicy::BackgroundAuto
            && resolved.speaker_refinement_enabled)
    {
        let dependency = ids.last().map(String::as_str);
        ids.push(
            insert_job(
                state.db_manager.pool(),
                &request.meeting_id,
                "speaker_refinement",
                &snapshot,
                dependency,
                force.is_none(),
            )
            .await
            .map_err(|error| error.to_string())?,
        );
    }
    let mut jobs = Vec::new();
    for id in &ids {
        jobs.push(
            get_job(state.db_manager.pool(), id)
                .await
                .map_err(|error| error.to_string())?,
        );
    }
    dispatch_pending(app).await;
    Ok(jobs)
}

#[tauri::command]
pub async fn processing_list_jobs<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Vec<MeetingProcessingJob>, String> {
    let state = app.state::<AppState>();
    sqlx::query_as("SELECT * FROM meeting_processing_jobs ORDER BY created_at DESC")
        .fetch_all(state.db_manager.pool())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn processing_cancel_job<R: Runtime>(
    app: AppHandle<R>,
    job_id: String,
) -> Result<(), String> {
    STOP_REQUESTED
        .lock()
        .expect("stop requests lock poisoned")
        .insert(job_id.clone());
    let state = app.state::<AppState>();
    set_state(state.db_manager.pool(), &job_id, "cancelled", 0, None, None)
        .await
        .map_err(|error| error.to_string())?;
    emit_job(&app, &job_id).await;
    if !RUNNING_JOBS
        .lock()
        .expect("running jobs lock poisoned")
        .contains(&job_id)
    {
        STOP_REQUESTED
            .lock()
            .expect("stop requests lock poisoned")
            .remove(&job_id);
    }
    Ok(())
}

#[tauri::command]
pub async fn processing_pause_job<R: Runtime>(
    app: AppHandle<R>,
    job_id: String,
) -> Result<(), String> {
    STOP_REQUESTED
        .lock()
        .expect("stop requests lock poisoned")
        .insert(job_id.clone());
    let state = app.state::<AppState>();
    let job = get_job(state.db_manager.pool(), &job_id)
        .await
        .map_err(|error| error.to_string())?;
    set_state(
        state.db_manager.pool(),
        &job_id,
        "paused",
        job.progress,
        None,
        job.metrics.as_deref(),
    )
    .await
    .map_err(|error| error.to_string())?;
    emit_job(&app, &job_id).await;
    Ok(())
}

#[tauri::command]
pub async fn processing_resume_job<R: Runtime>(
    app: AppHandle<R>,
    job_id: String,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    resume_or_retry_job(state.db_manager.pool(), &job_id)
        .await
        .map_err(|error| error.to_string())?;
    STOP_REQUESTED
        .lock()
        .expect("stop requests lock poisoned")
        .remove(&job_id);
    emit_job(&app, &job_id).await;
    dispatch_pending(app).await;
    Ok(())
}

async fn resume_or_retry_job(pool: &SqlitePool, job_id: &str) -> Result<()> {
    let job = get_job(pool, job_id).await?;
    if !matches!(job.status.as_str(), "paused" | "failed") {
        return Err(anyhow!("Only paused or failed jobs can be resumed"));
    }
    set_state(
        pool,
        job_id,
        "pending",
        job.progress,
        None,
        job.metrics.as_deref(),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn insert_meeting(pool: &SqlitePool, id: &str) {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO meetings (id, title, created_at, updated_at) VALUES (?, 'test', ?, ?)",
        )
        .bind(id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .unwrap();
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn active_job_kind_is_unique(pool: SqlitePool) {
        insert_meeting(&pool, "m1").await;
        insert_job(&pool, "m1", "speaker_refinement", "{}", None, true)
            .await
            .unwrap();
        assert!(
            insert_job(&pool, "m1", "speaker_refinement", "{}", None, true)
                .await
                .is_err()
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn deleting_a_meeting_cascades_to_processing_jobs(pool: SqlitePool) {
        insert_meeting(&pool, "m-cascade").await;
        insert_job(&pool, "m-cascade", "asr_recompute", "{}", None, true)
            .await
            .unwrap();
        sqlx::query("DELETE FROM meetings WHERE id = 'm-cascade'")
            .execute(&pool)
            .await
            .unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM meeting_processing_jobs WHERE meeting_id = 'm-cascade'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn interrupted_jobs_resume_from_their_persisted_checkpoint(pool: SqlitePool) {
        insert_meeting(&pool, "m-recover").await;
        let id = insert_job(&pool, "m-recover", "asr_recompute", "{}", None, true)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE meeting_processing_jobs SET status = 'processing', progress = 47, checkpoint = '{\"nextWindowStart\":300}' WHERE id = ?",
        )
        .bind(&id)
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(recover_interrupted_jobs(&pool).await.unwrap(), 1);
        let recovered = get_job(&pool, &id).await.unwrap();
        assert_eq!(recovered.status, "pending");
        assert_eq!(recovered.progress, 47);
        assert_eq!(
            recovered.checkpoint.as_deref(),
            Some("{\"nextWindowStart\":300}")
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn failed_dependency_cancels_its_pending_successor(pool: SqlitePool) {
        insert_meeting(&pool, "m-dependency").await;
        let first = insert_job(&pool, "m-dependency", "asr_recompute", "{}", None, true)
            .await
            .unwrap();
        let second = insert_job(
            &pool,
            "m-dependency",
            "speaker_refinement",
            "{}",
            Some(&first),
            true,
        )
        .await
        .unwrap();
        set_state(&pool, &first, "failed", 20, Some("test failure"), None)
            .await
            .unwrap();

        let cancelled = cancel_jobs_with_failed_dependencies(&pool).await.unwrap();
        assert_eq!(cancelled, vec![second.clone()]);
        let successor = get_job(&pool, &second).await.unwrap();
        assert_eq!(successor.status, "cancelled");
        assert!(successor.error.is_some());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn failed_job_retry_keeps_checkpoint_and_progress(pool: SqlitePool) {
        insert_meeting(&pool, "m-retry").await;
        let id = insert_job(&pool, "m-retry", "asr_recompute", "{}", None, false)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE meeting_processing_jobs SET status = 'failed', progress = 62,
             checkpoint = '{\"nextWindowStart\":600}', error = 'temporary failure',
             completed_at = 'done' WHERE id = ?",
        )
        .bind(&id)
        .execute(&pool)
        .await
        .unwrap();

        resume_or_retry_job(&pool, &id).await.unwrap();
        let retried = get_job(&pool, &id).await.unwrap();
        assert_eq!(retried.status, "pending");
        assert_eq!(retried.progress, 62);
        assert_eq!(
            retried.checkpoint.as_deref(),
            Some("{\"nextWindowStart\":600}")
        );
        assert!(retried.error.is_none());
        assert!(retried.completed_at.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn failed_transcript_replacement_rolls_back_the_original_rows(pool: SqlitePool) {
        insert_meeting(&pool, "m-rollback").await;
        sqlx::query(
            "INSERT INTO transcripts (id, meeting_id, transcript, timestamp) VALUES ('old', 'm-rollback', 'original', 'now')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TRIGGER reject_replacement BEFORE INSERT ON transcripts
             WHEN NEW.transcript = 'replacement'
             BEGIN SELECT RAISE(ABORT, 'forced replacement failure'); END",
        )
        .execute(&pool)
        .await
        .unwrap();
        let segments = vec![crate::api::TranscriptSegment {
            id: "new".into(),
            text: "replacement".into(),
            timestamp: "later".into(),
            audio_start_time: Some(0.0),
            audio_end_time: Some(1.0),
            duration: Some(1.0),
            speaker: None,
        }];

        assert!(replace_transcript_rows(&pool, "m-rollback", &segments)
            .await
            .is_err());
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT id, transcript FROM transcripts WHERE meeting_id = 'm-rollback'",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(rows, vec![("old".into(), "original".into())]);
    }

    #[test]
    fn long_meetings_are_planned_with_bounded_audio_windows() {
        for duration in [3_600.0, 7_200.0] {
            let windows = plan_recompute_windows(duration);
            assert!(!windows.is_empty());
            assert!(windows
                .iter()
                .all(|(_, length)| *length <= RECOMPUTE_WINDOW_SECONDS));
            assert!(
                (windows.iter().map(|(_, length)| length).sum::<f64>() - duration).abs() < 0.001
            );
            let one_window_audio_bytes = windows
                .iter()
                .map(|(_, length)| (*length * 16_000.0) as usize * std::mem::size_of::<f32>())
                .max()
                .unwrap();
            let peak_decoded_audio_bytes = one_window_audio_bytes * MAX_RESIDENT_AUDIO_WINDOWS;
            assert!(peak_decoded_audio_bytes <= 40 * 1024 * 1024);
        }
    }

    #[test]
    #[ignore = "requires MINGTILY_LONG_AUDIO_FIXTURE and MINGTILY_LONG_AUDIO_DURATION_SECONDS"]
    fn long_audio_fixture_is_actually_decoded_with_bounded_window_buffers() {
        let path = PathBuf::from(std::env::var("MINGTILY_LONG_AUDIO_FIXTURE").unwrap());
        let duration: f64 = std::env::var("MINGTILY_LONG_AUDIO_DURATION_SECONDS")
            .unwrap()
            .parse()
            .unwrap();
        let mut peak_samples = 0usize;
        for (start, length) in plan_recompute_windows(duration) {
            let samples = decode_audio_range_to_whisper_format(&path, start, length).unwrap();
            peak_samples = peak_samples.max(samples.len());
            assert!(samples.len() <= (RECOMPUTE_WINDOW_SECONDS * 16_000.0) as usize);
        }
        assert!(peak_samples > 0);
        let peak_resident_audio_bytes =
            peak_samples * std::mem::size_of::<f32>() * MAX_RESIDENT_AUDIO_WINDOWS;
        assert!(peak_resident_audio_bytes <= 40 * 1024 * 1024);
    }
}
