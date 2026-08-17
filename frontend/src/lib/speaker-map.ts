import type { TFunction } from 'i18next';
import { formatSpeakerLabel, speakerColor } from '@/lib/speaker-label';

export interface SpeakerParticipant {
  id: string;
  name: string;
  sourceSpeakers: string[];
}
export interface SpeakerStat {
  sourceSpeaker: string;
  segmentCount: number;
  duration: number;
  sample: string;
}

export interface MeetingSpeakerMap {
  meetingId: string;
  revision: number;
  participants: SpeakerParticipant[];
  speakers: SpeakerStat[];
}

export interface ResolvedSpeaker {
  sourceSpeaker: string;
  participantId: string | null;
  label: string;
  color: string;
}

export function resolveSpeaker(
  speaker: string | null | undefined,
  participants: SpeakerParticipant[],
  translate?: TFunction,
): ResolvedSpeaker | null {
  if (!speaker) return null;
  const participant = participants.find((item) => item.sourceSpeakers.includes(speaker));
  const identity = participant?.id || speaker;
  return {
    sourceSpeaker: speaker,
    participantId: participant?.id || null,
    label: participant?.name || formatSpeakerLabel(
      speaker,
      translate ? (key, options) => translate(`common:${key}`, options) : undefined,
    ) || speaker,
    color: speakerColor(identity),
  };
}

export function prefixResolvedSpeaker(
  text: string,
  speaker: string | null | undefined,
  participants: SpeakerParticipant[],
  translate?: TFunction,
) {
  const resolved = resolveSpeaker(speaker, participants, translate);
  return resolved ? `${resolved.label}: ${text}` : text;
}
