// Mise à jour automatique (D16) : au lancement, l'app cherche une version
// plus récente ; si elle en trouve une, ce dialogue la propose. « Installer »
// télécharge, vérifie la signature, remplace l'app et la relance.
import { useEffect, useState } from "react";
import { checkForUpdate, hasEngine, installUpdate, type UpdateInfo, type UpdateProgress } from "../engine";
import { Banner, Button, Dialog, Progress } from "../ui";
import { failureMessage, isDownloaded, progressLabel, progressShare } from "./update";
import "./update.css";

type Step = { kind: "offer" } | { kind: "installing"; progress: UpdateProgress | null } | { kind: "failed"; message: string };

/** Démonstration dans un simple navigateur : `?maj=demo` (capture d'écran). */
function demoUpdate(): UpdateInfo | null {
  if (hasEngine() || new URLSearchParams(window.location.search).get("maj") !== "demo") return null;
  return { currentVersion: "0.1.31", version: "0.1.32", notes: null };
}

/** Cherche une fois au lancement ; n'affiche rien si l'app est à jour. */
export function UpdateCheck() {
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  useEffect(() => {
    const demo = demoUpdate();
    if (demo) return setUpdate(demo);
    let alive = true;
    void checkForUpdate().then((found) => alive && setUpdate(found));
    return () => {
      alive = false;
    };
  }, []);
  if (!update) return null;
  return <UpdateDialog update={update} onClose={() => setUpdate(null)} />;
}

export function UpdateDialog({ update, onClose }: { update: UpdateInfo; onClose: () => void }) {
  const [step, setStep] = useState<Step>({ kind: "offer" });

  const install = async () => {
    setStep({ kind: "installing", progress: null });
    if (!hasEngine()) {
      // Démonstration : faux téléchargement, rien n'est installé.
      for (let done = 0; done <= 40; done += 8) {
        setStep({ kind: "installing", progress: { done: done * 1024 * 1024, total: 40 * 1024 * 1024 } });
        await new Promise((r) => setTimeout(r, 300));
      }
      return;
    }
    try {
      await installUpdate((progress) => setStep({ kind: "installing", progress }));
    } catch (error) {
      setStep({ kind: "failed", message: failureMessage(error) });
    }
  };

  if (step.kind === "installing") {
    const progress = step.progress;
    const share = progress ? progressShare(progress) : null;
    return (
      // Pas de fermeture pendant l'installation : l'app va se relancer.
      <Dialog title={`Installation d'InSitu ${update.version}`} onClose={() => {}}>
        <p>
          {progress && isDownloaded(progress)
            ? "Téléchargement terminé. Remplacement de l'app, puis relance…"
            : "Téléchargement de la nouvelle version…"}
        </p>
        <Progress
          done={share === null ? 0 : Math.round(share * 1000)}
          total={share === null ? 0 : 1000}
          label={progress ? progressLabel(progress) : "Connexion…"}
        />
        <p className="update-note">Vos projets ne sont pas touchés. L'app se relance toute seule.</p>
      </Dialog>
    );
  }

  return (
    <Dialog
      title="Une nouvelle version est disponible"
      onClose={onClose}
      actions={
        step.kind === "failed" ? (
          <Button variant="primary" onClick={onClose}>
            Fermer
          </Button>
        ) : (
          <>
            <Button onClick={onClose}>Plus tard</Button>
            <Button variant="primary" onClick={() => void install()}>
              Installer et relancer
            </Button>
          </>
        )
      }
    >
      <p>
        InSitu <strong>{update.version}</strong> est prête (vous avez la {update.currentVersion}).
      </p>
      {update.notes && <p className="update-note">{update.notes}</p>}
      {step.kind === "failed" ? (
        <Banner tone="danger">
          {step.message} Vous pouvez aussi la télécharger à la main dans l'onglet « Releases » du dépôt.
        </Banner>
      ) : (
        <p className="update-note">
          L'app télécharge la nouvelle version, vérifie qu'elle vient bien de Luminariste, se remplace et se relance. Vos projets ne sont pas
          touchés. « Plus tard » : la question reviendra au prochain lancement.
        </p>
      )}
    </Dialog>
  );
}
