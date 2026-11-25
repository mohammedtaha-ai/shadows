-- A recheck the person or a Planner asked for: asked is 1, cause_task is 0, and
-- asker_thread_id is the asking Planner's thread (NULL when the person asked).
ALTER TABLE run_rechecks ADD COLUMN asked INTEGER NOT NULL DEFAULT 0;
ALTER TABLE run_rechecks ADD COLUMN asker_thread_id TEXT;
