CREATE TABLE race_start_positions (
    race SMALLINT PRIMARY KEY,
    map_id INTEGER NOT NULL,
    x REAL NOT NULL,
    y REAL NOT NULL,
    z REAL NOT NULL,
    orientation REAL NOT NULL,
    area INTEGER NOT NULL
);

INSERT INTO race_start_positions (race, map_id, x, y, z, orientation, area) VALUES
    (1, 0, -8949.95, -132.493, 83.5312, 0, 9),
    (2, 1, -618.518, -4251.67, 38.718, 0, 363),
    (3, 0, -6240.32, 331.033, 382.758, 6.17716, 132),
    (4, 1, 10311.3, 832.463, 1326.41, 5.69632, 188),
    (5, 0, 1676.71, 1678.31, 121.67, 2.70526, 154),
    (6, 1, -2917.58, -257.98, 52.9968, 0, 221),
    (7, 0, -6240.32, 331.033, 382.758, 6.17716, 132),
    (8, 1, -618.518, -4251.67, 38.718, 0, 363);

CREATE TABLE characters (
    id BIGSERIAL PRIMARY KEY,
    account_id BIGINT NOT NULL,
    name VARCHAR(12) NOT NULL,
    race SMALLINT NOT NULL,
    class SMALLINT NOT NULL,
    gender SMALLINT NOT NULL,
    skin SMALLINT NOT NULL,
    face SMALLINT NOT NULL,
    hair_style SMALLINT NOT NULL,
    hair_color SMALLINT NOT NULL,
    facial_hair SMALLINT NOT NULL,
    map_id INTEGER NOT NULL,
    x REAL NOT NULL,
    y REAL NOT NULL,
    z REAL NOT NULL,
    orientation REAL NOT NULL,
    area INTEGER NOT NULL,
    first_login BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT characters_name_len CHECK (char_length(name) BETWEEN 2 AND 12)
);

CREATE UNIQUE INDEX characters_name_lower ON characters (LOWER(name));
CREATE INDEX characters_account_id ON characters (account_id);
