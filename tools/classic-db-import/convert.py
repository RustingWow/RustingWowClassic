#!/usr/bin/env python3
"""Convert CMaNGOS classic-db MySQL dump tables into Postgres COPY files in db/."""

from __future__ import annotations

import gzip
import io
import os
import sys
import urllib.request
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from faction_labels import FACTION_LABELS  # noqa: E402

DUMP_URL = (
    "https://raw.githubusercontent.com/cmangos/classic-db/master/"
    "Full_DB/ClassicDB_1_12_1_z2815.sql.gz"
)
SOURCE_REF = "cmangos/classic-db Full_DB/ClassicDB_1_12_1_z2815.sql.gz"

TABLES = {
    "creature_template",
    "creature",
    "item_template",
    "npc_vendor",
    "npc_vendor_template",
    "creature_loot_template",
    "reference_loot_template",
    "gossip_menu",
    "gossip_menu_option",
    "npc_text",
}

# Vanilla `character_map` labels (wow_world_base::vanilla::Map::as_test_case_value).
MAP_LABELS = {
    0: "EASTERN_KINGDOMS",
    1: "KALIMDOR",
    13: "TESTING",
    25: "SCOTT_TEST",
    29: "CASH_TEST",
    30: "ALTERAC_VALLEY",
    33: "SHADOWFANG_KEEP",
    34: "STORMWIND_STOCKADE",
    35: "STORMWIND_PRISON",
    36: "DEADMINES",
    37: "AZSHARA_CRATER",
    42: "COLLINS_TEST",
    43: "WAILING_CAVERNS",
    44: "MONASTERY_UNUSED",
    47: "RAZORFEN_KRAUL",
    48: "BLACKFATHOM_DEEPS",
    70: "ULDAMAN",
    90: "GNOMEREGAN",
    109: "SUNKEN_TEMPLE",
    129: "RAZORFEN_DOWNS",
    169: "EMERALD_DREAM",
    189: "SCARLET_MONASTERY",
    209: "ZUL_FARRAK",
    229: "BLACKROCK_SPIRE",
    230: "BLACKROCK_DEPTHS",
    249: "ONYXIAS_LAIR",
    269: "OPENING_OF_THE_DARK_PORTAL",
    289: "SCHOLOMANCE",
    309: "ZUL_GURUB",
    329: "STRATHOLME",
    349: "MARAUDON",
    369: "DEEPRUN_TRAM",
    389: "RAGEFIRE_CHASM",
    409: "MOLTEN_CORE",
    429: "DIRE_MAUL",
    449: "ALLIANCE_PVP_BARRACKS",
    450: "HORDE_PVP_BARRACKS",
    451: "DEVELOPMENT_LAND",
    469: "BLACKWING_LAIR",
    489: "WARSONG_GULCH",
    509: "RUINS_OF_AHN_QIRAJ",
    529: "ARATHI_BASIN",
    531: "AHN_QIRAJ_TEMPLE",
    533: "NAXXRAMAS",
}

# CMaNGOS gossip_menu_option.option_id (row action), stored as gossip_options.option_kind.
GOSSIP_KIND_LABELS = {
    0: "NONE",
    1: "GOSSIP",
    2: "QUESTGIVER",
    3: "VENDOR",
    4: "TAXIVENDOR",
    5: "TRAINER",
    6: "SPIRITHEALER",
    7: "SPIRITGUIDE",
    8: "INNKEEPER",
    9: "BANKER",
    10: "PETITIONER",
    11: "TABARDDESIGNER",
    12: "BATTLEFIELD",
    13: "AUCTIONEER",
    14: "STABLEPET",
    15: "ARMORER",
    16: "UNLEARN_TALENTS",
    17: "UNLEARN_PET_SKILLS",
    99: "BOT",
}

OUT = {
    "creature_template": "creature_types",
    "creature": "creature_spawns",
    "item_template": "items",
    "npc_vendor": "creature_vendors",
    "npc_vendor_template": "vendor_templates",
    "creature_loot_template": "creature_loot",
    "reference_loot_template": "loot_references",
    "gossip_menu": "gossip_menus",
    "gossip_menu_option": "gossip_options",
    "npc_text": "gossip_texts",
}


def pg_escape(value: str) -> str:
    return value.replace("\\", "\\\\").replace("\t", "\\t").replace("\n", "\\n").replace("\r", "\\r")


def sql_str(value: object) -> str:
    if value is None:
        return "\\N"
    if isinstance(value, bool):
        return "t" if value else "f"
    return pg_escape(str(value))


