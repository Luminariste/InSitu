// Logique pure de l'écran Bibliothèque (tâche 4.1) : recherche, dossiers,
// textes affichés. Testée sans écran (`logic.test.ts`).

import type { ImportReport, LibraryEntry } from "../engine";

/** Minuscules, sans accents ni espaces superflus : « Héron » → « heron ». */
export function normalize(text: string): string {
  return text
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .trim();
}

/**
 * Vrai si chaque mot de la recherche se trouve dans le nom du dessin ou dans
 * son dossier (« oiseau mer » trouve « Oiseaux/Mer/goéland »).
 */
export function matches(entry: Pick<LibraryEntry, "name" | "folder">, query: string): boolean {
  const words = normalize(query).split(/\s+/).filter(Boolean);
  if (words.length === 0) return true;
  const haystack = normalize(`${entry.folder} ${entry.name}`);
  return words.every((word) => haystack.includes(word));
}

/** Vrai si `folder` est `parent` ou l'un de ses sous-dossiers ; `null` : tous. */
export function inFolder(folder: string, parent: string | null): boolean {
  return parent === null || folder === parent || folder.startsWith(`${parent}/`);
}

export function filterEntries<T extends Pick<LibraryEntry, "name" | "folder">>(
  entries: T[],
  query: string,
  folder: string | null,
): T[] {
  return entries.filter((entry) => inFolder(entry.folder, folder) && matches(entry, query));
}

export interface FolderRow {
  /** Chemin complet (`Oiseaux/Grands`). */
  path: string;
  /** Dernier nom du chemin, affiché en retrait selon `depth`. */
  label: string;
  depth: number;
  /** Dessins dans ce dossier et ses sous-dossiers. */
  count: number;
}

/** L'arbre des dossiers, à plat, dans l'ordre alphabétique (accents compris). */
export function folderTree(entries: Pick<LibraryEntry, "folder">[]): FolderRow[] {
  const counts = new Map<string, number>();
  for (const { folder } of entries) {
    if (!folder) continue;
    const parts = folder.split("/");
    for (let i = 1; i <= parts.length; i++) {
      const path = parts.slice(0, i).join("/");
      counts.set(path, (counts.get(path) ?? 0) + 1);
    }
  }
  const collator = new Intl.Collator("fr", { numeric: true, sensitivity: "base" });
  return [...counts.entries()]
    .sort(([a], [b]) => {
      const pa = a.split("/");
      const pb = b.split("/");
      for (let i = 0; i < Math.min(pa.length, pb.length); i++) {
        const c = collator.compare(pa[i], pb[i]);
        if (c !== 0) return c;
      }
      return pa.length - pb.length;
    })
    .map(([path, count]) => {
      const parts = path.split("/");
      return { path, label: parts[parts.length - 1], depth: parts.length - 1, count };
    });
}

const integer = new Intl.NumberFormat("fr-FR", { maximumFractionDigits: 0 });

/** « 4 096 × 4 096 px », ou « 400 × 200 » pour un SVG (unités du fichier). */
export function formatSize(entry: Pick<LibraryEntry, "width" | "height" | "source">): string {
  const svg = entry.source.toLowerCase().endsWith(".svg");
  return `${integer.format(entry.width)} × ${integer.format(entry.height)}${svg ? " (vectoriel)" : " px"}`;
}

/** « 1 dessin », « 200 dessins ». */
export function countLabel(n: number): string {
  return `${integer.format(n)} dessin${n > 1 ? "s" : ""}`;
}

/** Phrase de compte rendu d'un import. */
export function importSummary(report: ImportReport): string {
  const parts: string[] = [];
  const added = report.added.length;
  parts.push(added === 0 ? "Aucun nouveau dessin" : `${countLabel(added)} ajouté${added > 1 ? "s" : ""}`);
  if (report.alreadyPresent > 0) {
    const n = report.alreadyPresent;
    parts.push(`${countLabel(n)} déjà présent${n > 1 ? "s" : ""}`);
  }
  if (report.failures.length > 0) {
    const n = report.failures.length;
    parts.push(`${n} fichier${n > 1 ? "s" : ""} illisible${n > 1 ? "s" : ""} (${report.failures.map((f) => f.file).join(", ")})`);
  }
  return `${parts.join(", ")}.`;
}

/** Texte de la barre d'avancement : « 37 / 200 ». */
export function progressLabel(done: number, total: number): string {
  return `Import : ${integer.format(done)} / ${integer.format(total)}`;
}
