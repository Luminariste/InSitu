import { useState } from "react";
import { FacadeScreen } from "./facade/FacadeScreen";
import { RectificationScreen } from "./rectification/RectificationScreen";
import { ViewerDemo } from "./ViewerDemo";

type Screen = "facade" | "redressement" | "demo";

// Trois écrans pour l'instant : la façade (tâche 3.1), le redressement des
// photos de mire (tâche 2.5) et la démonstration de la visionneuse (jalon 1,
// provisoire, voir D13).
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
        <button type="button" className={screen === "demo" ? "actif" : ""} onClick={() => setScreen("demo")}>
          Démonstration
        </button>
      </nav>
      <div className="app-screen">
        {screen === "facade" ? <FacadeScreen /> : screen === "redressement" ? <RectificationScreen /> : <ViewerDemo />}
      </div>
    </div>
  );
}
