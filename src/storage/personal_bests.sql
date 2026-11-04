WITH ranked AS (
    SELECT *, ROW_NUMBER() OVER (
        PARTITION BY mode, amount
        ORDER BY wpm DESC, accuracy DESC, completed_at_ms, id
    ) AS rank
    FROM scores
    WHERE (mode = 'words' AND amount IN (10, 25, 50, 100))
       OR (mode = 'time' AND amount IN (15, 30, 60, 120))
)
SELECT id, completed_at_ms, mode, amount, punctuation, numbers,
       wpm, raw_wpm, accuracy, consistency, elapsed_seconds
FROM ranked WHERE rank = 1
ORDER BY mode, amount;
