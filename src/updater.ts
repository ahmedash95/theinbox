import { relaunch } from "@tauri-apps/api/process";
import { check } from "@tauri-apps/plugin-updater";

export async function checkForUpdates(): Promise<void> {
  if (!import.meta.env.PROD) {
    return;
  }

  try {
    const update = await check();
    if (!update) {
      return;
    }

    const isAvailable = "available" in update ? update.available : true;
    if (!isAvailable) {
      return;
    }

    const version = "version" in update ? update.version : "latest";
    const shouldInstall = window.confirm(
      `Update ${version} is available. Install and restart now?`
    );
    if (!shouldInstall) {
      return;
    }

    await update.downloadAndInstall();
    await relaunch();
  } catch (error) {
    console.warn("Updater check failed", error);
  }
}
