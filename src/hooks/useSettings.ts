import { Store } from "@tauri-apps/plugin-store";
import { useCallback, useEffect, useState } from "react";
import type { AppSettings } from "@/lib/types";

const STORE_FILE = "settings.json";
const DEFAULT_SETTINGS: AppSettings = {
  api_key: null,
  language: "en",
  mic_device_id: null,
  system_device_id: null,
  font_size: 14,
  theme: "system",
  timestamps_enabled: true,
};

export function useSettings() {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [store, setStore] = useState<Store | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Store.load(STORE_FILE).then(async (s) => {
      setStore(s);
      const saved = await s.get<AppSettings>("settings");
      if (saved) setSettings({ ...DEFAULT_SETTINGS, ...saved });
      setLoading(false);
    });
  }, []);

  const updateSettings = useCallback(
    async (updates: Partial<AppSettings>) => {
      const newSettings = { ...settings, ...updates };
      setSettings(newSettings);
      if (store) {
        await store.set("settings", newSettings);
        await store.save();
      }
    },
    [settings, store],
  );

  return { settings, updateSettings, loading };
}
