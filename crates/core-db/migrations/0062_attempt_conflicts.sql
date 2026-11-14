-- A contract conflict an executor reported on its own attempt: what it
-- expected, what it observed and its evidence, when it was accepted, whether
-- the Planner's notice was enqueued, and that notice's payload as first
-- resolved, so a retry sends the same one. Attempts before this, and attempts
-- with no conflict, keep every field null.
ALTER TABLE run_attempts ADD COLUMN conflict_json TEXT
    CHECK (conflict_json IS NULL OR json_valid(conflict_json));
ALTER TABLE run_attempts ADD COLUMN conflict_at TEXT;
ALTER TABLE run_attempts ADD COLUMN conflict_delivery TEXT
    CHECK (conflict_delivery IS NULL
           OR conflict_delivery IN ('pending', 'delivered', 'no_planner'));
ALTER TABLE run_attempts ADD COLUMN conflict_notice_json TEXT
    CHECK (conflict_notice_json IS NULL OR json_valid(conflict_notice_json));
