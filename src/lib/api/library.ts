import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { LibraryCache, Track } from "$lib/types/library";

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
