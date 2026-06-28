import type { Album, Artist, Track } from "$lib/types/library";

const palette = ["#2f8f83", "#b95f3d", "#8b6bd6", "#c59b40", "#4d84c4", "#b24f72"];

export function buildAlbums(tracks: Track[]): Album[] {
  const albumsByKey = new Map<string, Album>();

  for (const track of tracks) {
    const title = track.album ?? "Unknown Album";
    const artist = track.albumArtist ?? track.artist ?? "Unknown Artist";
    const key = `${artist.toLowerCase()}\u0000${title.toLowerCase()}`;
    const existing = albumsByKey.get(key);

    if (existing) {
      existing.trackCount += 1;
      existing.year ??= track.year;
      existing.coverArtPath ??= track.coverArtPath;
      continue;
    }

    albumsByKey.set(key, {
      id: key,
      title,
      artist,
      year: track.year,
      trackCount: 1,
      color: colorFor(key),
      coverArtPath: track.coverArtPath,
    });
  }

  return [...albumsByKey.values()].sort((left, right) =>
    `${left.artist} ${left.title}`.localeCompare(`${right.artist} ${right.title}`),
  );
}

export function buildArtists(tracks: Track[]): Artist[] {
  const countsByArtist = new Map<string, number>();

  for (const track of tracks) {
    const artist = track.artist ?? track.albumArtist ?? "Unknown Artist";
    countsByArtist.set(artist, (countsByArtist.get(artist) ?? 0) + 1);
  }

  return [...countsByArtist.entries()]
    .map(([name, trackCount]) => ({
      name,
      detail: `${trackCount} ${trackCount === 1 ? "song" : "songs"}`,
      color: colorFor(name),
    }))
    .sort((left, right) => left.name.localeCompare(right.name));
}

function colorFor(value: string) {
  let hash = 0;

  for (let index = 0; index < value.length; index += 1) {
    hash = (hash * 31 + value.charCodeAt(index)) >>> 0;
  }

  return palette[hash % palette.length];
}
