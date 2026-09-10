CREATE TABLE creature_types (
    entry integer PRIMARY KEY,
    name text NOT NULL,
    sub_name text NOT NULL DEFAULT '',
    min_level smallint NOT NULL DEFAULT 1,
    max_level smallint NOT NULL DEFAULT 1,
    display_id integer NOT NULL DEFAULT 0,
    faction integer NOT NULL DEFAULT 0,
    family smallint NOT NULL DEFAULT 0,
    creature_type integer NOT NULL DEFAULT 0,
    npc_flags integer NOT NULL DEFAULT 0,
    unit_flags integer NOT NULL DEFAULT 0,
    civilian boolean NOT NULL DEFAULT false,
    health integer NOT NULL DEFAULT 1,
    melee_damage integer NOT NULL DEFAULT 0,
    loot_id integer NOT NULL DEFAULT 0,
    gossip_menu_id integer NOT NULL DEFAULT 0,
    vendor_template_id integer NOT NULL DEFAULT 0
);

CREATE TABLE creature_spawns (
    guid integer PRIMARY KEY,
    entry integer NOT NULL,
    map_id integer NOT NULL,
    x real NOT NULL,
    y real NOT NULL,
    z real NOT NULL,
    orientation real NOT NULL DEFAULT 0,
    respawn_secs integer NOT NULL DEFAULT 120,
    wander_dist real NOT NULL DEFAULT 0
);

CREATE INDEX creature_spawns_map_idx ON creature_spawns (map_id);

CREATE TABLE items (
    entry integer PRIMARY KEY,
    class smallint NOT NULL DEFAULT 0,
    subclass smallint NOT NULL DEFAULT 0,
    name text NOT NULL,
    display_id integer NOT NULL DEFAULT 0,
    quality smallint NOT NULL DEFAULT 0,
    flags integer NOT NULL DEFAULT 0,
    buy_count smallint NOT NULL DEFAULT 1,
    buy_price integer NOT NULL DEFAULT 0,
    sell_price integer NOT NULL DEFAULT 0,
    inventory_type smallint NOT NULL DEFAULT 0,
    allowable_class integer NOT NULL DEFAULT -1,
    allowable_race integer NOT NULL DEFAULT -1,
    item_level smallint NOT NULL DEFAULT 0,
    required_level smallint NOT NULL DEFAULT 0,
    stackable integer NOT NULL DEFAULT 1,
    container_slots smallint NOT NULL DEFAULT 0,
    max_durability integer NOT NULL DEFAULT 0,
    bonding smallint NOT NULL DEFAULT 0,
    description text NOT NULL DEFAULT '',
    delay integer NOT NULL DEFAULT 0,
    armor integer NOT NULL DEFAULT 0,
    dmg_min real NOT NULL DEFAULT 0,
    dmg_max real NOT NULL DEFAULT 0
);

CREATE TABLE creature_vendors (
    creature_entry integer NOT NULL,
    item_id integer NOT NULL,
    slot smallint NOT NULL DEFAULT 0,
    maxcount smallint NOT NULL DEFAULT 0,
    restock_secs integer NOT NULL DEFAULT 0,
    PRIMARY KEY (creature_entry, item_id)
);

CREATE TABLE vendor_templates (
    template_id integer NOT NULL,
    item_id integer NOT NULL,
    slot smallint NOT NULL DEFAULT 0,
    maxcount smallint NOT NULL DEFAULT 0,
    restock_secs integer NOT NULL DEFAULT 0,
    PRIMARY KEY (template_id, item_id)
);

CREATE TABLE creature_loot (
    loot_id integer NOT NULL,
    item_or_ref integer NOT NULL,
    chance real NOT NULL DEFAULT 100,
    group_id smallint NOT NULL DEFAULT 0,
    min_count integer NOT NULL DEFAULT 1,
    max_count integer NOT NULL DEFAULT 1,
    PRIMARY KEY (loot_id, item_or_ref)
);

CREATE TABLE loot_references (
    ref_id integer NOT NULL,
    item_or_ref integer NOT NULL,
    chance real NOT NULL DEFAULT 100,
    group_id smallint NOT NULL DEFAULT 0,
    min_count integer NOT NULL DEFAULT 1,
    max_count integer NOT NULL DEFAULT 1,
    PRIMARY KEY (ref_id, item_or_ref)
);

CREATE TABLE gossip_menus (
    menu_id integer NOT NULL,
    text_id integer NOT NULL,
    PRIMARY KEY (menu_id, text_id)
);

CREATE TABLE gossip_options (
    menu_id integer NOT NULL,
    option_id integer NOT NULL,
    icon smallint NOT NULL DEFAULT 0,
    text text NOT NULL DEFAULT '',
    option_kind smallint NOT NULL DEFAULT 0,
    action_menu_id integer NOT NULL DEFAULT 0,
    PRIMARY KEY (menu_id, option_id)
);

CREATE TABLE gossip_texts (
    text_id integer PRIMARY KEY,
    text text NOT NULL DEFAULT ''
);
