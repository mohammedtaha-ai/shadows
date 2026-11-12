-- A waiting message an executor's attempt sends the Planner through Shadows carries the
-- executor's thread and its card as JSON; NULL for the person's own messages. Owned by Turns.
ALTER TABLE queued_message ADD COLUMN notice_json TEXT;
