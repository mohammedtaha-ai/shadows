-- Why the judge accepted a task despite a check without a passed confirmation.
ALTER TABLE run_task_marks ADD COLUMN reason TEXT NULL;
