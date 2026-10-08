-- §23.8 (amends §12.2): a tool line and a subagent card are their own entry
-- kinds; a card's structured payload is a column, not text in the body.
ALTER TABLE thread_entry ADD COLUMN card_json TEXT NULL;

-- `[tool: <title>]` becomes ToolCall, its body the title. '[tool: ' is 7
-- characters.
UPDATE thread_entry
   SET kind = 'ToolCall',
       body = substr(body, 8, length(body) - 8)
 WHERE kind = 'AgentMessage' AND body LIKE '[tool: %]';

-- `[subagent: <json>]` becomes Subagent, its body the card's title.
-- '[subagent: ' is 11 characters. A body whose JSON is not valid is left as
-- it was.
UPDATE thread_entry
   SET kind = 'Subagent',
       card_json = substr(body, 12, length(body) - 12),
       body = coalesce(json_extract(substr(body, 12, length(body) - 12), '$.title'), '')
 WHERE kind = 'AgentMessage' AND body LIKE '[subagent: %]'
   AND json_valid(substr(body, 12, length(body) - 12));
