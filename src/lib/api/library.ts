import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  AutoLyricsResult,
  CdCoverLookupResult,
  CdDetectResult,
  CdMetadataLookupResult,
  CdRipMetadata,
  CdRipResult,
  LibraryCache,
  Playlist,
  Track,
  TrackLyrics,
} from "$lib/types/library";

export async function chooseLibraryFolder(): Promise<string | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    recursive: true,
    title: "Choose Music Folder",
  });

  return typeof selected === "string" ? selected : null;
}

export async function chooseOutputFolder(): Promise<string | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    recursive: true,
    title: "Choose CD Rip Output Folder",
  });

  return typeof selected === "string" ? selected : null;
}

export async function chooseCoverImage(): Promise<string | null> {
  const selected = await open({
    directory: false,
    multiple: false,
    title: "Choose Cover Image",
    filters: [
      {
        name: "Cover images",
        extensions: ["jpg", "jpeg", "png", "webp"],
      },
    ],
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

export async function renamePlaylist(playlistId: string, name: string): Promise<Playlist> {
  return invoke<Playlist>("rename_playlist", { playlistId, name });
}

export async function deletePlaylist(playlistId: string): Promise<void> {
  return invoke<void>("delete_playlist", { playlistId });
}

export async function addTrackToPlaylist(playlistId: string, trackId: string): Promise<Playlist> {
  return invoke<Playlist>("add_track_to_playlist", { playlistId, trackId });
}

export async function removeTrackFromPlaylist(playlistId: string, trackId: string): Promise<Playlist> {
  return invoke<Playlist>("remove_track_from_playlist", { playlistId, trackId });
}

export async function movePlaylistTrack(
  playlistId: string,
  trackId: string,
  direction: "up" | "down",
): Promise<Playlist> {
  return invoke<Playlist>("move_playlist_track", { playlistId, trackId, direction });
}

export async function readTrackLyrics(trackPath: string): Promise<TrackLyrics | null> {
  return invoke<TrackLyrics | null>("read_track_lyrics", { trackPath });
}

export async function autoFindTrackLyrics(trackPath: string, replaceCached = false): Promise<AutoLyricsResult> {
  return invoke<AutoLyricsResult>("auto_find_track_lyrics", { trackPath, replaceCached });
}

export async function detectAudioCd(): Promise<CdDetectResult> {
  return invoke<CdDetectResult>("detect_audio_cd");
}

export async function lookupCdMetadata(): Promise<CdMetadataLookupResult> {
  return invoke<CdMetadataLookupResult>("lookup_cd_metadata");
}

export async function lookupCdCover(releaseId: string): Promise<CdCoverLookupResult> {
  return invoke<CdCoverLookupResult>("lookup_cd_cover", { releaseId });
}

export async function inspectCoverImage(path: string): Promise<CdCoverLookupResult> {
  return invoke<CdCoverLookupResult>("inspect_cover_image", { path });
}

export async function ripCdToFlac(outputFolder: string, metadata: CdRipMetadata | null): Promise<CdRipResult> {
  return invoke<CdRipResult>("rip_cd_to_flac", { outputFolder, metadata });
}
