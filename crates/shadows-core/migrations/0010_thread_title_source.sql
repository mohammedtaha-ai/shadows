-- Where a thread's current title came from (spec §4.2, §6.4). A title is
-- replaced only by a source allowed to replace the one it has:
--   'client'         the name the client gave at create: every row before this
--   'first_message'  the first line of the thread's first message
--   'harness'        the title the harness generated for its session (§12.3)
--   'person'         a name a person chose, which nothing automatic replaces.
--                    No rename writes it yet.
ALTER TABLE planning_thread ADD COLUMN title_source TEXT NOT NULL DEFAULT 'client'
    CHECK (title_source IN ('client', 'first_message', 'harness', 'person'));
