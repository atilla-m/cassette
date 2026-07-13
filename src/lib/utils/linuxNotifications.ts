import { invoke } from "@tauri-apps/api/core";

export type LinuxNotificationResult =
  | { ok: true }
  | { ok: false; error: string; unavailable: boolean };

export async function sendLinuxNotification(title: string, body: string): Promise<LinuxNotificationResult> {
  if (!isTauriRuntime()) {
    return { ok: false, error: "Tauri runtime is unavailable.", unavailable: true };
  }

  try {
    await invoke("send_linux_notification", { title, body });
    return { ok: true };
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);

    return {
      ok: false,
      error: message,
      unavailable: message.toLowerCase().includes("notify-send is unavailable"),
    };
  }
}

function isTauriRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
