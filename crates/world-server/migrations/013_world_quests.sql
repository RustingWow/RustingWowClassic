CREATE TYPE questgiver_kind AS ENUM (
    'CREATURE',
    'GAMEOBJECT'
);

CREATE TABLE quests (
    entry integer PRIMARY KEY,
    title text NOT NULL DEFAULT '',
    details text NOT NULL DEFAULT '',
    objectives text NOT NULL DEFAULT '',
    offer_reward_text text NOT NULL DEFAULT '',
    request_items_text text NOT NULL DEFAULT '',
    end_text text NOT NULL DEFAULT '',
    objective_text_1 text NOT NULL DEFAULT '',
    objective_text_2 text NOT NULL DEFAULT '',
    objective_text_3 text NOT NULL DEFAULT '',
    objective_text_4 text NOT NULL DEFAULT '',
    min_level smallint NOT NULL DEFAULT 0,
    quest_level smallint NOT NULL DEFAULT 0,
    zone_or_sort integer NOT NULL DEFAULT 0,
    quest_type integer NOT NULL DEFAULT 0,
    required_races integer NOT NULL DEFAULT 0,
    required_classes integer NOT NULL DEFAULT 0,
    required_skill integer NOT NULL DEFAULT 0,
    required_skill_value integer NOT NULL DEFAULT 0,
    prev_quest_id integer NOT NULL DEFAULT 0,
    next_quest_id integer NOT NULL DEFAULT 0,
    exclusive_group integer NOT NULL DEFAULT 0,
    next_quest_in_chain integer NOT NULL DEFAULT 0,
    method smallint NOT NULL DEFAULT 2,
    quest_flags integer NOT NULL DEFAULT 0,
    special_flags integer NOT NULL DEFAULT 0,
    suggested_players smallint NOT NULL DEFAULT 0,
    time_limit_secs integer NOT NULL DEFAULT 0,
    src_item_id integer NOT NULL DEFAULT 0,
    src_item_count integer NOT NULL DEFAULT 0,
    req_item_id_1 integer NOT NULL DEFAULT 0,
    req_item_id_2 integer NOT NULL DEFAULT 0,
    req_item_id_3 integer NOT NULL DEFAULT 0,
    req_item_id_4 integer NOT NULL DEFAULT 0,
    req_item_count_1 integer NOT NULL DEFAULT 0,
    req_item_count_2 integer NOT NULL DEFAULT 0,
    req_item_count_3 integer NOT NULL DEFAULT 0,
    req_item_count_4 integer NOT NULL DEFAULT 0,
    req_creature_or_go_id_1 integer NOT NULL DEFAULT 0,
    req_creature_or_go_id_2 integer NOT NULL DEFAULT 0,
    req_creature_or_go_id_3 integer NOT NULL DEFAULT 0,
    req_creature_or_go_id_4 integer NOT NULL DEFAULT 0,
    req_creature_or_go_count_1 integer NOT NULL DEFAULT 0,
    req_creature_or_go_count_2 integer NOT NULL DEFAULT 0,
    req_creature_or_go_count_3 integer NOT NULL DEFAULT 0,
    req_creature_or_go_count_4 integer NOT NULL DEFAULT 0,
    rew_money integer NOT NULL DEFAULT 0,
    rew_money_max_level integer NOT NULL DEFAULT 0,
    rew_item_id_1 integer NOT NULL DEFAULT 0,
    rew_item_id_2 integer NOT NULL DEFAULT 0,
    rew_item_id_3 integer NOT NULL DEFAULT 0,
    rew_item_id_4 integer NOT NULL DEFAULT 0,
    rew_item_count_1 integer NOT NULL DEFAULT 0,
    rew_item_count_2 integer NOT NULL DEFAULT 0,
    rew_item_count_3 integer NOT NULL DEFAULT 0,
    rew_item_count_4 integer NOT NULL DEFAULT 0,
    rew_choice_item_id_1 integer NOT NULL DEFAULT 0,
    rew_choice_item_id_2 integer NOT NULL DEFAULT 0,
    rew_choice_item_id_3 integer NOT NULL DEFAULT 0,
    rew_choice_item_id_4 integer NOT NULL DEFAULT 0,
    rew_choice_item_id_5 integer NOT NULL DEFAULT 0,
    rew_choice_item_id_6 integer NOT NULL DEFAULT 0,
    rew_choice_item_count_1 integer NOT NULL DEFAULT 0,
    rew_choice_item_count_2 integer NOT NULL DEFAULT 0,
    rew_choice_item_count_3 integer NOT NULL DEFAULT 0,
    rew_choice_item_count_4 integer NOT NULL DEFAULT 0,
    rew_choice_item_count_5 integer NOT NULL DEFAULT 0,
    rew_choice_item_count_6 integer NOT NULL DEFAULT 0,
    rew_rep_faction_1 integer NOT NULL DEFAULT 0,
    rew_rep_faction_2 integer NOT NULL DEFAULT 0,
    rew_rep_faction_3 integer NOT NULL DEFAULT 0,
    rew_rep_faction_4 integer NOT NULL DEFAULT 0,
    rew_rep_faction_5 integer NOT NULL DEFAULT 0,
    rew_rep_value_1 integer NOT NULL DEFAULT 0,
    rew_rep_value_2 integer NOT NULL DEFAULT 0,
    rew_rep_value_3 integer NOT NULL DEFAULT 0,
    rew_rep_value_4 integer NOT NULL DEFAULT 0,
    rew_rep_value_5 integer NOT NULL DEFAULT 0,
    rew_spell integer NOT NULL DEFAULT 0,
    rew_spell_cast integer NOT NULL DEFAULT 0,
    point_map_id integer NOT NULL DEFAULT 0,
    point_x real NOT NULL DEFAULT 0,
    point_y real NOT NULL DEFAULT 0
);

