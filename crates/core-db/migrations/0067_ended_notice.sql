-- The committed or failed attempt's Planner notice as first resolved, which every retry sends
-- again unchanged; NULL until it is resolved.
ALTER TABLE run_attempts ADD COLUMN ended_notice_json TEXT NULL
    CHECK (ended_notice_json IS NULL OR json_valid(ended_notice_json));

-- The attempt a done mark judged, the only one its replay may still free; NULL for a mark that
-- judged none.
ALTER TABLE run_task_marks ADD COLUMN judged_attempt_id TEXT NULL
    REFERENCES run_attempts(id) ON DELETE SET NULL;
