ALTER TABLE guests
    ADD COLUMN first_name TEXT,
    ADD COLUMN last_name TEXT,
    ADD COLUMN accommodation_name TEXT,
    ADD COLUMN accommodation_address TEXT,
    ADD COLUMN phone TEXT,
    ADD COLUMN dietary_requirements TEXT;

CREATE TABLE rsvp_sheets_outbox (
    rsvp_id          UUID        PRIMARY KEY NOT NULL REFERENCES rsvps(id) ON DELETE CASCADE,
    payload          JSONB       NOT NULL,
    attempts         INTEGER     NOT NULL DEFAULT 0,
    next_attempt_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_error       TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delivered_at     TIMESTAMPTZ
);

CREATE INDEX idx_rsvp_sheets_outbox_pending
    ON rsvp_sheets_outbox (next_attempt_at, created_at)
    WHERE delivered_at IS NULL;

-- Queue a lossless snapshot of every existing RSVP. Legacy group responses remain
-- one row with their original party_size and name; no guest or RSVP rows are changed.
INSERT INTO rsvp_sheets_outbox (rsvp_id, payload)
SELECT
    r.id,
    jsonb_build_object(
        'rsvp_id', r.id,
        'submitted_at', r.created_at,
        'first_name', g.first_name,
        'last_name', g.last_name,
        'guest_name', g.name,
        'email', g.email,
        'attendance', CASE WHEN r.attending THEN 'Attending' ELSE 'Declining' END,
        'accommodation_name', g.accommodation_name,
        'accommodation_address', g.accommodation_address,
        'phone', g.phone,
        'dietary_requirements', g.dietary_requirements,
        'main_course_choice', r.meal_preference,
        'legacy_party_size', r.party_size
    )
FROM rsvps r
JOIN guests g ON g.id = r.guest_id
ON CONFLICT (rsvp_id) DO NOTHING;
