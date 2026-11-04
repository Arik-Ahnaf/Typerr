use std::time::{Duration, Instant};

use rusqlite::Connection;
use typerr::{
    statistics::Score,
    storage::{CompletedTest, Error, HistoryFilter, ScoreStore, Summary, TestMode},
    words::Options,
};

fn sample() -> CompletedTest {
    CompletedTest::from_score(
        &Score {
            wpm: 80.125,
            raw: 85.375,
            accuracy: 94.25,
            consistency: 78.625,
            elapsed: Duration::from_micros(12_345_678),
            samples: vec![],
        },
        1_793_664_000_123,
        TestMode::Words,
        50,
        Options {
            punctuation: true,
            numbers: true,
        },
    )
}

#[test]
fn creates_directories_and_schema_and_preserves_fractional_scores_on_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("data/typerr/scores.db");
    let store = ScoreStore::open(&path).unwrap();
    assert!(path.is_file());
    assert_eq!(store.summary().unwrap(), Summary::default());
    assert!(store.personal_bests().unwrap().is_empty());
    assert!(store.history(HistoryFilter::default()).unwrap().is_empty());
    let test = sample();
    let id = store.insert(&test).unwrap();
    drop(store);
    let store = ScoreStore::open(&path).unwrap();
    let rows = store.history(HistoryFilter::default()).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].test, test);
    assert_eq!(rows[0].id, id);
    let db = Connection::open(path).unwrap();
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE type='index' AND tbl_name='scores'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        2
    );
}

#[test]
fn dashboard_uses_per_test_averages_and_completed_durations() {
    let store = ScoreStore::in_memory().unwrap();
    let first = CompletedTest {
        wpm: 20.,
        accuracy: 100.,
        elapsed_seconds: 15.,
        ..sample()
    };
    let second = CompletedTest {
        wpm: 80.,
        accuracy: 50.,
        elapsed_seconds: 60.,
        ..sample()
    };
    store.insert(&first).unwrap();
    store.insert(&second).unwrap();
    assert_eq!(
        store.summary().unwrap(),
        Summary {
            completed_tests: 2,
            typing_seconds: 75.,
            average_wpm: Some(50.),
            highest_wpm: Some(80.),
            average_accuracy: Some(75.),
            highest_accuracy: Some(100.),
        }
    );
}

#[test]
fn bests_cover_eight_categories_and_keep_accuracy_from_winning_test() {
    let store = ScoreStore::in_memory().unwrap();
    let mut winners = vec![];
    for (mode, amounts) in [
        (TestMode::Time, [15, 30, 60, 120]),
        (TestMode::Words, [10, 25, 50, 100]),
    ] {
        for amount in amounts {
            let mut test = CompletedTest {
                mode,
                amount,
                wpm: 70.,
                accuracy: 100.,
                ..sample()
            };
            store.insert(&test).unwrap();
            test.wpm = 80.;
            test.accuracy = 85.;
            test.punctuation = false;
            test.numbers = false;
            store.insert(&test).unwrap();
            test.accuracy = 90.; // Accuracy breaks equal-speed ties.
            store.insert(&test).unwrap();
            test.completed_at_ms -= 1; // Earlier completion wins the next tie.
            test.numbers = true; // Options do not split rankings.
            winners.push(store.insert(&test).unwrap());
            store.insert(&test).unwrap(); // Earlier ID wins an otherwise exact tie.
        }
    }
    let bests = store.personal_bests().unwrap();
    assert_eq!(bests.iter().map(|row| row.id).collect::<Vec<_>>(), winners);
    assert!(
        bests
            .iter()
            .all(|row| row.test.accuracy == 90. && row.test.numbers)
    );
    store
        .insert(&CompletedTest {
            amount: 500,
            wpm: 1000.,
            ..sample()
        })
        .unwrap();
    assert_eq!(store.personal_bests().unwrap().len(), 8);
}

