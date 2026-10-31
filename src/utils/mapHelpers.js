/**
 * Helper functions and constants for map operations.
 * Pure functions with no React dependencies.
 */

import {
  MODE_COLORS,
  LEG_MODE_TO_SELECTED_MODES,
  isLegIncludedBySelectedModes,
} from "../constants/transportModes";

export { MODE_COLORS, LEG_MODE_TO_SELECTED_MODES, isLegIncludedBySelectedModes };

/**
 * Check if a leg is included by selected transport modes.
 */
/**
 * Generate unique key for a stop based on code or coordinates.
 */
export const getStopKey = (stop) => {
  if (!stop) return "";
  if (stop.code) return `code:${stop.code}`;
  return `coord:${stop.lat.toFixed(6)}:${stop.lon.toFixed(6)}:${stop.name || ""}`;
};

/**
 * Escape special HTML characters for safe rendering.
 */
export const escapeHtml = (value = "") =>
  value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");

/**
 * Extract intermediate stops from leg, checking multiple possible properties.
 */
export const getIntermediateStops = (leg) => {
  if (Array.isArray(leg.datasetIntermediateStops) && leg.datasetIntermediateStops.length > 0) {
    return leg.datasetIntermediateStops;
  }
  if (Array.isArray(leg.intermediateStops)) return leg.intermediateStops;
  if (Array.isArray(leg.intermediatePlaces)) return leg.intermediatePlaces;
  if (Array.isArray(leg.stops)) return leg.stops.slice(1, -1);
  return [];
};

/**
 * Extract extended stops from leg (additional stops beyond standard list).
 */
export const getExtendedStops = (leg) => {
  if (Array.isArray(leg.datasetExtendedStops) && leg.datasetExtendedStops.length > 0) {
    return leg.datasetExtendedStops;
  }
  return [];
};

/**
 * Format route display label from short/long names.
 */
export const getRouteDisplayLabel = (leg) => {
  const shortName = leg.route?.shortName?.trim();
  const longName = leg.route?.longName?.trim();
  if (shortName && longName) return `${shortName} - ${longName}`;
  if (shortName) return shortName;
  if (longName) return longName;
  return null;
};

/**
 * Normalize stop object to consistent structure.
 * Handles missing/nested properties and validates coordinates.
 */
export const normalizeStop = (stop) => {
  if (!stop) return null;
  const lat = Number(stop.lat ?? stop?.stop?.lat);
  const lon = Number(stop.lon ?? stop?.stop?.lon);
  if (!Number.isFinite(lat) || !Number.isFinite(lon)) return null;

  return {
    lat,
    lon,
    name: stop.name ?? stop?.stop?.name ?? "",
    code: stop.code ?? stop?.stop?.code ?? "",
  };
};

/**
 * Adjust brightness of a hex color by adding/subtracting a value.
 * Used to darken colors for multiple routes at same stops.
 */
export const adjustBrightness = (hex, percent) => {
  const num = parseInt(hex.replace("#", ""), 16);
  const r = Math.min(255, Math.max(0, (num >> 16) + percent));
  const g = Math.min(255, Math.max(0, ((num >> 8) & 0x00ff) + percent));
  const b = Math.min(255, Math.max(0, (num & 0x0000ff) + percent));
  return `#${((r << 16) | (g << 8) | b).toString(16).padStart(6, "0")}`;
};

/**
 * Calculate bearing (compass direction) between two points.
 * Returns degrees 0-360 (0=North, 90=East, 180=South, 270=West).
 */
export const calculateBearing = (p1, p2) => {
  const lat1 = (p1.lat * Math.PI) / 180;
  const lat2 = (p2.lat * Math.PI) / 180;
  const dLng = ((p2.lng - p1.lng) * Math.PI) / 180;
  const y = Math.sin(dLng) * Math.cos(lat2);
  const x = Math.cos(lat1) * Math.sin(lat2) - Math.sin(lat1) * Math.cos(lat2) * Math.cos(dLng);
  return ((Math.atan2(y, x) * 180) / Math.PI + 360) % 360;
};
