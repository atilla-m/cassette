import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { LibraryCache, Playlist, Track } from "$lib/types/library";

export async function chooseLibraryFolder(): Promise<string | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    recursive: true,
    title: "Choose Music Folder",
  });

  return typeof selected === "string" ? selected : null;
}

export async function scanLibrary(root: string): Promise<Track[]> {
  return invoke<Track[]>("scan_library", { root });
}

export async function getLibraryCache(): Promise<LibraryCache> {
  return invoke<LibraryCache>("get_library_cache");
}

export async function toggleTrackFavorite(id: string): Promise<boolean> {
  return invoke<boolean>("toggle_track_favorite", { id });
}

export async function recordTrackPlay(id: string): Promise<Track> {
  return invoke<Track>("record_track_play", { id });
}

export async function setAlbumGenres(albumId: string, genres: string[]): Promise<Track[]> {
  return invoke<Track[]>("set_album_genres", { albumId, genres });
}

export async function setArtistGenres(artistName: string, genres: string[]): Promise<Track[]> {
  return invoke<Track[]>("set_artist_genres", { artistName, genres });
}

export async function createPlaylist(name: string): Promise<Playlist> {
  return invoke<Playlist>("create_playlist", { name });
}

export async function addTrackToPlaylist(playlistId: string, trackId: string): Promise<Playlist> {
  return invoke<Playlist>("add_track_to_playlist", { playlistId, trackId });
}

export async function removeTrackFromPlaylist(playlistId: string, trackId: string): Promise<Playlist> {
  return invoke<Playlist>("remove_track_from_playlist", { playlistId, trackId });
}
