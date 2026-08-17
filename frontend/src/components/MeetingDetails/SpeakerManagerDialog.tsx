'use client';

import { useEffect, useMemo, useState } from 'react';
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { useTranslation } from 'react-i18next';
import { formatSpeakerLabel } from '@/lib/speaker-label';
import type { MeetingSpeakerMap, SpeakerParticipant } from '@/lib/speaker-map';

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  speakerMap: MeetingSpeakerMap;
  initialSpeaker?: string | null;
  onSave: (participants: SpeakerParticipant[]) => Promise<unknown>;
}

export function SpeakerManagerDialog({ open, onOpenChange, speakerMap, initialSpeaker, onSave }: Props) {
  const { t } = useTranslation(['meeting', 'common']);
  const [draft, setDraft] = useState<SpeakerParticipant[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [name, setName] = useState('');
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    setDraft(speakerMap.participants.map((participant) => ({
      ...participant,
      sourceSpeakers: [...participant.sourceSpeakers],
    })));
    setSelected(initialSpeaker ? new Set([initialSpeaker]) : new Set());
    const existing = speakerMap.participants.find((item) => initialSpeaker && item.sourceSpeakers.includes(initialSpeaker));
    setName(existing?.name || '');
  }, [initialSpeaker, open, speakerMap.participants]);

  const matchingParticipant = useMemo(
    () => draft.find((participant) => participant.name.trim() === name.trim()),
    [draft, name],
  );

  const assign = (mergeExisting: boolean) => {
    const cleanName = name.trim();
    if (!cleanName || selected.size === 0) return;
    const sources = [...selected];
    const cleaned = draft
      .map((participant) => ({
        ...participant,
        sourceSpeakers: participant.sourceSpeakers.filter((source) => !selected.has(source)),
      }))
      .filter((participant) => participant.sourceSpeakers.length > 0 || (
        mergeExisting && matchingParticipant?.id === participant.id
      ));
    if (mergeExisting && matchingParticipant) {
      setDraft(cleaned.map((participant) => participant.id === matchingParticipant.id
        ? { ...participant, sourceSpeakers: [...new Set([...participant.sourceSpeakers, ...sources])] }
        : participant));
    } else {
      setDraft([...cleaned, { id: crypto.randomUUID(), name: cleanName, sourceSpeakers: sources }]);
    }
    setSelected(new Set());
    setName('');
  };

  const removeSource = (participantId: string, source: string) => {
    setDraft((current) => current
      .map((participant) => participant.id === participantId
        ? { ...participant, sourceSpeakers: participant.sourceSpeakers.filter((item) => item !== source) }
        : participant)
      .filter((participant) => participant.sourceSpeakers.length > 0));
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      await onSave(draft);
      onOpenChange(false);
    } catch {
      // The save hook reports the conflict or validation error and keeps the dialog open.
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[85vh] max-w-2xl overflow-y-auto">
        <DialogHeader><DialogTitle>{t('meeting:manageSpeakers')}</DialogTitle></DialogHeader>
        <p className="text-sm text-gray-600">{t('meeting:manageSpeakersDescription')}</p>

        <div className="space-y-2">
          {speakerMap.speakers.map((speaker) => (
            <label key={speaker.sourceSpeaker} className="flex cursor-pointer gap-3 rounded-md border p-3">
              <input
                type="checkbox"
                checked={selected.has(speaker.sourceSpeaker)}
                onChange={(event) => setSelected((current) => {
                  const next = new Set(current);
                  event.target.checked ? next.add(speaker.sourceSpeaker) : next.delete(speaker.sourceSpeaker);
                  return next;
                })}
              />
              <div className="min-w-0 flex-1">
                <div className="flex items-center justify-between gap-2 text-sm font-medium">
                  <span>{formatSpeakerLabel(speaker.sourceSpeaker, (key, options) => t(`common:${key}`, options))}</span>
                  <span className="text-xs font-normal text-gray-500">
                    {t('meeting:speakerMapStats', { count: speaker.segmentCount, duration: Math.round(speaker.duration) })}
                  </span>
                </div>
                <p className="mt-1 truncate text-xs text-gray-500">{speaker.sample}</p>
              </div>
            </label>
          ))}
        </div>

        <div className="rounded-md bg-gray-50 p-3">
          <Input value={name} onChange={(event) => setName(event.target.value)} placeholder={t('meeting:speakerNamePlaceholder')} />
          <div className="mt-2 flex flex-wrap gap-2">
            {matchingParticipant ? (
              <>
                <Button size="sm" onClick={() => assign(true)} disabled={!name.trim() || selected.size === 0}>
                  {t('meeting:mergeIntoExisting', { name: matchingParticipant.name })}
                </Button>
                <Button size="sm" variant="outline" onClick={() => assign(false)} disabled={!name.trim() || selected.size === 0}>
                  {t('meeting:createSameName')}
                </Button>
              </>
            ) : (
              <Button size="sm" onClick={() => assign(false)} disabled={!name.trim() || selected.size === 0}>
                {selected.size > 1 ? t('meeting:mergeAndName') : t('meeting:nameSpeaker')}
              </Button>
            )}
          </div>
        </div>

        {draft.length > 0 && (
          <div className="space-y-3 border-t pt-3">
            {draft.map((participant) => (
              <div key={participant.id} className="rounded-md border p-3">
                <Input
                  value={participant.name}
                  onChange={(event) => setDraft((current) => current.map((item) => item.id === participant.id
                    ? { ...item, name: event.target.value }
                    : item))}
                />
                <div className="mt-2 flex flex-wrap gap-2">
                  {participant.sourceSpeakers.map((source) => (
                    <button key={source} type="button" onClick={() => removeSource(participant.id, source)} className="rounded-full bg-blue-50 px-2 py-1 text-xs text-blue-700">
                      {formatSpeakerLabel(source, (key, options) => t(`common:${key}`, options))} ×
                    </button>
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={() => setDraft([])}>{t('meeting:restoreDetectedSpeakers')}</Button>
          <Button variant="outline" onClick={() => onOpenChange(false)}>{t('common:cancel')}</Button>
          <Button onClick={handleSave} disabled={saving || draft.some((item) => !item.name.trim())}>
            {saving ? t('common:processing') : t('common:save')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
