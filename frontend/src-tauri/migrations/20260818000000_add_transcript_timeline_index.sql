-- Keep completed meeting transcript snapshots ordered without scanning and sorting
-- every transcript from every meeting.
CREATE INDEX IF NOT EXISTS idx_transcripts_meeting_timeline
ON transcripts(meeting_id, audio_start_time, id);
