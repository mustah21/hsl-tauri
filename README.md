# HSL Routes Planner

A native desktop transit planner for the Helsinki metropolitan area. This is a **React-to-[Yew](https://yew.rs/) migration exercise** where the UI layer will be reimplemented in Yew while the [Tauri](https://tauri.app/) Rust backend provides stable APIs for routing, geocoding, and data access. Route and geocoding logic runs entirely in Rust — no JS for that logic.

## Features

- **Route planning** — enter origin and destination addresses; routes are queried via the [HSL Routing API](https://digitransit.fi/en/developers/apis/1-routing-api/)
- **Interactive map** — rendered with [Leaflet](https://leafletjs.com/); map tiles are fetched through the Rust backend and cached on disk (Might need to look for WebAssembly/Rust alternative).
- **Transport mode filter** — toggle bus (local/regional/express), tram, subway, suburban rail, ferry, and walk
- **Nearby stops** — shows stops close to a given coordinate along with their serving routes
- **Reverse geocoding** — click anywhere on the map to resolve the nearest address
- **Forward geocoding** — type an address; resolved offline from the bundled `helsinki.bin.zst` dataset (in Tauri) or via the [Digitransit Geocoding API](https://digitransit.fi/en/developers/apis/2-geocoding-api/) (web)
- **Stats panel** — displays trip summary (duration, distance, legs)
- **Settings** — persistent UI preferences stored in LocalStorage

## Stack

| Layer         | Technology                                                                                                                                                    |
| :------------ | :------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| UI            | [React 19](https://react.dev/) --> [Yew](https://yew.rs/), [Vite 8](https://vite.dev/) --> [Trunk](https://trunkrs.dev/), [Leaflet 2](https://leafletjs.com/) |
| Desktop shell | [Tauri 2](https://tauri.app/)                                                                                                                                 |
| Rust backend  | `rstar` (R-tree), `bitcode`, `zstd`, SHA-256 cache keys                                                                                                       |
| Data          | [HSL GTFS](https://digitransit.fi/en/developers/apis/6-dataset-api/) (routes), [OpenAddresses](https://openaddresses.io/) (geocoding)                         |
| Testing       | [Playwright](https://playwright.dev/) (Tauri backend with Chromium frontend)                                                                                  |

## Project Structure

```
src/              # React frontend (reference for Yew migration)
src-tauri/        # Rust Tauri backend (stable API contracts, but partly missing implementation)
converter/        # Offline data conversion tools (reference for binary format)
data/             # Pre-built binary datasets (ready to use)
```

## HSL API Token

The [Digitransit APIs](https://digitransit.fi/en/developers/) which is used for routing, geocoding, and map tiles, require a subscription key for all requests.

Without a valid token every API call returns `401 Unauthorized`, so neither route planning, online geocoding, nor map tile fetching will work.
The app should still have functional offline features, only map tiles and route searches are not returning anything.

The key is passed as the `digitransit-subscription-key` HTTP header, as documented in the [API access rights page](https://digitransit.fi/en/developers/api-access-rights/).

### Obtaining a token (optional)

1. Go to the [Digitransit API portal](https://portal-api.digitransit.fi/) and click **Sign up** (top-right).
2. Register with an email address and confirm it via the verification link sent to your inbox.
3. Sign in and open the **Products** tab, then subscribe to the products you need:
   - **Routing API** — for transit route planning ([docs](https://digitransit.fi/en/developers/apis/1-routing-api/))
   - **Geocoding API** — for address search and reverse geocoding ([docs](https://digitransit.fi/en/developers/apis/2-geocoding-api/))
   - **Map API** — for raster map tiles ([docs](https://digitransit.fi/en/developers/apis/3-map-api/))
4. After subscribing, open the **Profile** page and copy the **Primary Key** (or **Secondary Key**) shown under your subscription.
5. Create a `.env` file at the root of `assignment-hsl-routes-react/` and add:

   ```env
   VITE_HSL_API_KEY=your_key_here
   ```

   The Vite dev server and the Tauri build both pick this up automatically at build time.

> **Never commit your token.** The `.env` file is listed in `.gitignore` — keep it that way. Do not hardcode the key anywhere in source files. Anyone who inspects or grades this project is expected to supply their own token in a local `.env` file.

> **Rate limits:** Each product subscription has its own quota. See the [usage policies](https://digitransit.fi/en/developers/api-access-rights/) for details.

## Development

```sh
npm ci
npm run tauri dev
```

## Production Build

```sh
npm run tauri build
```

Installers are placed in `src-tauri/target/release/bundle`.

## Testing

The automated testing in GitHub Actions are done without the HSL API token,
which is why the visual regression tests use images that have no map tiles loaded.

```sh
cd .github/tests/
npm ci
npx playwright install
npm test
```

## Migration Hand-off: React → Yew

This project is designed as a **Yew migration exercise**. The Rust backend is (almost) complete and (nearly) stable; you will reimplement the React UI in Yew while preserving the same architecture.

### What's Stable (Don't Change)

- **Tauri command signatures**: All Rust functions are registered as Tauri IPC commands. Yew will call the same commands (e.g., `forward_geocode()`, `get_route_path()`, `get_nearby_stops_with_routes()`) with identical parameters and return types.
- **Binary data formats**: Pre-built `helsinki.bin.zst` and `routes.bin.zst` are in `/data/` and ready to use.
- **Pipeline tests**: All 40 automated tests remain unchanged. The Playwright test contracts are identical.

### What You Implement

1. **Rust binary loading**:
   - `GeocodingData::load()` in `src-tauri/src/geocoding.rs` — deserialize `/data/helsinki.bin.zst`
   - `RoutesData::load()` in `src-tauri/src/routes.rs` — deserialize `/data/routes.bin.zst`
   - Refer to `converter/src/bin/lookup.rs` and `converter/src/bin/routes.rs` for deserialization patterns (zstd decompression, bitcode deserialization)
   - Refer to `converter/src/lib.rs` for type definitions (`Address`, `AddressPoint`, `Route`, `Shape`, `TransitDataset`)

2. **Yew UI layer** (primary task):
   - Migrate React components → Yew components
   - Migrate React hooks → Yew equivalents
   - Tauri command calls remain unchanged; only hook syntax differs

### Why This Approach

- **Lean learning curve**: Rust backend is battle-tested; you focus on UI patterns, not data formats
- **Real-world contract testing**: Your Yew components call the _same_ Tauri APIs as React did
- **Optional depth**: Implementing `GeocodingData::load()` and `RoutesData::load()` teaches binary data handling; studying the converter code reveals compression, quantization, and spatial indexing design

## React Hooks → Yew Migration Reference

Each React hook used in this app maps (or partially maps) to a Yew built-in or a [yew-hooks](https://github.com/jetli/yew-hooks) hook for the planned React → Yew migration.

Use the following tables as guidance, not as definitive implementation references. Final implementation decisions are yours.

### React built-ins → Yew built-ins

| React hook     | Yew equivalent    | Notes                     |
| :------------- | :---------------- | :------------------------ |
| `useState`     | `use_state`       | Direct equivalent         |
| `useEffect`    | `use_effect_with` | Deps list passed as tuple |
| `useCallback`  | `use_callback`    | Direct equivalent         |
| `useRef` (DOM) | `use_node_ref`    | Returns `NodeRef`         |
| `useMemo`      | `use_memo`        | Direct equivalent         |
| `useContext`   | `use_context`     | Direct equivalent         |

### App patterns → [yew-hooks](https://github.com/jetli/yew-hooks)

| Pattern used                    | React implementation                 | yew-hooks hook                         |
| :------------------------------ | :----------------------------------- | :------------------------------------- |
| Mutable non-DOM ref             | `useRef` + `.current`                | `use_mut_ref` / `use_mut_latest`       |
| Boolean toggle (`arriveBy`)     | `useState(false)` + setter           | `use_bool_toggle`                      |
| localStorage persistence        | `useEffect` + `localStorage.setItem` | `use_local_storage`                    |
| Debounced geocoding input       | `useEffect` + `setTimeout` 2 s       | `use_debounce_state`                   |
| Async data fetch (`buildInfo`)  | `useEffect` + `async fn`             | `use_async`                            |
| Keyboard navigation             | `onKeyDown` prop handler             | `use_event` (attach to window/element) |
| Route list state                | `useState([])` + spread updates      | `use_list`                             |
| Mode toggle map                 | `useState({})` object updates        | `use_map`                              |
| Interval polling (`StatsPanel`) | `useEffect` + `setInterval`          | `use_interval`                         |

### Map hooks — no yew-hooks counterpart

These hooks wrap Leaflet's imperative JS API. Yew has no bindings for Leaflet; they require raw interop via `wasm-bindgen` / `js-sys`.

| React hook             | Purpose                              | Yew approach                                |
| :--------------------- | :----------------------------------- | :------------------------------------------ |
| `useMapInitialization` | Creates Leaflet map, attaches to DOM | `use_effect_with` + `wasm_bindgen::JsValue` |
| `useFromToMarkers`     | Adds/moves from & to markers         | `use_effect_with` + JS interop              |
| `useMapClickHandler`   | Captures map click → popup           | `use_event` on map container element        |
| `useRoutePolylines`    | Draws route polylines on map         | `use_effect_with` + JS interop              |
| `useAllTransportLines` | Draws all-stops lines per mode       | `use_effect_with` + JS interop              |
| `useRouteStopMarkers`  | Renders stop circles with tooltips   | `use_effect_with` + JS interop              |

## Tauri vs. Web

[Tauri](https://tauri.app/) wraps the Vite/React app in a native window using a Rust backend. The frontend code is shared; only API and geocoding calls are routed differently.

**Note on CSP:** Map tiles load from `cdn.digitransit.fi`. Leaflet marker icons are served locally from `public/images/`.

| Feature         | Web (Vite)                      | Tauri Desktop                            |
| :-------------- | :------------------------------ | :--------------------------------------- |
| **Entry Point** | Browser URL                     | Native Window                            |
| **Env Vars**    | `VITE_` prefix                  | `VITE_` prefix (Frontend)                |
| **Storage**     | Browser LocalStorage            | Webview LocalStorage (Filesystem)        |
| **Network**     | Browser CORS Policies           | OS Network Stack (Often more permissive) |
| **Deployment**  | Static Hosting (Netlify/Vercel) | Binary Installers (.exe, .dmg, .deb)     |
| **Updates**     | Instant on refresh              | Requires app update mechanism            |

## Understanding Rust Data Binaries

The Rust backend loads two pre-built binary datasets on startup. You must implement (or complete) the `load()` functions to deserialize them.

All types are defined in `converter/src/lib.rs`.

### Data Flow

```
/data/helsinki.bin.zst
    ↓ (zstd decompress)
    bitcode buffer
    ↓ (bitcode deserialize)
    (RTree<AddressPoint>, Vec<Address>)
    ↓ (GeocodingData struct)
    used by: reverse_geocode(), forward_geocode() Tauri commands

/data/routes.bin.zst
    ↓ (zstd decompress)
    bitcode buffer
    ↓ (bitcode deserialize)
    TransitDataset { stops, routes, shapes, stop_to_routes }
    ↓ (RoutesData struct with indices)
    used by: get_route_path(), get_nearby_stops_with_routes(), etc. Tauri commands
```

### Verification

Once you've implemented `GeocodingData::load()` and `RoutesData::load()`:

```sh
# Should not panic during startup
npm run tauri dev

# Typing an address should trigger reverse geocoding
# Clicking on the map should show nearby stops
```

## Migration Workflow

1. **Understand the Rust contract**
   - Read `converter/src/lib.rs` for type definitions
   - Skim `converter/src/bin/lookup.rs` and `converter/src/bin/routes.rs` for deserialization patterns
   - Review Tauri command signatures in `src-tauri/src/lib.rs` (the `invoke_handler!` macro)

2. **Implement binary loading**
   - Complete `GeocodingData::load()` and `RoutesData::load()`
   - Test with `npm run tauri dev`; verify geocoding and nearby stops queries work
   - Ensure `cargo test --doc` passes

3. **Migrate React components to Yew**
   - Start with simple components (RouteList, TransportModeControl)
   - Move to hook-heavy components (RefactoredMapComponent with all map hooks)
   - Keep CSS/styling unchanged; focus on component hierarchy and state management
   - Map each React hook to Yew equivalent using the reference table below

4. **Test migrations locally**
   - `npm run tauri dev` for manual testing (make sure the app launches and UI renders)

## How the Address Data was generated

Address data sourced from [OpenAddresses](https://batch.openaddresses.io/data#map=4.01/62.9/22.13) (Finland dataset, 1.15 GB uncompressed [NDJSON](https://stevage.github.io/ndgeojson/)). The converter applies four steps to produce a compact binary suitable for embedding in the desktop app:

1. **Geographic filter** — keeps only addresses within the Helsinki metropolitan bounding box (roughly inside Kehä 1)
2. **Coordinate quantization** — stores address coordinates as `u16` (2 bytes) instead of `f64` (8 bytes), giving ~1.5 m precision, which is sufficient for address display
3. **Spatial index** — builds an [R-tree](https://docs.rs/rstar/latest/rstar/) for nearest-neighbour lookups using `f32` (~11 cm precision) to halve index memory versus `f64`
4. **Compression** — serializes with [`bitcode`](https://docs.rs/bitcode) and compresses with [Zstd](https://facebook.github.io/zstd/) level 5

Resulting in `helsinki.bin.zst` at ~1 MB. It is provided within this repository.

## Grading (max 50 points)

Automated scoring up to 35 points:

| Command / Criterion   | Points | Purpose                                                                                       |
| :-------------------- | -----: | :-------------------------------------------------------------------------------------------- |
| `cargo fmt`           |      1 | Consistent code formatting (Rust)                                                             |
| `cargo clippy`        |      2 | Lint checks and idiomatic Rust                                                                |
| `cargo test --doc`    |      3 | Documentation examples are correct and runnable                                               |
| Binary data loading   |      5 | ⚠️ `GeocodingData::load()` and `RoutesData::load()` deserialize binaries correctly at startup |
| No JS                 |      5 | Logic implemented in Rust, not JavaScript                                                     |
| `npm run tauri build` |      4 | Tauri desktop app builds successfully                                                         |
| `npm run test`        |     20 | Tauri usage tests with Playwright pass                                                        |

The final 15 points are evaluated manually by the teachers.
First 5 points are from running the `cargo tarpaulin` command to see that the unit test coverage is at least 92% (includes `GeocodingData::load()`, `RoutesData::load()`).

Then the final 10 points are evaluated by providing API token, running `npm run tauri dev` and searching for routes within Helsinki area, all transport mode options enabled.
Given that results appear in the results list and in the map as per selected from the list, and that address geocoding (via `GeocodingData::load()`) works correctly, this part is passed. No partial points given.

## Final note

The GitHub Actions pipeline takes nearly 30 minutes to complete and will quickly consume the available minutes provided by GitHub.

Please push only when you are ready.

If you see that a build will not be useful, cancel it from the GitHub UI.
Saving compute cycles saves energy, which helps conserve our planet’s resources.

Rust has the benefit of being one of the most “green” programming languages, but for sustainable development, any wasted resource is a loss.
