import { useState, useCallback, useEffect } from "react";
import { DEFAULT_SELECTED_TRANSPORT_MODES, TRANSPORT_MODE_IDS } from "../constants/transportModes";

const SETTINGS_STORAGE_KEY = "hslUiSettings";
const MODES_STORAGE_KEY = "hslTransportModes";
const SHOW_ALL_LINES_STORAGE_KEY = "hslShowAllLines";

const DEFAULT_SETTINGS = {
  showExtendedStops: true,
  showEachStopOnRoute: false,
  drawAllRoutes: false,
  showAllStopsForSelectedTransports: false,
};

const parseStoredJson = (key, fallbackValue) => {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return fallbackValue;
    return JSON.parse(raw);
  } catch {
    return fallbackValue;
  }
};

const normalizeModeIds = (modes) => {
  if (!Array.isArray(modes)) return DEFAULT_SELECTED_TRANSPORT_MODES;
  const uniqueModes = [...new Set(modes)];
  const validModes = uniqueModes.filter((mode) => TRANSPORT_MODE_IDS.includes(mode));
  return validModes.length > 0 ? validModes : DEFAULT_SELECTED_TRANSPORT_MODES;
};

export function useLocalSettings() {
  const [settings, setSettings] = useState(() => ({
    ...DEFAULT_SETTINGS,
    ...parseStoredJson(SETTINGS_STORAGE_KEY, {}),
  }));

  const [transportModes, setTransportModes] = useState(() => {
    const stored = parseStoredJson(MODES_STORAGE_KEY, DEFAULT_SELECTED_TRANSPORT_MODES);
    const normalized = normalizeModeIds(stored);
    if (!Array.isArray(normalized) || normalized.length === 0) {
      return DEFAULT_SELECTED_TRANSPORT_MODES;
    }
    return normalized;
  });

  const [modesShowAllLines, setModesShowAllLines] = useState(() => {
    const stored = parseStoredJson(SHOW_ALL_LINES_STORAGE_KEY, []);
    return normalizeModeIds(stored);
  });

  useEffect(() => {
    localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settings));
  }, [settings]);

  useEffect(() => {
    localStorage.setItem(MODES_STORAGE_KEY, JSON.stringify(transportModes));
  }, [transportModes]);

  useEffect(() => {
    localStorage.setItem(SHOW_ALL_LINES_STORAGE_KEY, JSON.stringify(modesShowAllLines));
  }, [modesShowAllLines]);

  const updateSetting = useCallback((settingName) => {
    setSettings((prev) => ({ ...prev, [settingName]: !prev[settingName] }));
  }, []);

  const handleModeToggle = useCallback((modeId) => {
    setTransportModes((prev) => {
      if (prev.includes(modeId)) {
        if (prev.length === 1) return prev;
        return prev.filter((m) => m !== modeId);
      }
      return [...prev, modeId];
    });
  }, []);

  const handleToggleShowAllLines = useCallback((modeId) => {
    setModesShowAllLines((prev) =>
      prev.includes(modeId) ? prev.filter((m) => m !== modeId) : [...prev, modeId],
    );
  }, []);

  return {
    settings,
    transportModes,
    modesShowAllLines,
    updateSetting,
    handleModeToggle,
    handleToggleShowAllLines,
  };
}
