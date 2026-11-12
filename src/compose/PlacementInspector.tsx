// Rubrique « Dessins posés » de l'inspecteur de l'écran Façade (tâche
// 4.2) : position, taille et rotation de la sélection, symétries,
// alignement, groupes, ordre. Tout s'enregistre aussitôt.

import { useEffect, useState } from "react";
import { formatLength, parseDistanceMm } from "../facade/scale";
import { Button, Property, Section } from "../ui";
import { rectCenter, type Placement, type Rect } from "./placement";
import type { CompositionActions, ComposeDrawing } from "./useComposition";

interface Props {
  selected: Placement[];
  frame: Rect | null;
  drawings: ComposeDrawing[];
  actions: CompositionActions;
}

/** Champ qui ne s'applique qu'à ↩ ou en quittant le champ ; Échap rétablit. */
function Field({
  value,
  onCommit,
  label,
  parse,
}: {
  value: string;
  onCommit: (v: number) => void;
  label: string;
  parse: (t: string) => number | null;
}) {
  const [text, setText] = useState(value);
  useEffect(() => setText(value), [value]);
  const commit = () => {
    const v = parse(text);
    if (v === null) setText(value);
    else if (text !== value) onCommit(v);
  };
  return (
    <input
      className="ui-input"
      type="text"
      inputMode="decimal"
      value={text}
      aria-label={label}
      onChange={(e) => setText(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") (e.target as HTMLInputElement).blur();
        if (e.key === "Escape") setText(value);
      }}
    />
  );
}

/** Longueur signée : « -1,2 m », « 350 », « 35 cm ». */
function parseSigned(text: string): number | null {
  const t = text.trim();
  const negative = t.startsWith("-") || t.startsWith("−");
  const v = parseDistanceMm(negative ? t.slice(1) : t);
  if (v === null) return /^[-−]?\s*0+([.,]0*)?\s*(mm|cm|m)?$/i.test(t) ? 0 : null;
  return negative ? -v : v;
}

function parseDegrees(text: string): number | null {
  const v = Number(text.replace("°", "").replace(",", ".").replace("−", "-").trim());
  return Number.isFinite(v) ? v : null;
}

const degrees = new Intl.NumberFormat("fr-FR", { maximumFractionDigits: 1 });

export function PlacementInspector({ selected, frame, drawings, actions }: Props) {
  if (selected.length === 0 || !frame) return null;
  const center = rectCenter(frame);
  const single = selected.length === 1 ? selected[0] : null;
  // Un dessin seul : sa propre taille, même tourné ; plusieurs : leur cadre.
  const size = single ?? frame;
  const groups = new Set(selected.map((p) => p.group ?? `seul:${p.id}`));
  const name = single
    ? (drawings.find((d) => d.id === single.drawing)?.name ?? "Dessin")
    : `${selected.length} dessins`;
  const grouped = selected.some((p) => p.group);
  return (
    <Section title="Dessins posés">
      <p className="compose-selection-name">
        {name}
        {grouped && groups.size === 1 ? " (groupe)" : ""}
      </p>
      <Property label="Centre X">
        <Field
          label="Centre, horizontalement"
          value={formatLength(center.x)}
          parse={parseSigned}
          onCommit={(x) => actions.moveTo({ x, y: center.y })}
        />
      </Property>
      <Property label="Centre Y">
        <Field
          label="Centre, verticalement"
          value={formatLength(center.y)}
          parse={parseSigned}
          onCommit={(y) => actions.moveTo({ x: center.x, y })}
        />
      </Property>
      <Property label="Largeur" hint={`Hauteur : ${formatLength(size.height)} (proportions gardées)`}>
        <Field
          label="Largeur"
          value={formatLength(size.width)}
          parse={parseDistanceMm}
          onCommit={(w) => actions.resizeTo((w / size.width) * frame.width)}
        />
      </Property>
      {single && (
        <Property label="Rotation">
          <Field
            label="Rotation en degrés"
            value={`${degrees.format(single.rotation)}°`}
            parse={parseDegrees}
            onCommit={actions.setRotation}
          />
        </Property>
      )}
      <div className="compose-buttons">
        <Button size="sm" onClick={() => actions.rotateBy(-90)} title="Tourner d'un quart de tour vers la gauche">
          ↺ 90°
        </Button>
        <Button size="sm" onClick={() => actions.rotateBy(90)} title="Tourner d'un quart de tour vers la droite">
          ↻ 90°
        </Button>
        <Button size="sm" onClick={() => actions.flip("horizontal")} title="Symétrie : échanger la gauche et la droite">
          ⇆ Symétrie
        </Button>
        <Button size="sm" onClick={() => actions.flip("vertical")} title="Symétrie : échanger le haut et le bas">
          ⇅ Symétrie
        </Button>
      </div>
      {groups.size > 1 && (
        <>
          <p className="compose-label">Aligner</p>
          <div className="compose-buttons compose-align">
            <Button size="sm" onClick={() => actions.align("left")} title="Aligner à gauche">
              ⇤
            </Button>
            <Button size="sm" onClick={() => actions.align("hcenter")} title="Centrer horizontalement">
              ↔
            </Button>
            <Button size="sm" onClick={() => actions.align("right")} title="Aligner à droite">
              ⇥
            </Button>
            <Button size="sm" onClick={() => actions.align("top")} title="Aligner en haut">
              ⤒
            </Button>
            <Button size="sm" onClick={() => actions.align("vcenter")} title="Centrer verticalement">
              ↕
            </Button>
            <Button size="sm" onClick={() => actions.align("bottom")} title="Aligner en bas">
              ⤓
            </Button>
          </div>
          {groups.size > 2 && (
            <div className="compose-buttons">
              <Button
                size="sm"
                onClick={() => actions.distribute("horizontal")}
                title="Même écart horizontal entre les dessins"
              >
                Répartir ↔
              </Button>
              <Button
                size="sm"
                onClick={() => actions.distribute("vertical")}
                title="Même écart vertical entre les dessins"
              >
                Répartir ↕
              </Button>
            </div>
          )}
        </>
      )}
      <p className="compose-label">Groupe et ordre</p>
      <div className="compose-buttons">
        <Button size="sm" shortcut="mod+g" onClick={actions.group} disabled={selected.length < 2}>
          Grouper
        </Button>
        <Button size="sm" shortcut="mod+shift+g" onClick={actions.ungroup} disabled={!grouped}>
          Dégrouper
        </Button>
        <Button
          size="sm"
          shortcut="mod+shift+]"
          onClick={() => actions.arrange("front")}
          title="Mettre au premier plan"
        >
          Premier plan
        </Button>
        <Button
          size="sm"
          shortcut="mod+shift+["
          onClick={() => actions.arrange("back")}
          title="Mettre à l'arrière-plan"
        >
          Arrière-plan
        </Button>
      </div>
      <div className="compose-buttons">
        <Button size="sm" variant="danger" icon="trash" shortcut="backspace" onClick={actions.remove}>
          Retirer de la façade
        </Button>
      </div>
    </Section>
  );
}