def parse_sql_value(token: str):
    token = token.strip()
    if token.upper() == "NULL":
        return None
    if token.startswith("'"):
        inner = token[1:-1] if token.endswith("'") else token[1:]
        return (
            inner.replace("\\'", "'")
            .replace("\\n", "\n")
            .replace("\\r", "\r")
            .replace("\\\\", "\\")
            .replace("\\0", "")
        )
    if "." in token or "e" in token.lower():
        try:
            return float(token)
        except ValueError:
            return token
    try:
        return int(token)
    except ValueError:
        return token


def split_sql_tuple(body: str) -> list:
    values = []
    buf = []
    in_str = False
    escape = False
    i = 0
    while i < len(body):
        ch = body[i]
        if in_str:
            buf.append(ch)
            if escape:
                escape = False
            elif ch == "\\":
                escape = True
            elif ch == "'":
                if i + 1 < len(body) and body[i + 1] == "'":
                    buf.append("'")
                    i += 1
                else:
                    in_str = False
            i += 1
            continue
        if ch == "'":
            in_str = True
            buf.append(ch)
        elif ch == ",":
            values.append(parse_sql_value("".join(buf)))
            buf = []
        else:
            buf.append(ch)
        i += 1
    if buf:
        values.append(parse_sql_value("".join(buf)))
    return values


def parse_create_columns(create_sql: str) -> list[str]:
    cols = []
    for raw in create_sql.splitlines():
        line = raw.strip().rstrip(",")
        if not line.startswith("`"):
            continue
        name = line.split("`")[1]
        if name:
            cols.append(name)
    return cols


def col(row: dict, *names, default=None):
    for name in names:
        if name in row:
            return row[name]
        lower = {k.lower(): k for k in row}
        if name.lower() in lower:
            return row[lower[name.lower()]]
    return default


def as_int(value, default=0) -> int:
    if value is None or value == "":
        return default
    try:
        return int(float(value))
    except (TypeError, ValueError):
        return default


def as_float(value, default=0.0) -> float:
    if value is None or value == "":
        return default
    try:
        return float(value)
    except (TypeError, ValueError):
        return default


def as_str(value, default="") -> str:
    if value is None:
        return default
    return str(value)


