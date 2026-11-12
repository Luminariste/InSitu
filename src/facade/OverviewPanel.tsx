// Panneau « Projecteurs » de l'écran Façade (tâche 3.3) : la liste des
// projecteurs posés sur la façade, ce qu'on montre, et les recouvrements.

import type { FacadeOverview } from "../engine";
import {
  cssColor,
  formatArea,
  overlapLabel,
  overviewSummary,
  projectorColor,
  projectorStatus,
  type OverviewDisplay,
} from "./overview";

interface Props {
  overview: FacadeOverview | null;
  display: OverviewDisplay;
  onDisplay: (display: OverviewDisplay) => void;
  onReload: (() => void) | null;
  busy: boolean;
}

export function OverviewPanel({ overview, display, onDisplay, onReload, busy }: Props) {
  const toggle = (id: string) => {
    const hidden = new Set(display.hidden);
    if (hidden.has(id)) hidden.delete(id);
    else hidden.add(id);
    onDisplay({ ...display, hidden });
  };
  return (
    <aside className="facade-overview" aria-label="Projecteurs">
      <div className="facade-overview-head">
        <h2>Projecteurs</h2>
        {onReload && (
          <button type="button" onClick={onReload} disabled={busy}>
            Relire
          </button>
        )}
      </div>
      {!overview ? (
        <p className="facade-overview-note">
          {busy ? "Calcul des emprises…" : "Ouvrir un projet pour voir ses projecteurs."}
        </p>
      ) : (
        <>
          <p className="facade-overview-note">{overviewSummary(overview)}</p>
          <fieldset className="facade-overview-show">
            <label>
              <input
                type="checkbox"
                checked={display.photos}
                onChange={(e) => onDisplay({ ...display, photos: e.target.checked })}
              />
              Photos
            </label>
            <label>
              <input
                type="checkbox"
                checked={display.footprints}
                onChange={(e) => onDisplay({ ...display, footprints: e.target.checked })}
              />
              Emprises
            </label>
            <label>
              <input
                type="checkbox"
                checked={display.overlaps}
                onChange={(e) => onDisplay({ ...display, overlaps: e.target.checked })}
              />
              Recouvrements
            </label>
            <label className="facade-overview-opacity">
              Opacité des photos
              <input
                type="range"
                min={0}
                max={1}
                step={0.05}
                value={display.photoOpacity}
                disabled={!display.photos}
                onChange={(e) =>
                  onDisplay({
                    ...display,
                    photoOpacity: Number(e.target.value),
                  })
                }
              />
            </label>
          </fieldset>
          <ul className="facade-overview-list">
            {overview.projectors.map((p, index) => {
              const placed = p.footprint.length > 2;
              return (
                <li key={p.projectorId} className={placed ? "" : "facade-overview-off"}>
                  <label>
                    <input
                      type="checkbox"
                      checked={placed && !display.hidden.has(p.projectorId)}
                      disabled={!placed}
                      onChange={() => toggle(p.projectorId)}
                    />
                    <span
                      className="facade-overview-swatch"
                      style={{
                        background: placed ? cssColor(projectorColor(index)) : "transparent",
                      }}
                    />
                    <strong>{p.label}</strong>
                  </label>
                  <span className={p.problem ? "facade-overview-problem" : "facade-overview-info"}>
                    {projectorStatus(p)}
                  </span>
                </li>
              );
            })}
          </ul>
          {overview.overlaps.length > 0 && (
            <>
              <h3>Recouvrements</h3>
              <ul className="facade-overview-list">
                {overview.overlaps.map((o) => (
                  <li key={o.projectors.join("+")}>
                    <span>{overlapLabel(o, overview.projectors)}</span>
                    <span className="facade-overview-info">{formatArea(o.areaM2)}</span>
                  </li>
                ))}
              </ul>
            </>
          )}
        </>
      )}
    </aside>
  );
}
