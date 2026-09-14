#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "DATABASE_URL is required" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")" && pwd)"
ORDER=(
  creature_types
  creature_spawns
  items
  creature_vendors
  vendor_templates
  creature_loot
  loot_references
  gameobject_loot
  item_loot
  fishing_loot
  skinning_loot
  pickpocket_loot
  disenchant_loot
  mail_loot
  gossip_menus
  gossip_options
  gossip_texts
  gossip_text_broadcasts
  gameobject_types
  gameobject_spawns
  quests
  creature_quest_starts
  creature_quest_ends
  gameobject_quest_starts
  gameobject_quest_ends
  areatrigger_quest_ends
  questgiver_greetings
  broadcast_texts
)

quoted=""
for table in "${ORDER[@]}"; do
  if [[ -n "${quoted}" ]]; then
    quoted+=", "
  fi
  quoted+="'${table}'"
done

psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -c "CREATE SCHEMA IF NOT EXISTS world;" >/dev/null

missing="$(
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -tAc "
SELECT coalesce(string_agg(t, ', ' ORDER BY t), '')
FROM unnest(ARRAY[${quoted}]) AS t
WHERE to_regclass('world.' || t) IS NULL;
"
)"
missing="$(printf '%s' "${missing}" | tr -d '[:space:]')"
if [[ -n "${missing}" ]]; then
  echo "Missing world tables: ${missing}" >&2
  echo "Start the world-server once so sqlx applies migrations (022_loot_tables.sql), then re-run ./db/load.sh" >&2
  exit 1
fi

psql "$DATABASE_URL" -v ON_ERROR_STOP=1 <<SQL
SET search_path TO world;
TRUNCATE gossip_options, gossip_menus, gossip_texts, gossip_text_broadcasts,
         creature_loot, loot_references,
         gameobject_loot, item_loot, fishing_loot, skinning_loot,
         pickpocket_loot, disenchant_loot, mail_loot,
         creature_vendors, vendor_templates,
         creature_spawns, creature_types, items,
         creature_quest_starts, creature_quest_ends,
         gameobject_quest_starts, gameobject_quest_ends,
         areatrigger_quest_ends, questgiver_greetings,
         quests, broadcast_texts,
         gameobject_spawns, gameobject_types;
SQL

for table in "${ORDER[@]}"; do
  path="${ROOT}/${table}.sql.gz"
  if [[ ! -f "${path}" ]]; then
    echo "skipping ${table} (no dump)"
    continue
  fi
  echo "loading ${table}"
  gzip -dc "${path}" | psql "$DATABASE_URL" -v ON_ERROR_STOP=1
done
