// Raccourcis clavier : écriture, affichage à la manière du Mac, et
// reconnaissance d'une frappe. Logique pure, testée sans écran
// (`shortcuts.test.ts`). La liste des raccourcis de l'app est `APP_SHORTCUTS`,
// reprise par la fenêtre « Raccourcis clavier » et par docs/design.md.
//
// Écriture : touches séparées par « + », par exemple "mod+shift+n".
// `mod` est ⌘ sur Mac (Ctrl ailleurs, pour essayer dans un navigateur).

export type ShortcutSpec = string;

interface Parsed {
  mod: boolean;
  shift: boolean;
  alt: boolean;
  ctrl: boolean;
  key: string;
}

const KEY_NAMES: Record<string, string> = {
  backspace: "⌫",
  delete: "⌦",
  escape: "Échap",
  enter: "↩",
  arrowup: "↑",
  arrowdown: "↓",
  arrowleft: "←",
  arrowright: "→",
  space: "Espace",
  tab: "⇥",
};

export function parseShortcut(spec: ShortcutSpec): Parsed {
  const parts = spec.toLowerCase().split("+");
  const key = parts.pop() ?? "";
  return {
    mod: parts.includes("mod"),
    shift: parts.includes("shift"),
    alt: parts.includes("alt"),
    ctrl: parts.includes("ctrl"),
    key,
  };
}

/** Affichage dans l'ordre du Mac : ⌃ ⌥ ⇧ ⌘ puis la touche. */
export function formatShortcut(spec: ShortcutSpec, mac = true): string {
  const p = parseShortcut(spec);
  const key = KEY_NAMES[p.key] ?? p.key.toUpperCase();
  if (!mac) {
    return [p.ctrl || p.mod ? "Ctrl" : "", p.alt ? "Alt" : "", p.shift ? "Maj" : "", key].filter(Boolean).join("+");
  }
  return `${p.ctrl ? "⌃" : ""}${p.alt ? "⌥" : ""}${p.shift ? "⇧" : ""}${p.mod ? "⌘" : ""}${key}`;
}

/** Frappe clavier, réduite à ce qui compte pour un raccourci. */
export interface KeyInput {
  key: string;
  code?: string;
  metaKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
}

function eventKey(event: KeyInput): string {
  // Avec ⌥, la touche produit un autre caractère : on lit sa position.
  if (event.code?.startsWith("Key")) return event.code.slice(3).toLowerCase();
  if (event.code?.startsWith("Digit")) return event.code.slice(5);
  // Crochets : la touche, pas le caractère (⇧ ou un clavier AZERTY le changent).
  if (event.code === "BracketLeft") return "[";
  if (event.code === "BracketRight") return "]";
  if (event.key === " ") return "space";
  return event.key.toLowerCase();
}

export function matchesShortcut(spec: ShortcutSpec, event: KeyInput, mac = true): boolean {
  const p = parseShortcut(spec);
  const modDown = mac ? event.metaKey : event.ctrlKey;
  const ctrlDown = mac ? event.ctrlKey : false;
  if (p.mod !== modDown || p.shift !== event.shiftKey || p.alt !== event.altKey || p.ctrl !== ctrlDown) return false;
  const key = eventKey(event);
  if (p.key === "backspace") return key === "backspace" || key === "delete";
  return key === p.key;
}

/** Une frappe dans un champ de texte ne déclenche pas les raccourcis sans ⌘. */
export function isTextEntry(target: { tagName?: string; isContentEditable?: boolean; type?: string } | null): boolean {
  if (!target) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName?.toLowerCase();
  if (tag === "textarea") return true;
  if (tag !== "input") return false;
  return !["checkbox", "radio", "range", "button", "submit", "color"].includes(target.type ?? "text");
}

export function isMac(): boolean {
  return typeof navigator === "undefined" || /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);
}

/** Les raccourcis de l'app, par rubrique, tels qu'affichés dans l'aide. */
export const APP_SHORTCUTS: { group: string; items: { keys: ShortcutSpec; label: string }[] }[] = [
  {
    group: "Projet",
    items: [
      { keys: "mod+n", label: "Nouveau projet" },
      { keys: "mod+o", label: "Ouvrir un projet" },
      { keys: "mod+r", label: "Relire le projet depuis le disque" },
      { keys: "mod+z", label: "Annuler la dernière modification" },
      { keys: "mod+shift+z", label: "Rétablir" },
      { keys: "mod+i", label: "Importer (plan, photo ou dossier de dessins, selon l'écran)" },
      { keys: "mod+e", label: "Exporter (selon l'écran)" },
    ],
  },
  {
    group: "Étapes",
    items: [
      { keys: "mod+1", label: "Façade" },
      { keys: "mod+2", label: "Redressement" },
      { keys: "mod+3", label: "Recalage" },
      { keys: "mod+4", label: "Diapositive" },
      { keys: "mod+5", label: "Bibliothèque" },
    ],
  },
  {
    group: "Affichage",
    items: [
      { keys: "mod+0", label: "Tout voir" },
      { keys: "mod+/", label: "Raccourcis clavier" },
    ],
  },
  {
    group: "Dessins posés (Façade)",
    items: [
      { keys: "mod+a", label: "Choisir tous les dessins posés" },
      { keys: "mod+shift+a", label: "Ne rien choisir" },
      { keys: "mod+c", label: "Copier" },
      { keys: "mod+x", label: "Couper" },
      { keys: "mod+v", label: "Coller au centre de la vue" },
      { keys: "mod+f", label: "Coller devant, à la même place" },
      { keys: "mod+b", label: "Coller derrière, à la même place" },
      { keys: "mod+d", label: "Répéter le dernier geste (dupliquer)" },
      { keys: "mod+g", label: "Grouper" },
      { keys: "mod+shift+g", label: "Dégrouper" },
      { keys: "mod+]", label: "Avancer d'un cran" },
      { keys: "mod+[", label: "Reculer d'un cran" },
      { keys: "mod+shift+]", label: "Mettre au premier plan" },
      { keys: "mod+shift+[", label: "Mettre à l'arrière-plan" },
      { keys: "shift+arrowright", label: "Déplacer de 100 mm (flèches seules : 10 mm)" },
    ],
  },
  {
    group: "Sélection",
    items: [
      { keys: "arrowup", label: "Élément précédent de la liste" },
      { keys: "arrowdown", label: "Élément suivant de la liste" },
      { keys: "backspace", label: "Supprimer l'élément choisi" },
      { keys: "escape", label: "Annuler le geste en cours, fermer un menu" },
    ],
  },
];
