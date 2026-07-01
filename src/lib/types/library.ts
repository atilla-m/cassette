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
  lyricsPath: string | null;
  lyricsKind: "synced" | "plain" | null;
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

export type TrackLyrics = {
  path: string;
  kind: "synced" | "plain";
  text: string;
  source: "local" | "lrclib";
  fetchedAt: number | null;
  trackPath: string | null;
};

export type AutoLyricsResult = {
  status: "existing" | "synced" | "plain" | "not_found";
  lyrics: TrackLyrics | null;
};

export type CdRipTrackStatus = "pending" | "ripping" | "done" | "error";

export type CdRipTrack = {
  number: number;
  duration: string | null;
  durationSeconds: number | null;
  status?: CdRipTrackStatus;
  outputFilename?: string;
  error?: string | null;
};

export type CdDetectResult = {
  driveFound: boolean;
  discFound: boolean;
  tracks: CdRipTrack[];
  rawOutput: string;
  error: string | null;
};

export type CdRipResult = {
  outputFolder: string;
  tracks: CdRipTrack[];
};

export type CdRipEvent = {
  outputFolder?: string;
  trackNumber?: number;
  outputFilename?: string;
  outputPath?: string;
  message?: string;
};
