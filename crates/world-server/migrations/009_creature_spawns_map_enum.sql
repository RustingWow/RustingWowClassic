ALTER TABLE creature_spawns
    ALTER COLUMN map_id TYPE character_map
    USING (CASE map_id
        WHEN 0 THEN 'EASTERN_KINGDOMS'::character_map
        WHEN 1 THEN 'KALIMDOR'::character_map
        WHEN 13 THEN 'TESTING'::character_map
        WHEN 25 THEN 'SCOTT_TEST'::character_map
        WHEN 29 THEN 'CASH_TEST'::character_map
        WHEN 30 THEN 'ALTERAC_VALLEY'::character_map
        WHEN 33 THEN 'SHADOWFANG_KEEP'::character_map
        WHEN 34 THEN 'STORMWIND_STOCKADE'::character_map
        WHEN 35 THEN 'STORMWIND_PRISON'::character_map
        WHEN 36 THEN 'DEADMINES'::character_map
        WHEN 37 THEN 'AZSHARA_CRATER'::character_map
        WHEN 42 THEN 'COLLINS_TEST'::character_map
        WHEN 43 THEN 'WAILING_CAVERNS'::character_map
        WHEN 44 THEN 'MONASTERY_UNUSED'::character_map
        WHEN 47 THEN 'RAZORFEN_KRAUL'::character_map
        WHEN 48 THEN 'BLACKFATHOM_DEEPS'::character_map
        WHEN 70 THEN 'ULDAMAN'::character_map
        WHEN 90 THEN 'GNOMEREGAN'::character_map
        WHEN 109 THEN 'SUNKEN_TEMPLE'::character_map
        WHEN 129 THEN 'RAZORFEN_DOWNS'::character_map
        WHEN 169 THEN 'EMERALD_DREAM'::character_map
        WHEN 189 THEN 'SCARLET_MONASTERY'::character_map
        WHEN 209 THEN 'ZUL_FARRAK'::character_map
        WHEN 229 THEN 'BLACKROCK_SPIRE'::character_map
        WHEN 230 THEN 'BLACKROCK_DEPTHS'::character_map
        WHEN 249 THEN 'ONYXIAS_LAIR'::character_map
        WHEN 269 THEN 'OPENING_OF_THE_DARK_PORTAL'::character_map
        WHEN 289 THEN 'SCHOLOMANCE'::character_map
        WHEN 309 THEN 'ZUL_GURUB'::character_map
        WHEN 329 THEN 'STRATHOLME'::character_map
        WHEN 349 THEN 'MARAUDON'::character_map
        WHEN 369 THEN 'DEEPRUN_TRAM'::character_map
        WHEN 389 THEN 'RAGEFIRE_CHASM'::character_map
        WHEN 409 THEN 'MOLTEN_CORE'::character_map
        WHEN 429 THEN 'DIRE_MAUL'::character_map
        WHEN 449 THEN 'ALLIANCE_PVP_BARRACKS'::character_map
        WHEN 450 THEN 'HORDE_PVP_BARRACKS'::character_map
        WHEN 451 THEN 'DEVELOPMENT_LAND'::character_map
        WHEN 469 THEN 'BLACKWING_LAIR'::character_map
        WHEN 489 THEN 'WARSONG_GULCH'::character_map
        WHEN 509 THEN 'RUINS_OF_AHN_QIRAJ'::character_map
        WHEN 529 THEN 'ARATHI_BASIN'::character_map
        WHEN 531 THEN 'AHN_QIRAJ_TEMPLE'::character_map
        WHEN 533 THEN 'NAXXRAMAS'::character_map
    END);
