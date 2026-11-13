import { useEffect, useState } from "react";
import { FacadeScreen } from "./facade/FacadeScreen";
import { LibraryScreen } from "./library/LibraryScreen";
import { RectificationScreen } from "./rectification/RectificationScreen";
import { RegistrationScreen } from "./registration/RegistrationScreen";
import { SlideScreen } from "./slide/SlideScreen";
import { ViewerDemo } from "./ViewerDemo";
import { UpdateCheck } from "./update/UpdateDialog";
import { listenMenu } from "./engine";
import { listenProject, type OpenProject } from "./project/current";
import {
  APPEARANCES,
  APP_SHORTCUTS,
  Button,
  Dialog,
  Kbd,
  MenuButton,
  Segmented,
  applyAppearance,
  readAppearance,
  runShortcut,
  useShortcuts,
  type Appearance,
} from "./ui";

type Screen = "facade" | "redressement" | "recalage" | "diapositive" | "bibliotheque" | "demo";

/** Les étapes du travail, dans l'ordre, chacune avec son raccourci ⌘1…⌘5. */
const STEPS: { value: Exclude<Screen, "demo">; label: string; keys: string; title: string }[] = [
  { value: "facade", label: "Façade", keys: "mod+1", title: "Projet, plan de la façade, échelle, projecteurs" },
  { value: "redressement", label: "Redressement", keys: "mod+2", title: "Photos de mire redressées, projecteur par projecteur" },
  { value: "recalage", label: "Recalage", keys: "mod+3", title: "Points communs entre chaque photo et la façade" },
  { value: "diapositive", label: "Diapositive", keys: "mod+4", title: "Chaque diapositive calculée, sur sa photo redressée" },
  { value: "bibliotheque", label: "Bibliothèque", keys: "mod+5", title: "Les dessins du projet" },
];

// Barre de l'app (tout en haut) : les étapes, puis le menu Présentation.
// Chaque écran a ensuite sa barre d'outils, son espace de travail et sa barre
// d'état, selon le gabarit de docs/design.md. La démonstration de la
// visionneuse (provisoire, D13) est rangée dans le menu Présentation.
export function App() {
  const [screen, setScreen] = useState<Screen>("facade");
  const [appearance, setAppearance] = useState<Appearance>(readAppearance);
  const [help, setHelp] = useState(false);

  useEffect(() => applyAppearance(appearance), [appearance]);

  // Barre de menus du Mac : chaque entrée envoie son raccourci (src-tauri/src/menu.rs).
  useEffect(() => listenMenu((keys) => void runShortcut(keys)), []);

  // Le projet ouvert, à côté du nom de l'app (le menu Projet est dans la
  // barre de menus du Mac, D15-o).
  const [openProject, setOpenProject] = useState<OpenProject>({ name: null, folder: null });
  useEffect(() => listenProject(setOpenProject), []);

  useShortcuts([
    ...STEPS.map((s) => ({ keys: s.keys, run: () => setScreen(s.value) })),
    { keys: "mod+/", run: () => setHelp(true) },
  ]);

  return (
    <div className="app">
      <nav className="app-bar" aria-label="Étapes">
        <span className="app-name" title={openProject.folder ?? undefined}>
          InSitu
          {openProject.name && <span className="app-project"> · {openProject.name}</span>}
        </span>
        <Segmented
          label="Étapes"
          size="lg"
          value={screen === "demo" ? ("" as Screen) : screen}
          onChange={setScreen}
          options={STEPS.map((s, i) => ({
            value: s.value,
            title: s.title,
            shortcut: s.keys,
            label: (
              <>
                <span className="app-step-number">{i + 1}</span>
                {s.label}
              </>
            ),
          }))}
        />
        <span className="app-bar-end">
          <MenuButton
            label="Présentation"
            variant="quiet"
            align="end"
            items={[
              { heading: "Apparence" },
              ...APPEARANCES.map((a) => ({
                label: a.label,
                checked: appearance === a.value,
                onSelect: () => setAppearance(a.value),
              })),
              { separator: true },
              { label: "Raccourcis clavier", shortcut: "mod+/", icon: "keyboard", onSelect: () => setHelp(true) },
              {
                label: "Démonstration de la visionneuse",
                checked: screen === "demo" ? true : undefined,
                onSelect: () => setScreen("demo"),
              },
            ]}
          />
        </span>
      </nav>
      <div className="app-screen">
        {screen === "facade" ? (
          <FacadeScreen />
        ) : screen === "redressement" ? (
          <RectificationScreen />
        ) : screen === "recalage" ? (
          <RegistrationScreen />
        ) : screen === "diapositive" ? (
          <SlideScreen />
        ) : screen === "bibliotheque" ? (
          <LibraryScreen />
        ) : (
          <ViewerDemo />
        )}
      </div>
      {help && <ShortcutsDialog onClose={() => setHelp(false)} />}
      <UpdateCheck />
    </div>
  );
}

function ShortcutsDialog({ onClose }: { onClose: () => void }) {
  return (
    <Dialog
      title="Raccourcis clavier"
      onClose={onClose}
      wide
      actions={
        <Button variant="primary" onClick={onClose}>
          Fermer
        </Button>
      }
    >
      <div className="app-shortcuts">
        {APP_SHORTCUTS.map((group) => (
          <section key={group.group}>
            <h3>{group.group}</h3>
            <dl>
              {group.items.map((item) => (
                <div key={item.keys}>
                  <dt>{item.label}</dt>
                  <dd>
                    <Kbd keys={item.keys} />
                  </dd>
                </div>
              ))}
            </dl>
          </section>
        ))}
      </div>
      <p className="app-shortcuts-note">
        Dans les images : glisser pour se déplacer, pincer ou ⌘ + défilement pour zoomer, double-clic pour 100 %.
      </p>
    </Dialog>
  );
}
