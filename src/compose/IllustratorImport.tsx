// Rubrique « Composition Illustrator » de l'onglet Calques de l'écran Façade
// (tâche 6.3) : recrée sur la façade une composition faite dans Illustrator.
// Le .ai est d'abord lu sans rien changer : une fenêtre montre le résultat,
// calque par calque, et dit quels dessins sont déjà dans la bibliothèque ;
// on y choisit ce qu'on importe (demande de Benjamin, 13 novembre 2026).
// Chaque image choisie devient un dessin posé, dans un calque du nom de
// celui d'Illustrator.

import { useState } from "react";
import { Button, Section } from "../ui";
import {
  chooseIllustratorFile,
  importIllustrator,
  previewIllustrator,
  type CompositionPreview,
  type ImportProgress,
  type Project,
} from "../engine";
import { compositionSummary, compositionWarnings } from "./illustratorLogic";
import { IllustratorPreviewDialog } from "./IllustratorPreviewDialog";

interface Props {
  engine: boolean;
  folder: string | null;
  /** La façade est à l'échelle : le plan de travail en couvre toute la largeur. */
  scaled: boolean;
  onProject: (project: Project) => void;
}

const message = (failure: unknown) => String(failure).replace(/^Error: /, "");

export function IllustratorImport({ engine, folder, scaled, onProject }: Props) {
  const [busy, setBusy] = useState<string | null>(null);
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const [pending, setPending] = useState<{ path: string; preview: CompositionPreview } | null>(null);
  const [summary, setSummary] = useState<string | null>(null);
  const [warnings, setWarnings] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);

  const choose = async () => {
    if (!folder) return;
    setError(null);
    try {
      const path = await chooseIllustratorFile();
      if (!path) return;
      setBusy("Lecture du .ai…");
      setSummary(null);
      setWarnings([]);
      setPending({ path, preview: await previewIllustrator(folder, path) });
    } catch (failure) {
      setError(message(failure));
    } finally {
      setBusy(null);
    }
  };

  const run = async (path: string, images: number[]) => {
    if (!folder) return;
    setPending(null);
    setBusy("Import en cours");
    try {
      const { project, report } = await importIllustrator(folder, path, images, setProgress);
      onProject(project);
      setSummary(compositionSummary(report));
      setWarnings(compositionWarnings(report));
    } catch (failure) {
      setError(message(failure));
    } finally {
      setBusy(null);
      setProgress(null);
    }
  };

  return (
    <Section title="Composition Illustrator">
      <p className="ui-note">
        Recrée une composition faite dans Illustrator sur le plan : chaque image liée devient un dessin posé, dans un
        calque du même nom. Une fenêtre montre le résultat avant l'import et laisse choisir les calques et les dessins.
      </p>
      <Button icon="import" onClick={() => void choose()} disabled={!engine || !folder || !scaled || busy !== null}>
        Importer un fichier .ai…
      </Button>
      {engine && folder && !scaled && (
        <p className="ui-note tone-warning">Mettre d'abord le plan à l'échelle (onglet Plan).</p>
      )}
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
          onImport={(images) => void run(pending.path, images)}
        />
      )}
    </Section>
  );
}
