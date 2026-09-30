-- Where a thread's current title came from (spec §4.2, §6.4). A title is
-- replaced only by a source allowed to replace the one it has:
--   'client'         the name the client gave at create: every row before this
--   'plan'           the title of the plan a draft from scratch created the
--                    thread for (§13.6), which nothing automatic replaces
--   'first_message'  the first line of the thread's first message
--   'harness'        the title the harness generated for its session (§12.3)
--   'person'         a name a person chose, which nothing automatic replaces.
--                    No rename writes it yet.
ALTER TABLE planning_thread ADD COLUMN title_source TEXT NOT NULL DEFAULT 'client'
    CHECK (title_source IN ('client', 'plan', 'first_message', 'harness', 'person'));

-- A draft from scratch wrote the thread and its v1 in one write, with one
-- timestamp and one title: those threads are the plan's, not the client's.
UPDATE planning_thread SET title_source = 'plan'
 WHERE EXISTS (SELECT 1 FROM workflow w
                WHERE w.thread_id = planning_thread.id AND w.version = 1
                  AND w.created_at = planning_thread.created_at
                  AND w.title = planning_thread.title);
