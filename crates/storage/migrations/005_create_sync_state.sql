CREATE TABLE IF NOT EXISTS sync_state (
    account_email  TEXT        NOT NULL,
    provider       TEXT        NOT NULL,
    cursor         TEXT,
    last_synced_at TIMESTAMPTZ,
    PRIMARY KEY (account_email, provider)
);
