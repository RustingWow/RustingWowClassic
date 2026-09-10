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
  gossip_menus
  gossip_options
  gossip_texts
)

psql "$DATABASE_URL" -v ON_ERROR_STOP=1 <<'SQL'
CREATE SCHEMA IF NOT EXISTS world;
SET search_path TO world;
TRUNCATE gossip_options, gossip_menus, gossip_texts,
         creature_loot, loot_references,
         creature_vendors, vendor_templates,
         creature_spawns, creature_types, items;
SQL

for table in "${ORDER[@]}"; do
  echo "loading ${table}"
  gzip -dc "${ROOT}/${table}.sql.gz" | psql "$DATABASE_URL" -v ON_ERROR_STOP=1
done
