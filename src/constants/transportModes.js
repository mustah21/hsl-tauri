export const TRANSPORT_MODES = [
  {
    id: "REGIONAL_BUS",
    label: "Regional bus",
    icon: "🚌",
    color: "#007AC9",
    gtfsType: 701,
    defaultSelected: true,
  },
  {
    id: "EXPRESS_BUS",
    label: "Express bus",
    icon: "🚍",
    color: "#005EA8",
    gtfsType: 702,
    defaultSelected: false,
  },
  {
    id: "LOCAL_BUS",
    label: "Local bus",
    icon: "🚐",
    color: "#26A65B",
    gtfsType: 704,
    defaultSelected: true,
  },
  { id: "TRAM", label: "Tram", icon: "🚊", color: "#00985F", gtfsType: 0, defaultSelected: true },
  {
    id: "TRAM_SERVICE",
    label: "Tram service",
    icon: "🚋",
    color: "#1B9E77",
    gtfsType: 900,
    defaultSelected: false,
  },
  {
    id: "SUBURBAN_RAIL",
    label: "Suburban rail",
    icon: "🚆",
    color: "#8C4799",
    gtfsType: 109,
    defaultSelected: true,
  },
  {
    id: "COACH",
    label: "Coach",
    icon: "🚎",
    color: "#6C3F99",
    gtfsType: 700,
    defaultSelected: false,
  },
  {
    id: "SUBWAY",
    label: "Metro",
    icon: "🚇",
    color: "#FF6319",
    gtfsType: 1,
    defaultSelected: true,
  },
  { id: "FERRY", label: "Ferry", icon: "⛴️", color: "#00B9E4", gtfsType: 4, defaultSelected: true },
  { id: "WALK", label: "Walk", icon: "🚶", color: "#888888", defaultSelected: true },
];

export const TRANSPORT_MODE_IDS = TRANSPORT_MODES.map((mode) => mode.id);

export const DEFAULT_SELECTED_TRANSPORT_MODES = TRANSPORT_MODES.filter(
  (mode) => mode.defaultSelected,
).map((mode) => mode.id);

export const MODE_COLORS = Object.fromEntries(TRANSPORT_MODES.map((mode) => [mode.id, mode.color]));

export const MODE_TO_GTFS_TYPE = Object.fromEntries(
  TRANSPORT_MODES.filter((mode) => Number.isInteger(mode.gtfsType)).map((mode) => [
    mode.id,
    mode.gtfsType,
  ]),
);

export const LEG_MODE_TO_SELECTED_MODES = {
  WALK: ["WALK"],
  BUS: ["REGIONAL_BUS", "EXPRESS_BUS", "LOCAL_BUS", "COACH"],
  TRAM: ["TRAM", "TRAM_SERVICE"],
  RAIL: ["SUBURBAN_RAIL"],
  SUBWAY: ["SUBWAY"],
  FERRY: ["FERRY"],
};

export const isLegIncludedBySelectedModes = (legMode, selectedModes) => {
  const matchingModes = LEG_MODE_TO_SELECTED_MODES[legMode] || [];
  return matchingModes.some((mode) => selectedModes.includes(mode));
};
