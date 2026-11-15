//! Le verrou : savoir si l'autre personne a déjà le projet ouvert.
//!
//! Il n'y a pas d'édition simultanée. À l'ouverture, l'app écrit dans le
//! dossier projet un fichier `verrou.json` qui dit qui l'a ouvert (machine,
//! utilisateur, date). Tant que le projet est ouvert, elle le rafraîchit
//! régulièrement ; à la fermeture, elle l'efface.
//!
//! **Le verrou avertit, il ne garantit rien.** Il voyage par Dropbox, avec
//! quelques secondes de retard, voire beaucoup plus si un Mac est hors
//! ligne : deux personnes qui ouvrent le projet presque en même temps, ou
//! sans réseau, peuvent le prendre toutes les deux. Dropbox garde alors les
//! deux versions de `projet.json` (« copie en conflit »), rien n'est perdu,
//! mais il faut choisir à la main. Voir `docs/format-projet.md`.
//!
//! Règles :
//!
//! - pas de verrou : on le prend ;
//! - verrou de la même machine et du même utilisateur : on le reprend (l'app
//!   a planté ou a été fermée brutalement sur ce poste) ;
//! - verrou de quelqu'un d'autre, rafraîchi il y a plus de
//!   [`LOCK_STALE_AFTER_SECONDS`] : il est périmé, on le reprend ;
//! - verrou de quelqu'un d'autre, récent : refus, sauf ouverture forcée
//!   ([`LockMode::Force`]), décidée par l'utilisateur après avertissement.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use super::error::ProjectError;
use super::model::new_id;
use super::store::{write_atomic, ProjectDir};
use super::time::Timestamp;

/// Nom du fichier de verrou, dans le dossier projet.
pub const LOCK_FILE: &str = "verrou.json";

/// Rythme auquel l'app doit rafraîchir son verrou tant que le projet est
/// ouvert : toutes les deux minutes.
pub const LOCK_REFRESH_INTERVAL_SECONDS: u64 = 120;

/// Un verrou qui n'a pas été rafraîchi depuis un quart d'heure est périmé :
/// l'app qui le tenait a planté, ou le Mac est éteint ou en veille.
pub const LOCK_STALE_AFTER_SECONDS: u64 = 900;

/// Qui demande le verrou.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockIdentity {
    pub machine: String,
    pub user: String,
}

impl LockIdentity {
    pub fn new(machine: impl Into<String>, user: impl Into<String>) -> Self {
        Self {
            machine: machine.into(),
            user: user.into(),
        }
    }

    /// Le nom de cette machine et de l'utilisateur de la session.
    pub fn current() -> Self {
        let from_env = |name: &str| std::env::var(name).ok().filter(|v| !v.trim().is_empty());
        let machine = Command::new("hostname")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .filter(|name| !name.is_empty())
            .or_else(|| from_env("HOSTNAME"))
            .unwrap_or_else(|| "machine inconnue".to_string());
        let user = from_env("USER")
            .or_else(|| from_env("LOGNAME"))
            .unwrap_or_else(|| "utilisateur inconnu".to_string());
        Self { machine, user }
    }
}

/// Le contenu de `verrou.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockInfo {
    /// Identifiant de l'ouverture : distingue deux ouvertures successives
    /// par la même personne.
    pub session_id: String,
    pub machine: String,
    pub user: String,
    /// Moment de l'ouverture du projet.
    pub opened_at: Timestamp,
    /// Dernier signe de vie de l'app qui tient le verrou.
    pub refreshed_at: Timestamp,
    /// Version de l'app qui tient le verrou.
    #[serde(default)]
    pub app_version: String,
}

impl LockInfo {
    /// Vrai si le verrou n'a pas été rafraîchi depuis trop longtemps.
    pub fn is_stale(&self, now: Timestamp) -> bool {
        now.seconds_since(self.refreshed_at) > LOCK_STALE_AFTER_SECONDS
    }

    pub fn is_held_by(&self, identity: &LockIdentity) -> bool {
        self.machine == identity.machine && self.user == identity.user
    }
}

/// Que faire si quelqu'un d'autre tient un verrou récent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockMode {
    /// Refuser l'ouverture.
    Normal,
    /// Prendre le verrou quand même : l'utilisateur a été averti et assure
    /// que l'autre personne a fermé le projet.
    Force,
}

