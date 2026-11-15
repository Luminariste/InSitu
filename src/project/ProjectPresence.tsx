// Travail à deux sur un projet partagé par Dropbox.
//
// Monté une fois par l'app, il suit le projet ouvert (celui que l'écran
// annonce) et :
//
// - prend le verrou à l'ouverture ; si quelqu'un d'autre a le projet ouvert,
//   il le dit et propose d'ouvrir quand même ou de fermer ;
// - donne signe de vie toutes les deux minutes ; si l'autre Mac a pris le
//   verrou entre-temps, il le dit ;
// - rend le verrou quand on change de projet (la coque le rend aussi à la
//   fermeture de l'app) ;
// - cherche les copies en conflit de Dropbox à l'ouverture, à chaque signe
//   de vie et au retour dans l'app, et propose de réunir les deux versions.
//
// Après une réunion, ou pour fermer le projet, la fenêtre de l'app est
// rechargée : chaque écran relit le projet du disque.

import { useCallback, useEffect, useRef, useState } from "react";
import {
  compareConflict,
  findConflicts,
  giveBackLock,
  hasEngine,
  keepLock,
  resolveConflict,
  takeLock,
  type ConflictReport,
  type ConflictSide,
  type LockInfo,
} from "../engine";
import { Banner, Button, Dialog, Segmented } from "../ui";
import { rememberProject } from "./current";
import {
  LOCK_REFRESH_MS,
  allChoices,
  copyTitle,
  describeCounts,
  groupBySection,
  holderName,
  initialChoices,
  minutesAgo,
  shortTime,
} from "./conflictLogic";
import "./presence.css";

export type Notice =
  | { kind: "busy"; holder: LockInfo }
  | { kind: "replaced"; holder: LockInfo }
  | { kind: "lost"; holder: LockInfo }
  | { kind: "error"; message: string };

function closeProject() {
  rememberProject(null);
  window.location.reload();
}

export function ProjectPresence({ folder }: { folder: string | null }) {
  const [notice, setNotice] = useState<Notice | null>(null);
  const [report, setReport] = useState<ConflictReport | null>(null);
  const [others, setOthers] = useState<string[]>([]);
  const session = useRef<{ folder: string; id: string } | null>(null);
  // Copies laissées pour plus tard : pas redemandées avant la prochaine ouverture.
  const later = useRef<Set<string>>(new Set());

  const fail = (failure: unknown) => setNotice({ kind: "error", message: String(failure).replace(/^Error: /, "") });

  const checkConflicts = useCallback(async (target: string) => {
    const scan = await findConflicts(target);
    setOthers(scan.others);
    const copy = scan.projects.find((c) => !later.current.has(c.file));
    if (!copy) return;
    const compared = await compareConflict(target, copy.file);
    setReport((shown) => shown ?? compared);
  }, []);

  const take = useCallback(
    async (target: string, force: boolean) => {
      const attempt = await takeLock(target, force);
      if (attempt.state === "busy") {
        setNotice({ kind: "busy", holder: attempt.holder });
        return;
      }
      session.current = { folder: target, id: attempt.sessionId };
      setNotice(attempt.replaced ? { kind: "replaced", holder: attempt.replaced } : null);
      await checkConflicts(target);
    },
    [checkConflicts],
  );

  // Ouverture et changement de projet.
  useEffect(() => {
    if (!hasEngine() || !folder) return;
    later.current = new Set();
    setReport(null);
    take(folder, false).catch(fail);
    return () => {
      const held = session.current;
      session.current = null;
      if (held) giveBackLock(held.folder, held.id).catch(() => undefined);
    };
  }, [folder, take]);

  // Signe de vie, et copies en conflit arrivées entre-temps.
  useEffect(() => {
    if (!hasEngine() || !folder) return;
    const tick = () => {
      const held = session.current;
      if (!held || held.folder !== folder) return;
      keepLock(held.folder, held.id)
        .then((check) => {
          if (check.state === "lost") {
            session.current = null;
            setNotice({ kind: "lost", holder: check.holder });
          }
          return checkConflicts(held.folder);
        })
        .catch(() => undefined); // Dropbox ou disque momentanément absents : on réessaiera.
    };
    const timer = window.setInterval(tick, LOCK_REFRESH_MS);
    const onFocus = () => {
      if (session.current) checkConflicts(session.current.folder).catch(() => undefined);
    };
    window.addEventListener("focus", onFocus);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener("focus", onFocus);
    };
  }, [folder, checkConflicts]);

  if (!folder) return null;

  return (
    <>
      {notice && <LockNotice notice={notice} folder={folder} onForce={() => take(folder, true).catch(fail)} onClose={() => setNotice(null)} />}
      {!notice && report && (
        <ConflictDialog
          folder={folder}
          report={report}
          others={others}
          onLater={() => {
            later.current.add(report.copy.file);
            setReport(null);
          }}
          onError={fail}
        />
      )}
    </>
  );
}

