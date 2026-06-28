export type NavItem = {
  label: string;
  icon: string;
};

export type Album = {
  title: string;
  artist: string;
  year: number;
  color: string;
};

export type Artist = {
  name: string;
  detail: string;
  color: string;
};

export type Track = {
  title: string;
  artist: string;
  album: string;
  duration: string;
  color: string;
};

export const navItems: NavItem[] = [
  { label: "Home", icon: "H" },
  { label: "Albums", icon: "A" },
  { label: "Artists", icon: "R" },
  { label: "Songs", icon: "S" },
  { label: "Playlists", icon: "P" },
  { label: "Settings", icon: "T" },
];

export const recentlyAdded: Track[] = [
  {
    title: "Low Light Runner",
    artist: "Mara Vale",
    album: "Afterimage",
    duration: "4:12",
    color: "#2f8f83",
  },
  {
    title: "Ceramic Sky",
    artist: "North Arcade",
    album: "Signal Bloom",
    duration: "3:38",
    color: "#b95f3d",
  },
  {
    title: "Tape Return",
    artist: "Echo Lanes",
    album: "Soft Mechanics",
    duration: "5:04",
    color: "#8b6bd6",
  },
  {
    title: "Night Bus Static",
    artist: "June Circuit",
    album: "Late Service",
    duration: "2:57",
    color: "#c59b40",
  },
];

export const albums: Album[] = [
  {
    title: "Afterimage",
    artist: "Mara Vale",
    year: 2026,
    color: "#2f8f83",
  },
  {
    title: "Signal Bloom",
    artist: "North Arcade",
    year: 2025,
    color: "#b95f3d",
  },
  {
    title: "Soft Mechanics",
    artist: "Echo Lanes",
    year: 2024,
    color: "#8b6bd6",
  },
  {
    title: "Late Service",
    artist: "June Circuit",
    year: 2023,
    color: "#c59b40",
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
  title: "Low Light Runner",
  artist: "Mara Vale",
  album: "Afterimage",
  duration: "4:12",
  color: "#2f8f83",
};
