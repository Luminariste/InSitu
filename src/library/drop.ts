// Dessins déposés depuis le Finder (demande de Benjamin, D-4.1-f) : lire ce
// que le Finder dépose (fichiers et dossiers), garder les dessins. La vue
// web du Mac ne donne pas le chemin d'un fichier déposé : l'app en lit les
// octets et les envoie au moteur (`importDroppedDrawings`, `src/engine.ts`).

/** Extensions des dessins, comme le moteur (`library::read::DRAWING_EXTENSIONS`). */
export const DRAWING_EXTENSIONS = ["png", "jpg", "jpeg", "tif", "tiff", "bmp", "psd", "svg"];

/** Un fichier déposé, avec son chemin dans le dossier déposé (`Oiseaux/merle.png`). */
export interface DroppedFile {
  file: File;
  path: string;
}

/** Vrai pour un nom de dessin lisible ; les fichiers cachés (`.DS_Store`, `._merle.png`) sont écartés. */
export function isDrawingName(path: string): boolean {
  const parts = path.split("/").filter(Boolean);
  if (parts.length === 0 || parts.some((p) => p.startsWith("."))) return false;
  const name = parts[parts.length - 1];
  const dot = name.lastIndexOf(".");
  return dot > 0 && DRAWING_EXTENSIONS.includes(name.slice(dot + 1).toLowerCase());
}

/** Vrai si le glisser apporte des fichiers du Finder (et pas un dessin de la bibliothèque). */
export function carriesFiles(types: readonly string[]): boolean {
  // Un glisser interne (dessin vers la façade, vignettes vers un dossier)
  // peut aussi annoncer « Files » dans la vue web : on l'ignore.
  return types.includes("Files") && !types.some((t) => t.startsWith("application/x-insitu"));
}

/**
 * JSON écrit en ASCII seul (les accents en `\uXXXX`), pour passer dans un
 * en-tête : `{"depot":"…","chemin":"Grands/héron.tif"}`.
 */
export function asciiJson(value: unknown): string {
  return JSON.stringify(value).replace(/[\u0080-￿]/g, (c) => `\\u${c.charCodeAt(0).toString(16).padStart(4, "0")}`);
}

/** Phrase de ce qui a été déposé et n'est pas un dessin. */
export function ignoredNotice(ignored: number): string | null {
  if (ignored === 0) return null;
  return ignored === 1
    ? "Un fichier déposé n'est pas un dessin (PNG, JPEG, TIFF, BMP, PSD, SVG) : il est laissé de côté."
    : `${ignored} fichiers déposés ne sont pas des dessins (PNG, JPEG, TIFF, BMP, PSD, SVG) : ils sont laissés de côté.`;
}

// --- Lecture du dépôt (navigateur) --------------------------------------------

interface Entry {
  isFile: boolean;
  isDirectory: boolean;
  name: string;
  file?(ok: (f: File) => void, ko: (e: unknown) => void): void;
  createReader?(): { readEntries(ok: (entries: Entry[]) => void, ko: (e: unknown) => void): void };
}

async function walk(entry: Entry, prefix: string, out: DroppedFile[], ignored: { count: number }): Promise<void> {
  const path = prefix ? `${prefix}/${entry.name}` : entry.name;
  if (entry.name.startsWith(".")) return;
  if (entry.isFile && entry.file) {
    if (!isDrawingName(path)) {
      ignored.count++;
      return;
    }
    const file = await new Promise<File>((ok, ko) => entry.file!(ok, ko));
    out.push({ file, path });
    return;
  }
  if (entry.isDirectory && entry.createReader) {
    const reader = entry.createReader();
    // Un dossier se lit par paquets, jusqu'à un paquet vide.
    for (;;) {
      const batch = await new Promise<Entry[]>((ok, ko) => reader.readEntries(ok, ko));
      if (batch.length === 0) break;
      for (const child of batch) await walk(child, path, out, ignored);
    }
  }
}

/**
 * Les dessins d'un dépôt du Finder, dossiers parcourus (un dossier déposé
 * garde son nom : `Oiseaux/merle.png`). À appeler pendant l'évènement
 * `drop` : les entrées ne sont lisibles qu'à ce moment-là (elles sont prises
 * tout de suite, lues ensuite).
 */
export async function droppedDrawings(data: DataTransfer): Promise<{ files: DroppedFile[]; ignored: number }> {
  const entries: Entry[] = [];
  const loose: File[] = [];
  for (const item of Array.from(data.items ?? [])) {
    if (item.kind !== "file") continue;
    const entry = (item as DataTransferItem & { webkitGetAsEntry?: () => Entry | null }).webkitGetAsEntry?.();
    if (entry) entries.push(entry);
    else {
      const file = item.getAsFile();
      if (file) loose.push(file);
    }
  }
  if (entries.length === 0 && loose.length === 0) loose.push(...Array.from(data.files ?? []));
  const out: DroppedFile[] = [];
  const ignored = { count: 0 };
  for (const entry of entries) await walk(entry, "", out, ignored);
  for (const file of loose) {
    if (isDrawingName(file.name)) out.push({ file, path: file.name });
    else ignored.count++;
  }
  return { files: out, ignored: ignored.count };
}
