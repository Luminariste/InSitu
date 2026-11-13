// Vue diapositive (tâche 3.6) : logique pure, testée dans logic.test.ts.

import type { SlideView } from "../engine";
import type { LayerSpec } from "../viewer";

export const SLIDE_MM = 280;

/** Couleurs proposées pour les dessins posés sur la photo. */
export const TINTS: { value: number; label: string }[] = [
  { value: 0x00e5ff, label: "Cyan" },
  { value: 0xffe600, label: "Jaune" },
  { value: 0xff3df2, label: "Magenta" },
  { value: 0xffffff, label: "Blanc" },
  { value: 0x000000, label: "Noir" },
];

/** Blanc et noir : les deux teintes qu'un fond peut rendre invisibles. */
export const WHITE = 0xffffff;
export const BLACK = 0x000000;

/** Fond du carré de la diapositive, vu quand la photo est cachée. */
export type SlideBackdrop = "noir" | "blanc" | "pierre";

export const SLIDE_BACKDROPS: { value: SlideBackdrop; label: string; title: string; color: number }[] = [
  { value: "noir", label: "Noir", title: "Fond noir : la diapositive telle qu'elle part dans LightBurn", color: 0x000000 },
  { value: "blanc", label: "Blanc", title: "Fond blanc : les dessins en noir", color: 0xffffff },
  { value: "pierre", label: "Pierre", title: "Couleur de pierre, comme sur la façade", color: 0x8a7764 },
];

/** Ce qu'on regarde dans l'écran Diapositive. */
export interface SlideLook {
  /** La photo redressée sous les dessins. */
  photo: boolean;
  backdrop: SlideBackdrop;
  showDrawings: boolean;
  tint: number;
  opacity: number;
}

export const DEFAULT_LOOK: SlideLook = { photo: true, backdrop: "noir", showDrawings: true, tint: TINTS[0].value, opacity: 0.9 };

/** La diapositive telle qu'elle est envoyée à LightBurn : traits blancs pleins sur noir, sans photo. */
export const LIGHTBURN_LOOK: SlideLook = { photo: false, backdrop: "noir", showDrawings: true, tint: WHITE, opacity: 1 };

/** Vrai si l'aperçu montre exactement ce qui part dans LightBurn. */
export function isLightBurnLook(look: SlideLook): boolean {
  return !look.photo && look.backdrop === "noir" && look.showDrawings && look.tint === WHITE && look.opacity === 1;
}

/**
 * Change le fond. Des traits blancs sur fond blanc (ou noirs sur fond noir
 * ou pierre) ne se verraient pas : la couleur des dessins passe alors au
 * noir (au blanc).
 */
export function withBackdrop(look: SlideLook, backdrop: SlideBackdrop): SlideLook {
  let tint = look.tint;
  if (backdrop === "blanc" && tint === WHITE) tint = BLACK;
  if (backdrop !== "blanc" && tint === BLACK) tint = WHITE;
  return { ...look, backdrop, tint };
}

/** Le carré de 280 mm rempli de la couleur du fond, sous la photo et les dessins. */
export function backdropLayer(backdrop: SlideBackdrop): LayerSpec {
  const color = SLIDE_BACKDROPS.find((b) => b.value === backdrop)?.color ?? 0;
  const [a, b, c, d] = SLIDE_SQUARE;
  return {
    id: "fond-diapositive",
    kind: "polygons",
    shapes: [{ points: [a, b, d, c], color, fillAlpha: 1, strokeAlpha: 0 }],
  };
}

/** Le réglage relu ; ce qui manque ou est abîmé prend la valeur par défaut. */
export function parseLook(saved: string | null): SlideLook {
  let raw: Record<string, unknown> = {};
  try {
    const parsed: unknown = saved ? JSON.parse(saved) : {};
    if (parsed && typeof parsed === "object") raw = parsed as Record<string, unknown>;
  } catch {
    return DEFAULT_LOOK;
  }
  const look = { ...DEFAULT_LOOK };
  if (typeof raw.photo === "boolean") look.photo = raw.photo;
  if (typeof raw.showDrawings === "boolean") look.showDrawings = raw.showDrawings;
  if (SLIDE_BACKDROPS.some((b) => b.value === raw.backdrop)) look.backdrop = raw.backdrop as SlideBackdrop;
  if (TINTS.some((t) => t.value === raw.tint)) look.tint = raw.tint as number;
  if (typeof raw.opacity === "number" && raw.opacity >= 0 && raw.opacity <= 1) look.opacity = raw.opacity;
  return look;
}

const LOOK_KEY = "insitu.affichageDiapositive";

export function readLook(): SlideLook {
  try {
    return parseLook(localStorage.getItem(LOOK_KEY));
  } catch {
    return DEFAULT_LOOK;
  }
}

export function saveLook(look: SlideLook) {
  try {
    localStorage.setItem(LOOK_KEY, JSON.stringify(look));
  } catch {
    // Sans conséquence : réglage par défaut au prochain lancement.
  }
}

/** Le carré de la diapositive, coin haut-gauche puis bas-droit (maillage 1 × 1). */
export const SLIDE_SQUARE = [
  { x: 0, y: 0 },
  { x: SLIDE_MM, y: 0 },
  { x: 0, y: SLIDE_MM },
  { x: SLIDE_MM, y: SLIDE_MM },
];

/** Calque des dessins calculés, posés sur la diapositive (mm). */
export function drawingsLayer(url: string | null, visible: boolean, opacity: number, tint: number): LayerSpec[] {
  if (!url) return [];
  return [
    {
      id: "diapositive-calculee",
      kind: "mesh-image",
      url,
      divisions: 1,
      points: SLIDE_SQUARE,
      opacity,
      visible,
      tint,
    },
  ];
}

/** Projecteur suivant (`step` = 1) ou précédent (-1), en boucle. */
export function nextProjector(views: SlideView[], current: string | null, step: number): string | null {
  if (views.length === 0) return null;
  const index = views.findIndex((v) => v.projectorId === current);
  if (index < 0) return views[0].projectorId;
  return views[(index + step + views.length) % views.length].projectorId;
}

/** Ce qu'on dit d'un projecteur quand rien ne peut être montré. */
export function viewProblem(view: SlideView | null): string | null {
  if (!view) return null;
  if (!view.rectifiedPhoto && !view.preview) {
    return `${view.label} : pas de photo redressée${view.problem ? ` ; ${view.problem}` : ""}.`;
  }
  if (!view.rectifiedPhoto) return `${view.label} : pas de photo redressée (onglet Redressement).`;
  if (!view.preview) return `${view.label} : ${view.problem ?? "diapositive non calculée"}.`;
  return null;
}

/**
 * Les dessins de la diapositive qui en dépassent (D-4.5-h) : la part hors
 * de l'emprise n'est projetée par aucune diapositive.
 */
export function cutNotice(view: SlideView | null): string | null {
  if (!view || view.cut.length === 0) return null;
  const names = view.cut.map((c) => `« ${c.name} »`).join(", ");
  const verb = view.cut.length > 1 ? "dépassent" : "dépasse";
  const total = view.cut.reduce((sum, c) => sum + c.lostM2, 0);
  const area = total.toLocaleString("fr-FR", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  return `${view.label} : ${names} ${verb} de cette diapositive ; ${area} m² ne seront pas projetés (en rouge sur la façade).`;
}
