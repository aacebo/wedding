CREATE TABLE IF NOT EXISTS google_accounts (
    id            UUID        PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(),
    email         TEXT        NOT NULL UNIQUE,
    google_sub    TEXT        UNIQUE,
    access_token  BYTEA       NOT NULL,
    refresh_token BYTEA,
    token_expiry  TIMESTAMPTZ,
    scopes        TEXT        NOT NULL DEFAULT '',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_google_accounts_email ON google_accounts(email);
