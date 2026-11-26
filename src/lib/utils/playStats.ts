import type { Album, Artist, Genre, Track } from "$lib/types/library";

export type ActivitySortOrder = "mostPlayed" | "leastPlayed" | "mostListened" | "leastListened";
export type ListeningActivity = Pick<Track, "playCount" | "listenedMs">;

export const activitySortOptions: { value: ActivitySortOrder; label: string }[] = [
  { value: "mostPlayed", label: "Most played" },
  { value: "leastPlayed", label: "Least played" },
  { value: "mostListened", label: "Most listening time" },
  { value: "leastListened", label: "Least listening time" },
];

export function isActivitySortOrder(key: string): key is ActivitySortOrder {
  return activitySortOptions.some((option) => option.value === key);
}

export const STATS_PAGE_SIZE = 50;

function compareText(left: string, right: string) {
  return left.localeCompare(right, undefined, { sensitivity: "base" })
    || left.localeCompare(right);
}

export function compareListeningActivity(left: ListeningActivity, right: ListeningActivity, order: ActivitySortOrder) {
  const byTime = order === "mostListened" || order === "leastListened";
  const difference = byTime ? (left.listenedMs ?? 0) - (right.listenedMs ?? 0) : left.playCount - right.playCount;
  return order === "mostPlayed" || order === "mostListened" ? -difference : difference;
}

export function sortAlbumsByActivity(albums: Album[], order: ActivitySortOrder) {
  return [...albums].sort((left, right) =>
    compareListeningActivity(left, right, order)
    || compareText(left.title, right.title)
    || compareText(left.artist, right.artist)
    || compareText(left.id, right.id),
  );
}

export function sortArtistsByActivity(artists: Artist[], order: ActivitySortOrder) {
  return [...artists].sort((left, right) =>
    compareListeningActivity(left, right, order)
    || compareText(left.name, right.name),
  );
}

export function sortGenresByActivity(genres: Genre[], order: ActivitySortOrder) {
  return [...genres].sort((left, right) =>
    compareListeningActivity(left, right, order)
    || compareText(left.name, right.name),
  );
}

export function rankMostPlayedTracks(tracks: Track[], activity = (track: Track) => track.playCount) {
  return [...tracks]
    .filter((track) => activity(track) > 0)
    .sort((left, right) =>
      activity(right) - activity(left)
      || (right.lastPlayedAt ?? 0) - (left.lastPlayedAt ?? 0)
      || compareText(left.title, right.title)
      || compareText(left.id, right.id),
    );
}

export function rankRecentlyPlayedTracks(tracks: Track[]) {
  return [...tracks]
    .filter((track) => track.lastPlayedAt !== null)
    .sort((left, right) =>
      (right.lastPlayedAt ?? 0) - (left.lastPlayedAt ?? 0)
      || compareText(left.title, right.title)
      || compareText(left.id, right.id),
    );
}

export function nextStatsLimit(currentLimit: number, total: number, pageSize = STATS_PAGE_SIZE) {
  return Math.min(total, currentLimit + pageSize);
}

export function visibleStatsItems<T>(
  items: T[],
  expanded: boolean,
  visibleLimit: number,
  previewLimit: number,
) {
  return items.slice(0, expanded ? visibleLimit : previewLimit);
}
