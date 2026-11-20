use kagami_core::{Change, DataImports, ImportData, ImportMode, Query, Store, Workspace};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::{fs, path::Path};

const NOW: i64 = 10_000;
const QUERY: Query = Query {
    week_start: 0,
    week_end: 20_000,
    today_start: 0,
    today_end: 20_000,
};

fn load(value: Value) -> ImportData {
    ImportData::read(value.to_string().as_bytes()).unwrap()
}

fn read(store: &mut Store) -> Workspace {
    store.read_workspace(QUERY, NOW).unwrap()
}

fn branch(store: &mut Store, name: &str, parent_id: Option<&str>) -> String {
    store
        .apply_change(
            Change::CreateContext {
                name: name.into(),
                parent_id: parent_id.map(str::to_owned),
                color: "#EFD76C".into(),
            },
            NOW,
        )
        .unwrap()
        .id
}

#[test]
fn add_matches_full_paths_and_copies_all_recordings_once_per_confirmation() {
    let mut store = Store::memory().unwrap();
    let japanese = branch(&mut store, "Japanese", None);
    let reading = branch(&mut store, "Reading", Some(&japanese));
    let operations = branch(&mut store, "Operations", None);
    let admin = branch(&mut store, "Admin", Some(&operations));
    let existing = store
        .apply_change(
            Change::CreateRecording {
                context_id: Some(reading.clone()),
                started_at: 1000,
                ended_at: 3000,
            },
            NOW,
        )
        .unwrap()
        .id;
    let running = store
        .apply_change(Change::StartTimer { context_id: None }, NOW)
        .unwrap()
        .id;
    let file = json!({
        "format": "kagami-data", "version": 1,
        "recordings": [
            {"contextPath": ["japanese", "READING"], "startedAt": 1000, "endedAt": 3000},
            {"contextPath": ["Client A", "Admin"], "startedAt": 2000, "endedAt": 4000}
        ]
    });
    let mut imports = DataImports::default();
    let preview =
        serde_json::to_value(imports.stage("data.json".into(), load(file.clone()))).unwrap();
    let token = preview["token"].as_str().unwrap();
    assert_eq!(preview["contexts"], 4);
    assert_eq!(preview["recordings"], 2);
    assert!(!imports
        .apply(token, &mut store, ImportMode::Add, Path::new(""), NOW)
        .unwrap());
    assert!(imports
        .apply(token, &mut store, ImportMode::Add, Path::new(""), NOW)
        .is_err());

    let snapshot = read(&mut store);
    assert_eq!(snapshot.running.unwrap().id, running);
    let existing_reading = snapshot
        .contexts
        .iter()
        .find(|context| context.id == reading)
        .unwrap();
    assert_eq!(existing_reading.color, "#EFD76C");
    assert_eq!(snapshot.contexts.len(), 6);
    assert_eq!(
        snapshot
            .recordings
            .iter()
            .filter(|entry| entry.context_id.as_deref() == Some(&reading))
            .count(),
        2
    );
    assert!(snapshot.overlap_ids.contains(&existing));
    assert_eq!(snapshot.totals.week_ms, 6000);
    let client = snapshot
        .contexts
        .iter()
        .find(|context| context.name == "Client A")
        .unwrap();
    let imported_admin = snapshot
        .contexts
        .iter()
        .find(|context| context.parent_id.as_deref() == Some(&client.id))
        .unwrap();
    assert_eq!(imported_admin.name, "Admin");
    assert_eq!(
        snapshot
            .recordings
            .iter()
            .filter(|entry| entry.context_id.as_deref() == Some(&imported_admin.id))
            .count(),
        1
    );
    assert!(!snapshot
        .recordings
        .iter()
        .any(|entry| entry.context_id.as_deref() == Some(&admin)));

    store
        .import_data(&load(file), ImportMode::Add, Path::new(""), NOW)
        .unwrap();
    let snapshot = read(&mut store);
    assert_eq!(snapshot.contexts.len(), 6);
    assert_eq!(snapshot.totals.week_ms, 10000);
}

#[test]
fn export_round_trips_and_replace_saves_recoverable_data_first() {
    let directory = tempfile::tempdir().unwrap();
    let export = directory.path().join("kagami-data.json");
    let recovery = directory.path().join("copies/recovery");
    let mut source = Store::memory().unwrap();
    let japanese = branch(&mut source, "Japanese", None);
    let reading = branch(&mut source, "Reading", Some(&japanese));
    let speaking = branch(&mut source, "Speaking", Some(&japanese));
    source
        .apply_change(
            Change::LockContext {
                id: speaking,
                locked: true,
            },
            NOW,
        )
        .unwrap();
    source
        .apply_change(
            Change::CreateRecording {
                context_id: None,
                started_at: 1123,
                ended_at: 3456,
            },
            NOW,
        )
        .unwrap();
    let running = source
        .apply_change(
            Change::StartTimer {
                context_id: Some(reading),
            },
            NOW - 1000,
        )
        .unwrap()
        .id;
    source.export_data(&export, NOW).unwrap();
    assert_eq!(source.running().unwrap().unwrap().id, running);

    let mut target = Store::memory().unwrap();
    let previous = branch(&mut target, "Previous", None);
    let data = ImportData::read(fs::File::open(export).unwrap()).unwrap();
    assert!(target
        .import_data(&data, ImportMode::Replace, &recovery, NOW)
        .unwrap());
    let snapshot = read(&mut target);
    assert!(snapshot.running.is_none());
    assert_eq!(snapshot.contexts.len(), 3);
    assert!(snapshot
        .contexts
        .iter()
        .all(|context| context.id != previous));
    let children: Vec<_> = snapshot
        .contexts
        .iter()
        .filter(|context| context.parent_id.is_some())
        .map(|context| (&*context.name, context.position, context.tracking_locked))
        .collect();
    assert_eq!(children, [("Speaking", 0, true), ("Reading", 1, false)]);
    assert_eq!(snapshot.recordings[0].context_id, None);
    assert_eq!(
        (
            snapshot.recordings[0].started_at,
            snapshot.recordings[0].ended_at
        ),
        (1123, Some(3456))
    );
    assert_eq!(
        (
            snapshot.recordings[1].started_at,
            snapshot.recordings[1].ended_at
        ),
        (9000, Some(NOW))
    );
    let copy = fs::read_dir(&recovery)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let old_data = ImportData::read(fs::File::open(copy).unwrap()).unwrap();
    target
        .import_data(&old_data, ImportMode::Replace, &recovery, NOW)
        .unwrap();
    assert_eq!(read(&mut target).contexts[0].name, "Previous");
}

