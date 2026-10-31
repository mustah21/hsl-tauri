/**
 * Hook for managing map click-to-popup interaction.
 * Displays a popup menu at clicked location and repositions on map pan.
 */

import { useEffect, useState } from "react";

export const useMapClickHandler = (mapInstance, isMapReady, onSetFrom, onSetTo) => {
  const [clickMenu, setClickMenu] = useState(null);

  const openClickMenu = (latlng) => {
    if (!mapInstance?.current || !latlng) return;
    const point = mapInstance.current.latLngToContainerPoint(latlng);
    setClickMenu({
      lat: latlng.lat,
      lng: latlng.lng,
      x: point.x,
      y: point.y,
    });
  };

  useEffect(() => {
    if (!isMapReady || !mapInstance?.current) return;

    const map = mapInstance.current;

    const handleClick = (e) => {
      openClickMenu(e.latlng);
    };

    const handleMove = () => {
      setClickMenu((prev) => {
        if (!prev) return prev;
        const point = map.latLngToContainerPoint([prev.lat, prev.lng]);
        return {
          ...prev,
          x: point.x,
          y: point.y,
        };
      });
    };

    map.on("click", handleClick);
    map.on("move", handleMove);

    return () => {
      map.off("click", handleClick);
      map.off("move", handleMove);
    };
  }, [isMapReady, mapInstance, onSetFrom, onSetTo]);

  return { clickMenu, setClickMenu };
};
