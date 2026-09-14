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
    "gameobject_loot_template",
    "item_loot_template",
    "fishing_loot_template",
    "skinning_loot_template",
    "pickpocketing_loot_template",
    "disenchant_loot_template",
    "mail_loot_template",
    "gossip_menu",
    "gossip_menu_option",
    "npc_text",
    "npc_text_broadcast_text",
    "quest_template",
    "creature_questrelation",
    "creature_involvedrelation",
    "gameobject_questrelation",
    "gameobject_involvedrelation",
    "areatrigger_involvedrelation",
    "questgiver_greeting",
    "broadcast_text",
    "gameobject_template",
    "gameobject",
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

GOSSIP_ICON_LABELS = {
    0: "CHAT",
    1: "VENDOR",
    2: "TAXI",
    3: "TRAINER",
    4: "INTERACT_1",
    5: "INTERACT_2",
    6: "MONEY_BAG",
    7: "TALK",
    8: "TABARD",
    9: "BATTLE",
    10: "DOT",
    11: "CHAT_11",
    12: "CHAT_12",
}

QUEST_TYPE_LABELS = {
    0: "NONE",
    1: "ELITE",
    21: "LIFE",
    41: "PVP",
    62: "RAID",
    81: "DUNGEON",
    82: "WORLD_EVENT",
    83: "LEGENDARY",
    84: "ESCORT",
}

RACE_MASK_BITS = (
    (1, "HUMAN"),
    (2, "ORC"),
    (4, "DWARF"),
    (8, "NIGHT_ELF"),
    (16, "UNDEAD"),
    (32, "TAUREN"),
    (64, "GNOME"),
    (128, "TROLL"),
)

CLASS_MASK_BITS = (
    (1, "WARRIOR"),
    (2, "PALADIN"),
    (4, "HUNTER"),
    (8, "ROGUE"),
    (16, "PRIEST"),
    (64, "SHAMAN"),
    (128, "MAGE"),
    (256, "WARLOCK"),
    (1024, "DRUID"),
)

OUT = {
    "creature_template": "creature_types",
    "creature": "creature_spawns",
    "item_template": "items",
    "npc_vendor": "creature_vendors",
    "npc_vendor_template": "vendor_templates",
    "creature_loot_template": "creature_loot",
    "reference_loot_template": "loot_references",
    "gameobject_loot_template": "gameobject_loot",
    "item_loot_template": "item_loot",
    "fishing_loot_template": "fishing_loot",
    "skinning_loot_template": "skinning_loot",
    "pickpocketing_loot_template": "pickpocket_loot",
    "disenchant_loot_template": "disenchant_loot",
    "mail_loot_template": "mail_loot",
    "gossip_menu": "gossip_menus",
    "gossip_menu_option": "gossip_options",
    "npc_text": "gossip_texts",
    "npc_text_broadcast_text": "gossip_text_broadcasts",
    "quest_template": "quests",
    "creature_questrelation": "creature_quest_starts",
    "creature_involvedrelation": "creature_quest_ends",
    "gameobject_questrelation": "gameobject_quest_starts",
    "gameobject_involvedrelation": "gameobject_quest_ends",
    "areatrigger_involvedrelation": "areatrigger_quest_ends",
    "questgiver_greeting": "questgiver_greetings",
    "broadcast_text": "broadcast_texts",
    "gameobject_template": "gameobject_types",
    "gameobject": "gameobject_spawns",
}

SINGLE_KEY = {
    "creature_types",
    "items",
    "gossip_texts",
    "gossip_text_broadcasts",
    "creature_spawns",
    "quests",
    "broadcast_texts",
    "gameobject_types",
    "gameobject_spawns",
}

LOOT_TABLES = {
    "creature_loot",
    "loot_references",
    "gameobject_loot",
    "item_loot",
    "fishing_loot",
    "skinning_loot",
    "pickpocket_loot",
    "disenchant_loot",
    "mail_loot",
}

LOOT_COPY = "loot_id\titem_or_ref\tchance\tgroup_id\tmin_count\tmax_count\tcondition_id"
LOOT_REF_COPY = "ref_id\titem_or_ref\tchance\tgroup_id\tmin_count\tmax_count\tcondition_id"

QUESTGIVER_KIND = {0: "CREATURE", 1: "GAMEOBJECT"}


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


def numbered(row: dict, *prefixes, count: int, mapper=as_int) -> list:
    values = []
    for i in range(1, count + 1):
        names = []
        for prefix in prefixes:
            names.append(f"{prefix}{i}")
            names.append(f"{prefix.lower()}{i}")
        values.append(mapper(col(row, *names)))
    return values


