ALTER TABLE run_attempts ADD COLUMN notice_due INTEGER NOT NULL DEFAULT 0
    CHECK (notice_due IN (0, 1));
