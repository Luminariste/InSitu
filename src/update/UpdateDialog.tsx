// Mise à jour automatique (D16) : au lancement, l'app cherche une version
// plus récente ; si elle en trouve une, ce dialogue la propose. Le menu
// InSitu → Rechercher les mises à jour… relance la recherche (D16-f). « Installer »
// télécharge, vérifie la signature, remplace l'app et la relance.
import { useEffect, useState } from "react";
import { appVersion, checkForUpdate, checkForUpdateNow, hasEngine, installUpdate, type UpdateInfo, type UpdateProgress } from "../engine";
import { Banner, Button, Dialog, Progress } from "../ui";
import { checkFailureMessage, failureMessage, isDownloaded, progressLabel, progressShare, upToDateMessage } from "./update";
import "./update.css";

type Step = { kind: "offer" } | { kind: "installing"; progress: UpdateProgress | null } | { kind: "failed"; message: string };

/** Démonstration dans un simple navigateur : `?maj=demo` (capture d'écran). */
function demoUpdate(): UpdateInfo | null {
  if (hasEngine() || new URLSearchParams(window.location.search).get("maj") !== "demo") return null;
  return { currentVersion: "0.1.31", version: "0.1.32", notes: null };
}

/** Identifiant de l'entrée « Rechercher les mises à jour… » du menu InSitu (src-tauri/src/menu.rs). */
export const CHECK_UPDATES_MENU = "mises-a-jour";

/** Résultat d'une recherche demandée à la main. */
type Manual = { kind: "searching" } | { kind: "upToDate"; version: string | null } | { kind: "failed"; message: string };

/**
 * Cherche une fois au lancement, sans rien dire si l'app est à jour ou si
 * la recherche échoue. Chaque nouvelle valeur de `request` (menu InSitu →
 * Rechercher les mises à jour…) relance la recherche, et dit cette fois
 * le résultat : à jour, ou pourquoi on ne sait pas.
 */
export function UpdateCheck({ request = 0 }: { request?: number }) {
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [manual, setManual] = useState<Manual | null>(null);
  useEffect(() => {
    const demo = demoUpdate();
    if (demo) return setUpdate(demo);
    let alive = true;
    void checkForUpdate().then((found) => alive && setUpdate(found));
    return () => {
      alive = false;
    };
  }, []);
  useEffect(() => {
    if (request === 0) return;
    let alive = true;
    setUpdate(null);
    setManual({ kind: "searching" });
    checkForUpdateNow().then(
      async (found) => {
        if (!alive) return;
        if (found) {
          setManual(null);
          setUpdate(found);
          return;
        }
        const version = await appVersion().catch(() => null);
        if (alive) setManual({ kind: "upToDate", version });
      },
      (error: unknown) => alive && setManual({ kind: "failed", message: checkFailureMessage(error) }),
    );
    return () => {
      alive = false;
    };
  }, [request]);
  if (update) return <UpdateDialog update={update} onClose={() => setUpdate(null)} />;
  if (!manual) return null;
  const close = () => setManual(null);
  return (
    <Dialog
      title={manual.kind === "searching" ? "Recherche de mise à jour…" : manual.kind === "upToDate" ? "InSitu est à jour" : "Recherche impossible"}
      onClose={close}
      actions={
        <Button variant="primary" onClick={close}>
          {manual.kind === "searching" ? "Annuler" : "OK"}
        </Button>
      }
    >
      {manual.kind === "searching" && <p>Recherche d'une version plus récente sur GitHub…</p>}
      {manual.kind === "upToDate" && <p>{upToDateMessage(manual.version)}</p>}
      {manual.kind === "failed" && <Banner tone="warning">{manual.message}</Banner>}
    </Dialog>
  );
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
