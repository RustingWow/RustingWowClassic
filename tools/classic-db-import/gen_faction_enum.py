#!/usr/bin/env python3
"""Regenerate CreatureFaction Rust + SQL from tools/classic-db-import/faction_labels.py."""

from __future__ import annotations

import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RUST_PATH = ROOT / "crates/wow-shared/src/enums/faction.rs"
SQL_PATH = ROOT / "crates/world-server/migrations/010_creature_faction_enum.sql"
LABELS_PATH = ROOT / "tools/classic-db-import/faction_labels.py"


def load_labels() -> dict[int, str]:
    spec = importlib.util.spec_from_file_location("faction_labels", LABELS_PATH)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return dict(module.FACTION_LABELS)


def rust_variant(db: str) -> str:
    return "".join(part[:1].upper() + part[1:].lower() for part in db.split("_") if part)


def write_rust(rows: list[tuple[int, str, str]]) -> None:
    variants = "\n".join(f'    #[serde(rename = "{db}")]\n    {rust},' for _, rust, db in rows)
    as_protocol = "\n".join(f"            Self::{rust} => {tid}," for tid, rust, _ in rows)
    as_str = "\n".join(f'            Self::{rust} => "{db}",' for _, rust, db in rows)
    from_protocol = "\n".join(f"            {tid} => Some(Self::{rust})," for tid, rust, _ in rows)
    from_str = "\n".join(f'            "{db}" => Some(Self::{rust}),' for _, rust, db in rows)
    variants_arr = ",\n            ".join(f"Self::{rust}" for _, rust, _ in rows)
    rustc = f'''//! Vanilla 1.12 `FactionTemplate.dbc` id (`UNIT_FIELD_FACTIONTEMPLATE`).
//!
//! Creature templates and player units use this id on the wire, not `Faction.dbc`.
use serde::{{Deserialize, Serialize}};

use super::DbEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CreatureFaction {{
{variants}
}}

impl CreatureFaction {{
    pub const fn as_protocol(self) -> u32 {{
        match self {{
{as_protocol}
        }}
    }}

    pub const fn as_str(self) -> &'static str {{
        match self {{
{as_str}
        }}
    }}

    pub fn from_protocol(value: u32) -> Option<Self> {{
        match value {{
{from_protocol}
            _ => None,
        }}
    }}

    pub fn from_str(value: &str) -> Option<Self> {{
        match value {{
{from_str}
            _ => None,
        }}
    }}

    pub const fn variants() -> [Self; {len(rows)}] {{
        [
            {variants_arr},
        ]
    }}
}}

impl DbEnum for CreatureFaction {{
    fn as_protocol(self) -> u32 {{
        Self::as_protocol(self)
    }}

    fn as_str(self) -> &'static str {{
        Self::as_str(self)
    }}

    fn from_protocol(value: u32) -> Option<Self> {{
        Self::from_protocol(value)
    }}

    fn from_str(value: &str) -> Option<Self> {{
        Self::from_str(value)
    }}
}}
'''
    RUST_PATH.write_text(rustc, encoding="utf-8")


def write_sql(rows: list[tuple[int, str, str]]) -> None:
    labels = sorted({db for _, _, db in rows})
    enum_body = ",\n    ".join(f"'{label}'" for label in labels)
    cases = "\n        ".join(f"WHEN {tid} THEN '{db}'::creature_faction" for tid, _, db in rows)
    sql = f"""ALTER TABLE creature_types ALTER COLUMN faction DROP DEFAULT;

CREATE TYPE creature_faction AS ENUM (
    {enum_body}
);

ALTER TABLE creature_types
    ALTER COLUMN faction TYPE creature_faction
    USING (CASE faction
        {cases}
    END);
"""
    SQL_PATH.write_text(sql, encoding="utf-8")


def main() -> int:
    labels = load_labels()
    rows = [(tid, rust_variant(db), db) for tid, db in sorted(labels.items())]
    rust_names = [rust for _, rust, _ in rows]
    if len(rust_names) != len(set(rust_names)):
        raise SystemExit("duplicate Rust variant names")
    write_rust(rows)
    write_sql(rows)
    print(f"wrote {len(rows)} creature factions")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
