use std::{
    ffi::OsString,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc::{self, Sender},
    thread::{self, JoinHandle},
    time::{SystemTime, UNIX_EPOCH},
};

use super::{CompletedTest, DashboardData, Error, Result, ScoreStore, TestMode};
use crate::{statistics::Score, words::Options};

/// Resolve once at startup, independent of working/executable directories.
pub fn database_path() -> Result<PathBuf> {
    data_path(cfg!(target_os = "windows"), |name| std::env::var_os(name))
}

fn data_path(windows: bool, env: impl Fn(&str) -> Option<OsString>) -> Result<PathBuf> {
    let absolute = |name| {
        env(name)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
    };
    if windows {
        absolute("LOCALAPPDATA").map(|root| root.join("Typerr/scores.db"))
    } else {
        absolute("XDG_DATA_HOME")
            .or_else(|| absolute("HOME").map(|root| root.join(".local/share")))
            .map(|root| root.join("typerr/scores.db"))
    }
    .ok_or(Error::Invalid(
        "cannot resolve an absolute user data directory for scores.db",
    ))
}

/// Owns the single writer thread. Dropping closes the queue, drains it, and joins.
pub struct ScoreWriter {
    sender: Option<Sender<Request>>,
    thread: Option<JoinHandle<()>>,
    path: PathBuf,
}

enum Request {
    Save(CompletedTest),
    Dashboard(Sender<Result<DashboardData>>),
}

impl ScoreWriter {
    pub fn start_default() -> Option<Self> {
        match database_path() {
            Ok(path) => match Self::start(path.clone()) {
                Ok(writer) => Some(writer),
                Err(error) => {
                    report(
                        Some(&path),
                        &format!("could not initialize score history: {error}"),
                    );
                    None
                }
            },
            Err(error) => {
                report(None, &error.to_string());
                None
            }
        }
    }

    pub fn start(path: PathBuf) -> Result<Self> {
        // Finish initialization before accepting input; subsequent I/O stays here.
        let store = ScoreStore::open(&path)?;
        let (sender, receiver) = mpsc::channel::<Request>();
        let worker_path = path.clone();
        let thread = thread::Builder::new()
            .name("score-history".into())
            .spawn(move || {
                for request in receiver {
                    let test = match request {
                        Request::Save(test) => test,
                        Request::Dashboard(reply) => {
                            let result = store.dashboard();
                            if let Err(error) = &result {
                                report(
                                    Some(&worker_path),
                                    &format!("could not read Dashboard: {error}"),
                                );
                            }
                            let _ = reply.send(result);
                            continue;
                        }
                    };
                    if let Err(error) = store.insert(&test) {
                        report(
                            Some(&worker_path),
                            &format!(
                                "could not save completed test at {} ({} {}): {error}",
                                test.completed_at_ms,
                                test.mode.as_str(),
                                test.amount,
                            ),
                        );
                    }
                }
            })?;
        Ok(Self {
            sender: Some(sender),
            thread: Some(thread),
            path,
        })
    }

    /// Snapshot only summary fields; graph vectors are never cloned or queued.
    pub fn save(&self, score: &Score, mode: TestMode, amount: u16, options: Options) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|duration| i64::try_from(duration.as_millis()).ok());
        let Some(timestamp) = timestamp else {
            report(
                Some(&self.path),
                "could not save score: wall clock is outside the supported Unix timestamp range",
            );
            return;
        };
        let test = CompletedTest::from_score(score, timestamp, mode, amount, options);
        if self
            .sender
            .as_ref()
            .is_none_or(|sender| sender.send(Request::Save(test)).is_err())
        {
            report(
                Some(&self.path),
                "could not save score: storage worker disconnected",
            );
        }
    }

    /// Ordered after earlier saves; the UI polls the reply without blocking input.
    pub fn dashboard(&self) -> mpsc::Receiver<Result<DashboardData>> {
        let (reply, receiver) = mpsc::channel();
        if self
            .sender
            .as_ref()
            .is_none_or(|sender| sender.send(Request::Dashboard(reply)).is_err())
        {
            report(
                Some(&self.path),
                "could not read Dashboard: storage worker disconnected",
            );
        }
        receiver
    }
}

impl Drop for ScoreWriter {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            report(Some(&self.path), "score storage worker panicked");
        }
    }
}

fn report(path: Option<&Path>, message: &str) {
    let location = path
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let line = format!("[{timestamp}] Typerr score history ({location}): {message}");
    eprintln!("{line}");
    if let Some(parent) = path.and_then(Path::parent)
        && let Ok(mut log) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(parent.join("scores-errors.log"))
    {
        let _ = writeln!(log, "{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_respect_absolute_xdg_and_fall_back_to_home() {
        let env = |name: &str| match name {
            "HOME" => Some(OsString::from("/home/test")),
            "XDG_DATA_HOME" => Some(OsString::from("relative")),
            _ => None,
        };
        // Unix absolute paths are tested on Unix; Windows path parsing is native.
        #[cfg(unix)]
        {
            assert_eq!(
                data_path(false, env).unwrap(),
                PathBuf::from("/home/test/.local/share/typerr/scores.db")
            );
            assert_eq!(
                data_path(false, |_| Some("/data".into())).unwrap(),
                PathBuf::from("/data/typerr/scores.db")
            );
        }
        #[cfg(windows)]
        assert_eq!(
            data_path(true, |_| Some(r"C:\Users\test\AppData\Local".into())).unwrap(),
            PathBuf::from(r"C:\Users\test\AppData\Local\Typerr\scores.db")
        );
        let _ = env;
        assert!(data_path(false, |_| None).is_err());
        assert!(data_path(true, |_| None).is_err());
    }

    #[test]
    fn saving_returns_while_another_connection_holds_the_write_lock() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scores.db");
        let writer = ScoreWriter::start(path.clone()).unwrap();
        let blocker = rusqlite::Connection::open(&path).unwrap();
        blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
        writer.save(&Score::default(), TestMode::Words, 10, Options::default());
        // A synchronous insert would time out before reaching this unlock and
        // lose the result. The queued insert can proceed after save returns.
        blocker.execute_batch("ROLLBACK").unwrap();
        drop(writer);
        assert_eq!(
            ScoreStore::open(&path)
                .unwrap()
                .summary()
                .unwrap()
                .completed_tests,
            1
        );
        assert!(!dir.path().join("scores-errors.log").exists());
    }

    #[test]
    fn shutdown_drains_queue_and_failed_write_does_not_stop_later_saves() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scores.db");
        let writer = ScoreWriter::start(path.clone()).unwrap();
        writer.save(
            &Score {
                wpm: f64::NAN,
                ..Score::default()
            },
            TestMode::Words,
            10,
            Options::default(),
        );
        for _ in 0..20 {
            writer.save(&Score::default(), TestMode::Time, 15, Options::default());
        }
        drop(writer);
        assert_eq!(
            ScoreStore::open(&path)
                .unwrap()
                .summary()
                .unwrap()
                .completed_tests,
            20
        );
        assert!(
            std::fs::read_to_string(dir.path().join("scores-errors.log"))
                .unwrap()
                .contains("must be finite")
        );
    }
}
