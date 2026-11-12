// Fresque et export des diapositives (tâche 3.5), dans l'inspecteur (à droite)
// de l'écran Façade : importer la fresque en une image, régler l'export,
// écrire un PNG par projecteur recalé. Les deux rubriques vont dans deux
// onglets (Fresque, Export, D15-i) mais partagent leur état : le crochet
// `useExportPanel` les rend toutes deux, et reste actif (⌘E) quel que soit
// l'onglet affiché.

import { useEffect, useState, type ReactNode } from "react";
import {
  chooseExportFolder,
  chooseFrescoFile,
  chooseSheetPath,
  exportSlides,
  importFresco,
  writeLayoutSheet,
  type ExportProgress,
  type ExportSettings,
  type Project,
} from "../engine";
import { Button, Progress, Property, Section, useShortcuts } from "../ui";
import {
  exportSettingsOf,
  exportSizeLabel,
  exportSummary,
  fileName,
  sheetFileName,
  sheetSummary,
  validDpi,
} from "./exportLogic";

interface Props {
  engine: boolean;
  folder: string | null;
  project: Project | null;
  onProject: (project: Project) => void;
  showFresco: boolean;
  onShowFresco: (show: boolean) => void;
}

export function useExportPanel({ engine, folder, project, onProject, showFresco, onShowFresco }: Props): {
  fresco: ReactNode;
  export: ReactNode;
} {
  const [settings, setSettings] = useState<ExportSettings>(() => exportSettingsOf(project?.export));
  const [dpiText, setDpiText] = useState(String(settings.dpi));
  const [busy, setBusy] = useState<string | null>(null);
  const [progress, setProgress] = useState<ExportProgress | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  /** Rubrique où montrer l'avancement et le compte rendu. */
  const [origin, setOrigin] = useState<"fresco" | "export">("export");

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
  // De quoi graver : la fresque, ou des dessins posés (le moteur écarte les calques masqués).
  const artwork = fresco !== null || (project?.placements?.length ?? 0) > 0;
  const ready = engine && folder !== null && busy === null;
  const set = (patch: Partial<ExportSettings>) => setSettings({ ...settings, ...patch });

  const run = async (label: string, task: () => Promise<void>, from: "fresco" | "export" = "export") => {
    setOrigin(from);
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
    }, "fresco");

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

  // Fiche de calepinage (tâche 6.2), avec les réglages tels qu'ils sont à l'écran.
  const writeSheet = () =>
    run("Fiche de calepinage…", async () => {
      if (!folder) return;
      const dpi = Number(dpiText.replace(",", "."));
      if (!validDpi(dpi)) throw new Error("la résolution doit être comprise entre 50 et 3 000 dpi");
      const path = await chooseSheetPath(sheetFileName(project?.name));
      if (!path) return;
      const report = await writeLayoutSheet(folder, path, { ...settings, dpi });
      setMessage(sheetSummary(report, path));
    });

  const dpiValue = Number(dpiText.replace(",", "."));
  useShortcuts([{ keys: "mod+e", run: () => void exportAll(), enabled: ready && artwork }]);

  const notes = (from: "fresco" | "export") =>
    origin === from && (
      <>
        {busy && !progress && <p className="ui-note">{busy}</p>}
        {message && <p className="ui-note tone-success">{message}</p>}
        {error && <p className="ui-note tone-danger">{error}</p>}
      </>
    );

  return {
    fresco: (
      <Section title="Fresque">
        <p>{fresco ? fileName(fresco.source) : engine ? "Aucune fresque importée." : "Dans l'app seulement."}</p>
        <div className="facade-export-row">
          <Button icon="import" onClick={() => void importOne()} disabled={!ready || !scaled}>
            {fresco ? "Remplacer…" : "Importer la fresque…"}
          </Button>
          {fresco && (
            <label className="ui-check">
              <input type="checkbox" checked={showFresco} onChange={(e) => onShowFresco(e.target.checked)} />
              Montrer
            </label>
          )}
        </div>
        {engine && folder && !scaled && (
          <p className="ui-note tone-warning">Mettre d'abord le plan à l'échelle : la fresque couvre toute sa largeur.</p>
        )}
        {notes("fresco")}
      </Section>
    ),
    export: (
      <Section title="Export des diapositives">
        <fieldset className="facade-export-options" disabled={busy !== null}>
          <Property label="Résolution" hint={validDpi(dpiValue) ? exportSizeLabel({ ...settings, dpi: dpiValue }) : "De 50 à 3 000 dpi"}>
            <input
              className="ui-input facade-export-dpi"
              type="text"
              inputMode="decimal"
              value={dpiText}
              onChange={(e) => setDpiText(e.target.value)}
              aria-label="Résolution en dpi"
            />
            dpi
          </Property>
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
          <label className="ui-check" title="Les courbes des dessins posés, en millimètres, pour une gravure vectorielle (la fresque importée en une image n'y est pas)">
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
        <div className="facade-export-go">
          <Button variant="primary" icon="export" shortcut="mod+e" title="Un PNG par projecteur recalé : la fresque et les dessins posés" onClick={() => void exportAll()} disabled={!ready || !artwork}>
            Exporter les diapositives…
          </Button>
          <Button
            icon="document"
            title="PDF A3 pour le site : plan avec les emprises et les dessins, liste des diapositives, réglages d'export, alertes, vignettes"
            onClick={() => void writeSheet()}
            disabled={!ready || project?.projectors.length === 0}
          >
            Fiche de calepinage…
          </Button>
        </div>
        {engine && folder && !artwork && (
          <p className="ui-note tone-warning">
            Rien à exporter pour l'instant : poser des dessins sur la façade (rubrique Dessins), ou importer une fresque
            ci-dessus.
          </p>
        )}
        {progress && progress.total > 0 && (
          <Progress
            done={progress.done}
            total={progress.total}
            label={progress.current ? `Diapositive ${progress.current} (${progress.done + 1} sur ${progress.total})…` : "Fin de l'écriture…"}
          />
        )}
        {notes("export")}
      </Section>
    ),
  };
}
