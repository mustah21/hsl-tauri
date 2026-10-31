import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  server: {
    port: 3000,
    strictPort: true, // Fail if port 3000 is in use (don't try other ports)
    proxy: {
      "/api": {
        target: "http://localhost:3001",
        configure: (proxy) => {
          proxy.on("error", (_err, _req, res) => {
            res.writeHead(503, { "Content-Type": "application/json" });
            res.end(JSON.stringify({ error: "api server not ready" }));
          });
        },
      },
    },
  },
});