def mask_to_enum_array(mask: int, bits: tuple[tuple[int, str], ...]) -> str:
    if mask <= 0:
        return "{}"
    labels = [label for bit, label in bits if mask & bit]
    return "{" + ",".join(labels) + "}"


def scale_probabilities(probs: list[float]) -> list[float]:
    if probs and max(probs) <= 1:
        return [value * 100 for value in probs]
    return probs


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
            as_int(col(row, "SkinningLootId", "skinloot", "skinningloot")),
            as_int(col(row, "PickpocketLootId", "pickpocketloot", "pickpocketlootid")),
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
    if table.endswith("_loot_template"):
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
            as_int(col(row, "condition_id", "conditionid")),
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
        icon = GOSSIP_ICON_LABELS.get(as_int(col(row, "option_icon")))
        if kind is None or icon is None:
            return None
        return [
            menu_id,
            option_id,
            icon,
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
    if table == "npc_text_broadcast_text":
        text_id = as_int(col(row, "Id", "ID", "id"))
        if text_id <= 0:
            return None
        probs = [
            as_float(col(row, f"Prob{i}", f"prob{i}"))
            for i in range(8)
        ]
        ids = [
            as_int(col(row, f"BroadcastTextId{i}", f"broadcasttextid{i}"))
            for i in range(8)
        ]
        if all(broadcast_id <= 0 for broadcast_id in ids):
            return None
        return [text_id, *scale_probabilities(probs), *ids]
    if table == "quest_template":
        entry = as_int(col(row, "entry", "Entry"))
        if entry <= 0:
            return None
        return [
            entry,
            as_str(col(row, "Title", "title"), f"Quest {entry}"),
            as_str(col(row, "Details", "details")),
            as_str(col(row, "Objectives", "objectives")),
            as_str(col(row, "OfferRewardText", "offerrewardtext")),
            as_str(col(row, "RequestItemsText", "requestitemstext")),
            as_str(col(row, "EndText", "endtext")),
            *numbered(row, "ObjectiveText", count=4, mapper=as_str),
            as_int(col(row, "MinLevel", "minlevel")),
            as_int(col(row, "QuestLevel", "questlevel")),
            as_int(col(row, "ZoneOrSort", "zoneorsort")),
            QUEST_TYPE_LABELS.get(as_int(col(row, "Type", "type")), "NONE"),
            mask_to_enum_array(as_int(col(row, "RequiredRaces", "requiredraces")), RACE_MASK_BITS),
            mask_to_enum_array(
                as_int(col(row, "RequiredClasses", "requiredclasses")), CLASS_MASK_BITS
            ),
            as_int(col(row, "RequiredSkill", "requiredskill")),
            as_int(col(row, "RequiredSkillValue", "requiredskillvalue")),
            as_int(col(row, "PrevQuestId", "prevquestid")),
            as_int(col(row, "NextQuestId", "nextquestid")),
            as_int(col(row, "ExclusiveGroup", "exclusivegroup")),
            as_int(col(row, "NextQuestInChain", "nextquestinchain")),
            as_int(col(row, "Method", "method"), 2),
            as_int(col(row, "QuestFlags", "questflags")),
            as_int(col(row, "SpecialFlags", "specialflags")),
            as_int(col(row, "SuggestedPlayers", "suggestedplayers")),
            as_int(col(row, "LimitTime", "limittime")),
            as_int(col(row, "SrcItemId", "srcitemid")),
            as_int(col(row, "SrcItemCount", "srcitemcount")),
            *numbered(row, "ReqItemId", count=4),
            *numbered(row, "ReqItemCount", count=4),
            *numbered(row, "ReqCreatureOrGOId", count=4),
            *numbered(row, "ReqCreatureOrGOCount", count=4),
            as_int(col(row, "RewOrReqMoney", "reworreqmoney")),
            as_int(col(row, "RewMoneyMaxLevel", "rewmoneymaxlevel")),
            *numbered(row, "RewItemId", count=4),
            *numbered(row, "RewItemCount", count=4),
            *numbered(row, "RewChoiceItemId", count=6),
            *numbered(row, "RewChoiceItemCount", count=6),
            *numbered(row, "RewRepFaction", count=5),
            *numbered(row, "RewRepValue", count=5),
            as_int(col(row, "RewSpell", "rewspell")),
            as_int(col(row, "RewSpellCast", "rewspellcast")),
            as_int(col(row, "PointMapId", "pointmapid")),
            as_float(col(row, "PointX", "pointx")),
            as_float(col(row, "PointY", "pointy")),
        ]
    if table in (
        "creature_questrelation",
        "creature_involvedrelation",
        "gameobject_questrelation",
        "gameobject_involvedrelation",
        "areatrigger_involvedrelation",
    ):
        entry = as_int(col(row, "id", "Id", "entry"))
        quest = as_int(col(row, "quest", "Quest"))
        if entry <= 0 or quest <= 0:
            return None
        return [entry, quest]
    if table == "questgiver_greeting":
        entry = as_int(col(row, "Entry", "entry"))
        kind = QUESTGIVER_KIND.get(as_int(col(row, "Type", "type")))
        if entry <= 0 or kind is None:
            return None
        return [entry, kind, as_str(col(row, "Greeting", "greeting"))]
    if table == "broadcast_text":
        text_id = as_int(col(row, "Id", "ID", "id"))
        if text_id <= 0:
            return None
        return [
            text_id,
            as_str(col(row, "Text", "text")),
            as_str(col(row, "Text1", "text1")),
            as_int(col(row, "ChatTypeID", "chattypeid")),
            as_int(col(row, "LanguageID", "languageid")),
            as_int(col(row, "SoundEntriesID1", "soundentriesid1", "SoundEntriesID")),
            as_int(col(row, "EmoteID1", "emoteid1")),
        ]
    if table == "gameobject_template":
        entry = as_int(col(row, "entry", "Entry"))
        if entry <= 0:
            return None
        data = [
            as_int(col(row, f"data{i}", f"Data{i}"))
            for i in range(24)
        ]
        faction_id = as_int(col(row, "faction", "Faction"))
        faction_label = FACTION_LABELS.get(faction_id)
        if faction_label is None:
            return None
        return [
            entry,
            as_str(col(row, "name", "Name"), f"GameObject {entry}"),
            as_int(col(row, "type", "Type")),
            as_int(col(row, "displayId", "displayid", "DisplayId")),
            faction_label,
            as_int(col(row, "flags", "Flags")),
            as_float(col(row, "size", "Size"), 1.0) or 1.0,
            *data,
        ]
    if table == "gameobject":
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
            as_float(col(row, "rotation0")),
            as_float(col(row, "rotation1")),
            as_float(col(row, "rotation2")),
            as_float(col(row, "rotation3")),
            as_int(col(row, "spawntimesecsmin", "spawntimesecs")),
            as_int(col(row, "animprogress")),
            as_int(col(row, "state"), 1),
        ]
    return None


