import { invokeTauri } from "./runtime";
import {
  IS_TAURI,
  ROUTING_API_URL,
  HSL_API_KEY,
  GEOCODING_API_URL,
  ENABLE_DEBUG_LOGGING,
} from "../config";
import { MODE_TO_GTFS_TYPE, isLegIncludedBySelectedModes } from "../constants/transportModes";

// HSL routing API endpoint from build-time config (GraphQL API for route planning)
const API_URL = ROUTING_API_URL;
// Subscription key for HSL APIs (authentication credential)
const API_KEY = HSL_API_KEY;
// Exported flag: true if API is configured and available for use
export const isApiAvailable = Boolean(API_KEY);
const SELECTED_LOCATION_LABEL = "Selected Location";
const isSuccessStatus = (status) => status >= 200 && status < 300;

// Eagerly polls /api/health until the axum REST server is reachable (or gives up after ~10s).
// All local API calls await this promise so they never fire before the backend is ready.
const API_READY = (async () => {
  for (let attempt = 0; attempt < 20; attempt++) {
    try {
      const r = await fetch("/api/health");
      if (r.ok) return;
    } catch {
      // connection refused — server not up yet
    }
    await new Promise((res) => setTimeout(res, 500));
  }
})();

// Helper: Convert degrees to radians for trigonometric calculations
const toRadians = (degrees) => (degrees * Math.PI) / 180;
const hasFiniteCoordinates = (lat, lon) => Number.isFinite(lat) && Number.isFinite(lon);

const EMPTY_POINT_INFO = {
  index: -1,
  distanceMeters: Number.POSITIVE_INFINITY,
};

// Calculate straight-line distance in meters between two lat/lon coordinates (Haversine formula)
const haversineMeters = (lat1, lon1, lat2, lon2) => {
  const earthRadiusMeters = 6371000;
  const dLat = toRadians(lat2 - lat1);
  const dLon = toRadians(lon2 - lon1);
  const rLat1 = toRadians(lat1);
  const rLat2 = toRadians(lat2);

  const a =
    Math.sin(dLat / 2) * Math.sin(dLat / 2) +
    Math.cos(rLat1) * Math.cos(rLat2) * Math.sin(dLon / 2) * Math.sin(dLon / 2);

  return 2 * earthRadiusMeters * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a));
};

// Create a unique key for a stop using its coordinates (for deduplication)
const stopKey = (stop) => {
  if (!stop) return "";
  const lat = Number(stop.lat);
  const lon = Number(stop.lon);
  if (!Number.isFinite(lat) || !Number.isFinite(lon)) return "";
  return `${lat.toFixed(6)}:${lon.toFixed(6)}`;
};

// Convert a stop object to a minimal point with lat/lon (used for local dataset stops)
const normalizeStopPoint = (stop) => ({
  lat: stop.lat,
  lon: stop.lon,
  name: "",
  code: "",
});

const isLegAllowedBySelectedModes = (legMode, selectedModesSet) =>
  isLegIncludedBySelectedModes(legMode, [...selectedModesSet]);

const TRANSPORT_MODE_TO_PLAN_MODE = {
  REGIONAL_BUS: "BUS",
  EXPRESS_BUS: "BUS",
  LOCAL_BUS: "BUS",
  TRAM: "TRAM",
  TRAM_SERVICE: "TRAM",
  SUBURBAN_RAIL: "RAIL",
  COACH: "BUS",
  SUBWAY: "SUBWAY",
  FERRY: "FERRY",
  WALK: "WALK",
};

const buildPlanTransportModes = (selectedModes = []) => {
  if (!Array.isArray(selectedModes) || selectedModes.length === 0) {
    return undefined;
  }

  const planModes = new Set();
  selectedModes.forEach((selectedMode) => {
    const planMode = TRANSPORT_MODE_TO_PLAN_MODE[selectedMode];
    if (planMode) {
      planModes.add(planMode);
    }
  });

  if (planModes.size === 0) {
    return undefined;
  }

  return [...planModes].map((mode) => ({ mode }));
};

