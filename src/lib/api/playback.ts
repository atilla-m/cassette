import { invoke } from "@tauri-apps/api/core";
import type { PlaybackStatus } from "$lib/types/library";

export async function playTrack(filePath: string): Promise<PlaybackStatus> {
  return invoke<PlaybackStatus>("play_track", { filePath });
}

export async function pausePlayback(): Promise<PlaybackStatus> {
  return invoke<PlaybackStatus>("pause_playback");
}

export async function resumePlayback(): Promise<PlaybackStatus> {
  return invoke<PlaybackStatus>("resume_playback");
}

export async function getPlaybackStatus(): Promise<PlaybackStatus> {
  return invoke<PlaybackStatus>("get_playback_status");
}

export async function seekPlayback(positionSeconds: number): Promise<PlaybackStatus> {
  return invoke<PlaybackStatus>("seek_playback", { positionSeconds });
}

export async function setPlaybackVolume(volume: number): Promise<PlaybackStatus> {
  return invoke<PlaybackStatus>("set_playback_volume", { volume });
}
