// audio/transcription/mod.rs
//
// Transcription module: Provider abstraction, engine management, and worker pool.

pub mod engine;
pub mod parakeet_provider;
pub mod provider;
pub mod terminology;
pub mod whisper_provider;
pub mod worker;

// Re-export commonly used types
pub use engine::{
    get_or_init_transcription_engine, load_transcription_provider,
    resolve_configured_streaming_model, resolve_transcription_selection,
    validate_transcription_model_ready, TranscriptionSelection,
};
pub use parakeet_provider::ParakeetProvider;
pub use provider::{TranscriptResult, TranscriptionError, TranscriptionProvider};
pub use whisper_provider::WhisperProvider;
pub use worker::{reset_speech_detected_flag, start_transcription_task, TranscriptUpdate};
