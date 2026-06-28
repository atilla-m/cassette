export type Track = {
  id: string;
  filePath: string;
  fileName: string;
  extension: string;
  title: string;
  artist: string | null;
  album: string | null;
  albumArtist: string | null;
  trackNumber: number | null;
  discNumber: number | null;
  year: number | null;
  durationSeconds: number | null;
  modifiedTime: number | null;
  fileSize: number | null;
  scannedAt: number | null;
  coverArtPath: string | null;
  isFavorite: boolean;
};

export type Album = {
  id: string;
  title: string;
  artist: string;
  year: number | null;
  trackCount: number;
  color: string;
  coverArtPath: string | null;
};

export type Artist = {
  name: string;
  detail: string;
  color: string;
};

export type NavItem = {
  label: string;
  icon: string;
};

export type PlaybackStatus = {
  filePath: string | null;
  isPlaying: boolean;
  hasEnded: boolean;
  positionSeconds: number;
  durationSeconds: number | null;
  volume: number;
};

export type LibraryCache = {
  tracks: Track[];
  lastScannedFolder: string | null;
  lastScannedAt: number | null;
};
