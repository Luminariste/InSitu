// Garder la vue de la façade quand le projet est relu (⌘Z, ⇧⌘Z, ⌘R) :
// retour d'essai de Benjamin (13 novembre 2026), l'annulation recadrait tout
// le plan. La vue gardée ne vaut que si l'échelle du plan n'a pas changé
// (une échelle annulée change la taille du plan dans la visionneuse : on
// recadre alors). Décision D15-l.

import type { ViewState } from "../viewer";

export interface KeptView {
  view: ViewState;
  /** Millimètres par unité du plan au moment où la vue a été gardée. */
  k: number | null;
}

/** La vue à remettre après une relecture, ou `null` pour tout recadrer. */
export function viewToRestore(kept: KeptView | null, k: number | null): ViewState | null {
  if (!kept) return null;
  if (kept.k !== k) return null;
  return kept.view;
}

/** Une vue enregistrée (D15-v), relue ; `undefined` si elle est abîmée. */
export function parseKeptView(raw: unknown): KeptView | undefined {
  if (!raw || typeof raw !== "object") return undefined;
  const { view, k } = raw as { view?: Record<string, unknown>; k?: unknown };
  if (!view || typeof view !== "object") return undefined;
  const { centerX, centerY, zoom } = view;
  const finite = (x: unknown): x is number => typeof x === "number" && Number.isFinite(x);
  if (!finite(centerX) || !finite(centerY) || !finite(zoom) || zoom <= 0) return undefined;
  if (k !== null && !finite(k)) return undefined;
  return { view: { centerX, centerY, zoom }, k: k as number | null };
}
