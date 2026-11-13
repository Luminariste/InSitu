// Dessins de l'onglet Dessins classés comme dans la Bibliothèque (demande de
// Benjamin, D-4.2-o) : un dossier par sous-dossier importé, ses dessins
// dessous, ses sous-dossiers ensuite. Calcul pur, testé sans écran.

export interface TreeDrawing {
  id: string;
  name: string;
  /** Dossier dans la bibliothèque (« Poissons/Raies ») ; vide : à la racine. */
  folder: string;
}

export interface DrawingFolder<D extends TreeDrawing> {
  /** Chemin complet (« Poissons/Raies ») ; vide pour la racine. */
  path: string;
  /** Dernier nom du chemin. */
  label: string;
  depth: number;
  /** Dessins de ce dossier et de ses sous-dossiers. */
  count: number;
  /** Dessins rangés directement dans ce dossier, par nom. */
  drawings: D[];
  children: DrawingFolder<D>[];
}

const collator = new Intl.Collator("fr", { numeric: true, sensitivity: "base" });

/**
 * Arbre des dossiers. `filter` (mots séparés par des espaces) ne garde que les
 * dessins dont le dossier ou le nom contient chaque mot, et les dossiers qui
 * en contiennent encore.
 */
export function drawingTree<D extends TreeDrawing>(drawings: D[], filter = ""): DrawingFolder<D> {
  const words = filter.trim().toLowerCase().split(/\s+/).filter(Boolean);
  const kept = drawings.filter((d) => {
    const text = `${d.folder} ${d.name}`.toLowerCase();
    return words.every((w) => text.includes(w));
  });
  const root: DrawingFolder<D> = { path: "", label: "", depth: -1, count: 0, drawings: [], children: [] };
  for (const d of kept) {
    let node = root;
    node.count++;
    const parts = d.folder ? d.folder.split("/").filter(Boolean) : [];
    parts.forEach((part, i) => {
      let child = node.children.find((c) => c.label === part);
      if (!child) {
        child = { path: parts.slice(0, i + 1).join("/"), label: part, depth: i, count: 0, drawings: [], children: [] };
        node.children.push(child);
      }
      node = child;
      node.count++;
    });
    node.drawings.push(d);
  }
  const sort = (node: DrawingFolder<D>) => {
    node.drawings.sort((a, b) => collator.compare(a.name, b.name));
    node.children.sort((a, b) => collator.compare(a.label, b.label));
    node.children.forEach(sort);
  };
  sort(root);
  return root;
}
