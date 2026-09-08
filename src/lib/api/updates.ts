import { Channel, invoke, isTauri } from "@tauri-apps/api/core";

export type CassetteUpdate = { version: string; body: string | null; operationId: string };

export const AUTOMATIC_UPDATE_SETTING_KEY = "cassette:auto-check-updates";
export const LAST_SUCCESSFUL_UPDATE_CHECK_KEY = "cassette:last-successful-update-check";
export const LAST_AUTOMATIC_UPDATE_ATTEMPT_KEY = "cassette:last-automatic-update-attempt";
export const UPDATE_CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000;

export type UpdateStorage = Pick<Storage, "getItem" | "setItem" | "removeItem">;

export type StoredUpdaterState = {
  automaticChecksEnabled: boolean;
  lastSuccessfulCheck: number | null;
  lastAutomaticAttempt: number | null;
  storageAvailable: boolean;
};

export type UpdateRuntimeInfo = {
  development: boolean;
  platform: string;
  packageKind: "appimage" | "native-or-unknown" | "unsupported";
  canSelfInstall: boolean;
  updaterAvailable: boolean;
};

export type UpdateDownloadProgress = {
  downloadedBytes: number;
  totalBytes: number | null;
  percent: number | null;
  installing: boolean;
};

export function shouldRunAutomaticUpdateCheck(
  enabled: boolean,
  lastAttempt: number | null,
  now = Date.now(),
): boolean {
  if (!enabled) {
    return false;
  }

  return lastAttempt === null || now - lastAttempt >= UPDATE_CHECK_INTERVAL_MS;
}

// Corrupt/future stored values defer for one interval, never trigger a retry loop.
// A missing new key migrates conservatively from the old successful timestamp.
export function automaticAttemptTimestamp(value: string | null, previousSuccess: string | null, now = Date.now()): number | null {
  const source = value ?? previousSuccess;
  if (source === null) return null;
  const parsed = Number(source);
  return source.trim() && Number.isFinite(parsed) && parsed >= 0 && parsed <= now ? parsed : now;
}

export function storedSuccessfulCheck(value: string | null): number | null {
  if (value === null || value.trim() === "") {
    return null;
  }

  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed >= 0 ? parsed : null;
}

export function loadUpdaterStorage(storage: UpdateStorage, now = Date.now()): StoredUpdaterState {
  try {
    const automaticChecksEnabled = storage.getItem(AUTOMATIC_UPDATE_SETTING_KEY) !== "off";
    const successfulValue = storage.getItem(LAST_SUCCESSFUL_UPDATE_CHECK_KEY);
    const lastSuccessfulCheck = storedSuccessfulCheck(successfulValue);
    const lastAutomaticAttempt = automaticAttemptTimestamp(
      storage.getItem(LAST_AUTOMATIC_UPDATE_ATTEMPT_KEY),
      successfulValue,
      now,
    );
    if (lastAutomaticAttempt !== null) {
      storage.setItem(LAST_AUTOMATIC_UPDATE_ATTEMPT_KEY, String(lastAutomaticAttempt));
    }
    return {
      automaticChecksEnabled,
      lastSuccessfulCheck,
      lastAutomaticAttempt,
      storageAvailable: true,
    };
  } catch {
    // Automatic networking is disabled for this session, while the runtime
    // probe and manually initiated checks remain independent of persistence.
    return {
      automaticChecksEnabled: false,
      lastSuccessfulCheck: null,
      lastAutomaticAttempt: null,
      storageAvailable: false,
    };
  }
}

export function persistUpdaterStorage(
  storage: UpdateStorage,
  key: string,
  value: string | null,
): boolean {
  try {
    if (value === null) storage.removeItem(key);
    else storage.setItem(key, value);
    return true;
  } catch {
    return false;
  }
}

export function updateReleaseUrl(version: string): string {
  return `https://github.com/atilla-m/cassette/releases/tag/v${encodeURIComponent(version)}`;
}

export async function getUpdateRuntimeInfo(): Promise<UpdateRuntimeInfo> {
  if (import.meta.env?.DEV || !isTauri()) {
    return {
      development: true,
      platform: "development",
      packageKind: "unsupported",
      canSelfInstall: false,
      updaterAvailable: false,
    };
  }

  return invoke<UpdateRuntimeInfo>("get_update_runtime_info");
}

export async function checkForCassetteUpdate(runtime: UpdateRuntimeInfo): Promise<CassetteUpdate | null> {
  if (runtime.development || !isTauri()) {
    throw new Error("Update checks are unavailable in development builds.");
  }

  if (runtime.platform !== "linux") {
    throw new Error("Update checks are available only in Linux builds for this beta.");
  }

  if (!runtime.updaterAvailable) throw new Error("Updater configuration is unavailable in this build or session.");
  return invoke<CassetteUpdate | null>("check_cassette_update");
}

export async function installAppImageUpdate(
  update: CassetteUpdate,
  onProgress: (progress: UpdateDownloadProgress) => void,
): Promise<void> {
  const currentRuntime = await getUpdateRuntimeInfo();
  if (
    currentRuntime.development
    || currentRuntime.platform !== "linux"
    || currentRuntime.packageKind !== "appimage"
    || !currentRuntime.canSelfInstall
  ) {
    throw new Error("The AppImage runtime could not be verified immediately before installation.");
  }

  const progress = new Channel<{ downloadedBytes: number; totalBytes: number | null; installing: boolean }>();
  progress.onmessage = (event) => {
    const percent = event.installing ? 100 : event.totalBytes
      ? Math.min(99, Math.round(event.downloadedBytes / event.totalBytes * 100)) : null;
    onProgress({ ...event, percent });
  };
  await invoke("install_cassette_update", { operationId: update.operationId, progress });

  const { relaunch } = await import("@tauri-apps/plugin-process");
  await relaunch();
}