#[test]
fn history_is_chronological_and_filters_use_half_open_dates() {
    let store = ScoreStore::in_memory().unwrap();
    let tests = [
        CompletedTest {
            completed_at_ms: 300,
            mode: TestMode::Time,
            amount: 30,
            ..sample()
        },
        CompletedTest {
            completed_at_ms: 100,
            ..sample()
        },
        CompletedTest {
            completed_at_ms: 200,
            ..sample()
        },
        CompletedTest {
            completed_at_ms: 200,
            amount: 10,
            ..sample()
        },
    ];
    for test in &tests {
        store.insert(test).unwrap();
    }
    let ids = |filter| {
        store
            .history(filter)
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(HistoryFilter::default()), [2, 3, 4, 1]);
    assert_eq!(
        ids(HistoryFilter {
            mode: Some(TestMode::Words),
            ..Default::default()
        }),
        [2, 3, 4]
    );
    assert_eq!(
        ids(HistoryFilter {
            amount: Some(10),
            ..Default::default()
        }),
        [4]
    );
    assert_eq!(
        ids(HistoryFilter {
            from_ms: Some(200),
            until_ms: Some(300),
            ..Default::default()
        }),
        [3, 4]
    );
    assert_eq!(
        ids(HistoryFilter {
            mode: Some(TestMode::Words),
            amount: Some(50),
            from_ms: Some(200),
            until_ms: Some(300)
        }),
        [3]
    );
    assert!(
        ids(HistoryFilter {
            from_ms: Some(301),
            ..Default::default()
        })
        .is_empty()
    );
}

#[test]
fn rejects_invalid_metrics_and_sql_constraints_protect_direct_inserts() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("scores.db");
    let store = ScoreStore::open(&path).unwrap();
    for invalid in [
        CompletedTest {
            wpm: -1.,
            ..sample()
        },
        CompletedTest {
            raw_wpm: -1.,
            ..sample()
        },
        CompletedTest {
            wpm: f64::INFINITY,
            ..sample()
        },
        CompletedTest {
            accuracy: f64::NAN,
            ..sample()
        },
        CompletedTest {
            accuracy: 100.1,
            ..sample()
        },
        CompletedTest {
            consistency: -1.,
            ..sample()
        },
        CompletedTest {
            elapsed_seconds: -1.,
            ..sample()
        },
        CompletedTest {
            amount: 0,
            ..sample()
        },
        CompletedTest {
            completed_at_ms: -1,
            ..sample()
        },
    ] {
        assert!(store.insert(&invalid).is_err());
    }
    let db = Connection::open(&path).unwrap();
    for (mode, punctuation, numbers) in [("other", 0, 0), ("words", 2, 0), ("words", 0, -1)] {
        assert!(
            db.execute(
                "INSERT INTO scores VALUES (NULL, 0, ?1, 10, ?2, ?3, 0, 0, 0, 0, 0)",
                rusqlite::params![mode, punctuation, numbers]
            )
            .is_err()
        );
    }
    assert!(
        db.execute(
            "INSERT INTO scores VALUES (NULL, 0, 'words', 10, 0, 0, NULL, 0, 0, 0, 0)",
            []
        )
        .is_err()
    );
    assert_eq!(store.summary().unwrap().completed_tests, 0);
    store
        .insert(&CompletedTest::from_score(
            &Score::default(),
            0,
            TestMode::Words,
            10,
            Options::default(),
        ))
        .unwrap();
}

#[test]
fn incompatible_and_corrupt_databases_are_not_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("scores.db");
    let store = ScoreStore::open(&path).unwrap();
    store.insert(&sample()).unwrap();
    drop(store);
    let db = Connection::open(&path).unwrap();
    db.pragma_update(None, "user_version", 99).unwrap();
    drop(db);
    let before = std::fs::read(&path).unwrap();
    assert!(matches!(
        ScoreStore::open(&path),
        Err(Error::SchemaVersion(99))
    ));
    assert_eq!(std::fs::read(&path).unwrap(), before);

    let foreign = dir.path().join("foreign.db");
    Connection::open(&foreign)
        .unwrap()
        .execute_batch(
            "CREATE TABLE unrelated (value TEXT); INSERT INTO unrelated VALUES ('keep me');",
        )
        .unwrap();
    let before = std::fs::read(&foreign).unwrap();
    assert!(ScoreStore::open(&foreign).is_err());
    assert_eq!(std::fs::read(&foreign).unwrap(), before);

    let corrupt = dir.path().join("corrupt.db");
    std::fs::write(&corrupt, b"not a sqlite database").unwrap();
    assert!(ScoreStore::open(&corrupt).is_err());
    assert_eq!(std::fs::read(&corrupt).unwrap(), b"not a sqlite database");
    assert!(ScoreStore::open(&corrupt.join("scores.db")).is_err());
}

