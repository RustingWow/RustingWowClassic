ALTER TABLE creature_types
    ADD COLUMN skinning_loot_id integer NOT NULL DEFAULT 0,
    ADD COLUMN pickpocket_loot_id integer NOT NULL DEFAULT 0;

ALTER TABLE creature_loot DROP CONSTRAINT creature_loot_pkey;
ALTER TABLE creature_loot ADD COLUMN condition_id integer NOT NULL DEFAULT 0;
ALTER TABLE creature_loot ADD PRIMARY KEY (loot_id, item_or_ref, group_id, condition_id);

ALTER TABLE loot_references DROP CONSTRAINT loot_references_pkey;
ALTER TABLE loot_references ADD COLUMN condition_id integer NOT NULL DEFAULT 0;
ALTER TABLE loot_references ADD PRIMARY KEY (ref_id, item_or_ref, group_id, condition_id);

CREATE TABLE gameobject_loot (
    loot_id integer NOT NULL,
    item_or_ref integer NOT NULL,
    chance real NOT NULL DEFAULT 100,
    group_id smallint NOT NULL DEFAULT 0,
    min_count integer NOT NULL DEFAULT 1,
    max_count integer NOT NULL DEFAULT 1,
    condition_id integer NOT NULL DEFAULT 0,
    PRIMARY KEY (loot_id, item_or_ref, group_id, condition_id)
);

CREATE TABLE item_loot (
    loot_id integer NOT NULL,
    item_or_ref integer NOT NULL,
    chance real NOT NULL DEFAULT 100,
    group_id smallint NOT NULL DEFAULT 0,
    min_count integer NOT NULL DEFAULT 1,
    max_count integer NOT NULL DEFAULT 1,
    condition_id integer NOT NULL DEFAULT 0,
    PRIMARY KEY (loot_id, item_or_ref, group_id, condition_id)
);

CREATE TABLE fishing_loot (
    loot_id integer NOT NULL,
    item_or_ref integer NOT NULL,
    chance real NOT NULL DEFAULT 100,
    group_id smallint NOT NULL DEFAULT 0,
    min_count integer NOT NULL DEFAULT 1,
    max_count integer NOT NULL DEFAULT 1,
    condition_id integer NOT NULL DEFAULT 0,
    PRIMARY KEY (loot_id, item_or_ref, group_id, condition_id)
);

CREATE TABLE skinning_loot (
    loot_id integer NOT NULL,
    item_or_ref integer NOT NULL,
    chance real NOT NULL DEFAULT 100,
    group_id smallint NOT NULL DEFAULT 0,
    min_count integer NOT NULL DEFAULT 1,
    max_count integer NOT NULL DEFAULT 1,
    condition_id integer NOT NULL DEFAULT 0,
    PRIMARY KEY (loot_id, item_or_ref, group_id, condition_id)
);

CREATE TABLE pickpocket_loot (
    loot_id integer NOT NULL,
    item_or_ref integer NOT NULL,
    chance real NOT NULL DEFAULT 100,
    group_id smallint NOT NULL DEFAULT 0,
    min_count integer NOT NULL DEFAULT 1,
    max_count integer NOT NULL DEFAULT 1,
    condition_id integer NOT NULL DEFAULT 0,
    PRIMARY KEY (loot_id, item_or_ref, group_id, condition_id)
);

CREATE TABLE disenchant_loot (
    loot_id integer NOT NULL,
    item_or_ref integer NOT NULL,
    chance real NOT NULL DEFAULT 100,
    group_id smallint NOT NULL DEFAULT 0,
    min_count integer NOT NULL DEFAULT 1,
    max_count integer NOT NULL DEFAULT 1,
    condition_id integer NOT NULL DEFAULT 0,
    PRIMARY KEY (loot_id, item_or_ref, group_id, condition_id)
);

CREATE TABLE mail_loot (
    loot_id integer NOT NULL,
    item_or_ref integer NOT NULL,
    chance real NOT NULL DEFAULT 100,
    group_id smallint NOT NULL DEFAULT 0,
    min_count integer NOT NULL DEFAULT 1,
    max_count integer NOT NULL DEFAULT 1,
    condition_id integer NOT NULL DEFAULT 0,
    PRIMARY KEY (loot_id, item_or_ref, group_id, condition_id)
);
