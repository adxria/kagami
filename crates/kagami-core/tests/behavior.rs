use kagami_core::{Change, Query, Store};
use rusqlite::Connection;

const NOW: i64 = 10_000;
const QUERY: Query = Query {
    week_start: 0,
    week_end: 20_000,
    today_start: 5_000,
    today_end: 15_000,
};
fn apply(store: &mut Store, change: Change) -> String {
    store.apply_change(change, NOW).unwrap().id
}
fn context(store: &mut Store, name: &str, parent: Option<&str>) -> String {
    apply(
        store,
        Change::CreateContext {
            name: name.into(),
            parent_id: parent.map(str::to_owned),
            color: "abc".into(),
        },
    )
}
fn recording(store: &mut Store, context: Option<&str>, start: i64, end: i64) -> String {
    apply(
        store,
        Change::CreateRecording {
            context_id: context.map(str::to_owned),
            started_at: start,
            ended_at: end,
        },
    )
}
#[test]
fn timer_is_exclusive_persistent_and_stale_actions_cannot_resurrect_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("timer.db");
    let mut store = Store::open(&path).unwrap();
    let id = apply(&mut store, Change::StartTimer { context_id: None });
    assert!(store
        .apply_change(Change::StartTimer { context_id: None }, NOW)
        .is_err());
    drop(store);
    let mut store = Store::open(&path).unwrap();
    assert_eq!(store.running().unwrap().unwrap().id, id);
    let a = context(&mut store, "Work", None);
    apply(
        &mut store,
        Change::AssignTimer {
            id: id.clone(),
            context_id: Some(a.clone()),
        },
    );
    assert_eq!(store.running().unwrap().unwrap().started_at, NOW);
    store
        .apply_change(Change::StopTimer { id: id.clone() }, NOW + 3)
        .unwrap();
    assert!(store
        .apply_change(
            Change::AssignTimer {
                id: id.clone(),
                context_id: None
            },
            NOW + 4
        )
        .is_err());
    assert!(store
        .apply_change(
            Change::EditRecording {
                id: id.clone(),
                context_id: None,
                started_at: NOW,
                ended_at: None,
                expected_running: true
            },
            NOW + 4
        )
        .is_err());
    let data = store.read_workspace(QUERY, NOW + 4).unwrap();
    assert_eq!(data.recordings[0].ended_at, Some(NOW + 3));
    assert_eq!(data.recordings[0].context_id, Some(a));
    // The database itself enforces exclusivity, even outside our API.
    let db = Connection::open(&path).unwrap();
    db.execute("INSERT INTO recordings VALUES('direct1',NULL,1,NULL)", [])
        .unwrap();
    assert!(db
        .execute("INSERT INTO recordings VALUES('direct2',NULL,1,NULL)", [])
        .is_err());
}
#[test]
fn immediate_stop_is_retained_and_clock_reversal_requires_correction() {
    let mut store = Store::memory().unwrap();
    let id = apply(&mut store, Change::StartTimer { context_id: None });
    let boundary = Query {
        week_start: NOW,
        week_end: NOW + 7 * 86_400_000,
        ..QUERY
    };
    assert_eq!(
        store
            .read_workspace(boundary, NOW)
            .unwrap()
            .recordings
            .len(),
        1
    );
    assert!(store
        .apply_change(Change::StopTimer { id: id.clone() }, NOW - 1)
        .is_err());
    apply(&mut store, Change::StopTimer { id: id.clone() });
    let data = store.read_workspace(QUERY, NOW).unwrap();
    assert_eq!(data.recordings[0].ended_at, Some(NOW));
    assert_eq!(data.totals.lifetime_ms, 0);
    apply(
        &mut store,
        Change::EditRecording {
            id,
            context_id: None,
            started_at: 9000,
            ended_at: Some(NOW),
            expected_running: false,
        },
    );
}
#[test]
fn locks_are_direct_and_completed_corrections_remain_possible() {
    let mut store = Store::memory().unwrap();
    let parent = context(&mut store, "Parent", None);
    let child = context(&mut store, "Child", Some(&parent));
    let timer = apply(
        &mut store,
        Change::StartTimer {
            context_id: Some(parent.clone()),
        },
    );
    assert!(store
        .apply_change(
            Change::LockContext {
                id: parent.clone(),
                locked: true
            },
            NOW
        )
        .is_err());
    apply(&mut store, Change::StopTimer { id: timer });
    apply(
        &mut store,
        Change::LockContext {
            id: parent.clone(),
            locked: true,
        },
    );
    assert!(store
        .apply_change(
            Change::StartTimer {
                context_id: Some(parent.clone())
            },
            NOW
        )
        .is_err());
    assert!(store
        .apply_change(
            Change::CreateRecording {
                context_id: Some(parent.clone()),
                started_at: 1,
                ended_at: 2
            },
            NOW
        )
        .is_err());
    let timer = apply(
        &mut store,
        Change::StartTimer {
            context_id: Some(child.clone()),
        },
    );
    assert!(store
        .apply_change(
            Change::AssignTimer {
                id: timer.clone(),
                context_id: Some(parent.clone())
            },
            NOW
        )
        .is_err());
    apply(&mut store, Change::StopTimer { id: timer });
    let id = recording(&mut store, Some(&child), 10, 20);
    apply(
        &mut store,
        Change::EditRecording {
            id: id.clone(),
            context_id: Some(parent),
            started_at: 10,
            ended_at: Some(20),
            expected_running: false,
        },
    );
    apply(
        &mut store,
        Change::EditRecording {
            id,
            context_id: None,
            started_at: 10,
            ended_at: Some(20),
            expected_running: false,
        },
    );
}
#[test]
fn targeted_context_changes_preserve_independent_properties_and_sibling_names() {
    let mut store = Store::memory().unwrap();
    let parent = context(&mut store, " Parent ", None);
    let child = context(&mut store, "Child", Some(&parent));
    let other = context(&mut store, "Other", None);
    let same_name = context(&mut store, " child ", Some(&other));
    assert!(store
        .apply_change(
            Change::CreateContext {
                name: "CHILD".into(),
                parent_id: Some(parent.clone()),
                color: "fff".into()
            },
            NOW
        )
        .is_err());
    assert!(store
        .apply_change(
            Change::RenameContext {
                id: other,
                name: "PARENT".into(),
            },
            NOW
        )
        .is_err());
    let error = store
        .apply_change(
            Change::MoveContext {
                id: same_name.clone(),
                parent_id: Some(parent.clone()),
                position: 0,
            },
            NOW,
        )
        .unwrap_err();
    assert_eq!(error.message, "Parent already has a context named child.");
    assert!(store
        .apply_change(
            Change::RenameContext {
                id: child.clone(),
                name: " Unassigned ".into()
            },
            NOW
        )
        .is_err());
    apply(
        &mut store,
        Change::LockContext {
            id: parent.clone(),
            locked: true,
        },
    );
    apply(
        &mut store,
        Change::ColorContext {
            id: parent.clone(),
            color: " #123 ".into(),
        },
    );
    apply(
        &mut store,
        Change::RenameContext {
            id: parent.clone(),
            name: "Updated".into(),
        },
    );
    let data = store.read_workspace(QUERY, NOW).unwrap();
    let p = data.contexts.iter().find(|c| c.id == parent).unwrap();
    let c = data.contexts.iter().find(|c| c.id == child).unwrap();
    assert_eq!(
        (&*p.name, &*p.color, p.tracking_locked),
        ("Updated", "#112233", true)
    );
    assert_eq!(c.color, "#AABBCC");
    assert_eq!(c.parent_id, Some(parent));
    assert_ne!(
        data.contexts
            .iter()
            .find(|c| c.id == same_name)
            .unwrap()
            .parent_id,
        c.parent_id
    );
}

