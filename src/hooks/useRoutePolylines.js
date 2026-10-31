/**
 * Hook for rendering route polylines and directional arrowheads.
 * Displays routes as colored lines with directional arrows at intervals.
 * Supports click-to-highlight functionality.
 */

import { useEffect, useRef } from "react";
import * as L from "leaflet";
import { decodePolyline } from "../services/hslService";
import { getLegColor } from "../utils/mapMarkers";
import { calculateBearing } from "../utils/mapHelpers";

export const useRoutePolylines = (mapInstance, routes, selectedRouteIndices, onRouteClick) => {
  const polylinesRef = useRef([]);
  const arrowheadsRef = useRef([]);

  useEffect(() => {
    if (!mapInstance?.current || !routes) return;

    polylinesRef.current.forEach((p) => mapInstance.current.removeLayer(p));
    polylinesRef.current = [];
    arrowheadsRef.current.forEach((a) => mapInstance.current.removeLayer(a));
    arrowheadsRef.current = [];

    const routeIndices = selectedRouteIndices;

    routeIndices.forEach((index) => {
      const route = routes[index];
      if (!route) return;

      route.legs.forEach((leg) => {
        if (!leg.legGeometry?.points) return;
        const latLngs = decodePolyline(leg.legGeometry.points);
        const legColor = getLegColor(leg, index);

        const polyline = new L.Polyline(latLngs, {
          color: legColor,
          weight: 5,
          opacity: 0.7,
          lineJoin: "round",
          bubblingMouseEvents: false,
        })
          .on("click", (e) => {
            L.DomEvent.stopPropagation(e);
            onRouteClick(index);
          })
          .addTo(mapInstance.current);
        polylinesRef.current.push(polyline);

        // Directional arrowheads at regular intervals
        if (latLngs.length >= 2) {
          const step = Math.max(1, Math.floor(latLngs.length / 5));
          for (let i = step; i < latLngs.length - 1; i += step) {
            const bearing = calculateBearing(latLngs[i - 1], latLngs[i]);
            const icon = new L.DivIcon({
              className: "route-arrow-icon",
              html: `<svg class="route-arrow" style="transform:rotate(${bearing - 90}deg);" viewBox="0 0 10 14" xmlns="http://www.w3.org/2000/svg"><polyline points="2,1 8,7 2,13" fill="none" stroke="${legColor}" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/></svg>`,
              iconSize: [18, 18],
              iconAnchor: [9, 9],
            });
            const arrowMarker = new L.Marker([latLngs[i].lat, latLngs[i].lng], {
              icon,
              interactive: false,
              keyboard: false,
            }).addTo(mapInstance.current);
            arrowheadsRef.current.push(arrowMarker);
          }
        }
      });
    });
  }, [routes, selectedRouteIndices, mapInstance, onRouteClick]);
};
