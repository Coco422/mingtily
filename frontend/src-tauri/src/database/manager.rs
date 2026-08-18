use sqlx::{
    migrate::{MigrateDatabase, MigrateError, Migrator},
    Result, Sqlite, SqlitePool, Transaction,
};
use std::fs;
use std::path::Path;
use tauri::Manager;

pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub fn missing_migration_version(error: &sqlx::Error) -> Option<i64> {
    match error {
        sqlx::Error::Migrate(source) => match source.as_ref() {
            MigrateError::VersionMissing(version) => Some(*version),
            _ => None,
        },
        _ => None,
    }
}

#[derive(Clone)]
pub struct DatabaseManager {
    pool: SqlitePool,
}

impl DatabaseManager {
    pub async fn new(tauri_db_path: &str) -> Result<Self> {
        if let Some(parent_dir) = Path::new(tauri_db_path).parent() {
            if !parent_dir.exists() {
                fs::create_dir_all(parent_dir).map_err(|e| sqlx::Error::Io(e))?;
            }
        }

        if !Path::new(tauri_db_path).exists() {
            log::info!("Creating database at {}", tauri_db_path);
            Sqlite::create_database(tauri_db_path).await?;
        }

        let pool = SqlitePool::connect(tauri_db_path).await?;

        MIGRATOR.run(&pool).await?;

        Ok(DatabaseManager { pool })
    }

    pub async fn new_from_app_handle(app_handle: &tauri::AppHandle) -> Result<Self> {
        // Resolve the app's data directory
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .expect("failed to get app data dir");
        if !app_data_dir.exists() {
            fs::create_dir_all(&app_data_dir).map_err(|e| sqlx::Error::Io(e))?;
        }

        // Define database paths
        let tauri_db_path = app_data_dir
            .join("meeting_minutes.sqlite")
            .to_string_lossy()
            .to_string();
        // WAL file paths for defensive cleanup
        let wal_path = app_data_dir.join("meeting_minutes.sqlite-wal");
        let shm_path = app_data_dir.join("meeting_minutes.sqlite-shm");

        log::info!("Tauri DB path: {}", tauri_db_path);
        // Try to open database with defensive WAL handling
        match Self::new(&tauri_db_path).await {
            Ok(db_manager) => {
                log::info!("Database opened successfully");
                Ok(db_manager)
            }
            Err(e) => {
                // Check if error is due to corrupted WAL file
                let error_msg = e.to_string();
                if error_msg.contains("malformed") || error_msg.contains("corrupt") {
                    log::warn!("Database appears corrupted, likely due to orphaned WAL file. Attempting recovery...");
                    log::warn!("Error details: {}", error_msg);

                    // Delete potentially corrupted WAL/SHM files
                    if wal_path.exists() {
                        match fs::remove_file(&wal_path) {
                            Ok(_) => log::info!("Removed orphaned WAL file: {:?}", wal_path),
                            Err(e) => log::warn!("Failed to remove WAL file: {}", e),
                        }
                    }
                    if shm_path.exists() {
                        match fs::remove_file(&shm_path) {
                            Ok(_) => log::info!("Removed orphaned SHM file: {:?}", shm_path),
                            Err(e) => log::warn!("Failed to remove SHM file: {}", e),
                        }
                    }

                    // Retry connection without WAL files
                    log::info!("Retrying database connection after WAL cleanup...");
                    match Self::new(&tauri_db_path).await {
                        Ok(db_manager) => {
                            log::info!("Database opened successfully after WAL recovery");
                            Ok(db_manager)
                        }
                        Err(retry_err) => {
                            log::error!(
                                "Database connection failed even after WAL cleanup: {}",
                                retry_err
                            );
                            Err(retry_err)
                        }
                    }
                } else {
                    // Not a WAL-related error, propagate original error
                    log::error!("Database connection failed: {}", error_msg);
                    Err(e)
                }
            }
        }
    }

    /// Check if this is the first launch (sqlite database doesn't exist yet)
    pub async fn is_first_launch(app_handle: &tauri::AppHandle) -> Result<bool> {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .expect("failed to get app data dir");

        let tauri_db_path = app_data_dir.join("meeting_minutes.sqlite");

        Ok(!tauri_db_path.exists())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn with_transaction<T, F, Fut>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut Transaction<'_, Sqlite>) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut tx = self.pool.begin().await?;
        let result = f(&mut tx).await;

        match result {
            Ok(val) => {
                tx.commit().await?;
                Ok(val)
            }
            Err(err) => {
                tx.rollback().await?;
                Err(err)
            }
        }
    }

    /// Cleanup database connection and checkpoint WAL
    /// This should be called on application shutdown to ensure:
    /// - All WAL changes are written to the main database file
    /// - The .wal and .shm files are deleted
    /// - Connection pool is gracefully closed
    pub async fn cleanup(&self) -> Result<()> {
        log::info!("Starting database cleanup...");

        // Force checkpoint of WAL to main database file and remove WAL file
        // TRUNCATE mode: checkpoints all pages AND deletes the WAL file
        match sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&self.pool)
            .await
        {
            Ok(_) => log::info!("WAL checkpoint completed successfully"),
            Err(e) => log::warn!("WAL checkpoint failed (non-fatal): {}", e),
        }

        // Close the connection pool gracefully
        self.pool.close().await;
        log::info!("Database connection pool closed");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{migrate::Migrate, Connection, Executor, Row, SqliteConnection};

