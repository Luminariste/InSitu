import { createReadStream, statSync } from "node:fs";
import type { IncomingMessage, ServerResponse } from "node:http";
import { extname, resolve, sep } from "node:path";
import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";

const CONTENT_TYPES: Record<string, string> = {
  ".json": "application/json",
  ".png": "image/png",
  ".jpg": "image/jpeg",
};

/**
 * Sert sous `/tuiles/` un dossier de pyramides fabriquées par `insitu tiles`
 * (par défaut `tuiles-demo/`, jamais versé au dépôt ; `INSITU_TILES_DIR` pour
 * un autre dossier). Sert à contrôler la visionneuse dans un navigateur, et
 * de solution de secours pour l'écran de démonstration. Dans l'application,
 * les tuiles viennent du cache de l'app (voir `src-tauri/src/lib.rs`).
 */
function demoTiles(): Plugin {
  const root = resolve(process.env.INSITU_TILES_DIR ?? "tuiles-demo");
  const serve = (request: IncomingMessage, response: ServerResponse, next: () => void) => {
    const path = decodeURIComponent((request.url ?? "").split("?")[0]);
    if (!path.startsWith("/tuiles/")) return next();
    const file = resolve(root, path.slice("/tuiles/".length));
    const type = CONTENT_TYPES[extname(file)];
    const size = type && file.startsWith(root + sep) ? fileSize(file) : null;
    if (size === null) {
      response.statusCode = 404;
      response.end();
      return;
    }
    response.setHeader("Content-Type", type);
    response.setHeader("Content-Length", size);
    response.setHeader("Cache-Control", "no-cache");
    createReadStream(file).pipe(response);
  };
  return {
    name: "insitu-tuiles-demo",
    configureServer: (server) => void server.middlewares.use(serve),
    configurePreviewServer: (server) => void server.middlewares.use(serve),
  };
}

function fileSize(file: string): number | null {
  try {
    const stat = statSync(file);
    return stat.isFile() ? stat.size : null;
  } catch {
    return null;
  }
}

// Réglages attendus par Tauri : port fixe, et pas de surveillance du dossier
// Rust (sinon Vite recharge l'interface à chaque compilation du moteur).
export default defineConfig({
  plugins: [react(), demoTiles()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**", "**/crates/**", "**/target/**", "**/tuiles-demo/**", "**/jeu-essai/**"] },
  },
  build: { target: "safari15" },
});
