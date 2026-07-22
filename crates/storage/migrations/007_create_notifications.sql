CREATE TABLE IF NOT EXISTS notifications (
    id          UUID        PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(),
    kind        TEXT        NOT NULL,
    ref_type    TEXT        NOT NULL,
    ref_id      UUID        NOT NULL,
    title       TEXT        NOT NULL,
    body        TEXT        NOT NULL DEFAULT '',
    due_date    DATE,
    status      TEXT        NOT NULL DEFAULT 'unread',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- One notification per referenced item; its `kind` escalates in place
-- (deadline_soon -> deadline_overdue) so re-generation never duplicates.
CREATE UNIQUE INDEX IF NOT EXISTS uq_notifications_ref
    ON notifications(ref_type, ref_id);

CREATE INDEX IF NOT EXISTS idx_notifications_status ON notifications(status);
CREATE INDEX IF NOT EXISTS idx_notifications_due    ON notifications(due_date);
