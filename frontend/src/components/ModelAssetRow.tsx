'use client';

import type { ReactNode } from 'react';
import type { LucideIcon } from 'lucide-react';
import { AudioLines } from 'lucide-react';
import { cn } from '@/lib/utils';

export type ModelAssetState =
  | 'installed'
  | 'missing'
  | 'downloading'
  | 'corrupt'
  | 'error';

interface ModelAssetBadge {
  label: string;
  tone?: 'accent' | 'warning' | 'neutral';
}

interface ModelAssetRowProps {
  name: string;
  provider: string;
  description?: string;
  metadata?: string[];
  state: ModelAssetState;
  statusLabel: string;
  inUse?: boolean;
  badges?: ModelAssetBadge[];
  progress?: number | null;
  progressLabel?: string;
  icon?: LucideIcon;
  actions?: ReactNode;
  className?: string;
  onClick?: () => void;
}

const stateDotClass: Record<ModelAssetState, string> = {
  installed: 'bg-emerald-500',
  missing: 'bg-gray-300',
  downloading: 'bg-sky-500',
  corrupt: 'bg-amber-500',
  error: 'bg-red-500',
};

const badgeToneClass: Record<NonNullable<ModelAssetBadge['tone']>, string> = {
  accent: 'border-sky-200 bg-sky-50 text-sky-700',
  warning: 'border-amber-200 bg-amber-50 text-amber-700',
  neutral: 'border-gray-200 bg-gray-50 text-gray-600',
};

export function ModelAssetRow({
  name,
  provider,
  description,
  metadata = [],
  state,
  statusLabel,
  inUse = false,
  badges = [],
  progress,
  progressLabel,
  icon: Icon = AudioLines,
  actions,
  className,
  onClick,
}: ModelAssetRowProps) {
  const interactive = Boolean(onClick);

  return (
    <div
      className={cn(
        'rounded-md border border-black/[0.08] bg-white px-4 py-3 transition-colors duration-150',
        interactive && 'cursor-pointer hover:border-sky-200 hover:bg-sky-50/20',
        inUse && 'border-sky-200 bg-sky-50/25',
        className
      )}
      onClick={onClick}
    >
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="flex min-w-0 flex-1 gap-3">
          <div className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-sky-100 bg-sky-50 text-sky-700">
            <Icon className="h-4 w-4" />
          </div>

          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-1.5">
              <h3 className="text-sm font-semibold leading-5 text-gray-900">{name}</h3>
              {badges.map((badge) => (
                <span
                  key={`${badge.label}-${badge.tone ?? 'neutral'}`}
                  className={cn(
                    'rounded border px-1.5 py-0.5 text-[10px] font-semibold leading-none',
                    badgeToneClass[badge.tone ?? 'neutral']
                  )}
                >
                  {badge.label}
                </span>
              ))}
              {inUse && (
                <span className="rounded border border-sky-200 bg-sky-100/70 px-1.5 py-0.5 text-[10px] font-semibold leading-none text-sky-700">
                  {statusLabel}
                </span>
              )}
            </div>

            <div className="mt-0.5 flex flex-wrap items-center gap-x-2 gap-y-0.5 text-xs text-gray-500">
              <span className="font-medium text-gray-600">{provider}</span>
              {metadata.map((item) => (
                <span key={item} className="before:mr-2 before:text-gray-300 before:content-['·']">
                  {item}
                </span>
              ))}
            </div>

            {description && (
              <p className="mt-1.5 max-w-3xl text-xs leading-5 text-gray-600">{description}</p>
            )}
          </div>
        </div>

        <div className="flex shrink-0 items-center justify-between gap-3 pl-11 sm:justify-end sm:pl-0">
          {!inUse && (
            <div className="flex items-center gap-1.5 whitespace-nowrap text-xs font-medium text-gray-500">
              <span className={cn('h-1.5 w-1.5 rounded-full', stateDotClass[state])} />
              {statusLabel}
            </div>
          )}
          {actions && (
            <div
              className="flex flex-wrap items-center justify-end gap-2"
              onClick={(event) => event.stopPropagation()}
            >
              {actions}
            </div>
          )}
        </div>
      </div>

      {progress !== undefined && progress !== null && (
        <div className="mt-3 space-y-1.5 pl-11">
          <div className="h-1.5 overflow-hidden rounded-full bg-gray-100">
            <div
              className="h-full rounded-full bg-sky-600 transition-[width] duration-200"
              style={{ width: `${Math.max(0, Math.min(100, progress))}%` }}
            />
          </div>
          <p className="text-xs text-gray-500">{progressLabel}</p>
        </div>
      )}
    </div>
  );
}