export function LockNotice({ notice, folder, onForce, onClose }: { notice: Notice; folder: string; onForce: () => void; onClose: () => void }) {
  if (notice.kind === "error") {
    return (
      <Dialog title="Travail à deux" onClose={onClose} actions={<Button variant="primary" onClick={onClose}>OK</Button>}>
        <p>{notice.message}</p>
      </Dialog>
    );
  }
  const who = holderName(notice.holder);
  if (notice.kind === "replaced") {
    return (
      <Dialog title="Projet repris" onClose={onClose} actions={<Button variant="primary" onClick={onClose}>OK</Button>}>
        <p>
          Ce projet avait été ouvert par {who} {shortTime(notice.holder.openedAt)}. Son app n'a pas donné signe de vie depuis{" "}
          {minutesAgo(notice.holder.refreshedAt).replace("il y a ", "")} (Mac fermé ou hors connexion) : vous avez repris la main.
        </p>
        <p>Si l'autre Mac rouvre le projet sans s'être resynchronisé, Dropbox fera deux versions ; l'app vous proposera alors de les réunir.</p>
      </Dialog>
    );
  }
  const busy = notice.kind === "busy";
  return (
    <Dialog
      title={busy ? "Projet déjà ouvert" : "Projet ouvert sur l'autre Mac"}
      onClose={busy ? closeProject : onClose}
      actions={
        <>
          {/* Fermer d'abord : c'est lui qui reçoit le focus (↩ ne force jamais). */}
          <Button variant="primary" onClick={closeProject}>
            Fermer le projet
          </Button>
          <Button onClick={onForce}>{busy ? "Ouvrir quand même" : "Reprendre la main"}</Button>
        </>
      }
    >
      <p>
        {busy ? (
          <>
            Ce projet est ouvert par {who} depuis {shortTime(notice.holder.openedAt)} (dernier signe de vie {minutesAgo(notice.holder.refreshedAt)}).
          </>
        ) : (
          <>
            {who} a ouvert ce projet {shortTime(notice.holder.openedAt)}, pendant que vous l'aviez ouvert ici.
          </>
        )}
      </p>
      <p>
        Si vous modifiez tous les deux en même temps, Dropbox gardera deux versions. Rien ne sera perdu (l'app proposera de les réunir), mais
        mieux vaut que l'un ferme le projet.
      </p>
      <p className="presence-folder" title={folder}>
        {folder.split("/").pop()}
      </p>
    </Dialog>
  );
}

export function ConflictDialog({
  folder,
  report,
  others,
  onLater,
  onError,
}: {
  folder: string;
  report: ConflictReport;
  others: string[];
  onLater: () => void;
  onError: (failure: unknown) => void;
}) {
  const [choices, setChoices] = useState<Record<string, ConflictSide>>(() => initialChoices(report.differences));
  const [working, setWorking] = useState(false);
  const differences = report.differences;
  const title = copyTitle(report.copy.file);

  const merge = () => {
    setWorking(true);
    resolveConflict(folder, report.copy.file, choices).then(
      () => window.location.reload(),
      (failure: unknown) => {
        setWorking(false);
        onError(failure);
      },
    );
  };

  return (
    <Dialog
      title="Deux versions du projet"
      wide
      onClose={onLater}
      actions={
        <>
          <Button onClick={onLater} disabled={working}>
            Plus tard
          </Button>
          <Button variant="primary" onClick={merge} disabled={working}>
            {working ? "Réunion…" : "Réunir les deux versions"}
          </Button>
        </>
      }
    >
      <p>
        Le projet a été modifié sur deux Mac en même temps. Dropbox a gardé les deux : la version ouverte (enregistrée{" "}
        {shortTime(report.currentModified)}) et « {title} » (enregistrée {shortTime(report.copy.modified)}).
      </p>
      <section className="presence-kept">
        <h3>Ajouts gardés d'office</h3>
        {report.onlyCurrent.length === 0 && report.onlyCopy.length === 0 ? (
          <p>Aucun ajout propre à une seule version.</p>
        ) : (
          <ul>
            {report.onlyCurrent.length > 0 && <li>Seulement dans la version ouverte — {describeCounts(report.onlyCurrent)}</li>}
            {report.onlyCopy.length > 0 && <li>Seulement dans la copie — {describeCounts(report.onlyCopy)}</li>}
          </ul>
        )}
        <p className="presence-hint">Un élément effacé d'un seul côté revient : effacez-le à nouveau après la réunion.</p>
      </section>
      {differences.length > 0 && (
        <section className="presence-choices">
          <div className="presence-choices-head">
            <h3>Changé des deux côtés : {differences.length}</h3>
            <span>
              <Button size="sm" onClick={() => setChoices(allChoices(differences, "current"))}>
                Tout : version ouverte
              </Button>
              <Button size="sm" onClick={() => setChoices(allChoices(differences, "copy"))}>
                Tout : copie
              </Button>
              <Button size="sm" onClick={() => setChoices(initialChoices(differences))}>
                Suggestions
              </Button>
            </span>
          </div>
          <div className="presence-list">
            {groupBySection(differences).map((group) => (
              <div key={group.section}>
                <h4>{group.section}</h4>
                {group.items.map((d) => (
                  <div className="presence-row" key={d.key}>
                    <span className="presence-label">
                      {d.label}
                      <span className="presence-detail">
                        {d.detail}
                        {d.suggested && ` · plus récent : ${d.suggested === "copy" ? "copie" : "version ouverte"}`}
                      </span>
                    </span>
                    <Segmented
                      label={`Version gardée pour ${d.label}`}
                      value={choices[d.key] ?? "current"}
                      onChange={(side) => setChoices((c) => ({ ...c, [d.key]: side }))}
                      options={[
                        { value: "current", label: "Ouverte" },
                        { value: "copy", label: "Copie" },
                      ]}
                    />
                  </div>
                ))}
              </div>
            ))}
          </div>
          <p className="presence-hint">« Plus récent » vient de l'historique d'annulation de ce Mac ; sans indication, la version ouverte est gardée.</p>
        </section>
      )}
      {others.length > 0 && (
        <Banner tone="warning">
          Dropbox a aussi gardé en double {others.length === 1 ? "ce fichier" : "ces fichiers"} : {others.join(", ")}. L'app ne les réunit pas :
          vérifiez-les dans le Finder.
        </Banner>
      )}
      <p className="presence-hint">La copie est ensuite rangée dans le dossier « copies-en-conflit » du projet, et ⌘Z annule la réunion.</p>
    </Dialog>
  );
}
