import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Réglages attendus par Tauri : port fixe, et pas de surveillance du dossier
// Rust (sinon Vite recharge l'interface à chaque compilation du moteur).
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**", "**/crates/**", "**/target/**"] },
  },
  build: { target: "safari15" },
});
