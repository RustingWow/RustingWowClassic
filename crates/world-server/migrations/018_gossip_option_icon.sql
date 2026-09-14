ALTER TABLE gossip_options ALTER COLUMN icon DROP DEFAULT;

CREATE TYPE gossip_option_icon AS ENUM (
    'BATTLE',
    'CHAT',
    'CHAT_11',
    'CHAT_12',
    'DOT',
    'INTERACT_1',
    'INTERACT_2',
    'MONEY_BAG',
    'TABARD',
    'TALK',
    'TAXI',
    'TRAINER',
    'VENDOR'
);

ALTER TABLE gossip_options
    ALTER COLUMN icon TYPE gossip_option_icon
    USING (CASE icon
        WHEN 0 THEN 'CHAT'::gossip_option_icon
        WHEN 1 THEN 'VENDOR'::gossip_option_icon
        WHEN 2 THEN 'TAXI'::gossip_option_icon
        WHEN 3 THEN 'TRAINER'::gossip_option_icon
        WHEN 4 THEN 'INTERACT_1'::gossip_option_icon
        WHEN 5 THEN 'INTERACT_2'::gossip_option_icon
        WHEN 6 THEN 'MONEY_BAG'::gossip_option_icon
        WHEN 7 THEN 'TALK'::gossip_option_icon
        WHEN 8 THEN 'TABARD'::gossip_option_icon
        WHEN 9 THEN 'BATTLE'::gossip_option_icon
        WHEN 10 THEN 'DOT'::gossip_option_icon
        WHEN 11 THEN 'CHAT_11'::gossip_option_icon
        WHEN 12 THEN 'CHAT_12'::gossip_option_icon
        ELSE 'CHAT'::gossip_option_icon
    END);

ALTER TABLE gossip_options ALTER COLUMN icon SET DEFAULT 'CHAT';
