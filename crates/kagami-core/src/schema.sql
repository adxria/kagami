CREATE TABLE contexts (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 120),
    name_key TEXT NOT NULL UNIQUE CHECK(name_key <> 'unassigned'),
    parent_id TEXT REFERENCES contexts(id) ON DELETE CASCADE,
    color TEXT NOT NULL CHECK(length(color) = 7 AND color GLOB '#[0-9A-F][0-9A-F][0-9A-F][0-9A-F][0-9A-F][0-9A-F]'),
    tracking_locked INTEGER NOT NULL DEFAULT 0 CHECK(tracking_locked IN (0,1)),
    position INTEGER NOT NULL CHECK(position >= 0),
    CHECK(parent_id IS NULL OR parent_id <> id)
);
CREATE INDEX context_siblings ON contexts(parent_id, position);
CREATE TABLE recordings (
    id TEXT PRIMARY KEY NOT NULL,
    context_id TEXT REFERENCES contexts(id) ON DELETE CASCADE,
    started_at INTEGER NOT NULL CHECK(started_at >= 0),
    ended_at INTEGER CHECK(ended_at IS NULL OR ended_at >= started_at)
);
CREATE UNIQUE INDEX one_running_recording ON recordings((1)) WHERE ended_at IS NULL;
CREATE INDEX recording_time ON recordings(started_at, ended_at);
CREATE INDEX recording_context ON recordings(context_id);
PRAGMA user_version = 1;
