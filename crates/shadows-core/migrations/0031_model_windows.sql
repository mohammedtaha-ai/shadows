-- Retain the old context_window column unused to avoid a table rebuild.
UPDATE aylul_provider SET models = (
    SELECT json_group_array(json_object('id', value,
        'context_window', aylul_provider.context_window))
    FROM (SELECT value FROM json_each(aylul_provider.models) ORDER BY CAST(key AS INTEGER))
);
