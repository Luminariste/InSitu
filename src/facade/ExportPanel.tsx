// Fresque et export des diapositives (tâche 3.5), dans le panneau de droite
// de l'écran Façade : importer la fresque en une image, régler l'export,
// écrire un PNG par projecteur recalé.

import { useEffect, useState } from "react";
import {
  chooseExportFolder,
  chooseFrescoFile,
  exportSlides,
  importFresco,
  type ExportProgress,
  type ExportSettings,
  type Project,
} from "../engine";
import { exportSettingsOf, exportSizeLabel, exportSummary, fileName, validDpi } from "./exportLogic";

interface Props {
  engine: boolean;
  folder: string | null;
  project: Project | null;
  onProject: (project: Project) => void;
  showFresco: boolean;
  onShowFresco: (show: boolean) => void;
}

export function ExportPanel({ engine, folder, project, onProject, showFresco, onShowFresco }: Props) {
  const [settings, setSettings] = useState<ExportSettings>(() => exportSettingsOf(project?.export));
  const [dpiText, setDpiText] = useState(String(settings.dpi));
  const [busy, setBusy] = useState<string | null>(null);
  const [progress, setProgress] = useState<ExportProgress | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Un autre projet : ses réglages.
  useEffect(() => {
    const next = exportSettingsOf(project?.export);
    setSettings(next);
    setDpiText(String(next.dpi));
    setMessage(null);
    setError(null);
  }, [project?.id]);

  const fresco = project?.fresco ?? null;
  const scaled = Boolean(project?.facade?.scale);
  const ready = engine && folder !== null && busy === null;
  const set = (patch: Partial<ExportSettings>) => setSettings({ ...settings, ...patch });

  const run = async (label: string, task: () => Promise<void>) => {
    setBusy(label);
    setError(null);
    setMessage(null);
    try {
      await task();
    } catch (failure) {
      setError(String(failure).replace(/^Error: /, ""));
    } finally {
      setBusy(null);
      setProgress(null);
    }
  };

  const importOne = () =>
    run("Import de la fresque…", async () => {
      if (!folder) return;
      const path = await chooseFrescoFile();
      if (!path) return;
      onProject(await importFresco(folder, path));
      onShowFresco(true);
    });

  const exportAll = () =>
    run("Export des diapositives…", async () => {
      if (!folder) return;
      const dpi = Number(dpiText.replace(",", "."));
      if (!validDpi(dpi)) throw new Error("la résolution doit être comprise entre 50 et 3 000 dpi");
      const chosen = { ...settings, dpi };
      const destination = await chooseExportFolder();
      if (!destination) return;
      const report = await exportSlides(folder, destination, chosen, setProgress);
      setSettings(chosen);
      setMessage(exportSummary(report, destination));
    });

  return (
    <section className="facade-export" aria-label="Fresque et export">
      <h2>Fresque</h2>
      <p className="facade-overview-note">
        {fresco ? fileName(fresco.source) : engine ? "Aucune fresque importée." : "Dans l'app seulement."}
      </p>
      <div className="facade-export-row">
        <button type="button" onClick={() => void importOne()} disabled={!ready || !scaled}>
          {fresco ? "Remplacer…" : "Importer la fresque…"}
        </button>
        {fresco && (
          <label>
            <input type="checkbox" checked={showFresco} onChange={(e) => onShowFresco(e.target.checked)} />
            Montrer
          </label>
        )}
      </div>
      {engine && folder && !scaled && (
        <p className="facade-overview-note">Mettre d'abord le plan à l'échelle : la fresque couvre toute sa largeur.</p>
      )}

      <h3>Export des diapositives</h3>
      <fieldset className="facade-overview-show" disabled={busy !== null}>
        <label className="facade-export-dpi">
          Résolution
          <input
            type="text"
            inputMode="decimal"
            value={dpiText}
            onChange={(e) => setDpiText(e.target.value)}
            aria-label="Résolution en dpi"
          />
          dpi
        </label>
        <label>
          <input type="checkbox" checked={settings.margin} onChange={(e) => set({ margin: e.target.checked })} />
          Plaque de 300 mm avec nom et repères (sinon 280 mm)
        </label>
        <label>
          <input type="checkbox" checked={settings.mirror} onChange={(e) => set({ mirror: e.target.checked })} />
          Miroir
        </label>
        <label>
          <input type="checkbox" checked={settings.negative} onChange={(e) => set({ negative: e.target.checked })} />
          Négatif
        </label>
        <label>
          <input type="checkbox" checked={settings.oneBit} onChange={(e) => set({ oneBit: e.target.checked })} />
          Noir et blanc pur (1 bit)
        </label>
        {settings.oneBit && (
          <label className="facade-overview-opacity">
            Seuil : {settings.threshold}
            <input
              type="range"
              min={1}
              max={255}
              value={settings.threshold}
              onChange={(e) => set({ threshold: Number(e.target.value) })}
            />
          </label>
        )}
        <span className="facade-overview-note">
          {validDpi(Number(dpiText.replace(",", ".")))
            ? exportSizeLabel({ ...settings, dpi: Number(dpiText.replace(",", ".")) })
            : "Résolution : de 50 à 3 000 dpi"}
        </span>
      </fieldset>
      <button type="button" className="facade-export-go" onClick={() => void exportAll()} disabled={!ready || !fresco}>
        Exporter les diapositives…
      </button>
      {progress && progress.total > 0 && (
        <p className="facade-overview-note">
          {progress.current
            ? `Diapositive ${progress.current} (${progress.done + 1} sur ${progress.total})…`
            : "Fin de l'écriture…"}
        </p>
      )}
      {busy && !progress && <p className="facade-overview-note">{busy}</p>}
      {message && <p className="facade-export-ok">{message}</p>}
      {error && <p className="facade-overview-problem">{error}</p>}
    </section>
  );
}
