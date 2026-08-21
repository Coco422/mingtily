'use client';

import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';

export interface MeetingProcessingJob {
  id: string;
  meetingId: string;
  kind: 'asr_recompute' | 'speaker_refinement';
  status: 'pending' | 'processing' | 'paused' | 'completed' | 'failed' | 'cancelled';
  progress: number;
  error: string | null;
  metrics: string | null;
  createdAt: string;
  updatedAt: string;
}

interface JobsContextValue {
  jobs: MeetingProcessingJob[];
  refresh: () => Promise<void>;
  pause: (jobId: string) => Promise<void>;
  resume: (jobId: string) => Promise<void>;
  cancel: (jobId: string) => Promise<void>;
  retry: (jobId: string) => Promise<void>;
}

const Context = createContext<JobsContextValue | null>(null);

export function MeetingProcessingJobsProvider({ children }: { children: React.ReactNode }) {
  const { t } = useTranslation('settings');
  const [jobs, setJobs] = useState<MeetingProcessingJob[]>([]);
  const refresh = useCallback(async () => setJobs(await invoke<MeetingProcessingJob[]>('processing_list_jobs')), []);
  const run = useCallback(async (command: string, jobId: string) => {
    await invoke(command, { jobId });
    await refresh();
  }, [refresh]);

  useEffect(() => {
    void refresh();
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<MeetingProcessingJob>('meeting-processing-job-updated', (event) => {
      setJobs((current) => [event.payload, ...current.filter((job) => job.id !== event.payload.id)]);
      if (event.payload.status === 'failed') {
        toast.error(t('pipeline.jobs.failedToast'), { description: event.payload.error ?? undefined });
      }
    }).then((dispose) => { if (disposed) dispose(); else unlisten = dispose; });
    return () => { disposed = true; unlisten?.(); };
  }, [refresh, t]);

  const value = useMemo<JobsContextValue>(() => ({
    jobs,
    refresh,
    pause: (id) => run('processing_pause_job', id),
    resume: (id) => run('processing_resume_job', id),
    cancel: (id) => run('processing_cancel_job', id),
    retry: (id) => run('processing_resume_job', id),
  }), [jobs, refresh, run]);
  return <Context.Provider value={value}>{children}<ProcessingJobsTray /></Context.Provider>;
}

export function useMeetingProcessingJobs() {
  const value = useContext(Context);
  if (!value) throw new Error('useMeetingProcessingJobs must be used within MeetingProcessingJobsProvider');
  return value;
}

function ProcessingJobsTray() {
  const { t } = useTranslation('settings');
  const { jobs, pause, resume, cancel, retry } = useMeetingProcessingJobs();
  const active = jobs.filter((job) => ['pending', 'processing', 'paused', 'failed'].includes(job.status));
  if (active.length === 0) return null;
  const action = async (operation: () => Promise<void>) => {
    try { await operation(); }
    catch (error) { toast.error(t('pipeline.jobs.actionFailed'), { description: String(error) }); }
  };
  return <aside className="fixed bottom-4 right-4 z-40 w-[min(24rem,calc(100vw-2rem))] rounded-lg border border-sky-200 bg-white p-3 shadow-xl" aria-label={t('pipeline.jobs.title')}>
    <div className="mb-2 text-sm font-semibold text-slate-900">{t('pipeline.jobs.title')}</div>
    <div className="space-y-2">{active.slice(0, 3).map((job) => {
      let eta: number | undefined;
      try { eta = job.metrics ? JSON.parse(job.metrics).estimatedRemainingSeconds : undefined; } catch { eta = undefined; }
      return <div key={job.id} className="rounded-md bg-slate-50 p-2 text-xs text-slate-700">
        <div className="flex items-center justify-between gap-2"><span className="font-medium">{t(`pipeline.jobs.${job.kind}`)}</span><span>{job.progress}%</span></div>
        <div className="mt-1 h-1.5 overflow-hidden rounded-full bg-slate-200"><div className="h-full bg-sky-500 transition-all" style={{ width: `${job.progress}%` }} /></div>
        <div className="mt-1 flex items-center gap-2"><span>{t(`pipeline.jobs.status.${job.status}`)}</span>{eta !== undefined && <span>· {t('pipeline.jobs.etaMinutes', { count: Math.max(1, Math.ceil(eta / 60)) })}</span>}</div>
        {job.error && <div className="mt-1 text-amber-700">{job.error}</div>}
        <div className="mt-2 flex justify-end gap-1">
          {job.status === 'processing' && <button className="rounded px-2 py-1 hover:bg-white" onClick={() => void action(() => pause(job.id))}>{t('pipeline.jobs.pause')}</button>}
          {job.status === 'paused' && <button className="rounded px-2 py-1 hover:bg-white" onClick={() => void action(() => resume(job.id))}>{t('pipeline.jobs.resume')}</button>}
          {job.status === 'failed' && <button className="rounded px-2 py-1 hover:bg-white" onClick={() => void action(() => retry(job.id))}>{t('pipeline.jobs.retry')}</button>}
          {!['failed', 'cancelled'].includes(job.status) && <button className="rounded px-2 py-1 text-red-700 hover:bg-red-50" onClick={() => void action(() => cancel(job.id))}>{t('pipeline.jobs.cancel')}</button>}
        </div>
      </div>;
    })}</div>
  </aside>;
}
