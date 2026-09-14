CREATE TYPE character_quest_status AS ENUM (
    'ACTIVE',
    'REWARDED'
);

CREATE TABLE character_quests (
    character_id bigint NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    quest_id integer NOT NULL,
    status character_quest_status NOT NULL DEFAULT 'ACTIVE',
    kill_count_0 integer NOT NULL DEFAULT 0,
    kill_count_1 integer NOT NULL DEFAULT 0,
    kill_count_2 integer NOT NULL DEFAULT 0,
    kill_count_3 integer NOT NULL DEFAULT 0,
    PRIMARY KEY (character_id, quest_id)
);
