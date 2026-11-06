-- The framework scaffold owns starter files. Keep map row order and the
-- content_json string representation while dropping the obsolete field.
UPDATE design_part SET content_json = json_remove(content_json, '$.files');

WITH rows_of(project_id, number, key, value, inner) AS (
    SELECT design_map_version.project_id, design_map_version.number, element.key,
           element.value, json_extract(element.value, '$.content_json')
    FROM design_map_version, json_each(design_map_version.parts_json) AS element
), rewritten(project_id, number, parts_json) AS (
    SELECT project_id, number,
           json_group_array(
               json_set(value, '$.content_json',
                   json_quote(json_remove(json(inner), '$.files') || ''))
               ORDER BY key
           )
    FROM rows_of GROUP BY project_id, number
)
UPDATE design_map_version SET parts_json = COALESCE(
    (SELECT parts_json FROM rewritten
     WHERE rewritten.project_id = design_map_version.project_id
       AND rewritten.number = design_map_version.number),
    design_map_version.parts_json
);
