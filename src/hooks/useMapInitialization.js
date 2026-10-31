/**
 * Hook for initializing and managing the Leaflet map instance.
 * Handles map creation, tile layer setup, and cleanup.
 */

import { useEffect, useRef, useState } from "react";
import * as L from "leaflet";
import {
  IS_TAURI,
  HSL_API_KEY,
  DEFAULT_LAT,
  DEFAULT_LNG,
  DEFAULT_ZOOM,
  ENABLE_TILE_DEBUG,
  MAP_BASE_URL,
} from "../config";

export const useMapInitialization = (mapRef) => {
  const mapInstance = useRef(null);
  const [isMapReady, setIsMapReady] = useState(false);

  useEffect(() => {
    const mapElement = mapRef?.current;
    if (!mapElement) return;

    const tileBaseUrl = document.body.dataset.mapTilesBaseUrl || MAP_BASE_URL;

    // React StrictMode can remount effects quickly in dev; clear stale Leaflet id if present.
    if (mapElement._leaflet_id) {
      mapElement._leaflet_id = undefined;
    }

    let createdMap = null;
    try {
      createdMap = new L.Map(mapElement).setView([DEFAULT_LAT, DEFAULT_LNG], DEFAULT_ZOOM);
    } catch (error) {
      console.error("[useMapInitialization] Failed to initialize map:", error);
      return;
    }

    mapInstance.current = createdMap;
    window.__leafletMap = createdMap;

    const canLoadTiles = IS_TAURI || Boolean(HSL_API_KEY);
    if (canLoadTiles) {
      const keyParam = HSL_API_KEY ? `?digitransit-subscription-key=${HSL_API_KEY}` : "";
      const tileLayer = new L.TileLayer(`${tileBaseUrl}/hsl-map/{z}/{x}/{y}.png${keyParam}`, {
        attribution: '&copy; <a href="https://www.hsl.fi/hsl/kartta">HSL</a>',
        maxZoom: 18,
        errorTileUrl: ENABLE_TILE_DEBUG
          ? "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
          : undefined,
      });

      if (ENABLE_TILE_DEBUG) {
        tileLayer.on("tileerror", (error) => {
          console.error("[useMapInitialization] Tile loading error:", error);
        });
      }

      tileLayer.addTo(createdMap);
    }
    setIsMapReady(true);

    return () => {
      setIsMapReady(false);
      if (createdMap) {
        try {
          createdMap.remove();
        } catch (error) {
          console.warn("[useMapInitialization] Map cleanup warning:", error);
        }
      }
      if (mapInstance.current === createdMap) {
        mapInstance.current = null;
      }
      if (window.__leafletMap === createdMap) {
        window.__leafletMap = null;
      }
      if (mapElement._leaflet_id) {
        mapElement._leaflet_id = undefined;
      }
    };
  }, [mapRef]);

  return { mapInstance, isMapReady };
};
