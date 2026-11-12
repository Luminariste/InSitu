// Rubrique « Composition Illustrator » de l'onglet Calques de l'écran Façade
// (tâche 6.3) : recrée sur la façade une composition faite dans Illustrator.
// Chaque fichier lié du .ai entre dans la bibliothèque, chaque image placée
// devient un dessin posé, dans un calque du nom de celui d'Illustrator.

import { useState } from "react";
import { Button, Section } from "../ui";
import { chooseIllustratorFile, importIllustrator, type ImportProgress, type Project } from "../engine";
import { compositionSummary, compositionWarnings } from "./illustratorLogic";

interface Props {
  engine: boolean;
  folder: string | null;
  /** La façade est à l'échelle : le plan de travail en couvre toute la largeur. */
  scaled: boolean;
  onProject: (project: Project) => void;
}

export function IllustratorImport({ engine, folder, scaled, onProject }: Props) {
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const [summary, setSummary] = useState<string | null>(null);
  const [warnings, setWarnings] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);

  const run = async () => {
    if (!folder) return;
    setError(null);
    try {
      const path = await chooseIllustratorFile();
      if (!path) return;
      setBusy(true);
      setSummary(null);
      setWarnings([]);
      const { project, report } = await importIllustrator(folder, path, setProgress);
      onProject(project);
      setSummary(compositionSummary(report));
      setWarnings(compositionWarnings(report));
    } catch (failure) {
      setError(String(failure).replace(/^Error: /, ""));
    } finally {
      setBusy(false);
      setProgress(null);
    }
  };

  return (
    <Section title="Composition Illustrator">
      <p className="ui-note">
        Recrée une composition faite dans Illustrator sur le plan : chaque image liée devient un dessin posé, dans un
        calque du même nom. Le plan de travail couvre toute la largeur de la façade.
      </p>
      <Button icon="import" onClick={() => void run()} disabled={!engine || !folder || !scaled || busy}>
        Importer un fichier .ai…
      </Button>
      {engine && folder && !scaled && (
        <p className="ui-note tone-warning">Mettre d'abord le plan à l'échelle (onglet Plan).</p>
      )}
      {busy && (
        <p className="ui-note">
          Import en cours{progress && progress.total > 0 ? ` : ${progress.done} / ${progress.total} fichiers` : "…"}
        </p>
      )}
      {summary && <p className="ui-note tone-success">{summary}</p>}
      {warnings.map((line) => (
        <p key={line} className="ui-note tone-warning">
          {line}
        </p>
      ))}
      {error && <p className="ui-note tone-danger">{error}</p>}
    </Section>
  );
}