const filterItinerariesBySelectedModes = (itineraries, selectedModes = []) => {
  if (!Array.isArray(itineraries) || itineraries.length === 0) {
    return [];
  }

  if (!Array.isArray(selectedModes) || selectedModes.length === 0) {
    return itineraries;
  }

  const selectedModesSet = new Set(selectedModes);
  const includeWalk = selectedModesSet.has("WALK");
  const walkOnly = includeWalk && selectedModesSet.size === 1;

  return itineraries.flatMap((itinerary) => {
    if (!Array.isArray(itinerary.legs) || itinerary.legs.length === 0) {
      return [];
    }

    if (walkOnly) {
      const hasTransitLeg = itinerary.legs.some((leg) => leg.mode !== "WALK");
      if (hasTransitLeg) {
        return [];
      }

      return [itinerary];
    }

    const hasDisallowedTransitLeg = itinerary.legs.some(
      (leg) => leg.mode !== "WALK" && !isLegAllowedBySelectedModes(leg.mode, selectedModesSet),
    );

    if (hasDisallowedTransitLeg) {
      return [];
    }

    if (includeWalk) {
      const hasDisallowedWalkLeg = itinerary.legs.some(
        (leg) => leg.mode === "WALK" && !isLegAllowedBySelectedModes(leg.mode, selectedModesSet),
      );
      if (hasDisallowedWalkLeg) {
        return [];
      }
      return [itinerary];
    }

    const nonWalkLegs = itinerary.legs.filter((leg) => leg.mode !== "WALK");
    if (nonWalkLegs.length === 0) {
      return [];
    }

    const firstLeg = nonWalkLegs[0];
    const lastLeg = nonWalkLegs[nonWalkLegs.length - 1];
    const startTime = firstLeg?.startTime ?? itinerary.startTime;
    const endTime = lastLeg?.endTime ?? itinerary.endTime;
    const calculatedDuration =
      Number.isFinite(startTime) && Number.isFinite(endTime)
        ? Math.max(0, Math.round((endTime - startTime) / 1000))
        : itinerary.duration;

    return [
      {
        ...itinerary,
        legs: nonWalkLegs,
        walkDistance: 0,
        startTime,
        endTime,
        duration: calculatedDuration,
      },
    ];
  });
};

// Find the closest point on a polyline to a given stop; returns index and distance
const nearestPolylinePointInfo = (stop, polylinePoints) => {
  if (!Array.isArray(polylinePoints) || polylinePoints.length === 0) {
    return { ...EMPTY_POINT_INFO };
  }

  let minDistance = Number.POSITIVE_INFINITY;
  let minIndex = -1;
  for (let index = 0; index < polylinePoints.length; index += 1) {
    const point = polylinePoints[index];
    const distance = haversineMeters(stop.lat, stop.lon, point.lat, point.lng);
    if (distance < minDistance) {
      minDistance = distance;
      minIndex = index;
    }
  }

  return {
    index: minIndex,
    distanceMeters: minDistance,
  };
};

const getPolylinePointInfo = (lat, lon, decodedPolyline) => {
  if (!hasFiniteCoordinates(lat, lon)) {
    return { ...EMPTY_POINT_INFO };
  }

  return nearestPolylinePointInfo({ lat, lon }, decodedPolyline);
};

const isStopNearPoint = (stop, lat, lon, maxDistanceMeters) => {
  if (!hasFiniteCoordinates(lat, lon)) {
    return false;
  }

  return haversineMeters(stop.lat, stop.lon, lat, lon) < maxDistanceMeters;
};

const findAdjacentCandidate = (routeCandidates, index, isForward, direction) => {
  if (direction === "before") {
    return isForward
      ? [...routeCandidates].reverse().find((candidate) => candidate.index < index)
      : routeCandidates.find((candidate) => candidate.index > index);
  }

  return isForward
    ? routeCandidates.find((candidate) => candidate.index > index)
    : [...routeCandidates].reverse().find((candidate) => candidate.index < index);
};

/**
 * Enrich itineraries with local HSL stops from the REST API backend.
 * Adds intermediate and extended stops to each leg.
 */
