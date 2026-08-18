'use client';

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import React, { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import {
  normalizeSummaryData,
  normalizeSummaryStatus,
  type BackendSummaryResponse,
  type BackendSummaryStatus,
} from '@/lib/summary-data';
import type { Summary } from '@/types';

interface SummaryGenerationStreamPayload {
  meeting_id: string;
  markdown: string;
  thinking: string | null;
  thinking_complete: boolean;
  phase: 'final';
}

export type SummaryProgressPhase =
  | 'preparing'
  | 'analyzing_chunks'
  | 'combining'
  | 'understanding'
  | 'streaming'
  | 'translating';

interface SummaryGenerationProgressPayload {
  meeting_id: string;
  phase: SummaryProgressPhase;
  current?: number | null;
  total?: number | null;
}

export interface SummaryJob {
  meetingId: string;
  status: BackendSummaryStatus;
  data: Summary | null;
  error: string | null;
  meetingName: string | null;
  streamingSummary: string;
  streamingThinking: string | null;
  streamingThinkingComplete: boolean;
  phase: SummaryProgressPhase | null;
  currentStep: number | null;
  totalSteps: number | null;
  startedAt: number | null;
  unread: boolean;
  updatedAt: number;
}

interface SummaryJobsContextValue {
  jobs: Record<string, SummaryJob>;
  getJob: (meetingId: string) => SummaryJob | undefined;
  refreshJob: (meetingId: string) => Promise<SummaryJob>;
  trackJob: (meetingId: string) => void;
  cancelJob: (meetingId: string) => Promise<void>;
  acknowledgeJob: (meetingId: string) => void;
  removeJob: (meetingId: string) => void;
}

const SummaryJobsContext = createContext<SummaryJobsContextValue | null>(null);
const ACTIVE_STATUSES = new Set<BackendSummaryStatus>(['pending', 'processing']);
const TERMINAL_STATUSES = new Set<BackendSummaryStatus>([
  'completed', 'failed', 'error', 'cancelled', 'interrupted',
]);

function parseStartedAt(value: string | null | undefined): number | null {
  if (!value) return null;
  const parsed = Date.parse(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function createActiveJob(meetingId: string): SummaryJob {
  return {
    meetingId,
    status: 'processing',
    data: null,
    error: null,
    meetingName: null,
    streamingSummary: '',
    streamingThinking: null,
    streamingThinkingComplete: false,
    phase: 'preparing',
    currentStep: null,
    totalSteps: null,
    startedAt: Date.now(),
    unread: false,
    updatedAt: Date.now(),
  };
}

export function SummaryJobsProvider({ children }: { children: React.ReactNode }) {
  const { t } = useTranslation('summary');
  const [jobs, setJobs] = useState<Record<string, SummaryJob>>({});
  const jobsRef = useRef(jobs);
  const pollersRef = useRef(new Map<string, ReturnType<typeof setInterval>>());
  const notifyOnTerminalRef = useRef(new Set<string>());
  const notifiedTerminalRef = useRef(new Set<string>());
  const hydratedRef = useRef(false);

  useEffect(() => {
    jobsRef.current = jobs;
  }, [jobs]);

  const stopPolling = useCallback((meetingId: string) => {
    const timer = pollersRef.current.get(meetingId);
    if (timer) clearInterval(timer);
    pollersRef.current.delete(meetingId);
  }, []);

  const applyResponse = useCallback((meetingId: string, raw: BackendSummaryResponse) => {
    const status = normalizeSummaryStatus(raw.status);
    const data = normalizeSummaryData(raw.data);
    const previous = jobsRef.current[meetingId];
    const transitionedToTerminal = TERMINAL_STATUSES.has(status) &&
      !!previous && ACTIVE_STATUSES.has(previous.status);
    const error = status === 'interrupted'
      ? t('interrupted')
      : raw.error || (status === 'completed' && !data ? t('emptyResult') : null);
    const effectiveStatus = status === 'completed' && !data ? 'failed' : status;
    const isActive = ACTIVE_STATUSES.has(effectiveStatus);
    const responseStartedAt = parseStartedAt(raw.start);
    const next: SummaryJob = {
      meetingId,
      status: effectiveStatus,
      data,
      error,
      meetingName: raw.meetingName || null,
      streamingSummary: isActive ? previous?.streamingSummary || '' : '',
      streamingThinking: isActive ? previous?.streamingThinking ?? null : null,
      streamingThinkingComplete: isActive
        ? previous?.streamingThinkingComplete || false
        : false,
      phase: isActive ? previous?.phase || 'preparing' : null,
      currentStep: isActive ? previous?.currentStep ?? null : null,
      totalSteps: isActive ? previous?.totalSteps ?? null : null,
      startedAt: responseStartedAt ?? previous?.startedAt ?? (isActive ? Date.now() : null),
      unread: previous?.unread || transitionedToTerminal,
      updatedAt: Date.now(),
    };

    jobsRef.current = { ...jobsRef.current, [meetingId]: next };
    setJobs(jobsRef.current);

    if (TERMINAL_STATUSES.has(effectiveStatus)) stopPolling(meetingId);
    if (
      transitionedToTerminal &&
      notifyOnTerminalRef.current.has(meetingId) &&
      !notifiedTerminalRef.current.has(meetingId)
    ) {
      notifiedTerminalRef.current.add(meetingId);
      if (effectiveStatus === 'completed') {
        toast.success(t('success'), { description: t('ready') });
      } else if (effectiveStatus !== 'cancelled') {
        toast.error(t('generationFailed'), { description: error || undefined });
      }
    }
    return next;
  }, [stopPolling, t]);

  const refreshJob = useCallback(async (meetingId: string) => {
    const raw = await invoke<BackendSummaryResponse>('api_get_summary', { meetingId });
    const next = applyResponse(meetingId, raw);
    if (ACTIVE_STATUSES.has(next.status) && !pollersRef.current.has(meetingId)) {
      notifyOnTerminalRef.current.add(meetingId);
      const timer = setInterval(() => {
        void invoke<BackendSummaryResponse>('api_get_summary', { meetingId })
          .then((response) => applyResponse(meetingId, response))
          .catch((error) => console.error(`Failed to poll summary ${meetingId}:`, error));
      }, 2000);
      pollersRef.current.set(meetingId, timer);
    }
    return next;
  }, [applyResponse]);

  const trackJob = useCallback((meetingId: string) => {
    notifyOnTerminalRef.current.add(meetingId);
    notifiedTerminalRef.current.delete(meetingId);
    const previous = jobsRef.current[meetingId];
    const pending: SummaryJob = previous && ACTIVE_STATUSES.has(previous.status) ? {
      ...previous,
      error: null,
      unread: false,
      updatedAt: Date.now(),
    } : {
      meetingId,
      status: 'pending',
      data: previous?.data || null,
      error: null,
      meetingName: previous?.meetingName || null,
      streamingSummary: '',
      streamingThinking: null,
      streamingThinkingComplete: false,
      phase: 'preparing',
      currentStep: null,
      totalSteps: null,
      startedAt: Date.now(),
      unread: false,
      updatedAt: Date.now(),
    };
    jobsRef.current = { ...jobsRef.current, [meetingId]: pending };
    setJobs(jobsRef.current);
    void refreshJob(meetingId);
  }, [refreshJob]);

  const cancelJob = useCallback(async (meetingId: string) => {
    await invoke('api_cancel_summary', { meetingId });
    await refreshJob(meetingId);
  }, [refreshJob]);

  const acknowledgeJob = useCallback((meetingId: string) => {
    const current = jobsRef.current[meetingId];
    if (!current?.unread) return;
    jobsRef.current = { ...jobsRef.current, [meetingId]: { ...current, unread: false } };
    setJobs(jobsRef.current);
  }, []);

  const removeJob = useCallback((meetingId: string) => {
    stopPolling(meetingId);
    const next = { ...jobsRef.current };
    delete next[meetingId];
    jobsRef.current = next;
    setJobs(next);
    notifyOnTerminalRef.current.delete(meetingId);
    notifiedTerminalRef.current.delete(meetingId);
  }, [stopPolling]);

  useEffect(() => {
    let disposed = false;
    let unlistenStream: UnlistenFn | undefined;
    let unlistenProgress: UnlistenFn | undefined;
    void listen<SummaryGenerationStreamPayload>('summary-generation-stream', (event) => {
      const payload = event.payload;
      if (payload.phase !== 'final') return;
      const current = jobsRef.current[payload.meeting_id] || createActiveJob(payload.meeting_id);
      if (!ACTIVE_STATUSES.has(current.status)) return;
      const next = {
        ...current,
        streamingSummary: payload.markdown,
        streamingThinking: payload.thinking,
        streamingThinkingComplete: payload.thinking_complete,
        phase: 'streaming' as const,
        currentStep: null,
        totalSteps: null,
        updatedAt: Date.now(),
      };
      jobsRef.current = { ...jobsRef.current, [payload.meeting_id]: next };
      setJobs(jobsRef.current);
    }).then((fn) => disposed ? fn() : (unlistenStream = fn));
    void listen<SummaryGenerationProgressPayload>('summary-generation-progress', (event) => {
      const payload = event.payload;
      const current = jobsRef.current[payload.meeting_id] || createActiveJob(payload.meeting_id);
      if (!ACTIVE_STATUSES.has(current.status)) return;
      const next: SummaryJob = {
        ...current,
        phase: payload.phase,
        currentStep: payload.current ?? null,
        totalSteps: payload.total ?? null,
        updatedAt: Date.now(),
      };
      jobsRef.current = { ...jobsRef.current, [payload.meeting_id]: next };
      setJobs(jobsRef.current);
    }).then((fn) => disposed ? fn() : (unlistenProgress = fn));
    return () => {
      disposed = true;
      unlistenStream?.();
      unlistenProgress?.();
      pollersRef.current.forEach(clearInterval);
      pollersRef.current.clear();
    };
  }, []);

  useEffect(() => {
    if (hydratedRef.current) return;
    hydratedRef.current = true;
    void invoke<Array<BackendSummaryResponse & { meeting_id: string }>>(
      'api_list_recoverable_summary_jobs',
    ).then((responses) => {
      for (const response of responses) {
        const next = applyResponse(response.meeting_id, response);
        if (next.status === 'interrupted') {
          const unread = { ...next, unread: true };
          jobsRef.current = { ...jobsRef.current, [response.meeting_id]: unread };
          setJobs(jobsRef.current);
        } else if (ACTIVE_STATUSES.has(next.status)) {
          void refreshJob(response.meeting_id);
        }
      }
    }).catch((error) => console.error('Failed to restore background summary jobs:', error));
  }, [applyResponse, refreshJob]);

  const value = useMemo<SummaryJobsContextValue>(() => ({
    jobs,
    getJob: (meetingId) => jobs[meetingId],
    refreshJob,
    trackJob,
    cancelJob,
    acknowledgeJob,
    removeJob,
  }), [jobs, refreshJob, trackJob, cancelJob, acknowledgeJob, removeJob]);

  return <SummaryJobsContext.Provider value={value}>{children}</SummaryJobsContext.Provider>;
}

export function useSummaryJobs() {
  const context = useContext(SummaryJobsContext);
  if (!context) throw new Error('useSummaryJobs must be used within SummaryJobsProvider');
  return context;
}
