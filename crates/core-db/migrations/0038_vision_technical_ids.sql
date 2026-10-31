-- Technical direction ids read TD<n>, so a vision item never reads like a plan task's T<n>.
-- Counters stay: they count items, not prefixes.
UPDATE design_workspace SET vision_content = json_set(vision_content, '$.technical_direction',
    (SELECT json_group_array(json(CASE WHEN json_extract(v.value, '$.id') GLOB 'T[0-9]*'
        THEN json_set(v.value, '$.id', 'TD' || substr(json_extract(v.value, '$.id'), 2))
        ELSE v.value END))
       FROM (SELECT value FROM json_each(design_workspace.vision_content, '$.technical_direction')
              ORDER BY key) v))
 WHERE json_type(vision_content, '$.technical_direction') = 'array';

UPDATE design_vision_version SET content_json = json_set(content_json, '$.technical_direction',
    (SELECT json_group_array(json(CASE WHEN json_extract(v.value, '$.id') GLOB 'T[0-9]*'
        THEN json_set(v.value, '$.id', 'TD' || substr(json_extract(v.value, '$.id'), 2))
        ELSE v.value END))
       FROM (SELECT value FROM json_each(design_vision_version.content_json, '$.technical_direction')
              ORDER BY key) v))
 WHERE json_type(content_json, '$.technical_direction') = 'array';

UPDATE design_vision_proposal SET item_id = 'TD' || substr(item_id, 2)
 WHERE item_id GLOB 'T[0-9]*';
