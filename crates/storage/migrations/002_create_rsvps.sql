CREATE TABLE IF NOT EXISTS rsvps (
    id               UUID        PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(),
    guest_id         UUID        NOT NULL REFERENCES guests(id) ON DELETE CASCADE,
    attending        BOOLEAN     NOT NULL DEFAULT FALSE,
    meal_preference  TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_rsvps_guest_id   ON rsvps(guest_id);
CREATE INDEX IF NOT EXISTS idx_rsvps_created_at ON rsvps(created_at);
