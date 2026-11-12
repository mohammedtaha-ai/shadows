-- An executor commits its own work and reports that commit. `start_head` is the commit HEAD
-- named when the attempt started (NULL on a branch with no commit yet): a reported commit
-- already reachable from it is not the attempt's work. `report` is the executor's own
-- account of what it did, ran and found, stored with the accepted commit.
ALTER TABLE run_attempts ADD COLUMN start_head TEXT NULL;
ALTER TABLE run_attempts ADD COLUMN report TEXT NULL;
