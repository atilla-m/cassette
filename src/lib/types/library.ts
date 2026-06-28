export type Track = {
  id: string;
  path: string;
  fileName: string;
  extension: string;
  title: string;
  artist: string | null;
  album: string | null;
  durationSeconds: number | null;
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

export type NavItem = {
  label: string;
  icon: string;
};
