export type Track = {
  id: string;
  filePath: string;
  fileName: string;
  extension: string;
  title: string;
  artist: string | null;
  album: string | null;
  albumArtist: string | null;
  genres: string[];
  trackNumber: number | null;
  discNumber: number | null;
  year: number | null;
  durationSeconds: number | null;
  modifiedTime: number | null;
  fileSize: number | null;
  scannedAt: number | null;
  coverArtPath: string | null;
  isFavorite: boolean;
  playCount: number;
  lastPlayedAt: number | null;
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

export type Genre = {
  name: string;
  songCount: number;
  artistCount: number;
  albumCount: number;
  detail: string;
  color: string;
};

export type Playlist = {
  id: string;
  name: string;
  createdAt: number;
  updatedAt: number;
  trackIds: string[];
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
  playlists: Playlist[];
  lastScannedFolder: string | null;
  lastScannedAt: number | null;
};
