export function formatRecordingDuration(seconds: number | null | undefined): string {
  const safeSeconds = Number.isFinite(seconds) ? Math.max(0, Math.floor(seconds ?? 0)) : 0;
  const hours = Math.floor(safeSeconds / 3600);
  const minutes = Math.floor((safeSeconds % 3600) / 60);
  const remainingSeconds = safeSeconds % 60;

  const minuteSecondDuration = `${minutes.toString().padStart(2, '0')}:${remainingSeconds
    .toString()
    .padStart(2, '0')}`;

  return hours > 0
    ? `${hours.toString().padStart(2, '0')}:${minuteSecondDuration}`
    : minuteSecondDuration;
}
