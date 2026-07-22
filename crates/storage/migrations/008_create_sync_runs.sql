CREATE TABLE IF NOT EXISTS sync_runs (
    id                UUID        PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(),
    trigger           TEXT        NOT NULL DEFAULT 'manual',   -- manual | scheduled
    status            TEXT        NOT NULL DEFAULT 'running',  -- running | success | error | skipped
    sources_new       INTEGER     NOT NULL DEFAULT 0,
    sources_updated   INTEGER     NOT NULL DEFAULT 0,
    todos_created     INTEGER     NOT NULL DEFAULT 0,
    events_created    INTEGER     NOT NULL DEFAULT 0,
    deadlines_created INTEGER     NOT NULL DEFAULT 0,
    error             TEXT,
    started_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at       TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_sync_runs_started ON sync_runs(started_at DESC);
