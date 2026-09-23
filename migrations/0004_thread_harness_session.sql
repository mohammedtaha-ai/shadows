-- A planning thread remembers the harness session its turns continue
-- (spec §4.2, §6.4; docs/evidence/harness/SERVE_STREAM_SPIKE.md Finding 3).
--
-- NULL until a turn on the thread has reached the harness's turn-end result:
-- only then is the session known to exist in the harness's own store, so that
-- `--resume` will accept it. Existing threads keep NULL, and their next turn
-- starts a new session — the one thing no migration can recover is what the
-- model remembered from turns whose session was never recorded.
ALTER TABLE planning_thread ADD COLUMN harness_session_id TEXT NULL;
