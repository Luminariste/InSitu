// Rubrique « Calques » du panneau de gauche de l'écran Façade (tâche 4.3).
// Le calque du haut de la liste est devant, comme dans Illustrator. Un clic
// choisit le calque où se posent les nouveaux dessins ; double-clic sur le
// nom pour le changer. L'œil masque le calque, le cadenas empêche de choisir
// ses dessins. Supprimer un calque retire ses dessins : ⌘Z le rétablit.

import { useState } from "react";
import { IconButton, List, ListRow, Section } from "../ui";
import { isLocked, isShown } from "./layers";
import type { Layer } from "./placement";
import type { LayerActions } from "./useComposition";

interface Props {
  /** Du fond vers l'avant. */
  layers: Layer[];
  active: string;
  counts: Map<string, number>;
  actions: LayerActions;
  /** Pourquoi on ne peut pas composer ; `null` si on peut. */
  blocked: string | null;
}

function NameField({ value, onDone }: { value: string; onDone: (name: string | null) => void }) {
  const [text, setText] = useState(value);
  return (
    <input
      className="ui-input compose-layer-name"
      autoFocus
      value={text}
      aria-label="Nom du calque"
      onChange={(e) => setText(e.target.value)}
      onClick={(e) => e.stopPropagation()}
      onBlur={() => onDone(text)}
      onKeyDown={(e) => {
        e.stopPropagation();
        if (e.key === "Enter") onDone(text);
        if (e.key === "Escape") onDone(null);
      }}
    />
  );
}

export function LayersSection({ layers, active, counts, actions, blocked }: Props) {
  const [editing, setEditing] = useState<string | null>(null);
  if (blocked) return null;
  const top = [...layers].reverse();
  const index = (id: string) => layers.findIndex((l) => l.id === id);
  const moveActive = (step: -1 | 1) => {
    // ↑ dans la liste : le calque au-dessus, donc plus en avant.
    const i = index(active) - step;
    if (i >= 0 && i < layers.length) actions.activate(layers[i].id);
  };
  return (
    <Section
      title="Calques"
      actions={<IconButton icon="plus" label="Nouveau calque au-dessus du calque choisi" onClick={actions.add} />}
    >
      <List label="Calques" onMove={moveActive}>
        {top.map((l) => {
          const i = index(l.id);
          const count = counts.get(l.id) ?? 0;
          const shown = isShown(l);
          const locked = isLocked(l);
          return (
            <div key={l.id} className="compose-layer" onDoubleClick={() => setEditing(l.id)}>
              <ListRow
                selected={l.id === active}
                onSelect={() => actions.activate(l.id)}
                dimmed={!shown}
                tooltip="Les nouveaux dessins se posent sur le calque choisi. Double-clic : renommer."
                leading={
                  <>
                    <IconButton
                      icon={shown ? "eye" : "eyeOff"}
                      label={shown ? "Masquer le calque" : "Afficher le calque"}
                      size="sm"
                      onClick={(e) => {
                        e.stopPropagation();
                        actions.toggleVisible(l.id);
                      }}
                    />
                    <IconButton
                      icon={locked ? "lock" : "lockOpen"}
                      label={locked ? "Déverrouiller le calque" : "Verrouiller le calque"}
                      size="sm"
                      active={locked}
                      onClick={(e) => {
                        e.stopPropagation();
                        actions.toggleLocked(l.id);
                      }}
                    />
                  </>
                }
                title={
                  editing === l.id ? (
                    <NameField
                      value={l.name}
                      onDone={(name) => {
                        setEditing(null);
                        if (name !== null) actions.rename(l.id, name);
                      }}
                    />
                  ) : (
                    l.name
                  )
                }
                trailing={count}
                actions={
                  <>
                    <IconButton
                      icon="arrowUp"
                      label="Monter le calque (vers l'avant)"
                      size="sm"
                      disabled={i === layers.length - 1}
                      onClick={() => actions.move(l.id, 1)}
                    />
                    <IconButton
                      icon="arrowDown"
                      label="Descendre le calque (vers le fond)"
                      size="sm"
                      disabled={i === 0}
                      onClick={() => actions.move(l.id, -1)}
                    />
                    <IconButton
                      icon="trash"
                      label={
                        count > 0
                          ? `Supprimer le calque et ses ${count} dessins (⌘Z pour revenir)`
                          : "Supprimer le calque"
                      }
                      size="sm"
                      onClick={() => actions.remove(l.id)}
                    />
                  </>
                }
              />
            </div>
          );
        })}
      </List>
    </Section>
  );
}
