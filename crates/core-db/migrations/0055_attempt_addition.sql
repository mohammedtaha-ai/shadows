-- An attempt that carries on another in its session holds the Planner's addition;
-- NULL for an attempt Start began.
ALTER TABLE run_attempts ADD COLUMN addition TEXT NULL;
