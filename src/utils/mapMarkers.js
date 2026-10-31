/**
 * Marker-specific utilities for managing Leaflet markers and their interactions.
 */

import * as L from "leaflet";
import { MODE_COLORS, adjustBrightness } from "./mapHelpers";

/**
 * Get color for a leg's polyline, considering route color or mode color.
 * Darkens color for multiple routes to avoid color collisions.
 */
export const getLegColor = (leg, routeIndex) => {
  if (leg.route?.color) return `#${leg.route.color}`;
  const baseColor = MODE_COLORS[leg.mode] || "#666666";
  if (routeIndex > 0) return adjustBrightness(baseColor, routeIndex * 10);
  return baseColor;
};

/**
 * Set up event listeners on tooltip to prevent map dragging during interaction.
 * Handles all pointer, mouse, and touch events to maintain drag control.
 * Idempotent: applies guards only once per element.
 */
export const setupTooltipInteractionGuards = (tooltipElement, map) => {
  if (!tooltipElement || !map || tooltipElement.dataset.dragGuardApplied === "true") {
    return;
  }

  tooltipElement.dataset.dragGuardApplied = "true";
  L.DomEvent.disableClickPropagation(tooltipElement);
  L.DomEvent.disableScrollPropagation(tooltipElement);

  const stopPropagation = (event) => {
    L.DomEvent.stopPropagation(event);
  };

  const disableDrag = (event) => {
    stopPropagation(event);
    if (map.dragging?.enabled()) {
      map.dragging.disable();
    }
  };

  const enableDrag = () => {
    if (map.dragging && !map.dragging.enabled()) {
      map.dragging.enable();
    }
  };

  tooltipElement.addEventListener("pointerdown", disableDrag);
  tooltipElement.addEventListener("mousedown", disableDrag);
  tooltipElement.addEventListener("touchstart", disableDrag, { passive: false });
  tooltipElement.addEventListener("pointerup", enableDrag);
  tooltipElement.addEventListener("mouseup", enableDrag);
  tooltipElement.addEventListener("touchend", enableDrag);
  tooltipElement.addEventListener("pointercancel", enableDrag);
  tooltipElement.addEventListener("pointerleave", enableDrag);
  tooltipElement.addEventListener("mouseleave", enableDrag);
  window.addEventListener("pointerup", enableDrag);
};
