// Vue diapositive inventée pour le navigateur seul (`npm run dev`) : une
// fausse photo redressée (arcade en gris) et, pour trois projecteurs, des
// dessins qui en suivent les arcs.

import type { SlideDrawing, SlideView } from "../engine";

export function demoSlideViews(): SlideView[] {
  return ["A", "B", "C"].map((label, i) => ({
    projectorId: `demo-${label}`,
    label,
    rectifiedPhoto: "demo",
    preview: i < 2 ? "demo" : null,
    problem: i < 2 ? null : "recalage incomplet : 2 paire(s) de points sur 4 au moins",
    drawings: i < 2 ? [demoDrawing(`demo-pose-${label}`, 95 + 6 * i, 45)] : [],
  }));
}

/** Un dessin posé inventé, situé dans la diapositive (mm), pour essayer la retouche (tâche 4.6). */
function demoDrawing(placement: string, x: number, y: number): SlideDrawing {
  const n = 16;
  const points = [];
  for (let j = 0; j <= n; j++)
    for (let i = 0; i <= n; i++) points.push({ x: x + (90 * i) / n + 4 * (j / n), y: y + (110 * j) / n });
  return { placement, drawing: "demo-0", divisions: n, points };
}

function canvas(draw: (g: CanvasRenderingContext2D, n: number) => void): string {
  const c = document.createElement("canvas");
  c.width = c.height = 1024;
  const g = c.getContext("2d");
  if (!g) return "";
  draw(g, 1024);
  return c.toDataURL("image/png");
}

function arches(g: CanvasRenderingContext2D, n: number, shift: number) {
  for (let k = 0; k < 3; k++) {
    const cx = n * (0.2 + 0.3 * k) + shift;
    g.beginPath();
    g.arc(cx, n * 0.45, n * 0.12, Math.PI, 0);
    g.lineTo(cx + n * 0.12, n * 0.9);
    g.moveTo(cx - n * 0.12, n * 0.9);
    g.lineTo(cx - n * 0.12, n * 0.45);
    g.stroke();
  }
}

export function demoImages(views: SlideView[]): { photo: string; drawings: Record<string, string> } {
  const photo = canvas((g, n) => {
    g.fillStyle = "#8a8378";
    g.fillRect(0, 0, n, n);
    g.strokeStyle = "#3d3a34";
    g.lineWidth = 18;
    arches(g, n, 0);
  });
  const drawings: Record<string, string> = {};
  views.forEach((v, i) => {
    if (!v.preview) return;
    drawings[v.projectorId] = canvas((g, n) => {
      g.strokeStyle = "white";
      g.lineWidth = 5;
      // Le second projecteur est volontairement décalé de 6 mm.
      arches(g, n, i * 22);
      g.font = "bold 80px sans-serif";
      g.fillStyle = "white";
      g.fillText(`Dessin ${v.label}`, n * 0.32, n * 0.16);
    });
  });
  return { photo, drawings };
}
