// Loupe : la photo à pleine résolution, grossie, autour d'un point. Faite de
// simples images (les tuiles du niveau 0) et d'un dessin SVG, sans seconde
// scène WebGL : légère, et chaque pixel de la photo y est net.

import type { Point, TileSource } from "../viewer";
import { loupePosition, loupeTiles } from "./logic";

export interface LoupeProps {
  source: TileSource;
  /** Point visé, en pixels de photo. */
  center: Point;
  /** Pixels d'écran par pixel de photo. */
  zoom: number;
  /** Côté de la loupe, en pixels d'écran. */
  size: number;
  /** Poignées des coins à montrer (pixels de photo), avec leur numéro. */
  handles?: Point[];
}

export function Loupe({ source, center, zoom, size, handles }: LoupeProps) {
  const tiles = loupeTiles(source.pyramid, center, zoom, size);
  const marks = (handles ?? [])
    .map((h, i) => ({ i, p: loupePosition(h, center, zoom, size) }))
    .filter(({ p }) => p.x > -20 && p.y > -20 && p.x < size + 20 && p.y < size + 20);
  const c = size / 2;
  return (
    <div className="loupe" style={{ width: size, height: size }}>
      {tiles.map((t) => (
        <img
          key={t.path}
          src={`${source.baseUrl}/${t.path}`}
          alt=""
          draggable={false}
          style={{ left: t.left, top: t.top, width: t.width, height: t.height }}
        />
      ))}
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
        {marks.map(({ i, p }) => (
          <g key={i} className="loupe-handle">
            <circle cx={p.x} cy={p.y} r={14} />
          </g>
        ))}
        {/* Croix de visée : le pixel visé est au centre. */}
        <g className="loupe-cross">
          <path d={`M${c - 22} ${c}H${c - 4}M${c + 4} ${c}H${c + 22}M${c} ${c - 22}V${c - 4}M${c} ${c + 4}V${c + 22}`} />
        </g>
      </svg>
    </div>
  );
}
