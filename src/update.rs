//! Startup release checks and explicit, user-accepted updates. Independent of Iced.
pub mod archive;
pub mod install;
pub mod release;
use release::{Distribution, Release};
use std::{
    fs,
    path::PathBuf,
    process::ChildStdin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    time::Duration,
};
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    Checking,
    Available,
    Downloading { received: u64, total: u64 },
    Verifying,
    Installing,
    Failed(String),
    Dismissed,
    Restarting,
}
enum Reply {
    Checked(std::result::Result<Option<(Release, Distribution)>, String>),
    Progress(u64, u64),
    Verifying,
    InstallReady(Sender<bool>),
    Handoff(ChildStdin),
    Failed(String),
}
pub struct Updater {
    pub phase: Phase,
    pub visible: bool,
    pub focus_primary: bool,
    pub release: Option<Release>,
    kind: Option<Distribution>,
    lease: Option<install::Lease>,
    confirmation: Option<PathBuf>,
    reply: Option<Receiver<Reply>>,
    cancel: Arc<AtomicBool>,
    handoff: Option<ChildStdin>,
}
impl Default for Updater {
    fn default() -> Self {
        Self {
            phase: Phase::Dismissed,
            visible: false,
            focus_primary: false,
            release: None,
            kind: None,
            lease: None,
            confirmation: None,
            reply: None,
            cancel: Arc::new(AtomicBool::new(false)),
            handoff: None,
        }
    }
}
impl Updater {
    pub fn start(startup: install::Startup) -> Self {
        let error = startup.error;
        let mut this = Self {
            lease: startup.lease,
            confirmation: startup.confirmation,
            ..Self::default()
        };
        if let Some(error) = error {
            this.phase = Phase::Failed(error);
            this.visible = true;
        } else {
            this.check();
        }
        this
    }
    fn check(&mut self) {
        let Some(lease) = &self.lease else {
            return;
        };
        let root = lease.root.clone();
        let (tx, rx) = mpsc::channel();
        self.reply = Some(rx);
        self.phase = Phase::Checking;
        std::thread::spawn(move || {
            let result = (|| {
                let kind = install::distribution(&root)?;
                Ok(release::check(kind)?.map(|release| (release, kind)))
            })();
            let _ = tx.send(Reply::Checked(result.map_err(|e: Error| e.to_string())));
        });
    }
    pub fn pending(&self) -> bool {
        self.reply.is_some() || self.confirmation.is_some()
    }
    pub fn confirm_startup(&mut self) {
        if let Some(work) = self.confirmation.take() {
            // Called on the first native redraw, after Iced and score storage initialize.
            if let Err(error) =
                fs::File::create_new(work.join("confirmed")).and_then(|f| f.sync_all())
            {
                diagnostic(&format!("Could not confirm updated startup: {error}"));
            }
        }
    }
    pub fn poll(&mut self, safe: bool) {
        loop {
            let reply = match self.reply.as_ref().map(Receiver::try_recv) {
                Some(Ok(reply)) => reply,
                Some(Err(mpsc::TryRecvError::Disconnected)) => {
                    self.reply = None;
                    if let Some(lease) = &self.lease {
                        lease.shared();
                    }
                    if !matches!(
                        self.phase,
                        Phase::Dismissed | Phase::Restarting | Phase::Available | Phase::Failed(_)
                    ) {
                        self.phase = Phase::Failed(
                            "Update worker stopped unexpectedly. Retry or close this dialog."
                                .into(),
                        );
                        self.visible = true;
                    }
                    break;
                }
                _ => break,
            };
            match reply {
                Reply::Checked(result) => {
                    self.reply = None;
                    match result {
                        Ok(Some((release, kind))) => {
                            self.release = Some(release);
                            self.kind = Some(kind);
                            self.phase = Phase::Available;
                        }
                        Ok(None) => {
                            self.phase = Phase::Dismissed;
                            self.visible = false;
                        }
                        Err(error) => {
                            log_diagnostic(&format!("Startup update check: {error}"));
                            self.phase = Phase::Dismissed;
                            self.visible = false;
                        }
                    }
                }
                Reply::Progress(received, total) => {
                    self.phase = Phase::Downloading { received, total }
                }
                Reply::Verifying => self.phase = Phase::Verifying,
                Reply::InstallReady(ack) => {
                    if self.cancel.load(Ordering::Relaxed) {
                        let _ = ack.send(false);
                    } else {
                        self.phase = Phase::Installing;
                        let _ = ack.send(true);
                    }
                }
                Reply::Handoff(pipe) => {
                    self.handoff = Some(pipe);
                    self.phase = Phase::Restarting;
                    self.reply = None;
                }
                Reply::Failed(error) => {
                    self.reply = None;
                    if let Some(lease) = &self.lease {
                        lease.shared();
                    }
                    if self.cancel.load(Ordering::Relaxed) {
                        self.phase = Phase::Dismissed;
                        self.visible = false;
                    } else {
                        diagnostic(&error);
                        self.phase = Phase::Failed(error);
                    }
                }
            }
        }
        if safe && self.phase == Phase::Available {
            self.visible = true;
        }
    }
    pub fn dismiss(&mut self) {
        match self.phase {
            Phase::Available | Phase::Failed(_) => {
                self.visible = false;
                self.phase = Phase::Dismissed;
            }
            Phase::Downloading { .. } | Phase::Verifying => {
                self.cancel.store(true, Ordering::Relaxed);
            }
            _ => {}
        }
    }
    pub fn accept(&mut self) {
        if !matches!(self.phase, Phase::Available | Phase::Failed(_)) {
            return;
        }
        let Some(release) = self.release.clone() else {
            self.visible = false;
            self.check();
            return;
        };
        let Some(lease) = &self.lease else {
            return;
        };
        if let Err(error) = lease.exclusive() {
            self.phase = Phase::Failed(error.to_string());
            return;
        }
        let root = lease.root.clone();
        let kind = self.kind.expect("release has distribution");
        let (tx, rx) = mpsc::channel();
        self.reply = Some(rx);
        self.cancel = Arc::new(AtomicBool::new(false));
        let cancel = self.cancel.clone();
        self.phase = Phase::Downloading {
            received: 0,
            total: release.archive.size,
        };
        self.focus_primary = false;
        std::thread::spawn(move || {
            if let Err(error) = prepare(&root, kind, &release, &cancel, &tx) {
                let _ = tx.send(Reply::Failed(error.to_string()));
            }
        });
    }
    pub fn shutdown(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.lease.take();
        self.handoff.take();
    }
    pub fn cancelling(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
            && matches!(self.phase, Phase::Downloading { .. } | Phase::Verifying)
    }
    pub fn busy(&self) -> bool {
        matches!(self.phase, Phase::Installing | Phase::Restarting)
    }
}

