import type { Track } from "$lib/types/library";
import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

export const TRACK_CHANGE_NOTIFICATIONS_SETTING_KEY = "cassette:track-change-notifications";
const TRACK_CHANGE_NOTIFICATIONS_PERMISSION_REQUESTED_KEY = "cassette:track-change-notifications-permission-requested";

export type TrackNotificationPermissionState = "granted" | "denied" | "default" | "unavailable";
export type TrackNotificationResultCode =
  | "sent"
  | "permission-default"
  | "permission-denied"
  | "permission-unavailable"
  | "send-failed";
export type TrackNotificationResult = {
  ok: boolean;
  code: TrackNotificationResultCode;
  permission: TrackNotificationPermissionState;
  diagnostics: string[];
  sendAttempted: boolean;
  runningInTauri: boolean;
  errorMessage?: string;
};
export type LinuxNotificationDebugResult = {
  ok: boolean;
  message: string;
  errorMessage?: string;
};

export function trackNotificationKey(track: Track | null) {
  return track ? track.id || track.filePath : null;
}

export async function checkTrackNotificationPermission(): Promise<TrackNotificationPermissionState> {
  return (await resolveNotificationPermission(false)).permission;
}

export async function requestTrackNotificationPermissionOnce(): Promise<TrackNotificationPermissionState> {
  return (await resolveNotificationPermission(true)).permission;
}

export async function notifyTrackChange(track: Track): Promise<TrackNotificationResult> {
  return sendTrackNotification(track.title, notificationBody(track), false);
}

export async function notifyTrackNotificationTest(): Promise<TrackNotificationResult> {
  return sendTrackNotification("Cassette", "Notifications are working.", true);
}

export function isLinuxNotificationDebugAvailable() {
  return isRunningInTauri() && isProbablyLinux();
}

export async function sendLinuxDebugNotification(): Promise<LinuxNotificationDebugResult> {
  debugNotification("debug", "Linux notification debug requested", {
    runningInTauri: isRunningInTauri(),
    isLinux: isProbablyLinux(),
  });

  if (!isRunningInTauri()) {
    return {
      ok: false,
      message: "Debug Linux notification is only available in the Tauri app.",
      errorMessage: "Not running under Tauri.",
    };
  }

  try {
    await invoke("send_linux_test_notification");
    debugNotification("debug", "notify-send fallback command completed");
    return {
      ok: true,
      message: "notify-send called. If nothing appeared, GNOME/system notifications are blocking it.",
    };
  } catch (error) {
    const message = errorMessage(error);
    debugNotification("warn", "notify-send fallback command failed", error);
    return {
      ok: false,
      message: `Debug Linux notification failed: ${message}`,
      errorMessage: message,
    };
  }
}

async function resolveNotificationPermission(requestPermissionIfNeeded: boolean): Promise<Pick<TrackNotificationResult, "permission" | "diagnostics" | "runningInTauri">> {
  const diagnostics: string[] = [];
  const runningInTauri = isRunningInTauri();

  debugNotification("debug", "Notification environment", {
    runningInTauri,
    notificationApiAvailable: hasNotificationApi(),
    browserPermission: currentNotificationPermission(),
  });

  if (!hasNotificationApi()) {
    debugNotification("warn", "Notification API is unavailable");
    diagnostics.push("Notification API unavailable");
    return { permission: "unavailable", diagnostics, runningInTauri };
  }

  try {
    const permissionGranted = await isPermissionGranted();
    debugNotification("debug", "isPermissionGranted result", {
      permissionGranted,
      runningInTauri,
      browserPermission: currentNotificationPermission(),
    });

    if (permissionGranted) {
      storeNotificationPermissionResult("granted");
      diagnostics.push("Permission already granted");
      return { permission: "granted", diagnostics, runningInTauri };
    }

    const currentPermission = currentNotificationPermission();

    if (!requestPermissionIfNeeded) {
      diagnostics.push(`Permission check: ${currentPermission}`);
      return { permission: currentPermission, diagnostics, runningInTauri };
    }

    if (hasRequestedNotificationPermission()) {
      debugNotification("debug", "Notification permission was already requested and is not granted", {
        currentPermission,
      });
      diagnostics.push(`Permission previously requested: ${currentPermission}`);
      return { permission: currentPermission, diagnostics, runningInTauri };
    }

    const permission = await requestPermission();
    debugNotification("debug", "requestPermission result", { permission, runningInTauri });
    diagnostics.push(`Permission requested: ${permission}`);

    const permissionState = notificationPermissionState(permission);
    storeNotificationPermissionResult(permissionState);
    return { permission: permissionState, diagnostics, runningInTauri };
  } catch (error) {
    debugNotification("warn", "Notification permission request failed", error);
    diagnostics.push(`Permission check failed: ${errorMessage(error)}`);
    return { permission: "unavailable", diagnostics, runningInTauri };
  }
}

