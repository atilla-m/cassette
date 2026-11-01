import type { CdRipMetadata } from "../types/library";

/** A lookup may fill untouched fields, but may not replace a manual edit. */
export function mergeCdLookupMetadata(
  current: CdRipMetadata,
  incoming: CdRipMetadata,
  editedFields: ReadonlySet<string>,
): CdRipMetadata {
  return {
    ...incoming,
    albumArtist: editedFields.has("albumArtist") ? current.albumArtist : incoming.albumArtist,
    albumTitle: editedFields.has("albumTitle") ? current.albumTitle : incoming.albumTitle,
    year: editedFields.has("year") ? current.year : incoming.year,
    genre: editedFields.has("genre") ? current.genre : incoming.genre,
    discNumber: editedFields.has("discNumber") ? current.discNumber : incoming.discNumber,
    tracks: incoming.tracks.map((track) => {
      const previous = current.tracks.find((candidate) => candidate.number === track.number);
      return {
        ...track,
        title: previous && editedFields.has(`track.${track.number}.title`) ? previous.title : track.title,
        artist: previous && editedFields.has(`track.${track.number}.artist`) ? previous.artist : track.artist,
        discNumber: editedFields.has("discNumber") ? previous?.discNumber ?? current.discNumber : track.discNumber,
      };
    }),
  };
}

/** Own the submitted values; filename sanitization belongs to the native writer. */
export function createCdRipMetadataSnapshot(metadata: CdRipMetadata): CdRipMetadata {
  return {
    ...metadata,
    cover: metadata.cover ? { ...metadata.cover } : null,
    tracks: metadata.tracks.map((track) => ({
      ...track,
      title: track.title.trim() || `Track ${String(track.number).padStart(2, "0")}`,
    })),
  };
}
