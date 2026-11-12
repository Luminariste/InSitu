import { useState } from "react";
import { FacadeScreen } from "./facade/FacadeScreen";
import { LibraryScreen } from "./library/LibraryScreen";
import { RectificationScreen } from "./rectification/RectificationScreen";
import { RegistrationScreen } from "./registration/RegistrationScreen";
import { ViewerDemo } from "./ViewerDemo";

type Screen = "facade" | "redressement" | "recalage" | "bibliotheque" | "demo";

// Cinq écrans pour l'instant : la façade (tâche 3.1), le redressement des
// photos de mire (tâche 2.5), le recalage des projecteurs sur la façade
// (tâche 3.2), la bibliothèque de dessins (tâche 4.1) et la démonstration de
// la visionneuse (jalon 1, provisoire, voir D13).
export function App() {
  const [screen, setScreen] = useState<Screen>("facade");
  return (
    <div className="app">
      <nav className="app-tabs" aria-label="Écrans">
        <button type="button" className={screen === "facade" ? "actif" : ""} onClick={() => setScreen("facade")}>
          Façade
        </button>
        <button
          type="button"
          className={screen === "redressement" ? "actif" : ""}
          onClick={() => setScreen("redressement")}
        >
          Redressement
        </button>
        <button type="button" className={screen === "recalage" ? "actif" : ""} onClick={() => setScreen("recalage")}>
          Recalage
        </button>
        <button
          type="button"
          className={screen === "bibliotheque" ? "actif" : ""}
          onClick={() => setScreen("bibliotheque")}
        >
          Bibliothèque
        </button>
        <button type="button" className={screen === "demo" ? "actif" : ""} onClick={() => setScreen("demo")}>
          Démonstration
        </button>
      </nav>
      <div className="app-screen">
        {screen === "facade" ? (
          <FacadeScreen />
        ) : screen === "redressement" ? (
          <RectificationScreen />
        ) : screen === "recalage" ? (
          <RegistrationScreen />
        ) : screen === "bibliotheque" ? (
          <LibraryScreen />
        ) : (
          <ViewerDemo />
        )}
      </div>
    </div>
  );
}
