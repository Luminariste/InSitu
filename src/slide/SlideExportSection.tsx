// Onglet Export de la vue Diapositive (demande de Benjamin, 13 novembre
// 2026, D-3.6-f) : les mêmes réglages que l'onglet Export de la Façade,
// dont la fresque gravée ou non (D-3.5-o), pour cette diapositive seulement,
// et le plan de contrôle (D-3.5-m). Les réglages deviennent ceux du projet.

import type { ExportSettings, Project } from "../engine";
import { Button, Section } from "../ui";
import { ExportOptions } from "../facade/ExportOptions";
import { exportSettingsOf, parseDpi, validDpi } from "../facade/exportLogic";

interface Props {
  /** Nom de la diapositive (« A ») ; `null` : aucune. */
  label: string | null;
  project: Project | null;
  /** Pourquoi on ne peut pas exporter ; `null` si on peut. */
  blocked: string | null;
  /** Le plan de contrôle est possible (projecteur recalé). */
  canControlPlan: boolean;
  busy: boolean;
  settings: ExportSettings;
  onSettings: (settings: ExportSettings) => void;
  dpiText: string;
  onDpiText: (text: string) => void;
  onExport: (settings: ExportSettings) => void;
  onControlPlan: () => void;
}

/** Réglages de départ : ceux du projet, la fresque comme à l'écran. */
export function startSettings(project: Project | null, frescoShown: boolean): ExportSettings {
  return { ...exportSettingsOf(project?.export), fresco: project?.fresco ? frescoShown : true };
}

export function SlideExportSection(props: Props) {
  const { label, project, blocked, canControlPlan, busy, settings, onSettings, dpiText, onDpiText, onExport, onControlPlan } = props;
  const dpi = parseDpi(dpiText);
  const ok = validDpi(dpi);
  return (
    <Section title={label ? `Diapositive ${label}` : "Export"}>
      <ExportOptions
        settings={settings}
        onChange={onSettings}
        dpiText={dpiText}
        onDpiText={onDpiText}
        hasFresco={Boolean(project?.fresco)}
        disabled={busy}
      />
      {blocked && <p className="ui-note">{blocked}</p>}
      <div className="slide-export-actions">
        <Button
          variant="primary"
          icon="export"
          shortcut="mod+e"
          disabled={blocked !== null || !ok || busy}
          title="Le PNG de cette diapositive seulement, avec ces réglages"
          onClick={() => onExport({ ...settings, dpi })}
        >
          Exporter cette diapositive…
        </Button>
        <Button
          icon="export"
          disabled={!canControlPlan || busy}
          title="Le plan de la façade vu par ce projecteur, à graver pour vérifier le recalage : projeté, il doit tomber sur la façade"
          onClick={onControlPlan}
        >
          Plan de contrôle…
        </Button>
      </div>
      <p className="ui-note">Ces réglages deviennent ceux du projet, comme dans l'onglet Export de la Façade.</p>
    </Section>
  );
}
