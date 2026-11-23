-- The last twelve calendar months through today, grouped by UTC completion day.
-- Fill missing dates without fetching individual tests or changing the schema.
WITH RECURSIVE bounds(first_day, last_day) AS (
    SELECT CAST(strftime('%s', date(?1 / 1000, 'unixepoch', '-1 year', 'floor', '+1 day')) AS INTEGER) / 86400,
           ?1 / 86400000
), days(day) AS (
    SELECT first_day FROM bounds
    UNION ALL
    SELECT day + 1 FROM days, bounds WHERE day < last_day
), counts AS (
    SELECT completed_at_ms / 86400000 AS day, COUNT(*) AS tests
    FROM scores, bounds
    WHERE completed_at_ms >= first_day * 86400000
      AND completed_at_ms < (last_day + 1) * 86400000
    GROUP BY day
)
SELECT days.day, date(days.day * 86400, 'unixepoch'), COALESCE(counts.tests, 0)
FROM days LEFT JOIN counts ON counts.day = days.day
ORDER BY days.day;
