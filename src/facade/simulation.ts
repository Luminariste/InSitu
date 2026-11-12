// Simulation et alertes (tâche 6.1) : logique pure de l'écran Façade, testée
// dans simulation.test.ts.

import type { AlertKind, FacadeOverview, Simulation, SimulationAlert } from "../engine";
import type { LayerSpec } from "../viewer";
import type { Rect } from "../viewer/geometry";
import { OVERLAP_COLOR, isShown, projectorColor, type OverviewDisplay } from "./overview";

/** Titre court de chaque sorte d'alerte. */
export const ALERT_TITLES: Record<AlertKind, string> = {
  cut: "Dessin coupé",
  outside: "Hors emprise",
  doubled: "Projeté deux fois",
};

/** Résumé en tête de la rubrique. */
export function simulationSummary(simulation: Simulation): string {
  const n = simulation.alerts.length;
  if (n === 0) return "Aucune alerte : chaque dessin tient dans sa diapositive, rien n'est projeté deux fois.";
  const count = (kind: AlertKind) => simulation.alerts.filter((a) => a.kind === kind).length;
  const parts = (["cut", "outside", "doubled"] as const)
    .filter((kind) => count(kind) > 0)
    .map((kind) => `${count(kind)} ${ALERT_TITLES[kind].toLowerCase()}`);
  return `${n} alerte${n > 1 ? "s" : ""} : ${parts.join(", ")}.`;
}

/**
 * Où mène une alerte : le dessin à choisir (s'il y en a un) et le rectangle
 * de la façade à montrer, élargi pour voir ce qui l'entoure. Un rectangle
 * trop fin (un trait) est porté à `minSize` mm au moins.
 */
export function alertTarget(alert: SimulationAlert, minSize = 200): { selection: string[]; rect: Rect } {
  const b = alert.bounds;
  const width = Math.max(b.width * 1.5, minSize);
  const height = Math.max(b.height * 1.5, minSize);
  return {
    selection: alert.placement ? [alert.placement] : [],
    rect: { x: b.x + b.width / 2 - width / 2, y: b.y + b.height / 2 - height / 2, width, height },
  };
}

/**
 * Calques de la simulation, au-dessus de la vue façade : la diapositive
 * calculée de chaque projecteur montré, tendue sur son maillage et teintée
 * de sa couleur (deux diapositives qui allument le même point mêlent leurs
 * couleurs), puis le cadre rouge de chaque alerte.
 */
export function simulationLayers(
  overview: FacadeOverview | null,
  simulation: Simulation | null,
  display: OverviewDisplay,
  showSlides: boolean,
  url: (request: string) => string,
): LayerSpec[] {
  if (!overview || !simulation) return [];
  const layers: LayerSpec[] = [];
  overview.projectors.forEach((p, index) => {
    const slide = simulation.slides.find((s) => s.projectorId === p.projectorId);
    if (!p.mesh || !slide?.preview) return;
    layers.push({
      id: `simulation:${p.projectorId}`,
      kind: "mesh-image",
      url: url(slide.preview),
      divisions: p.mesh.divisions,
      points: p.mesh.facade,
      tint: projectorColor(index),
      visible: showSlides && isShown(p, display),
      blend: "screen",
    });
  });
  layers.push({
    id: "alertes",
    kind: "polygons",
    shapes: simulation.alerts.map((a) => {
      const b = a.bounds;
      return {
        points: [
          { x: b.x, y: b.y },
          { x: b.x + b.width, y: b.y },
          { x: b.x + b.width, y: b.y + b.height },
          { x: b.x, y: b.y + b.height },
        ],
        color: OVERLAP_COLOR,
        widthPx: 2,
      };
    }),
  });
  return layers;
}
