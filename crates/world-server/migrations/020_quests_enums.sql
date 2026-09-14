ALTER TABLE quests ALTER COLUMN required_races DROP DEFAULT;
ALTER TABLE quests ALTER COLUMN required_classes DROP DEFAULT;
ALTER TABLE quests ALTER COLUMN quest_type DROP DEFAULT;

CREATE TYPE quest_type AS ENUM (
    'DUNGEON',
    'ELITE',
    'ESCORT',
    'LEGENDARY',
    'LIFE',
    'NONE',
    'PVP',
    'RAID',
    'WORLD_EVENT'
);

ALTER TABLE quests
    ALTER COLUMN required_races TYPE character_race[]
    USING ARRAY_REMOVE(ARRAY[
        CASE WHEN required_races & 1 <> 0 THEN 'HUMAN'::character_race END,
        CASE WHEN required_races & 2 <> 0 THEN 'ORC'::character_race END,
        CASE WHEN required_races & 4 <> 0 THEN 'DWARF'::character_race END,
        CASE WHEN required_races & 8 <> 0 THEN 'NIGHT_ELF'::character_race END,
        CASE WHEN required_races & 16 <> 0 THEN 'UNDEAD'::character_race END,
        CASE WHEN required_races & 32 <> 0 THEN 'TAUREN'::character_race END,
        CASE WHEN required_races & 64 <> 0 THEN 'GNOME'::character_race END,
        CASE WHEN required_races & 128 <> 0 THEN 'TROLL'::character_race END
    ], NULL);

ALTER TABLE quests
    ALTER COLUMN required_classes TYPE character_class[]
    USING ARRAY_REMOVE(ARRAY[
        CASE WHEN required_classes & 1 <> 0 THEN 'WARRIOR'::character_class END,
        CASE WHEN required_classes & 2 <> 0 THEN 'PALADIN'::character_class END,
        CASE WHEN required_classes & 4 <> 0 THEN 'HUNTER'::character_class END,
        CASE WHEN required_classes & 8 <> 0 THEN 'ROGUE'::character_class END,
        CASE WHEN required_classes & 16 <> 0 THEN 'PRIEST'::character_class END,
        CASE WHEN required_classes & 64 <> 0 THEN 'SHAMAN'::character_class END,
        CASE WHEN required_classes & 128 <> 0 THEN 'MAGE'::character_class END,
        CASE WHEN required_classes & 256 <> 0 THEN 'WARLOCK'::character_class END,
        CASE WHEN required_classes & 1024 <> 0 THEN 'DRUID'::character_class END
    ], NULL);

ALTER TABLE quests
    ALTER COLUMN quest_type TYPE quest_type
    USING (CASE quest_type
        WHEN 1 THEN 'ELITE'::quest_type
        WHEN 21 THEN 'LIFE'::quest_type
        WHEN 41 THEN 'PVP'::quest_type
        WHEN 62 THEN 'RAID'::quest_type
        WHEN 81 THEN 'DUNGEON'::quest_type
        WHEN 82 THEN 'WORLD_EVENT'::quest_type
        WHEN 83 THEN 'LEGENDARY'::quest_type
        WHEN 84 THEN 'ESCORT'::quest_type
        ELSE 'NONE'::quest_type
    END);

ALTER TABLE quests ALTER COLUMN required_races SET DEFAULT '{}';
ALTER TABLE quests ALTER COLUMN required_classes SET DEFAULT '{}';
ALTER TABLE quests ALTER COLUMN quest_type SET DEFAULT 'NONE';
