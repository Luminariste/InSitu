// Rubrique « Dessins posés » de l'inspecteur de l'écran Façade (tâche
// 4.2) : position, taille et rotation de la sélection, symétries,
// alignement, groupes, ordre. Tout s'enregistre aussitôt.

import { useEffect, useState } from "react";
import type { Assignment } from "../engine";
import { cssColor, projectorColor } from "../facade/overview";
import { formatLength, parseDistanceMm } from "../facade/scale";
import { Button, Property, Section } from "../ui";
import { layerOf } from "./layers";
import { rectCenter, sharedDeformation, type Layer, type Placement, type Rect } from "./placement";
import type { CompositionActions, ComposeDrawing } from "./useComposition";

interface Props {
  selected: Placement[];
  frame: Rect | null;
  drawings: ComposeDrawing[];
  actions: CompositionActions;
  /** Calques du projet (tâche 4.3) ; le choix n'apparaît qu'à partir de deux. */
  layers: Layer[];
  /** Projecteurs du projet, dans l'ordre (leurs couleurs suivent cet ordre). */
  projectors: { id: string; label: string }[];
  /** Diapositive de chaque dessin posé (tâche 4.5). */
  assignments: Map<string, Assignment>;
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

/** Grilles proposées (colonnes × rangées) : une rangée de six pour une voussure, des grilles carrées. */
const GRIDS: [number, number][] = [
  [2, 2],
  [3, 3],
  [4, 4],
  [6, 1],
  [6, 2],
  [8, 4],
];

/** Rubrique « Déformation » (tâche 4.4). */
function DeformationSection({ selected, actions }: { selected: Placement[]; actions: CompositionActions }) {
  const shared = sharedDeformation(selected);
  const some = selected.some((p) => p.deformation);
  const state = shared
    ? shared.columns === 1 && shared.rows === 1
      ? "Quatre coins : tirer les points bleus."
      : `Grille ${shared.columns} × ${shared.rows} : tirer les points bleus.`
    : some
      ? "Les dessins choisis n'ont pas la même déformation : en tendre une nouvelle pour les déformer ensemble."
      : selected.length > 1
        ? "Une grille tendue sur la sélection la déforme d'un bloc."
        : "Le dessin est droit.";
  return (
    <>
      <p className="compose-label">Déformation</p>
      <p className="compose-hint">{state}</p>
      <div className="compose-buttons">
        <Button
          size="sm"
          onClick={() => actions.deform(1, 1)}
          title="Quatre coins à tirer, en perspective (remplace la déformation actuelle)"
        >
          Quatre coins
        </Button>
        <select
          className="ui-select"
          aria-label="Tendre une grille de déformation"
          value=""
          onChange={(e) => {
            const [c, r] = e.target.value.split("x").map(Number);
            if (c > 0 && r > 0) actions.deform(c, r);
          }}
        >
          <option value="">Grille…</option>
          {GRIDS.map(([c, r]) => (
            <option key={`${c}x${r}`} value={`${c}x${r}`}>
              {c} × {r}
            </option>
          ))}
        </select>
        {some && (
          <Button size="sm" onClick={() => actions.deform(0, 0)} title="Le dessin redevient droit">
            Redresser
          </Button>
        )}
      </div>
    </>
  );
}

/** Rubrique « Diapositive » (tâche 4.5) : où part la sélection, et le choix à la main. */
function SlideChoice({
  selected,
  actions,
  projectors,
  assignments,
}: {
  selected: Placement[];
  actions: CompositionActions;
  projectors: { id: string; label: string }[];
  assignments: Map<string, Assignment>;
}) {
  const found = selected.map((p) => assignments.get(p.id));
  if (projectors.length === 0 || found.some((a) => !a)) return null;
  const list = found as Assignment[];
  const label = (id: string | null) => projectors.find((q) => q.id === id)?.label ?? "aucune";
  const same = (f: (a: Assignment) => unknown) => new Set(list.map(f)).size === 1;
  // Valeur du menu : « auto » si tout est proposé, l'identifiant si tout est choisi au même endroit.
  const value = list.every((a) => !a.manual) ? "auto" : same((a) => a.manual && a.projector) ? list[0].projector! : "";
  const proposal = same((a) => a.projector) ? label(list[0].projector) : "plusieurs";
  const cut = list.filter((a) => !a.whole).length;
  const index = projectors.findIndex((q) => q.id === list[0].projector);
  return (
    <>
      <Property label="Diapositive">
        <select
          className="ui-select"
          aria-label="Diapositive de la sélection"
          value={value}
          onChange={(e) => {
            if (e.target.value) actions.assignTo(e.target.value === "auto" ? null : e.target.value);
          }}
        >
          {value === "" && <option value="">Plusieurs</option>}
          <option value="auto">Automatique{list.every((a) => !a.manual) ? ` (${proposal})` : ""}</option>
          {projectors.map((q) => (
            <option key={q.id} value={q.id}>
              {q.label}
              {list.every((a) => a.candidates.includes(q.id)) ? "" : " (coupe le dessin)"}
            </option>
          ))}
        </select>
      </Property>
      <p className="compose-hint">
        {same((a) => a.projector) && index >= 0 && (
          <span className="ui-swatch compose-swatch" style={{ background: cssColor(projectorColor(index)) }} />
        )}
        {cut > 0
          ? `${cut === 1 && list.length === 1 ? "Le dessin est" : `${cut} dessin(s)`} à cheval : ${
              list.some((a) => a.candidates.length > 0)
                ? "une autre diapositive le contient en entier."
                : "aucune diapositive ne le contient en entier (le réduire ou le déplacer)."
            }`
          : list.some((a) => a.manual)
            ? "Choisie à la main."
            : "Proposée : le dessin y tient en entier, le plus loin du bord."}
      </p>
    </>
  );
}

export function PlacementInspector({ selected, frame, drawings, actions, layers, projectors, assignments }: Props) {
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
      {layers.length > 1 && (
        <Property label="Calque">
          <select
            className="ui-select"
            aria-label="Calque de la sélection"
            value={
              new Set(selected.map((p) => layerOf(layers, p).id)).size === 1 ? layerOf(layers, selected[0]).id : ""
            }
            onChange={(e) => e.target.value && actions.moveToLayer(e.target.value)}
          >
            {new Set(selected.map((p) => layerOf(layers, p).id)).size > 1 && <option value="">Plusieurs</option>}
            {[...layers].reverse().map((l) => (
              <option key={l.id} value={l.id}>
                {l.name}
              </option>
            ))}
          </select>
        </Property>
      )}
      <SlideChoice selected={selected} actions={actions} projectors={projectors} assignments={assignments} />
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
      <DeformationSection selected={selected} actions={actions} />
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
