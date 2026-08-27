import { create } from "zustand";
import { safeStorage } from "../utils/storage";

const sidebarCollapsedStorageKey = "ui.sidebar_collapsed.v1";

type UiPreferencesStore = {
  sidebarCollapsed: boolean;
  pluginToolMenuEnabled: boolean;
  setSidebarCollapsed: (collapsed: boolean) => void;
  toggleSidebarCollapsed: () => void;
  setPluginToolMenuEnabled: (enabled: boolean) => void;
};

export const useUiPreferencesStore = create<UiPreferencesStore>((set, get) => ({
  sidebarCollapsed:
    safeStorage.getItem(sidebarCollapsedStorageKey) === "true",
  pluginToolMenuEnabled: true,
  setSidebarCollapsed: (collapsed) => {
    safeStorage.setItem(sidebarCollapsedStorageKey, String(collapsed));
    set({ sidebarCollapsed: collapsed });
  },
  toggleSidebarCollapsed: () => {
    const collapsed = !get().sidebarCollapsed;
    safeStorage.setItem(sidebarCollapsedStorageKey, String(collapsed));
    set({ sidebarCollapsed: collapsed });
  },
  setPluginToolMenuEnabled: (enabled) =>
    set({ pluginToolMenuEnabled: enabled }),
}));
