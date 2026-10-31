/**
 * MapContainer: Simple presentational component for the Leaflet map DOM element.
 */

import React from "react";

const MapContainer = React.forwardRef(({ ...props }, ref) => (
  <div ref={ref} className="leaflet-mount" {...props} />
));

MapContainer.displayName = "MapContainer";

export default MapContainer;
