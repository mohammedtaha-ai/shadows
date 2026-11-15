-- What began an attempt; earlier additions were all requested by the Planner.
ALTER TABLE run_attempts ADD COLUMN origin TEXT NOT NULL DEFAULT 'start'
    CHECK (origin IN ('start','planner','checks'));
UPDATE run_attempts SET origin = 'planner' WHERE addition IS NOT NULL;
-- Durable enqueue time, absent for an attempt superseded by a continuation.
ALTER TABLE run_attempts ADD COLUMN notice_sent_at TEXT NULL;
