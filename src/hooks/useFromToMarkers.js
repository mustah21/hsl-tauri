/**
 * Hook for managing from/to location markers on the map.
 * Displays blue marker for start location, red for end location.
 * Keeps current map view unchanged when locations are selected.
 */

import { useEffect, useRef } from "react";
import * as L from "leaflet";

export const useFromToMarkers = (mapInstance, fromLoc, toLoc) => {
  const markersRef = useRef([]);

  useEffect(() => {
    if (!mapInstance?.current) return;

    markersRef.current.forEach((m) => mapInstance.current.removeLayer(m));
    markersRef.current = [];

    const addMarker = (loc, color) => {
      if (!loc) return;
      const marker = new L.CircleMarker([loc.lat, loc.lon], {
        radius: 16,
        fillColor: color,
        color: "#fff",
        weight: 2,
        opacity: 1,
        fillOpacity: 0.8,
        bubblingMouseEvents: false,
      }).addTo(mapInstance.current);
      markersRef.current.push(marker);
    };

    addMarker(fromLoc, "#007bff");
    addMarker(toLoc, "#dc3545");
  }, [fromLoc, toLoc, mapInstance]);
};