#[test]
fn an_unassigned_recording_requires_an_explicit_null_context_path() {
    let missing =
        br#"{"format":"kagami-data","version":1,"recordings":[{"startedAt":1000,"endedAt":2000}]}"#;
    let error = ImportData::read(missing.as_slice()).err().unwrap();
    assert!(error.message.contains("missing field `contextPath`"));
    let explicit = br#"{"format":"kagami-data","version":1,"recordings":[{"contextPath":null,"startedAt":1000,"endedAt":2000}]}"#;
    assert_eq!(
        ImportData::read(explicit.as_slice()).unwrap().counts(),
        (0, 1)
    );
}

#[test]
fn replacing_and_deleting_a_deep_context_tree_succeeds() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite3");
    let recovery = directory.path().join("recovery");
    let mut store = Store::open(&database).unwrap();
    let deep = load(json!({
        "format": "kagami-data", "version": 1,
        "recordings": [
            {"contextPath": vec!["Nested"; 1002], "startedAt": 1000, "endedAt": 2000}
        ]
    }));
    let empty = load(json!({"format": "kagami-data", "version": 1, "recordings": []}));
    store
        .import_data(&deep, ImportMode::Add, &recovery, NOW)
        .unwrap();
    assert!(store
        .import_data(&empty, ImportMode::Replace, &recovery, NOW)
        .unwrap());
    assert!(read(&mut store).contexts.is_empty());

    let copy = fs::read_dir(&recovery)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let copy = ImportData::read(fs::File::open(copy).unwrap()).unwrap();
    assert_eq!(copy.counts(), (1002, 1));
    store
        .import_data(&copy, ImportMode::Add, &recovery, NOW)
        .unwrap();
    let id = Connection::open(&database)
        .unwrap()
        .query_row(
            "SELECT id FROM contexts WHERE parent_id IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    store
        .apply_change(Change::DeleteContext { id }, NOW)
        .unwrap();
    let snapshot = read(&mut store);
    assert!(snapshot.contexts.is_empty());
    assert!(snapshot.recordings.is_empty());
}

#[test]
fn invalid_data_and_failed_replacement_leave_the_current_workspace_intact() {
    let invalid = br#"{"format":"kagami-data","version":1,"recordings":[{"contextPath":null,"startedAt":20,"endedAt":10}]}"#;
    assert!(ImportData::read(invalid.as_slice()).is_err());
    let unsupported = br#"{"format":"kagami-data","version":91,"recordings":false}"#;
    assert!(ImportData::read(unsupported.as_slice())
        .err()
        .unwrap()
        .message
        .contains("version 91"));

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.sqlite3");
    let mut store = Store::open(&path).unwrap();
    let keep = branch(&mut store, "Keep", None);
    let data = load(
        json!({"format": "kagami-data", "version": 1, "recordings": [
            {"contextPath": ["Replace"], "startedAt": 1000, "endedAt": 3000}
        ]}),
    );
    let running = store
        .apply_change(Change::StartTimer { context_id: None }, NOW)
        .unwrap()
        .id;
    let recovery = directory.path().join("recovery");
    assert!(store
        .import_data(&data, ImportMode::Replace, &recovery, NOW)
        .is_err());
    store
        .apply_change(Change::StopTimer { id: running }, NOW)
        .unwrap();
    assert!(store
        .import_data(&data, ImportMode::Replace, &path, NOW)
        .is_err());

    let original = read(&mut store);
    let db = Connection::open(path).unwrap();
    db.execute_batch(
        "CREATE TRIGGER reject_import BEFORE INSERT ON recordings
        BEGIN SELECT RAISE(ABORT,'write failed'); END;",
    )
    .unwrap();
    assert!(store
        .import_data(&data, ImportMode::Replace, &recovery, NOW)
        .is_err());
    let result = read(&mut store);
    assert_eq!(result.contexts.len(), 1);
    assert_eq!(result.contexts[0].id, keep);
    assert_eq!(result.recordings.len(), 1);
    assert_eq!(result.recordings[0].id, original.recordings[0].id);
}
