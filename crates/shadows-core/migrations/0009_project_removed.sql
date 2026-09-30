-- A person can remove a project they no longer need (spec §4.2, §15.4).
--
-- The row is never deleted: `durable_event.project_id` references it ON DELETE
-- RESTRICT and the journal is never erased. A removed project has `removed_at`
-- set, and every read of a project leaves it out, so it answers NotFound. Its
-- slug stays taken (the column's UNIQUE is unchanged): adding the folder again
-- needs another slug.
ALTER TABLE project ADD COLUMN removed_at TEXT NULL;
