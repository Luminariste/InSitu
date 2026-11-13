// Fenêtre « Exporter cette diapositive… » de l'écran Diapositive : les
// mêmes réglages que l'onglet Export de la Façade, dont la fresque gravée
// ou non (demande de Benjamin, 13 novembre 2026, D-3.5-o).

import { useState } from "react";
import type { ExportSettings } from "../engine";
import { Button, Dialog } from "../ui";
import { ExportOptions } from "../facade/ExportOptions";
import { parseDpi, validDpi } from "../facade/exportLogic";

interface Props {
  /** Nom de la diapositive (« A »). */
  label: string;
  /** Les réglages de départ : ceux du projet, la fresque comme à l'écran. */
  initial: ExportSettings;
  hasFresco: boolean;
  onCancel: () => void;
  /** Lance l'export avec les réglages choisis. */
  onExport: (settings: ExportSettings) => void;
}

export function SlideExportDialog({ label, initial, hasFresco, onCancel, onExport }: Props) {
  const [settings, setSettings] = useState(initial);
  const [dpiText, setDpiText] = useState(String(initial.dpi));
  const dpi = parseDpi(dpiText);
  const ok = validDpi(dpi);
  return (
    <Dialog
      title={`Exporter la diapositive ${label}`}
      onClose={onCancel}
      actions={
        <>
          <Button onClick={onCancel}>Annuler</Button>
          <Button variant="primary" icon="export" disabled={!ok} onClick={() => onExport({ ...settings, dpi })}>
            Choisir le dossier et exporter…
          </Button>
        </>
      }
    >
      <ExportOptions settings={settings} onChange={setSettings} dpiText={dpiText} onDpiText={setDpiText} hasFresco={hasFresco} />
      <p className="ui-note">Ces réglages deviennent ceux du projet, comme dans l'onglet Export de la Façade.</p>
    </Dialog>
  );
}
