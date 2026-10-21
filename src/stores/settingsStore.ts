import { create } from "zustand";
import { settingsApi } from "@/services";
import type { ThemeMode } from "@/types";

interface SettingsState {
  values: Record<string, string>;
  loading: boolean;
  load: () => Promise<void>;
  set: (key: string, value: string) => Promise<void>;
  applyTheme: (theme: ThemeMode) => void;
  theme: ThemeMode;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  values: {},
  loading: false,
  theme: "dark",

  load: async () => {
    set({ loading: true });
    try {
      const pairs = await settingsApi.all();
      const values = Object.fromEntries(pairs);
      set({ values, theme: values["theme"] === "light" ? "light" : "dark" });
      get().applyTheme(get().theme);
    } finally {
      set({ loading: false });
    }
  },

  set: async (key, value) => {
    await settingsApi.set(key, value);
    set((s) => ({ values: { ...s.values, [key]: value } }));
    if (key === "theme") get().applyTheme(value === "light" ? "light" : "dark");
  },

  applyTheme: (theme) => {
    document.documentElement.dataset["theme"] = theme;
    set({ theme });
  },
}));
