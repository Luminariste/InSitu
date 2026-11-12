// Vue façade inventée pour le navigateur seul (`npm run dev`) : dix
// projecteurs en deux rangées sur un mur de 24 m, chacun légèrement de
// biais, avec une fausse photo redressée. Les recouvrements sont des
// rectangles approchés : le vrai calcul est fait par le moteur.

import type { FacadeOverview, Overlap, ProjectorOverview, SourcePoint } from "../engine";

const DIVISIONS = 16;

/** Une fausse photo redressée : la mire, traits clairs sur fond sombre. */
function fakePhoto(index: number): string {
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = 256;
  const g = canvas.getContext("2d");
  if (!g) return "";
  g.fillStyle = "#050505";
  g.fillRect(0, 0, 256, 256);
  g.strokeStyle = "#e8e2d0";
  g.lineWidth = 2;
  for (let k = 0; k <= 8; k++) {
    g.beginPath();
    g.moveTo(16 + k * 28, 16);
    g.lineTo(16 + k * 28, 240);
    g.moveTo(16, 16 + k * 28);
    g.lineTo(240, 16 + k * 28);
    g.stroke();
  }
  g.font = "bold 64px sans-serif";
  g.fillStyle = "#e8e2d0";
  g.textAlign = "center";
  g.textBaseline = "middle";
  g.fillText(`P${index + 1}`, 128, 128);
  return canvas.toDataURL("image/png");
}

/** Place la diapositive : centre, taille sur la façade (mm), angle, trapèze. */
function placer(cx: number, cy: number, side: number, angle: number, keystone: number) {
  const cos = Math.cos(angle);
  const sin = Math.sin(angle);
  return (u: number, v: number): SourcePoint => {
    // u, v de 0 à 1 sur la diapositive ; le bas s'élargit un peu (trapèze).
    const widen = 1 + keystone * (v - 0.5);
    const x = (u - 0.5) * side * widen;
    const y = (v - 0.5) * side;
    return { x: cx + x * cos - y * sin, y: cy + x * sin + y * cos };
  };
}

export function demoOverview(): FacadeOverview {
  const projectors: ProjectorOverview[] = [];
  const boxes: {
    id: string;
    x0: number;
    y0: number;
    x1: number;
    y1: number;
  }[] = [];
  for (let i = 0; i < 10; i++) {
    const row = i < 5 ? 0 : 1;
    const col = i % 5;
    const side = 5200 + 300 * ((i * 7) % 3);
    const at = placer(2800 + col * 4600 + row * 900, 3200 + row * 4300, side, ((i % 3) - 1) * 0.03, 0.12);
    const points: SourcePoint[] = [];
    for (let j = 0; j <= DIVISIONS; j++)
      for (let k = 0; k <= DIVISIONS; k++) points.push(at(k / DIVISIONS, j / DIVISIONS));
    const n = DIVISIONS + 1;
    const corners = [points[0], points[n - 1], points[n * n - 1], points[n * (n - 1)]];
    const footprint = [...corners, corners[0]];
    let area = 0;
    for (let k = 0; k < 4; k++) area += corners[k].x * corners[(k + 1) % 4].y - corners[(k + 1) % 4].x * corners[k].y;
    const id = `demo-${i + 1}`;
    projectors.push({
      projectorId: id,
      label: `P${i + 1}`,
      pairs: 6 + (i % 4),
      footprint,
      areaM2: Math.abs(area) / 2 / 1e6,
      mesh: { divisions: DIVISIONS, facade: points },
      photoPreview: fakePhoto(i),
      problem: null,
    });
    const xs = corners.map((c) => c.x);
    const ys = corners.map((c) => c.y);
    boxes.push({
      id,
      x0: Math.min(...xs),
      y0: Math.min(...ys),
      x1: Math.max(...xs),
      y1: Math.max(...ys),
    });
  }
  projectors.push({
    projectorId: "demo-11",
    label: "P11",
    pairs: 2,
    footprint: [],
    areaM2: null,
    mesh: null,
    photoPreview: null,
    problem: "Recalage incomplet : 2 paires de points sur 4.",
  });
  const overlaps: Overlap[] = [];
  for (let a = 0; a < boxes.length; a++) {
    for (let b = a + 1; b < boxes.length; b++) {
      const [p, q] = [boxes[a], boxes[b]];
      const x0 = Math.max(p.x0, q.x0);
      const x1 = Math.min(p.x1, q.x1);
      const y0 = Math.max(p.y0, q.y0);
      const y1 = Math.min(p.y1, q.y1);
      if (x1 <= x0 || y1 <= y0) continue;
      overlaps.push({
        projectors: [p.id, q.id],
        areaM2: ((x1 - x0) * (y1 - y0)) / 1e6,
        outlines: [
          [
            { x: x0, y: y0 },
            { x: x1, y: y0 },
            { x: x1, y: y1 },
            { x: x0, y: y1 },
            { x: x0, y: y0 },
          ],
        ],
      });
    }
  }
  return { projectors, overlaps };
}