fn cancelled(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        Err("Update cancelled".into())
    } else {
        Ok(())
    }
}
fn prepare(
    root: &std::path::Path,
    kind: Distribution,
    release: &Release,
    cancel: &AtomicBool,
    tx: &Sender<Reply>,
) -> Result<()> {
    let work = install::prepare_work(root, kind)?;
    let outcome = (|| {
        let checksum_path = work.join("SHA256SUMS");
        release::download(&release.checksums, &checksum_path, cancel, |_, _| {})?;
        let expected = release::checksum(&fs::read(&checksum_path)?, &release.archive.name)?;
        let archive = work.join(&release.archive.name);
        let actual = release::download(&release.archive, &archive, cancel, |received, total| {
            let _ = tx.send(Reply::Progress(received, total));
        })?;
        if actual != expected {
            return Err("Release archive checksum mismatch; nothing was installed.".into());
        }
        let _ = tx.send(Reply::Verifying);
        cancelled(cancel)?;
        if kind != Distribution::Pacman {
            archive::extract(
                &archive,
                &work.join("payload"),
                &release.version.to_string(),
                kind,
            )?;
        }
        cancelled(cancel)?;
        let token = install::journal(
            root,
            &work,
            &release.version.to_string(),
            kind == Distribution::Pacman,
        )?;
        let (ack, ready) = mpsc::channel();
        tx.send(Reply::InstallReady(ack))
            .map_err(|_| "Application closed before installation")?;
        if !ready.recv().unwrap_or(false) {
            return Err("Update cancelled".into());
        }
        install::copy_helper(&work)?;
        if kind == Distribution::Pacman {
            install::install_package(&archive, &release.version.to_string(), |command| {
                archive::output_timeout(command, Duration::from_secs(600))
            })?;
        }
        let pipe = install::launch_helper(root, &work, &token)?;
        let _ = tx.send(Reply::Handoff(pipe));
        Ok(())
    })();
    if outcome.is_err() && kind != Distribution::Pacman {
        install::cleanup(&work);
    }
    outcome
}
pub fn diagnostic(message: &str) {
    eprintln!("Typerr updater: {message}");
    log_diagnostic(message);
}
fn log_diagnostic(message: &str) {
    #[cfg(test)]
    let _ = message;
    #[cfg(not(test))]
    use std::io::Write;
    #[cfg(not(test))]
    if let Ok(path) = crate::storage::database_path()
        && let Some(parent) = path.parent()
    {
        let _ = fs::create_dir_all(parent);
        if let Ok(mut file) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(parent.join("updates-errors.log"))
        {
            let _ = writeln!(file, "{message}");
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_and_deferred_prompt() {
        let mut u = Updater::default();
        assert!(!u.pending());
        u.phase = Phase::Available;
        u.poll(false);
        assert!(!u.visible);
        u.poll(true);
        assert!(u.visible);
        u.dismiss();
        u.poll(true);
        assert!(!u.visible);
    }
    #[test]
    fn cancellation_does_not_interrupt_install() {
        let mut u = Updater {
            phase: Phase::Downloading {
                received: 1,
                total: 2,
            },
            ..Updater::default()
        };
        u.dismiss();
        assert!(u.cancel.load(Ordering::Relaxed));
        u.cancel.store(false, Ordering::Relaxed);
        u.phase = Phase::Installing;
        u.dismiss();
        assert!(!u.cancel.load(Ordering::Relaxed));
    }
    #[test]
    fn background_check_failures_are_quiet() {
        for error in ["offline", "timeout", "GitHub returned 403", "invalid JSON"] {
            let mut updater = Updater::default();
            let (tx, rx) = mpsc::channel();
            updater.reply = Some(rx);
            updater.phase = Phase::Checking;
            tx.send(Reply::Checked(Err(error.into()))).unwrap();
            updater.poll(true);
            assert_eq!(updater.phase, Phase::Dismissed);
            assert!(!updater.visible);
            assert!(!updater.pending());
        }
    }
}
