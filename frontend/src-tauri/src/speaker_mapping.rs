use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::collections::{HashMap, HashSet};

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
    #[serde(default)]
    segment_speakers: HashMap<String, String>,
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

pub(crate) async fn load_speaker_overrides(
    pool: &SqlitePool,
    meeting_id: &str,
) -> Result<HashMap<String, String>, sqlx::Error> {
    let raw: Option<String> =
        sqlx::query_scalar("SELECT mapping_json FROM meeting_speaker_maps WHERE meeting_id = ?")
            .bind(meeting_id)
            .fetch_optional(pool)
            .await?;
    Ok(raw
        .and_then(|value| serde_json::from_str::<StoredSpeakerMap>(&value).ok())
        .map(|mapping| mapping.segment_speakers)
        .unwrap_or_default())
}

pub(crate) async fn save_speaker_overrides(
    pool: &SqlitePool,
    meeting_id: &str,
    overrides: HashMap<String, String>,
) -> Result<(), String> {
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| format!("Failed to start speaker refinement transaction: {error}"))?;
    let row =
        sqlx::query("SELECT revision, mapping_json FROM meeting_speaker_maps WHERE meeting_id = ?")
            .bind(meeting_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|error| format!("Failed to load speaker refinement overlay: {error}"))?;
    let (revision, mut mapping) = if let Some(row) = row {
        let revision = row.get::<i64, _>("revision");
        let raw = row.get::<String, _>("mapping_json");
        let mapping = serde_json::from_str::<StoredSpeakerMap>(&raw)
            .map_err(|error| format!("Stored speaker map is invalid: {error}"))?;
        (revision, mapping)
    } else {
        (
            0,
            StoredSpeakerMap {
                schema_version: 2,
                participants: Vec::new(),
                segment_speakers: HashMap::new(),
            },
        )
    };
    mapping.schema_version = 2;
    mapping.segment_speakers = overrides;
    let mapping_json = serde_json::to_string(&mapping)
        .map_err(|error| format!("Failed to serialize speaker refinement overlay: {error}"))?;
    let next_revision = revision + 1;
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
    .bind(meeting_id)
    .bind(next_revision)
    .bind(mapping_json)
    .bind(Utc::now().to_rfc3339())
    .bind(revision)
    .execute(&mut *transaction)
    .await
    .map_err(|error| format!("Failed to save speaker refinement overlay: {error}"))?;
    if result.rows_affected() != 1 {
        return Err("Speaker map changed while refinement was completing; retry the job".into());
    }
    transaction
        .commit()
        .await
        .map_err(|error| format!("Failed to commit speaker refinement overlay: {error}"))
}

async fn load_stats(pool: &SqlitePool, meeting_id: &str) -> Result<Vec<SpeakerStat>, String> {
    let rows = sqlx::query(
        "SELECT id, speaker, COALESCE(duration, 0.0) AS duration, transcript
         FROM transcripts WHERE meeting_id = ? ORDER BY COALESCE(audio_start_time, 0), id",
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|error| format!("Failed to load speaker statistics: {error}"))?;
    let overrides = load_speaker_overrides(pool, meeting_id)
        .await
        .map_err(|error| format!("Failed to load speaker refinement overlay: {error}"))?;
    let mut order = Vec::<String>::new();
    let mut stats = HashMap::<String, SpeakerStat>::new();
    for row in rows {
        let id = row.get::<String, _>("id");
        let speaker = overrides
            .get(&id)
            .cloned()
            .or_else(|| row.get::<Option<String>, _>("speaker"));
        let Some(speaker) = speaker.filter(|value| !value.trim().is_empty()) else {
            continue;
        };
        if !stats.contains_key(&speaker) {
            order.push(speaker.clone());
            stats.insert(
                speaker.clone(),
                SpeakerStat {
                    source_speaker: speaker.clone(),
                    segment_count: 0,
                    duration: 0.0,
                    sample: row.get::<String, _>("transcript"),
                },
            );
        }
        if let Some(stat) = stats.get_mut(&speaker) {
            stat.segment_count += 1;
            stat.duration += row.get::<f64, _>("duration");
        }
    }
    Ok(order
        .into_iter()
        .filter_map(|speaker| stats.remove(&speaker))
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

    let available = load_stats(state.db_manager.pool(), &meeting_id)
        .await?
        .into_iter()
        .map(|stat| stat.source_speaker)
        .collect::<HashSet<_>>();
    let normalized = validate_participants(participants, &available)?;

    let existing_segment_speakers = sqlx::query_scalar::<_, String>(
        "SELECT mapping_json FROM meeting_speaker_maps WHERE meeting_id = ?",
    )
    .bind(&meeting_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|error| format!("Failed to load speaker refinement overlay: {error}"))?
    .and_then(|raw| serde_json::from_str::<StoredSpeakerMap>(&raw).ok())
    .map(|mapping| mapping.segment_speakers)
    .unwrap_or_default();

    let mapping = StoredSpeakerMap {
        schema_version: 2,
        participants: normalized,
        segment_speakers: existing_segment_speakers,
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
    use crate::database::repositories::meeting::MeetingsRepository;

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

    #[sqlx::test(migrations = "./migrations")]
    async fn refinement_overlay_preserves_raw_speaker_and_changes_reads(pool: SqlitePool) {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO meetings (id, title, created_at, updated_at) VALUES ('m1', 'test', ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO transcripts (id, meeting_id, transcript, timestamp, speaker)
             VALUES ('t1', 'm1', 'hello', '12:00:00', 'speaker_00')",
        )
        .execute(&pool)
        .await
        .unwrap();

        save_speaker_overrides(
            &pool,
            "m1",
            HashMap::from([("t1".to_string(), "speaker_01".to_string())]),
        )
        .await
        .unwrap();

        let raw: String = sqlx::query_scalar("SELECT speaker FROM transcripts WHERE id = 't1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(raw, "speaker_00");
        let effective = MeetingsRepository::get_meeting_transcripts(&pool, "m1")
            .await
            .unwrap();
        assert_eq!(effective[0].speaker.as_deref(), Some("speaker_01"));
    }
}
