-- A part owns one folder and its layers. The v1 folders list becomes that one
-- folder: its first entry, or the part's title slug when it had none. Every
-- layer starts empty, and a pending map proposal is superseded so the Planner
-- proposes again under the new shape.
UPDATE design_part
SET content_json = json_set(
        json_remove(
            json_set(
                content_json,
                '$.folder',
                json_quote(COALESCE(
                    json_extract(content_json, '$.folders[0]'),
                    lower(replace(trim(COALESCE(json_extract(content_json, '$.title'), '')), ' ', '-'))
                ))
            ),
            '$.folders'
        ),
        '$.layers', json('[]')
    );

-- A map version holds its rows as an array, and each row holds its part as a
-- JSON string. The string is rewritten the same way, and the rows keep their
-- order.
WITH rows_of(project_id, number, key, value, inner) AS (
    SELECT design_map_version.project_id, design_map_version.number, element.key,
           element.value,
           COALESCE(json_extract(element.value, '$.content_json'), '{}')
    FROM design_map_version, json_each(design_map_version.parts_json) AS element
),
rewritten(project_id, number, parts_json) AS (
    SELECT project_id, number,
           json_group_array(
               json_set(
                   value,
                   '$.content_json',
                   -- `|| ''` drops the JSON subtype, so json_quote escapes the
                   -- new text instead of passing it through as JSON.
                   json_quote((
                       json_set(
                           json_set(
                               json_remove(json(inner), '$.folders'),
                               '$.folder',
                               json_quote(COALESCE(
                                   json_extract(json(inner), '$.folders[0]'),
                                   lower(replace(trim(COALESCE(json_extract(json(inner), '$.title'), '')), ' ', '-'))
                               ))
                           ),
                           '$.layers', json('[]')
                       ) || ''
                   ))
               )
               ORDER BY key
           )
    FROM rows_of
    GROUP BY project_id, number
)
UPDATE design_map_version
SET parts_json = COALESCE(
        (SELECT parts_json FROM rewritten
         WHERE rewritten.project_id = design_map_version.project_id
           AND rewritten.number = design_map_version.number),
        design_map_version.parts_json
    );

-- The Planner proposes again under the new shape; the old card is history.
UPDATE design_map_proposal SET status='Superseded' WHERE status='Pending';
