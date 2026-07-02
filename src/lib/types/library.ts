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

export type VideoType = "music_video" | "live_show" | "concert" | "interview_documentary" | "behind_the_scenes" | "other";
export type VideoSource = "local_file" | "dvd_import";

export type VideoEntry = {
  id: string;
  filePath: string;
  fileName: string;
  title: string;
  artist: string | null;
  videoType: VideoType;
  source: VideoSource;
  releaseOrCollection: string | null;
  year: number | null;
  venue: string | null;
  city: string | null;
  country: string | null;
  descriptionOrNotes: string | null;
  durationSeconds: number | null;
  thumbnailPath: string | null;
  lastPositionSeconds: number;
  playCount: number;
  lastPlayedAt: number | null;
  createdAt: number;
  updatedAt: number;
};

export type VideoLibrary = {
  videos: VideoEntry[];
  lastVideoFolder: string | null;
  lastVideoScannedAt: number | null;
};

export type VideoInfoUpdate = {
  title: string;
  artist: string | null;
  videoType: VideoType;
  releaseOrCollection: string | null;
  year: number | null;
  venue: string | null;
  city: string | null;
  country: string | null;
  descriptionOrNotes: string | null;
};

export type DvdDetectResult = {
  found: boolean;
  devicePath: string | null;
  readable: boolean;
  error: string | null;
};

export type DvdTitle = {
  number: number;
  duration: string | null;
  durationSeconds: number | null;
  chapters: number | null;
  likelyMainTitle: boolean;
};

export type DvdTitleScanResult = {
  sourceType: "physical_device" | "video_ts_folder";
  sourcePath: string;
  titles: DvdTitle[];
  rawOutput: string | null;
  error: string | null;
};

export type DvdImportMetadata = {
  title: string;
  artist: string;
  videoType: VideoType;
  releaseOrCollection: string | null;
  year: number | null;
  venue: string | null;
  city: string | null;
  country: string | null;
  descriptionOrNotes: string | null;
  outputFilename: string | null;
};

export type DvdImportResult = {
  video: VideoEntry;
  outputFolder: string;
  outputPath: string;
};

export type DvdImportEvent = {
  outputFolder?: string;
  outputPath?: string;
  titleNumber?: number;
  message?: string;
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

export type VideoPlaybackStatus = {
  videoId: string | null;
  filePath: string | null;
  isPlaying: boolean;
  hasEnded: boolean;
  positionSeconds: number;
  durationSeconds: number | null;
  volume: number;
  hasVideoWindow: boolean;
  isFullscreen: boolean;
  backend: string;
  error: string | null;
};

export type VideoCodecInfo = {
  container: string | null;
  videoCodec: string | null;
  audioCodec: string | null;
  resolution: string | null;
  durationSeconds: number | null;
  error: string | null;
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
  warning?: string | null;
};

export type CdRipMetadataTrack = {
  number: number;
  title: string;
  artist: string;
  discNumber: number | null;
};

export type CdRipCover = {
  source: "cover-art-archive" | "manual";
  path: string;
  mimeType: string;
  extension: string;
};

export type CdRipMetadata = {
  albumArtist: string;
  albumTitle: string;
  year: string;
  genre: string;
  discNumber: number | null;
  cover: CdRipCover | null;
  tracks: CdRipMetadataTrack[];
};

export type CdMetadataRelease = {
  id: string;
  title: string;
  artist: string;
  date: string | null;
  year: string | null;
  country: string | null;
  format: string | null;
  label: string | null;
  catalogNumber: string | null;
  trackCount: number;
  discNumber: number | null;
  tracks: CdRipMetadataTrack[];
};

export type CdMetadataLookupResult = {
  discId: string;
  toc: string;
  releases: CdMetadataRelease[];
  error: string | null;
};

export type CdCoverLookupResult = {
  found: boolean;
  cover: CdRipCover | null;
  message: string | null;
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
