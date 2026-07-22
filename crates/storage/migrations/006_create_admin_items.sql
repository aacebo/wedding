CREATE TABLE IF NOT EXISTS todos_admin (
    id          UUID        PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(),
    source_id   UUID        REFERENCES comm_sources(id) ON DELETE SET NULL,
    title       TEXT        NOT NULL,
    owner       TEXT,
    due_date    DATE,
    priority    TEXT        NOT NULL DEFAULT 'med',
    status      TEXT        NOT NULL DEFAULT 'open',
    confidence  REAL        NOT NULL DEFAULT 0,
    notes       TEXT,
    created_by  TEXT        NOT NULL DEFAULT 'ai',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS timeline_events (
    id          UUID        PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(),
    source_id   UUID        REFERENCES comm_sources(id) ON DELETE SET NULL,
    title       TEXT        NOT NULL,
    event_date  DATE,
    category    TEXT,
    confidence  REAL        NOT NULL DEFAULT 0,
    created_by  TEXT        NOT NULL DEFAULT 'ai',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS deadlines (
    id          UUID        PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(),
    source_id   UUID        REFERENCES comm_sources(id) ON DELETE SET NULL,
    title       TEXT        NOT NULL,
    due_date    DATE,
    severity    TEXT        NOT NULL DEFAULT 'med',
    status      TEXT        NOT NULL DEFAULT 'open',
    confidence  REAL        NOT NULL DEFAULT 0,
    created_by  TEXT        NOT NULL DEFAULT 'ai',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS llm_runs (
    id                UUID        PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(),
    source_id         UUID        REFERENCES comm_sources(id) ON DELETE SET NULL,
    model             TEXT        NOT NULL,
    prompt_tokens     INTEGER     NOT NULL DEFAULT 0,
    completion_tokens INTEGER     NOT NULL DEFAULT 0,
    cost_estimate     REAL        NOT NULL DEFAULT 0,
    status            TEXT        NOT NULL,
    error             TEXT,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_todos_admin_source     ON todos_admin(source_id);
CREATE INDEX IF NOT EXISTS idx_todos_admin_status     ON todos_admin(status);
CREATE INDEX IF NOT EXISTS idx_todos_admin_due_date   ON todos_admin(due_date);
CREATE INDEX IF NOT EXISTS idx_timeline_events_source ON timeline_events(source_id);
CREATE INDEX IF NOT EXISTS idx_timeline_events_date   ON timeline_events(event_date);
CREATE INDEX IF NOT EXISTS idx_deadlines_source       ON deadlines(source_id);
CREATE INDEX IF NOT EXISTS idx_deadlines_due_date     ON deadlines(due_date);
CREATE INDEX IF NOT EXISTS idx_llm_runs_source        ON llm_runs(source_id);
