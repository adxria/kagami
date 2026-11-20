use crate::{contexts, Error, Recording, Result};
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

pub(crate) const MAX_TIMESTAMP: i64 = 253_402_300_799_999;

pub(crate) fn valid_now(now: i64) -> Result<()> {
    if !(0..=MAX_TIMESTAMP).contains(&now) {
        return Err(Error::invalid(
            "The system clock is outside the supported calendar.",
        ));
    }
    Ok(())
}
fn interval(start: i64, end: Option<i64>, now: i64) -> Result<()> {
    if start < 0 || start > now || end.is_some_and(|end| end <= start || end > now) {
        return Err(Error::invalid(
            "Start and End must be in the past or present, and End must be later than Start.",
        ));
    }
    Ok(())
}
pub(crate) fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Recording> {
    Ok(Recording {
        id: r.get(0)?,
        context_id: r.get(1)?,
        started_at: r.get(2)?,
        ended_at: r.get(3)?,
    })
}
pub(crate) fn running(db: &Connection) -> Result<Option<Recording>> {
    Ok(db
        .query_row(
            "SELECT id,context_id,started_at,ended_at FROM recordings WHERE ended_at IS NULL",
            [],
            row,
        )
        .optional()?)
}
fn get(db: &Connection, id: &str, expected_running: bool) -> Result<Recording> {
    let value = db
        .query_row(
            "SELECT id,context_id,started_at,ended_at FROM recordings WHERE id=?1",
            [id],
            row,
        )
        .optional()?
        .ok_or_else(Error::stale)?;
    if value.ended_at.is_none() != expected_running {
        return Err(Error::stale());
    }
    Ok(value)
}
pub(crate) fn start(db: &Connection, context: Option<&str>, now: i64) -> Result<String> {
    contexts::check_target(db, context, true)?;
    if running(db)?.is_some() {
        return Err(Error::invalid(
            "A timer is already running. Stop it before starting another.",
        ));
    }
    let id = Uuid::new_v4().to_string();
    db.execute(
        "INSERT INTO recordings(id,context_id,started_at) VALUES(?1,?2,?3)",
        params![id, context, now],
    )?;
    Ok(id)
}
pub(crate) fn stop(db: &Connection, id: &str, now: i64) -> Result<Recording> {
    let mut value = get(db, id, true)?;
    if now < value.started_at {
        return Err(Error::invalid(
            "The clock moved before the timer's Start. Correct Start before stopping.",
        ));
    }
    // A timer stopped in its starting millisecond is retained as a zero-length
    // recording, rather than inventing future time or silently discarding it.
    db.execute(
        "UPDATE recordings SET ended_at=?1 WHERE id=?2",
        params![now, id],
    )?;
    value.ended_at = Some(now);
    Ok(value)
}
pub(crate) fn assign(db: &Connection, id: &str, context: Option<&str>) -> Result<()> {
    get(db, id, true)?;
    contexts::check_target(db, context, true)?;
    db.execute(
        "UPDATE recordings SET context_id=?1 WHERE id=?2",
        params![context, id],
    )?;
    Ok(())
}
pub(crate) fn create(
    db: &Connection,
    context: Option<&str>,
    start: i64,
    end: i64,
    now: i64,
) -> Result<String> {
    interval(start, Some(end), now)?;
    contexts::check_target(db, context, true)?;
    let id = Uuid::new_v4().to_string();
    db.execute(
        "INSERT INTO recordings(id,context_id,started_at,ended_at) VALUES(?1,?2,?3,?4)",
        params![id, context, start, end],
    )?;
    Ok(id)
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn edit(
    db: &Connection,
    id: &str,
    context: Option<&str>,
    start: i64,
    end: Option<i64>,
    expected_running: bool,
    now: i64,
) -> Result<()> {
    let previous = get(db, id, expected_running)?;
    if end.is_none() != expected_running {
        return Err(Error::invalid(
            "Use the timer controls to start or stop tracking.",
        ));
    }
    // Keep an unchanged same-millisecond stop editable without losing it.
    if !(end == Some(start) && previous.started_at == start && previous.ended_at == end) {
        interval(start, end, now)?;
    }
    contexts::check_target(db, context, expected_running)?;
    db.execute(
        "UPDATE recordings SET context_id=?1,started_at=?2,ended_at=?3 WHERE id=?4",
        params![context, start, end, id],
    )?;
    Ok(())
}
pub(crate) fn delete(db: &Connection, id: &str, expected_running: bool) -> Result<()> {
    get(db, id, expected_running)?;
    db.execute("DELETE FROM recordings WHERE id=?1", [id])?;
    Ok(())
}