def map_row(table: str, row: dict) -> list | None:
    if table == "creature_template":
        entry = as_int(col(row, "Entry", "entry"))
        if entry <= 0:
            return None
        health = as_int(col(row, "MinLevelHealth", "minlevelhealth"), 1) or 1
        civilian = as_int(col(row, "Civilian", "civilian")) != 0
        faction_id = as_int(col(row, "Faction", "FactionAlliance", "faction"))
        faction_label = FACTION_LABELS.get(faction_id)
        if faction_label is None:
            return None
        return [
            entry,
            as_str(col(row, "Name", "name"), f"Creature {entry}"),
            as_str(col(row, "SubName", "subname", "sub_name")),
            as_int(col(row, "MinLevel", "minlevel"), 1),
            as_int(col(row, "MaxLevel", "maxlevel"), 1),
            as_int(col(row, "DisplayId1", "ModelId1", "displayid1", "modelid1")),
            faction_label,
            as_int(col(row, "Family", "family")),
            as_int(col(row, "CreatureType", "type", "creature_type")),
            as_int(col(row, "NpcFlags", "npcflag", "npc_flags")),
            as_int(col(row, "UnitFlags", "unit_flags")),
            civilian,
            health,
            max(0, int(as_float(col(row, "MinMeleeDmg", "minmeleedmg", "mindmg")))),
            as_int(col(row, "LootId", "lootid", "loot_id"), entry),
            as_int(col(row, "GossipMenuId", "gossip_menu_id")),
            as_int(col(row, "VendorTemplateId", "vendor_id", "VendorId")),
        ]
    if table == "creature":
        guid = as_int(col(row, "guid"))
        entry = as_int(col(row, "id"))
        if guid <= 0 or entry <= 0:
            return None
        map_id = as_int(col(row, "map"))
        map_label = MAP_LABELS.get(map_id)
        if map_label is None:
            return None
        return [
            guid,
            entry,
            map_label,
            as_float(col(row, "position_x")),
            as_float(col(row, "position_y")),
            as_float(col(row, "position_z")),
            as_float(col(row, "orientation")),
            as_int(col(row, "spawntimesecsmin", "spawntimesecs"), 120),
            as_float(col(row, "spawndist")),
        ]
    if table == "item_template":
        entry = as_int(col(row, "entry"))
        if entry <= 0:
            return None
        return [
            entry,
            as_int(col(row, "class")),
            as_int(col(row, "subclass")),
            as_str(col(row, "name"), f"Item {entry}"),
            as_int(col(row, "displayid")),
            as_int(col(row, "Quality", "quality")),
            as_int(col(row, "Flags", "flags")),
            max(1, as_int(col(row, "BuyCount", "buycount"), 1)),
            as_int(col(row, "BuyPrice", "buyprice")),
            as_int(col(row, "SellPrice", "sellprice")),
            as_int(col(row, "InventoryType", "inventorytype")),
            as_int(col(row, "AllowableClass", "allowableclass"), -1),
            as_int(col(row, "AllowableRace", "allowablerace"), -1),
            as_int(col(row, "ItemLevel", "itemlevel")),
            as_int(col(row, "RequiredLevel", "requiredlevel")),
            max(1, as_int(col(row, "stackable"), 1)),
            as_int(col(row, "ContainerSlots", "containerslots")),
            as_int(col(row, "MaxDurability", "maxdurability")),
            as_int(col(row, "bonding")),
            as_str(col(row, "description")),
            as_int(col(row, "delay")),
            as_int(col(row, "armor")),
            as_float(col(row, "dmg_min1")),
            as_float(col(row, "dmg_max1")),
        ]
    if table in ("npc_vendor", "npc_vendor_template"):
        entry = as_int(col(row, "entry"))
        item = as_int(col(row, "item"))
        if entry <= 0 or item <= 0:
            return None
        return [
            entry,
            item,
            as_int(col(row, "slot")),
            as_int(col(row, "maxcount")),
            as_int(col(row, "incrtime")),
        ]
    if table in ("creature_loot_template", "reference_loot_template"):
        loot_id = as_int(col(row, "entry"))
        item = as_int(col(row, "item"))
        min_or_ref = as_int(col(row, "mincountOrRef", "mincountorref"), 1)
        max_count = as_int(col(row, "maxcount"), 1)
        if min_or_ref < 0:
            item_or_ref = min_or_ref
            min_count = 1
        else:
            item_or_ref = item
            min_count = min_or_ref if min_or_ref > 0 else 1
        return [
            loot_id,
            item_or_ref,
            as_float(col(row, "ChanceOrQuestChance", "chance"), 100),
            as_int(col(row, "groupid")),
            min_count,
            max(1, max_count),
        ]
    if table == "gossip_menu":
        menu_id = as_int(col(row, "entry"))
        text_id = as_int(col(row, "text_id"))
        if menu_id < 0 or text_id <= 0:
            return None
        return [menu_id, text_id]
    if table == "gossip_menu_option":
        menu_id = as_int(col(row, "menu_id"))
        option_id = as_int(col(row, "id"))
        kind = GOSSIP_KIND_LABELS.get(as_int(col(row, "option_id")))
        if kind is None:
            return None
        return [
            menu_id,
            option_id,
            as_int(col(row, "option_icon")),
            as_str(col(row, "option_text")),
            kind,
            as_int(col(row, "action_menu_id")),
        ]
    if table == "npc_text":
        text_id = as_int(col(row, "ID", "id"))
        text = as_str(col(row, "text0_0")) or as_str(col(row, "text0_1"))
        if text_id <= 0:
            return None
        return [text_id, text]
    return None


COPY_HEADER = {
    "creature_types": (
        "entry\tname\tsub_name\tmin_level\tmax_level\tdisplay_id\tfaction\tfamily\t"
        "creature_type\tnpc_flags\tunit_flags\tcivilian\thealth\tmelee_damage\tloot_id\t"
        "gossip_menu_id\tvendor_template_id"
    ),
    "creature_spawns": "guid\tentry\tmap_id\tx\ty\tz\torientation\trespawn_secs\twander_dist",
    "items": (
        "entry\tclass\tsubclass\tname\tdisplay_id\tquality\tflags\tbuy_count\tbuy_price\t"
        "sell_price\tinventory_type\tallowable_class\tallowable_race\titem_level\t"
        "required_level\tstackable\tcontainer_slots\tmax_durability\tbonding\tdescription\t"
        "delay\tarmor\tdmg_min\tdmg_max"
    ),
    "creature_vendors": "creature_entry\titem_id\tslot\tmaxcount\trestock_secs",
    "vendor_templates": "template_id\titem_id\tslot\tmaxcount\trestock_secs",
    "creature_loot": "loot_id\titem_or_ref\tchance\tgroup_id\tmin_count\tmax_count",
    "loot_references": "ref_id\titem_or_ref\tchance\tgroup_id\tmin_count\tmax_count",
    "gossip_menus": "menu_id\ttext_id",
    "gossip_options": "menu_id\toption_id\ticon\ttext\toption_kind\taction_menu_id",
    "gossip_texts": "text_id\ttext",
}


