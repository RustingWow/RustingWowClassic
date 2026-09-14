ALTER TABLE accounts
    ADD COLUMN gmlevel SMALLINT NOT NULL DEFAULT 0,
    ADD CONSTRAINT accounts_gmlevel_range CHECK (gmlevel BETWEEN 0 AND 3);
