export type ListeningStats = {
  trackCounts: Array<{ trackId: string; plays: number; lastPlayedAt: number | null }>;
  dailyPlays: number[];
  totalPlays: number;
  datedPlays: number;
  undatedLegacyPlays: number;
  detailedHistoryStartedAtUtc: number | null;
  pendingTracks: Array<{ trackId: string; title: string; artist: string | null; album: string | null; albumArtist: string | null; genres: string[] }>;
  coverageSources: Array<{ sourceId: string; detailedTrackingStartedAtUtc: number | null }>;
};

export type ListeningHistoryExport = {
  trackCount: number;
  eventCount: number;
  undatedPlays: number;
};

export type ListeningImportTrack = {
  referenceId: string;
  pendingTrackId: string;
  title: string;
  artist: string | null;
  status: "matched" | "unmatched" | "ambiguous";
  matchedTrackId: string | null;
  candidates: string[];
  newEvents: number;
  duplicateEvents: number;
  undatedPlays: number;
  conflict: string | null;
  retained: boolean;
};

export type ListeningImportPreview = {
  approvalToken: string;
  tracks: ListeningImportTrack[];
  matchedTracks: number;
  unmatchedTracks: number;
  ambiguousTracks: number;
  newEvents: number;
  duplicateEvents: number;
  undatedPlays: number;
  conflicts: string[];
};

export type ListeningImportResult = {
  importedEvents: number;
  pendingTracks: number;
  matchedTracks: number;
  importedUndatedPlays: number;
};
