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
import { ExportOptions } from "./ExportOptions";
import { isIllustratorFile } from "../compose/IllustratorImport";
import {
  ALL_SLIDES,
  exportButtonLabel,
  controlPlanButtonLabel,
  controlPlanSummary,
  exportSettingsOf,
  exportTargets,
  keptTarget,
  exportSummary,
  fileName,
  sheetFileName,
  parseDpi,
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
  /**
   * Un .ai : ses images deviennent des dessins dans les calques, après une
   * fenêtre d'aperçu (demande de Benjamin, 13 novembre 2026). Rend ce
   * qu'il faut montrer sous le bouton.
   */
  illustrator: { start: (path: string) => Promise<void>; busy: boolean; node: ReactNode };
}

export function useExportPanel({ engine, folder, project, onProject, showFresco, onShowFresco, illustrator }: Props): {
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
  const artwork = (fresco !== null && settings.fresco) || (project?.placements?.length ?? 0) > 0;
  const ready = engine && folder !== null && busy === null;
  // Toutes les diapositives, ou celle d'un seul projecteur recalé.
  const targets = exportTargets(project?.projectors ?? []);
  const [targetChoice, setTargetChoice] = useState<string>(ALL_SLIDES);
  const target = keptTarget(targetChoice, targets);

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
      if (isIllustratorFile(path)) {
        await illustrator.start(path);
        return;
      }
      onProject(await importFresco(folder, path));
      onShowFresco(true);
    }, "fresco");

  const exportAll = () =>
    run("Export des diapositives…", async () => {
      if (!folder) return;
      const dpi = parseDpi(dpiText);
      if (!validDpi(dpi)) throw new Error("la résolution doit être comprise entre 50 et 3 000 dpi");
      const chosen = { ...settings, dpi };
      const destination = await chooseExportFolder();
      if (!destination) return;
      const report = await exportSlides(folder, destination, chosen, setProgress, target === ALL_SLIDES ? undefined : [target]);
      setSettings(chosen);
      setMessage(exportSummary(report, destination));
    });

  // Plan de contrôle (D-3.5-m) : le plan vu par chaque projecteur, à graver
  // pour vérifier le recalage ; mêmes réglages, ceux du projet ne changent pas.
  const exportControlPlans = () =>
    run("Export des plans de contrôle…", async () => {
      if (!folder) return;
      const dpi = parseDpi(dpiText);
      if (!validDpi(dpi)) throw new Error("la résolution doit être comprise entre 50 et 3 000 dpi");
      const destination = await chooseExportFolder();
      if (!destination) return;
      const only = target === ALL_SLIDES ? undefined : [target];
      const report = await exportSlides(folder, destination, { ...settings, dpi }, setProgress, only, true);
      setMessage(controlPlanSummary(report, destination));
    });

  // Fiche de calepinage (tâche 6.2), avec les réglages tels qu'ils sont à l'écran.
  const writeSheet = () =>
    run("Fiche de calepinage…", async () => {
      if (!folder) return;
      const dpi = parseDpi(dpiText);
      if (!validDpi(dpi)) throw new Error("la résolution doit être comprise entre 50 et 3 000 dpi");
      const path = await chooseSheetPath(sheetFileName(project?.name));
      if (!path) return;
      const report = await writeLayoutSheet(folder, path, { ...settings, dpi });
      setMessage(sheetSummary(report, path));
    });

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
        <p>
          {fresco
            ? fileName(fresco.vector?.source ?? fresco.source)
            : engine
              ? "Aucune fresque importée. Un .ai : ses images deviennent des dessins dans les calques. Une image, un PDF ou un SVG : un fond de référence."
              : "Dans l'app seulement."}
        </p>
        <div className="facade-export-row">
          <Button icon="import" onClick={() => void importOne()} disabled={!ready || !scaled || illustrator.busy}>
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
        {illustrator.node}
      </Section>
    ),
    export: (
      <Section title="Export des diapositives">
        <ExportOptions
          settings={settings}
          onChange={setSettings}
          dpiText={dpiText}
          onDpiText={setDpiText}
          disabled={busy !== null}
          hasFresco={fresco !== null}
        />
        <Property label="Diapositives" hint={target === ALL_SLIDES ? `${targets.length} projecteur${targets.length > 1 ? "s" : ""} recalé${targets.length > 1 ? "s" : ""}` : "Un seul PNG"}>
          <select
            className="ui-select"
            value={target}
            onChange={(e) => setTargetChoice(e.target.value)}
            disabled={busy !== null || targets.length === 0}
            aria-label="Diapositives à exporter"
          >
            <option value={ALL_SLIDES}>Toutes</option>
            {targets.map((t) => (
              <option key={t.id} value={t.id}>
                {t.label} seulement
              </option>
            ))}
          </select>
        </Property>
        <div className="facade-export-go">
          <Button
            variant="primary"
            icon="export"
            shortcut="mod+e"
            title={target === ALL_SLIDES ? "Un PNG par projecteur recalé : la fresque et les dessins posés" : "Le PNG de ce projecteur seulement"}
            onClick={() => void exportAll()}
            disabled={!ready || !artwork}
          >
            {exportButtonLabel(target, targets)}
          </Button>
          <Button
            icon="export"
            title="Le plan de la façade vu par le projecteur, passé par la même déformation que les dessins : gravé et projeté, il doit tomber sur la façade. Vérifie le recalage avant de graver les dessins."
            onClick={() => void exportControlPlans()}
            disabled={!ready || !scaled || targets.length === 0}
          >
            {controlPlanButtonLabel(target, targets)}
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
