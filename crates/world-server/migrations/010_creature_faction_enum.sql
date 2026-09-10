ALTER TABLE creature_types ALTER COLUMN faction DROP DEFAULT;

CREATE TYPE creature_faction AS ENUM (
    'ALLIANCE_GENERIC',
    'ALLIANCE_GENERIC_1054',
    'ALLIANCE_GENERIC_1055',
    'ALLIANCE_GENERIC_1315',
    'ALLIANCE_GENERIC_210',
    'ALLIANCE_GENERIC_534',
    'ALLIANCE_GENERIC_694',
    'AMBIENT',
    'AMBIENT_190',
    'ARGENT_DAWN',
    'ARGENT_DAWN_1624',
    'ARGENT_DAWN_1625',
    'ARGENT_DAWN_814',
    'ARMIES_OF_CTHUN',
    'BASILISK',
    'BASILISK_410',
    'BATTLEGROUND_NEUTRAL',
    'BEAST_BAT',
    'BEAST_BEAR',
    'BEAST_BOAR',
    'BEAST_CARRION_BIRD',
    'BEAST_GORILLA',
    'BEAST_RAPTOR',
    'BEAST_SPIDER',
    'BEAST_SPIDER_312',
    'BEAST_WOLF',
    'BEAST_WOLF_38',
    'BLACKFATHOM',
    'BLACKFATHOM_350',
    'BLOODSAIL_BUCCANEERS',
    'BLUE',
    'BOOTY_BAY',
    'BOOTY_BAY_121',
    'BOOTY_BAY_390',
    'BROOD_OF_NOZDORMU',
    'BROOD_OF_NOZDORMU_1601',
    'BURNING_BLADE',
    'CENARION_CIRCLE',
    'CENARION_CIRCLE_1254',
    'CENARION_CIRCLE_1608',
    'CENARION_CIRCLE_994',
    'CENARION_CIRCLE_996',
    'CENTAUR_GALAK',
    'CENTAUR_KOLKAR',
    'CENTAUR_KOLKAR_655',
    'CREATURE',
    'CREATURE_1274',
    'CREATURE_1275',
    'CREATURE_15',
    'CREATURE_1606',
    'CREATURE_1607',
    'CREATURE_189',
    'CREATURE_58',
    'DALARAN',
    'DARKHOWL',
    'DARKMOON_FAIRE',
    'DARKSPEAR_TROLLS',
    'DARKSPEAR_TROLLS_876',
    'DARKSPEAR_TROLLS_877',
    'DARK_IRON_DWARVES',
    'DARK_IRON_DWARVES_674',
    'DARK_IRON_DWARVES_734',
    'DARK_IRON_DWARVES_754',
    'DARNASSUS',
    'DARNASSUS_1076',
    'DARNASSUS_1097',
    'DARNASSUS_124',
    'DARNASSUS_1594',
    'DARNASSUS_1600',
    'DARNASSUS_80',
    'DEFIAS_BROTHERHOOD',
    'DEFIAS_BROTHERHOOD_27',
    'DEFIAS_BROTHERHOOD_34',
    'DEMON',
    'DEMON_954',
    'DRAGONFLIGHT_BLACK',
    'DRAGONFLIGHT_BLACK_1394',
    'DRAGONFLIGHT_BLACK_BAIT',
    'DRAGONFLIGHT_BRONZE',
    'DRAGONFLIGHT_GREEN',
    'DRAGONFLIGHT_RED',
    'ELEMENTAL',
    'ELEMENTAL_1081',
    'ELEMENTAL_834',
    'ENEMY',
    'ENEMY_1620',
    'ESCORTEE',
    'ESCORTEE_113',
    'ESCORTEE_231',
    'ESCORTEE_232',
    'ESCORTEE_250',
    'ESCORTEE_290',
    'ESCORTEE_33',
    'ESCORTEE_495',
    'ESCORTEE_774',
    'ESCORTEE_775',
    'EVERLOOK',
    'EVERLOOK_855',
    'EXODAR',
    'EXODAR_1639',
    'EXODAR_1640',
    'FARSTRIDERS',
    'FARSTRIDERS_1636',
    'FARSTRIDERS_1637',
    'FORLORN_SPIRIT',
    'FRIENDLY',
    'FRIENDLY_1080',
    'FROSTWOLF_CLAN',
    'FROSTWOLF_CLAN_1215',
    'FROSTWOLF_CLAN_1335',
    'FROSTWOLF_CLAN_1554',
    'FROSTWOLF_CLAN_1597',
    'FURBOLG',
    'FURBOLG_UNCORRUPTED',
    'GADGETZAN',
    'GADGETZAN_475',
    'GELKIS_CLAN_CENTAUR',
    'GIANT',
    'GIZLOCK',
    'GIZLOCKS_CHARM',
    'GIZLOCKS_DUMMY',
    'GNOLL_MOSSHIDE',
    'GNOLL_MUDSNOUT',
    'GNOLL_REDRIDGE',
    'GNOLL_RIVERPAW',
    'GNOLL_ROTHIDE',
    'GNOLL_SHADOWHIDE',
    'GNOMEREGAN_BUG',
    'GNOMEREGAN_EXILES',
    'GNOMEREGAN_EXILES_64',
    'GNOMEREGAN_EXILES_875',
    'GNOME_LEPER',
    'GOBLIN_DARK_IRON_BAR_PATRON',
    'GOBLIN_DARK_IRON_BAR_PATRON_736',
    'GRELL',
    'HARPY',
    'HILLSBRAD_MILITIA',
    'HILLSBRAD_SOUTHSHORE_MAYOR',
    'HORDE_GENERIC',
    'HORDE_GENERIC_1034',
    'HORDE_GENERIC_106',
    'HORDE_GENERIC_1314',
    'HORDE_GENERIC_1494',
    'HORDE_GENERIC_1495',
    'HORDE_GENERIC_1496',
    'HORDE_GENERIC_714',
    'HUMAN_NIGHT_WATCH',
    'HUMAN_NIGHT_WATCH_56',
    'HYDRAXIAN_WATERLORDS',
    'IRONFORGE',
    'IRONFORGE_122',
    'IRONFORGE_1611',
    'IRONFORGE_1618',
    'IRONFORGE_57',
    'JAEDENAR',
    'KOBOLD',
    'KOBOLD_26',
    'KURZENS_MERCENARIES',
    'LEOPARD',
    'LOST_ONES',
    'MAGRAM_CLAN_CENTAUR',
    'MAKRURA',
    'MARAUDINE',
    'MIGHT_OF_KALIMDOR',
    'MIGHT_OF_KALIMDOR_1613',
    'MONSTER',
    'MONSTER_148',
    'MONSTER_16',
    'MONSTER_1614',
    'MONSTER_634',
    'MONSTER_93',
    'MURLOC',
    'NAGA',
    'NETHERGARDE_CARAVAN',
    'NETHERGARDE_CARAVAN_209',
    'OGRE',
    'OGRE_CAPTAIN_KROMCRUSH',
    'ORC_BLACKROCK',
    'ORC_DRAGONMAW',
    'ORGRIMMAR',
    'ORGRIMMAR_1074',
    'ORGRIMMAR_1174',
    'ORGRIMMAR_125',
    'ORGRIMMAR_1595',
    'ORGRIMMAR_1612',
    'ORGRIMMAR_1619',
    'ORGRIMMAR_65',
    'ORGRIMMAR_85',
    'PLAYER_BLOOD_ELF',
    'PLAYER_DRAENEI',
    'PLAYER_DWARF',
    'PLAYER_GNOME',
    'PLAYER_HUMAN',
    'PLAYER_NIGHT_ELF',
    'PLAYER_ORC',
    'PLAYER_TAUREN',
    'PLAYER_TROLL',
    'PLAYER_UNDEAD',
    'PREY',
    'QUILBOAR_BRISTLEBACK',
    'QUILBOAR_BRISTLEBACK_112',
    'QUILBOAR_DEATHSHEAD',
    'QUILBOAR_RAZORFEN',
    'QUILBOAR_RAZORFEN_153',
    'QUILBOAR_RAZORMANE_2',
    'QUILBOAR_RAZORMANE_2_110',
    'RATCHET',
    'RATCHET_637',
    'RAVENHOLDT',
    'RAVENHOLDT_473',
    'RC_ENEMIES',
    'RC_OBJECTS',
    'RED',
    'SCARLET_CRUSADE',
    'SCARLET_CRUSADE_89',
    'SCORPID',
    'SCOURGE_INVADERS',
    'SCOURGE_INVADERS_1634',
    'SEARING_SPIDER',
    'SHADOWSILK_POACHER',
    'SHATTERSPEAR_TROLLS',
    'SHATTERSPEAR_TROLLS_1015',
    'SHENDRALAR',
    'SHENDRALAR_1355',
    'SILITHID',
    'SILITHID_311',
    'SILITHID_ATTACKERS',
    'SILVERMOON_CITY',
    'SILVERMOON_CITY_1603',
    'SILVERMOON_CITY_1604',
    'SILVERMOON_REMNANT',
    'SILVERMOON_REMNANT_1576',
    'SILVERWING_SENTINELS',
    'SILVERWING_SENTINELS_1642',
    'SOUTHSEA_FREEBOOTERS',
    'SPIRIT',
    'SPIRIT_GUIDE_ALLIANCE',
    'SPIRIT_GUIDE_HORDE',
    'STEAMWHEEDLE_CARTEL',
    'STEAMWHEEDLE_CARTEL_1635',
    'STORMPIKE_GUARD',
    'STORMPIKE_GUARD_1217',
    'STORMPIKE_GUARD_1334',
    'STORMPIKE_GUARD_1534',
    'STORMPIKE_GUARD_1596',
    'STORMWIND',
    'STORMWIND_1078',
    'STORMWIND_12',
    'STORMWIND_123',
    'STORMWIND_1575',
    'SULFURON_FIRELORDS',
    'SULFURON_FIRELORDS_1235',
    'SULFURON_FIRELORDS_1236',
    'SYNDICATE',
    'SYNDICATE_108',
    'SYNDICATE_472',
    'SYNDICATE_97',
    'TASKMASTER_FIZZULE',
    'THERAMORE',
    'THERAMORE_1075',
    'THERAMORE_1077',
    'THERAMORE_1096',
    'THERAMORE_150',
    'THERAMORE_151',
    'THERAMORE_894',
    'THE_DEFILERS',
    'THE_DEFILERS_1598',
    'THE_LEAGUE_OF_ARATHOR',
    'THE_LEAGUE_OF_ARATHOR_1599',
    'THORIUM_BROTHERHOOD',
    'THORIUM_BROTHERHOOD_1475',
    'THUNDER_BLUFF',
    'THUNDER_BLUFF_105',
    'THUNDER_BLUFF_995',
    'TIMBERMAW_HOLD',
    'TIMBERMAW_HOLD_636',
    'TITAN',
    'TITAN_416',
    'TITAN_470',
    'TRAINING_DUMMY',
    'TRAINING_DUMMY_1095',
    'TRANQUILLIEN',
    'TRANQUILLIEN_1628',
    'TREASURE',
    'TREASURE_100',
    'TREASURE_101',
    'TREASURE_102',
    'TREASURE_114',
    'TREASURE_1375',
    'TROGG',
    'TROGG_59',
    'TROGG_594',
    'TROLL_BLOODSCALP',
    'TROLL_FROSTMANE',
    'TROLL_FROSTMANE_107',
    'TROLL_SKULLSPLITTER',
    'TROLL_VILEBRANCH',
    'TROLL_WITHERBARK',
    'UNDEAD_SCOURGE',
    'UNDEAD_SCOURGE_1626',
    'UNDEAD_SCOURGE_233',
    'UNDEAD_SCOURGE_974',
    'UNDERCITY',
    'UNDERCITY_1134',
    'UNDERCITY_1154',
    'UNDERCITY_118',
    'UNDERCITY_71',
    'UNDERCITY_98',
    'VENTURE_COMPANY',
    'VICTIM',
    'VICTIM_1454',
    'VICTIM_614',
    'VICTIM_99',
    'VILLIAN',
    'VILLIAN_127',
    'VILLIAN_43',
    'WAILING_CAVERNS',
    'WAILING_CAVERNS_330',
    'WAILING_CAVERNS_450',
    'WARSONG_OUTRIDERS',
    'WARSONG_OUTRIDERS_1641',
    'WINTERSABER_TRAINERS',
    'WORGEN',
    'ZANDALAR_TRIBE'
);

