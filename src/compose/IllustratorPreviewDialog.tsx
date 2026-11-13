// Fenêtre d'aperçu de l'import Illustrator (demande de Benjamin, 13 novembre
// 2026) : montre, avant de rien écrire, où chaque image du .ai sera posée,
// dans quel calque, et si son dessin est déjà dans la bibliothèque. On coche
// les calques ou les fichiers à importer ; « Importer » ne prend que ceux-là.

import { useMemo, useState } from "react";
import { Button, Dialog, Icon } from "../ui";
import type { CompositionPreview } from "../engine";
import {
  checkState,
  choiceSummary,
  choiceText,
  defaultChoice,
  emptyLayers,
  importable,
  layerColor,
  layerGroups,
  STATUS_HINT,
  STATUS_LABEL,
  toggleImages,
  type CheckState,
} from "./illustratorLogic";

interface Props {
  preview: CompositionPreview;
  onImport: (images: number[]) => void;
  /** Poser le .ai en un bloc, comme fond de référence, sans dessins. */
  onBackground: () => void;
  onClose: () => void;
}

function Check({ state, onChange, label, disabled }: { state: CheckState; onChange: () => void; label: string; disabled?: boolean }) {
  return (
    <input
      type="checkbox"
      aria-label={label}
      checked={state === "all"}
      disabled={disabled}
      ref={(el) => {
        if (el) el.indeterminate = state === "some";
      }}
      onChange={onChange}
      onClick={(e) => e.stopPropagation()}
    />
  );
}

const plural = (n: number, one: string, many: string) => `${n.toLocaleString("fr-FR")} ${n > 1 ? many : one}`;

