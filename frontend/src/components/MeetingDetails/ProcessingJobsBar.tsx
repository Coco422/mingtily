'use client';

import { Button } from '@/components/ui/button';
import { useMeetingProcessingJobs } from '@/contexts/MeetingProcessingJobsContext';
import { useTranslation } from 'react-i18next';
import { useEffect, useRef } from 'react';
import { toast } from 'sonner';

export function ProcessingJobsBar({ meetingId, onComplete }: { meetingId: string; onComplete?: () => void }) {
  const { t } = useTranslation('settings');
  const { jobs, pause, resume, cancel, retry } = useMeetingProcessingJobs();
  const completedId = jobs.find((candidate) => candidate.meetingId === meetingId && candidate.status === 'completed')?.id;
  const reportedCompletion = useRef<string>();
  useEffect(() => {
    if (completedId && reportedCompletion.current !== completedId) {
      reportedCompletion.current = completedId;
      onComplete?.();
    }
  }, [completedId, onComplete]);
  const job = jobs.find((candidate) => candidate.meetingId === meetingId && ['pending', 'processing', 'paused', 'failed'].includes(candidate.status));
  if (!job) return null;
  const action = async (operation: () => Promise<void>) => {
    try {
      await operation();
    } catch (error) {
      toast.error(t('pipeline.jobs.actionFailed'), { description: String(error) });
    }
  };
  return <div className="flex items-center gap-3 border-b border-sky-200 bg-sky-50 px-4 py-2 text-sm">
    <span className="font-medium">{t(`pipeline.jobs.${job.kind}`)}</span>
    <span>{t(`pipeline.jobs.status.${job.status}`)} · {job.progress}%</span>
    {job.error && <span className="text-amber-700">{job.error}</span>}
    <div className="ml-auto flex gap-2">
      {job.status === 'processing' && <Button size="sm" variant="outline" onClick={() => void action(() => pause(job.id))}>{t('pipeline.jobs.pause')}</Button>}
      {job.status === 'paused' && <Button size="sm" variant="outline" onClick={() => void action(() => resume(job.id))}>{t('pipeline.jobs.resume')}</Button>}
      {job.status === 'failed' && <Button size="sm" variant="outline" onClick={() => void action(() => retry(job.id))}>{t('pipeline.jobs.retry')}</Button>}
      {!['failed', 'cancelled'].includes(job.status) && <Button size="sm" variant="outline" onClick={() => void action(() => cancel(job.id))}>{t('pipeline.jobs.cancel')}</Button>}
    </div>
  </div>;
}
