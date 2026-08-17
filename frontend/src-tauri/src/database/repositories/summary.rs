use crate::database::models::SummaryProcess;
use chrono::Utc;
use serde_json::Value;
use sqlx::SqlitePool;
use tracing::{error, info as log_info};

pub struct SummaryProcessesRepository;

impl SummaryProcessesRepository {
    /// Retrieves the current summary process state for a given meeting ID.
    pub async fn get_summary_data(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Option<SummaryProcess>, sqlx::Error> {
        sqlx::query_as::<_, SummaryProcess>("SELECT * FROM summary_processes WHERE meeting_id = ?")
            .bind(meeting_id)
            .fetch_optional(pool)
            .await
    }

    pub async fn update_meeting_summary(
        pool: &SqlitePool,
        meeting_id: &str,
        summary: &Value,
    ) -> Result<bool, sqlx::Error> {
        let mut transaction = pool.begin().await?;

        let meeting_exists: bool = sqlx::query("SELECT 1 FROM meetings WHERE id = ?")
            .bind(meeting_id)
            .fetch_optional(&mut *transaction)
            .await?
            .is_some();

        if !meeting_exists {
            log_info!(
                "Attempted to save summary for a non-existent meeting_id: {}",
                meeting_id
            );
            transaction.rollback().await?;
            return Ok(false);
        }

        let result_json = serde_json::to_string(summary);
        if result_json.is_err() {
            error!("Can't convert the json to string for saving to Database");
            transaction.rollback().await?;
            return Ok(false);
        }
        let now = Utc::now();

        sqlx::query("UPDATE summary_processes SET result = ?, updated_at = ? WHERE meeting_id = ?")
            .bind(&result_json.unwrap())
            .bind(now)
            .bind(meeting_id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query("UPDATE meetings SET updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(meeting_id)
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;

        log_info!(
            "Successfully updated summary and timestamp for meeting_id: {}",
            meeting_id
        );
        Ok(true)
    }

    pub async fn get_summary_data_for_meeting(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Option<SummaryProcess>, sqlx::Error> {
        sqlx::query_as::<_, SummaryProcess>(
            "SELECT p.* FROM summary_processes p JOIN transcript_chunks t ON p.meeting_id = t.meeting_id WHERE p.meeting_id = ?",
        )
        .bind(meeting_id)
        .fetch_optional(pool)
        .await
    }

    /// Atomically starts a summary job. Returns false when the same meeting already
    /// has a pending or processing job, so callers cannot overwrite its cancellation
    /// token or race its terminal database write.
    pub async fn try_start_process(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<bool, sqlx::Error> {
        log_info!(
            "Creating or resetting summary process for meeting_id: {}",
            meeting_id
        );
        let now = Utc::now();
        let result = sqlx::query(
            r#"
            INSERT INTO summary_processes (meeting_id, status, created_at, updated_at, start_time, result, error)
            VALUES (?, 'PENDING', ?, ?, ?, NULL, NULL)
            ON CONFLICT(meeting_id) DO UPDATE SET
                status = 'PENDING',
                updated_at = excluded.updated_at,
                start_time = excluded.start_time,
                end_time = NULL,
                result_backup = result,
                result_backup_timestamp = excluded.updated_at,
                result = result,
                error = NULL
            WHERE lower(COALESCE(summary_processes.status, '')) NOT IN ('pending', 'processing')
            "#
        )
        .bind(meeting_id)
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;
        log_info!(
            "Backed up existing summary before regeneration for meeting_id: {}",
            meeting_id
        );
        Ok(result.rows_affected() == 1)
    }

    pub async fn update_process_processing(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE summary_processes SET status = 'PROCESSING', updated_at = ? WHERE meeting_id = ? AND lower(status) = 'pending'",
        )
        .bind(Utc::now())
        .bind(meeting_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    /// Jobs cannot survive an application process restart. Restore any pre-regeneration
    /// result and make the interruption explicit instead of leaving an eternal spinner.
    pub async fn mark_incomplete_processes_interrupted(
        pool: &SqlitePool,
    ) -> Result<u64, sqlx::Error> {
        let now = Utc::now();
        let result = sqlx::query(
            r#"
            UPDATE summary_processes
            SET status = 'interrupted',
                error = 'Summary generation was interrupted when the application exited',
                updated_at = ?,
                end_time = ?,
                result = COALESCE(result_backup, result),
                result_backup = NULL,
                result_backup_timestamp = NULL
            WHERE lower(status) IN ('pending', 'processing')
            "#,
        )
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn update_process_completed(
        pool: &SqlitePool,
        meeting_id: &str,
        result: Value, // Keep this as Value to handle both old and new formats if needed
        chunk_count: i64,
        processing_time: f64,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        let result_str = serde_json::to_string(&result)
            .map_err(|e| sqlx::Error::Protocol(format!("Failed to serialize result: {}", e)))?;

        sqlx::query(
            r#"
            UPDATE summary_processes
            SET status = 'completed', result = ?, updated_at = ?, end_time = ?, chunk_count = ?, processing_time = ?, error = NULL, result_backup = NULL, result_backup_timestamp = NULL
            WHERE meeting_id = ?
            "#
        )
        .bind(result_str)
        .bind(now)
        .bind(now)
        .bind(chunk_count)
        .bind(processing_time)
        .bind(meeting_id)
        .execute(pool)
        .await?;
        log_info!(
            "Summary completed and backup cleared for meeting_id: {}",
            meeting_id
        );
        Ok(())
    }

    pub async fn update_process_failed(
        pool: &SqlitePool,
        meeting_id: &str,
        error: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();

        // Restore from backup if it exists, otherwise keep current result
        sqlx::query(
            r#"
            UPDATE summary_processes
            SET
                status = 'failed',
                error = ?,
                updated_at = ?,
                end_time = ?,
                result = COALESCE(result_backup, result),
                result_backup = NULL,
                result_backup_timestamp = NULL
            WHERE meeting_id = ?
            "#,
        )
        .bind(error)
        .bind(now)
        .bind(now)
        .bind(meeting_id)
        .execute(pool)
        .await?;
        log_info!(
            "Summary generation failed and backup restored for meeting_id: {}",
            meeting_id
        );
        Ok(())
    }

    pub async fn update_process_cancelled(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();

        // Restore from backup if it exists, otherwise keep current result
        sqlx::query(
            r#"
            UPDATE summary_processes
            SET
                status = 'cancelled',
                updated_at = ?,
                end_time = ?,
                error = 'Generation was cancelled by user',
                result = COALESCE(result_backup, result),
                result_backup = NULL,
                result_backup_timestamp = NULL
            WHERE meeting_id = ?
            "#,
        )
        .bind(now)
        .bind(now)
        .bind(meeting_id)
        .execute(pool)
        .await?;
        log_info!(
            "Marked summary process as cancelled and restored backup for meeting_id: {}",
            meeting_id
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            r#"
            CREATE TABLE summary_processes (
                meeting_id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                created_at TEXT,
                updated_at TEXT,
                start_time TEXT,
                end_time TEXT,
                result TEXT,
                error TEXT,
                result_backup TEXT,
                result_backup_timestamp TEXT,
                chunk_count INTEGER,
                processing_time REAL
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    #[tokio::test]
    async fn duplicate_active_jobs_are_rejected_atomically() {
        let pool = test_pool().await;
        assert!(
            SummaryProcessesRepository::try_start_process(&pool, "meeting-1")
                .await
                .unwrap()
        );
        assert!(
            !SummaryProcessesRepository::try_start_process(&pool, "meeting-1")
                .await
                .unwrap()
        );
        assert!(
            SummaryProcessesRepository::update_process_processing(&pool, "meeting-1")
                .await
                .unwrap()
        );
        assert!(
            !SummaryProcessesRepository::try_start_process(&pool, "meeting-1")
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn restart_recovery_restores_backup_and_marks_interrupted() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO summary_processes (meeting_id, status, result, result_backup) VALUES (?, 'PROCESSING', ?, ?)",
        )
        .bind("meeting-1")
        .bind("new result")
        .bind("old result")
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(
            SummaryProcessesRepository::mark_incomplete_processes_interrupted(&pool)
                .await
                .unwrap(),
            1
        );
        let row = sqlx::query_as::<_, (String, Option<String>, Option<String>, Option<String>)>(
            "SELECT status, result, result_backup, error FROM summary_processes WHERE meeting_id = ?",
        )
        .bind("meeting-1")
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, "interrupted");
        assert_eq!(row.1.as_deref(), Some("old result"));
        assert!(row.2.is_none());
        assert!(row.3.unwrap().contains("interrupted"));
    }

    #[tokio::test]
    async fn cancellation_restores_the_previous_summary() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO summary_processes (meeting_id, status, result, result_backup) VALUES (?, 'PROCESSING', ?, ?)",
        )
        .bind("meeting-1")
        .bind("new result")
        .bind("old result")
        .execute(&pool)
        .await
        .unwrap();

        SummaryProcessesRepository::update_process_cancelled(&pool, "meeting-1")
            .await
            .unwrap();
        let row = sqlx::query_as::<_, (String, Option<String>, Option<String>)>(
            "SELECT status, result, result_backup FROM summary_processes WHERE meeting_id = ?",
        )
        .bind("meeting-1")
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, "cancelled");
        assert_eq!(row.1.as_deref(), Some("old result"));
        assert!(row.2.is_none());
    }
}
