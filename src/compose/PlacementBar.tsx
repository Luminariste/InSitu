// Barre des réglages du dessin posé, sous la barre d'outils de l'écran
// Façade, comme le panneau Contrôle d'Illustrator (demande de Benjamin,
// D-4.2-p) : nom, calque, diapositive, centre X et Y, largeur, rotation,
// état de la déformation, retirer. Tout s'enregistre aussitôt. Les autres
// outils (tourner, miroir, déformer, aligner, ordre, groupe) sont les icônes
// de la barre d'outils (CompositionToolbar, D15-j) ; l'onglet Dessins ne
// garde que la bibliothèque.

import { useEffect, useState, type ReactNode } from "react";
import type { Assignment } from "../engine";
import { cssColor, projectorColor } from "../facade/overview";
import { formatLength, parseDistanceMm } from "../facade/scale";
import { Button, Segmented, ToolSeparator } from "../ui";
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
  /** Faux quand la composition est indisponible (pas de plan, pas d'échelle, dessins cachés). */
  enabled: boolean;
  /** Mode 4 coins : les points de la déformation se tirent (D-4.2-s). */
  deforming: boolean;
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
      className="ui-input placement-input"
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

/** Une case de la barre : petit libellé, puis le réglage. */
function Item({ label, children, title }: { label: string; children: ReactNode; title?: string }) {
  return (
    <label className="placement-item" title={title}>
      <span className="placement-label">{label}</span>
      {children}
    </label>
  );
}

/** État de la déformation (tâche 4.4), en quelques mots. */
function deformationState(selected: Placement[]): string | null {
  const shared = sharedDeformation(selected);
  if (shared)
    return shared.columns === 1 && shared.rows === 1
      ? "Quatre coins : tirer les points bleus"
      : `Grille ${shared.columns} × ${shared.rows} : tirer les points bleus`;
  return selected.some((p) => p.deformation) ? "Déformations différentes" : null;
}

/** Diapositive de la sélection (tâche 4.5) : le choix, et l'explication en bulle d'aide. */
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
  const explanation =
    cut > 0
      ? `${cut === 1 && list.length === 1 ? "Le dessin est" : `${cut} dessin(s)`} à cheval : ${
          list.some((a) => a.candidates.length > 0)
            ? "une autre diapositive le contient en entier."
            : "aucune diapositive ne le contient en entier (le réduire ou le déplacer)."
        }`
      : list.some((a) => a.manual)
        ? "Choisie à la main."
        : "Proposée : le dessin y tient en entier, le plus loin du bord.";
  return (
    <Item label="Diapositive" title={explanation}>
      {same((a) => a.projector) && index >= 0 && (
        <span className="ui-swatch compose-swatch" style={{ background: cssColor(projectorColor(index)) }} />
      )}
      <select
        className="ui-select placement-select"
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
      {cut > 0 && <span className="placement-warning">à cheval</span>}
    </Item>
  );
}

export function PlacementBar({ selected, frame, drawings, actions, layers, projectors, assignments, enabled, deforming }: Props) {
  if (!enabled) return null;
  if (selected.length === 0 || !frame)
    return <p className="placement-empty">Dessin posé : cliquer un dessin sur la façade pour le régler ici.</p>;
  const center = rectCenter(frame);
  const single = selected.length === 1 ? selected[0] : null;
  // Un dessin seul : sa propre taille, même tourné ; plusieurs : leur cadre.
  const size = single ?? frame;
  const groups = new Set(selected.map((p) => p.group ?? `seul:${p.id}`));
  const name = single
    ? (drawings.find((d) => d.id === single.drawing)?.name ?? "Dessin")
    : `${selected.length} dessins`;
  const grouped = selected.some((p) => p.group);
  const layerIds = new Set(selected.map((p) => layerOf(layers, p).id));
  const deformation = deforming ? deformationState(selected) : null;
  return (
    <>
      <span className="placement-name" title={name}>
        {name}
        {grouped && groups.size === 1 ? " (groupe)" : ""}
        {single?.mirrored ? " · retourné" : ""}
      </span>
      {layers.length > 1 && (
        <Item label="Calque">
          <select
            className="ui-select placement-select"
            aria-label="Calque de la sélection"
            value={layerIds.size === 1 ? layerOf(layers, selected[0]).id : ""}
            onChange={(e) => e.target.value && actions.moveToLayer(e.target.value)}
          >
            {layerIds.size > 1 && <option value="">Plusieurs</option>}
            {[...layers].reverse().map((l) => (
              <option key={l.id} value={l.id}>
                {l.name}
              </option>
            ))}
          </select>
        </Item>
      )}
      <SlideChoice selected={selected} actions={actions} projectors={projectors} assignments={assignments} />
      <ToolSeparator />
      <Segmented
        label="Mode de la sélection"
        value={deforming ? "corners" : "normal"}
        onChange={actions.setMode}
        options={[
          { value: "normal", label: "Normal", title: "Cadre, poignées et rotation ; la déformation suit d'un bloc (Échap)" },
          { value: "corners", label: "4 coins", title: "Tirer les coins du dessin, en perspective" },
        ]}
      />
      <ToolSeparator />
      <Item label="X" title="Centre, horizontalement">
        <Field
          label="Centre, horizontalement"
          value={formatLength(center.x)}
          parse={parseSigned}
          onCommit={(x) => actions.moveTo({ x, y: center.y })}
        />
      </Item>
      <Item label="Y" title="Centre, verticalement">
        <Field
          label="Centre, verticalement"
          value={formatLength(center.y)}
          parse={parseSigned}
          onCommit={(y) => actions.moveTo({ x: center.x, y })}
        />
      </Item>
      <Item label="L" title={`Largeur ; hauteur : ${formatLength(size.height)} (proportions gardées)`}>
        <Field
          label="Largeur"
          value={formatLength(size.width)}
          parse={parseDistanceMm}
          onCommit={(w) => actions.resizeTo((w / size.width) * frame.width)}
        />
      </Item>
      <span className="placement-height" title="Hauteur (suit la largeur)">
        H {formatLength(size.height)}
      </span>
      {single && (
        <Item label="Angle" title="Rotation en degrés, dans le sens des aiguilles d'une montre">
          <Field
            label="Rotation en degrés"
            value={`${degrees.format(single.rotation)}°`}
            parse={parseDegrees}
            onCommit={actions.setRotation}
          />
        </Item>
      )}
      {deformation && <span className="placement-hint">{deformation}</span>}
      <ToolSeparator />
      <Button size="sm" variant="danger" icon="trash" shortcut="backspace" onClick={actions.remove}>
        Retirer
      </Button>
    </>
  );
}