/// Comment le verrou a été obtenu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockOutcome {
    /// Le projet n'était ouvert par personne.
    Acquired,
    /// Un verrou laissé par ce même poste a été repris.
    ReclaimedOwn(LockInfo),
    /// Un verrou périmé de quelqu'un d'autre a été repris.
    ReclaimedStale(LockInfo),
    /// Un verrou récent de quelqu'un d'autre a été écarté de force.
    Forced(LockInfo),
}

/// Un verrou tenu sur un projet. Il est rendu par [`ProjectLock::release`],
/// ou à défaut quand la valeur disparaît.
#[derive(Debug)]
pub struct ProjectLock {
    file: PathBuf,
    info: LockInfo,
    released: bool,
}

impl ProjectLock {
    /// Prend le verrou du projet pour cette machine, maintenant.
    pub fn acquire(dir: &ProjectDir, mode: LockMode) -> Result<(Self, LockOutcome), ProjectError> {
        Self::acquire_at(dir, &LockIdentity::current(), Timestamp::now(), mode)
    }

    /// Prend le verrou au nom de `identity`, à l'instant `now`.
    pub fn acquire_at(
        dir: &ProjectDir,
        identity: &LockIdentity,
        now: Timestamp,
        mode: LockMode,
    ) -> Result<(Self, LockOutcome), ProjectError> {
        let outcome = match read_lock(dir)? {
            None => LockOutcome::Acquired,
            Some(holder) if holder.is_held_by(identity) => LockOutcome::ReclaimedOwn(holder),
            Some(holder) if holder.is_stale(now) => LockOutcome::ReclaimedStale(holder),
            Some(holder) if mode == LockMode::Force => LockOutcome::Forced(holder),
            Some(holder) => return Err(ProjectError::Locked { holder }),
        };
        let lock = Self {
            file: lock_file(dir),
            info: LockInfo {
                session_id: new_id(),
                machine: identity.machine.clone(),
                user: identity.user.clone(),
                opened_at: now,
                refreshed_at: now,
                app_version: env!("CARGO_PKG_VERSION").to_string(),
            },
            released: false,
        };
        lock.write()?;
        Ok((lock, outcome))
    }

    pub fn info(&self) -> &LockInfo {
        &self.info
    }

    /// Donne un signe de vie, maintenant. À appeler toutes les
    /// [`LOCK_REFRESH_INTERVAL_SECONDS`] et avant chaque enregistrement.
    pub fn refresh(&mut self) -> Result<(), ProjectError> {
        self.refresh_at(Timestamp::now())
    }

    /// Donne un signe de vie à l'instant `now`. Échoue avec
    /// [`ProjectError::LockLost`] si quelqu'un d'autre a pris le verrou
    /// entre-temps : il ne faut alors plus enregistrer sans s'être concerté.
    pub fn refresh_at(&mut self, now: Timestamp) -> Result<(), ProjectError> {
        if let Some(holder) = read_lock_file(&self.file)? {
            if holder.session_id != self.info.session_id {
                return Err(ProjectError::LockLost { holder });
            }
        }
        self.info.refreshed_at = now;
        self.write()
    }

    /// Rend le verrou : efface `verrou.json` s'il est toujours le nôtre.
    pub fn release(mut self) -> Result<(), ProjectError> {
        self.release_inner()
    }

    fn release_inner(&mut self) -> Result<(), ProjectError> {
        if self.released {
            return Ok(());
        }
        self.released = true;
        match read_lock_file(&self.file)? {
            Some(holder) if holder.session_id == self.info.session_id => {
                match fs::remove_file(&self.file) {
                    Ok(()) => Ok(()),
                    Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                    Err(e) => Err(ProjectError::io("d'effacer", &self.file)(e)),
                }
            }
            // Absent, ou repris par quelqu'un d'autre : on n'y touche pas.
            _ => Ok(()),
        }
    }

    fn write(&self) -> Result<(), ProjectError> {
        let mut text =
            serde_json::to_string_pretty(&self.info).map_err(|e| ProjectError::InvalidJson {
                path: self.file.clone(),
                message: format!("le verrou ne peut pas être écrit ({e})"),
            })?;
        text.push('\n');
        write_atomic(&self.file, text.as_bytes())
    }
}

impl Drop for ProjectLock {
    fn drop(&mut self) {
        let _ = self.release_inner();
    }
}

// --- Verrou tenu par l'app, sans valeur gardée en mémoire -------------------
//
// L'interface relit le projet à chaque commande : elle ne garde pas de
// [`ProjectLock`] entre deux appels. Elle garde seulement l'identifiant de
// son ouverture (`sessionId`) et appelle ces trois fonctions : prendre à
// l'ouverture, donner signe de vie toutes les deux minutes, rendre à la
// fermeture.