const enrichItinerariesWithLocalStops = async (itineraries) => {
  if (!Array.isArray(itineraries) || itineraries.length === 0) {
    return itineraries;
  }

  const stopsByRouteShortName = new Map();

  const fetchStopsForRouteShortName = async (shortName) => {
    if (!shortName) return [];
    if (stopsByRouteShortName.has(shortName)) {
      return stopsByRouteShortName.get(shortName);
    }

    await API_READY;
    try {
      const params = new URLSearchParams({ shortName });
      const response = await fetch(`/api/routes/stops?${params}`);
      const stops = response.ok ? await response.json() : [];
      const normalizedStops = Array.isArray(stops)
        ? stops
            .map((stop) => ({
              lat: Number(stop.lat),
              lon: Number(stop.lon),
            }))
            .filter((stop) => Number.isFinite(stop.lat) && Number.isFinite(stop.lon))
        : [];
      stopsByRouteShortName.set(shortName, normalizedStops);
      return normalizedStops;
    } catch (error) {
      console.error("Failed to fetch local route stops:", shortName, error);
      stopsByRouteShortName.set(shortName, []);
      return [];
    }
  };

  for (const itinerary of itineraries) {
    if (!Array.isArray(itinerary.legs)) continue;

    for (const leg of itinerary.legs) {
      leg.datasetIntermediateStops = [];
      leg.datasetExtendedStops = [];

      if (leg.mode === "WALK" || !leg.legGeometry?.points) {
        continue;
      }

      const shortName = leg.route?.shortName?.trim();
      if (!shortName) continue;

      const routeStops = await fetchStopsForRouteShortName(shortName);
      if (routeStops.length === 0) continue;

      const decoded = decodePolyline(leg.legGeometry.points);
      if (decoded.length === 0) continue;

      const fromLat = Number(leg.from?.lat);
      const fromLon = Number(leg.from?.lon);
      const toLat = Number(leg.to?.lat);
      const toLon = Number(leg.to?.lon);

      const fromInfo = getPolylinePointInfo(fromLat, fromLon, decoded);
      const toInfo = getPolylinePointInfo(toLat, toLon, decoded);

      if (fromInfo.index < 0 || toInfo.index < 0) continue;

      const isForward = fromInfo.index <= toInfo.index;
      const segmentStart = isForward ? fromInfo.index : toInfo.index;
      const segmentEnd = isForward ? toInfo.index : fromInfo.index;

      const uniqueStops = new Map();
      const routeCandidates = [];

      for (const stop of routeStops) {
        const nearest = nearestPolylinePointInfo(stop, decoded);
        if (nearest.distanceMeters > 120 || nearest.index < 0) continue;

        const nearFrom = isStopNearPoint(stop, fromLat, fromLon, 35);
        const nearTo = isStopNearPoint(stop, toLat, toLon, 35);

        if (nearFrom || nearTo) continue;

        const key = stopKey(stop);
        if (!key || uniqueStops.has(key)) continue;

        uniqueStops.set(key, {
          key,
          index: nearest.index,
          stop: normalizeStopPoint(stop),
        });
      }

      for (const candidate of uniqueStops.values()) {
        routeCandidates.push(candidate);
      }

      routeCandidates.sort((a, b) => a.index - b.index);

      const inSegmentStops = routeCandidates
        .filter((candidate) => candidate.index > segmentStart && candidate.index < segmentEnd)
        .map((candidate) => candidate.stop);

      const beforeCandidate = findAdjacentCandidate(
        routeCandidates,
        fromInfo.index,
        isForward,
        "before",
      );

      const afterCandidate = findAdjacentCandidate(
        routeCandidates,
        toInfo.index,
        isForward,
        "after",
      );

      const extendedStops = [];
      if (beforeCandidate) {
        extendedStops.push(beforeCandidate.stop);
      }
      if (afterCandidate) {
        const afterKey = stopKey(afterCandidate.stop);
        const beforeKey = beforeCandidate ? stopKey(beforeCandidate.stop) : "";
        if (!beforeKey || beforeKey !== afterKey) {
          extendedStops.push(afterCandidate.stop);
        }
      }

      leg.datasetIntermediateStops = inSegmentStops;
      leg.datasetExtendedStops = extendedStops;
    }
  }

  return itineraries;
};

// Helper function to make requests through Tauri proxy
/**
 * Make HTTP requests through Tauri proxy (bypasses CORS restrictions in desktop mode).
 * Supports both JSON and text responses based on content-type header.
 * @param {string} url - The target URL
 * @param {object} options - Fetch options (method, headers, body)
 * @returns {Promise<Response>} Fetch-like response object with json() or text() methods
 */
const fetchThroughProxy = async (url, options = {}) => {
  const request = {
    url,
    method: options.method || "GET",
    headers: options.headers || {},
    body: options.body ? JSON.stringify(options.body) : null,
  };

  try {
    const response = await invokeTauri("fetch_with_proxy", { request });

    if (!isSuccessStatus(response.status)) {
      throw new Error(`HTTP Error: ${response.status}`);
    }

    // Parse JSON response if content-type indicates JSON
    const contentType = response.headers["content-type"] || "";
    if (contentType.includes("application/json")) {
      return {
        ok: isSuccessStatus(response.status),
        status: response.status,
        json: async () => JSON.parse(response.body),
      };
    }

    return {
      ok: isSuccessStatus(response.status),
      status: response.status,
      text: async () => response.body,
    };
  } catch (error) {
    console.error("Proxy request failed:", error);
    throw new Error(`Proxy Error: ${error}`);
  }
};

