-- A fork is born locked to its source's harness (spec §12.6, §12.9): its
-- fork_session_id belongs to that harness, and no other could continue it.
-- Replaces 0005's lock, which held only from a thread's first operation.
DROP TRIGGER planning_thread_harness_locked;

CREATE TRIGGER planning_thread_harness_locked
BEFORE UPDATE OF harness_kind ON planning_thread
WHEN NEW.harness_kind <> OLD.harness_kind
 AND (OLD.forked_from_thread IS NOT NULL
      OR EXISTS (SELECT 1 FROM operation WHERE thread_id = OLD.id))
BEGIN SELECT RAISE(ABORT, 'harness_locked'); END;
