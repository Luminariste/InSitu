// Panneau « Projecteurs » de l'écran Façade (tâche 3.3) : la liste des
// projecteurs posés sur la façade, ce qu'on montre, et les recouvrements.

import type { FacadeOverview } from "../engine";
import { IconButton, List, ListRow, Panel, Section } from "../ui";
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
    <Panel
      side="left"
      title="Projecteurs"
      actions={onReload && <IconButton icon="refresh" label="Recalculer les emprises" size="sm" onClick={onReload} disabled={busy} />}
    >
      {!overview ? (
        <p className="ui-list-empty">{busy ? "Calcul des emprises…" : "Ouvrir un projet pour voir ses projecteurs."}</p>
      ) : (
        <>
          <Section>
            <p>{overviewSummary(overview)}</p>
            <List label="Projecteurs">
              {overview.projectors.map((p, index) => {
                const placed = p.footprint.length > 2;
                const shown = placed && !display.hidden.has(p.projectorId);
                return (
                  <ListRow
                    key={p.projectorId}
                    dimmed={!placed}
                    tone={p.problem ? "warning" : undefined}
                    onSelect={placed ? () => toggle(p.projectorId) : undefined}
                    tooltip={placed ? "Cliquer pour montrer ou cacher ce projecteur" : (p.problem ?? undefined)}
                    leading={
                      <>
                        <input type="checkbox" checked={shown} disabled={!placed} onChange={() => toggle(p.projectorId)} aria-label={`Montrer ${p.label}`} onClick={(e) => e.stopPropagation()} />
                        <span className="ui-swatch" style={{ background: placed ? cssColor(projectorColor(index)) : "transparent" }} />
                      </>
                    }
                    title={p.label}
                    detail={projectorStatus(p)}
                  />
                );
              })}
            </List>
          </Section>
          <Section title="Afficher">
            <div className="facade-overview-show">
              <label className="ui-check">
                <input type="checkbox" checked={display.photos} onChange={(e) => onDisplay({ ...display, photos: e.target.checked })} />
                Photos
              </label>
              <label className="ui-check">
                <input type="checkbox" checked={display.footprints} onChange={(e) => onDisplay({ ...display, footprints: e.target.checked })} />
                Emprises
              </label>
              <label className="ui-check">
                <input type="checkbox" checked={display.overlaps} onChange={(e) => onDisplay({ ...display, overlaps: e.target.checked })} />
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
                  onChange={(e) => onDisplay({ ...display, photoOpacity: Number(e.target.value) })}
                />
              </label>
            </div>
          </Section>
          {overview.overlaps.length > 0 && (
            <Section title="Recouvrements">
              <List label="Recouvrements">
                {overview.overlaps.map((o) => (
                  <ListRow key={o.projectors.join("+")} title={overlapLabel(o, overview.projectors)} trailing={formatArea(o.areaM2)} />
                ))}
              </List>
            </Section>
          )}
        </>
      )}
    </Panel>
  );
}
