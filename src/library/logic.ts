// Logique pure de l'écran Bibliothèque (tâche 4.1) : recherche, dossiers,
// textes affichés. Testée sans écran (`logic.test.ts`).

import type { ImportReport, LibraryEntry, SharedCopyReport, SharedStatus } from "../engine";

/** Minuscules, sans accents ni espaces superflus : « Héron » → « heron ». */
export function normalize(text: string): string {
  return text
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .trim();
}

/** Nom du fichier d'un dessin, sans dossier ni extension (`bibliotheque/a/merle.png` → `merle`). */
export function fileStem(source: string): string {
  const base = source.split(/[\\/]/).pop() ?? "";
  const dot = base.lastIndexOf(".");
  return dot > 0 ? base.slice(0, dot) : base;
}

function words(query: string): string[] {
  return normalize(query).split(/\s+/).filter(Boolean);
}

/**
 * Vrai si chaque mot de la recherche se trouve dans le nom du dessin, dans
 * son dossier, ou dans le nom de son fichier (« oiseau mer » trouve
 * « Oiseaux/Mer/goéland » ; un dessin renommé se retrouve par son fichier).
 */
export function matches(entry: Pick<LibraryEntry, "name" | "folder"> & { source?: string }, query: string): boolean {
  const wanted = words(query);
  if (wanted.length === 0) return true;
  const haystack = normalize(`${entry.folder} ${entry.name} ${entry.source ? fileStem(entry.source) : ""}`);
  return wanted.every((word) => haystack.includes(word));
}

/** Vrai si `folder` est `parent` ou l'un de ses sous-dossiers ; `null` : tous. */
export function inFolder(folder: string, parent: string | null): boolean {
  return parent === null || folder === parent || folder.startsWith(`${parent}/`);
}

