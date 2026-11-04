# Local score history

Typerr automatically opens or creates `scores.db` at startup:

- Linux: `$XDG_DATA_HOME/typerr/scores.db`, or
  `$HOME/.local/share/typerr/scores.db` when XDG_DATA_HOME is missing, empty, or
  relative. HOME must be absolute for the fallback.
- Windows: `%LOCALAPPDATA%\Typerr\scores.db` (an absolute directory is required).

The executable's directory and working directory do not affect the location.
Portable builds use the same user data location as installed builds. SQLite is
compiled into the executable; users do not need SQLite installed. The database
is local, unencrypted, and never included in packages or source control.

## Records and lifecycle

The schema is in `src/storage/schema.sql`; `PRAGMA user_version` is 1.
Initialization and the version check run in one immediate transaction, including
on simultaneous first launches. Existing version-1 history is preserved; unknown
versions, nonempty unversioned databases, read-only files, and corrupt files are
reported without replacing their contents. Future migrations must preserve data.

Each completed test has an integer primary key, completion timestamp in UTC Unix
milliseconds, `words`/`time` mode, selected amount, punctuation/numbers booleans,
full-precision WPM/raw WPM, accuracy/consistency percentages, and elapsed seconds.
The wall-clock timestamp records when completion is handled; duration remains
monotonic and freezes at the exact Time deadline, even if redraw is delayed.
Values are not rounded before storage. Graph samples, target text, profile data,
and incomplete attempts are not stored.

`App::new` initializes a `ScoreWriter`; `App::default` is deliberately storage-free
for tests. The Typing-to-Transition boundary queues one immutable summary from
the frozen Score. Redraws, Next, Retry, and Tab cannot duplicate that record.
Tab during the fade keeps the completed result; completing a retry saves a new
record. Full-buffer exhaustion and zero-duration completions also save.

One standard-library worker thread owns the connection and processes the queue
in order. A SQLite insert is atomic. The normal shutdown path closes the queue
and joins the worker after queued writes finish. An abrupt process termination
or power loss can lose pending in-memory records. SQLite uses its default rollback
journal; `scores.db-journal` may exist while writing. Close Typerr before copying
the database for a simple backup.

Lock waits are limited to 250 ms per database operation. On a failed save, the
worker logs the error and continues processing subsequent records; it does not
retry or silently mark the failed record as saved. Initialization failure disables
saving for that launch without disabling typing. Errors go to stderr and, when
the data directory is writable, `scores-errors.log` beside the database. Correct
the cause and relaunch after an initialization failure. Existing results are
never deleted as an error-recovery action.

## SQL and Rust query interface

`typerr::storage::ScoreStore` exposes `open`, `in_memory`, `insert`, `summary`,
`personal_bests`, `history`, and `dashboard`. Results are typed (`CompletedTest`, `StoredScore`,
`Summary`, `HistoryFilter`) and errors are returned to the caller. Tests use
temporary directories or in-memory stores, never the default user database.

- `insert.sql`: ten positional parameters in column order: timestamp, mode,
  amount, punctuation, numbers, WPM, raw WPM, accuracy, consistency, duration.
- `summary.sql`: completed count, sum of completed durations, arithmetic mean
  and maximum WPM, arithmetic mean and maximum accuracy. Every completed test
  has equal weight in averages. Empty history has zero count/time and NULL metrics.
- `personal_bests.sql`: one winning row per Time 15/30/60/120 or Words
  10/25/50/100. Options are combined. Rank by descending WPM, descending accuracy,
  then ascending completion timestamp and ID. All returned fields, including
  accuracy, come from that winning row. Categories without tests return no row.
- `history.sql`: chronological summaries ordered by timestamp then ID. Optional
  parameters are mode, amount, inclusive starting timestamp, and exclusive ending
  timestamp. NULL means no filter. Combined filters intersect; an empty/reversed
  time range returns no rows. History remains available for chronological summary queries.

- `distribution.sql`: parameterized WPM-range counts over all completed tests,
  including zero WPM. Dashboard chooses twelve ranges with a minimum width of
  ten WPM, widening in multiples of ten when needed.

Schema constraints enforce nonnegative finite metrics/durations, percentages in
0–100, positive amounts, valid modes, required fields, and boolean option flags.
`CompletedTest.amount` is a `u16`; callers use the app's existing allowed amounts.
Completion-order and mode/amount/ranking indexes support the read queries.

`ScoreWriter::dashboard` queues a read after prior saves and returns a receiver.
The GUI polls without blocking, only while a reply is pending. `ScoreStore::dashboard`
returns summary, bests, and histogram in one read transaction; it does not load
all history records or open another connection. Reads do not change the schema.
A failed query is logged to stderr and the existing local log. Dashboard renders
N/A for unavailable/empty data and refreshes on each opening. Profile editing,
profile persistence, and historical per-test Score graphs remain excluded.
