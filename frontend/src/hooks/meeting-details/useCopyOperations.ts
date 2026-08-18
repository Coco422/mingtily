import { useCallback, RefObject } from 'react';
import { Transcript, Summary } from '@/types';
import { BlockNoteSummaryViewRef } from '@/components/AISummary/BlockNoteSummaryView';
import { toast } from 'sonner';
import { invoke as invokeTauri } from '@tauri-apps/api/core';
import { prefixResolvedSpeaker, type SpeakerParticipant } from '@/lib/speaker-map';
import { useTranslation } from 'react-i18next';

interface UseCopyOperationsProps {
  meeting: any;
  transcripts: Transcript[];
  meetingTitle: string;
  aiSummary: Summary | null;
  blockNoteSummaryRef: RefObject<BlockNoteSummaryViewRef>;
  speakerParticipants?: SpeakerParticipant[];
}

export function useCopyOperations({
  meeting,
  transcripts,
  meetingTitle,
  aiSummary,
  blockNoteSummaryRef,
  speakerParticipants = [],
}: UseCopyOperationsProps) {
  const { t, i18n } = useTranslation(['meeting', 'common']);

  // Helper function to fetch ALL transcripts for copying (not just paginated data)
  const fetchAllTranscripts = useCallback(async (meetingId: string): Promise<Transcript[]> => {
    try {
      console.log('📊 Fetching all transcripts for copying:', meetingId);

      const allData = await invokeTauri('api_get_meeting_transcripts', {
        meetingId,
      }) as { transcripts: Transcript[]; total_count: number; has_more: boolean };

      console.log(`✅ Fetched ${allData.transcripts.length} transcripts from database for copying`);
      return allData.transcripts;
    } catch (error) {
      console.error('❌ Error fetching all transcripts:', error);
      toast.error(t('meeting:copyFetchFailed'));
      return [];
    }
  }, [t]);

  // Copy transcript to clipboard
  const handleCopyTranscript = useCallback(async () => {
    // CHANGE: Fetch ALL transcripts from database, not from pagination state
    console.log('📊 Fetching all transcripts for copying...');
    const allTranscripts = await fetchAllTranscripts(meeting.id);

    if (!allTranscripts.length) {
      const error_msg = t('meeting:noCopyTranscript');
      console.log(error_msg);
      toast.error(error_msg);
      return;
    }

    console.log(`✅ Copying ${allTranscripts.length} transcripts to clipboard`);

    // Format timestamps as recording-relative [MM:SS] instead of wall-clock time
    const formatTime = (seconds: number | undefined, fallbackTimestamp: string): string => {
      if (seconds === undefined) {
        // For old transcripts without audio_start_time, use wall-clock time
        return fallbackTimestamp;
      }
      const totalSecs = Math.floor(seconds);
      const mins = Math.floor(totalSecs / 60);
      const secs = totalSecs % 60;
      return `[${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}]`;
    };

    const title = meetingTitle ?? meeting.title;
    const header = `# ${t('meeting:transcriptExportTitle', { title })}\n\n`;
    const date = `## ${t('meeting:dateLabel')}: ${new Date(meeting.created_at).toLocaleDateString(i18n.language)}\n\n`;
    const fullTranscript = allTranscripts
      .map(segment => `${formatTime(segment.audio_start_time, segment.timestamp)} ${prefixResolvedSpeaker(segment.text, segment.speaker, speakerParticipants, t)}  `)
      .join('\n');

    await navigator.clipboard.writeText(header + date + fullTranscript);
    toast.success(t('meeting:copyTranscriptSuccess'));

  }, [meeting, meetingTitle, fetchAllTranscripts, i18n.language, speakerParticipants, t]);

  // Copy summary to clipboard
  const handleCopySummary = useCallback(async () => {
    try {
      let summaryMarkdown = '';

      console.log('🔍 Copy Summary - Starting...');

      // Try to get markdown from BlockNote editor first
      if (blockNoteSummaryRef.current?.getMarkdown) {
        console.log('📝 Trying to get markdown from ref...');
        summaryMarkdown = await blockNoteSummaryRef.current.getMarkdown();
        console.log('📝 Got markdown from ref, length:', summaryMarkdown.length);
      }

      // Fallback: Check if aiSummary has markdown property
      if (!summaryMarkdown && aiSummary && 'markdown' in aiSummary) {
        console.log('📝 Using markdown from aiSummary');
        summaryMarkdown = (aiSummary as any).markdown || '';
        console.log('📝 Markdown from aiSummary, length:', summaryMarkdown.length);
      }

      // Fallback: Check for legacy format
      if (!summaryMarkdown && aiSummary) {
        console.log('📝 Converting legacy format to markdown');
        const sections = Object.entries(aiSummary)
          .filter(([key]) => {
            // Skip non-section keys
            return key !== 'markdown' && key !== 'summary_json' && key !== '_section_order' && key !== 'MeetingName';
          })
          .map(([, section]) => {
            if (section && typeof section === 'object' && 'title' in section && 'blocks' in section) {
              const sectionTitle = `## ${section.title}\n\n`;
              const sectionContent = section.blocks
                .map((block: any) => `- ${block.content}`)
                .join('\n');
              return sectionTitle + sectionContent;
            }
            return '';
          })
          .filter(s => s.trim())
          .join('\n\n');
        summaryMarkdown = sections;
        console.log('📝 Converted legacy format, length:', summaryMarkdown.length);
      }

      // If still no summary content, show message
      if (!summaryMarkdown.trim()) {
        console.error('❌ No summary content available to copy');
        toast.error(t('meeting:noCopySummary'));
        return;
      }

      // Build metadata header
      const dateOptions: Intl.DateTimeFormatOptions = {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit'
      };
      const header = `# ${t('meeting:summaryExportTitle', { title: meetingTitle })}\n\n`;
      const metadata = `**${t('meeting:meetingIdLabel')}:** ${meeting.id}\n**${t('meeting:dateLabel')}:** ${new Date(meeting.created_at).toLocaleString(i18n.language, dateOptions)}\n**${t('meeting:copiedOnLabel')}:** ${new Date().toLocaleString(i18n.language, dateOptions)}\n\n---\n\n`;

      const fullMarkdown = header + metadata + summaryMarkdown;
      await navigator.clipboard.writeText(fullMarkdown);

      console.log('✅ Successfully copied to clipboard!');
      toast.success(t('meeting:copySummarySuccess'));

    } catch (error) {
      console.error('❌ Failed to copy summary:', error);
      toast.error(t('meeting:copySummaryFailed'));
    }
  }, [aiSummary, meetingTitle, meeting, blockNoteSummaryRef, i18n.language, t]);

  return {
    handleCopyTranscript,
    handleCopySummary,
  };
}