export function filterEntries<T extends Pick<LibraryEntry, "name" | "folder"> & { source?: string }>(
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

/**
 * L'arbre des dossiers, à plat, dans l'ordre alphabétique (accents compris).
 * `extra` : dossiers créés à la main, montrés même vides (D-4.1-g).
 */
export function folderTree(entries: Pick<LibraryEntry, "folder">[], extra: string[] = []): FolderRow[] {
  const counts = new Map<string, number>();
  const add = (folder: string, n: number) => {
    const parts = folder.split("/").filter(Boolean);
    for (let i = 1; i <= parts.length; i++) {
      const path = parts.slice(0, i).join("/");
      counts.set(path, (counts.get(path) ?? 0) + n);
    }
  };
  for (const { folder } of entries) if (folder) add(folder, 1);
  for (const folder of extra) add(folder, 0);
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

// --- Ranger la bibliothèque (D-4.1-g) -------------------------------------------

/** Dossier parent (`Oiseaux` pour `Oiseaux/Mer`, vide pour `Oiseaux`). */
export function parentFolder(path: string): string {
  const i = path.lastIndexOf("/");
  return i < 0 ? "" : path.slice(0, i);
}

/** Chemin d'un sous-dossier `name` de `parent` (vide : la racine). */
export function childFolder(parent: string | null, name: string): string {
  return parent ? `${parent}/${name}` : name;
}

/** « Nouveau dossier », ou « Nouveau dossier 2 »… s'il existe déjà dans `parent`. */
export function newFolderName(existing: string[], parent: string | null): string {
  const taken = new Set(existing);
  for (let n = 1; ; n++) {
    const name = n === 1 ? "Nouveau dossier" : `Nouveau dossier ${n}`;
    if (!taken.has(childFolder(parent, name))) return name;
  }
}

/** Dessins à ranger quand on glisse `id` : toute la sélection s'il en fait partie, sinon lui seul. */
export function draggedDrawings(id: string, selection: Set<string>): string[] {
  return selection.has(id) ? [...selection] : [id];
}

/** Phrase de compte rendu d'un rangement. */
export function moveSummary(names: string[], target: string): string {
  const where = target ? `dans « ${target} »` : "hors de tout dossier";
  if (names.length === 1) return `« ${names[0]} » rangé ${where}.`;
  return `${countLabel(names.length)} rangés ${where}.`;
}

// --- Bibliothèque commune (D-4.1-i) ----------------------------------------------

/** Étiquette d'une vignette : d'où vient le dessin, ou s'il a changé. */
export function sharedBadge(state: SharedStatus["state"] | null, commune: boolean): string | null {
  if (state === null) return null;
  if (commune) return state === "newer" ? "Projet à reprendre" : "Dans le projet";
  if (state === "newer") return "Nouvelle version";
  if (state === "missing") return "Retiré de la commune";
  return "Commune";
}

/** Phrase de compte rendu d'une copie vers le projet ou d'un envoi vers la commune. */
export function sharedSummary(report: SharedCopyReport, direction: "copier" | "envoyer"): string {
  const parts: string[] = [];
  const added = report.added.length;
  if (direction === "copier") {
    if (added > 0) parts.push(`${countLabel(added)} copié${added > 1 ? "s" : ""} dans le projet`);
    if (report.updated.length > 0) {
      const n = report.updated.length;
      parts.push(`${countLabel(n)} repris dans ${n > 1 ? "leur" : "sa"} nouvelle version`);
    }
    if (report.alreadyPresent > 0) {
      const n = report.alreadyPresent;
      parts.push(`${countLabel(n)} déjà à jour dans le projet`);
    }
  } else {
    if (added > 0) parts.push(`${countLabel(added)} envoyé${added > 1 ? "s" : ""} dans la bibliothèque commune`);
    if (report.alreadyPresent > 0) {
      const n = report.alreadyPresent;
      parts.push(`${countLabel(n)} déjà dans la bibliothèque commune`);
    }
  }
  if (report.failures.length > 0) parts.push(`échec : ${report.failures.join(", ")}`);
  if (parts.length === 0) return "Rien à faire.";
  const text = parts.join(", ");
  return `${text.charAt(0).toUpperCase()}${text.slice(1)}.`;
}

/** Une ligne de l'arbre pendant une recherche. */
export interface FoundFolder extends FolderRow {
  /** Vrai si le nom du dossier (ou de ses parents) répond à la recherche. */
  hit: boolean;
}

/**
 * L'arbre des dossiers pendant une recherche : les dossiers dont le chemin
 * répond à tous les mots, ceux qui contiennent des dessins trouvés, et leurs
 * parents (pour garder l'arbre lisible). `count` : dessins trouvés dedans.
 * Sans recherche, l'arbre entier, inchangé.
 */
export function searchFolders(
  rows: FolderRow[],
  entries: (Pick<LibraryEntry, "name" | "folder"> & { source?: string })[],
  query: string,
): FoundFolder[] {
  const wanted = words(query);
  if (wanted.length === 0) return rows.map((row) => ({ ...row, hit: false }));
  const found = entries.filter((entry) => matches(entry, query));
  const kept = new Set<string>();
  const keepWithParents = (path: string) => {
    const parts = path.split("/");
    for (let i = 1; i <= parts.length; i++) kept.add(parts.slice(0, i).join("/"));
  };
  const hits = new Set<string>();
  for (const row of rows) {
    const name = normalize(row.path);
    if (wanted.every((word) => name.includes(word))) {
      hits.add(row.path);
      keepWithParents(row.path);
    }
  }
  for (const entry of found) if (entry.folder) keepWithParents(entry.folder);
  return rows
    .filter((row) => kept.has(row.path))
    .map((row) => ({
      ...row,
      hit: hits.has(row.path),
      count: found.filter((entry) => inFolder(entry.folder, row.path)).length,
    }));
}

/** Résumé d'une recherche : « 3 dessins, 1 dossier ». */
export function searchSummary(drawings: number, folders: number): string {
  const d = drawings === 0 ? "aucun dessin" : drawings === 1 ? "1 dessin" : `${drawings} dessins`;
  if (folders === 0) return d;
  return `${d}, ${folders === 1 ? "1 dossier" : `${folders} dossiers`}`;
}
