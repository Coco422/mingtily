const SPEAKER_COLORS = [
  '#2563eb',
  '#7c3aed',
  '#059669',
  '#d97706',
  '#dc2626',
  '#0891b2',
  '#c026d3',
  '#4f46e5',
  '#65a30d',
  '#ea580c',
];

export function formatSpeakerLabel(
  speaker?: string | null,
  translate?: (key: string, options: Record<string, number>) => string,
): string | null {
  if (!speaker) return null;
  const match = /^speaker_(\d+)$/.exec(speaker);
  if (!match) return speaker;
  const number = Number.parseInt(match[1], 10) + 1;
  return translate ? translate('speaker', { number }) : `Speaker ${number}`;
}

export function speakerColor(speaker?: string | null): string {
  if (!speaker) return '#6b7280';
  const match = /^speaker_(\d+)$/.exec(speaker);
  const index = match
    ? Number.parseInt(match[1], 10)
    : Array.from(speaker).reduce((sum, char) => sum + char.charCodeAt(0), 0);
  return SPEAKER_COLORS[index % SPEAKER_COLORS.length];
}

export function prefixSpeaker(
  text: string,
  speaker?: string | null,
  translate?: (key: string, options: Record<string, number>) => string,
): string {
  const label = formatSpeakerLabel(speaker, translate);
  return label ? `${label}: ${text}` : text;
}
