import { useEffect, useState } from "react";
import { fetchEngineInfo, type EngineInfo } from "./engine";

type EngineState =
  | { status: "loading" }
  | { status: "ready"; info: EngineInfo }
  | { status: "browser" }
  | { status: "error"; message: string };

const nombre = new Intl.NumberFormat("fr-FR");

export function App() {
  const [engine, setEngine] = useState<EngineState>({ status: "loading" });

  useEffect(() => {
    let cancelled = false;
    fetchEngineInfo()
      .then((info) => {
        if (cancelled) return;
        setEngine(info ? { status: "ready", info } : { status: "browser" });
      })
      .catch((error: unknown) => {
        if (cancelled) return;
        setEngine({ status: "error", message: String(error) });
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <main className="accueil">
      <header>
        <h1>InSitu</h1>
        <p className="sous-titre">Atelier de calepinage pour fresques projetées</p>
      </header>

      <section className="etat" aria-live="polite">
        {engine.status === "loading" && <p>Connexion au moteur…</p>}

        {engine.status === "ready" && (
          <dl>
            <div>
              <dt>Moteur</dt>
              <dd>version {engine.info.version}</dd>
            </div>
            <div>
              <dt>Diapositive</dt>
              <dd>
                {engine.info.slideSizeMm} mm sur plaque de {engine.info.plateSizeMm} mm
              </dd>
            </div>
            <div>
              <dt>Export</dt>
              <dd>
                {nombre.format(engine.info.defaultExportPx)} px à{" "}
                {nombre.format(engine.info.defaultExportDpi)} dpi
              </dd>
            </div>
          </dl>
        )}

        {engine.status === "browser" && (
          <p>
            Interface ouverte dans un navigateur : le moteur n'est disponible que dans
            l'application.
          </p>
        )}

        {engine.status === "error" && (
          <p className="erreur">Le moteur ne répond pas : {engine.message}</p>
        )}
      </section>

      <footer>Jalon 1 · squelette de l'application</footer>
    </main>
  );
}
