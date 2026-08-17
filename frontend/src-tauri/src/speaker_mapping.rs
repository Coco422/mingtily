use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerParticipant {
    pub id: String,
    pub name: String,
    pub source_speakers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredSpeakerMap {
    schema_version: u32,
    participants: Vec<SpeakerParticipant>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerStat {
    pub source_speaker: String,
    pub segment_count: i64,
    pub duration: f64,
    pub sample: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSpeakerMapResponse {
    pub meeting_id: String,
    pub revision: i64,
    pub participants: Vec<SpeakerParticipant>,
    pub speakers: Vec<SpeakerStat>,
}

async fn load_map(
    pool: &SqlitePool,
    meeting_id: &str,
) -> Result<(i64, Vec<SpeakerParticipant>), String> {
    let row =
        sqlx::query("SELECT revision, mapping_json FROM meeting_speaker_maps WHERE meeting_id = ?")
            .bind(meeting_id)
            .fetch_optional(pool)
            .await
            .map_err(|error| format!("Failed to load speaker map: {error}"))?;
    let Some(row) = row else {
        return Ok((0, Vec::new()));
    };
    let revision = row.get::<i64, _>("revision");
    let raw = row.get::<String, _>("mapping_json");
    let mapping: StoredSpeakerMap = serde_json::from_str(&raw)
        .map_err(|error| format!("Stored speaker map is invalid: {error}"))?;
    Ok((revision, mapping.participants))
}

async fn load_stats(pool: &SqlitePool, meeting_id: &str) -> Result<Vec<SpeakerStat>, String> {
    let rows = sqlx::query(
        r#"
        SELECT speaker,
               COUNT(*) AS segment_count,
               COALESCE(SUM(COALESCE(duration, 0.0)), 0.0) AS total_duration,
               MIN(transcript) AS sample
        FROM transcripts
        WHERE meeting_id = ? AND speaker IS NOT NULL AND trim(speaker) <> ''
        GROUP BY speaker
        ORDER BY MIN(COALESCE(audio_start_time, 0)), speaker
        "#,
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|error| format!("Failed to load speaker statistics: {error}"))?;

    Ok(rows
        .into_iter()
        .map(|row| SpeakerStat {
            source_speaker: row.get("speaker"),
            segment_count: row.get("segment_count"),
            duration: row.get("total_duration"),
            sample: row.get::<Option<String>, _>("sample").unwrap_or_default(),
        })
        .collect())
}

async fn response(
    pool: &SqlitePool,
    meeting_id: &str,
) -> Result<MeetingSpeakerMapResponse, String> {
    let (revision, participants) = load_map(pool, meeting_id).await?;
    let speakers = load_stats(pool, meeting_id).await?;
    Ok(MeetingSpeakerMapResponse {
        meeting_id: meeting_id.to_string(),
        revision,
        participants,
        speakers,
    })
}

fn validate_participants(
    participants: Vec<SpeakerParticipant>,
    available: &HashSet<String>,
) -> Result<Vec<SpeakerParticipant>, String> {
    let mut participant_ids = HashSet::new();
    let mut assigned_speakers = HashSet::new();
    let mut normalized = Vec::with_capacity(participants.len());
    for participant in participants {
        uuid::Uuid::parse_str(&participant.id)
            .map_err(|_| format!("Participant ID '{}' is not a valid UUID", participant.id))?;
        if !participant_ids.insert(participant.id.clone()) {
            return Err(format!("Participant ID '{}' is duplicated", participant.id));
        }
        let name = participant.name.trim().to_string();
        if name.is_empty() {
            return Err("Participant names cannot be empty".to_string());
        }
        let mut sources = Vec::new();
        for source in participant.source_speakers {
            let source = source.trim().to_string();
            if !available.contains(&source) {
                return Err(format!(
                    "Speaker label '{source}' does not exist in this meeting"
                ));
            }
            if !assigned_speakers.insert(source.clone()) {
                return Err(format!(
                    "Speaker label '{source}' belongs to more than one participant"
                ));
            }
            sources.push(source);
        }
        if sources.is_empty() {
            return Err(format!(
                "Participant '{name}' must contain at least one speaker label"
            ));
        }
        normalized.push(SpeakerParticipant {
            id: participant.id,
            name,
            source_speakers: sources,
        });
    }
    Ok(normalized)
}

#[tauri::command]
pub async fn api_get_meeting_speaker_map(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<MeetingSpeakerMapResponse, String> {
    if meeting_id.trim().is_empty() {
        return Err("meeting_id cannot be empty".to_string());
    }
    response(state.db_manager.pool(), &meeting_id).await
}

#[tauri::command]
pub async fn api_save_meeting_speaker_map(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    expected_revision: i64,
    participants: Vec<SpeakerParticipant>,
) -> Result<MeetingSpeakerMapResponse, String> {
    if meeting_id.trim().is_empty() || expected_revision < 0 {
        return Err("Invalid meeting ID or revision".to_string());
    }

    let mut transaction = state
        .db_manager
        .pool()
        .begin()
        .await
        .map_err(|error| format!("Failed to start speaker map transaction: {error}"))?;
    let current_revision = sqlx::query_scalar::<_, i64>(
        "SELECT revision FROM meeting_speaker_maps WHERE meeting_id = ?",
    )
    .bind(&meeting_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|error| format!("Failed to inspect speaker map revision: {error}"))?;
    if current_revision.unwrap_or(0) != expected_revision {
        return Err(
            "Speaker map changed in another view. Refresh before saving again.".to_string(),
        );
    }

    let available = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT speaker FROM transcripts WHERE meeting_id = ? AND speaker IS NOT NULL AND trim(speaker) <> ''",
    )
        .bind(&meeting_id)
        .fetch_all(&mut *transaction)
        .await
        .map_err(|error| format!("Failed to inspect meeting speaker labels: {error}"))?
        .into_iter()
        .collect::<HashSet<_>>();
    let normalized = validate_participants(participants, &available)?;

    let mapping = StoredSpeakerMap {
        schema_version: 1,
        participants: normalized,
    };
    let mapping_json = serde_json::to_string(&mapping)
        .map_err(|error| format!("Failed to serialize speaker map: {error}"))?;
    let next_revision = expected_revision + 1;
    let result = sqlx::query(
        r#"
        INSERT INTO meeting_speaker_maps (meeting_id, revision, mapping_json, updated_at)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(meeting_id) DO UPDATE SET
            revision = excluded.revision,
            mapping_json = excluded.mapping_json,
            updated_at = excluded.updated_at
        WHERE meeting_speaker_maps.revision = ?
        "#,
    )
    .bind(&meeting_id)
    .bind(next_revision)
    .bind(mapping_json)
    .bind(Utc::now().to_rfc3339())
    .bind(expected_revision)
    .execute(&mut *transaction)
    .await
    .map_err(|error| format!("Failed to save speaker map: {error}"))?;

    if result.rows_affected() != 1 {
        return Err(
            "Speaker map changed in another view. Refresh before saving again.".to_string(),
        );
    }
    transaction
        .commit()
        .await
        .map_err(|error| format!("Failed to commit speaker map: {error}"))?;
    response(state.db_manager.pool(), &meeting_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn participant(id: &str, name: &str, sources: &[&str]) -> SpeakerParticipant {
        SpeakerParticipant {
            id: id.to_string(),
            name: name.to_string(),
            source_speakers: sources.iter().map(|source| source.to_string()).collect(),
        }
    }

    #[test]
    fn source_speaker_can_only_belong_to_one_participant() {
        let available = HashSet::from(["speaker_00".to_string()]);
        let result = validate_participants(
            vec![
                participant(
                    "25c720ea-3d8d-4c52-ae19-8bfe3e462e95",
                    "Zhang",
                    &["speaker_00"],
                ),
                participant(
                    "28806c69-73e8-472d-a580-41a84ef1df5f",
                    "Li",
                    &["speaker_00"],
                ),
            ],
            &available,
        );
        assert!(result.unwrap_err().contains("more than one participant"));
    }

    #[test]
    fn names_are_trimmed_and_same_names_are_allowed() {
        let available = HashSet::from(["speaker_00".to_string(), "speaker_01".to_string()]);
        let result = validate_participants(
            vec![
                participant(
                    "25c720ea-3d8d-4c52-ae19-8bfe3e462e95",
                    " Zhang ",
                    &["speaker_00"],
                ),
                participant(
                    "28806c69-73e8-472d-a580-41a84ef1df5f",
                    "Zhang",
                    &["speaker_01"],
                ),
            ],
            &available,
        )
        .unwrap();
        assert_eq!(result[0].name, "Zhang");
        assert_eq!(result[1].name, "Zhang");
    }
}
