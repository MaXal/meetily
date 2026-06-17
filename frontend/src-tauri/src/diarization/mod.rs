// diarization/mod.rs
//
// Speaker identification (diarization) for the live transcription pipeline.
// Rust-native and fully local: WeSpeaker ONNX embeddings (via ort, the same
// runtime parakeet uses) + online cosine clustering. See docs in each module.
//
// Integration model for this fork: the audio pipeline runs per-source VAD, so
// the microphone (the local user, "You") is never diarized. Only system-audio
// speech segments are routed here, where they are split into "Speaker 1/2/…".
// The session lives for exactly one recording: it is created at the start of
// the transcription task and cleared when that task finishes.

pub mod clustering;
pub mod commands;
pub mod embedding;
pub mod fbank;
pub mod models;
pub mod session;

pub use session::DiarizationSession;

use std::sync::Mutex;
use tauri::{AppHandle, Manager, Runtime};

/// Active diarization session for the current recording, or None when the
/// feature is disabled / the model is missing / a recording is not running.
/// Accessed synchronously from the (serial) transcription worker — never held
/// across an await.
static DIARIZATION_SESSION: Mutex<Option<DiarizationSession>> = Mutex::new(None);

/// Initialize the diarization session for a recording if the feature is
/// enabled and the embedding model is present. Any failure degrades to "no
/// diarization" (system segments keep the legacy "Others" label) and never
/// affects transcription.
pub async fn maybe_init_session<R: Runtime>(app: &AppHandle<R>) {
    // Start from a clean slate for every recording.
    clear_session();

    let enabled = {
        let state = app.state::<crate::state::AppState>();
        commands::is_enabled(state.db_manager.pool()).await
    };
    if !enabled {
        log::info!("🎙️ Speaker identification disabled - segments will use source labels");
        return;
    }

    if !models::is_embedding_model_present(app) {
        log::warn!(
            "🎙️ Speaker identification enabled but embedding model not downloaded - labels disabled"
        );
        return;
    }

    let model_path = match models::embedding_model_path(app) {
        Ok(p) => p,
        Err(e) => {
            log::warn!("🎙️ Could not resolve embedding model path: {}", e);
            return;
        }
    };

    match DiarizationSession::new(&model_path) {
        Ok(session) => {
            *DIARIZATION_SESSION.lock().unwrap() = Some(session);
            log::info!("🎙️ Speaker identification active for this recording");
        }
        Err(e) => {
            log::warn!("🎙️ Failed to initialize diarization session: {}", e);
        }
    }
}

/// Cheap check for whether a diarization session is active, so hot-path
/// callers can skip copying/resampling audio when the feature is off.
pub fn is_active() -> bool {
    DIARIZATION_SESSION.lock().unwrap().is_some()
}

/// Label a system-audio speech segment (16kHz mono). Returns None when no
/// session is active, so the caller can fall back to the legacy label.
pub fn label_system_segment(samples_16k: &[f32]) -> Option<String> {
    let mut guard = DIARIZATION_SESSION.lock().unwrap();
    guard.as_mut().and_then(|s| s.label_segment(samples_16k))
}

/// Drop the diarization session at the end of a recording.
pub fn clear_session() {
    *DIARIZATION_SESSION.lock().unwrap() = None;
}
