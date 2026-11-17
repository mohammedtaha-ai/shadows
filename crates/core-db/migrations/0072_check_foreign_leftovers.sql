ALTER TABLE run_checks ADD COLUMN foreign_leftovers INTEGER NOT NULL DEFAULT 0
    CHECK (foreign_leftovers IN (0,1));
ALTER TABLE run_recheck_results ADD COLUMN foreign_leftovers INTEGER NOT NULL DEFAULT 0
    CHECK (foreign_leftovers IN (0,1));
