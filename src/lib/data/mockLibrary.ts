import type { Album, Artist, NavItem, Track } from "$lib/types/library";

export const navItems: NavItem[] = [
  { label: "Home", icon: "H" },
  { label: "Albums", icon: "A" },
  { label: "Artists", icon: "R" },
  { label: "Songs", icon: "S" },
  { label: "Playlists", icon: "P" },
  { label: "Settings", icon: "T" },
];

export const albums: Album[] = [
  {
    id: "mara-vale-afterimage",
    title: "Afterimage",
    artist: "Mara Vale",
    year: 2026,
    trackCount: 11,
    color: "#2f8f83",
    coverArtPath: null,
  },
  {
    id: "north-arcade-signal-bloom",
    title: "Signal Bloom",
    artist: "North Arcade",
    year: 2025,
    trackCount: 9,
    color: "#b95f3d",
    coverArtPath: null,
  },
  {
    id: "echo-lanes-soft-mechanics",
    title: "Soft Mechanics",
    artist: "Echo Lanes",
    year: 2024,
    trackCount: 13,
    color: "#8b6bd6",
    coverArtPath: null,
  },
  {
    id: "june-circuit-late-service",
    title: "Late Service",
    artist: "June Circuit",
    year: 2023,
    trackCount: 8,
    color: "#c59b40",
    coverArtPath: null,
  },
];

export const artists: Artist[] = [
  {
    name: "Mara Vale",
    detail: "24 songs",
    color: "#2f8f83",
  },
  {
    name: "North Arcade",
    detail: "3 albums",
    color: "#b95f3d",
  },
  {
    name: "Echo Lanes",
    detail: "41 songs",
    color: "#8b6bd6",
  },
  {
    name: "June Circuit",
    detail: "18 songs",
    color: "#c59b40",
  },
];

export const nowPlaying: Track = {
  id: "mock-now-playing",
  filePath: "",
  fileName: "low-light-runner.flac",
  extension: "flac",
  title: "Low Light Runner",
  artist: "Mara Vale",
  album: "Afterimage",
  albumArtist: "Mara Vale",
  trackNumber: 1,
  discNumber: 1,
  year: 2026,
  durationSeconds: 252,
  modifiedTime: null,
  fileSize: null,
  scannedAt: null,
  coverArtPath: null,
};
