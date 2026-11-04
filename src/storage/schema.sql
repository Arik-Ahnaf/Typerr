CREATE TABLE scores (
    id INTEGER PRIMARY KEY,
    completed_at_ms INTEGER NOT NULL CHECK (completed_at_ms >= 0),
    mode TEXT NOT NULL CHECK (mode IN ('words', 'time')),
    amount INTEGER NOT NULL CHECK (amount > 0),
    punctuation INTEGER NOT NULL CHECK (punctuation IN (0, 1)),
    numbers INTEGER NOT NULL CHECK (numbers IN (0, 1)),
    wpm REAL NOT NULL CHECK (wpm BETWEEN 0 AND 1.7976931348623157e308),
    raw_wpm REAL NOT NULL CHECK (raw_wpm BETWEEN 0 AND 1.7976931348623157e308),
    accuracy REAL NOT NULL CHECK (accuracy BETWEEN 0 AND 100),
    consistency REAL NOT NULL CHECK (consistency BETWEEN 0 AND 100),
    elapsed_seconds REAL NOT NULL CHECK (elapsed_seconds BETWEEN 0 AND 1.7976931348623157e308)
) STRICT;
CREATE INDEX scores_completed ON scores (completed_at_ms, id);
CREATE INDEX scores_best ON scores (mode, amount, wpm DESC, accuracy DESC, completed_at_ms, id);
PRAGMA user_version = 1;
