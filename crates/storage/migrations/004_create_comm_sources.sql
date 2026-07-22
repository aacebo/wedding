CREATE TABLE IF NOT EXISTS comm_sources (
    id            UUID        PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(),
    account_email TEXT        NOT NULL,
    provider      TEXT        NOT NULL,
    external_id   TEXT        NOT NULL,
    kind          TEXT        NOT NULL,
    sender        TEXT,
    title         TEXT        NOT NULL DEFAULT '',
    snippet       TEXT        NOT NULL DEFAULT '',
    body_text     TEXT        NOT NULL DEFAULT '',
    url           TEXT,
    occurred_at   TIMESTAMPTZ,
    raw           JSONB       NOT NULL DEFAULT '{}'::jsonb,
    processed_at  TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (provider, external_id)
);

CREATE INDEX IF NOT EXISTS idx_comm_sources_processed_at ON comm_sources(processed_at);
CREATE INDEX IF NOT EXISTS idx_comm_sources_occurred_at  ON comm_sources(occurred_at);
CREATE INDEX IF NOT EXISTS idx_comm_sources_account      ON comm_sources(account_email);
