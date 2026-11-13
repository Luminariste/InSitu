// Options d'affichage de l'écran Façade (menu « Affichage » de la barre
// d'outils) : montrer ou cacher le plan, la fresque, les dessins posés, et
// passer le plan ou toute la vue en négatif. Ne touche pas au projet : c'est
// une façon de regarder, retenue d'un lancement à l'autre. Logique pure,
// testée sans écran (`viewOptions.test.ts`).

export interface ViewOptions {
  /** Le plan de la façade (PDF, image…). */
  plan: boolean;
  /** Plan en négatif : ses traits noirs sur blanc deviennent blancs sur noir. */
  planNegative: boolean;
  /** La fresque en une image (tâche 3.5). */
  fresco: boolean;
  /** Les dessins posés (tâche 4.2) et leurs contours. */
  drawings: boolean;
  /**
   * Toute la vue en négatif : fond blanc, traits noirs, comme une planche
   * Illustrator. Les couleurs des repères gardent leur teinte.
   */
  negative: boolean;
}

export const DEFAULT_VIEW_OPTIONS: ViewOptions = {
  plan: true,
  planNegative: false,
  fresco: true,
  drawings: true,
  negative: false,
};

/** Les options lues dans un texte enregistré ; ce qui manque prend la valeur par défaut. */
export function parseViewOptions(saved: string | null): ViewOptions {
  if (!saved) return DEFAULT_VIEW_OPTIONS;
  let raw: unknown;
  try {
    raw = JSON.parse(saved);
  } catch {
    return DEFAULT_VIEW_OPTIONS;
  }
  if (typeof raw !== "object" || raw === null) return DEFAULT_VIEW_OPTIONS;
  const record = raw as Record<string, unknown>;
  const options = { ...DEFAULT_VIEW_OPTIONS };
  for (const key of Object.keys(DEFAULT_VIEW_OPTIONS) as (keyof ViewOptions)[]) {
    if (typeof record[key] === "boolean") options[key] = record[key] as boolean;
  }
  return options;
}

/** Inverse une option. */
export function toggleViewOption(options: ViewOptions, key: keyof ViewOptions): ViewOptions {
  return { ...options, [key]: !options[key] };
}

/** Nombre d'éléments cachés ou passés en négatif : la pastille du menu. */
export function changedViewOptions(options: ViewOptions): number {
  return (Object.keys(DEFAULT_VIEW_OPTIONS) as (keyof ViewOptions)[]).filter((key) => options[key] !== DEFAULT_VIEW_OPTIONS[key]).length;
}

const STORAGE_KEY = "insitu.affichageFacade";

export function readViewOptions(): ViewOptions {
  try {
    return parseViewOptions(localStorage.getItem(STORAGE_KEY));
  } catch {
    return DEFAULT_VIEW_OPTIONS;
  }
}

export function saveViewOptions(options: ViewOptions) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(options));
  } catch {
    // Sans conséquence : tout sera montré au prochain lancement.
  }
}
