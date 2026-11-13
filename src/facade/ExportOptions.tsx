// Réglages d'export des diapositives, les mêmes partout : onglet Export de
// la Façade et fenêtre « Exporter cette diapositive… » de l'écran
// Diapositive (D-3.5-n).

import type { ExportSettings } from "../engine";
import { Property } from "../ui";
import { exportSizeLabel, parseDpi, validDpi } from "./exportLogic";

interface Props {
  settings: ExportSettings;
  onChange: (settings: ExportSettings) => void;
  /** La résolution telle qu'elle est tapée (« 1400 », « 1 400,5 »…). */
  dpiText: string;
  onDpiText: (text: string) => void;
  disabled?: boolean;
  /** Le projet a une fresque importée en une image : on peut la graver ou non. */
  hasFresco: boolean;
}

export function ExportOptions({ settings, onChange, dpiText, onDpiText, disabled, hasFresco }: Props) {
  const set = (patch: Partial<ExportSettings>) => onChange({ ...settings, ...patch });
  const dpi = parseDpi(dpiText);
  return (
    <fieldset className="facade-export-options" disabled={disabled}>
      <Property label="Résolution" hint={validDpi(dpi) ? exportSizeLabel({ ...settings, dpi }) : "De 50 à 3 000 dpi"}>
        <input
          className="ui-input facade-export-dpi"
          type="text"
          inputMode="decimal"
          value={dpiText}
          onChange={(e) => onDpiText(e.target.value)}
          aria-label="Résolution en dpi"
        />
        dpi
      </Property>
      {hasFresco && (
        <label
          className="ui-check"
          title="La fresque importée en une image est gravée avec les dessins posés ; décochée, les dessins seuls"
        >
          <input type="checkbox" checked={settings.fresco} onChange={(e) => set({ fresco: e.target.checked })} />
          Graver la fresque importée (sinon les dessins seuls)
        </label>
      )}
      <label className="ui-check">
        <input type="checkbox" checked={settings.margin} onChange={(e) => set({ margin: e.target.checked })} />
        Plaque de 300 mm avec nom et repères (sinon 280 mm)
      </label>
      <label className="ui-check">
        <input type="checkbox" checked={settings.mirror} onChange={(e) => set({ mirror: e.target.checked })} />
        Miroir
      </label>
      <label className="ui-check">
        <input type="checkbox" checked={settings.negative} onChange={(e) => set({ negative: e.target.checked })} />
        Négatif
      </label>
      <label className="ui-check">
        <input type="checkbox" checked={settings.oneBit} onChange={(e) => set({ oneBit: e.target.checked })} />
        Noir et blanc pur (1 bit)
      </label>
      <label
        className="ui-check"
        title="Les courbes des dessins posés, en millimètres, pour une gravure vectorielle (la fresque importée en une image n'y est pas)"
      >
        <input type="checkbox" checked={settings.svg} onChange={(e) => set({ svg: e.target.checked })} />
        SVG en plus (courbes des dessins posés)
      </label>
      {settings.oneBit && (
        <label className="facade-overview-opacity">
          Seuil : {settings.threshold}
          <input type="range" min={1} max={255} value={settings.threshold} onChange={(e) => set({ threshold: Number(e.target.value) })} />
        </label>
      )}
    </fieldset>
  );
}
