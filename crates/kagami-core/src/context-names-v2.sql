CREATE TABLE contexts_new (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 120),
    name_key TEXT NOT NULL CHECK(name_key <> 'unassigned'),
    parent_id TEXT REFERENCES contexts_new(id) ON DELETE CASCADE,
    color TEXT NOT NULL CHECK(length(color) = 7 AND color GLOB '#[0-9A-F][0-9A-F][0-9A-F][0-9A-F][0-9A-F][0-9A-F]'),
    tracking_locked INTEGER NOT NULL DEFAULT 0 CHECK(tracking_locked IN (0,1)),
    position INTEGER NOT NULL CHECK(position >= 0),
    CHECK(parent_id IS NULL OR parent_id <> id)
);
INSERT INTO contexts_new SELECT id, name, name_key, parent_id, color, tracking_locked, position FROM contexts;
DROP TABLE contexts;
ALTER TABLE contexts_new RENAME TO contexts;
CREATE INDEX context_siblings ON contexts(parent_id, position);
CREATE UNIQUE INDEX context_root_name ON contexts(name_key) WHERE parent_id IS NULL;
CREATE UNIQUE INDEX context_child_name ON contexts(parent_id, name_key) WHERE parent_id IS NOT NULL;
PRAGMA user_version = 2;
