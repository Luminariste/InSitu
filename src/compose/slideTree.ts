// Arborescence « Diapositives » de l'écran Façade (demande de Benjamin,
// 12 novembre 2026) : les dessins posés rangés par diapositive, comme les
// calques d'Illustrator. Le dessin du haut d'un groupe est devant. On glisse
// une ligne sur une autre diapositive pour l'y affecter à la main (tâche
// 4.5), ou au-dessus d'un autre dessin pour la passer devant lui. Logique
// pure, testée sans écran (`slideTree.test.ts`). Clic, ⌘-clic, ⇧-clic, œil,
// cadenas et menu au clic droit suivent le panneau Calques d'Illustrator
// (D-4.5-g).

import type { Assignment } from "../engine";
import { stack } from "./layers";
import type { Layer, Placement } from "./placement";

export interface SlideRow {
  placement: Placement;
  /** Choisie à la main plutôt que proposée par le moteur. */
  manual: boolean;
  /** Le dessin tient en entier dans la diapositive. */
  whole: boolean;
}

export interface SlideGroup {
  /** Projecteur ; `null` : les dessins qui ne tombent dans aucune emprise. */
  projector: string | null;
  label: string;
  /** Rang du projecteur dans le projet (sa couleur) ; `null` pour le groupe sans diapositive. */
  colorIndex: number | null;
  /** Du devant vers le fond. */
  rows: SlideRow[];
}

/**
 * Les groupes, dans l'ordre des projecteurs du projet, puis les dessins
 * sans diapositive s'il y en a. Un projecteur sans dessin garde son groupe
 * (on peut y glisser un dessin) s'il est recalé.
 */
export function slideTree(
  placements: Placement[],
  layers: Layer[],
  projectors: { id: string; label: string; registered: boolean }[],
  assignments: Map<string, Assignment>,
): SlideGroup[] {
  const front = [...stack(placements, layers)].reverse();
  const rowsOf = (projector: string | null) =>
    front.flatMap((placement) => {
      const a = assignments.get(placement.id);
      if (!a || a.projector !== projector) return [];
      return [{ placement, manual: a.manual, whole: a.whole }];
    });
  const groups: SlideGroup[] = projectors.flatMap((q, index) => {
    const rows = rowsOf(q.id);
    return q.registered || rows.length > 0 ? [{ projector: q.id, label: q.label, colorIndex: index, rows }] : [];
  });
  const loose = rowsOf(null);
  if (loose.length > 0)
    groups.push({ projector: null, label: "Hors de toute diapositive", colorIndex: null, rows: loose });
  return groups;
}

/** Où lâcher une ligne : dans un groupe, et devant un dessin de ce groupe ou non. */
export interface TreeDrop {
  projector: string;
  /** Le dessin lâché passe juste devant celui-ci ; absent : sa place ne change pas. */
  before?: string;
}

/**
 * Le dessin `dragged` lâché sur `drop` : il est affecté à la main à la
 * diapositive du groupe s'il n'y est pas déjà, et, lâché sur un dessin, il
 * passe juste devant lui, sur le même calque. Rend la liste inchangée si
 * rien ne change.
 */
export function dropInTree(
  placements: Placement[],
  assignments: Map<string, Assignment>,
  dragged: string,
  drop: TreeDrop,
): Placement[] {
  const from = placements.findIndex((p) => p.id === dragged);
  if (from < 0 || drop.before === dragged) return placements;
  const current = assignments.get(dragged)?.projector ?? null;
  let moved = placements[from];
  if (current !== drop.projector) moved = { ...moved, projector: drop.projector };
  if (drop.before === undefined) {
    if (moved === placements[from]) return placements;
    return placements.map((p, i) => (i === from ? moved : p));
  }
  const target = placements.find((p) => p.id === drop.before);
  if (!target) return placements;
  if ((moved.layer ?? null) !== (target.layer ?? null)) moved = { ...moved, layer: target.layer ?? null };
  const rest = placements.filter((p) => p.id !== dragged);
  const at = rest.findIndex((p) => p.id === drop.before);
  // L'ordre de la liste va du fond vers l'avant : devant = juste après.
  const next = [...rest.slice(0, at + 1), moved, ...rest.slice(at + 1)];
  return next.every((p, i) => p === placements[i]) ? placements : next;
}

/** Masque ou affiche, verrouille ou déverrouille ces dessins un à un (D-4.5-g). */
export function setFlag(
  placements: Placement[],
  ids: Set<string>,
  flag: "hidden" | "locked",
  value: boolean,
): Placement[] {
  let changed = false;
  const next = placements.map((p) => {
    if (!ids.has(p.id) || (p[flag] ?? false) === value) return p;
    changed = true;
    const q = { ...p };
    if (value) q[flag] = true;
    else delete q[flag];
    return q;
  });
  return changed ? next : placements;
}

/**
 * Un clic sur la ligne `id`, comme dans Illustrator : seul, il choisit ce
 * dessin ; ⌘ l'ajoute ou le retire ; ⇧ choisit toutes les lignes entre la
 * dernière cliquée (`anchor`) et celle-ci, dans l'ordre affiché (`order`).
 */
export function treeClick(
  order: string[],
  selected: Set<string>,
  anchor: string | null,
  id: string,
  mods: { toggle: boolean; range: boolean },
): Set<string> {
  if (mods.range && anchor !== null) {
    const a = order.indexOf(anchor);
    const b = order.indexOf(id);
    if (a >= 0 && b >= 0) {
      const [from, to] = a < b ? [a, b] : [b, a];
      return new Set([...(mods.toggle ? selected : []), ...order.slice(from, to + 1)]);
    }
  }
  if (mods.toggle) {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    return next;
  }
  return new Set([id]);
}

/**
 * Les dessins qui se montrent sur la façade : ni posés dans une diapositive
 * (D-3.6-g, leur position est en mm de diapositive), ni affectés à une
 * diapositive masquée dans l'onglet Calques (affichage seul, D15-u).
 */
export function onFacadeOnly(
  list: Placement[],
  assignments: Map<string, Assignment>,
  hiddenSlides: ReadonlySet<string>,
): Placement[] {
  return list.filter((p) => {
    if (p.slide) return false;
    const projector = assignments.get(p.id)?.projector;
    return !projector || !hiddenSlides.has(projector);
  });
}
