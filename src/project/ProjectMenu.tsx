// Menu « Projet », le même en tête de la barre d'outils de chaque écran :
// le nom du projet ouvert, et Nouveau / Ouvrir / Relire avec leurs
// raccourcis (⌘N, ⌘O, ⌘R), et Annuler / Rétablir (⌘Z, ⇧⌘Z, tâche 4.3).
// L'écran garde la main sur l'ouverture : le menu lui donne seulement le
// dossier choisi, et lui fait relire le projet après une annulation.

import { chooseNewProjectPath, chooseProjectFolder, createProject, hasEngine, historyStep } from "../engine";
import { projectNameFromPath, withProjectExtension } from "../facade/scale";
import { MenuButton, useShortcuts } from "../ui";
import { isTextEntry } from "../ui/shortcuts";
import { rememberProject } from "./current";

interface Props {
  /** Nom du projet ouvert ; `null` si aucun. */
  name: string | null;
  /** Dossier du projet ouvert, montré en bulle d'aide. */
  folder: string | null;
  /** Ouvre le projet de ce dossier dans l'écran (après création ou choix). */
  onOpen: (folder: string) => void;
  /** Relit le projet depuis le disque (changements faits ailleurs). */
  onReload?: () => void;
  /** Faux pendant un travail en cours. */
  ready: boolean;
  /** Ce que montre le menu sans moteur (navigateur seul). */
  placeholder?: string;
  /** Erreur à remonter à l'écran. */
  onError: (message: string) => void;
}

/** Crée un projet là où l'utilisateur le demande ; renvoie son dossier. */
export async function askNewProject(): Promise<string | null> {
  const chosen = await chooseNewProjectPath();
  if (!chosen) return null;
  const path = withProjectExtension(chosen);
  await createProject(path, projectNameFromPath(path));
  return path;
}

export function ProjectMenu({ name, folder, onOpen, onReload, ready, placeholder, onError }: Props) {
  const engine = hasEngine();
  const attempt = (task: () => Promise<string | null>) => () => {
    task().then(
      (path) => {
        if (!path) return;
        // Les autres écrans rouvriront ce projet.
        rememberProject(path);
        onOpen(path);
      },
      (failure: unknown) => onError(String(failure).replace(/^Error: /, "")),
    );
  };
  const create = attempt(askNewProject);
  const open = attempt(chooseProjectFolder);
  const canReload = engine && folder !== null && ready && onReload !== undefined;
  const canUndo = canReload;

  // Annule ou rétablit la dernière action enregistrée, quel que soit l'écran
  // qui l'a faite, puis l'écran relit le projet.
  const step = (redo: boolean) => {
    if (!folder || !canUndo) return;
    historyStep(folder, redo).then(
      () => onReload?.(),
      (failure: unknown) => {
        const message = String(failure).replace(/^Error: /, "");
        // « Rien à annuler » n'est pas une erreur.
        if (!message.startsWith("Rien à")) onError(message);
      },
    );
  };
  // ⌘Z dans un champ de texte reste l'annulation de la frappe. Les mêmes
  // raccourcis arrivent aussi du menu Édition du Mac (src-tauri/src/menu.rs).
  const undoOrRedo = (redo: boolean) => () => {
    if (isTextEntry(document.activeElement as HTMLElement | null)) document.execCommand(redo ? "redo" : "undo");
    else step(redo);
  };

  useShortcuts([
    { keys: "mod+n", run: create, enabled: engine && ready },
    { keys: "mod+o", run: open, enabled: engine && ready },
    { keys: "mod+r", run: () => onReload?.(), enabled: canReload },
    { keys: "mod+shift+z", run: undoOrRedo(true) },
    { keys: "mod+z", run: undoOrRedo(false) },
  ]);

  return (
    <MenuButton
      icon="folder"
      label={name ?? (engine ? "Aucun projet" : (placeholder ?? "Exemple sans moteur"))}
      title={folder ?? "Projet"}
      className="project-menu"
      items={[
        { label: "Nouveau projet…", shortcut: "mod+n", onSelect: create, disabled: !engine || !ready },
        { label: "Ouvrir un projet…", shortcut: "mod+o", onSelect: open, disabled: !engine || !ready },
        { separator: true },
        { label: "Annuler", shortcut: "mod+z", onSelect: () => step(false), disabled: !canUndo },
        { label: "Rétablir", shortcut: "mod+shift+z", onSelect: () => step(true), disabled: !canUndo },
        { separator: true },
        {
          label: "Relire depuis le disque",
          shortcut: "mod+r",
          onSelect: () => onReload?.(),
          disabled: !canReload,
        },
      ]}
    />
  );
}
