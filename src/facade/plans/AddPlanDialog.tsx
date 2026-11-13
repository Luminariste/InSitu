// Choix des calques (ou des pages) d'un fichier à ajouter comme plans
// (D-PL-c) : chaque calque coché devient une vue du groupe, qu'on montre ou
// cache à part ; toutes les vues restent calées entre elles.

import { useState } from "react";
import { Button, Dialog } from "../../ui";
import type { PlanChoice } from "../../engine";
import { choicesFor } from "./planGroups";
import type { PendingPlanFile } from "./usePlanGroups";

interface Props {
  pending: PendingPlanFile;
  onConfirm: (choices: PlanChoice[]) => void;
  onCancel: () => void;
}

export function AddPlanDialog({ pending, onConfirm, onCancel }: Props) {
  const { info } = pending;
  const layers = info.layers ?? [];
  const pageCount = info.kind === "pdf" ? (info.pageCount ?? 1) : 1;
  const [checkedLayers, setCheckedLayers] = useState<string[]>([]);
  const [pages, setPages] = useState<number[]>(layers.length > 0 || pageCount === 1 ? [1] : []);
  const name = pending.path.split(/[\\/]/).pop() ?? pending.path;
  const choices = choicesFor(info, pages, checkedLayers);
  const toggle = <T,>(list: T[], value: T) => (list.includes(value) ? list.filter((v) => v !== value) : [...list, value]);

  return (
    <Dialog
      title={pending.group ? "Quelles vues ajouter à ce plan ?" : "Quels plans tirer de ce fichier ?"}
      onClose={onCancel}
      actions={
        <>
          <span className="facade-plan-dialog-summary">
            {choices.length === 0 ? "Rien de coché" : `${choices.length} vue${choices.length > 1 ? "s" : ""}, calée${choices.length > 1 ? "s" : ""} entre elles`}
          </span>
          <Button onClick={onCancel}>Annuler</Button>
          <Button variant="primary" disabled={choices.length === 0} onClick={() => onConfirm(choices)}>
            Ajouter
          </Button>
        </>
      }
    >
      <p>
        <strong>{name}</strong>
        {layers.length > 0
          ? ` a ${layers.length} calques. Chaque calque coché devient un plan qu'on montre ou cache à part ; ils bougent ensemble.`
          : ` a ${pageCount} pages. Chaque page cochée devient un plan ; elles bougent ensemble.`}
      </p>
      {layers.length > 0 && pageCount > 1 && (
        <label className="facade-plan-dialog-page">
          Page{" "}
          <select className="ui-select" value={pages[0] ?? 1} onChange={(e) => setPages([Number(e.target.value)])} aria-label="Page">
            {Array.from({ length: pageCount }, (_, i) => (
              <option key={i + 1} value={i + 1}>
                {i + 1}
              </option>
            ))}
          </select>
        </label>
      )}
      <div className="facade-plan-dialog-list">
        {layers.length > 0
          ? layers.map((layer, i) => (
              <label key={`${layer}-${i}`} className="facade-plan-dialog-item">
                <input type="checkbox" checked={checkedLayers.includes(layer)} onChange={() => setCheckedLayers(toggle(checkedLayers, layer))} />
                {layer}
              </label>
            ))
          : Array.from({ length: pageCount }, (_, i) => (
              <label key={i + 1} className="facade-plan-dialog-item">
                <input type="checkbox" checked={pages.includes(i + 1)} onChange={() => setPages(toggle(pages, i + 1).sort((a, b) => a - b))} />
                Page {i + 1}
              </label>
            ))}
      </div>
      {layers.length > 0 && (
        <p className="ui-property-hint">Ce qui n'est sur aucun calque du fichier reste visible dans chaque vue.</p>
      )}
    </Dialog>
  );
}
