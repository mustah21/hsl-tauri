/**
 * ClickPopupMenu: Renders the popup menu that appears when clicking on the map.
 * Positioned absolutely at the click location with "Start" and "End" buttons.
 */

import React from "react";

const ClickPopupMenu = ({ clickMenu, onSetFrom, onSetTo, onClose }) => {
  if (!clickMenu) return null;

  return (
    <div
      className="map-click-menu-anchor"
      style={{ left: `${clickMenu.x}px`, top: `${clickMenu.y}px` }}
      onMouseDown={(e) => e.stopPropagation()}
      onClick={(e) => e.stopPropagation()}
    >
      <div className="map-click-popup">
        <button
          type="button"
          className="leaflet-popup-close-button map-click-popup-close"
          aria-label="Close"
          onClick={onClose}
        >
          <span aria-hidden="true">x</span>
        </button>
        <button
          type="button"
          className="map-popup-btn"
          onClick={() => {
            onSetFrom(clickMenu.lat, clickMenu.lng);
            onClose();
          }}
        >
          Start
        </button>
        <button
          type="button"
          className="map-popup-btn"
          onClick={() => {
            onSetTo(clickMenu.lat, clickMenu.lng);
            onClose();
          }}
        >
          End
        </button>
      </div>
    </div>
  );
};

export default ClickPopupMenu;