/// Ce que l'interface apprend en demandant le verrou.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum LockAttempt {
    /// Verrou pris. `session_id` est à rappeler pour le rafraîchir ou le
    /// rendre ; `replaced` est la personne dont le verrou a été repris
    /// (périmé, ou écarté de force), s'il y en avait une autre.
    #[serde(rename_all = "camelCase")]
    Taken {
        session_id: String,
        replaced: Option<LockInfo>,
    },
    /// Quelqu'un d'autre a le projet ouvert (verrou récent) ; rien n'est
    /// écrit.
    Busy { holder: LockInfo },
}

/// Où en est le verrou d'une ouverture, à chaque signe de vie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum LockCheck {
    /// Toujours le nôtre ; il vient d'être rafraîchi.
    Held,
    /// Quelqu'un d'autre l'a pris entre-temps.
    Lost { holder: LockInfo },
}

/// Prend le verrou et le laisse en place pour la durée de l'ouverture. Si
/// quelqu'un d'autre tient un verrou récent et que `force` est faux, rend
/// [`LockAttempt::Busy`].
pub fn take_lock(
    dir: &ProjectDir,
    identity: &LockIdentity,
    now: Timestamp,
    force: bool,
) -> Result<LockAttempt, ProjectError> {
    remove_lock_conflict_copies(dir);
    let mode = if force {
        LockMode::Force
    } else {
        LockMode::Normal
    };
    match ProjectLock::acquire_at(dir, identity, now, mode) {
        Ok((mut lock, outcome)) => {
            lock.released = true; // rendu plus tard par `give_back_lock`
            let replaced = match outcome {
                LockOutcome::ReclaimedStale(holder) | LockOutcome::Forced(holder) => Some(holder),
                LockOutcome::Acquired | LockOutcome::ReclaimedOwn(_) => None,
            };
            Ok(LockAttempt::Taken {
                session_id: lock.info.session_id.clone(),
                replaced,
            })
        }
        Err(ProjectError::Locked { holder }) => Ok(LockAttempt::Busy { holder }),
        Err(e) => Err(e),
    }
}

/// Signe de vie de l'ouverture `session_id`. Si le fichier a disparu (effacé
/// par Dropbox, ou par l'autre Mac après une prise de force puis sa
/// fermeture), le verrou est réécrit à notre nom.
pub fn keep_lock(
    dir: &ProjectDir,
    identity: &LockIdentity,
    session_id: &str,
    now: Timestamp,
) -> Result<LockCheck, ProjectError> {
    remove_lock_conflict_copies(dir);
    let file = lock_file(dir);
    let opened_at = match read_lock_file(&file)? {
        Some(holder) if holder.session_id != session_id => {
            return Ok(LockCheck::Lost { holder });
        }
        Some(holder) => holder.opened_at,
        None => now,
    };
    let lock = ProjectLock {
        file,
        info: LockInfo {
            session_id: session_id.to_string(),
            machine: identity.machine.clone(),
            user: identity.user.clone(),
            opened_at,
            refreshed_at: now,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
        },
        released: true,
    };
    lock.write()?;
    Ok(LockCheck::Held)
}

/// Rend le verrou de l'ouverture `session_id`, s'il est toujours le nôtre.
pub fn give_back_lock(dir: &ProjectDir, session_id: &str) -> Result<(), ProjectError> {
    let file = lock_file(dir);
    if let Some(holder) = read_lock_file(&file)? {
        if holder.session_id == session_id {
            match fs::remove_file(&file) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(ProjectError::io("d'effacer", &file)(e)),
            }
        }
    }
    Ok(())
}

/// Dropbox peut aussi garder deux versions du verrou (« verrou (Copie en
/// conflit…).json ») : elles ne disent rien d'utile, on les efface.
fn remove_lock_conflict_copies(dir: &ProjectDir) {
    let Ok(entries) = fs::read_dir(dir.root()) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("verrou (") && super::conflict::is_conflict_copy_name(&name) {
            let _ = fs::remove_file(entry.path());
        }
    }
}

fn lock_file(dir: &ProjectDir) -> PathBuf {
    dir.root().join(LOCK_FILE)
}

/// Lit le verrou d'un projet sans le prendre. `None` si personne ne l'a
/// ouvert.
pub fn read_lock(dir: &ProjectDir) -> Result<Option<LockInfo>, ProjectError> {
    read_lock_file(&lock_file(dir))
}

