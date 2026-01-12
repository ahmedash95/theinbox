import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent } from "@tauri-apps/plugin-updater";
import { showUpdateConfirm, showUpdateProgress, showUpdateError, showUpdateMessage } from "./updateDialog";

export function formatUpdaterError(error: unknown): string {
  if (error instanceof Error) {
    const parts = [`${error.name}: ${error.message}`.trim()];
    if ("cause" in error && error.cause) {
      parts.push(`Cause: ${formatUpdaterError(error.cause)}`);
    }
    if (error.stack) {
      parts.push(error.stack);
    }
    return parts.join("\n");
  }
  if (typeof error === "string") {
    return error;
  }
  try {
    return JSON.stringify(error, null, 2);
  } catch {
    return String(error);
  }
}

async function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> {
  let timeoutId: number | undefined;
  const timeout = new Promise<never>((_, reject) => {
    timeoutId = window.setTimeout(() => {
      reject(new Error("Update check timed out"));
    }, ms);
  });

  try {
    return await Promise.race([promise, timeout]);
  } finally {
    if (timeoutId !== undefined) {
      window.clearTimeout(timeoutId);
    }
  }
}

// Track progress state for UI updates
let progressCallback: ((progress: number, stage: "downloading" | "installing") => void) | null = null;

export function setProgressCallback(
  callback: ((progress: number, stage: "downloading" | "installing") => void) | null
): void {
  progressCallback = callback;
}

export async function checkForUpdates(options?: { forcePrompt?: boolean }): Promise<void> {
  const forcePrompt = options?.forcePrompt ?? false;
  if (!import.meta.env.PROD) {
    if (forcePrompt) {
      await showUpdateMessage("Updates", "Updates are only checked in release builds.");
    }
    return;
  }

  try {
    const update = await withTimeout(check(), 8000);
    if (!update) {
      if (forcePrompt) {
        await showUpdateMessage("Updates", "You're up to date.");
      }
      return;
    }

    const isAvailable = "available" in update ? update.available : true;
    if (!isAvailable) {
      if (forcePrompt) {
        await showUpdateMessage("Updates", "You're up to date.");
      }
      return;
    }

    const version = "version" in update ? update.version : "latest";
    const shouldInstall = await showUpdateConfirm(
      "Update Available",
      `Update ${version} is available. Install and restart now?`,
      { confirmText: "Install and Restart", cancelText: "Not Now" }
    );
    if (!shouldInstall) {
      return;
    }

    // Show progress dialog and start download
    let contentLength = 0;
    let downloaded = 0;

    await showUpdateProgress("Updating", "Starting download...", 0, "downloading");

    try {
      await update.downloadAndInstall((event: DownloadEvent) => {
        if (event.event === "Started") {
          contentLength = event.data.contentLength ?? 0;
          downloaded = 0;
          const progress = 0;
          progressCallback?.(progress, "downloading");
          void showUpdateProgress(
            "Updating",
            "Downloading update...",
            progress,
            "downloading"
          );
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          const progress = contentLength > 0 ? Math.round((downloaded / contentLength) * 100) : 0;
          progressCallback?.(progress, "downloading");
          void showUpdateProgress(
            "Updating",
            `Downloading... ${formatBytes(downloaded)}${contentLength > 0 ? ` / ${formatBytes(contentLength)}` : ""}`,
            progress,
            "downloading"
          );
        } else if (event.event === "Finished") {
          progressCallback?.(100, "installing");
          void showUpdateProgress(
            "Updating",
            "Installing update...",
            100,
            "installing"
          );
        }
      });

      // Show installing state briefly before relaunch
      await showUpdateProgress("Updating", "Restarting...", 100, "installing");
      await relaunch();
    } catch (downloadError) {
      const detail = formatUpdaterError(downloadError);
      console.error("Update download/install failed:", detail);
      await showUpdateError(
        "Update Failed",
        "Failed to download or install the update. Please try again later.",
        detail
      );
    }
  } catch (error) {
    const detail = formatUpdaterError(error);
    console.warn("Updater check failed:", detail);
    if (forcePrompt) {
      await showUpdateError(
        "Update Check Failed",
        "We couldn't reach the update server. Please try again later.",
        detail
      );
    }
  }
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}
