// Travail à deux : textes et choix de la fenêtre « Deux versions du projet »
// et des avertissements du verrou. Logique pure, testée sans écran.

import type { ConflictDifference, ConflictSide, LockInfo, SectionCount } from "../engine";

/** Rythme du signe de vie du verrou (LOCK_REFRESH_INTERVAL_SECONDS du moteur). */
export const LOCK_REFRESH_MS = 120_000;

/** « Dessins posés : 5 · Bibliothèque : 1 » à partir des comptes du moteur. */
export function describeCounts(counts: SectionCount[]): string {
  return counts.map((c) => `${c.section} : ${c.count}`).join(" · ");
}

/** Choix de départ : la suggestion, sinon la version ouverte. */
export function initialChoices(differences: ConflictDifference[]): Record<string, ConflictSide> {
  const choices: Record<string, ConflictSide> = {};
  for (const d of differences) choices[d.key] = d.suggested ?? "current";
  return choices;
}

/** Tout d'un côté. */
export function allChoices(differences: ConflictDifference[], side: ConflictSide): Record<string, ConflictSide> {
  const choices: Record<string, ConflictSide> = {};
  for (const d of differences) choices[d.key] = side;
  return choices;
}

/** Les différences par rubrique, dans l'ordre du moteur. */
export function groupBySection(differences: ConflictDifference[]): { section: string; items: ConflictDifference[] }[] {
  const groups: { section: string; items: ConflictDifference[] }[] = [];
  for (const d of differences) {
    const last = groups[groups.length - 1];
    if (last && last.section === d.section) last.items.push(d);
    else groups.push({ section: d.section, items: [d] });
  }
  return groups;
}

/** Nom lisible de la copie : « Copie en conflit de Benjamin Nesme 2026-11-15 ». */
export function copyTitle(file: string): string {
  const inside = /\(([^)]*)\)/.exec(file);
  return inside ? inside[1] : file;
}

/** Date et heure locales, courtes : « 15/11 à 12:03 ». */
export function shortTime(iso: string | null, now: Date = new Date()): string {
  if (!iso) return "date inconnue";
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "date inconnue";
  const pad = (n: number) => String(n).padStart(2, "0");
  const hour = `${pad(date.getHours())}:${pad(date.getMinutes())}`;
  const sameDay = date.toDateString() === now.toDateString();
  return sameDay ? `aujourd'hui à ${hour}` : `le ${pad(date.getDate())}/${pad(date.getMonth() + 1)} à ${hour}`;
}

/** « il y a 3 min » depuis le dernier signe de vie. */
export function minutesAgo(iso: string, now: Date = new Date()): string {
  const minutes = Math.max(0, Math.round((now.getTime() - new Date(iso).getTime()) / 60_000));
  if (minutes < 1) return "à l'instant";
  if (minutes < 60) return `il y a ${minutes} min`;
  const hours = Math.floor(minutes / 60);
  return `il y a ${hours} h ${String(minutes % 60).padStart(2, "0")}`;
}

/** « benjamin sur MacBook-Pro-de-Benjamin » : qui tient le verrou. */
export function holderName(holder: LockInfo): string {
  const machine = holder.machine.replace(/\.local$/, "");
  return `${holder.user} sur « ${machine} »`;
}
