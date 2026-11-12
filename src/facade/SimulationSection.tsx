// Rubrique « Simulation » du panneau de gauche de l'écran Façade (tâche 6.1) :
// lancer la simulation, montrer les diapositives calculées sur la façade, et
// la liste des alertes ; un clic sur une alerte mène au dessin concerné.

import type { Simulation, SimulationAlert } from "../engine";
import { Button, Dot, List, ListRow, Section } from "../ui";
import { ALERT_TITLES, simulationSummary } from "./simulation";

interface Props {
  simulation: Simulation | null;
  /** Ce qui empêche de simuler (pas de projet, pas de moteur) ; `null` : possible. */
  blocked: string | null;
  busy: boolean;
  onRun: () => void;
  showSlides: boolean;
  onShowSlides: (show: boolean) => void;
  chosen: number | null;
  onAlert: (alert: SimulationAlert, index: number) => void;
}

export function SimulationSection({
  simulation,
  blocked,
  busy,
  onRun,
  showSlides,
  onShowSlides,
  chosen,
  onAlert,
}: Props) {
  return (
    <Section title="Simulation">
      <Button
        icon="refresh"
        title="Calculer toutes les diapositives et les poser ensemble sur la façade"
        onClick={onRun}
        disabled={blocked !== null || busy}
      >
        {busy ? "Simulation…" : simulation ? "Simuler à nouveau" : "Simuler"}
      </Button>
      {blocked && <p className="ui-list-empty">{blocked}</p>}
      {simulation && (
        <>
          <label className="ui-check">
            <input type="checkbox" checked={showSlides} onChange={(e) => onShowSlides(e.target.checked)} />
            Montrer les diapositives calculées
          </label>
          <p>{simulationSummary(simulation)}</p>
          {simulation.alerts.length > 0 && (
            <List label="Alertes">
              {simulation.alerts.map((alert, index) => (
                <ListRow
                  key={`${alert.kind}:${alert.placement ?? ""}:${index}`}
                  selected={chosen === index}
                  onSelect={() => onAlert(alert, index)}
                  tone={alert.kind === "outside" ? "danger" : "warning"}
                  tooltip={`${alert.message} (cliquer pour aller voir sur la façade)`}
                  leading={<Dot tone={alert.kind === "outside" ? "danger" : "warning"} />}
                  title={ALERT_TITLES[alert.kind]}
                  detail={alert.message}
                />
              ))}
            </List>
          )}
        </>
      )}
    </Section>
  );
}
