CREATE TABLE accounts (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(16) NOT NULL UNIQUE,
    email TEXT,
    password_hash TEXT NOT NULL,
    srp_salt BYTEA NOT NULL,
    srp_verifier BYTEA NOT NULL,
    locked BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT accounts_username_len CHECK (char_length(username) BETWEEN 2 AND 16),
    CONSTRAINT accounts_salt_len CHECK (octet_length(srp_salt) = 32),
    CONSTRAINT accounts_verifier_len CHECK (octet_length(srp_verifier) = 32)
);
