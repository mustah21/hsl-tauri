import { useState, useCallback, useEffect } from "react";
import { fetchRoutes } from "../services/hslService";

export function useRouteSearch({
  fromLoc,
  toLoc,
  transportModes,
  departureTime,
  arriveBy,
  drawAllRoutes,
}) {
  const [routes, setRoutes] = useState([]);
  const [selectedRouteIndices, setSelectedRouteIndices] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);

  const handleSearch = useCallback(async () => {
    if (!fromLoc || !toLoc) return;

    if (
      typeof fromLoc.lat !== "number" ||
      typeof fromLoc.lon !== "number" ||
      typeof toLoc.lat !== "number" ||
      typeof toLoc.lon !== "number" ||
      isNaN(fromLoc.lat) ||
      isNaN(fromLoc.lon) ||
      isNaN(toLoc.lat) ||
      isNaN(toLoc.lon)
    ) {
      console.error("Invalid location coordinates:", { fromLoc, toLoc });
      setError("Invalid location coordinates");
      return;
    }

    console.log("Searching routes from", fromLoc, "to", toLoc);
    setLoading(true);
    setError(null);
    setSelectedRouteIndices([]);

    try {
      const results = await fetchRoutes(
        fromLoc.lat,
        fromLoc.lon,
        toLoc.lat,
        toLoc.lon,
        transportModes,
        departureTime,
        arriveBy,
      );
      console.log("Route results:", results?.length || 0, "routes found");
      setRoutes(results);
      if (results.length > 0) {
        setSelectedRouteIndices(drawAllRoutes ? results.map((_, i) => i) : [0]);
      }
    } catch (err) {
      setError(err.message || "Failed to fetch routes");
      console.error("Route search error:", err);
    } finally {
      setLoading(false);
    }
  }, [fromLoc, toLoc, transportModes, departureTime, arriveBy, drawAllRoutes]);

  const toggleRouteSelection = useCallback((index) => {
    setSelectedRouteIndices((prev) => {
      if (prev.includes(index)) return prev.filter((i) => i !== index);
      if (prev.length >= 5) return prev;
      return [...prev, index];
    });
  }, []);

  const clearRoutes = useCallback(() => {
    setRoutes([]);
    setSelectedRouteIndices([]);
  }, []);

  const selectAllRoutes = useCallback(() => {
    setSelectedRouteIndices(routes.map((_, i) => i));
  }, [routes]);

  useEffect(() => {
    if (fromLoc && toLoc) {
      handleSearch();
    }
  }, [fromLoc, toLoc, transportModes, departureTime, arriveBy, handleSearch]);

  return {
    routes,
    selectedRouteIndices,
    loading,
    error,
    toggleRouteSelection,
    clearRoutes,
    selectAllRoutes,
  };
}
