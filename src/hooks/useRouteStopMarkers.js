/**
 * Hook for rendering stop markers with tooltips and popups.
 * Handles intermediate stops, extended stops, and route labels.
 * Manages tooltip interaction to prevent map dragging during tooltip interaction.
 */

import { useEffect, useRef } from "react";
import * as L from "leaflet";
import {
  getStopKey,
  escapeHtml,
  getIntermediateStops,
  getExtendedStops,
  getRouteDisplayLabel,
  normalizeStop,
  isLegIncludedBySelectedModes,
} from "../utils/mapHelpers";
import { getLegColor, setupTooltipInteractionGuards } from "../utils/mapMarkers";

export const useRouteStopMarkers = (
  mapInstance,
  routes,
  selectedRouteIndices,
  showExtendedStops,
  showEachStopOnRoute,
  showAllStopsForSelectedTransports,
  transportModes,
) => {
  const stopMarkersRef = useRef([]);

  useEffect(() => {
    if (!mapInstance?.current) return;

    stopMarkersRef.current.forEach((m) => mapInstance.current.removeLayer(m));
    stopMarkersRef.current = [];

    if (!routes) return;

    const routeIndices = showAllStopsForSelectedTransports
      ? routes.map((_, index) => index)
      : selectedRouteIndices;

    const stopRouteLabels = new Map();

    // First pass: collect all unique stops and their route labels
    routeIndices.forEach((index) => {
      const route = routes[index];
      if (!route) return;

      route.legs.forEach((leg) => {
        if (
          showAllStopsForSelectedTransports &&
          !isLegIncludedBySelectedModes(leg.mode, transportModes)
        ) {
          return;
        }

        const routeLabel = getRouteDisplayLabel(leg);
        if (!routeLabel) return;

        const allLegStops = [leg.from, ...getIntermediateStops(leg), leg.to];
        allLegStops.forEach((stop) => {
          const normalized = normalizeStop(stop);
          if (!normalized) return;
          const key = getStopKey(normalized);
          if (!key) return;

          if (!stopRouteLabels.has(key)) {
            stopRouteLabels.set(key, new Set());
          }
          stopRouteLabels.get(key).add(routeLabel);
        });
      });
    });

    // Second pass: create markers
    const stopMarkerByKey = new Map();
    const stickyTooltipKeys = new Set();

    routeIndices.forEach((index) => {
      const route = routes[index];
      if (!route) return;

      route.legs.forEach((leg, legIndex) => {
        if (
          showAllStopsForSelectedTransports &&
          !isLegIncludedBySelectedModes(leg.mode, transportModes)
        ) {
          return;
        }

        const legColor = getLegColor(leg, index);

        const addStop = (stop, isEndpoint, stickyLabel) => {
          const normalized = normalizeStop(stop);
          if (!normalized) return;

          const stopKey = getStopKey(normalized);
          if (!stopKey) return;

          const routeList = [...(stopRouteLabels.get(stopKey) || [])].sort();
          const routeDetails =
            routeList.length > 0
              ? `<br><strong>Routes at this stop:</strong><br>${routeList.map((label) => escapeHtml(label)).join("<br>")}`
              : "<br><strong>Routes at this stop:</strong><br>Not available";

          let marker = stopMarkerByKey.get(stopKey);
          if (!marker) {
            marker = new L.CircleMarker([normalized.lat, normalized.lon], {
              radius: isEndpoint ? 14 : 10,
              fillColor: legColor,
              color: "#fff",
              weight: 1,
              opacity: 1,
              fillOpacity: 0.9,
              bubblingMouseEvents: false,
            })
              .bindPopup(
                `<strong>${escapeHtml(normalized.name || "")}</strong>${normalized.code ? `<br>Code: ${escapeHtml(normalized.code)}` : ""}${routeDetails}`,
              )
              .addTo(mapInstance.current);
            stopMarkerByKey.set(stopKey, marker);
            stopMarkersRef.current.push(marker);
          } else {
            marker.setRadius(Math.max(marker.getRadius(), isEndpoint ? 14 : 10));
          }

          if (stickyLabel && normalized.name && !stickyTooltipKeys.has(stopKey)) {
            marker.bindTooltip(escapeHtml(normalized.name), {
              permanent: true,
              interactive: true,
              direction: "top",
              offset: [0, -8],
              className: "stop-sticky-tooltip",
            });
            marker.on("tooltipopen", (event) => {
              const tooltipElement = event.tooltip?.getElement();
              setupTooltipInteractionGuards(tooltipElement, mapInstance.current);
            });
            stickyTooltipKeys.add(stopKey);
          }
        };

        const intermediateStops = getIntermediateStops(leg);

        addStop(leg.from, true, legIndex === 0);
        if (showEachStopOnRoute) {
          intermediateStops.forEach((s) => addStop(s, false, false));
        }

        if (showExtendedStops) {
          getExtendedStops(leg).forEach((s) => addStop(s, false, false));
        }

        const isEndStop = legIndex === route.legs.length - 1;
        const isTransferStop = legIndex < route.legs.length - 1;
        addStop(leg.to, true, isEndStop || isTransferStop);
      });
    });
  }, [
    routes,
    selectedRouteIndices,
    showExtendedStops,
    showEachStopOnRoute,
    showAllStopsForSelectedTransports,
    transportModes,
    mapInstance,
  ]);
};
