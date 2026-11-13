// Réglages de l'app (D-4.1-j), partagés par tous les écrans : lus une fois
// auprès du moteur, puis tenus à jour par la fenêtre Réglages….

import { useEffect, useState } from "react";
import { fetchSettings, hasEngine, type AppSettings } from "../engine";

let current: AppSettings = {};
let loaded = false;
const listeners = new Set<(settings: AppSettings) => void>();

/** Remplace les réglages connus (après un enregistrement) et prévient les écrans. */
export function publishSettings(next: AppSettings): void {
  current = next;
  loaded = true;
  for (const listener of listeners) listener(next);
}

/** Les réglages de l'app, à jour. Sans moteur : aucun réglage. */
export function useAppSettings(): AppSettings {
  const [settings, setSettings] = useState<AppSettings>(current);
  useEffect(() => {
    listeners.add(setSettings);
    if (!loaded && hasEngine()) {
      loaded = true;
      fetchSettings().then(publishSettings, () => {
        loaded = false;
      });
    }
    return () => {
      listeners.delete(setSettings);
    };
  }, []);
  return settings;
}
