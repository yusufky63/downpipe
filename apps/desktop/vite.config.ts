import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// Tauri loads the dev server from a fixed port (tauri.conf.json `devUrl`);
// `clearScreen: false` keeps the Rust build output visible in the terminal.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "localhost",
    watch: { ignored: ["**/src-tauri/**"] },
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    target: "es2022",
    sourcemap: false,
    rolldownOptions: {
      // The main window, the confirmation window that asks about
      // downloads from the browser (src-tauri/src/confirm.rs), and the
      // remote control's page for phones (crates/remote serves it).
      input: {
        main: "index.html",
        confirm: "confirm.html",
        remote: "remote.html",
      },
    },
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
