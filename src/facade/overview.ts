// Vue façade (tâche 3.3) : ce que l'écran Façade pose par-dessus le plan.
// Logique pure (sans écran), testée dans overview.test.ts.

import type { FacadeOverview, Overlap, ProjectorOverview } from "../engine";
import type { LayerSpec } from "../viewer";

/** Couleurs des projecteurs, dans l'ordre du projet (on reboucle après 12). */
export const PROJECTOR_COLORS = [
  0x4fc3f7, 0xffb74d, 0x81c784, 0xf06292, 0xba68c8, 0xfff176, 0x4db6ac, 0xff8a65, 0x9575cd, 0xaed581, 0x64b5f6,
  0xe57373,
];

/** Couleur des zones de recouvrement. */
export const OVERLAP_COLOR = 0xff3b30;

export function projectorColor(index: number): number {
  return PROJECTOR_COLORS[((index % PROJECTOR_COLORS.length) + PROJECTOR_COLORS.length) % PROJECTOR_COLORS.length];
}

export function cssColor(color: number): string {
  return `#${color.toString(16).padStart(6, "0")}`;
}

/** Ce que l'on choisit de montrer. */
export interface OverviewDisplay {
  photos: boolean;
  footprints: boolean;
  overlaps: boolean;
  /** Opacité des photos, de 0 à 1. */
  photoOpacity: number;
  /** Projecteurs masqués, par identifiant. */
  hidden: ReadonlySet<string>;
}

export const DEFAULT_DISPLAY: OverviewDisplay = {
  photos: true,
  footprints: true,
  overlaps: true,
  photoOpacity: 0.8,
  hidden: new Set(),
};

/** Un projecteur se montre s'il est recalé et pas masqué. */
export function isShown(projector: ProjectorOverview, display: OverviewDisplay): boolean {
  return projector.footprint.length > 2 && !display.hidden.has(projector.projectorId);
}

/** Les calques réellement visibles. */
export function visibleLayers(layers: LayerSpec[]): LayerSpec[] {
  return layers.filter((l) => l.visible !== false);
}

/**
 * Calques de la vue façade, à poser au-dessus du plan : les photos (dans
 * l'ordre du projet), puis les recouvrements, puis les emprises. `photoUrl`
 * donne l'adresse d'un aperçu du cache.
 */
export function overviewLayers(
  overview: FacadeOverview | null,
  display: OverviewDisplay,
  photoUrl: (request: string) => string,
): LayerSpec[] {
  if (!overview) return [];
  const layers: LayerSpec[] = [];
  const shown = overview.projectors.filter((p) => isShown(p, display));
  // Les photos restent dans la scène quand on les masque : les réafficher
  // ne demande pas de les recharger.
  for (const p of overview.projectors) {
    if (!p.mesh || !p.photoPreview) continue;
    layers.push({
      id: `photo:${p.projectorId}`,
      kind: "mesh-image",
      url: photoUrl(p.photoPreview),
      divisions: p.mesh.divisions,
      points: p.mesh.facade,
      opacity: display.photoOpacity,
      visible: display.photos && isShown(p, display),
      // Le noir de la photo redressée ne cache pas le plan ni les voisines.
      blend: "screen",
    });
  }
  if (display.overlaps) {
    const visible = new Set(shown.map((p) => p.projectorId));
    const shapes = overview.overlaps
      .filter((o) => visible.has(o.projectors[0]) && visible.has(o.projectors[1]))
      .flatMap((o) =>
        o.outlines.map((points) => ({
          points,
          color: OVERLAP_COLOR,
          widthPx: 1,
          fillAlpha: 0.25,
        })),
      );
    layers.push({ id: "recouvrements", kind: "polygons", shapes });
  }
  if (display.footprints) {
    layers.push({
      id: "emprises",
      kind: "polygons",
      shapes: overview.projectors.flatMap((p, index) =>
        isShown(p, display) ? [{ points: p.footprint, color: projectorColor(index), widthPx: 2 }] : [],
      ),
    });
  }
  return layers;
}

/** « 7,84 m² ». */
export function formatArea(m2: number | null): string {
  if (m2 === null || !Number.isFinite(m2)) return "—";
  return `${m2.toLocaleString("fr-FR", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} m²`;
}

/** « P1 et P2 », d'après les noms des projecteurs. */
export function overlapLabel(overlap: Overlap, projectors: ProjectorOverview[]): string {
  const name = (id: string) => projectors.find((p) => p.projectorId === id)?.label ?? id;
  return `${name(overlap.projectors[0])} et ${name(overlap.projectors[1])}`;
}

/** Ce qu'on dit d'un projecteur dans la liste. */
export function projectorStatus(p: ProjectorOverview): string {
  if (p.problem) return p.problem;
  return `${p.pairs} points · ${formatArea(p.areaM2)}`;
}

/** Résumé en tête du panneau. */
export function overviewSummary(overview: FacadeOverview): string {
  const total = overview.projectors.length;
  const placed = overview.projectors.filter((p) => p.footprint.length > 2).length;
  if (total === 0) return "Aucun projecteur dans ce projet.";
  const head =
    placed === total
      ? `${total} projecteur${total > 1 ? "s" : ""} recalé${total > 1 ? "s" : ""}`
      : `${placed} recalé${placed > 1 ? "s" : ""} sur ${total}`;
  const n = overview.overlaps.length;
  return `${head} · ${n === 0 ? "aucun recouvrement" : `${n} recouvrement${n > 1 ? "s" : ""}`}`;
}
