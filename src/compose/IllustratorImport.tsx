// Import d'une composition Illustrator (tâche 6.3), lancé par le bouton
// « Importer la fresque… » de l'onglet Plan quand le fichier choisi est un
// .ai (demande de Benjamin, 13 novembre 2026 : un seul bouton d'import). Le
// .ai est d'abord lu sans rien changer : une fenêtre montre le résultat,
// calque par calque, dit quels dessins sont déjà dans la bibliothèque et
// lesquels sont incorporés dans le .ai ; on y choisit ce qu'on importe.
// Chaque image choisie devient un dessin posé, dans un calque du nom de
// celui d'Illustrator. « Poser en fond seulement » garde l'ancien import de
// la fresque en un bloc (utile pour un .ai fait de tracés, sans images).

import { useState, type ReactNode } from "react";
import {
  importFresco,
  importIllustrator,
  previewIllustrator,
  type CompositionPreview,
  type ImportProgress,
  type Project,
} from "../engine";
import { compositionSummary, compositionWarnings } from "./illustratorLogic";
import { IllustratorPreviewDialog } from "./IllustratorPreviewDialog";

interface Options {
  folder: string | null;
  onProject: (project: Project) => void;
  /** La fresque vient d'être posée en fond : la montrer. */
  onFresco: () => void;
}

const message = (failure: unknown) => String(failure).replace(/^Error: /, "");

/** Le fichier est un .ai : il passe par la fenêtre d'aperçu. */
export const isIllustratorFile = (path: string) => /\.ai$/i.test(path);

export function useIllustratorImport({ folder, onProject, onFresco }: Options): {
  /** Lit le .ai et ouvre la fenêtre d'aperçu. */
  start: (path: string) => Promise<void>;
  busy: boolean;
  /** Avancement, compte rendu et fenêtre, à poser sous le bouton. */
  node: ReactNode;
} {
  const [busy, setBusy] = useState<string | null>(null);
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const [pending, setPending] = useState<{ path: string; preview: CompositionPreview } | null>(null);
  const [summary, setSummary] = useState<string | null>(null);
  const [warnings, setWarnings] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);

  const run = async (label: string, task: () => Promise<void>) => {
    setBusy(label);
    setError(null);
    try {
      await task();
    } catch (failure) {
      setError(message(failure));
    } finally {
      setBusy(null);
      setProgress(null);
    }
  };

  const start = (path: string) =>
    run("Lecture du .ai…", async () => {
      if (!folder) return;
      setSummary(null);
      setWarnings([]);
      setPending({ path, preview: await previewIllustrator(folder, path) });
    });

  const importChosen = (path: string, images: number[]) => {
    setPending(null);
    return run("Import en cours", async () => {
      if (!folder) return;
      const { project, report } = await importIllustrator(folder, path, images, setProgress);
      onProject(project);
      setSummary(compositionSummary(report));
      setWarnings(compositionWarnings(report));
    });
  };

  const importAsBackground = (path: string) => {
    setPending(null);
    return run("Import de la fresque en fond…", async () => {
      if (!folder) return;
      onProject(await importFresco(folder, path));
      onFresco();
      setSummary("Le .ai est posé en fond de référence, sur toute la largeur de la façade.");
    });
  };

  const node = (
    <>
      {busy && (
        <p className="ui-note">
          {busy}
          {progress && progress.total > 0 ? ` : ${progress.done} / ${progress.total} fichiers` : ""}
        </p>
      )}
      {summary && <p className="ui-note tone-success">{summary}</p>}
      {warnings.map((line) => (
        <p key={line} className="ui-note tone-warning">
          {line}
        </p>
      ))}
      {error && <p className="ui-note tone-danger">{error}</p>}
      {pending && (
        <IllustratorPreviewDialog
          preview={pending.preview}
          onClose={() => setPending(null)}
          onImport={(images) => void importChosen(pending.path, images)}
          onBackground={() => void importAsBackground(pending.path)}
        />
      )}
    </>
  );
  return { start, busy: busy !== null, node };
}
