// Rubrique « Dessins » du panneau de gauche de l'écran Façade (tâche 4.2) :
// la bibliothèque en petites vignettes, à glisser sur la façade. Un
// double-clic pose le dessin au centre de la vue.

import { useState } from "react";
import { Section } from "../ui";
import { DRAWING_DRAG_TYPE, type ComposeDrawing } from "./useComposition";

interface Props {
  drawings: ComposeDrawing[];
  /** Pourquoi on ne peut pas poser de dessin ; `null` si on peut. */
  blocked: string | null;
  onPlace: (id: string) => void;
}

export function DrawingsSection({ drawings, blocked, onPlace }: Props) {
  const [filter, setFilter] = useState("");
  const words = filter.trim().toLowerCase();
  const shown = words ? drawings.filter((d) => `${d.folder} ${d.name}`.toLowerCase().includes(words)) : drawings;
  return (
    <Section title="Dessins">
      {drawings.length === 0 ? (
        <p className="ui-list-empty">Importer des dessins dans l'étape Bibliothèque (⌘5).</p>
      ) : blocked ? (
        <p className="ui-list-empty">{blocked}</p>
      ) : (
        <>
          <input
            className="ui-input compose-filter"
            type="search"
            placeholder="Chercher un dessin"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            aria-label="Chercher un dessin"
          />
          <div className="compose-drawings" role="list" aria-label="Dessins à poser">
            {shown.map((d) => (
              <div
                key={d.id}
                role="listitem"
                className="compose-drawing"
                draggable={!d.problem}
                title={`${d.folder ? `${d.folder}/` : ""}${d.name} — glisser sur la façade, ou double-cliquer`}
                onDragStart={(e) => {
                  e.dataTransfer.setData(DRAWING_DRAG_TYPE, d.id);
                  e.dataTransfer.effectAllowed = "copy";
                }}
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
        </>
      )}
    </Section>
  );
}