ALTER TABLE creature_types
    ALTER COLUMN faction TYPE creature_faction
    USING (CASE faction
        WHEN 1 THEN 'PLAYER_HUMAN'::creature_faction
        WHEN 2 THEN 'PLAYER_ORC'::creature_faction
        WHEN 3 THEN 'PLAYER_DWARF'::creature_faction
        WHEN 4 THEN 'PLAYER_NIGHT_ELF'::creature_faction
        WHEN 5 THEN 'PLAYER_UNDEAD'::creature_faction
        WHEN 6 THEN 'PLAYER_TAUREN'::creature_faction
        WHEN 7 THEN 'CREATURE'::creature_faction
        WHEN 10 THEN 'ESCORTEE'::creature_faction
        WHEN 11 THEN 'STORMWIND'::creature_faction
        WHEN 12 THEN 'STORMWIND_12'::creature_faction
        WHEN 14 THEN 'MONSTER'::creature_faction
        WHEN 15 THEN 'CREATURE_15'::creature_faction
        WHEN 16 THEN 'MONSTER_16'::creature_faction
        WHEN 17 THEN 'DEFIAS_BROTHERHOOD'::creature_faction
        WHEN 18 THEN 'MURLOC'::creature_faction
        WHEN 19 THEN 'GNOLL_REDRIDGE'::creature_faction
        WHEN 20 THEN 'GNOLL_RIVERPAW'::creature_faction
        WHEN 21 THEN 'UNDEAD_SCOURGE'::creature_faction
        WHEN 22 THEN 'BEAST_SPIDER'::creature_faction
        WHEN 23 THEN 'GNOMEREGAN_EXILES'::creature_faction
        WHEN 24 THEN 'WORGEN'::creature_faction
        WHEN 25 THEN 'KOBOLD'::creature_faction
        WHEN 26 THEN 'KOBOLD_26'::creature_faction
        WHEN 27 THEN 'DEFIAS_BROTHERHOOD_27'::creature_faction
        WHEN 28 THEN 'TROLL_BLOODSCALP'::creature_faction
        WHEN 29 THEN 'ORGRIMMAR'::creature_faction
        WHEN 30 THEN 'TROLL_SKULLSPLITTER'::creature_faction
        WHEN 31 THEN 'PREY'::creature_faction
        WHEN 32 THEN 'BEAST_WOLF'::creature_faction
        WHEN 33 THEN 'ESCORTEE_33'::creature_faction
        WHEN 34 THEN 'DEFIAS_BROTHERHOOD_34'::creature_faction
        WHEN 35 THEN 'FRIENDLY'::creature_faction
        WHEN 36 THEN 'TROGG'::creature_faction
        WHEN 37 THEN 'TROLL_FROSTMANE'::creature_faction
        WHEN 38 THEN 'BEAST_WOLF_38'::creature_faction
        WHEN 39 THEN 'GNOLL_SHADOWHIDE'::creature_faction
        WHEN 40 THEN 'ORC_BLACKROCK'::creature_faction
        WHEN 41 THEN 'VILLIAN'::creature_faction
        WHEN 42 THEN 'VICTIM'::creature_faction
        WHEN 43 THEN 'VILLIAN_43'::creature_faction
        WHEN 44 THEN 'BEAST_BEAR'::creature_faction
        WHEN 45 THEN 'OGRE'::creature_faction
        WHEN 46 THEN 'KURZENS_MERCENARIES'::creature_faction
        WHEN 47 THEN 'VENTURE_COMPANY'::creature_faction
        WHEN 48 THEN 'BEAST_RAPTOR'::creature_faction
        WHEN 49 THEN 'BASILISK'::creature_faction
        WHEN 50 THEN 'DRAGONFLIGHT_GREEN'::creature_faction
        WHEN 51 THEN 'LOST_ONES'::creature_faction
        WHEN 52 THEN 'GIZLOCKS_DUMMY'::creature_faction
        WHEN 53 THEN 'HUMAN_NIGHT_WATCH'::creature_faction
        WHEN 54 THEN 'DARK_IRON_DWARVES'::creature_faction
        WHEN 55 THEN 'IRONFORGE'::creature_faction
        WHEN 56 THEN 'HUMAN_NIGHT_WATCH_56'::creature_faction
        WHEN 57 THEN 'IRONFORGE_57'::creature_faction
        WHEN 58 THEN 'CREATURE_58'::creature_faction
        WHEN 59 THEN 'TROGG_59'::creature_faction
        WHEN 60 THEN 'DRAGONFLIGHT_RED'::creature_faction
        WHEN 61 THEN 'GNOLL_MOSSHIDE'::creature_faction
        WHEN 62 THEN 'ORC_DRAGONMAW'::creature_faction
        WHEN 63 THEN 'GNOME_LEPER'::creature_faction
        WHEN 64 THEN 'GNOMEREGAN_EXILES_64'::creature_faction
        WHEN 65 THEN 'ORGRIMMAR_65'::creature_faction
        WHEN 66 THEN 'LEOPARD'::creature_faction
        WHEN 67 THEN 'SCARLET_CRUSADE'::creature_faction
        WHEN 68 THEN 'UNDERCITY'::creature_faction
        WHEN 69 THEN 'RATCHET'::creature_faction
        WHEN 70 THEN 'GNOLL_ROTHIDE'::creature_faction
        WHEN 71 THEN 'UNDERCITY_71'::creature_faction
        WHEN 72 THEN 'BEAST_GORILLA'::creature_faction
        WHEN 73 THEN 'BEAST_CARRION_BIRD'::creature_faction
        WHEN 74 THEN 'NAGA'::creature_faction
        WHEN 76 THEN 'DALARAN'::creature_faction
        WHEN 77 THEN 'FORLORN_SPIRIT'::creature_faction
        WHEN 78 THEN 'DARKHOWL'::creature_faction
        WHEN 79 THEN 'DARNASSUS'::creature_faction
        WHEN 80 THEN 'DARNASSUS_80'::creature_faction
        WHEN 81 THEN 'GRELL'::creature_faction
        WHEN 82 THEN 'FURBOLG'::creature_faction
        WHEN 83 THEN 'HORDE_GENERIC'::creature_faction
        WHEN 84 THEN 'ALLIANCE_GENERIC'::creature_faction
        WHEN 85 THEN 'ORGRIMMAR_85'::creature_faction
        WHEN 86 THEN 'GIZLOCKS_CHARM'::creature_faction
        WHEN 87 THEN 'SYNDICATE'::creature_faction
        WHEN 88 THEN 'HILLSBRAD_MILITIA'::creature_faction
        WHEN 89 THEN 'SCARLET_CRUSADE_89'::creature_faction
        WHEN 90 THEN 'DEMON'::creature_faction
        WHEN 91 THEN 'ELEMENTAL'::creature_faction
        WHEN 92 THEN 'SPIRIT'::creature_faction
        WHEN 93 THEN 'MONSTER_93'::creature_faction
        WHEN 94 THEN 'TREASURE'::creature_faction
        WHEN 95 THEN 'GNOLL_MUDSNOUT'::creature_faction
        WHEN 96 THEN 'HILLSBRAD_SOUTHSHORE_MAYOR'::creature_faction
        WHEN 97 THEN 'SYNDICATE_97'::creature_faction
        WHEN 98 THEN 'UNDERCITY_98'::creature_faction
        WHEN 99 THEN 'VICTIM_99'::creature_faction
        WHEN 100 THEN 'TREASURE_100'::creature_faction
        WHEN 101 THEN 'TREASURE_101'::creature_faction
        WHEN 102 THEN 'TREASURE_102'::creature_faction
        WHEN 103 THEN 'DRAGONFLIGHT_BLACK'::creature_faction
        WHEN 104 THEN 'THUNDER_BLUFF'::creature_faction
        WHEN 105 THEN 'THUNDER_BLUFF_105'::creature_faction
        WHEN 106 THEN 'HORDE_GENERIC_106'::creature_faction
        WHEN 107 THEN 'TROLL_FROSTMANE_107'::creature_faction
        WHEN 108 THEN 'SYNDICATE_108'::creature_faction
        WHEN 109 THEN 'QUILBOAR_RAZORMANE_2'::creature_faction
        WHEN 110 THEN 'QUILBOAR_RAZORMANE_2_110'::creature_faction
        WHEN 111 THEN 'QUILBOAR_BRISTLEBACK'::creature_faction
        WHEN 112 THEN 'QUILBOAR_BRISTLEBACK_112'::creature_faction
        WHEN 113 THEN 'ESCORTEE_113'::creature_faction
        WHEN 114 THEN 'TREASURE_114'::creature_faction
        WHEN 115 THEN 'PLAYER_GNOME'::creature_faction
        WHEN 116 THEN 'PLAYER_TROLL'::creature_faction
        WHEN 118 THEN 'UNDERCITY_118'::creature_faction
        WHEN 119 THEN 'BLOODSAIL_BUCCANEERS'::creature_faction
        WHEN 120 THEN 'BOOTY_BAY'::creature_faction
        WHEN 121 THEN 'BOOTY_BAY_121'::creature_faction
        WHEN 122 THEN 'IRONFORGE_122'::creature_faction
        WHEN 123 THEN 'STORMWIND_123'::creature_faction
        WHEN 124 THEN 'DARNASSUS_124'::creature_faction
        WHEN 125 THEN 'ORGRIMMAR_125'::creature_faction
        WHEN 126 THEN 'DARKSPEAR_TROLLS'::creature_faction
        WHEN 127 THEN 'VILLIAN_127'::creature_faction
        WHEN 128 THEN 'BLACKFATHOM'::creature_faction
        WHEN 129 THEN 'MAKRURA'::creature_faction
        WHEN 130 THEN 'CENTAUR_KOLKAR'::creature_faction
        WHEN 131 THEN 'CENTAUR_GALAK'::creature_faction
        WHEN 132 THEN 'GELKIS_CLAN_CENTAUR'::creature_faction
        WHEN 133 THEN 'MAGRAM_CLAN_CENTAUR'::creature_faction
        WHEN 134 THEN 'MARAUDINE'::creature_faction
        WHEN 148 THEN 'MONSTER_148'::creature_faction
        WHEN 149 THEN 'THERAMORE'::creature_faction
        WHEN 150 THEN 'THERAMORE_150'::creature_faction
        WHEN 151 THEN 'THERAMORE_151'::creature_faction
        WHEN 152 THEN 'QUILBOAR_RAZORFEN'::creature_faction
        WHEN 153 THEN 'QUILBOAR_RAZORFEN_153'::creature_faction
        WHEN 154 THEN 'QUILBOAR_DEATHSHEAD'::creature_faction
        WHEN 168 THEN 'ENEMY'::creature_faction
        WHEN 188 THEN 'AMBIENT'::creature_faction
        WHEN 189 THEN 'CREATURE_189'::creature_faction
        WHEN 190 THEN 'AMBIENT_190'::creature_faction
        WHEN 208 THEN 'NETHERGARDE_CARAVAN'::creature_faction
        WHEN 209 THEN 'NETHERGARDE_CARAVAN_209'::creature_faction
        WHEN 210 THEN 'ALLIANCE_GENERIC_210'::creature_faction
        WHEN 230 THEN 'SOUTHSEA_FREEBOOTERS'::creature_faction
        WHEN 231 THEN 'ESCORTEE_231'::creature_faction
        WHEN 232 THEN 'ESCORTEE_232'::creature_faction
        WHEN 233 THEN 'UNDEAD_SCOURGE_233'::creature_faction
        WHEN 250 THEN 'ESCORTEE_250'::creature_faction
        WHEN 270 THEN 'WAILING_CAVERNS'::creature_faction
        WHEN 290 THEN 'ESCORTEE_290'::creature_faction
        WHEN 310 THEN 'SILITHID'::creature_faction
        WHEN 311 THEN 'SILITHID_311'::creature_faction
        WHEN 312 THEN 'BEAST_SPIDER_312'::creature_faction
        WHEN 330 THEN 'WAILING_CAVERNS_330'::creature_faction
        WHEN 350 THEN 'BLACKFATHOM_350'::creature_faction
        WHEN 370 THEN 'ARMIES_OF_CTHUN'::creature_faction
        WHEN 371 THEN 'SILVERMOON_REMNANT'::creature_faction
        WHEN 390 THEN 'BOOTY_BAY_390'::creature_faction
        WHEN 410 THEN 'BASILISK_410'::creature_faction
        WHEN 411 THEN 'BEAST_BAT'::creature_faction
        WHEN 412 THEN 'THE_DEFILERS'::creature_faction
        WHEN 413 THEN 'SCORPID'::creature_faction
        WHEN 414 THEN 'TIMBERMAW_HOLD'::creature_faction
        WHEN 415 THEN 'TITAN'::creature_faction
        WHEN 416 THEN 'TITAN_416'::creature_faction
        WHEN 430 THEN 'TASKMASTER_FIZZULE'::creature_faction
        WHEN 450 THEN 'WAILING_CAVERNS_450'::creature_faction
        WHEN 470 THEN 'TITAN_470'::creature_faction
        WHEN 471 THEN 'RAVENHOLDT'::creature_faction
        WHEN 472 THEN 'SYNDICATE_472'::creature_faction
        WHEN 473 THEN 'RAVENHOLDT_473'::creature_faction
        WHEN 474 THEN 'GADGETZAN'::creature_faction
        WHEN 475 THEN 'GADGETZAN_475'::creature_faction
        WHEN 494 THEN 'GNOMEREGAN_BUG'::creature_faction
        WHEN 495 THEN 'ESCORTEE_495'::creature_faction
        WHEN 514 THEN 'HARPY'::creature_faction
        WHEN 534 THEN 'ALLIANCE_GENERIC_534'::creature_faction
        WHEN 554 THEN 'BURNING_BLADE'::creature_faction
        WHEN 574 THEN 'SHADOWSILK_POACHER'::creature_faction
        WHEN 575 THEN 'SEARING_SPIDER'::creature_faction
        WHEN 594 THEN 'TROGG_594'::creature_faction
        WHEN 614 THEN 'VICTIM_614'::creature_faction
        WHEN 634 THEN 'MONSTER_634'::creature_faction
        WHEN 635 THEN 'CENARION_CIRCLE'::creature_faction
        WHEN 636 THEN 'TIMBERMAW_HOLD_636'::creature_faction
        WHEN 637 THEN 'RATCHET_637'::creature_faction
        WHEN 654 THEN 'TROLL_WITHERBARK'::creature_faction
        WHEN 655 THEN 'CENTAUR_KOLKAR_655'::creature_faction
        WHEN 674 THEN 'DARK_IRON_DWARVES_674'::creature_faction
        WHEN 694 THEN 'ALLIANCE_GENERIC_694'::creature_faction
        WHEN 695 THEN 'HYDRAXIAN_WATERLORDS'::creature_faction
        WHEN 714 THEN 'HORDE_GENERIC_714'::creature_faction
        WHEN 734 THEN 'DARK_IRON_DWARVES_734'::creature_faction
        WHEN 735 THEN 'GOBLIN_DARK_IRON_BAR_PATRON'::creature_faction
        WHEN 736 THEN 'GOBLIN_DARK_IRON_BAR_PATRON_736'::creature_faction
        WHEN 754 THEN 'DARK_IRON_DWARVES_754'::creature_faction
        WHEN 774 THEN 'ESCORTEE_774'::creature_faction
        WHEN 775 THEN 'ESCORTEE_775'::creature_faction
        WHEN 776 THEN 'BROOD_OF_NOZDORMU'::creature_faction
        WHEN 777 THEN 'MIGHT_OF_KALIMDOR'::creature_faction
        WHEN 778 THEN 'GIANT'::creature_faction
        WHEN 794 THEN 'ARGENT_DAWN'::creature_faction
        WHEN 795 THEN 'TROLL_VILEBRANCH'::creature_faction
        WHEN 814 THEN 'ARGENT_DAWN_814'::creature_faction
        WHEN 834 THEN 'ELEMENTAL_834'::creature_faction
        WHEN 854 THEN 'EVERLOOK'::creature_faction
        WHEN 855 THEN 'EVERLOOK_855'::creature_faction
        WHEN 874 THEN 'WINTERSABER_TRAINERS'::creature_faction
        WHEN 875 THEN 'GNOMEREGAN_EXILES_875'::creature_faction
        WHEN 876 THEN 'DARKSPEAR_TROLLS_876'::creature_faction
        WHEN 877 THEN 'DARKSPEAR_TROLLS_877'::creature_faction
        WHEN 894 THEN 'THERAMORE_894'::creature_faction
        WHEN 914 THEN 'TRAINING_DUMMY'::creature_faction
        WHEN 934 THEN 'FURBOLG_UNCORRUPTED'::creature_faction
        WHEN 954 THEN 'DEMON_954'::creature_faction
        WHEN 974 THEN 'UNDEAD_SCOURGE_974'::creature_faction
        WHEN 994 THEN 'CENARION_CIRCLE_994'::creature_faction
        WHEN 995 THEN 'THUNDER_BLUFF_995'::creature_faction
        WHEN 996 THEN 'CENARION_CIRCLE_996'::creature_faction
        WHEN 1014 THEN 'SHATTERSPEAR_TROLLS'::creature_faction
        WHEN 1015 THEN 'SHATTERSPEAR_TROLLS_1015'::creature_faction
        WHEN 1034 THEN 'HORDE_GENERIC_1034'::creature_faction
        WHEN 1054 THEN 'ALLIANCE_GENERIC_1054'::creature_faction
        WHEN 1055 THEN 'ALLIANCE_GENERIC_1055'::creature_faction
        WHEN 1074 THEN 'ORGRIMMAR_1074'::creature_faction
        WHEN 1075 THEN 'THERAMORE_1075'::creature_faction
        WHEN 1076 THEN 'DARNASSUS_1076'::creature_faction
        WHEN 1077 THEN 'THERAMORE_1077'::creature_faction
        WHEN 1078 THEN 'STORMWIND_1078'::creature_faction
        WHEN 1080 THEN 'FRIENDLY_1080'::creature_faction
        WHEN 1081 THEN 'ELEMENTAL_1081'::creature_faction
        WHEN 1094 THEN 'BEAST_BOAR'::creature_faction
        WHEN 1095 THEN 'TRAINING_DUMMY_1095'::creature_faction
        WHEN 1096 THEN 'THERAMORE_1096'::creature_faction
        WHEN 1097 THEN 'DARNASSUS_1097'::creature_faction
        WHEN 1114 THEN 'DRAGONFLIGHT_BLACK_BAIT'::creature_faction
        WHEN 1134 THEN 'UNDERCITY_1134'::creature_faction
        WHEN 1154 THEN 'UNDERCITY_1154'::creature_faction
        WHEN 1174 THEN 'ORGRIMMAR_1174'::creature_faction
        WHEN 1194 THEN 'BATTLEGROUND_NEUTRAL'::creature_faction
        WHEN 1214 THEN 'FROSTWOLF_CLAN'::creature_faction
        WHEN 1215 THEN 'FROSTWOLF_CLAN_1215'::creature_faction
        WHEN 1216 THEN 'STORMPIKE_GUARD'::creature_faction
        WHEN 1217 THEN 'STORMPIKE_GUARD_1217'::creature_faction
        WHEN 1234 THEN 'SULFURON_FIRELORDS'::creature_faction
        WHEN 1235 THEN 'SULFURON_FIRELORDS_1235'::creature_faction
        WHEN 1236 THEN 'SULFURON_FIRELORDS_1236'::creature_faction
        WHEN 1254 THEN 'CENARION_CIRCLE_1254'::creature_faction
        WHEN 1274 THEN 'CREATURE_1274'::creature_faction
        WHEN 1275 THEN 'CREATURE_1275'::creature_faction
        WHEN 1294 THEN 'GIZLOCK'::creature_faction
        WHEN 1314 THEN 'HORDE_GENERIC_1314'::creature_faction
        WHEN 1315 THEN 'ALLIANCE_GENERIC_1315'::creature_faction
        WHEN 1334 THEN 'STORMPIKE_GUARD_1334'::creature_faction
        WHEN 1335 THEN 'FROSTWOLF_CLAN_1335'::creature_faction
        WHEN 1354 THEN 'SHENDRALAR'::creature_faction
        WHEN 1355 THEN 'SHENDRALAR_1355'::creature_faction
        WHEN 1374 THEN 'OGRE_CAPTAIN_KROMCRUSH'::creature_faction
        WHEN 1375 THEN 'TREASURE_1375'::creature_faction
        WHEN 1394 THEN 'DRAGONFLIGHT_BLACK_1394'::creature_faction
        WHEN 1395 THEN 'SILITHID_ATTACKERS'::creature_faction
        WHEN 1414 THEN 'SPIRIT_GUIDE_ALLIANCE'::creature_faction
        WHEN 1415 THEN 'SPIRIT_GUIDE_HORDE'::creature_faction
        WHEN 1434 THEN 'JAEDENAR'::creature_faction
        WHEN 1454 THEN 'VICTIM_1454'::creature_faction
        WHEN 1474 THEN 'THORIUM_BROTHERHOOD'::creature_faction
        WHEN 1475 THEN 'THORIUM_BROTHERHOOD_1475'::creature_faction
        WHEN 1494 THEN 'HORDE_GENERIC_1494'::creature_faction
        WHEN 1495 THEN 'HORDE_GENERIC_1495'::creature_faction
        WHEN 1496 THEN 'HORDE_GENERIC_1496'::creature_faction
        WHEN 1514 THEN 'SILVERWING_SENTINELS'::creature_faction
        WHEN 1515 THEN 'WARSONG_OUTRIDERS'::creature_faction
        WHEN 1534 THEN 'STORMPIKE_GUARD_1534'::creature_faction
        WHEN 1554 THEN 'FROSTWOLF_CLAN_1554'::creature_faction
        WHEN 1555 THEN 'DARKMOON_FAIRE'::creature_faction
        WHEN 1574 THEN 'ZANDALAR_TRIBE'::creature_faction
        WHEN 1575 THEN 'STORMWIND_1575'::creature_faction
        WHEN 1576 THEN 'SILVERMOON_REMNANT_1576'::creature_faction
        WHEN 1577 THEN 'THE_LEAGUE_OF_ARATHOR'::creature_faction
        WHEN 1594 THEN 'DARNASSUS_1594'::creature_faction
        WHEN 1595 THEN 'ORGRIMMAR_1595'::creature_faction
        WHEN 1596 THEN 'STORMPIKE_GUARD_1596'::creature_faction
        WHEN 1597 THEN 'FROSTWOLF_CLAN_1597'::creature_faction
        WHEN 1598 THEN 'THE_DEFILERS_1598'::creature_faction
        WHEN 1599 THEN 'THE_LEAGUE_OF_ARATHOR_1599'::creature_faction
        WHEN 1600 THEN 'DARNASSUS_1600'::creature_faction
        WHEN 1601 THEN 'BROOD_OF_NOZDORMU_1601'::creature_faction
        WHEN 1602 THEN 'SILVERMOON_CITY'::creature_faction
        WHEN 1603 THEN 'SILVERMOON_CITY_1603'::creature_faction
        WHEN 1604 THEN 'SILVERMOON_CITY_1604'::creature_faction
        WHEN 1605 THEN 'DRAGONFLIGHT_BRONZE'::creature_faction
        WHEN 1606 THEN 'CREATURE_1606'::creature_faction
        WHEN 1607 THEN 'CREATURE_1607'::creature_faction
        WHEN 1608 THEN 'CENARION_CIRCLE_1608'::creature_faction
        WHEN 1610 THEN 'PLAYER_BLOOD_ELF'::creature_faction
        WHEN 1611 THEN 'IRONFORGE_1611'::creature_faction
        WHEN 1612 THEN 'ORGRIMMAR_1612'::creature_faction
        WHEN 1613 THEN 'MIGHT_OF_KALIMDOR_1613'::creature_faction
        WHEN 1614 THEN 'MONSTER_1614'::creature_faction
        WHEN 1615 THEN 'STEAMWHEEDLE_CARTEL'::creature_faction
        WHEN 1616 THEN 'RC_OBJECTS'::creature_faction
        WHEN 1617 THEN 'RC_ENEMIES'::creature_faction
        WHEN 1618 THEN 'IRONFORGE_1618'::creature_faction
        WHEN 1619 THEN 'ORGRIMMAR_1619'::creature_faction
        WHEN 1620 THEN 'ENEMY_1620'::creature_faction
        WHEN 1621 THEN 'BLUE'::creature_faction
        WHEN 1622 THEN 'RED'::creature_faction
        WHEN 1623 THEN 'TRANQUILLIEN'::creature_faction
        WHEN 1624 THEN 'ARGENT_DAWN_1624'::creature_faction
        WHEN 1625 THEN 'ARGENT_DAWN_1625'::creature_faction
        WHEN 1626 THEN 'UNDEAD_SCOURGE_1626'::creature_faction
        WHEN 1627 THEN 'FARSTRIDERS'::creature_faction
        WHEN 1628 THEN 'TRANQUILLIEN_1628'::creature_faction
        WHEN 1629 THEN 'PLAYER_DRAENEI'::creature_faction
        WHEN 1630 THEN 'SCOURGE_INVADERS'::creature_faction
        WHEN 1634 THEN 'SCOURGE_INVADERS_1634'::creature_faction
        WHEN 1635 THEN 'STEAMWHEEDLE_CARTEL_1635'::creature_faction
        WHEN 1636 THEN 'FARSTRIDERS_1636'::creature_faction
        WHEN 1637 THEN 'FARSTRIDERS_1637'::creature_faction
        WHEN 1638 THEN 'EXODAR'::creature_faction
        WHEN 1639 THEN 'EXODAR_1639'::creature_faction
        WHEN 1640 THEN 'EXODAR_1640'::creature_faction
        WHEN 1641 THEN 'WARSONG_OUTRIDERS_1641'::creature_faction
        WHEN 1642 THEN 'SILVERWING_SENTINELS_1642'::creature_faction
    END);
