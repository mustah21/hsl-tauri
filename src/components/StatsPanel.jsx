import React, { useState, useEffect } from "react";
import { invokeTauri } from "../services/runtime";
import { IS_TAURI } from "../config";

const formatBytes = (bytes) => {
  if (!bytes) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + " " + sizes[i];
};

const StatsPanel = ({ isActive = true }) => {
  const [httpStats, setHttpStats] = useState(null);
  const [tileStats, setTileStats] = useState(null);
  const [appMemory, setAppMemory] = useState(0);
  const [tileLoading, setTileLoading] = useState(false);
  const [error, setError] = useState(null);

  const refreshTileStats = async (isCancelled) => {
    if (!IS_TAURI) return;
    try {
      const stats = await invokeTauri("get_tile_cache_stats");
      if (!isCancelled) {
        setTileStats(stats);
      }
    } catch (err) {
      if (!isCancelled) {
        setError(`Failed to load tile stats: ${err}`);
      }
    }
  };

  // Poll memory and HTTP cache every 2 s
  useEffect(() => {
    if (!isActive) {
      setTileLoading(false);
      return;
    }

    let isCancelled = false;

    if (!isTauri) {
      if (!isCancelled && performance.memory) {
        setAppMemory(performance.memory.usedJSHeapSize);
      }
      return () => {
        isCancelled = true;
      };
    }

    const update = async () => {
      if (isCancelled) return;
      try {
        const [stats, tile] = await Promise.all([
          invokeTauri("get_cache_stats"),
          invokeTauri("get_tile_cache_stats"),
        ]);
        if (!isCancelled) {
          setHttpStats(stats);
          setTileStats(tile);
        }
      } catch (err) {
        if (!isCancelled) {
          setError(`Failed to refresh stats: ${err}`);
        }
      }
      if (!isCancelled && performance.memory) {
        setAppMemory(performance.memory.usedJSHeapSize);
      }
    };

    update();
    const interval = setInterval(update, 2000);
    return () => {
      isCancelled = true;
      clearInterval(interval);
    };
  }, [isTauri, isActive]);

  useEffect(() => {
    if (!isActive) {
      setTileLoading(false);
      setError(null);
      return;
    }
  }, [isTauri, isActive]);

  const clearTile = async () => {
    if (!isTauri) return;
    setTileLoading(true);
    setError(null);
    try {
      await invokeTauri("clear_tile_cache");
      await refreshTileStats(false);
    } catch (err) {
      setError(`Failed to clear tile cache: ${err}`);
      setTileLoading(false);
    } finally {
      setTileLoading(false);
    }
  };

  const clearHttp = async () => {
    if (!isTauri) return;
    setError(null);
    try {
      await invokeTauri("clear_cache");
    } catch (err) {
      setError(`Failed to clear HTTP cache: ${err}`);
    }
  };

  const clearAll = async () => {
    if (!isTauri) return;
    setTileLoading(true);
    setError(null);
    try {
      await invokeTauri("clear_tile_cache");
      await invokeTauri("clear_cache");
      await refreshTileStats(false);
    } catch (err) {
      setError(`Failed to clear caches: ${err}`);
      setTileLoading(false);
    } finally {
      setTileLoading(false);
    }
  };

  const hitRate = tileStats
    ? Math.round((tileStats.hits / Math.max(1, tileStats.hits + tileStats.misses)) * 100)
    : null;

  return (
    <div className="stats-panel">
      {error && (
        <p className="stats-error" role="alert">
          {error}
        </p>
      )}

      <section className="stats-section">
        <h4 className="stats-section-title">Runtime</h4>
        <dl className="stats-list">
          <div className="stats-row">
            <dt>Mode</dt>
            <dd>{isTauri ? "Tauri desktop" : "Web browser"}</dd>
          </div>
          <div className="stats-row">
            <dt>Map tiles</dt>
            <dd>{document.body.dataset.mapTilesSource || "Unknown"}</dd>
          </div>
        </dl>
      </section>

      <section className="stats-section">
        <h4 className="stats-section-title">Memory</h4>
        <dl className="stats-list">
          <div className="stats-row">
            <dt>App</dt>
            <dd>{formatBytes(appMemory)}</dd>
          </div>
          {httpStats && (
            <>
              <div className="stats-row">
                <dt>HTTP cache</dt>
                <dd>{formatBytes(httpStats.estimated_bytes)}</dd>
              </div>
              <div className="stats-row">
                <dt>HTTP entries</dt>
                <dd>{httpStats.entries}</dd>
              </div>
              <div className="stats-row stats-total">
                <dt>Total</dt>
                <dd>{formatBytes(appMemory + httpStats.estimated_bytes)}</dd>
              </div>
            </>
          )}
          {!isTauri && (
            <div className="stats-row">
              <dt>Backend</dt>
              <dd>Web mode</dd>
            </div>
          )}
        </dl>
        <button className="stats-btn" onClick={clearHttp} disabled={!isTauri}>
          Clear HTTP Cache
        </button>
      </section>

      <section className="stats-section">
        <h4 className="stats-section-title">Tile Cache</h4>
        {tileStats ? (
          <>
            <dl className="stats-list">
              <div className="stats-row">
                <dt>Entries</dt>
                <dd>{tileStats.total_entries}</dd>
              </div>
              <div className="stats-row">
                <dt>Size</dt>
                <dd>{formatBytes(tileStats.total_bytes)}</dd>
              </div>
              <div className="stats-row">
                <dt>Hit rate</dt>
                <dd>{hitRate}%</dd>
              </div>
              <div className="stats-row">
                <dt>Evictions</dt>
                <dd>{tileStats.evictions}</dd>
              </div>
            </dl>
            <button className="stats-btn" onClick={clearTile} disabled={tileLoading}>
              Clear Tile Cache
            </button>
          </>
        ) : (
          <p className="search-loading">
            {isTauri ? "Loading tile stats…" : "Tile stats unavailable in web mode"}
          </p>
        )}
      </section>

      <button
        className="stats-btn stats-btn-danger"
        onClick={clearAll}
        disabled={tileLoading || !isTauri}
      >
        Clear All Caches
      </button>
    </div>
  );
};

export default StatsPanel;