// Clear the Rust-side cache
/**
 * Clear cached data from Tauri backend (stops, geocoding, temp files).
 * Only works in desktop mode; no-op on web.
 */
export const clearCache = async () => {
  if (!IS_TAURI) return;

  try {
    await invokeTauri("clear_cache");
    console.log("Cache cleared successfully");
  } catch (error) {
    console.error("Failed to clear cache:", error);
  }
};

// Check HTTP status and parse response as JSON; throw error on non-2xx status
const getJsonResponse = async (response) => {
  if (!response.ok) {
    throw new Error(`API Error: ${response.status}`);
  }
  return response.json();
};

// Fetch JSON from a URL in browser mode (no CORS proxy needed)
const fetchJsonWeb = async (url, options = {}) => {
  const response = await fetch(url, options);
  return getJsonResponse(response);
};

const JSON_POST_HEADERS = {
  "Content-Type": "application/json",
};

const createJsonPostOptions = (body) => ({
  method: "POST",
  headers: JSON_POST_HEADERS,
  body,
});

const fetchJsonFromRoutingApi = async (url, requestBody) => {
  if (IS_TAURI) {
    const proxyResponse = await fetchThroughProxy(url, createJsonPostOptions(requestBody));
    return proxyResponse.json();
  }

  return fetchJsonWeb(url, createJsonPostOptions(JSON.stringify(requestBody)));
};

const withSubscriptionKey = (params) => {
  if (API_KEY) {
    params.set("digitransit-subscription-key", API_KEY);
  }
  return params;
};

const getFirstFeature = (response) => response?.features?.[0];

const fetchFirstGeocodingFeature = async (path, params) => {
  const url = `${GEOCODING_API_URL}/${path}?${withSubscriptionKey(params).toString()}`;
  const response = await fetchJsonWeb(url);
  return getFirstFeature(response);
};

const canUseGeocoding = () => true;
const buildSingleResultParams = (params) => new URLSearchParams({ ...params, size: "1" });
const featureLabelOr = (feature, fallback) => feature?.properties?.label || fallback;

// GraphQL query for OpenTripPlanner: fetches multimodal route itineraries with geometry and intermediate stops
const PLAN_QUERY = `
  query plan(
    $fromPlace: String!
    $toPlace: String!
    $date: String
    $time: String
    $arriveBy: Boolean
    $transportModes: [TransportMode]
  ) {
    plan(
      fromPlace: $fromPlace
      toPlace: $toPlace
      walkSpeed: 1.3
      wheelchair: false
      date: $date
      time: $time
      arriveBy: $arriveBy
      transportModes: $transportModes
    ) {
      itineraries {
        startTime
        endTime
        duration
        walkDistance
        legs {
          mode
          route {
            shortName
            longName
            color
          }
          from {
            name
            lat
            lon
            stop {
              code
            }
          }
          to {
            name
            lat
            lon
            stop {
              code
            }
          }
          legGeometry {
            points
          }
          intermediateStops {
            name
            lat
            lon
            code
          }
        }
      }
    }
  }
`;

/**
 * Fetch multimodal route options from HSL API.
 * Plans routes with optional date/time and arrival/departure preferences.
 * Enriches legs with local stop data when in Tauri desktop mode.
 * @param {number} fromLat, fromLon, toLat, toLon - Start and end coordinates
 * @param {string[]} transportModes - Filter modes (e.g., ['BUS', 'SUBWAY'])
 * @param {Date|null} departureTime - When to depart (null = now)
 * @param {boolean} arriveBy - If true, departureTime is arrival time instead
 * @returns {Promise<Array>} Itineraries with legs, geometry, and stop data
 * @throws {Error} If API key is missing or GraphQL returns errors
 */
