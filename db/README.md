# World catalog (CMaNGOS classic-db)

Content converted from [cmangos/classic-db](https://github.com/cmangos/classic-db) (`Full_DB/ClassicDB_1_12_1_z2815.sql.gz`). Schema lives in sqlx (`008_world_catalog.sql`–`022_loot_tables.sql`); these files are **data** and are loaded by hand.

Typed columns:

- `creature_spawns.map_id`, `gameobject_spawns.map_id`, `game_tele.map_id` → `character_map`
- `creature_types.faction`, `gameobject_types.faction` → `creature_faction` (`NONE` = template 0)
- `gossip_options.option_kind` → `gossip_option_kind` (e.g. `VENDOR`, `GOSSIP`)
- `gossip_options.icon` → `gossip_option_icon` (`CHAT`, `VENDOR`, …)
- `quests.required_races` / `required_classes` → `character_race[]` / `character_class[]` (`{}` = no filter)
- `quests.quest_type` → `quest_type` (`NONE`, `ELITE`, `DUNGEON`, …)
- `gossip_text_broadcasts.probability_0`–`7` stored **0–100** (the client packet uses `db/100`)

GPL-3: see `LICENSE.md`. Blizzard materials: see `COPYRIGHT.md`.

## Apply

Start the world-server once (creates schema `world` and empty tables, including new sqlx migrations), then:

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
| gameobject_loot_template | gameobject_loot |
| item_loot_template | item_loot |
| fishing_loot_template | fishing_loot |
| skinning_loot_template | skinning_loot |
| pickpocketing_loot_template | pickpocket_loot |
| disenchant_loot_template | disenchant_loot |
| mail_loot_template | mail_loot |
| gossip_menu | gossip_menus |
| gossip_menu_option | gossip_options |
| npc_text | gossip_texts |
| npc_text_broadcast_text | gossip_text_broadcasts |
| quest_template | quests |
| creature_questrelation | creature_quest_starts |
| creature_involvedrelation | creature_quest_ends |
| gameobject_questrelation | gameobject_quest_starts |
| gameobject_involvedrelation | gameobject_quest_ends |
| areatrigger_involvedrelation | areatrigger_quest_ends |
| questgiver_greeting | questgiver_greetings |
| broadcast_text | broadcast_texts |
| gameobject_template | gameobject_types |
| gameobject | gameobject_spawns |