    const V0_6_2_LAST_MIGRATION: i64 = 20251229000000;
    const V0_6_2_MIGRATION_CHECKSUMS: &[(i64, &str)] = &[
        (
            20250916100000,
            "7458ff9ff8c4fc1d0c21fba3078b72555ed8e7585c05c12f21e92a6406ce2fa5bdb37b1a571d0fd37be9f1142786685b",
        ),
        (
            20250920155811,
            "214d65142af1c9f35bf70c8b56742f90cea59ad238f88474f4a4e68d0d2379daf2cce26aa019f86cfca02a2bea380bd2",
        ),
        (
            20251006000000,
            "8a82ee1452083b05f6ac4a1ff84cdb08ae1c579b9e8405f11a91d612f8d4ffe1dcbaa864b6661bcc1465b5a325aa54d4",
        ),
        (
            20251010153942,
            "27c924fdd27b50288ef0bbaccde5731acccfc1e72a7c41018e69638c72b6c074d851e8536f840c8663a829ffd55e43e5",
        ),
        (
            20251101000000,
            "dab914be5bdf2f3b718f2c6b30f62e30750b2ff24af94b83a82e4a7f30a336dcab1187dcb32959a1c3bdcfd78ec62139",
        ),
        (
            20251105120000,
            "f5de8e11d508dd1bef80b8b95ba33675b24fea4d63742c7d16b2cdfd8054c5a02415daf9b7becc8277beef7beb446e91",
        ),
        (
            20251110000000,
            "0b7283bccaede4f067c375539736e2da7a77bd0aa2d52643aafcf9d379b8bbbf9251b04f0a08057ef0dfb1b182d1ec3d",
        ),
        (
            20251110000001,
            "8c7e0dd650585576c55da259a091618cc6597f13a77d62bdb1ed4c827ed77e5e4e759bcd418eb79e2158fb2f3801f667",
        ),
        (
            20251223000000,
            "9a2891ab5801b6c987ec9c4aca34581ca41ff7f25385e039340da9688cc19df4b590b9e98d5f01bf5f33a7e985d3d15c",
        ),
        (
            20251229000000,
            "f24ee2cf9a10d33141258f0edfc59bdd3354dfb0bd4307e82b5a0d9aede013df91252ce84f2382613e3f04e4f61c331b",
        ),
    ];

    fn checksum_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    async fn create_v0_6_2_database(path: &str) {
        Sqlite::create_database(path).await.unwrap();
        let mut connection = SqliteConnection::connect(path).await.unwrap();
        connection.ensure_migrations_table().await.unwrap();

        for migration in MIGRATOR
            .iter()
            .filter(|migration| migration.version <= V0_6_2_LAST_MIGRATION)
        {
            connection.apply(migration).await.unwrap();
        }

        connection.close().await.unwrap();
    }

    #[test]
    fn v0_6_2_migrations_keep_their_published_checksums() {
        for (version, expected_checksum) in V0_6_2_MIGRATION_CHECKSUMS {
            let migration = MIGRATOR
                .iter()
                .find(|migration| migration.version == *version)
                .unwrap_or_else(|| panic!("published migration {version} is missing"));
            assert_eq!(checksum_hex(&migration.checksum), *expected_checksum);
        }
    }

    #[tokio::test]
    async fn upgrades_v0_6_2_database_to_latest() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("meeting_minutes.sqlite");
        let database_path = database_path.to_string_lossy().to_string();
        create_v0_6_2_database(&database_path).await;

        let manager = DatabaseManager::new(&database_path).await.unwrap();

        let latest_version: i64 = sqlx::query("SELECT MAX(version) FROM _sqlx_migrations")
            .fetch_one(manager.pool())
            .await
            .unwrap()
            .get(0);
        let speaker_map_table: String = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'meeting_speaker_maps'",
        )
        .fetch_one(manager.pool())
        .await
        .unwrap()
        .get(0);
        let transcript_timeline_index: String = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type = 'index' AND name = 'idx_transcripts_meeting_timeline'",
        )
        .fetch_one(manager.pool())
        .await
        .unwrap()
        .get(0);
        let transcript_query_plan = sqlx::query(
            "EXPLAIN QUERY PLAN
             SELECT * FROM transcripts
             WHERE meeting_id = ?
             ORDER BY audio_start_time ASC, id ASC",
        )
        .bind("meeting-test")
        .fetch_all(manager.pool())
        .await
        .unwrap();

        assert_eq!(latest_version, 20260818000000);
        assert_eq!(speaker_map_table, "meeting_speaker_maps");
        assert_eq!(
            transcript_timeline_index,
            "idx_transcripts_meeting_timeline"
        );
        assert!(transcript_query_plan.iter().any(|row| {
            row.get::<String, _>("detail")
                .contains("idx_transcripts_meeting_timeline")
        }));
        manager.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn detects_database_created_by_a_newer_version() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("meeting_minutes.sqlite");
        let database_path = database_path.to_string_lossy().to_string();

        let manager = DatabaseManager::new(&database_path).await.unwrap();
        manager
            .pool()
            .execute(
                "INSERT INTO _sqlx_migrations \
                 (version, description, installed_on, success, checksum, execution_time) \
                 VALUES (20991231000000, 'future migration', CURRENT_TIMESTAMP, TRUE, X'00', 0)",
            )
            .await
            .unwrap();
        manager.cleanup().await.unwrap();

        let error = match DatabaseManager::new(&database_path).await {
            Ok(_) => panic!("database with an unknown migration should be rejected"),
            Err(error) => error,
        };
        assert_eq!(missing_migration_version(&error), Some(20991231000000));
    }
}