def emit_copy(out_dir: Path, pg_table: str, rows: list[list]) -> None:
    path = out_dir / f"{pg_table}.sql.gz"
    cols = COPY_HEADER[pg_table]
    colnames = cols.split("\t")
    with gzip.open(path, "wt", encoding="utf-8", newline="\n") as fh:
        fh.write("SET search_path TO world;\n")
        fh.write(f"COPY {pg_table} ({', '.join(colnames)}) FROM stdin;\n")
        seen = set()
        for row in rows:
            key = (row[0], row[1]) if pg_table not in {"creature_types", "items", "gossip_texts", "creature_spawns"} else (row[0],)
            if key in seen:
                continue
            seen.add(key)
            fh.write("\t".join(sql_str(v) for v in row) + "\n")
        fh.write("\\.\n")
    print(f"wrote {path} ({len(seen)} rows)")


def extract_tables(dump_path: Path) -> dict[str, tuple[list[str], list[list]]]:
    columns: dict[str, list[str]] = {}
    rows: dict[str, list[list]] = {name: [] for name in TABLES}
    creating = None
    create_buf: list[str] = []
    inserting = None
    insert_buf: list[str] = []

    opener = gzip.open if str(dump_path).endswith(".gz") else open
    with opener(dump_path, "rt", encoding="utf-8", errors="replace") as fh:
        for line in fh:
            stripped = line.lstrip()
            if creating:
                create_buf.append(line)
                if stripped.startswith(")") or stripped.startswith(") ENGINE"):
                    columns[creating] = parse_create_columns("".join(create_buf))
                    creating = None
                    create_buf = []
                continue
            if inserting is not None:
                insert_buf.append(line)
                if stripped.rstrip().endswith(";") or stripped.startswith("/*!") or stripped.startswith("UNLOCK"):
                    consume_insert(inserting, "".join(insert_buf), columns.get(inserting, []), rows[inserting])
                    inserting = None
                    insert_buf = []
                continue
            if stripped.startswith("CREATE TABLE"):
                name = stripped.split("`")[1] if "`" in stripped else ""
                if name in TABLES:
                    creating = name
                    create_buf = [line]
                continue
            if stripped.upper().startswith("INSERT INTO"):
                name = stripped.split("`")[1] if "`" in stripped else ""
                if name in TABLES:
                    inserting = name
                    insert_buf = [line]
                    if stripped.rstrip().endswith(";"):
                        consume_insert(inserting, "".join(insert_buf), columns.get(inserting, []), rows[inserting])
                        inserting = None
                        insert_buf = []
    return {name: (columns.get(name, []), rows[name]) for name in TABLES}


def consume_insert(table: str, sql: str, colnames: list[str], dest: list[list]) -> None:
    upper = sql.upper()
    marker = " VALUES"
    values_at = upper.find(marker)
    if values_at < 0:
        return
    payload = sql[values_at + len(marker) :].strip()
    if payload.endswith(";"):
        payload = payload[:-1]
    depth = 0
    start = None
    for i, ch in enumerate(payload):
        if ch == "(":
            if depth == 0:
                start = i + 1
            depth += 1
        elif ch == ")":
            depth -= 1
            if depth == 0 and start is not None:
                raw = payload[start:i]
                values = split_sql_tuple(raw)
                if colnames and len(values) == len(colnames):
                    mapped = map_row(table, dict(zip(colnames, values)))
                else:
                    mapped = map_row(table, {str(i): v for i, v in enumerate(values)})
                    if mapped is None and colnames:
                        mapped = map_row(table, dict(zip(colnames, values + [None] * max(0, len(colnames) - len(values)))))
                if mapped is not None:
                    dest.append(mapped)
                start = None


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    out_dir = root / "db"
    out_dir.mkdir(parents=True, exist_ok=True)
    cache = out_dir / ".classicdb-cache.sql.gz"
    src = os.environ.get("CLASSIC_DB_SQL", "")
    if src:
        dump_path = Path(src)
    else:
        dump_path = cache
        if not dump_path.exists():
            print(f"downloading {DUMP_URL}", file=sys.stderr)
            urllib.request.urlretrieve(DUMP_URL, dump_path)
    extracted = extract_tables(dump_path)
    for mysql_name, (_cols, mysql_rows) in extracted.items():
        print(f"{mysql_name}: {len(mysql_rows)} source rows")
        emit_copy(out_dir, OUT[mysql_name], mysql_rows)
    (out_dir / "SOURCE.txt").write_text(
        f"source: {SOURCE_REF}\n"
        "convert: tools/classic-db-import/convert.py\n"
        "MaNGOS → Postgres names are listed in README.md\n",
        encoding="utf-8",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
