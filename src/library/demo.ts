// Bibliothèque inventée pour le navigateur seul (`npm run dev`) : des
// dessins au trait blanc sur transparent, tracés au hasard (toujours les
// mêmes), pour juger du parcours sans l'app.

import type { LibraryEntry } from "../engine";

const FOLDERS = ["Oiseaux", "Oiseaux/Mer", "Végétaux", "Végétaux/Arbres", "Ornements"];
const NAMES = ["merle", "héron", "goéland", "chêne", "lierre", "rosace", "volute", "fougère", "mouette", "frise"];

function random(seed: number): () => number {
  let state = seed * 2654435761 || 1;
  return () => {
    state ^= state << 13;
    state ^= state >>> 17;
    state ^= state << 5;
    return ((state >>> 0) % 100000) / 100000;
  };
}

function sketch(seed: number, side: number): string {
  const canvas = document.createElement("canvas");
  canvas.width = side;
  canvas.height = side;
  const g = canvas.getContext("2d");
  if (!g) return "";
  const next = random(seed + 1);
  g.strokeStyle = "white";
  g.lineCap = "round";
  for (let s = 0; s < 5; s++) {
    g.lineWidth = 1 + next() * 2.5;
    g.beginPath();
    let x = side * (0.2 + 0.6 * next());
    let y = side * (0.2 + 0.6 * next());
    let heading = next() * Math.PI * 2;
    g.moveTo(x, y);
    for (let i = 0; i < 60; i++) {
      heading += (next() - 0.5) * 0.9;
      x = Math.min(side - 8, Math.max(8, x + Math.cos(heading) * side * 0.03));
      y = Math.min(side - 8, Math.max(8, y + Math.sin(heading) * side * 0.03));
      g.lineTo(x, y);
    }
    g.stroke();
  }
  return canvas.toDataURL("image/png");
}

/** `count` dessins inventés, avec leur vignette. */
export function demoLibrary(count: number): (LibraryEntry & { url: string | null })[] {
  return Array.from({ length: count }, (_, i) => {
    const folder = FOLDERS[i % FOLDERS.length];
    const name = `${NAMES[i % NAMES.length]}-${String(i + 1).padStart(3, "0")}`;
    return {
      id: `demo-${i}`,
      name,
      folder,
      source: `bibliotheque/${folder}/${name}.png`,
      width: 4096,
      height: 4096,
      thumbnail: null,
      problem: null,
      url: sketch(i, 256),
    };
  });
}