#[test]
fn locked_database_times_out_and_accepts_later_writes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("scores.db");
    let store = ScoreStore::open(&path).unwrap();
    let blocker = Connection::open(&path).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
    let start = Instant::now();
    assert!(store.insert(&sample()).is_err());
    assert!(start.elapsed() < Duration::from_secs(3));
    assert!(ScoreStore::open(&path).is_err());
    blocker.execute_batch("ROLLBACK").unwrap();
    assert_eq!(store.summary().unwrap().completed_tests, 0);
    store.insert(&sample()).unwrap();
    assert_eq!(store.summary().unwrap().completed_tests, 1);
}

#[cfg(unix)]
#[test]
fn readonly_database_is_preserved() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("scores.db");
    let store = ScoreStore::open(&path).unwrap();
    store.insert(&sample()).unwrap();
    drop(store);
    let before = std::fs::read(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444)).unwrap();
    assert!(ScoreStore::open(&path).is_err());
    assert_eq!(std::fs::read(path).unwrap(), before);
}

#[test]
fn dashboard_snapshot_bins_fractional_boundaries_and_extreme_speeds() {
    let store = ScoreStore::in_memory().unwrap();
    let empty = store.dashboard().unwrap();
    assert_eq!(empty.summary.completed_tests, 0);
    assert_eq!(empty.bins, [0; 12]);
    assert!(empty.personal_bests.is_empty());
    for wpm in [0., 9.999, 10., 79.9, 80., 119.999] {
        store.insert(&CompletedTest { wpm, ..sample() }).unwrap();
    }
    let data = store.dashboard().unwrap();
    assert_eq!(data.bin_width, 10.);
    assert_eq!(data.bins, [2, 1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1]);
    assert_eq!(data.summary.completed_tests, data.bins.iter().sum::<u64>());
    assert_eq!(data.personal_bests[0].test.wpm, 119.999);
    store
        .insert(&CompletedTest {
            wpm: 120.,
            ..sample()
        })
        .unwrap();
    let widened = store.dashboard().unwrap();
    assert_eq!(widened.bin_width, 20.);
    assert_eq!(widened.bins.iter().sum::<u64>(), 7);
    assert_eq!(widened.bins[6], 1);
    store
        .insert(&CompletedTest {
            wpm: f64::MAX,
            ..sample()
        })
        .unwrap();
    let extreme = store.dashboard().unwrap();
    assert!(extreme.bin_width.is_finite());
    assert_eq!(extreme.bins.iter().sum::<u64>(), 8);
}

#[test]
fn dashboard_worker_snapshot_follows_queued_saves_without_reopening() {
    use typerr::storage::ScoreWriter;
    let directory = tempfile::tempdir().unwrap();
    let writer = ScoreWriter::start(directory.path().join("scores.db")).unwrap();
    for _ in 0..25 {
        writer.save(
            &Score {
                wpm: 85.,
                accuracy: 96.,
                ..Score::default()
            },
            TestMode::Time,
            30,
            Options::default(),
        );
    }
    let data = writer
        .dashboard()
        .recv_timeout(Duration::from_secs(2))
        .unwrap()
        .unwrap();
    assert_eq!(data.summary.completed_tests, 25);
    assert_eq!(data.bins[8], 25);
    assert_eq!(data.personal_bests[0].test.accuracy, 96.);
    writer.save(&Score::default(), TestMode::Words, 10, Options::default());
    assert_eq!(
        writer
            .dashboard()
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap()
            .summary
            .completed_tests,
        26
    );
}
