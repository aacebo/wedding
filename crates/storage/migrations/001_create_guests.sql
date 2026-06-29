CREATE TABLE IF NOT EXISTS guests (
    id          UUID        PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(),
    name        TEXT        NOT NULL,
    email       TEXT        NOT NULL UNIQUE,
    plus_one    BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_guests_email      ON guests(email);
CREATE INDEX IF NOT EXISTS idx_guests_created_at ON guests(created_at);
CREATE INDEX IF NOT EXISTS idx_guests_updated_at ON guests(updated_at);
