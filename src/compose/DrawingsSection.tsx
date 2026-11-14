// Rubrique « Dessins » du panneau de gauche de l'écran Façade (tâche 4.2) :
// la bibliothèque en petites vignettes, à glisser sur la façade. Un
// double-clic pose le dessin au centre de la vue. Les dessins sont classés
// par dossier, comme dans la Bibliothèque ; un clic sur un dossier le replie
// (D-4.2-o).

import { useMemo, useState } from "react";
import { Icon, Section } from "../ui";
import { asStrings, useRemembered } from "../ui/remember";
import { endDrawingDrag, startDrawingDrag } from "./dragDrawing";
import { drawingTree, type DrawingFolder } from "./drawingTree";
import type { ComposeDrawing } from "./useComposition";

interface Props {
  drawings: ComposeDrawing[];
  /** Pourquoi on ne peut pas poser de dessin ; `null` si on peut. */
  blocked: string | null;
  onPlace: (id: string) => void;
}

function Thumbnails({ drawings, onPlace }: { drawings: ComposeDrawing[]; onPlace: (id: string) => void }) {
  if (drawings.length === 0) return null;
  return (
    <div className="compose-drawings" role="list">
      {drawings.map((d) => (
        <div
          key={d.id}
          role="listitem"
          className="compose-drawing"
          draggable={!d.problem}
          title={`${d.folder ? `${d.folder}/` : ""}${d.name} — glisser sur la façade, ou double-cliquer`}
          onDragStart={(e) => startDrawingDrag(e, d.id)}
          onDragEnd={endDrawingDrag}
          onDoubleClick={() => !d.problem && onPlace(d.id)}
        >
          {d.url ? (
            <img src={d.url} alt="" draggable={false} loading="lazy" />
          ) : (
            <span className="compose-drawing-missing">?</span>
          )}
          <span className="compose-drawing-name">{d.name}</span>
        </div>
      ))}
    </div>
  );
}

function Folder({
  node,
  closed,
  toggle,
  onPlace,
}: {
  node: DrawingFolder<ComposeDrawing>;
  closed: (path: string) => boolean;
  toggle: (path: string) => void;
  onPlace: (id: string) => void;
}) {
  const open = !closed(node.path);
  return (
    <div role="treeitem" aria-expanded={open} aria-label={node.label}>
      <div
        className="compose-tree-head compose-folder-head"
        style={{ paddingLeft: 4 + 14 * node.depth }}
        onClick={() => toggle(node.path)}
        title={node.path}
      >
        <Icon name={open ? "chevronDown" : "chevronRight"} />
        <Icon name="folder" />
        <span className="compose-tree-label">{node.label}</span>
        <span className="compose-tree-count">{node.count}</span>
      </div>
      {open && (
        <div role="group">
          <div style={{ paddingLeft: 14 * (node.depth + 1) }}>
            <Thumbnails drawings={node.drawings} onPlace={onPlace} />
          </div>
          {node.children.map((c) => (
            <Folder key={c.path} node={c} closed={closed} toggle={toggle} onPlace={onPlace} />
          ))}
        </div>
      )}
    </div>
  );
}

export function DrawingsSection({ drawings, blocked, onPlace }: Props) {
  const [filter, setFilter] = useState("");
  // Dossiers repliés, retenus (D15-v).
  const [foldedList, setFoldedList] = useRemembered<string[]>("insitu.dossiersRepliesDessins", asStrings, []);
  const folded = new Set(foldedList);
  const tree = useMemo(() => drawingTree(drawings, filter), [drawings, filter]);
  // Pendant une recherche, tout est déplié pour voir ce qui est trouvé.
  const searching = filter.trim() !== "";
  const closed = (path: string) => !searching && folded.has(path);
  const toggle = (path: string) =>
    setFoldedList((current) => (current.includes(path) ? current.filter((p) => p !== path) : [...current, path]));
  return (
    <Section title="Dessins">
      {drawings.length === 0 ? (
        <p className="ui-list-empty">Glisser des dessins depuis le Finder sur la fenêtre, ou les importer dans l'étape Bibliothèque (⌘5).</p>
      ) : blocked ? (
        <p className="ui-list-empty">{blocked}</p>
      ) : (
        <>
          <input
            className="ui-input compose-filter"
            type="search"
            placeholder="Chercher un dessin ou un dossier"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            aria-label="Chercher un dessin"
          />
          <div className="compose-library" role="tree" aria-label="Dessins à poser, par dossier">
            {tree.count === 0 && <p className="ui-list-empty">Aucun dessin trouvé.</p>}
            <Thumbnails drawings={tree.drawings} onPlace={onPlace} />
            {tree.children.map((c) => (
              <Folder key={c.path} node={c} closed={closed} toggle={toggle} onPlace={onPlace} />
            ))}
          </div>
        </>
      )}
    </Section>
  );
}
