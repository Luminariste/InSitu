// Rubrique « Plans » de l'onglet Plan (D-PL-a à g) : le plan de référence,
// puis les plans ajoutés, rangés en groupes (un fichier, une ou plusieurs
// vues : pages ou calques). L'œil montre ou cache un groupe ou une vue,
// selon le moment du travail ; c'est enregistré dans le projet. Le plan
// choisi se règle en dessous : place, largeur, sens, calage par deux points.

import { useEffect, useState } from "react";
import { Button, Icon, IconButton, List, ListRow, Property, Section } from "../../ui";
import type { PlanGroup } from "../../engine";
import { formatMeters, parseMeters, scaleForWidth, groupRect } from "./planGroups";
import type { PlanGroupsState } from "./usePlanGroups";
import { rotationLabel } from "../rotation";

interface Props {
  state: PlanGroupsState;
  /** Nom du plan de référence ; `null` s'il n'y en a pas. */
  referenceName: string | null;
  referenceShown: boolean;
  onToggleReference: () => void;
  /** Pourquoi on ne peut pas ajouter de plan ; `null` si on peut. */
  blocked: string | null;
  disabled: boolean;
}

function NameField({ value, onDone }: { value: string; onDone: (name: string | null) => void }) {
  const [text, setText] = useState(value);
  return (
    <input
      className="ui-input facade-plan-name"
      autoFocus
      value={text}
      aria-label="Nom du plan"
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

/** Un champ en mètres, appliqué quand on quitte le champ ou qu'on presse ↩. */
function MetersField({ label, value, onCommit, disabled }: { label: string; value: number; onCommit: (mm: number) => void; disabled: boolean }) {
  const [text, setText] = useState(formatMeters(value));
  useEffect(() => setText(formatMeters(value)), [value]);
  const commit = () => {
    const mm = parseMeters(text);
    if (mm === null || Math.abs(mm - value) < 1e-6) {
      setText(formatMeters(value));
      return;
    }
    onCommit(mm);
  };
  return (
    <input
      className="ui-input"
      type="text"
      inputMode="decimal"
      aria-label={label}
      value={text}
      disabled={disabled}
      onChange={(e) => setText(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        e.stopPropagation();
        if (e.key === "Enter") commit();
        if (e.key === "Escape") setText(formatMeters(value));
      }}
    />
  );
}

function GroupSettings({ group, state, disabled }: { group: PlanGroup; state: PlanGroupsState; disabled: boolean }) {
  const { actions, mode, setMode } = state;
  const rect = groupRect(group);
  const aligning = mode !== "off" && mode !== "move";
  return (
    <Section title={`Plan choisi : ${group.name}`}>
      <Property label="Position" hint="Coin haut-gauche, en mètres depuis celui du plan de référence (horizontal, vertical).">
        <div className="facade-plan-xy">
          <MetersField label="Position horizontale" value={group.origin.x} disabled={disabled} onCommit={(x) => void actions.place(group.id, { x, y: group.origin.y }, group.mmPerUnit)} />
          <MetersField label="Position verticale" value={group.origin.y} disabled={disabled} onCommit={(y) => void actions.place(group.id, { x: group.origin.x, y }, group.mmPerUnit)} />
        </div>
      </Property>
      {rect && (
        <Property label="Largeur sur la façade" hint={`Hauteur : ${formatMeters(rect.height)}. Le coin haut-gauche ne bouge pas.`}>
          <MetersField
            label="Largeur du plan sur la façade"
            value={rect.width}
            disabled={disabled}
            onCommit={(width) => {
              const k = scaleForWidth(group, width);
              if (k !== null) void actions.place(group.id, group.origin, k);
            }}
          />
        </Property>
      )}
      <Property label="Orientation" hint={rotationLabel(group.rotation)}>
        <div className="facade-rotate">
          <IconButton icon="rotateLeft" label="Tourner ce plan d'un quart de tour à gauche" variant="default" size="sm" onClick={() => void actions.rotate(group.id, -1)} disabled={disabled} />
          <IconButton icon="rotateRight" label="Tourner ce plan d'un quart de tour à droite" variant="default" size="sm" onClick={() => void actions.rotate(group.id, 1)} disabled={disabled} />
        </div>
      </Property>
      <div className="facade-plan-tools">
        <Button
          icon="cursor"
          active={mode === "move"}
          onClick={() => setMode(mode === "move" ? "off" : "move")}
          disabled={disabled || group.hidden}
          title="Glisser sur la toile déplace ce plan (Échap : terminer)"
        >
          {mode === "move" ? "Terminer le déplacement" : "Déplacer à la souris"}
        </Button>
        <Button
          icon="target"
          active={aligning}
          onClick={() => setMode(aligning ? "off" : "from1")}
          disabled={disabled || group.hidden}
          title="Deux points du plan, puis leur place sur la façade : le plan est déplacé et mis à l'échelle"
        >
          {aligning ? "Annuler le calage" : "Caler par deux points…"}
        </Button>
        <Button icon="plus" onClick={() => void actions.chooseFile(group.id)} disabled={disabled} title="Ajouter d'autres pages ou calques du même fichier à ce groupe">
          Ajouter des calques ou des pages…
        </Button>
        <Button variant="danger" size="sm" icon="trash" onClick={() => void actions.remove(group.id, null)} disabled={disabled} title="Retirer ce plan et ses vues du projet (⌘Z pour revenir) ; le fichier reste dans facade/plans/">
          Retirer ce plan
        </Button>
      </div>
    </Section>
  );
}

export function PlansSection({ state, referenceName, referenceShown, onToggleReference, blocked, disabled }: Props) {
  const [editing, setEditing] = useState<string | null>(null);
  const { groups, selected, actions } = state;
  // Le plan du haut de la liste est devant, comme dans Illustrator.
  const top = [...groups].reverse();
  return (
    <>
      <Section
        title="Plans"
        actions={
          <IconButton
            icon="plus"
            label={blocked ?? "Ajouter un plan par-dessus le plan de référence : PDF, .ai, image, SVG ou DXF"}
            onClick={() => void actions.chooseFile()}
            disabled={blocked !== null || disabled}
          />
        }
      >
        <List label="Plans">
          {top.map((group) => {
            const index = groups.indexOf(group);
            return (
              <div key={group.id} className="facade-plan-group">
                <ListRow
                  selected={selected?.id === group.id}
                  onSelect={() => state.select(group.id)}
                  onDoubleClick={() => setEditing(group.id)}
                  dimmed={group.hidden}
                  tooltip="Clic : régler ce plan. Double-clic : renommer."
                  leading={
                    <IconButton
                      icon={group.hidden ? "eyeOff" : "eye"}
                      label={group.hidden ? "Montrer ce plan" : "Cacher ce plan"}
                      size="sm"
                      onClick={(e) => {
                        e.stopPropagation();
                        void actions.setHidden(group.id, null, !group.hidden);
                      }}
                    />
                  }
                  title={
                    editing === group.id ? (
                      <NameField
                        value={group.name}
                        onDone={(name) => {
                          setEditing(null);
                          if (name !== null && name.trim() && name !== group.name) void actions.rename(group.id, null, name);
                        }}
                      />
                    ) : (
                      group.name
                    )
                  }
                  trailing={group.plans.length > 1 ? group.plans.length : undefined}
                  actions={
                    <>
                      <IconButton icon="arrowUp" label="Monter ce plan (vers l'avant)" size="sm" disabled={index === groups.length - 1} onClick={(e) => { e.stopPropagation(); void actions.move(group.id, index + 1); }} />
                      <IconButton icon="arrowDown" label="Descendre ce plan (vers le fond)" size="sm" disabled={index === 0} onClick={(e) => { e.stopPropagation(); void actions.move(group.id, index - 1); }} />
                    </>
                  }
                />
                {group.plans.length > 1 && (
                  <div className="facade-plan-views">
                    {group.plans.map((plan) => (
                      <ListRow
                        key={plan.id}
                        onSelect={() => state.select(group.id)}
                        onDoubleClick={() => setEditing(plan.id)}
                        dimmed={group.hidden || plan.hidden}
                        tooltip="Une vue du même fichier : elle bouge avec son groupe. Double-clic : renommer."
                        leading={
                          <IconButton
                            icon={plan.hidden ? "eyeOff" : "eye"}
                            label={plan.hidden ? "Montrer cette vue" : "Cacher cette vue"}
                            size="sm"
                            onClick={(e) => {
                              e.stopPropagation();
                              void actions.setHidden(group.id, plan.id, !plan.hidden);
                            }}
                          />
                        }
                        title={
                          editing === plan.id ? (
                            <NameField
                              value={plan.name}
                              onDone={(name) => {
                                setEditing(null);
                                if (name !== null && name.trim() && name !== plan.name) void actions.rename(group.id, plan.id, name);
                              }}
                            />
                          ) : (
                            plan.name
                          )
                        }
                        actions={
                          <IconButton
                            icon="trash"
                            label="Retirer cette vue (⌘Z pour revenir)"
                            size="sm"
                            onClick={(e) => {
                              e.stopPropagation();
                              void actions.remove(group.id, plan.id);
                            }}
                          />
                        }
                      />
                    ))}
                  </div>
                )}
              </div>
            );
          })}
          {referenceName !== null && (
            <ListRow
              onSelect={() => state.select(null)}
              selected={selected === null && groups.length > 0}
              dimmed={!referenceShown}
              tooltip="Le plan de référence donne l'échelle ; les dessins posés et les recalages y sont attachés. Il ne se déplace pas."
              leading={
                <IconButton
                  icon={referenceShown ? "eye" : "eyeOff"}
                  label={referenceShown ? "Cacher le plan de référence" : "Montrer le plan de référence"}
                  size="sm"
                  onClick={(e) => {
                    e.stopPropagation();
                    onToggleReference();
                  }}
                />
              }
              title={referenceName}
              detail="Plan de référence"
              trailing={<Icon name="lock" />}
            />
          )}
        </List>
        {blocked && <p className="ui-property-hint">{blocked}</p>}
        {!blocked && groups.length === 0 && (
          <p className="ui-property-hint">
            « + » ajoute un plan par-dessus celui-ci : plan simplifié, plan de guides, calques d'un même .ai… On les montre ou les cache selon le moment du travail.
          </p>
        )}
      </Section>
      {selected && <GroupSettings group={selected} state={state} disabled={disabled} />}
    </>
  );
}
