// Icônes de l'interface : dessin au trait de 1,5 px sur une grille de 16 px,
// couleur du texte (`currentColor`), à la manière des symboles du Mac.
// Une icône accompagne un libellé ; seule, elle porte une bulle d'aide
// (`IconButton`). Ajouter une icône ici plutôt que d'en dessiner une ailleurs.

import type { ReactNode } from "react";

const PATHS = {
  plus: <path d="M8 3v10M3 8h10" />,
  minus: <path d="M3 8h10" />,
  trash: (
    <>
      <path d="M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.6 8.5h5.8l.6-8.5" />
      <path d="M7 7v4M9 7v4" />
    </>
  ),
  pencil: <path d="M10.5 3.5l2 2L6 12H4v-2zM9.5 4.5l2 2" />,
  folder: <path d="M2.5 4.5v7.5h11V6H8L6.5 4.5z" />,
  document: (
    <>
      <path d="M4 2.5h5.5L12 5v8.5H4z" />
      <path d="M9.5 2.5V5H12" />
    </>
  ),
  import: (
    <>
      <path d="M8 2.5v7M5 6.5l3 3 3-3" />
      <path d="M3 10.5v2.5h10v-2.5" />
    </>
  ),
  export: (
    <>
      <path d="M8 9.5v-7M5 5.5l3-3 3 3" />
      <path d="M3 10.5v2.5h10v-2.5" />
    </>
  ),
  refresh: (
    <>
      <path d="M12.5 8a4.5 4.5 0 1 1-1.3-3.2" />
      <path d="M12 2.5v2.8H9.2" />
    </>
  ),
  fit: <path d="M3 6V3h3M10 3h3v3M13 10v3h-3M6 13H3v-3" />,
  chevronDown: <path d="M4.5 6.5L8 10l3.5-3.5" />,
  chevronRight: <path d="M6.5 4.5L10 8l-3.5 3.5" />,
  check: <path d="M3.5 8.5l3 3 6-7" />,
  close: <path d="M4.5 4.5l7 7M11.5 4.5l-7 7" />,
  search: (
    <>
      <circle cx="7" cy="7" r="4" />
      <path d="M10 10l3 3" />
    </>
  ),
  eye: (
    <>
      <path d="M1.5 8s2.5-4.5 6.5-4.5S14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z" />
      <circle cx="8" cy="8" r="2" />
    </>
  ),
  eyeOff: (
    <>
      <path d="M6 4c.6-.3 1.3-.5 2-.5 4 0 6.5 4.5 6.5 4.5s-.7 1.3-2 2.5M4.2 5.3C2.5 6.5 1.5 8 1.5 8s2.5 4.5 6.5 4.5c1.1 0 2.1-.3 2.9-.8" />
      <path d="M2.5 2.5l11 11" />
    </>
  ),
  warning: (
    <>
      <path d="M8 2.5l6 10.5H2z" />
      <path d="M8 6.5v3M8 11.2v.3" />
    </>
  ),
  error: (
    <>
      <circle cx="8" cy="8" r="5.5" />
      <path d="M8 5v3.5M8 10.7v.3" />
    </>
  ),
  info: (
    <>
      <circle cx="8" cy="8" r="5.5" />
      <path d="M8 7.5V11M8 5.2v.3" />
    </>
  ),
  target: (
    <>
      <circle cx="8" cy="8" r="5" />
      <path d="M8 1.5v4M8 10.5v4M1.5 8h4M10.5 8h4" />
    </>
  ),
  corners: <path d="M2.5 5.5v-3h3M10.5 2.5h3v3M13.5 10.5v3h-3M5.5 13.5h-3v-3M6 6h4v4H6z" />,
  projector: (
    <>
      <rect x="2" y="5" width="12" height="6.5" rx="1" />
      <circle cx="10.5" cy="8.2" r="1.7" />
      <path d="M4 7.5h2.5" />
    </>
  ),
  image: (
    <>
      <rect x="2.5" y="3" width="11" height="10" rx="1" />
      <path d="M2.5 11l3.5-3.5 3 3 2-2 2.5 2.5" />
      <circle cx="10.5" cy="6" r="1" />
    </>
  ),
  ruler: (
    <>
      <path d="M2 10.5L10.5 2 14 5.5 5.5 14z" />
      <path d="M5 7.5l1.5 1.5M7 5.5l1 1M9 3.5l1.5 1.5" />
    </>
  ),
  link: <path d="M7 9l2-2M6 6.5L4.5 8a2.1 2.1 0 0 0 3 3L9 9.5M10 9.5L11.5 8a2.1 2.1 0 0 0-3-3L7 6.5" />,
  ellipsis: (
    <>
      <circle cx="4" cy="8" r=".9" fill="currentColor" />
      <circle cx="8" cy="8" r=".9" fill="currentColor" />
      <circle cx="12" cy="8" r=".9" fill="currentColor" />
    </>
  ),
  keyboard: (
    <>
      <rect x="1.5" y="4" width="13" height="8" rx="1" />
      <path d="M4 6.5h.5M6.5 6.5H7M9 6.5h.5M11.5 6.5h.5M4.5 9.5h7" />
    </>
  ),
  lock: (
    <>
      <rect x="3.5" y="7" width="9" height="6.5" rx="1" />
      <path d="M5.5 7V5a2.5 2.5 0 0 1 5 0v2" />
    </>
  ),
  lockOpen: (
    <>
      <rect x="3.5" y="7" width="9" height="6.5" rx="1" />
      <path d="M5.5 7V5a2.5 2.5 0 0 1 4.9-.7" />
    </>
  ),
  arrowUp: <path d="M8 13V3M4 7l4-4 4 4" />,
  arrowDown: <path d="M8 3v10M4 9l4 4 4-4" />,
  sidebar: (
    <>
      <rect x="2" y="3" width="12" height="10" rx="1" />
      <path d="M6 3v10" />
    </>
  ),
} satisfies Record<string, ReactNode>;

export type IconName = keyof typeof PATHS;

export function Icon({ name, size = 16, className }: { name: IconName; size?: number; className?: string }) {
  return (
    <svg
      className={className ? `ui-icon ${className}` : "ui-icon"}
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {PATHS[name]}
    </svg>
  );
}