CREATE TABLE creature_quest_starts (
    entry integer NOT NULL,
    quest_id integer NOT NULL,
    PRIMARY KEY (entry, quest_id)
);

CREATE TABLE creature_quest_ends (
    entry integer NOT NULL,
    quest_id integer NOT NULL,
    PRIMARY KEY (entry, quest_id)
);

CREATE TABLE gameobject_quest_starts (
    entry integer NOT NULL,
    quest_id integer NOT NULL,
    PRIMARY KEY (entry, quest_id)
);

CREATE TABLE gameobject_quest_ends (
    entry integer NOT NULL,
    quest_id integer NOT NULL,
    PRIMARY KEY (entry, quest_id)
);

CREATE TABLE areatrigger_quest_ends (
    entry integer NOT NULL,
    quest_id integer NOT NULL,
    PRIMARY KEY (entry, quest_id)
);

CREATE TABLE questgiver_greetings (
    entry integer NOT NULL,
    kind questgiver_kind NOT NULL,
    text text NOT NULL DEFAULT '',
    PRIMARY KEY (entry, kind)
);

CREATE TABLE broadcast_texts (
    id integer PRIMARY KEY,
    text text NOT NULL DEFAULT '',
    text_female text NOT NULL DEFAULT '',
    chat_type smallint NOT NULL DEFAULT 0,
    language_id integer NOT NULL DEFAULT 0,
    sound_id integer NOT NULL DEFAULT 0,
    emote_id integer NOT NULL DEFAULT 0
);

CREATE TABLE gameobject_types (
    entry integer PRIMARY KEY,
    name text NOT NULL,
    object_type integer NOT NULL DEFAULT 0,
    display_id integer NOT NULL DEFAULT 0,
    faction integer NOT NULL DEFAULT 0,
    flags integer NOT NULL DEFAULT 0,
    size real NOT NULL DEFAULT 1,
    data_0 integer NOT NULL DEFAULT 0,
    data_1 integer NOT NULL DEFAULT 0,
    data_2 integer NOT NULL DEFAULT 0,
    data_3 integer NOT NULL DEFAULT 0,
    data_4 integer NOT NULL DEFAULT 0,
    data_5 integer NOT NULL DEFAULT 0,
    data_6 integer NOT NULL DEFAULT 0,
    data_7 integer NOT NULL DEFAULT 0,
    data_8 integer NOT NULL DEFAULT 0,
    data_9 integer NOT NULL DEFAULT 0,
    data_10 integer NOT NULL DEFAULT 0,
    data_11 integer NOT NULL DEFAULT 0,
    data_12 integer NOT NULL DEFAULT 0,
    data_13 integer NOT NULL DEFAULT 0,
    data_14 integer NOT NULL DEFAULT 0,
    data_15 integer NOT NULL DEFAULT 0,
    data_16 integer NOT NULL DEFAULT 0,
    data_17 integer NOT NULL DEFAULT 0,
    data_18 integer NOT NULL DEFAULT 0,
    data_19 integer NOT NULL DEFAULT 0,
    data_20 integer NOT NULL DEFAULT 0,
    data_21 integer NOT NULL DEFAULT 0,
    data_22 integer NOT NULL DEFAULT 0,
    data_23 integer NOT NULL DEFAULT 0
);

CREATE TABLE gameobject_spawns (
    guid integer PRIMARY KEY,
    entry integer NOT NULL,
    map_id character_map NOT NULL,
    x real NOT NULL,
    y real NOT NULL,
    z real NOT NULL,
    orientation real NOT NULL DEFAULT 0,
    rotation_0 real NOT NULL DEFAULT 0,
    rotation_1 real NOT NULL DEFAULT 0,
    rotation_2 real NOT NULL DEFAULT 0,
    rotation_3 real NOT NULL DEFAULT 0,
    respawn_secs integer NOT NULL DEFAULT 0,
    anim_progress integer NOT NULL DEFAULT 0,
    state integer NOT NULL DEFAULT 1
);

CREATE INDEX gameobject_spawns_map_idx ON gameobject_spawns (map_id);
