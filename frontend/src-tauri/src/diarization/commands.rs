// diarization/commands.rs
//
// Tauri command surface for speaker identification: feature toggle
// (persisted in the diarization_settings table), model status, and
// on-demand model download.

use crate::state::AppState;
use sqlx::SqlitePool;
use tauri::{command, AppHandle, Runtime};

/// Whether speaker identification is enabled (persisted setting, default off).
pub async fn is_enabled(pool: &SqlitePool) -> bool {
    sqlx::query_scalar::<_, i64>("SELECT enabled FROM diarization_settings WHERE id = '1'")
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|v| v != 0)
        .unwrap_or(false)
}

#[command]
pub async fn diarization_get_status<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let enabled = is_enabled(state.db_manager.pool()).await;
    let model_present = super::models::is_embedding_model_present(&app);
    Ok(serde_json::json!({
        "enabled": enabled,
        "model_present": model_present,
        "model_filename": super::models::EMBEDDING_MODEL_FILENAME,
    }))
}

#[command]
pub async fn diarization_set_enabled(
    state: tauri::State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    sqlx::query(
        r#"
        INSERT INTO diarization_settings (id, enabled) VALUES ('1', $1)
        ON CONFLICT(id) DO UPDATE SET enabled = excluded.enabled
        "#,
    )
    .bind(enabled as i64)
    .execute(state.db_manager.pool())
    .await
    .map_err(|e| format!("Failed to save diarization setting: {}", e))?;
    log::info!(
        "Speaker identification {}",
        if enabled { "enabled" } else { "disabled" }
    );
    Ok(())
}

#[command]
pub async fn diarization_download_model<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    super::models::download_embedding_model(&app).await
}
