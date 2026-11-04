//! Local completed-test history. SQL and filesystem I/O stay outside scoring/UI.
use std::{fmt, path::Path, time::Duration};

use rusqlite::{Connection, TransactionBehavior, params};

use crate::statistics::Score;

mod worker;
pub use worker::{ScoreWriter, database_path};

/// A lock may delay the storage worker, never the typing event handler.
const BUSY_TIMEOUT: Duration = Duration::from_millis(250);

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Sql(rusqlite::Error),
    SchemaVersion(i64),
    Invalid(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => error.fmt(f),
            Self::Sql(error) => error.fmt(f),
            Self::SchemaVersion(version) => {
                write!(f, "unsupported scores schema version {version}")
            }
            Self::Invalid(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Sql(error) => Some(error),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sql(error)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestMode {
    Words,
    Time,
}

impl TestMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Words => "words",
            Self::Time => "time",
        }
    }
}

/// A snapshot of a completed test, without target text or graph samples.
#[derive(Debug, Clone, PartialEq)]
pub struct CompletedTest {
    pub completed_at_ms: i64,
    pub mode: TestMode,
    pub amount: u16,
    pub punctuation: bool,
    pub numbers: bool,
    pub wpm: f64,
    pub raw_wpm: f64,
    pub accuracy: f64,
    pub consistency: f64,
    pub elapsed_seconds: f64,
}

