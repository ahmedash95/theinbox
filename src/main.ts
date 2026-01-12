import { createApp } from "vue";
import { isTauri } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { Menu, MenuItem, PredefinedMenuItem, Submenu } from "@tauri-apps/api/menu";
import App from "./App.vue";
import { checkForUpdates } from "./updater";
import "./assets/main.css";

function syncColorScheme() {
  const prefersDark = window.matchMedia("(prefers-color-scheme: dark)");
  const apply = () => {
    document.documentElement.classList.toggle("dark", prefersDark.matches);
  };
  apply();
  prefersDark.addEventListener("change", apply);
}

if (typeof window !== "undefined") {
  syncColorScheme();
}

async function setupAppMenu() {
  if (!isTauri()) return;

  try {
    const aboutItem = await PredefinedMenuItem.new({ item: { About: { name: "TheInbox" } } });
    const checkForUpdatesItem = await MenuItem.new({
      id: "check_for_updates",
      text: "Check for Updates...",
      action: () => {
        void checkForUpdates({ forcePrompt: true });
      },
    });
    const servicesItem = await PredefinedMenuItem.new({ item: "Services" });
    const hideItem = await PredefinedMenuItem.new({ item: "Hide" });
    const hideOthersItem = await PredefinedMenuItem.new({ item: "HideOthers" });
    const showAllItem = await PredefinedMenuItem.new({ item: "ShowAll" });
    const quitItem = await PredefinedMenuItem.new({ item: "Quit" });

    const appMenu = await Submenu.new({
      text: "TheInbox",
      items: [
        aboutItem,
        await PredefinedMenuItem.new({ item: "Separator" }),
        checkForUpdatesItem,
        await PredefinedMenuItem.new({ item: "Separator" }),
        servicesItem,
        await PredefinedMenuItem.new({ item: "Separator" }),
        hideItem,
        hideOthersItem,
        showAllItem,
        await PredefinedMenuItem.new({ item: "Separator" }),
        quitItem,
      ],
    });

    const editMenu = await Submenu.new({
      text: "Edit",
      items: [
        await PredefinedMenuItem.new({ item: "Undo" }),
        await PredefinedMenuItem.new({ item: "Redo" }),
        await PredefinedMenuItem.new({ item: "Separator" }),
        await PredefinedMenuItem.new({ item: "Cut" }),
        await PredefinedMenuItem.new({ item: "Copy" }),
        await PredefinedMenuItem.new({ item: "Paste" }),
        await PredefinedMenuItem.new({ item: "SelectAll" }),
        await PredefinedMenuItem.new({ item: "Separator" }),
        await MenuItem.new({
          id: "rematch_filters",
          text: "Rematch Filters",
          action: () => {
            try {
              localStorage.removeItem("inboxcleanup_filter_backfill_done_v2");
              localStorage.setItem("inboxcleanup_filter_rematch_requested_v2", "1");
            } catch {
              // ignore storage errors
            }
            void emit("request_filter_rematch");
          },
        }),
      ],
    });

    const windowMenu = await Submenu.new({
      text: "Window",
      items: [
        await PredefinedMenuItem.new({ item: "Minimize" }),
        await PredefinedMenuItem.new({ item: "Separator" }),
        await PredefinedMenuItem.new({ item: "CloseWindow" }),
      ],
    });

    const menu = await Menu.new({ items: [appMenu, editMenu, windowMenu] });
    await menu.setAsAppMenu();
  } catch (error) {
    console.warn("[UI] Failed to set app menu", error);
  }
}

void setupAppMenu();

createApp(App).mount("#app");
