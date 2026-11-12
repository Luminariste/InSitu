// Rubrique « Diapositives » du panneau de gauche de l'écran Façade : les
// dessins posés rangés par diapositive, comme les calques d'Illustrator
// (demande de Benjamin, 12 novembre 2026). Le dessin du haut d'un groupe est
// devant. Glisser une ligne sur le titre d'une autre diapositive l'y affecte
// à la main ; la glisser sur un dessin la passe juste devant lui. Un clic
// choisit le dessin sur la façade.

import { useState, type DragEvent } from "react";
import { cssColor, projectorColor } from "../facade/overview";
import { Icon, Section } from "../ui";
import type { Placement } from "./placement";
import type { SlideGroup, TreeDrop } from "./slideTree";

const ROW_DRAG_TYPE = "application/x-insitu-placement";

/** Ligne en cours de glisser (la vue web ne rend pas toujours les données pendant le survol). */
let dragging: string | null = null;

interface Props {
  groups: SlideGroup[];
  selected: Set<string>;
  /** Nom et vignette du dessin d'un dessin posé. */
  describe: (p: Placement) => { name: string; url?: string | null };
  onSelect: (id: string) => void;
  onDrop: (dragged: string, drop: TreeDrop) => void;
  /** Pourquoi l'arborescence n'est pas disponible ; `null` si elle l'est. */
  blocked: string | null;
}

export function SlidesSection({ groups, selected, describe, onSelect, onDrop, blocked }: Props) {
  const [closed, setClosed] = useState<Set<string>>(new Set());
  /** Cible survolée : `g:<projecteur>` ou `r:<dessin>`. */
  const [over, setOver] = useState<string | null>(null);
  if (blocked) return null;
  if (groups.length === 0) return null;

  const accept = (key: string) => (event: DragEvent) => {
    if (dragging === null && !event.dataTransfer.types.includes(ROW_DRAG_TYPE)) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
    if (over !== key) setOver(key);
  };
  const release = (drop: TreeDrop) => (event: DragEvent) => {
    const id = event.dataTransfer.getData(ROW_DRAG_TYPE) || dragging;
    dragging = null;
    setOver(null);
    if (!id) return;
    event.preventDefault();
    event.stopPropagation();
    onDrop(id, drop);
  };
  const toggle = (key: string) =>
    setClosed((before) => {
      const next = new Set(before);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });

  return (
    <Section title="Diapositives">
      <div
        className="compose-tree"
        role="tree"
        aria-label="Dessins par diapositive"
        onDragLeave={(e) => e.currentTarget === e.target && setOver(null)}
      >
        {groups.map((g) => {
          const key = g.projector ?? "aucune";
          const open = !closed.has(key);
          const droppable = g.projector !== null;
          const color = g.colorIndex === null ? null : cssColor(projectorColor(g.colorIndex));
          return (
            <div key={key} role="treeitem" aria-expanded={open} className="compose-tree-group">
              <div
                className={`compose-tree-head${over === `g:${key}` ? " is-over" : ""}`}
                onClick={() => toggle(key)}
                onDragOver={droppable ? accept(`g:${key}`) : undefined}
                onDrop={droppable ? release({ projector: g.projector! }) : undefined}
                title={
                  droppable
                    ? "Glisser un dessin ici pour l'envoyer dans cette diapositive"
                    : "Dessins qui ne tombent dans aucune emprise"
                }
              >
                <Icon name={open ? "chevronDown" : "chevronRight"} />
                <span className="ui-swatch" style={{ background: color ?? "transparent" }} />
                <span className="compose-tree-label">{g.label}</span>
                <span className="compose-tree-count">{g.rows.length}</span>
              </div>
              {open && (
                <div role="group">
                  {g.rows.length === 0 && <p className="compose-tree-empty">Aucun dessin</p>}
                  {g.rows.map((r) => {
                    const { name, url } = describe(r.placement);
                    const id = r.placement.id;
                    const notes = [
                      r.manual ? "choisie à la main" : null,
                      r.whole ? null : g.projector ? "à cheval" : null,
                    ].filter(Boolean);
                    return (
                      <div
                        key={id}
                        role="treeitem"
                        aria-selected={selected.has(id)}
                        className={`compose-tree-row${selected.has(id) ? " is-selected" : ""}${over === `r:${id}` ? " is-over" : ""}${r.whole ? "" : " is-cut"}`}
                        draggable
                        onDragStart={(e) => {
                          dragging = id;
                          e.dataTransfer.setData(ROW_DRAG_TYPE, id);
                          e.dataTransfer.effectAllowed = "move";
                        }}
                        onDragEnd={() => {
                          dragging = null;
                          setOver(null);
                        }}
                        onDragOver={droppable ? accept(`r:${id}`) : undefined}
                        onDrop={droppable ? release({ projector: g.projector!, before: id }) : undefined}
                        onClick={() => onSelect(id)}
                        title="Clic : choisir sur la façade. Glisser : sur une autre diapositive, ou au-dessus d'un dessin pour le passer devant."
                      >
                        {url ? <img src={url} alt="" draggable={false} /> : <span className="compose-tree-thumb" />}
                        <span className="compose-tree-label">{name}</span>
                        {notes.length > 0 && <span className="compose-tree-note">{notes.join(", ")}</span>}
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </Section>
  );
}
