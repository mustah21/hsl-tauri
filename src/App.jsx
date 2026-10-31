import React, { useState, useCallback } from "react";
import SplitPane from "./components/SplitPane";
import RouteList from "./components/RouteList";
import RefactoredMapComponent from "./components/RefactoredMapComponent";
import TransportModeControl from "./components/TransportModeControl";
import StatsPanel from "./components/StatsPanel";
import { isApiAvailable } from "./services/hslService";
import { IS_TAURI } from "./config";
import { useGeocoding } from "./hooks/useGeocoding";
import { useRouteSearch } from "./hooks/useRouteSearch";
import { useLocalSettings } from "./hooks/useLocalSettings";
import { useTabNavigation } from "./hooks/useTabNavigation";
import { useBuildInfo } from "./hooks/useBuildInfo";

function App() {
  const { activeTab, setActiveTab, focusedTab, setFocusedTab, tabRefs, handleTabKeyDown } =
    useTabNavigation();

  const {
    settings,
    transportModes,
    modesShowAllLines,
    updateSetting,
    handleModeToggle,
    handleToggleShowAllLines,
  } = useLocalSettings();

  const {
    fromLoc,
    toLoc,
    fromInput,
    setFromInput,
    toInput,
    setToInput,
    fromError,
    setFromError,
    toError,
    setToError,
    handleSetFrom,
    handleSetTo,
    swapLocations,
  } = useGeocoding();

  const [departureTime, setDepartureTime] = useState(() => {
    const now = new Date();
    now.setSeconds(0, 0);
    return now.toISOString().slice(0, 16);
  });
  const [arriveBy, setArriveBy] = useState(false);

  const {
    routes,
    selectedRouteIndices,
    loading,
    error,
    toggleRouteSelection,
    clearRoutes,
    selectAllRoutes,
  } = useRouteSearch({
    fromLoc,
    toLoc,
    transportModes,
    departureTime,
    arriveBy,
    drawAllRoutes: settings.drawAllRoutes,
  });

  const { buildInfo, buildInfoLoading, buildInfoError } = useBuildInfo({
    activeTab,
    isTauri: IS_TAURI,
  });

  // Wrap map-click from-handler to clear stale routes immediately
  const onSetFrom = useCallback(
    async (lat, lon) => {
      clearRoutes();
      await handleSetFrom(lat, lon);
    },
    [clearRoutes, handleSetFrom],
  );

  const handleSettingToggle = useCallback(
    (settingName) => {
      const willEnable = !settings[settingName];
      updateSetting(settingName);
      if (settingName === "drawAllRoutes" && willEnable) {
        selectAllRoutes();
      }
    },
    [settings, updateSetting, selectAllRoutes],
  );

  return (
    <>
      <SplitPane
        leftChildren={
          <div className="left-pane-content">
            <div
              className="left-tabs"
              role="tablist"
              aria-label="Route planner sections"
              aria-orientation="horizontal"
            >
              <button
                type="button"
                className={`left-tab-btn ${activeTab === "routes" ? "active" : ""}`}
                role="tab"
                aria-selected={activeTab === "routes"}
                aria-controls="panel-routes"
                id="tab-routes"
                tabIndex={focusedTab === "routes" ? 0 : -1}
                ref={(node) => {
                  tabRefs.current.routes = node;
                }}
                onFocus={() => setFocusedTab("routes")}
                onKeyDown={(event) => handleTabKeyDown(event, "routes")}
                onClick={() => {
                  setFocusedTab("routes");
                  setActiveTab("routes");
                }}
              >
                Addresses &amp; Routes
              </button>
              <button
                type="button"
                className={`left-tab-btn ${activeTab === "settings" ? "active" : ""}`}
                role="tab"
                aria-selected={activeTab === "settings"}
                aria-controls="panel-settings"
                id="tab-settings"
                tabIndex={focusedTab === "settings" ? 0 : -1}
                ref={(node) => {
                  tabRefs.current.settings = node;
                }}
                onFocus={() => setFocusedTab("settings")}
                onKeyDown={(event) => handleTabKeyDown(event, "settings")}
                onClick={() => {
                  setFocusedTab("settings");
                  setActiveTab("settings");
                }}
              >
                Settings
              </button>
              <button
                type="button"
                className={`left-tab-btn ${activeTab === "about" ? "active" : ""}`}
                role="tab"
                aria-selected={activeTab === "about"}
                aria-controls="panel-about"
                id="tab-about"
                tabIndex={focusedTab === "about" ? 0 : -1}
                ref={(node) => {
                  tabRefs.current.about = node;
                }}
                onFocus={() => setFocusedTab("about")}
                onKeyDown={(event) => handleTabKeyDown(event, "about")}
                onClick={() => {
                  setFocusedTab("about");
                  setActiveTab("about");
                }}
              >
                About
              </button>
            </div>

            {activeTab === "routes" && (
              <div
                id="panel-routes"
                role="tabpanel"
                aria-labelledby="tab-routes"
                className="left-tab-panel routes-tab-panel"
              >
                <div className="search-container">
                  <h1 className="search-title">HSL Route Planner</h1>

                  {!isApiAvailable && !IS_TAURI && (
                    <p className="search-error" role="status">
                      HSL API key is not configured. Route search and map tiles are unavailable.
                    </p>
                  )}

                  <div className="input-group">
                    <label htmlFor="from-input">From</label>
                    <input
                      id="from-input"
                      type="text"
                      value={fromInput}
                      onChange={(e) => {
                        setFromInput(e.target.value);
                        setFromError(false);
                      }}
                      placeholder="Enter address or click map"
                      aria-label="From location address"
                      className={fromError ? "input-error" : ""}
                      disabled={!isApiAvailable && !IS_TAURI}
                    />
                  </div>

                  <div className="swap-locations-row">
                    <button
                      type="button"
                      className="swap-locations-btn"
                      aria-label="Swap from and to locations"
                      onClick={() => {
                        clearRoutes();
                        swapLocations();
                      }}
                      disabled={!isApiAvailable && !IS_TAURI}
                    >
                      ⇅
                    </button>
                  </div>

                  <div className="input-group">
                    <label htmlFor="to-input">To</label>
                    <input
                      id="to-input"
                      type="text"
                      value={toInput}
                      onChange={(e) => {
                        setToInput(e.target.value);
                        setToError(false);
                      }}
                      placeholder="Enter address or click map"
                      aria-label="To location address"
                      className={toError ? "input-error" : ""}
                      disabled={!isApiAvailable && !IS_TAURI}
                    />
                  </div>

                  <div className="input-group">
                    <label htmlFor="time-type-select">Time</label>
                    <div className="time-row">
                      <select
                        id="time-type-select"
                        value={arriveBy ? "arrive" : "depart"}
                        onChange={(e) => setArriveBy(e.target.value === "arrive")}
                        aria-label="Journey time type"
                      >
                        <option value="depart">Depart at</option>
                        <option value="arrive">Arrive by</option>
                      </select>
                      <input
                        id="departure-time"
                        type="datetime-local"
                        value={departureTime}
                        onChange={(e) => setDepartureTime(e.target.value)}
                        aria-label="Departure or arrival time"
                      />
                    </div>
                  </div>

                  {error && (
                    <p className="search-error" role="alert">
                      {error}
                    </p>
                  )}
                  {loading && <p className="search-loading">Loading routes...</p>}

                  <p className="search-tip">
                    Click map to set start/end points, or type an address.
                  </p>
                </div>

                <RouteList
                  routes={routes}
                  selectedIds={selectedRouteIndices}
                  onToggleRoute={toggleRouteSelection}
                />
              </div>
            )}

            {activeTab === "settings" && (
              <div
                id="panel-settings"
                role="tabpanel"
                aria-labelledby="tab-settings"
                className="left-tab-panel"
              >
                <section className="settings-section">
                  <h2 className="left-panel-title">Transport Types</h2>
                  <TransportModeControl
                    selectedModes={transportModes}
                    onModeToggle={handleModeToggle}
                    modesShowAllLines={modesShowAllLines}
                    onToggleShowAllLines={handleToggleShowAllLines}
                    isTauri={IS_TAURI}
                  />
                </section>

                <section className="settings-section">
                  <h2 className="left-panel-title">Route Display</h2>
                  <div className="feature-list">
                    <label className="feature-option">
                      <input
                        type="checkbox"
                        checked={settings.showExtendedStops}
                        onChange={() => handleSettingToggle("showExtendedStops")}
                      />
                      <span className="feature-label">
                        Show extended stops (one before and one after each transit leg)
                      </span>
                    </label>
                    <label className="feature-option">
                      <input
                        type="checkbox"
                        checked={settings.showEachStopOnRoute}
                        onChange={() => handleSettingToggle("showEachStopOnRoute")}
                      />
                      <span className="feature-label">Show each stop on the route</span>
                    </label>
                    <label className="feature-option">
                      <input
                        type="checkbox"
                        checked={settings.drawAllRoutes}
                        onChange={() => handleSettingToggle("drawAllRoutes")}
                      />
                      <span className="feature-label">Draw full route lines for all results</span>
                    </label>
                    <label className="feature-option">
                      <input
                        type="checkbox"
                        checked={settings.showAllStopsForSelectedTransports}
                        onChange={() => handleSettingToggle("showAllStopsForSelectedTransports")}
                      />
                      <span className="feature-label">
                        Show all stops for selected transport types
                      </span>
                    </label>
                  </div>
                </section>
              </div>
            )}

            {activeTab === "about" && (
              <div
                id="panel-about"
                role="tabpanel"
                aria-labelledby="tab-about"
                className="left-tab-panel"
              >
                <section className="settings-section">
                  <h2 className="left-panel-title">Dataset Metadata</h2>
                  {buildInfoError && (
                    <p className="search-error" role="alert">
                      {buildInfoError}
                    </p>
                  )}
                  {buildInfoLoading && <p className="search-loading">Loading dataset stats...</p>}
                  {buildInfo && (
                    <dl className="stats-list about-list">
                      <div className="stats-row">
                        <dt>Stops</dt>
                        <dd>{buildInfo.stops}</dd>
                      </div>
                      <div className="stats-row">
                        <dt>Routes</dt>
                        <dd>{buildInfo.routes}</dd>
                      </div>
                      <div className="stats-row">
                        <dt>Shapes</dt>
                        <dd>{buildInfo.shapes}</dd>
                      </div>
                      <div className="stats-row">
                        <dt>Stop-route links</dt>
                        <dd>{buildInfo.stop_routes}</dd>
                      </div>
                    </dl>
                  )}
                  {!IS_TAURI && (
                    <p className="search-tip">Dataset stats are available in Tauri runtime.</p>
                  )}
                </section>

                <section className="settings-section">
                  <h2 className="left-panel-title">Cache, Memory &amp; Runtime</h2>
                  <StatsPanel isActive={activeTab === "about"} />
                </section>
              </div>
            )}
          </div>
        }
        rightChildren={
          <RefactoredMapComponent
            fromLoc={fromLoc}
            toLoc={toLoc}
            routes={routes}
            selectedRouteIndices={selectedRouteIndices}
            showExtendedStops={settings.showExtendedStops}
            showEachStopOnRoute={settings.showEachStopOnRoute}
            showAllStopsForSelectedTransports={settings.showAllStopsForSelectedTransports}
            transportModes={transportModes}
            modesShowAllLines={modesShowAllLines}
            onSetFrom={onSetFrom}
            onSetTo={handleSetTo}
            onRouteClick={toggleRouteSelection}
          />
        }
      />
    </>
  );
}

export default App;
