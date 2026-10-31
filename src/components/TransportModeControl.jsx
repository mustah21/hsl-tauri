import React from "react";
import "../styles/transport-controls.css";
import { TRANSPORT_MODES } from "../constants/transportModes";

const TransportModeControl = ({
  selectedModes,
  onModeToggle,
  modesShowAllLines,
  onToggleShowAllLines,
  isTauri,
}) => {
  return (
    <div className="transport-mode-list">
      {TRANSPORT_MODES.map((mode) => (
        <div key={mode.id} className="transport-mode-row">
          <label className="transport-mode-option">
            <input
              type="checkbox"
              checked={selectedModes.includes(mode.id)}
              onChange={() => onModeToggle(mode.id)}
              aria-label={`Toggle ${mode.label}`}
            />
            <span className="mode-icon" role="img" aria-hidden="true">
              {mode.icon}
            </span>
            <span className="mode-label">{mode.label}</span>
          </label>
          {mode.id !== "WALK" && (
            <label
              className="mode-show-all-label"
              title={
                isTauri
                  ? `Show all ${mode.label} lines on map`
                  : `Available only in desktop app (Tauri) for ${mode.label}`
              }
            >
              <input
                type="checkbox"
                className="mode-show-all-checkbox"
                checked={isTauri && modesShowAllLines.includes(mode.id)}
                onChange={() => {
                  if (isTauri) {
                    onToggleShowAllLines(mode.id);
                  }
                }}
                disabled={!isTauri}
                aria-label={
                  isTauri
                    ? `Show all ${mode.label} lines on map`
                    : `Show all ${mode.label} lines is available only in desktop app`
                }
              />
              <span className="mode-show-all-text">All lines</span>
            </label>
          )}
        </div>
      ))}
    </div>
  );
};

export default TransportModeControl;