async function sendTrackNotification(title: string, body: string, requestPermissionIfNeeded: boolean): Promise<TrackNotificationResult> {
  const permissionResult = await resolveNotificationPermission(requestPermissionIfNeeded);
  const { permission, runningInTauri } = permissionResult;
  const diagnostics = [...permissionResult.diagnostics];

  if (permission === "unavailable") {
    return { ok: false, code: "permission-unavailable", permission, diagnostics, sendAttempted: false, runningInTauri };
  }

  if (permission === "default") {
    return { ok: false, code: "permission-default", permission, diagnostics, sendAttempted: false, runningInTauri };
  }

  if (permission !== "granted") {
    return { ok: false, code: "permission-denied", permission, diagnostics, sendAttempted: false, runningInTauri };
  }

  try {
    debugNotification("debug", "sendNotification called", { title, body, runningInTauri });
    diagnostics.push("sendNotification called");
    sendNotification({
      title,
      body,
    });

    diagnostics.push("Notification sent, but OS may have suppressed it");
    return { ok: true, code: "sent", permission, diagnostics, sendAttempted: true, runningInTauri };
  } catch (error) {
    const message = errorMessage(error);
    debugNotification("warn", "sendNotification failed", error);
    diagnostics.push(`sendNotification failed: ${message}`);

    if (requestPermissionIfNeeded && isLinuxNotificationDebugAvailable()) {
      const fallbackResult = await sendLinuxDebugNotification();
      diagnostics.push(fallbackResult.message);
    }

    return {
      ok: false,
      code: "send-failed",
      permission,
      diagnostics,
      sendAttempted: true,
      runningInTauri,
      errorMessage: message,
    };
  }
}

function notificationBody(track: Track) {
  const artist = track.artist ?? track.albumArtist ?? "Unknown Artist";
  const album = track.album ?? "Unknown Album";

  return `${artist} · ${album}`;
}

function hasNotificationApi() {
  return typeof window !== "undefined" && "Notification" in window;
}

function currentNotificationPermission(): TrackNotificationPermissionState {
  if (!hasNotificationApi()) {
    return "unavailable";
  }

  return notificationPermissionState(window.Notification.permission);
}

function notificationPermissionState(permission: NotificationPermission): TrackNotificationPermissionState {
  if (permission === "granted" || permission === "denied" || permission === "default") {
    return permission;
  }

  return "unavailable";
}

function hasRequestedNotificationPermission() {
  if (typeof localStorage === "undefined") {
    return false;
  }

  const storedPermission = localStorage.getItem(TRACK_CHANGE_NOTIFICATIONS_PERMISSION_REQUESTED_KEY);
  return storedPermission === "denied" || storedPermission === "default";
}

function storeNotificationPermissionResult(permission: TrackNotificationPermissionState) {
  if (typeof localStorage === "undefined") {
    return;
  }

  localStorage.setItem(TRACK_CHANGE_NOTIFICATIONS_PERMISSION_REQUESTED_KEY, permission);
}

function isRunningInTauri() {
  try {
    return isTauri();
  } catch {
    return false;
  }
}

function isProbablyLinux() {
  if (typeof navigator === "undefined") {
    return false;
  }

  return /linux/i.test(`${navigator.platform} ${navigator.userAgent}`);
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function debugNotification(level: "debug" | "warn", message: string, details?: unknown) {
  if (!import.meta.env.DEV) {
    return;
  }

  const logger = level === "warn" ? console.warn : console.debug;
  logger(`[Cassette notifications] ${message}`, details ?? "");
}
