// Bibliothèque inventée pour le navigateur seul (`npm run dev`) : des
// dessins au trait blanc sur transparent, tracés au hasard (toujours les
// mêmes), pour juger du parcours sans l'app.

import { DEFAULT_VECTOR_SETTINGS, type LibraryEntry, type VectorSummary } from "../engine";

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

/** Les traits d'un dessin inventé : épaisseur et suite de points. */
function strokes(seed: number, side: number): { width: number; points: [number, number][] }[] {
  const next = random(seed + 1);
  const out: { width: number; points: [number, number][] }[] = [];
  for (let s = 0; s < 5; s++) {
    const width = 1 + next() * 2.5;
    let x = side * (0.2 + 0.6 * next());
    let y = side * (0.2 + 0.6 * next());
    let heading = next() * Math.PI * 2;
    const points: [number, number][] = [[x, y]];
    for (let i = 0; i < 60; i++) {
      heading += (next() - 0.5) * 0.9;
      x = Math.min(side - 8, Math.max(8, x + Math.cos(heading) * side * 0.03));
      y = Math.min(side - 8, Math.max(8, y + Math.sin(heading) * side * 0.03));
      points.push([x, y]);
    }
    out.push({ width, points });
  }
  return out;
}

function sketch(seed: number, side: number): string {
  const canvas = document.createElement("canvas");
  canvas.width = side;
  canvas.height = side;
  const g = canvas.getContext("2d");
  if (!g) return "";
  g.strokeStyle = "white";
  g.lineCap = "round";
  for (const stroke of strokes(seed, side)) {
    g.lineWidth = stroke.width;
    g.beginPath();
    stroke.points.forEach(([x, y], i) => (i === 0 ? g.moveTo(x, y) : g.lineTo(x, y)));
    g.stroke();
  }
  return canvas.toDataURL("image/png");
}

/**
 * Silhouette d'un dessin inventé : l'enveloppe convexe de ses traits, un peu
 * élargie (dans l'app, le moteur suit le vrai contour extérieur).
 */
export function demoSilhouette(id: string): [number, number][][] {
  const index = Number(id.replace(/^demo-/, ""));
  if (!Number.isInteger(index)) return [];
  const side = 256;
  const points = strokes(index, side)
    .flatMap((s) => s.points)
    .sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  const cross = (o: [number, number], a: [number, number], b: [number, number]) =>
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
  const half = (list: [number, number][]) => {
    const out: [number, number][] = [];
    for (const p of list) {
      while (out.length >= 2 && cross(out[out.length - 2], out[out.length - 1], p) <= 0) out.pop();
      out.push(p);
    }
    out.pop();
    return out;
  };
  const hull = [...half(points), ...half([...points].reverse())];
  const cx = hull.reduce((s, p) => s + p[0], 0) / hull.length;
  const cy = hull.reduce((s, p) => s + p[1], 0) / hull.length;
  return [
    hull.map(([x, y]) => {
      const d = Math.hypot(x - cx, y - cy) || 1;
      const grow = 6 / d;
      return [
        Math.min(1, Math.max(0, (x + (x - cx) * grow) / side)),
        Math.min(1, Math.max(0, (y + (y - cy) * grow) / side)),
      ];
    }),
  ];
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
      vector: null,
      url: sketch(i, 256),
    };
  });
}

/**
 * Vecteurs inventés d'un dessin de démonstration : les mêmes traits en SVG
 * (le contour rouge suit le milieu du trait, pas son bord comme dans l'app).
 */
export function demoVectors(index: number, imageUrl: string): VectorSummary {
  const side = 256;
  const paths = strokes(index, side);
  const d = (p: [number, number][]) => p.map(([x, y], i) => `${i === 0 ? "M" : "L"}${x.toFixed(2)} ${y.toFixed(2)}`).join("");
  const svg = (body: string) =>
    `data:image/svg+xml;utf8,${encodeURIComponent(
      `<svg xmlns="http://www.w3.org/2000/svg" width="${side}" height="${side}" viewBox="0 0 ${side} ${side}">${body}</svg>`,
    )}`;
  const filled = svg(
    paths
      .map((s) => `<path d="${d(s.points)}" fill="none" stroke="white" stroke-width="${s.width}" stroke-linecap="round"/>`)
      .join(""),
  );
  const outline = svg(
    paths
      .map((s) => `<path d="${d(s.points)}" fill="none" stroke="#ff2d2d" stroke-width="1" vector-effect="non-scaling-stroke"/>`)
      .join(""),
  );
  return {
    settings: DEFAULT_VECTOR_SETTINGS,
    defaultSettings: true,
    width: side,
    height: side,
    stats: { contours: 5, removedContours: 0, rawPoints: 1200, lines: 40 + (index % 7), cubics: 110, maxFitErrorPx: 0.3 },
    comparison: {
      maxDeviationPx: 0.31,
      rawToVectorPx: 0.31,
      vectorToRawPx: 0.29,
      sourcePixels: 4000,
      vectorPixels: 4010,
      mismatchedPixels: 20,
      maxMismatchDistancePx: 0.28,
    },
    seconds: 0.05,
    filledSvg: filled,
    outlineSvg: outline,
    image: imageUrl,
  };
}
