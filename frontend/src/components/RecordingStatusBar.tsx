'use client';

import { motion } from 'framer-motion';
import { useRecordingState } from '@/contexts/RecordingStateContext';
import { formatRecordingDuration } from '@/lib/recordingDuration';
import { useTranslation } from 'react-i18next';

interface RecordingStatusBarProps {
  isPaused?: boolean;
}

export const RecordingStatusBar: React.FC<RecordingStatusBarProps> = ({ isPaused = false }) => {
  const { t } = useTranslation('recording');
  // Get recording duration from backend-synced context (in seconds)
  // Backend polls every 500ms, providing smooth updates
  const { activeDuration } = useRecordingState();
  const recordingDuration = formatRecordingDuration(activeDuration);

  return (
    <motion.div
      initial={{ opacity: 0, y: -10 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -10 }}
      transition={{ duration: 0.2 }}
      className="flex items-center gap-2 px-3 py-2 bg-gray-50 rounded-lg mb-2"
    >
      <div className={`w-2 h-2 rounded-full ${isPaused ? 'bg-orange-500' : 'bg-red-500 animate-pulse'}`} />
      <span
        role="timer"
        aria-label={`${t('duration')}: ${recordingDuration}`}
        className={`text-sm ${isPaused ? 'text-orange-700' : 'text-gray-700'}`}
      >
        {isPaused ? t('paused') : t('recording')} • <span className="font-mono tabular-nums">{recordingDuration}</span>
      </span>
    </motion.div>
  );
};
