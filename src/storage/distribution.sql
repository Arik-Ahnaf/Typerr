SELECT MIN(CAST(wpm / ?1 AS INTEGER), 11) AS bucket, COUNT(*)
FROM scores
GROUP BY bucket
ORDER BY bucket;