impl CompletedTest {
    pub fn from_score(
        score: &Score,
        completed_at_ms: i64,
        mode: TestMode,
        amount: u16,
        options: crate::words::Options,
    ) -> Self {
        Self {
            completed_at_ms,
            mode,
            amount,
            punctuation: options.punctuation,
            numbers: options.numbers,
            wpm: score.wpm,
            raw_wpm: score.raw,
            accuracy: score.accuracy,
            consistency: score.consistency,
            elapsed_seconds: score.elapsed.as_secs_f64(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredScore {
    pub id: i64,
    pub test: CompletedTest,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Summary {
    pub completed_tests: u64,
    pub typing_seconds: f64,
    pub average_wpm: Option<f64>,
    pub highest_wpm: Option<f64>,
    pub average_accuracy: Option<f64>,
    pub highest_accuracy: Option<f64>,
}

/// One consistent Dashboard read, including an all-time WPM histogram.
#[derive(Debug, Default, Clone)]
pub struct DashboardData {
    pub summary: Summary,
    pub personal_bests: Vec<StoredScore>,
    pub bin_width: f64,
    pub bins: [u64; 12],
}

/// UTC range is start-inclusive and end-exclusive. None means no restriction.
#[derive(Debug, Default, Clone, Copy)]
pub struct HistoryFilter {
    pub mode: Option<TestMode>,
    pub amount: Option<u16>,
    pub from_ms: Option<i64>,
    pub until_ms: Option<i64>,
}

pub struct ScoreStore {
    connection: Connection,
}

impl ScoreStore {
    pub fn dashboard(&self) -> Result<DashboardData> {
        let transaction = self.connection.unchecked_transaction()?;
        let summary = self.summary()?;
        // Keep twelve readable ranges, widening by multiples of ten as needed.
        let bin_width = ((summary.highest_wpm.unwrap_or(0.) / 120.).floor() + 1.) * 10.;
        let personal_bests = self.personal_bests()?;
        let mut bins = [0; 12];
        {
            let mut statement = self
                .connection
                .prepare(include_str!("storage/distribution.sql"))?;
            let rows = statement.query_map([bin_width], |row| {
                Ok((row.get::<_, i64>(0)? as usize, row.get::<_, i64>(1)? as u64))
            })?;
            for row in rows {
                let (index, count) = row?;
                bins[index.min(11)] = count;
            }
        }
        transaction.commit()?;
        Ok(DashboardData {
            summary,
            personal_bests,
            bin_width,
            bins,
        })
    }

    /// SQLite opens or creates the file atomically; no separate exists/create race.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        Self::initialize(Connection::open(path)?)
    }

    pub fn in_memory() -> Result<Self> {
        Self::initialize(Connection::open_in_memory()?)
    }

    fn initialize(mut connection: Connection) -> Result<Self> {
        if connection.is_readonly("main")? {
            return Err(Error::Invalid("scores database is read-only"));
        }
        connection.busy_timeout(BUSY_TIMEOUT)?;
        // Serialize first-launch initialization, including the version check.
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let version =
            transaction.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))?;
        match version {
            0 => {
                let tables: i64 = transaction.query_row(
                    "SELECT COUNT(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
                    [],
                    |row| row.get(0),
                )?;
                if tables != 0 {
                    return Err(Error::Invalid(
                        "refusing to initialize a nonempty unversioned database",
                    ));
                }
                transaction.execute_batch(include_str!("storage/schema.sql"))?;
            }
            1 => {
                // Detect a missing/incompatible table without changing existing data.
                transaction.prepare(include_str!("storage/history.sql"))?;
            }
            other => return Err(Error::SchemaVersion(other)),
        }
        transaction.commit()?;
        Ok(Self { connection })
    }

    pub fn insert(&self, test: &CompletedTest) -> Result<i64> {
        if [
            test.wpm,
            test.raw_wpm,
            test.accuracy,
            test.consistency,
            test.elapsed_seconds,
        ]
        .iter()
        .any(|value| !value.is_finite())
        {
            return Err(Error::Invalid("score metrics must be finite"));
        }
        self.connection.execute(
            include_str!("storage/insert.sql"),
            params![
                test.completed_at_ms,
                test.mode.as_str(),
                test.amount,
                test.punctuation,
                test.numbers,
                test.wpm,
                test.raw_wpm,
                test.accuracy,
                test.consistency,
                test.elapsed_seconds,
            ],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn summary(&self) -> Result<Summary> {
        Ok(self
            .connection
            .query_row(include_str!("storage/summary.sql"), [], |row| {
                Ok(Summary {
                    completed_tests: row.get::<_, i64>(0)? as u64,
                    typing_seconds: row.get(1)?,
                    average_wpm: row.get(2)?,
                    highest_wpm: row.get(3)?,
                    average_accuracy: row.get(4)?,
                    highest_accuracy: row.get(5)?,
                })
            })?)
    }

    /// Accuracy and every other field belong to the winning WPM test.
    pub fn personal_bests(&self) -> Result<Vec<StoredScore>> {
        let mut statement = self
            .connection
            .prepare(include_str!("storage/personal_bests.sql"))?;
        Ok(statement
            .query_map([], read_score)?
            .collect::<rusqlite::Result<_>>()?)
    }

    pub fn history(&self, filter: HistoryFilter) -> Result<Vec<StoredScore>> {
        let mut statement = self
            .connection
            .prepare(include_str!("storage/history.sql"))?;
        Ok(statement
            .query_map(
                params![
                    filter.mode.map(TestMode::as_str),
                    filter.amount,
                    filter.from_ms,
                    filter.until_ms,
                ],
                read_score,
            )?
            .collect::<rusqlite::Result<_>>()?)
    }
}

fn read_score(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredScore> {
    let mode = match row.get::<_, String>(2)?.as_str() {
        "words" => TestMode::Words,
        "time" => TestMode::Time,
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    Ok(StoredScore {
        id: row.get(0)?,
        test: CompletedTest {
            completed_at_ms: row.get(1)?,
            mode,
            amount: row.get(3)?,
            punctuation: row.get(4)?,
            numbers: row.get(5)?,
            wpm: row.get(6)?,
            raw_wpm: row.get(7)?,
            accuracy: row.get(8)?,
            consistency: row.get(9)?,
            elapsed_seconds: row.get(10)?,
        },
    })
}
