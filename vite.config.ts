import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Nada de CDN em runtime: tudo que a tela usa entra no bundle.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    target: "chrome105",
    assetsInlineLimit: 0,
  },
});
