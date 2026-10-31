/**
 * Hook for rendering dimmed "all lines" for selected transport modes.
 * Fetches and displays all route paths for selected modes with low opacity.
 * Lines are pushed to back so route-specific lines render on top.
 */

import { useEffect, useRef } from "react";
import * as L from "leaflet";
import { fetchAllRoutePathsForMode } from "../services/hslService";
import { MODE_COLORS } from "../utils/mapHelpers";

export const useAllTransportLines = (mapInstance, modesShowAllLines, isMapReady) => {
  const allLinesRef = useRef([]);

  useEffect(() => {
    const map = mapInstance?.current;
    if (!map || !isMapReady) return;

    allLinesRef.current.forEach((p) => map.removeLayer(p));
    allLinesRef.current = [];

    if (!modesShowAllLines || modesShowAllLines.length === 0) return;

    // Keep background transport lines above tiles but below regular overlays.
    if (!map.getPane("all-lines-pane")) {
      const pane = map.createPane("all-lines-pane");
      pane.style.zIndex = "350";
      pane.style.pointerEvents = "none";
    }

    let cancelled = false;

    const fetchAndDraw = async () => {
      try {
        for (const mode of modesShowAllLines) {
          const baseColor = MODE_COLORS[mode] || "#888888";
          const paths = await fetchAllRoutePathsForMode(mode);
          if (cancelled || !mapInstance.current) return;

          for (const routePath of paths) {
            const shapePaths = routePath?.shape_paths || [];
            for (const shapePath of shapePaths) {
              const latLngs = shapePath.map((c) => ({ lat: c.lat, lng: c.lon }));
              if (latLngs.length < 2) continue;

              const polyline = new L.Polyline(latLngs, {
                color: baseColor,
                weight: 2,
                opacity: 0.25,
                lineJoin: "round",
                interactive: false,
                pane: "all-lines-pane",
              }).addTo(map);

              allLinesRef.current.push(polyline);
            }
          }
        }
      } catch (error) {
        console.error("[useAllTransportLines] Failed to render transport lines:", error);
      }
    };

    fetchAndDraw();

    return () => {
      cancelled = true;
      allLinesRef.current.forEach((line) => {
        if (map.hasLayer(line)) {
          map.removeLayer(line);
        }
      });
      allLinesRef.current = [];
    };
  }, [modesShowAllLines, isMapReady, mapInstance]);
};