export const fetchRoutes = async (
  fromLat,
  fromLon,
  toLat,
  toLon,
  transportModes = [],
  departureTime = null,
  arriveBy = false,
) => {
  if (!API_KEY) {
    console.warn("HSL API key is not configured — route search is unavailable.");
    throw new Error("Route search unavailable: HSL API key is not configured.");
  }

  const fromPlace = `${fromLat},${fromLon}`;
  const toPlace = `${toLat},${toLon}`;

  if (ENABLE_DEBUG_LOGGING) {
    console.log("Fetching routes:", {
      fromPlace,
      toPlace,
      transportModes,
      departureTime,
      arriveBy,
    });
  }

  let date, time;
  if (departureTime) {
    const dt = new Date(departureTime);
    const pad = (n) => String(n).padStart(2, "0");
    date = `${dt.getFullYear()}-${pad(dt.getMonth() + 1)}-${pad(dt.getDate())}`;
    time = `${pad(dt.getHours())}:${pad(dt.getMinutes())}:00`;
  }

  const url = `${API_URL}?digitransit-subscription-key=${API_KEY}`;
  const selectedPlanTransportModes = buildPlanTransportModes(transportModes);

  const requestBody = {
    query: PLAN_QUERY,
    variables: {
      fromPlace,
      toPlace,
      ...(date && { date }),
      ...(time && { time }),
      arriveBy,
      ...(selectedPlanTransportModes && { transportModes: selectedPlanTransportModes }),
    },
  };

  const result = await fetchJsonFromRoutingApi(url, requestBody);

  if (ENABLE_DEBUG_LOGGING) {
    console.log("Routes response:", result);
  }

  if (result.errors) {
    console.error("GraphQL errors:", result.errors);
    throw new Error(`GraphQL Error: ${result.errors[0]?.message || "Unknown error"}`);
  }

  const itineraries = result.data?.plan?.itineraries || [];
  const enrichedItineraries = await enrichItinerariesWithLocalStops(itineraries);
  return filterItinerariesBySelectedModes(enrichedItineraries, transportModes);
};

/**
 * Convert an address or place name to coordinates via geocoding API.
 * Returns first match; uses local REST API (works in both browser and Tauri mode).
 * @param {string} query - Place name or address to search
 * @returns {Promise<{lat: number, lon: number, name: string}|null>} First result or null
 */
export const forwardGeocode = async (query) => {
  if (!query?.trim()) return null;

  if (!canUseGeocoding()) return null;

  await API_READY;
  try {
    const params = new URLSearchParams({ query });
    const response = await fetch(`/api/geocode/forward?${params}`);
    if (!response.ok) return null;
    const result = await response.json();
    return { lat: result.lat, lon: result.lon, name: result.name };
  } catch {
    return null;
  }
};

/**
 * Convert coordinates to a readable address via reverse geocoding API.
 * Returns a human-friendly place name or 'Selected Location' as fallback.
 */
export const reverseGeocode = async (lat, lon) => {
  if (!hasFiniteCoordinates(lat, lon)) {
    return SELECTED_LOCATION_LABEL;
  }

  if (!canUseGeocoding()) return SELECTED_LOCATION_LABEL;

  await API_READY;
  try {
    const params = new URLSearchParams({ lat: String(lat), lng: String(lon) });
    const response = await fetch(`/api/geocode/reverse?${params}`);
    if (!response.ok) return SELECTED_LOCATION_LABEL;
    return response.json();
  } catch {
    return SELECTED_LOCATION_LABEL;
  }
};

// Decode Google polyline-encoded geometry string to array of {lat, lng} points
// (polyline format is standard for route APIs, more compact than lat/lon arrays)
export const decodePolyline = (encoded) => {
  let points = [];
  let index = 0,
    lat = 0,
    lng = 0;

  while (index < encoded.length) {
    let b,
      shift = 0,
      result = 0;
    do {
      b = encoded.charCodeAt(index++) - 63;
      result |= (b & 0x1f) << shift;
      shift += 5;
    } while (b >= 0x20);
    let dlat = (result & 1) !== 0 ? ~(result >> 1) : result >> 1;
    lat += dlat;

    shift = 0;
    result = 0;
    do {
      b = encoded.charCodeAt(index++) - 63;
      result |= (b & 0x1f) << shift;
      shift += 5;
    } while (b >= 0x20);
    let dlng = (result & 1) !== 0 ? ~(result >> 1) : result >> 1;
    lng += dlng;

    points.push({ lat: lat / 1e5, lng: lng / 1e5 });
  }
  return points;
};

/**
 * Fetch all polylines (route paths) for a given transport mode from the local HSL dataset.
 * Uses the REST API; returns empty array if mode is unknown or server unreachable.
 * @param {string} mode - OTP transport mode (e.g., 'BUS', 'TRAM', 'SUBWAY')
 * @returns {Promise<Array>} Array of route paths (polyline-encoded geometry)
 */
export const fetchAllRoutePathsForMode = async (mode) => {
  const modeType = MODE_TO_GTFS_TYPE[mode];
  if (modeType === undefined) return [];
  await API_READY;
  try {
    const params = new URLSearchParams({ modeType: String(modeType) });
    const response = await fetch(`/api/routes/all-paths?${params}`);
    if (!response.ok) return [];
    return response.json();
  } catch (error) {
    console.error("Failed to fetch all route paths for mode:", mode, error);
    return [];
  }
};
