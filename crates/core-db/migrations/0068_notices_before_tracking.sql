-- A committed attempt that started before notice_sent_at existed (migration 64) had its notice
-- sent when its turn ended, untracked, so recovery must never send it again. It counts as sent
-- when tracking began. Attempts started after it keep their own record, sent or not.
UPDATE run_attempts
SET notice_sent_at = (
    SELECT strftime('%Y-%m-%dT%H:%M:%SZ', installed_on) FROM _sqlx_migrations WHERE version = 64
)
WHERE state = 'committed' AND notice_sent_at IS NULL
    AND julianday(started_at) < (
        SELECT julianday(installed_on) FROM _sqlx_migrations WHERE version = 64
    );
