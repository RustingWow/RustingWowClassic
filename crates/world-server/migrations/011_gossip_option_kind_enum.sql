ALTER TABLE gossip_options ALTER COLUMN option_kind DROP DEFAULT;

CREATE TYPE gossip_option_kind AS ENUM (
    'ARMORER',
    'AUCTIONEER',
    'BANKER',
    'BATTLEFIELD',
    'BOT',
    'GOSSIP',
    'INNKEEPER',
    'NONE',
    'PETITIONER',
    'QUESTGIVER',
    'SPIRITGUIDE',
    'SPIRITHEALER',
    'STABLEPET',
    'TABARDDESIGNER',
    'TAXIVENDOR',
    'TRAINER',
    'UNLEARN_PET_SKILLS',
    'UNLEARN_TALENTS',
    'VENDOR'
);

ALTER TABLE gossip_options
    ALTER COLUMN option_kind TYPE gossip_option_kind
    USING (CASE option_kind
        WHEN 0 THEN 'NONE'::gossip_option_kind
        WHEN 1 THEN 'GOSSIP'::gossip_option_kind
        WHEN 2 THEN 'QUESTGIVER'::gossip_option_kind
        WHEN 3 THEN 'VENDOR'::gossip_option_kind
        WHEN 4 THEN 'TAXIVENDOR'::gossip_option_kind
        WHEN 5 THEN 'TRAINER'::gossip_option_kind
        WHEN 6 THEN 'SPIRITHEALER'::gossip_option_kind
        WHEN 7 THEN 'SPIRITGUIDE'::gossip_option_kind
        WHEN 8 THEN 'INNKEEPER'::gossip_option_kind
        WHEN 9 THEN 'BANKER'::gossip_option_kind
        WHEN 10 THEN 'PETITIONER'::gossip_option_kind
        WHEN 11 THEN 'TABARDDESIGNER'::gossip_option_kind
        WHEN 12 THEN 'BATTLEFIELD'::gossip_option_kind
        WHEN 13 THEN 'AUCTIONEER'::gossip_option_kind
        WHEN 14 THEN 'STABLEPET'::gossip_option_kind
        WHEN 15 THEN 'ARMORER'::gossip_option_kind
        WHEN 16 THEN 'UNLEARN_TALENTS'::gossip_option_kind
        WHEN 17 THEN 'UNLEARN_PET_SKILLS'::gossip_option_kind
        WHEN 99 THEN 'BOT'::gossip_option_kind
    END);

ALTER TABLE gossip_options ALTER COLUMN option_kind SET DEFAULT 'NONE';
