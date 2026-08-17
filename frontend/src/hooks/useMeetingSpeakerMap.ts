import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';
import type { MeetingSpeakerMap, SpeakerParticipant } from '@/lib/speaker-map';

const EMPTY_MAP = (meetingId: string): MeetingSpeakerMap => ({
  meetingId,
  revision: 0,
  participants: [],
  speakers: [],
});

export function useMeetingSpeakerMap(meetingId: string) {
  const { t } = useTranslation('meeting');
  const [speakerMap, setSpeakerMap] = useState<MeetingSpeakerMap>(() => EMPTY_MAP(meetingId));
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const next = await invoke<MeetingSpeakerMap>('api_get_meeting_speaker_map', { meetingId });
      setSpeakerMap(next);
      return next;
    } finally {
      setLoading(false);
    }
  }, [meetingId]);

  useEffect(() => {
    setSpeakerMap(EMPTY_MAP(meetingId));
    void refresh().catch((error) => console.error('Failed to load meeting speaker map:', error));
  }, [meetingId, refresh]);

  const save = useCallback(async (
    participants: SpeakerParticipant[],
    options: { toastOnSuccess?: boolean } = { toastOnSuccess: true },
  ) => {
    const previous = speakerMap;
    try {
      const next = await invoke<MeetingSpeakerMap>('api_save_meeting_speaker_map', {
        meetingId,
        expectedRevision: previous.revision,
        participants,
      });
      setSpeakerMap(next);
      if (options.toastOnSuccess !== false) {
        toast.success(t('speakerMapSaved'), {
          description: t('speakerMapSummaryHint'),
          action: {
            label: t('speakerMapUndo'),
            onClick: () => {
              void invoke<MeetingSpeakerMap>('api_save_meeting_speaker_map', {
                meetingId,
                expectedRevision: next.revision,
                participants: previous.participants,
              }).then(setSpeakerMap).catch((error) => {
                toast.error(t('speakerMapUndoFailed'), { description: String(error) });
              });
            },
          },
        });
      }
      return next;
    } catch (error) {
      const message = String(error);
      if (message.includes('changed in another view')) {
        await refresh().catch((refreshError) => {
          console.error('Failed to refresh speaker map after a revision conflict:', refreshError);
        });
      }
      toast.error(t('speakerMapSaveFailed'), { description: message });
      throw error;
    }
  }, [meetingId, refresh, speakerMap, t]);

  return { speakerMap, loading, refresh, save };
}
