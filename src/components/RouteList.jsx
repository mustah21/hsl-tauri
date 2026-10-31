import React, { useEffect, useRef } from "react";

const formatTime = (ms) => {
  const date = new Date(ms);
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
};

const formatDuration = (seconds) => {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  return h > 0 ? `${h}h ${m}m` : `${m}m`;
};

const RouteList = ({ routes, selectedIds, onToggleRoute }) => {
  const cardRefs = useRef([]);
  const prevSelectedIds = useRef([]);

  useEffect(() => {
    const newlySelected = selectedIds.filter((id) => !prevSelectedIds.current.includes(id));
    if (newlySelected.length > 0) {
      const idx = newlySelected[0];
      cardRefs.current[idx]?.scrollIntoView({ behavior: "smooth", block: "nearest" });
    }
    prevSelectedIds.current = selectedIds;
  }, [selectedIds]);

  if (!routes || routes.length === 0)
    return <div className="route-empty">No routes found. Try searching.</div>;

  return (
    <div className="route-list" role="list" aria-label="Route suggestions">
      {routes.map((route, index) => {
        const isSelected = selectedIds.includes(index);
        const checkboxId = `route-toggle-${index}`;

        return (
          <div
            key={index}
            ref={(el) => {
              cardRefs.current[index] = el;
            }}
            className={`route-card ${isSelected ? "selected" : ""}`}
            role="listitem"
          >
            <label className="route-card-label" htmlFor={checkboxId}>
              <div className="route-card-content">
                <div className="route-header">
                  <span>
                    {formatTime(route.startTime)} - {formatTime(route.endTime)}
                  </span>
                  <span>{formatDuration(route.duration)}</span>
                </div>
                <div className="route-details">
                  {route.legs.map((leg, i) => (
                    <span key={i}>
                      {leg.mode === "WALK" ? "Walk" : leg.route?.shortName || "Transit"}
                      {i < route.legs.length - 1 ? " → " : ""}
                    </span>
                  ))}
                  <div className="route-walking-distance">
                    {Math.round(route.walkDistance)}m walking
                  </div>
                </div>
              </div>
              <input
                id={checkboxId}
                type="checkbox"
                className="route-card-checkbox"
                checked={isSelected}
                onChange={() => onToggleRoute(index)}
                aria-label={`Show route ${index + 1} on map`}
              />
            </label>
          </div>
        );
      })}
    </div>
  );
};

export default RouteList;
