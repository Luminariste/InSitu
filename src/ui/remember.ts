// Mémoire de l'interface (règle D15-v, demande de Benjamin du 14 novembre
// 2026 : « d'une manière générale, on mémorise l'interface »). Ce qu'on
// regarde et comment (écran, onglets, projecteur choisi, groupes dépliés,
// dossiers ouverts, zoom…) est retenu sur ce Mac (`localStorage`), au
// changement d'écran comme d'un lancement à l'autre ; jamais dans le projet.
// Un réglage propre à un projet porte le dossier du projet dans sa clé.

import { useCallback, useEffect, useRef, useState } from "react";

/** Clé d'un réglage propre à un projet (sans projet : la clé seule). */
export function projectKey(key: string, folder: string | null): string {
  return folder ? `${key}:${folder}` : key;
}

/** La valeur enregistrée sous `key`, relue par `parse` ; `fallback` si rien ou abîmé. */
export function recall<T>(key: string, parse: (raw: unknown) => T | undefined, fallback: T): T {
  try {
    const saved = localStorage.getItem(key);
    if (saved === null) return fallback;
    return parse(JSON.parse(saved)) ?? fallback;
  } catch {
    return fallback;
  }
}

/** Enregistre `value` sous `key` (`undefined` ou `null` : oublie). */
export function keep(key: string, value: unknown) {
  try {
    if (value === undefined || value === null) localStorage.removeItem(key);
    else localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Sans conséquence : l'interface reprendra son état d'ouverture.
  }
}

// Lecteurs usuels.
export const asString = (raw: unknown): string | undefined => (typeof raw === "string" ? raw : undefined);
export const asStrings = (raw: unknown): string[] | undefined =>
  Array.isArray(raw) ? raw.filter((x): x is string => typeof x === "string") : undefined;
export const oneOf =
  <T extends string>(values: readonly T[]) =>
  (raw: unknown): T | undefined =>
    values.includes(raw as T) ? (raw as T) : undefined;

/**
 * Un état React retenu sous `key` : relu quand la clé change (autre projet),
 * enregistré à chaque changement.
 */
export function useRemembered<T>(
  key: string,
  parse: (raw: unknown) => T | undefined,
  fallback: T,
): [T, (next: T | ((current: T) => T)) => void] {
  const [state, setState] = useState<{ key: string; value: T }>(() => ({ key, value: recall(key, parse, fallback) }));
  const parseRef = useRef(parse);
  parseRef.current = parse;
  const fallbackRef = useRef(fallback);
  fallbackRef.current = fallback;
  useEffect(() => {
    if (state.key !== key) setState({ key, value: recall(key, parseRef.current, fallbackRef.current) });
  }, [key, state.key]);
  const value = state.key === key ? state.value : recall(key, parse, fallback);
  const set = useCallback(
    (next: T | ((current: T) => T)) =>
      setState((current) => {
        const base = current.key === key ? current.value : recall(key, parseRef.current, fallbackRef.current);
        const updated = typeof next === "function" ? (next as (c: T) => T)(base) : next;
        keep(key, updated);
        return { key, value: updated };
      }),
    [key],
  );
  return [value, set];
}
