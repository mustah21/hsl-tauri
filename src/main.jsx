import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App.jsx";
import "leaflet/dist/leaflet.css";
import "./styles/variables.css";
import "./styles/App.css";
import "./styles/transport-controls.css";
import { RuntimeConfig } from "./config";

// Set data attributes based on build-time configuration (not runtime detection)
document.body.dataset.runtime = RuntimeConfig.isTauri ? "tauri" : "web";
document.body.dataset.mapTilesSource = RuntimeConfig.tileSource;
document.body.dataset.mapTilesBaseUrl = RuntimeConfig.mapBaseUrl;

ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
