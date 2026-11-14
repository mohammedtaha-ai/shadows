-- The task context an attempt received, saved once in the write that admits it.
-- Attempts recorded before this table have no row; none is reconstructed.
CREATE TABLE run_attempt_context (
    attempt_id TEXT PRIMARY KEY REFERENCES run_attempts(id) ON DELETE CASCADE,
    context_json TEXT NOT NULL CHECK (json_valid(context_json)),
    captured_at TEXT NOT NULL
);

CREATE TRIGGER run_attempt_context_fixed BEFORE UPDATE ON run_attempt_context
BEGIN
    SELECT RAISE(ABORT, 'a saved attempt context never changes');
END;
