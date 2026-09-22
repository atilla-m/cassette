import type { Album, Artist, Genre, Track } from "$lib/types/library";

export type PlayCountOrder = "mostPlayed" | "leastPlayed";

export const STATS_PAGE_SIZE = 50;

function compareText(left: string, right: string) {
  return left.localeCompare(right, undefined, { sensitivity: "base" })
    || left.localeCompare(right);
}

function comparePlayCounts(left: number, right: number, order: PlayCountOrder) {
  return order === "mostPlayed" ? right - left : left - right;
}

export function sortAlbumsByPlayCount(albums: Album[], order: PlayCountOrder) {
  return [...albums].sort((left, right) =>
    comparePlayCounts(left.playCount, right.playCount, order)
    || compareText(left.title, right.title)
    || compareText(left.artist, right.artist)
    || compareText(left.id, right.id),
  );
}

export function sortArtistsByPlayCount(artists: Artist[], order: PlayCountOrder) {
  return [...artists].sort((left, right) =>
    comparePlayCounts(left.playCount, right.playCount, order)
    || compareText(left.name, right.name),
  );
}

export function sortGenresByPlayCount(genres: Genre[], order: PlayCountOrder) {
  return [...genres].sort((left, right) =>
    comparePlayCounts(left.playCount, right.playCount, order)
    || compareText(left.name, right.name),
  );
}

export function rankMostPlayedTracks(tracks: Track[]) {
  return [...tracks]
    .filter((track) => track.playCount > 0)
    .sort((left, right) =>
      right.playCount - left.playCount
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
