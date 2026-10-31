/**
 * Build-time configuration for HSL Routes app.
 * Tauri builds run Vite with mode "tauri" (see tauri.conf.json beforeBuildCommand).
 */

/**
 * Application is running in Tauri desktop environment.
 * True when running Tauri build/dev commands (Vite mode "tauri", set in tauri.conf.json).
 */
export const IS_TAURI = import.meta.env.MODE === "tauri";

/**
 * Map tile base URL: whether local Tauri proxy or HSL CDN.
 * Derived from IS_TAURI; hardcoded for reliability rather than env-dependent.
 */
export const MAP_BASE_URL = IS_TAURI
  ? "http://tile.localhost"
  : import.meta.env.VITE_HSL_MAP_BASE_URL || "https://cdn.digitransit.fi/map/v3";

/**
 * Default map center latitude (Helsinki).
 */
export const DEFAULT_LAT = parseFloat(import.meta.env.VITE_DEFAULT_LAT || "60.1699");

/**
 * Default map center longitude (Helsinki).
 */
export const DEFAULT_LNG = parseFloat(import.meta.env.VITE_DEFAULT_LNG || "24.9384");

/**
 * Default map zoom level.
 */
export const DEFAULT_ZOOM = parseInt(import.meta.env.VITE_DEFAULT_ZOOM || "12", 10);

/**
 * Enable tile loading debug output and fallback error tile.
 */
export const ENABLE_TILE_DEBUG = import.meta.env.VITE_ENABLE_TILE_DEBUG === "true";

/**
 * HSL API key for Digitransit endpoints.
 * Only required for web mode; Tauri mode uses local backend proxy.
 */
export const HSL_API_KEY = import.meta.env.VITE_HSL_API_KEY || "";

/**
 * Geocoding API endpoint for address-to-coords conversion.
 * Only used in web mode; Tauri has local equivalent.
 */
export const GEOCODING_API_URL =
  import.meta.env.VITE_HSL_GEOCODING_API_URL || "https://api.digitransit.fi/geocoding/v1";

/**
 * Routing API endpoint for multimodal route planning.
 * Only used in web mode; Tauri has local equivalent.
 */
export const ROUTING_API_URL =
  import.meta.env.VITE_HSL_ROUTING_API_URL || "https://api.digitransit.fi/routing/v2/hsl/gtfs/v1";

/**
 * Enable debug logging for API calls and component lifecycle.
 */
export const ENABLE_DEBUG_LOGGING = import.meta.env.VITE_ENABLE_DEBUG_LOGGING === "true";

/**
 * Tile source: 'local' for Tauri (http://tile.localhost) or 'hsl-cdn' for web.
 * Derived from IS_TAURI; no need to check at runtime.
 */
export const TILE_SOURCE = IS_TAURI ? "local" : "hsl-cdn";

/**
 * Consolidated config object for convenient access.
 */
export const RuntimeConfig = {
  mapBaseUrl: MAP_BASE_URL,
  defaultLat: DEFAULT_LAT,
  defaultLng: DEFAULT_LNG,
  defaultZoom: DEFAULT_ZOOM,
  hslApiKey: HSL_API_KEY,
  geocodingApiUrl: GEOCODING_API_URL,
  routingApiUrl: ROUTING_API_URL,
  enableDebugLogging: ENABLE_DEBUG_LOGGING,
  enableTileDebug: ENABLE_TILE_DEBUG,
  isTauri: IS_TAURI,
  tileSource: TILE_SOURCE,
};
