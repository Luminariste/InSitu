// Calques de la composition (tâche 4.3). Fonctions pures : l'ordre de la
// liste des calques est l'ordre d'empilement, le premier dessous. Un dessin
// posé sans calque, ou sur un calque inconnu, appartient au premier calque,
// comme dans le moteur (`Project::stacked_placements`).
//
// Un projet sans calque se montre avec un seul calque, « Calque 1 » : il ne
// s'écrit dans le projet qu'à la première action sur les calques.

import { newId, type Layer, type Placement } from "./placement";

/** Le calque montré tant que le projet n'en a aucun. */
export const FIRST_LAYER: Layer = { id: "calque-1", name: "Calque 1" };

/** Les calques du projet, ou le calque implicite s'il n'y en a pas. */
export function effectiveLayers(layers: Layer[] | undefined): Layer[] {
  return layers && layers.length > 0 ? layers : [FIRST_LAYER];
}

/** Rang du calque d'un dessin posé (0 si absent ou inconnu). */
export function layerRank(layers: Layer[], p: Placement): number {
  const i = p.layer ? layers.findIndex((l) => l.id === p.layer) : -1;
  return i < 0 ? 0 : i;
}

/** Le calque d'un dessin posé. */
export function layerOf(layers: Layer[], p: Placement): Layer {
  const all = effectiveLayers(layers);
  return all[layerRank(all, p)];
}

/** Les dessins posés dans l'ordre d'affichage : par calque, puis dans l'ordre de la liste. */
export function stack(placements: Placement[], layers: Layer[]): Placement[] {
  if (layers.length < 2) return placements;
  return placements
    .map((p, i) => ({ p, i, r: layerRank(layers, p) }))
    .sort((a, b) => a.r - b.r || a.i - b.i)
    .map((x) => x.p);
}

export const isShown = (l: Layer) => l.visible !== false;
export const isLocked = (l: Layer) => l.locked === true;

/** Dessins affichés, dans l'ordre d'affichage. */
export function shown(placements: Placement[], layers: Layer[]): Placement[] {
  return stack(placements, layers).filter((p) => isShown(layerOf(layers, p)));
}

/** Dessins qu'on peut choisir à la souris : ni masqués ni verrouillés. */
export function pickable(placements: Placement[], layers: Layer[]): Placement[] {
  return stack(placements, layers).filter((p) => {
    const l = layerOf(layers, p);
    return isShown(l) && !isLocked(l);
  });
}

export interface Composition {
  layers: Layer[];
  placements: Placement[];
}

/** Rend explicite le calque de chaque dessin, pour qu'un changement d'ordre ne les déplace pas. */
function settle(layers: Layer[], placements: Placement[]): Placement[] {
  return placements.map((p) => {
    const id = layerOf(layers, p).id;
    return p.layer === id ? p : { ...p, layer: id };
  });
}

/** Nom libre suivant : « Calque N ». */
export function nextLayerName(layers: Layer[]): string {
  let n = 0;
  for (const l of layers) {
    const m = /^Calque (\d+)$/.exec(l.name);
    if (m) n = Math.max(n, Number(m[1]));
  }
  return `Calque ${Math.max(n, layers.length) + 1}`;
}

/** Ajoute un calque au-dessus de `above` (ou tout en haut). */
export function addLayer(c: Composition, above?: string, id = newId()): Composition & { added: Layer } {
  const layers = effectiveLayers(c.layers);
  const added: Layer = { id, name: nextLayerName(layers) };
  const at = above ? layers.findIndex((l) => l.id === above) + 1 : layers.length;
  const next = [...layers];
  next.splice(at <= 0 ? layers.length : at, 0, added);
  return { layers: next, placements: settle(layers, c.placements), added };
}

/** Supprime un calque et les dessins qu'il porte. */
export function removeLayer(c: Composition, id: string): Composition {
  const layers = effectiveLayers(c.layers);
  return {
    layers: layers.filter((l) => l.id !== id),
    placements: settle(layers, c.placements).filter((p) => p.layer !== id),
  };
}

/** Change un calque (nom, visibilité, verrou). */
export function updateLayer(c: Composition, id: string, change: Partial<Omit<Layer, "id">>): Composition {
  const layers = effectiveLayers(c.layers).map((l) => {
    if (l.id !== id) return l;
    const next: Layer = { ...l, ...change };
    // Les valeurs par défaut ne s'écrivent pas.
    if (next.visible !== false) delete next.visible;
    if (next.locked !== true) delete next.locked;
    return next;
  });
  return { layers, placements: settle(layers, c.placements) };
}

/** Monte (+1, vers l'avant) ou descend (-1) un calque. */
export function moveLayer(c: Composition, id: string, step: -1 | 1): Composition {
  const layers = effectiveLayers(c.layers);
  const i = layers.findIndex((l) => l.id === id);
  const j = i + step;
  if (i < 0 || j < 0 || j >= layers.length) return c;
  const placements = settle(layers, c.placements);
  const next = [...layers];
  [next[i], next[j]] = [next[j], next[i]];
  return { layers: next, placements };
}

/** Place des dessins posés sur un calque (ils passent devant ceux qui y sont déjà). */
export function moveToLayer(c: Composition, ids: Set<string>, id: string): Composition {
  const layers = effectiveLayers(c.layers);
  if (!layers.some((l) => l.id === id)) return c;
  const settled = settle(layers, c.placements);
  const moved = settled.filter((p) => ids.has(p.id)).map((p) => ({ ...p, layer: id }));
  return { layers, placements: [...settled.filter((p) => !ids.has(p.id)), ...moved] };
}
