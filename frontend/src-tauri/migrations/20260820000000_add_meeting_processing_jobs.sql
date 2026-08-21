CREATE TABLE meeting_processing_jobs (
    id TEXT PRIMARY KEY NOT NULL,
    meeting_id TEXT NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('asr_recompute', 'speaker_refinement')),
    automatic INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL CHECK(status IN ('pending', 'processing', 'paused', 'completed', 'failed', 'cancelled')),
    progress INTEGER NOT NULL DEFAULT 0 CHECK(progress BETWEEN 0 AND 100),
    config_snapshot TEXT NOT NULL,
    checkpoint TEXT,
    depends_on TEXT,
    error TEXT,
    metrics TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    started_at TEXT,
    completed_at TEXT,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE,
    FOREIGN KEY (depends_on) REFERENCES meeting_processing_jobs(id) ON DELETE SET NULL
);

CREATE INDEX idx_meeting_processing_jobs_meeting
ON meeting_processing_jobs(meeting_id, created_at);

CREATE INDEX idx_meeting_processing_jobs_runnable
ON meeting_processing_jobs(status, created_at);

CREATE UNIQUE INDEX idx_meeting_processing_jobs_active_kind
ON meeting_processing_jobs(meeting_id, kind)
WHERE status IN ('pending', 'processing', 'paused');
