import type { Track } from "$lib/types/library";

export type AlbumTrackSortKey = "trackNumber" | "title" | "artist" | "duration" | "mostPlayed" | "leastPlayed";
export type AlbumTrackSortDirection = "asc" | "desc";

function compareText(left: string, right: string) {
  return left.localeCompare(right, undefined, { sensitivity: "base" }) || left.localeCompare(right);
}

function compareOptionalNumber(left: number | null, right: number | null) {
  if (left === null) return right === null ? 0 : 1;
  if (right === null) return -1;
  return left - right;
}

function trackOrder(left: Track, right: Track) {
  return compareOptionalNumber(left.discNumber, right.discNumber)
    || compareOptionalNumber(left.trackNumber, right.trackNumber)
    || compareText(left.title, right.title)
    || compareText(left.filePath, right.filePath);
}

function directedOptionalNumber(left: number | null, right: number | null, direction: AlbumTrackSortDirection) {
  if (left === null || right === null) return compareOptionalNumber(left, right);
  return direction === "desc" ? right - left : left - right;
}

// Display-only ordering. Playback and queue actions keep using the album's
// original track-number order, and this never writes tags.
export function sortAlbumDisplayTracks(tracks: Track[], key: AlbumTrackSortKey, direction: AlbumTrackSortDirection) {
  return [...tracks].sort((left, right) => {
    if (key === "mostPlayed" || key === "leastPlayed") {
      return (key === "mostPlayed" ? right.playCount - left.playCount : left.playCount - right.playCount)
        || trackOrder(left, right);
    }

    let comparison = 0;
    if (key === "trackNumber") {
      comparison = directedOptionalNumber(left.discNumber, right.discNumber, direction)
        || directedOptionalNumber(left.trackNumber, right.trackNumber, direction);
      return comparison || trackOrder(left, right);
    }
    if (key === "title") comparison = compareText(left.title, right.title);
    if (key === "artist") comparison = compareText(left.artist ?? "", right.artist ?? "");
    if (key === "duration") {
      return directedOptionalNumber(left.durationSeconds, right.durationSeconds, direction) || trackOrder(left, right);
    }

    return (direction === "desc" ? -comparison : comparison) || trackOrder(left, right);
  });
}
