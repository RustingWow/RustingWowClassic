# World catalog (CMaNGOS classic-db)

Content converted from [cmangos/classic-db](https://github.com/cmangos/classic-db) (`Full_DB/ClassicDB_1_12_1_z2815.sql.gz`). Schema lives in sqlx (`008_world_catalog.sql`–`011_gossip_option_kind_enum.sql`); these files are **data** and are loaded by hand. `creature_spawns.map_id` is `character_map`, `creature_types.faction` is `creature_faction`, `gossip_options.option_kind` is `gossip_option_kind` (e.g. `VENDOR`, `GOSSIP`).

GPL-3: see `LICENSE.md`. Blizzard materials: see `COPYRIGHT.md`.

## Apply

Start the world-server once (creates schema `world` and empty tables), then:

```bash
export DATABASE_URL=postgres://wow:wow@127.0.0.1:5432/wow
chmod +x db/load.sh
./db/load.sh
```

Re-import after a new convert: `TRUNCATE` is inside `load.sh`.

## Re-convert

```bash
python3 tools/classic-db-import/convert.py
```

Optional: `CLASSIC_DB_SQL=/path/to/ClassicDB_1_12_1_z2815.sql.gz`

## Names

| MaNGOS | Postgres |
| --- | --- |
| creature_template | creature_types |
| creature | creature_spawns |
| item_template | items |
| npc_vendor | creature_vendors |
| npc_vendor_template | vendor_templates |
| creature_loot_template | creature_loot |
| reference_loot_template | loot_references |
| gossip_menu | gossip_menus |
| gossip_menu_option | gossip_options |
| npc_text | gossip_texts |