QUEST_COPY = (
    "entry\ttitle\tdetails\tobjectives\toffer_reward_text\trequest_items_text\tend_text\t"
    "objective_text_1\tobjective_text_2\tobjective_text_3\tobjective_text_4\t"
    "min_level\tquest_level\tzone_or_sort\tquest_type\trequired_races\trequired_classes\t"
    "required_skill\trequired_skill_value\tprev_quest_id\tnext_quest_id\texclusive_group\t"
    "next_quest_in_chain\tmethod\tquest_flags\tspecial_flags\tsuggested_players\t"
    "time_limit_secs\tsrc_item_id\tsrc_item_count\t"
    "req_item_id_1\treq_item_id_2\treq_item_id_3\treq_item_id_4\t"
    "req_item_count_1\treq_item_count_2\treq_item_count_3\treq_item_count_4\t"
    "req_creature_or_go_id_1\treq_creature_or_go_id_2\treq_creature_or_go_id_3\t"
    "req_creature_or_go_id_4\treq_creature_or_go_count_1\treq_creature_or_go_count_2\t"
    "req_creature_or_go_count_3\treq_creature_or_go_count_4\t"
    "rew_money\trew_money_max_level\t"
    "rew_item_id_1\trew_item_id_2\trew_item_id_3\trew_item_id_4\t"
    "rew_item_count_1\trew_item_count_2\trew_item_count_3\trew_item_count_4\t"
    "rew_choice_item_id_1\trew_choice_item_id_2\trew_choice_item_id_3\t"
    "rew_choice_item_id_4\trew_choice_item_id_5\trew_choice_item_id_6\t"
    "rew_choice_item_count_1\trew_choice_item_count_2\trew_choice_item_count_3\t"
    "rew_choice_item_count_4\trew_choice_item_count_5\trew_choice_item_count_6\t"
    "rew_rep_faction_1\trew_rep_faction_2\trew_rep_faction_3\trew_rep_faction_4\t"
    "rew_rep_faction_5\trew_rep_value_1\trew_rep_value_2\trew_rep_value_3\t"
    "rew_rep_value_4\trew_rep_value_5\trew_spell\trew_spell_cast\t"
    "point_map_id\tpoint_x\tpoint_y"
)

