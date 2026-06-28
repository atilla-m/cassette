import { convertFileSrc, isTauri } from "@tauri-apps/api/core";

export function localImageSource(path: string | null | undefined): string | null {
  if (!path) {
    return null;
  }

  return isTauri() ? convertFileSrc(path) : path;
}
