import type { Track } from "$lib/types/library";

export function statsDayValueLabel(day: string | null, chartDates: string[], dailyPlays: number[]) {
  if (!day) return null;
  const index = chartDates.indexOf(day);
  if (index < 0 || dailyPlays[index] === undefined) return null;
  const count = dailyPlays[index];
  return `${day}: ${count} ${count === 1 ? "play" : "plays"}`;
}

// Period rows contain display-only counts. Playback and context actions must use
// current library records, never those overlays or an unavailable history row.
export function resolveStatsPlayback(trackId: string, periodQueue: Track[], libraryById: Map<string, Track>) {
  const track = libraryById.get(trackId);
  if (!track) return null;
  const queue = periodQueue.flatMap((item) => {
    const actual = libraryById.get(item.id);
    return actual ? [actual] : [];
  });
  return { track, queue };
}
