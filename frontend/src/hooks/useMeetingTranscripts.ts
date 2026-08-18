import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  MeetingMetadata,
  PaginatedTranscriptsResponse,
  Transcript,
  TranscriptSegmentData,
} from '@/types';
import { useTranslation } from 'react-i18next';

interface UseMeetingTranscriptsProps {
  meetingId: string | null;
}

interface UseMeetingTranscriptsReturn {
  metadata: MeetingMetadata | null;
  segments: TranscriptSegmentData[];
  transcripts: Transcript[];
  isLoading: boolean;
  error: string | null;
  refetch: () => Promise<boolean>;
}

function convertTranscriptsToSegments(transcripts: Transcript[]): TranscriptSegmentData[] {
  return transcripts.map((transcript) => ({
    id: transcript.id,
    timestamp: transcript.audio_start_time ?? 0,
    endTime: transcript.audio_end_time,
    text: transcript.text,
    confidence: transcript.confidence,
    speaker: transcript.speaker,
    speakerIsProvisional: transcript.speaker_is_provisional,
  }));
}

async function fetchMeetingSnapshot(meetingId: string) {
  const [metadata, response] = await Promise.all([
    invoke<MeetingMetadata>('api_get_meeting_metadata', { meetingId }),
    invoke<PaginatedTranscriptsResponse>('api_get_meeting_transcripts', { meetingId }),
  ]);

  return { metadata, response };
}

export function useMeetingTranscripts({
  meetingId,
}: UseMeetingTranscriptsProps): UseMeetingTranscriptsReturn {
  const { t } = useTranslation('meeting');
  const [metadata, setMetadata] = useState<MeetingMetadata | null>(null);
  const [transcripts, setTranscripts] = useState<Transcript[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const requestIdRef = useRef(0);

  useEffect(() => {
    const requestId = ++requestIdRef.current;

    if (!meetingId) {
      setMetadata(null);
      setTranscripts([]);
      setError(null);
      setIsLoading(false);
      return;
    }

    setMetadata(null);
    setTranscripts([]);
    setError(null);
    setIsLoading(true);

    void fetchMeetingSnapshot(meetingId)
      .then((snapshot) => {
        if (requestId !== requestIdRef.current) return;
        setMetadata(snapshot.metadata);
        setTranscripts(snapshot.response.transcripts);
      })
      .catch((fetchError) => {
        if (requestId !== requestIdRef.current) return;
        console.error('Failed to load meeting transcripts:', fetchError);
        setError(t('transcriptLoadFailed'));
      })
      .finally(() => {
        if (requestId === requestIdRef.current) {
          setIsLoading(false);
        }
      });

    return () => {
      requestIdRef.current += 1;
    };
  }, [meetingId, t]);

  const refetch = useCallback(async (): Promise<boolean> => {
    if (!meetingId) return false;

    const requestId = ++requestIdRef.current;
    try {
      const snapshot = await fetchMeetingSnapshot(meetingId);
      if (requestId !== requestIdRef.current) return false;

      // Replace metadata and transcripts together only after the complete snapshot succeeds.
      setMetadata(snapshot.metadata);
      setTranscripts(snapshot.response.transcripts);
      setError(null);
      return true;
    } catch (fetchError) {
      if (requestId === requestIdRef.current) {
        console.error('Failed to refresh meeting transcripts:', fetchError);
      }
      return false;
    }
  }, [meetingId]);

  const segments = useMemo(
    () => convertTranscriptsToSegments(transcripts),
    [transcripts],
  );

  return {
    metadata,
    segments,
    transcripts,
    isLoading,
    error,
    refetch,
  };
}
