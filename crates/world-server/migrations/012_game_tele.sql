CREATE TABLE game_tele (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    map_id INTEGER NOT NULL,
    x REAL NOT NULL,
    y REAL NOT NULL,
    z REAL NOT NULL,
    orientation REAL NOT NULL DEFAULT 0
);

INSERT INTO game_tele (name, map_id, x, y, z, orientation) VALUES
    ('Stormwind', 0, -8833.38, 622.738, 93.7444, 0.7),
    ('Ironforge', 0, -4981.25, -881.542, 501.66, 0.38),
    ('Darnassus', 1, 9949.56, 2482.58, 1316.18, 4.05),
    ('Orgrimmar', 1, 1633.33, -4439.31, 15.4499, 3.61),
    ('ThunderBluff', 1, -1277.37, 124.804, 131.287, 5.22),
    ('Undercity', 0, 1584.07, 241.987, -52.1534, 3.14),
    ('Northshire', 0, -8949.95, -132.493, 83.5312, 0.0),
    ('Goldshire', 0, -9465.57, 72.773, 56.258, 4.4),
    ('Coldridge', 0, -6240.32, 331.033, 382.758, 6.17716),
    ('Deathknell', 0, 1676.71, 1678.31, 121.67, 2.70526),
    ('ValleyOfTrials', 1, -618.518, -4251.67, 38.718, 0.0),
    ('RazorHill', 1, 321.741, -4734.4, 9.49, 4.71),
    ('Crossroads', 1, -456.263, -2652.7, 95.615, 1.57),
    ('Shadowglen', 1, 10311.3, 832.463, 1326.41, 5.69632),
    ('CampNarache', 1, -2917.58, -257.98, 52.9968, 0.0),
    ('BootyBay', 0, -14297.2, 530.993, 8.779, 4.4);
