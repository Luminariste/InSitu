// Écoute les fichiers glissés depuis le Finder sur toute la fenêtre
// (D-4.1-f). Les glisser internes (dessin vers la façade, vignettes vers un
// dossier) ne portent pas de fichiers : ils sont laissés à leur écran.

import { useEffect, useRef, useState } from "react";
import { carriesFiles, droppedDrawings, type DroppedFile } from "./drop";

/** Où le dépôt a eu lieu, pris au moment du lâcher. */
export interface DropPlace {
  clientX: number;
  clientY: number;
  target: EventTarget | null;
}

/**
 * Rend vrai pendant qu'un glisser du Finder survole la fenêtre. `onDrop`
 * reçoit les dessins déposés (dossiers parcourus) et le nombre de fichiers
 * écartés ; sans `enabled`, le lâcher est ignoré (rien n'est ouvert).
 */
export function useFinderDrop(
  enabled: boolean,
  onDrop: (files: DroppedFile[], ignored: number, place: DropPlace) => void,
): boolean {
  const [dragging, setDragging] = useState(false);
  const handler = useRef(onDrop);
  handler.current = onDrop;
  const allowed = useRef(enabled);
  allowed.current = enabled;

  useEffect(() => {
    const over = (event: DragEvent) => {
      if (!event.dataTransfer || !carriesFiles(event.dataTransfer.types)) return;
      // Sans cela, la vue web ouvrirait le fichier à la place de l'app.
      event.preventDefault();
      event.dataTransfer.dropEffect = allowed.current ? "copy" : "none";
      setDragging(allowed.current);
    };
    const leave = (event: DragEvent) => {
      // Sortie de la fenêtre : plus de cible.
      if (event.relatedTarget === null) setDragging(false);
    };
    const drop = (event: DragEvent) => {
      if (!event.dataTransfer || !carriesFiles(event.dataTransfer.types)) return;
      event.preventDefault();
      setDragging(false);
      if (!allowed.current) return;
      const place = { clientX: event.clientX, clientY: event.clientY, target: event.target };
      void droppedDrawings(event.dataTransfer).then(({ files, ignored }) => handler.current(files, ignored, place));
    };
    window.addEventListener("dragenter", over);
    window.addEventListener("dragover", over);
    window.addEventListener("dragleave", leave);
    window.addEventListener("drop", drop);
    return () => {
      window.removeEventListener("dragenter", over);
      window.removeEventListener("dragover", over);
      window.removeEventListener("dragleave", leave);
      window.removeEventListener("drop", drop);
    };
  }, []);

  return dragging;
}
