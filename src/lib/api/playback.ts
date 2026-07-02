import { invoke } from "@tauri-apps/api/core";
import type { PlaybackStatus, VideoCodecInfo, VideoPlaybackStatus } from "$lib/types/library";

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

export async function playVideo(videoId: string, startPositionSeconds: number | null = null): Promise<VideoPlaybackStatus> {
  return invoke<VideoPlaybackStatus>("play_video", { videoId, startPositionSeconds });
}

export async function pauseVideo(): Promise<VideoPlaybackStatus> {
  return invoke<VideoPlaybackStatus>("pause_video");
}

export async function resumeVideo(): Promise<VideoPlaybackStatus> {
  return invoke<VideoPlaybackStatus>("resume_video");
}

export async function stopVideo(): Promise<VideoPlaybackStatus> {
  return invoke<VideoPlaybackStatus>("stop_video");
}

export async function bringVideoWindowToFront(): Promise<VideoPlaybackStatus> {
  return invoke<VideoPlaybackStatus>("bring_video_window_to_front");
}

export async function fullscreenVideoWindow(): Promise<VideoPlaybackStatus> {
  return invoke<VideoPlaybackStatus>("fullscreen_video_window");
}

export async function closeVideoWindow(): Promise<VideoPlaybackStatus> {
  return invoke<VideoPlaybackStatus>("close_video_window");
}

export async function seekVideo(positionSeconds: number): Promise<VideoPlaybackStatus> {
  return invoke<VideoPlaybackStatus>("seek_video", { positionSeconds });
}

export async function setVideoVolume(volume: number): Promise<VideoPlaybackStatus> {
  return invoke<VideoPlaybackStatus>("set_video_volume", { volume });
}

export async function getVideoState(): Promise<VideoPlaybackStatus> {
  return invoke<VideoPlaybackStatus>("get_video_state");
}

export async function getVideoPosition(): Promise<VideoPlaybackStatus> {
  return invoke<VideoPlaybackStatus>("get_video_position");
}

export async function getVideoCodecInfo(videoId: string): Promise<VideoCodecInfo> {
  return invoke<VideoCodecInfo>("get_video_codec_info", { videoId });
}
