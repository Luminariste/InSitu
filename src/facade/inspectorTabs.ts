// Onglets de l'inspecteur de l'écran Façade (D15-i) : leur liste, et
// l'onglet que le contexte appelle. Logique pure, testée sans écran
// (`inspectorTabs.test.ts`).

export type FacadeTab = "plan" | "echelle" | "outils" | "calques" | "fresque" | "export";

export const FACADE_TABS: FacadeTab[] = ["plan", "echelle", "outils", "calques", "fresque", "export"];

/** Ce qui, à l'écran, appelle un onglet plutôt qu'un autre. */
export interface TabContext {
  /** Un plan (ou une façade d'essai) est affiché. */
  hasPlan: boolean;
  /** On est en train de cliquer les points A et B de l'échelle. */
  picking: boolean;
  /** Nombre de dessins posés choisis sur la façade. */
  selection: number;
}

/**
 * L'onglet à montrer après un changement de contexte. Seuls les
 * changements comptent : un onglet choisi à la main reste tant que rien de
 * neuf ne se passe.
 *  - des dessins viennent d'être choisis : Outils ;
 *  - la pose des points A et B commence : Mise en plan ;
 *  - le plan vient de disparaître (projet sans plan, plan retiré) : Plan.
 */
export function contextTab(previous: TabContext | null, current: TabContext, tab: FacadeTab): FacadeTab {
  if (previous === null) return current.hasPlan ? tab : "plan";
  if (current.selection > 0 && previous.selection === 0) return "outils";
  if (current.picking && !previous.picking) return "echelle";
  if (!current.hasPlan && previous.hasPlan) return "plan";
  return tab;
}

const STORAGE_KEY = "insitu.ongletFacade";

export function readFacadeTab(): FacadeTab {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved && (FACADE_TABS as string[]).includes(saved)) return saved as FacadeTab;
  } catch {
    // Stockage indisponible : onglet par défaut.
  }
  return "plan";
}

export function saveFacadeTab(tab: FacadeTab) {
  try {
    localStorage.setItem(STORAGE_KEY, tab);
  } catch {
    // Sans conséquence : l'onglet sera « Plan » au prochain lancement.
  }
}
