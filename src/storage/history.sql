SELECT id, completed_at_ms, mode, amount, punctuation, numbers,
       wpm, raw_wpm, accuracy, consistency, elapsed_seconds
FROM scores
WHERE (?1 IS NULL OR mode = ?1)
  AND (?2 IS NULL OR amount = ?2)
  AND (?3 IS NULL OR completed_at_ms >= ?3)
  AND (?4 IS NULL OR completed_at_ms < ?4)
ORDER BY completed_at_ms, id;