#[test]
fn upgrading_v1_preserves_tree_timer_and_recording_relationships() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("v1.sqlite3");
    let db = Connection::open(&path).unwrap();
    db.execute_batch(include_str!("../src/schema.sql")).unwrap();
    db.execute_batch(
        "
        INSERT INTO contexts VALUES ('a','Japanese','japanese',NULL,'#7FBCF5',0,1);
        INSERT INTO contexts VALUES ('b','English','english',NULL,'#7FBCF5',0,0);
        INSERT INTO contexts VALUES ('c','Reading','reading','a','#7DCF9F',1,0);
        INSERT INTO recordings VALUES ('entry','c',1000,2000);
        INSERT INTO recordings VALUES ('running','b',3000,NULL);
    ",
    )
    .unwrap();
    drop(db);

    let mut store = Store::open(&path).unwrap();
    context(&mut store, "Reading", Some("b"));
    let workspace = store.read_workspace(QUERY, NOW).unwrap();
    assert_eq!(workspace.running.unwrap().id, "running");
    let original = workspace
        .contexts
        .iter()
        .find(|context| context.id == "c")
        .unwrap();
    assert_eq!(original.parent_id.as_deref(), Some("a"));
    assert_eq!(
        (&*original.color, original.tracking_locked),
        ("#7DCF9F", true)
    );
    let roots: Vec<_> = workspace
        .contexts
        .iter()
        .filter(|context| context.parent_id.is_none())
        .map(|context| context.id.as_str())
        .collect();
    assert_eq!(roots, ["b", "a"]);
    let entry = workspace
        .recordings
        .iter()
        .find(|entry| entry.id == "entry")
        .unwrap();
    assert_eq!(entry.context_id.as_deref(), Some("c"));
    apply(&mut store, Change::DeleteContext { id: "a".into() });
    assert_eq!(
        store.read_workspace(QUERY, NOW).unwrap().recordings.len(),
        1
    );
}
#[test]
fn hierarchy_and_sibling_positions_commit_together_and_reject_cycles() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("order.db");
    let mut store = Store::open(&path).unwrap();
    let a = context(&mut store, "A", None);
    let b = context(&mut store, "B", None);
    let c = context(&mut store, "C", None);
    apply(
        &mut store,
        Change::MoveContext {
            id: c.clone(),
            parent_id: None,
            position: 2,
        },
    );
    apply(
        &mut store,
        Change::MoveContext {
            id: b.clone(),
            parent_id: Some(a.clone()),
            position: 0,
        },
    );
    assert!(store
        .apply_change(
            Change::MoveContext {
                id: a.clone(),
                parent_id: Some(b.clone()),
                position: 0
            },
            NOW
        )
        .is_err());
    assert!(store
        .apply_change(
            Change::MoveContext {
                id: a.clone(),
                parent_id: None,
                position: 99
            },
            NOW
        )
        .is_err());
    drop(store);
    let mut store = Store::open(path).unwrap();
    let data = store.read_workspace(QUERY, NOW).unwrap();
    let roots: Vec<_> = data
        .contexts
        .iter()
        .filter(|c| c.parent_id.is_none())
        .map(|c| (&c.id, c.position))
        .collect();
    assert_eq!(roots, vec![(&a, 0), (&c, 1)]);
    assert_eq!(
        data.contexts.iter().find(|c| c.id == b).unwrap().parent_id,
        Some(a)
    );
}
#[test]
fn failed_compound_write_rolls_back_order_and_subtree_deletion() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rollback.db");
    let mut store = Store::open(&path).unwrap();
    let parent = context(&mut store, "Parent", None);
    let child = context(&mut store, "Child", Some(&parent));
    let other = context(&mut store, "Other", None);
    let timer = apply(
        &mut store,
        Change::StartTimer {
            context_id: Some(child.clone()),
        },
    );
    let db = Connection::open(&path).unwrap();
    db.execute_batch("CREATE TRIGGER reject_context_delete BEFORE DELETE ON recordings BEGIN SELECT RAISE(ABORT,'disk write simulation'); END;").unwrap();
    assert!(store
        .apply_change(Change::DeleteContext { id: parent.clone() }, NOW)
        .is_err());
    assert_eq!(store.running().unwrap().unwrap().id, timer);
    assert_eq!(store.read_workspace(QUERY, NOW).unwrap().contexts.len(), 3);
    db.execute_batch("DROP TRIGGER reject_context_delete; CREATE TRIGGER reject_order BEFORE UPDATE OF position ON contexts WHEN NEW.position=1 BEGIN SELECT RAISE(ABORT,'disk write simulation'); END;").unwrap();
    assert!(store
        .apply_change(
            Change::MoveContext {
                id: other.clone(),
                parent_id: Some(parent.clone()),
                position: 0
            },
            NOW
        )
        .is_err());
    let data = store.read_workspace(QUERY, NOW).unwrap();
    assert!(data
        .contexts
        .iter()
        .find(|c| c.id == other)
        .unwrap()
        .parent_id
        .is_none());
    assert_eq!(
        data.contexts
            .iter()
            .find(|c| c.id == child)
            .unwrap()
            .position,
        0
    );
    db.execute_batch("DROP TRIGGER reject_order;").unwrap();
    apply(&mut store, Change::DeleteContext { id: parent });
    let data = store.read_workspace(QUERY, NOW).unwrap();
    assert_eq!(data.contexts.len(), 1);
    assert!(data.running.is_none());
    assert!(data.recordings.is_empty());
}
#[test]
fn intervals_are_validated_before_any_write() {
    let mut store = Store::memory().unwrap();
    for (start, end) in [(-1, 2), (2, 2), (3, 2), (1, NOW + 1), (NOW + 1, NOW + 2)] {
        assert!(store
            .apply_change(
                Change::CreateRecording {
                    context_id: None,
                    started_at: start,
                    ended_at: end
                },
                NOW
            )
            .is_err());
    }
    assert!(store
        .apply_change(
            Change::CreateRecording {
                context_id: Some("missing".into()),
                started_at: 1,
                ended_at: 2
            },
            NOW
        )
        .is_err());
    assert!(store
        .read_workspace(QUERY, NOW)
        .unwrap()
        .recordings
        .is_empty());
}
#[test]
fn reporting_clips_half_open_ranges_sums_overlaps_and_rolls_up_current_ancestry() {
    let mut store = Store::memory().unwrap();
    let p = context(&mut store, "Parent", None);
    let c = context(&mut store, "Child", Some(&p));
    let first = recording(&mut store, Some(&c), 1000, 6000);
    let overlap = recording(&mut store, Some(&p), 4000, 7000);
    let adjacent = recording(&mut store, None, 7000, 8000);
    let q = Query {
        week_start: 2000,
        week_end: 8000,
        ..QUERY
    };
    let data = store.read_workspace(q, NOW).unwrap();
    assert_eq!(data.totals.week_ms, 8000);
    assert_eq!(data.totals.today_ms, 4000);
    assert_eq!(data.totals.lifetime_ms, 9000);
    assert_eq!(
        data.context_totals
            .iter()
            .find(|t| t.context_id == p)
            .unwrap()
            .total
            .week_ms,
        7000
    );
    assert_eq!(data.unassigned.week_ms, 1000);
    assert!(data.overlap_ids.contains(&first) && data.overlap_ids.contains(&overlap));
    assert!(!data.overlap_ids.contains(&adjacent));
    apply(
        &mut store,
        Change::MoveContext {
            id: c,
            parent_id: None,
            position: 0,
        },
    );
    let data = store.read_workspace(q, NOW).unwrap();
    assert_eq!(
        data.context_totals
            .iter()
            .find(|t| t.context_id == p)
            .unwrap()
            .total
            .week_ms,
        3000
    );
    assert_eq!(data.totals.week_ms, 8000);
}
#[test]
fn running_outside_the_selected_week_is_global_but_not_counted_in_that_week() {
    let mut store = Store::memory().unwrap();
    let id = apply(&mut store, Change::StartTimer { context_id: None });
    let q = Query {
        week_start: 1000,
        week_end: 5000,
        ..QUERY
    };
    let data = store.read_workspace(q, NOW + 1000).unwrap();
    assert_eq!(data.running.unwrap().id, id);
    assert!(data.recordings.is_empty());
    assert_eq!(data.totals.week_ms, 0);
    assert_eq!(data.totals.lifetime_ms, 1000);
}
#[test]
fn future_database_version_is_rejected_without_modification() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("future.db");
    let db = Connection::open(&path).unwrap();
    db.execute_batch("PRAGMA user_version=99; CREATE TABLE future(value TEXT);")
        .unwrap();
    drop(db);
    let before = std::fs::read(&path).unwrap();
    assert!(Store::open(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
}
