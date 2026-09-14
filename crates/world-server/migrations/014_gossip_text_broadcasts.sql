-- npc_text_broadcast_text: same gossip text_id as gossip_texts, 8 slots → broadcast_texts.
CREATE TABLE gossip_text_broadcasts (
    text_id integer PRIMARY KEY,
    probability_0 real NOT NULL DEFAULT 0,
    probability_1 real NOT NULL DEFAULT 0,
    probability_2 real NOT NULL DEFAULT 0,
    probability_3 real NOT NULL DEFAULT 0,
    probability_4 real NOT NULL DEFAULT 0,
    probability_5 real NOT NULL DEFAULT 0,
    probability_6 real NOT NULL DEFAULT 0,
    probability_7 real NOT NULL DEFAULT 0,
    broadcast_text_id_0 integer NOT NULL DEFAULT 0,
    broadcast_text_id_1 integer NOT NULL DEFAULT 0,
    broadcast_text_id_2 integer NOT NULL DEFAULT 0,
    broadcast_text_id_3 integer NOT NULL DEFAULT 0,
    broadcast_text_id_4 integer NOT NULL DEFAULT 0,
    broadcast_text_id_5 integer NOT NULL DEFAULT 0,
    broadcast_text_id_6 integer NOT NULL DEFAULT 0,
    broadcast_text_id_7 integer NOT NULL DEFAULT 0
);
