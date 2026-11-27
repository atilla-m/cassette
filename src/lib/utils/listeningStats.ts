import type { Track } from "$lib/types/library";

export function listeningDurationLabel(ms: number) {
  const seconds = Math.floor(ms / 1000);
  if (ms > 0 && seconds === 0) return "<1s";
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor(seconds / 60) % 60;
  const remainder = seconds % 60;
  return hours ? `${hours}h ${minutes}m ${remainder}s` : minutes ? `${minutes}m ${remainder}s` : `${seconds}s`;
}

export function listeningTotalLabel(listenedMs: number, playCount: number) {
  return listenedMs === 0 && playCount > 0 ? "Time unavailable" : listeningDurationLabel(listenedMs);
}

export function statsDayValueLabel(day: string | null, chartDates: string[], dailyPlays: number[], label = (count: number) => `${count} ${count === 1 ? "play" : "plays"}`) {
  if (!day) return null;
  const index = chartDates.indexOf(day);
  if (index < 0 || dailyPlays[index] === undefined) return null;
  const count = dailyPlays[index];
  return `${day}: ${label(count)}`;
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
