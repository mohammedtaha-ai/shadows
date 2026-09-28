-- Operation transition events are scoped to their operation's thread.
--
-- Before this migration only `OperationCreated` carried `thread_id`; every
-- later transition (Started, Completed, Failed, CancellationRequested,
-- Cancelled, Interrupted) was written with only `operation_id`, so no thread's
-- replay or live stream could select it (spec §2.10). New rows carry the thread
-- from the write path; this backfills the rows already written. An operation
-- with no thread leaves its events' thread NULL.
UPDATE durable_event
   SET thread_id = (SELECT o.thread_id FROM operation o WHERE o.id = durable_event.operation_id)
 WHERE thread_id IS NULL
   AND operation_id IS NOT NULL;
