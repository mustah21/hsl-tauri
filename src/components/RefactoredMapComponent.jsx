/**
 * RefactoredMapComponent: Main map component orchestrating hooks and UI.
 *
 * Separates concerns:
 * - Hooks manage Leaflet instance, markers, polylines, and interactions
 * - Sub-components render UI (MapContainer, ClickPopupMenu)
 * - All business logic extracted to separate files
 *
 * Much cleaner and easier to maintain than the original 450+ line version.
 */

import React, { useRef } from "react";
import * as L from "leaflet";
import MapContainer from "./MapContainer";
import ClickPopupMenu from "./ClickPopupMenu";
import { useMapInitialization } from "../hooks/useMapInitialization";
import { useMapClickHandler } from "../hooks/useMapClickHandler";
import { useFromToMarkers } from "../hooks/useFromToMarkers";
import { useRoutePolylines } from "../hooks/useRoutePolylines";
import { useAllTransportLines } from "../hooks/useAllTransportLines";
import { useRouteStopMarkers } from "../hooks/useRouteStopMarkers";

// Configure Leaflet default icons
delete L.Icon.Default.prototype._getIconUrl;
L.Icon.Default.mergeOptions({
  iconRetinaUrl: "/images/marker-icon-2x.png",
  iconUrl: "/images/marker-icon.png",
  shadowUrl: "/images/marker-shadow.png",
});

const RefactoredMapComponent = ({
  fromLoc,
  toLoc,
  routes,
  selectedRouteIndices,
  showExtendedStops,
  showEachStopOnRoute,
  showAllStopsForSelectedTransports,
  transportModes,
  modesShowAllLines,
  onSetFrom,
  onSetTo,
  onRouteClick,
}) => {
  const mapRef = useRef(null);

  // Initialize map
  const { mapInstance, isMapReady } = useMapInitialization(mapRef);

  // Manage click-to-popup interaction
  const { clickMenu, setClickMenu } = useMapClickHandler(
    mapInstance,
    isMapReady,
    onSetFrom,
    onSetTo,
  );

  // Manage from/to location markers
  useFromToMarkers(mapInstance, fromLoc, toLoc);

  // Manage route polylines and arrowheads
  useRoutePolylines(mapInstance, routes, selectedRouteIndices, onRouteClick);

  // Manage dimmed background transport mode lines
  useAllTransportLines(mapInstance, modesShowAllLines, isMapReady);

  // Manage stop markers with tooltips and popups
  useRouteStopMarkers(
    mapInstance,
    routes,
    selectedRouteIndices,
    showExtendedStops,
    showEachStopOnRoute,
    showAllStopsForSelectedTransports,
    transportModes,
  );

  return (
    <div className="map-component-wrapper">
      <MapContainer ref={mapRef} />
      <ClickPopupMenu
        clickMenu={clickMenu}
        onSetFrom={onSetFrom}
        onSetTo={onSetTo}
        onClose={() => setClickMenu(null)}
      />
    </div>
  );
};

export default RefactoredMapComponent;
