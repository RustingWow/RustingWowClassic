CREATE TYPE character_map AS ENUM (
    'eastern_kingdoms',
    'kalimdor',
    'deeprun_tram'
);

ALTER TABLE race_start_positions
    ALTER COLUMN map_id TYPE character_map
    USING (CASE map_id
        WHEN 0 THEN 'eastern_kingdoms'::character_map
        WHEN 1 THEN 'kalimdor'::character_map
        WHEN 369 THEN 'deeprun_tram'::character_map
    END);

ALTER TABLE characters
    ALTER COLUMN map_id TYPE character_map
    USING (CASE map_id
        WHEN 0 THEN 'eastern_kingdoms'::character_map
        WHEN 1 THEN 'kalimdor'::character_map
        WHEN 369 THEN 'deeprun_tram'::character_map
    END);
