import type { Summary } from '@/types';

export type BackendSummaryStatus =
  | 'idle'
  | 'pending'
  | 'processing'
  | 'completed'
  | 'failed'
  | 'error'
  | 'cancelled'
  | 'interrupted';

export interface BackendSummaryResponse {
  status?: string;
  data?: unknown;
  error?: string | null;
  meetingName?: string | null;
  start?: string | null;
}

export function normalizeSummaryStatus(status: unknown): BackendSummaryStatus {
  const normalized = typeof status === 'string' ? status.toLowerCase() : 'idle';
  switch (normalized) {
    case 'pending':
    case 'processing':
    case 'completed':
    case 'failed':
    case 'error':
    case 'cancelled':
    case 'interrupted':
      return normalized;
    default:
      return 'idle';
  }
}

export function normalizeSummaryData(value: unknown): Summary | null {
  if (value == null) return null;

  let parsed = value;
  if (typeof parsed === 'string') {
    try {
      parsed = JSON.parse(parsed);
    } catch {
      return null;
    }
  }
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return null;

  const record = parsed as Record<string, any>;
  if (
    (typeof record.markdown === 'string' && record.markdown.trim().length > 0) ||
    (Array.isArray(record.summary_json) && record.summary_json.length > 0)
  ) {
    return record as Summary;
  }

  const { MeetingName: _meetingName, _section_order, english_cache: _cache, ...sections } = record;
  const sectionKeys = Array.isArray(_section_order) ? _section_order : Object.keys(sections);
  const formatted: Summary = {};
  for (const key of sectionKeys) {
    const section = sections[key];
    if (!section || typeof section !== 'object' || !Array.isArray(section.blocks)) continue;
    formatted[key] = {
      title: typeof section.title === 'string' ? section.title : key,
      blocks: section.blocks.map((block: any) => ({
        ...block,
        color: block?.color || 'default',
        content: typeof block?.content === 'string' ? block.content.trim() : '',
      })),
    };
  }
  return Object.values(formatted).some((section) => section.blocks.length > 0)
    ? formatted
    : null;
}
