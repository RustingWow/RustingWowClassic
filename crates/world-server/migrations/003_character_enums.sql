CREATE TYPE character_race AS ENUM (
    'human',
    'orc',
    'dwarf',
    'night_elf',
    'undead',
    'tauren',
    'gnome',
    'troll'
);

CREATE TYPE character_class AS ENUM (
    'warrior',
    'paladin',
    'hunter',
    'rogue',
    'priest',
    'shaman',
    'mage',
    'warlock',
    'druid'
);

CREATE TYPE character_gender AS ENUM (
    'male',
    'female'
);

CREATE TYPE character_area AS ENUM (
    'northshire_valley',
    'coldridge_valley',
    'shadowglen',
    'deathknell',
    'camp_narache',
    'valley_of_trials'
);

ALTER TABLE race_start_positions
    ALTER COLUMN race TYPE character_race
    USING (CASE race
        WHEN 1 THEN 'human'::character_race
        WHEN 2 THEN 'orc'::character_race
        WHEN 3 THEN 'dwarf'::character_race
        WHEN 4 THEN 'night_elf'::character_race
        WHEN 5 THEN 'undead'::character_race
        WHEN 6 THEN 'tauren'::character_race
        WHEN 7 THEN 'gnome'::character_race
        WHEN 8 THEN 'troll'::character_race
    END),
    ALTER COLUMN area TYPE character_area
    USING (CASE area
        WHEN 9 THEN 'northshire_valley'::character_area
        WHEN 132 THEN 'coldridge_valley'::character_area
        WHEN 188 THEN 'shadowglen'::character_area
        WHEN 154 THEN 'deathknell'::character_area
        WHEN 221 THEN 'camp_narache'::character_area
        WHEN 363 THEN 'valley_of_trials'::character_area
    END);

ALTER TABLE characters
    ALTER COLUMN race TYPE character_race
    USING (CASE race
        WHEN 1 THEN 'human'::character_race
        WHEN 2 THEN 'orc'::character_race
        WHEN 3 THEN 'dwarf'::character_race
        WHEN 4 THEN 'night_elf'::character_race
        WHEN 5 THEN 'undead'::character_race
        WHEN 6 THEN 'tauren'::character_race
        WHEN 7 THEN 'gnome'::character_race
        WHEN 8 THEN 'troll'::character_race
    END),
    ALTER COLUMN class TYPE character_class
    USING (CASE class
        WHEN 1 THEN 'warrior'::character_class
        WHEN 2 THEN 'paladin'::character_class
        WHEN 3 THEN 'hunter'::character_class
        WHEN 4 THEN 'rogue'::character_class
        WHEN 5 THEN 'priest'::character_class
        WHEN 7 THEN 'shaman'::character_class
        WHEN 8 THEN 'mage'::character_class
        WHEN 9 THEN 'warlock'::character_class
        WHEN 11 THEN 'druid'::character_class
    END),
    ALTER COLUMN gender TYPE character_gender
    USING (CASE gender
        WHEN 0 THEN 'male'::character_gender
        WHEN 1 THEN 'female'::character_gender
    END),
    ALTER COLUMN area TYPE character_area
    USING (CASE area
        WHEN 9 THEN 'northshire_valley'::character_area
        WHEN 132 THEN 'coldridge_valley'::character_area
        WHEN 188 THEN 'shadowglen'::character_area
        WHEN 154 THEN 'deathknell'::character_area
        WHEN 221 THEN 'camp_narache'::character_area
        WHEN 363 THEN 'valley_of_trials'::character_area
    END);