fn read_lock_file(file: &Path) -> Result<Option<LockInfo>, ProjectError> {
    let text = match fs::read_to_string(file) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(ProjectError::io("de lire", file)(e)),
    };
    if let Ok(info) = serde_json::from_str::<LockInfo>(&text) {
        return Ok(Some(info));
    }
    // Verrou illisible : on ne sait pas qui le tient. On le traite comme un
    // verrou d'un inconnu, daté de la dernière modification du fichier ; il
    // deviendra périmé comme les autres.
    let modified = fs::metadata(file)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| Timestamp::from_unix_seconds(d.as_secs()))
        .unwrap_or_else(Timestamp::now);
    Ok(Some(LockInfo {
        session_id: String::new(),
        machine: "machine inconnue".to_string(),
        user: "utilisateur inconnu".to_string(),
        opened_at: modified,
        refreshed_at: modified,
        app_version: String::new(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::test_support::TempDir;

    fn benjamin() -> LockIdentity {
        LockIdentity::new("MacBook-Pro-de-Benjamin.local", "benjamin")
    }

    fn marc() -> LockIdentity {
        LockIdentity::new("iMac-de-Marc.local", "marc")
    }

    fn at(seconds: u64) -> Timestamp {
        // 11 novembre 2026, 14 h 03 min 22 s UTC, plus `seconds`.
        Timestamp::from_unix_seconds(1_794_405_802 + seconds)
    }

    fn new_project(temp: &TempDir) -> ProjectDir {
        ProjectDir::create(temp.path().join("Essai.insitu"), None)
            .expect("création")
            .0
    }

    #[test]
    fn a_free_project_is_locked_and_the_file_says_by_whom() {
        let temp = TempDir::new("verrou-libre");
        let dir = new_project(&temp);
        assert_eq!(read_lock(&dir).expect("lecture"), None);

        let (lock, outcome) =
            ProjectLock::acquire_at(&dir, &benjamin(), at(0), LockMode::Normal).expect("verrou");
        assert_eq!(outcome, LockOutcome::Acquired);

        let text = fs::read_to_string(dir.root().join(LOCK_FILE)).expect("fichier");
        assert!(text.contains("\"machine\": \"MacBook-Pro-de-Benjamin.local\""));
        assert!(text.contains("\"user\": \"benjamin\""));
        assert!(text.contains("\"openedAt\": \"2026-11-11T14:03:22Z\""));
        assert_eq!(
            read_lock(&dir).expect("lecture").as_ref(),
            Some(lock.info())
        );
    }

    #[test]
    fn a_project_opened_by_someone_else_is_refused_with_their_name() {
        let temp = TempDir::new("verrou-pris");
        let dir = new_project(&temp);
        let (_held, _) =
            ProjectLock::acquire_at(&dir, &benjamin(), at(0), LockMode::Normal).expect("verrou");

        let error = ProjectLock::acquire_at(&dir, &marc(), at(60), LockMode::Normal)
            .expect_err("déjà ouvert");
        match &error {
            ProjectError::Locked { holder } => {
                assert_eq!(holder.user, "benjamin");
                assert_eq!(holder.machine, "MacBook-Pro-de-Benjamin.local");
            }
            other => panic!("erreur inattendue : {other}"),
        }
        let message = error.to_string();
        assert!(message.contains("déjà ouvert par benjamin"), "{message}");
        // Le refus n'a pas touché au verrou de Benjamin.
        assert_eq!(
            read_lock(&dir).expect("lecture").map(|info| info.user),
            Some("benjamin".to_string())
        );
    }

    #[test]
    fn releasing_frees_the_project_for_the_other_person() {
        let temp = TempDir::new("verrou-rendu");
        let dir = new_project(&temp);
        let (lock, _) =
            ProjectLock::acquire_at(&dir, &benjamin(), at(0), LockMode::Normal).expect("verrou");
        lock.release().expect("fermeture");
        assert!(!dir.root().join(LOCK_FILE).exists());
        let (_, outcome) =
            ProjectLock::acquire_at(&dir, &marc(), at(10), LockMode::Normal).expect("verrou");
        assert_eq!(outcome, LockOutcome::Acquired);
    }

    #[test]
    fn a_dropped_lock_is_released() {
        let temp = TempDir::new("verrou-abandon");
        let dir = new_project(&temp);
        {
            let _lock = ProjectLock::acquire_at(&dir, &benjamin(), at(0), LockMode::Normal)
                .expect("verrou");
        }
        assert_eq!(read_lock(&dir).expect("lecture"), None);
    }

    #[test]
    fn a_stale_lock_is_taken_over_but_a_fresh_one_is_not() {
        let temp = TempDir::new("verrou-perime");
        let dir = new_project(&temp);
        let (crashed, _) =
            ProjectLock::acquire_at(&dir, &benjamin(), at(0), LockMode::Normal).expect("verrou");
        // L'app de Benjamin plante : le verrou reste sur le disque.
        std::mem::forget(crashed);

        // À la limite exacte, le verrou tient encore.
        let error = ProjectLock::acquire_at(
            &dir,
            &marc(),
            at(LOCK_STALE_AFTER_SECONDS),
            LockMode::Normal,
        )
        .expect_err("pas encore périmé");
        assert!(matches!(error, ProjectError::Locked { .. }));

        // Une seconde plus tard, il est périmé : Marc le reprend.
        let (lock, outcome) = ProjectLock::acquire_at(
            &dir,
            &marc(),
            at(LOCK_STALE_AFTER_SECONDS + 1),
            LockMode::Normal,
        )
        .expect("verrou périmé repris");
        match outcome {
            LockOutcome::ReclaimedStale(previous) => assert_eq!(previous.user, "benjamin"),
            other => panic!("issue inattendue : {other:?}"),
        }
        assert_eq!(lock.info().user, "marc");
        assert_eq!(
            read_lock(&dir).expect("lecture").map(|info| info.user),
            Some("marc".to_string())
        );
    }

    #[test]
    fn refreshing_keeps_a_lock_alive() {
        let temp = TempDir::new("verrou-rafraichi");
        let dir = new_project(&temp);
        let (mut lock, _) =
            ProjectLock::acquire_at(&dir, &benjamin(), at(0), LockMode::Normal).expect("verrou");
        lock.refresh_at(at(800)).expect("signe de vie");
        // 1 000 s après l'ouverture, mais 200 s après le dernier signe de vie.
        let error = ProjectLock::acquire_at(&dir, &marc(), at(1000), LockMode::Normal)
            .expect_err("toujours ouvert");
        assert!(matches!(error, ProjectError::Locked { .. }));
        let info = read_lock(&dir).expect("lecture").expect("verrou présent");
        assert_eq!(info.opened_at, at(0));
        assert_eq!(info.refreshed_at, at(800));
    }

    #[test]
    fn the_same_machine_takes_back_its_own_lock_after_a_crash() {
        let temp = TempDir::new("verrou-meme-poste");
        let dir = new_project(&temp);
        let (crashed, _) =
            ProjectLock::acquire_at(&dir, &benjamin(), at(0), LockMode::Normal).expect("verrou");
        let old_session = crashed.info().session_id.clone();
        std::mem::forget(crashed);

        let (lock, outcome) =
            ProjectLock::acquire_at(&dir, &benjamin(), at(30), LockMode::Normal).expect("reprise");
        assert!(matches!(outcome, LockOutcome::ReclaimedOwn(_)));
        assert_ne!(lock.info().session_id, old_session);
    }

    #[test]
    fn forcing_takes_a_fresh_lock_and_the_first_holder_finds_out() {
        let temp = TempDir::new("verrou-force");
        let dir = new_project(&temp);
        let (mut first, _) =
            ProjectLock::acquire_at(&dir, &benjamin(), at(0), LockMode::Normal).expect("verrou");
        let (second, outcome) =
            ProjectLock::acquire_at(&dir, &marc(), at(60), LockMode::Force).expect("forcé");
        assert!(matches!(outcome, LockOutcome::Forced(_)));

        // Benjamin l'apprend à son prochain signe de vie…
        let error = first.refresh_at(at(120)).expect_err("verrou perdu");
        match &error {
            ProjectError::LockLost { holder } => assert_eq!(holder.user, "marc"),
            other => panic!("erreur inattendue : {other}"),
        }
        assert!(error.to_string().contains("ouvert entre-temps par marc"));
        // … et sa fermeture n'efface pas le verrou de Marc.
        first.release().expect("fermeture");
        assert_eq!(
            read_lock(&dir).expect("lecture").as_ref(),
            Some(second.info())
        );
    }

    #[test]
    fn an_unreadable_lock_counts_as_someone_unknown() {
        let temp = TempDir::new("verrou-illisible");
        let dir = new_project(&temp);
        fs::write(dir.root().join(LOCK_FILE), "{ \"machine\": ").expect("écriture");
        let now = Timestamp::now();
        let error = ProjectLock::acquire_at(&dir, &marc(), now, LockMode::Normal)
            .expect_err("verrou inconnu");
        assert!(matches!(error, ProjectError::Locked { .. }));
        // Il finit par être périmé comme un autre.
        let later = Timestamp::from_unix_seconds(now.unix_seconds() + LOCK_STALE_AFTER_SECONDS + 5);
        let (_, outcome) =
            ProjectLock::acquire_at(&dir, &marc(), later, LockMode::Normal).expect("repris");
        assert!(matches!(outcome, LockOutcome::ReclaimedStale(_)));
    }

    fn taken(attempt: Result<LockAttempt, ProjectError>) -> (String, Option<LockInfo>) {
        match attempt.expect("lecture") {
            LockAttempt::Taken {
                session_id,
                replaced,
            } => (session_id, replaced),
            LockAttempt::Busy { holder } => panic!("verrou tenu par {}", holder.user),
        }
    }

    #[test]
    fn the_app_takes_keeps_and_gives_back_its_lock_between_calls() {
        let temp = TempDir::new("verrou-app");
        let dir = new_project(&temp);
        let (session, replaced) = taken(take_lock(&dir, &benjamin(), at(0), false));
        assert_eq!(replaced, None);
        // Le verrou reste en place : Marc est averti.
        let busy = take_lock(&dir, &marc(), at(60), false).expect("lecture");
        assert!(matches!(busy, LockAttempt::Busy { ref holder } if holder.user == "benjamin"));
        // Signe de vie : toujours le nôtre, l'heure avance.
        assert_eq!(
            keep_lock(&dir, &benjamin(), &session, at(120)).expect("signe"),
            LockCheck::Held
        );
        let info = read_lock(&dir).expect("lecture").expect("verrou");
        assert_eq!(info.refreshed_at, at(120));
        assert_eq!(info.opened_at, at(0));
        // Rendu : le projet est libre.
        give_back_lock(&dir, &session).expect("rendu");
        assert_eq!(read_lock(&dir).expect("lecture"), None);
    }

    #[test]
    fn a_lock_taken_by_force_is_reported_as_lost_to_the_first_holder() {
        let temp = TempDir::new("verrou-app-force");
        let dir = new_project(&temp);
        let (first, _) = taken(take_lock(&dir, &marc(), at(0), false));
        let (second, replaced) = taken(take_lock(&dir, &benjamin(), at(30), true));
        assert_eq!(replaced.as_ref().map(|h| h.user.as_str()), Some("marc"));
        let check = keep_lock(&dir, &marc(), &first, at(120)).expect("signe");
        assert!(matches!(check, LockCheck::Lost { ref holder } if holder.user == "benjamin"));
        // Marc rend « son » verrou en fermant : celui de Benjamin reste.
        give_back_lock(&dir, &first).expect("rendu");
        assert_eq!(
            read_lock(&dir).expect("l").map(|h| h.session_id),
            Some(second)
        );
    }

    #[test]
    fn a_vanished_lock_is_written_again_and_lock_conflict_copies_are_removed() {
        let temp = TempDir::new("verrou-app-disparu");
        let dir = new_project(&temp);
        let (session, _) = taken(take_lock(&dir, &benjamin(), at(0), false));
        fs::remove_file(dir.root().join(LOCK_FILE)).expect("effacé");
        let copy = dir
            .root()
            .join("verrou (Copie en conflit de Marc 2026-11-15).json");
        fs::write(&copy, "{}").expect("copie");
        assert_eq!(
            keep_lock(&dir, &benjamin(), &session, at(120)).expect("signe"),
            LockCheck::Held
        );
        assert!(dir.root().join(LOCK_FILE).is_file());
        assert!(!copy.exists());
    }

    #[test]
    fn the_current_identity_is_never_empty() {
        let identity = LockIdentity::current();
        assert!(!identity.machine.is_empty());
        assert!(!identity.user.is_empty());
    }

    #[test]
    fn the_lock_file_is_not_part_of_the_project_description() {
        let temp = TempDir::new("verrou-a-part");
        let dir = new_project(&temp);
        let before = fs::read(dir.project_file()).expect("lecture");
        let (lock, _) =
            ProjectLock::acquire_at(&dir, &benjamin(), at(0), LockMode::Normal).expect("verrou");
        lock.release().expect("fermeture");
        assert_eq!(fs::read(dir.project_file()).expect("lecture"), before);
    }
}
