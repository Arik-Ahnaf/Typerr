SELECT COUNT(*), COALESCE(SUM(elapsed_seconds), 0.0),
       AVG(wpm), MAX(wpm), AVG(accuracy), MAX(accuracy)
FROM scores;
