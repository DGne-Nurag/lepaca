import { defineConfig } from "vite";

// Feste Ports und kein Bildschirm-Löschen, damit `tauri dev` die Ausgabe zeigt.
export default defineConfig({
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: "es2022", outDir: "dist", sourcemap: false },
});
