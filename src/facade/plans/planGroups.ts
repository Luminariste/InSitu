// Plans ajoutés par-dessus le plan de référence (demande de Benjamin du
// 13 novembre 2026, décisions D-PL-a à g) : ce qu'il faut pour les afficher
// et les manipuler. Logique pure, testée sans écran (`planGroups.test.ts`).

import type { FacadeView, GroupPlan, PlanChoice, PlanGroup, PlanInfo } from "../../engine";
import type { LayerSpec, Point, Rect } from "../../viewer";

/**
 * Clé d'une vue dans la table des affichages préparés : un quart de tour du
 * groupe demande un autre affichage.
 */
export function viewKey(group: PlanGroup, plan: GroupPlan): string {
  return `${group.id}/${plan.id}/q${(group.rotation ?? 0) % 4}`;
}

/** Vrai si la vue est montrée : ni elle ni son groupe ne sont cachés. */
export function isShown(group: PlanGroup, plan: GroupPlan): boolean {
  return !group.hidden && !plan.hidden;
}

/** Rectangle occupé par le groupe sur la façade, en mm (celui de sa première vue). */
export function groupRect(group: PlanGroup, origin = group.origin, mmPerUnit = group.mmPerUnit): Rect | null {
  const first = group.plans[0];
  if (!first) return null;
  return { x: origin.x, y: origin.y, width: first.sourceWidth * mmPerUnit, height: first.sourceHeight * mmPerUnit };
}

/** Place provisoire d'un groupe pendant un geste (déplacement à la souris). */
export interface GroupOverride {
  group: string;
  origin: Point;
  mmPerUnit: number;
}

/**
 * Calques de la visionneuse pour les plans ajoutés : sous chaque groupe
 * montré, une feuille blanche (sans elle, ses traits noirs disparaîtraient
 * sur le fond sombre de la visionneuse), puis ses vues. Les vues se
 * superposent comme des calques de papier calque (« produit ») : le blanc
 * laisse voir dessous, les traits s'ajoutent. En négatif, l'inverse
 * (« superposition claire ») sur le fond sombre, sans feuille.
 */
export function planGroupLayers(
  groups: PlanGroup[],
  views: ReadonlyMap<string, FacadeView>,
  options: { negative: boolean; override?: GroupOverride | null },
): { sheets: LayerSpec[]; plans: LayerSpec[] } {
  const sheets: LayerSpec[] = [];
  const plans: LayerSpec[] = [];
  for (const group of groups) {
    const moved = options.override?.group === group.id ? options.override : null;
    const origin = moved?.origin ?? group.origin;
    const mmPerUnit = moved?.mmPerUnit ?? group.mmPerUnit;
    const shown = group.plans.filter((plan) => isShown(group, plan) && views.has(viewKey(group, plan)));
    if (shown.length === 0) continue;
    if (!options.negative) {
      const shapes = shown.map((plan) => {
        const w = plan.sourceWidth * mmPerUnit;
        const h = plan.sourceHeight * mmPerUnit;
        const { x, y } = origin;
        return {
          points: [
            { x, y },
            { x: x + w, y },
            { x: x + w, y: y + h },
            { x, y: y + h },
          ],
          color: 0xffffff,
          strokeAlpha: 0,
          fillAlpha: 1,
        };
      });
      sheets.push({ id: `plan-feuille-${group.id}`, kind: "polygons", shapes });
    }
    for (const plan of shown) {
      const view = views.get(viewKey(group, plan))!;
      plans.push({
        id: `plan-${group.id}-${plan.id}`,
        kind: "tiles",
        source: view.source,
        placement: { x: origin.x, y: origin.y, mmPerPixel: view.sourceUnitsPerPixel * mmPerUnit },
        blend: options.negative ? "screen" : "multiply",
        invert: options.negative,
      });
    }
  }
  return { sheets, plans };
}

/** Nouvelle origine d'un groupe glissé de `from` à `to` (mm). */
export function dragOrigin(origin: Point, from: Point, to: Point): Point {
  return { x: origin.x + to.x - from.x, y: origin.y + to.y - from.y };
}

/**
 * Échelle qui donne au groupe la largeur `widthMm` sur la façade (largeur
 * de sa première vue). `null` si la largeur n'est pas utilisable.
 */
export function scaleForWidth(group: PlanGroup, widthMm: number): number | null {
  const first = group.plans[0];
  if (!first || !(widthMm > 0) || !Number.isFinite(widthMm) || !(first.sourceWidth > 0)) return null;
  return widthMm / first.sourceWidth;
}

/** Étapes du calage par deux points : un point du plan, puis sa place sur la façade, deux fois. */
export type AlignStep = "from1" | "to1" | "from2" | "to2";

export const ALIGN_STEPS: AlignStep[] = ["from1", "to1", "from2", "to2"];

/** Ce qu'il faut cliquer à chaque étape du calage. */
export function alignPrompt(step: AlignStep): string {
  switch (step) {
    case "from1":
      return "Calage, point 1 : cliquer un point repérable sur le plan à caler";
    case "to1":
      return "Point 1′ : cliquer l'endroit où ce point doit venir sur la façade";
    case "from2":
      return "Point 2 : cliquer un second point, loin du premier, sur le plan à caler";
    case "to2":
      return "Point 2′ : cliquer l'endroit où ce second point doit venir";
  }
}

/** Étape suivante, ou `null` quand les quatre points sont posés. */
export function nextAlignStep(step: AlignStep): AlignStep | null {
  const index = ALIGN_STEPS.indexOf(step);
  return ALIGN_STEPS[index + 1] ?? null;
}

/**
 * Vues à créer à partir d'un fichier, selon ce qui est coché : une vue par
 * calque coché (D-PL-c) ; sans calque, une vue par page cochée.
 */
export function choicesFor(info: PlanInfo, pages: number[], layers: string[]): PlanChoice[] {
  const page = pages[0] ?? info.page ?? 1;
  if ((info.layers?.length ?? 0) > 0) {
    return layers.map((name) => ({ page: info.kind === "pdf" ? page : null, layers: [name], name }));
  }
  if (info.kind === "pdf" && (info.pageCount ?? 1) > 1) {
    return pages.map((p) => ({ page: p }));
  }
  return [{}];
}

/** Formate une longueur en mm en mètres, à la française. */
export function formatMeters(mm: number): string {
  return `${(mm / 1000).toLocaleString("fr-FR", { maximumFractionDigits: 3 })} m`;
}

/** Lit une longueur écrite en mètres (« 12,5 » ou « 12,5 m ») et la rend en mm. */
export function parseMeters(text: string): number | null {
  const cleaned = text.trim().replace(/\s+/g, "").replace(/m$/i, "").replace(",", ".");
  if (cleaned === "") return null;
  const value = Number(cleaned);
  return Number.isFinite(value) ? value * 1000 : null;
}
