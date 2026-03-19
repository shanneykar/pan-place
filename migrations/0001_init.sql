CREATE TABLE IF NOT EXISTS actors (
    actor_id    TEXT PRIMARY KEY,
    pubkey      BLOB NOT NULL,
    phone_dhash TEXT NOT NULL UNIQUE,
    created_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS pan_nodes (
    pan_id        TEXT PRIMARY KEY,
    lat           REAL NOT NULL,
    lon           REAL NOT NULL,
    radius_miles  REAL NOT NULL DEFAULT 1.0,
    placed_at     INTEGER NOT NULL,
    node_type     TEXT NOT NULL DEFAULT 'fixed',
    status        TEXT NOT NULL DEFAULT 'active'
);

CREATE TABLE IF NOT EXISTS events (
    event_id          TEXT PRIMARY KEY,
    entity_id         TEXT NOT NULL,
    event_type        TEXT NOT NULL,
    timestamp         INTEGER NOT NULL,
    content           TEXT NOT NULL DEFAULT '',
    tags              TEXT NOT NULL DEFAULT '[]',
    references_event  TEXT,
    signature         TEXT NOT NULL,
    actor_id          TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS event_parents (
    event_id     TEXT NOT NULL REFERENCES events(event_id),
    parent_hash  TEXT NOT NULL,
    PRIMARY KEY (event_id, parent_hash)
);

CREATE INDEX IF NOT EXISTS idx_events_entity_id   ON events(entity_id);
CREATE INDEX IF NOT EXISTS idx_events_actor_id    ON events(actor_id);
CREATE INDEX IF NOT EXISTS idx_events_timestamp   ON events(timestamp);
CREATE INDEX IF NOT EXISTS idx_events_event_type  ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_references  ON events(references_event);
CREATE INDEX IF NOT EXISTS idx_event_parents_parent ON event_parents(parent_hash);