GO_TYPE_COPY = (
    "entry\tname\tobject_type\tdisplay_id\tfaction\tflags\tsize\t"
    + "\t".join(f"data_{i}" for i in range(24))
)

COPY_HEADER = {
    "creature_types": (
        "entry\tname\tsub_name\tmin_level\tmax_level\tdisplay_id\tfaction\tfamily\t"
        "creature_type\tnpc_flags\tunit_flags\tcivilian\thealth\tmelee_damage\tloot_id\t"
        "gossip_menu_id\tvendor_template_id\tskinning_loot_id\tpickpocket_loot_id"
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
    "creature_loot": LOOT_COPY,
    "loot_references": LOOT_REF_COPY,
    "gameobject_loot": LOOT_COPY,
    "item_loot": LOOT_COPY,
    "fishing_loot": LOOT_COPY,
    "skinning_loot": LOOT_COPY,
    "pickpocket_loot": LOOT_COPY,
    "disenchant_loot": LOOT_COPY,
    "mail_loot": LOOT_COPY,
    "gossip_menus": "menu_id\ttext_id",
    "gossip_options": "menu_id\toption_id\ticon\ttext\toption_kind\taction_menu_id",
    "gossip_texts": "text_id\ttext",
    "gossip_text_broadcasts": (
        "text_id\t"
        + "\t".join(f"probability_{i}" for i in range(8))
        + "\t"
        + "\t".join(f"broadcast_text_id_{i}" for i in range(8))
    ),
    "quests": QUEST_COPY,
    "creature_quest_starts": "entry\tquest_id",
    "creature_quest_ends": "entry\tquest_id",
    "gameobject_quest_starts": "entry\tquest_id",
    "gameobject_quest_ends": "entry\tquest_id",
    "areatrigger_quest_ends": "entry\tquest_id",
    "questgiver_greetings": "entry\tkind\ttext",
    "broadcast_texts": "id\ttext\ttext_female\tchat_type\tlanguage_id\tsound_id\temote_id",
    "gameobject_types": GO_TYPE_COPY,
    "gameobject_spawns": (
        "guid\tentry\tmap_id\tx\ty\tz\torientation\t"
        "rotation_0\trotation_1\trotation_2\trotation_3\trespawn_secs\tanim_progress\tstate"
    ),
}


def copy_row_key(pg_table: str, row: list) -> tuple:
    if pg_table in SINGLE_KEY:
        return (row[0],)
    if pg_table in LOOT_TABLES:
        return (row[0], row[1], row[3], row[6])
    return (row[0], row[1])


def emit_copy(out_dir: Path, pg_table: str, rows: list[list]) -> None:
    path = out_dir / f"{pg_table}.sql.gz"
    cols = COPY_HEADER[pg_table]
    colnames = cols.split("\t")
    with gzip.open(path, "wt", encoding="utf-8", newline="\n") as fh:
        fh.write("SET search_path TO world;\n")
        fh.write(f"COPY {pg_table} ({', '.join(colnames)}) FROM stdin;\n")
        seen = set()
        for row in rows:
            if len(row) != len(colnames):
                raise SystemExit(
                    f"{pg_table}: row has {len(row)} values, expected {len(colnames)}"
                )
            key = copy_row_key(pg_table, row)
            if key in seen:
                continue
            seen.add(key)
            fh.write("\t".join(sql_str(v) for v in row) + "\n")
        fh.write("\\.\n")
    print(f"wrote {path} ({len(seen)} rows)")


def _self_check() -> None:
    same_item_group_one = [1, 117, 50.0, 1, 1, 1, 0]
    same_item_group_two = [1, 117, 50.0, 2, 1, 1, 0]
    duplicate = [1, 117, 50.0, 1, 1, 1, 0]
    assert copy_row_key("creature_loot", same_item_group_one) != copy_row_key(
        "creature_loot", same_item_group_two
    )
    seen: set[tuple] = set()
    kept = 0
    for row in (same_item_group_one, same_item_group_two, duplicate):
        key = copy_row_key("creature_loot", row)
        if key in seen:
            continue
        seen.add(key)
        kept += 1
    assert kept == 2, kept


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
    _self_check()
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
    for mysql_name, (cols, mysql_rows) in extracted.items():
        if not cols and not mysql_rows:
            print(f"{mysql_name}: skipped (not in dump)")
            emit_copy(out_dir, OUT[mysql_name], [])
            continue
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
