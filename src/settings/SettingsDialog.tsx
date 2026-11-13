// Fenêtre « Réglages… » du menu InSitu (⌘,) : les choix propres à ce Mac et
// communs à tous les projets (demande de Benjamin, 13 novembre 2026,
// D-4.1-j). Premier réglage : le dossier de la bibliothèque commune
// (D-4.1-i) ; l'apparence y est aussi. D'autres réglages du même genre
// viendront s'y ranger, une rubrique chacun.

import { useState } from "react";
import { chooseFolder, hasEngine, saveSharedLibrary } from "../engine";
import { APPEARANCES, type Appearance } from "../ui/appearance";
import { Banner, Button, Dialog, Section, Segmented } from "../ui";
import { publishSettings, useAppSettings } from "./store";
import "./settings.css";

interface Props {
  appearance: Appearance;
  onAppearance: (appearance: Appearance) => void;
  onClose: () => void;
}

export function SettingsDialog({ appearance, onAppearance, onClose }: Props) {
  const engine = hasEngine();
  const settings = useAppSettings();
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const setShared = (folder: string | null) => {
    setBusy(true);
    setError(null);
    saveSharedLibrary(folder)
      .then(publishSettings, (failure: unknown) => setError(String(failure).replace(/^Error: /, "")))
      .finally(() => setBusy(false));
  };

  const choose = () => {
    chooseFolder("Dossier de la bibliothèque commune (dans Dropbox)").then(
      (folder) => folder && setShared(folder),
      (failure: unknown) => setError(String(failure)),
    );
  };

  return (
    <Dialog
      title="Réglages"
      wide
      onClose={onClose}
      actions={
        <Button variant="primary" onClick={onClose}>
          Fermer
        </Button>
      }
    >
      <Section title="Bibliothèque commune">
        <p className="settings-help">
          Des dessins partagés par tous les projets, dans un dossier Dropbox. Un dessin commun est copié dans le projet
          où on le prend : un projet déjà fait ne change jamais tout seul.
        </p>
        <p className="settings-path ui-selectable">
          {settings.sharedLibrary ?? (engine ? "Aucune bibliothèque commune choisie." : "Dans l'app seulement.")}
        </p>
        <div className="settings-actions">
          <Button icon="folder" onClick={choose} disabled={!engine || busy}>
            {settings.sharedLibrary ? "Changer de dossier…" : "Choisir un dossier…"}
          </Button>
          {settings.sharedLibrary && (
            <Button variant="quiet" onClick={() => setShared(null)} disabled={busy}>
              Ne plus utiliser
            </Button>
          )}
        </div>
        <p className="settings-help">
          Choisir un dossier qui n'est pas encore une bibliothèque y crée « Bibliothèque commune.insitu ». Choisir
          directement un dossier « .insitu » s'en sert tel quel.
        </p>
        {error && <Banner tone="danger">{error}</Banner>}
      </Section>
      <Section title="Apparence">
        <Segmented label="Apparence" value={appearance} onChange={onAppearance} options={APPEARANCES} />
      </Section>
    </Dialog>
  );
}
