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