export function IllustratorPreviewDialog({ preview, onImport, onBackground, onClose }: Props) {
  const [chosen, setChosen] = useState<Set<number>>(() => defaultChoice(preview));
  const [open, setOpen] = useState<Set<number>>(() => new Set());
  const [hover, setHover] = useState<Set<number> | null>(null);
  const groups = useMemo(() => layerGroups(preview), [preview]);
  const empty = useMemo(() => emptyLayers(preview), [preview]);
  const summary = choiceSummary(preview, chosen);
  const all = preview.images.map((_, i) => i);
  const { frame } = preview;
  // Le plan de travail, et les images qui en débordent.
  const box = useMemo(() => {
    let [x0, y0, x1, y1] = [frame.x, frame.y, frame.x + frame.width, frame.y + frame.height];
    for (const im of preview.images) {
      const r = Math.hypot(im.width, im.height) / 2;
      x0 = Math.min(x0, im.center.x - r);
      y0 = Math.min(y0, im.center.y - r);
      x1 = Math.max(x1, im.center.x + r);
      y1 = Math.max(y1, im.center.y + r);
    }
    const m = Math.max(x1 - x0, y1 - y0) * 0.02;
    return `${x0 - m} ${y0 - m} ${x1 - x0 + 2 * m} ${y1 - y0 + 2 * m}`;
  }, [preview, frame]);

  const toggleOpen = (layer: number) =>
    setOpen((current) => {
      const next = new Set(current);
      if (next.has(layer)) next.delete(layer);
      else next.add(layer);
      return next;
    });

  return (
    <Dialog
      title={`Importer « ${preview.name} »`}
      size="large"
      onClose={onClose}
      actions={
        <>
          <span className="compose-ai-summary">{choiceText(summary)}</span>
          <Button onClick={onBackground} title="Le .ai en une seule image de fond, sous les dessins, sans créer de dessins">
            Poser en fond seulement
          </Button>
          <Button onClick={onClose}>Annuler</Button>
          <Button variant="primary" icon="import" disabled={summary.images === 0} onClick={() => onImport([...chosen].sort((a, b) => a - b))}>
            Importer
          </Button>
        </>
      }
    >
      <div className="compose-ai">
        <figure className="compose-ai-map">
          <svg
            viewBox={box}
            preserveAspectRatio="xMidYMid meet"
            role="img"
            aria-label="Où les images seront posées sur la façade"
          >
            <rect className="compose-ai-board" x={frame.x} y={frame.y} width={frame.width} height={frame.height} />
            {preview.images.map((image, i) => {
              const layer = preview.layers[image.layer];
              const color = layerColor(layer.color) ?? undefined;
              const usable = importable(preview, i);
              const lit = hover?.has(i) ?? false;
              const state = !usable ? "is-missing" : chosen.has(i) ? "is-chosen" : "is-left";
              const link = preview.links[image.link];
              return (
                <rect
                  key={i}
                  className={`compose-ai-image ${state}${lit ? " is-lit" : ""}${image.visible ? "" : " is-hidden"}`}
                  x={image.center.x - image.width / 2}
                  y={image.center.y - image.height / 2}
                  width={image.width}
                  height={image.height}
                  transform={`rotate(${image.rotation} ${image.center.x} ${image.center.y})`}
                  style={{ stroke: usable ? color : undefined, fill: usable ? color : undefined, strokeWidth: lit ? 2.5 : 1 }}
                >
                  <title>{`${link.name} — ${layer.name} — ${STATUS_LABEL[link.status]}`}</title>
                </rect>
              );
            })}
          </svg>
          <figcaption className="ui-note">
            Le plan de travail d'Illustrator couvre toute la largeur de la façade. Pointillés rouges : lien introuvable ;
            trait clair : image laissée de côté.
          </figcaption>
        </figure>

        <div className="compose-ai-tree">
          <div className="compose-ai-row is-head">
            <Check
              state={checkState(preview, chosen, all)}
              label="Tout choisir"
              onChange={() => setChosen(toggleImages(preview, chosen, all))}
            />
            <span className="compose-ai-name">Calques d'Illustrator</span>
            <span className="compose-ai-count">{plural(preview.images.length, "image", "images")}</span>
          </div>
          {groups.map((g) => {
            const layer = preview.layers[g.layer];
            const expanded = open.has(g.layer);
            const color = layerColor(layer.color);
            return (
              <div key={g.layer} className="compose-ai-layer">
                <div
                  className="compose-ai-row is-layer"
                  onClick={() => toggleOpen(g.layer)}
                  onMouseEnter={() => setHover(new Set(g.images))}
                  onMouseLeave={() => setHover(null)}
                >
                  <Check
                    state={checkState(preview, chosen, g.images)}
                    label={`Importer le calque ${layer.name}`}
                    disabled={!g.images.some((i) => importable(preview, i))}
                    onChange={() => setChosen(toggleImages(preview, chosen, g.images))}
                  />
                  <Icon name={expanded ? "chevronDown" : "chevronRight"} />
                  <span className="compose-ai-swatch" style={color ? { background: color } : undefined} />
                  <span className="compose-ai-name">
                    {layer.name}
                    {!layer.visible && <Icon name="eyeOff" />}
                    {layer.locked && <Icon name="lock" />}
                  </span>
                  <span className="compose-ai-count">
                    {g.images.length}
                    {layer.embedded > 0 && ` + ${layer.embedded} incorporée${layer.embedded > 1 ? "s" : ""}`}
                  </span>
                </div>
                {layer.existing && expanded && (
                  <p className="ui-note compose-ai-indent">Calque déjà dans le projet : les dessins s'y ajouteront.</p>
                )}
                {expanded &&
                  g.rows.map((row) => {
                    const link = preview.links[row.link];
                    return (
                      <div
                        key={row.link}
                        className={`compose-ai-row is-file tone-${link.status}`}
                        title={`${STATUS_HINT[link.status]}${link.drawingName ? ` (${link.drawingName})` : ""}\n${link.link}`}
                        onMouseEnter={() => setHover(new Set(row.images))}
                        onMouseLeave={() => setHover(null)}
                      >
                        <Check
                          state={checkState(preview, chosen, row.images)}
                          label={`Importer ${link.name}`}
                          disabled={link.status === "missing"}
                          onChange={() => setChosen(toggleImages(preview, chosen, row.images))}
                        />
                        <span className="compose-ai-name">{link.name}</span>
                        <span className={`compose-ai-status is-${link.status}`}>{STATUS_LABEL[link.status]}</span>
                        <span className="compose-ai-count">×{row.images.length}</span>
                      </div>
                    );
                  })}
              </div>
            );
          })}
          {empty.length > 0 && (
            <p className="ui-note">Sans image liée (tracés seulement, non repris) : {empty.join(", ")}.</p>
          )}
          {summary.missing > 0 && (
            <p className="ui-note tone-warning">
              {plural(summary.missing, "image a son lien introuvable", "images ont leur lien introuvable")} : mettre les
              dossiers des dessins à côté du .ai, ou les importer d'abord dans la bibliothèque.
            </p>
          )}
          {preview.embedded > 0 && (
            <p className="ui-note">
              {plural(preview.embedded, "image incorporée", "images incorporées")} non reprise
              {preview.embedded > 1 ? "s" : ""} : hors de tout calque (réserve de motifs d'Illustrator) ou format inconnu.
            </p>
          )}
        </div>
      </div>
    </Dialog>
  );
}
