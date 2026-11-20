use crate::{
    contexts, recordings, ActiveBreak, ContextTotals, Error, Query, Recording, Result, Totals,
    Workspace,
};
use rusqlite::{params, Connection};
use std::collections::{HashMap, HashSet};

fn clipped(start: i64, end: i64, lower: i64, upper: i64) -> i64 {
    (end.min(upper) - start.max(lower)).max(0)
}
fn totals(recording: &Recording, q: Query, now: i64) -> Totals {
    let end = recording.ended_at.unwrap_or(now);
    Totals {
        today_ms: clipped(recording.started_at, end, q.today_start, q.today_end),
        week_ms: clipped(recording.started_at, end, q.week_start, q.week_end),
        lifetime_ms: (end - recording.started_at).max(0),
    }
}
pub(crate) fn read(
    db: &Connection,
    q: Query,
    now: i64,
    active_break: Option<ActiveBreak>,
) -> Result<Workspace> {
    for (start, end) in [(q.week_start, q.week_end), (q.today_start, q.today_end)] {
        if start < 0 || end <= start || end > 253_402_300_799_999 {
            return Err(Error::invalid("Invalid calendar boundaries."));
        }
    }
    let contexts = contexts::all(db)?;
    let running = recordings::running(db)?;
    let mut direct: HashMap<Option<String>, Totals> = HashMap::new();
    // Stream history; don't retain a second copy or issue per-context queries.
    let mut statement = db.prepare("SELECT id,context_id,started_at,ended_at FROM recordings")?;
    let mut total = Totals::default();
    for recording in statement.query_map([], recordings::row)? {
        let recording = recording?;
        let value = totals(&recording, q, now);
        total += value;
        *direct.entry(recording.context_id).or_default() += value;
    }
    let mut inclusive = direct.clone();
    for context in &contexts {
        let value = direct
            .get(&Some(context.id.clone()))
            .copied()
            .unwrap_or_default();
        let mut parent = context.parent_id.as_ref();
        while let Some(id) = parent {
            *inclusive.entry(Some(id.clone())).or_default() += value;
            parent = contexts
                .iter()
                .find(|c| &c.id == id)
                .and_then(|c| c.parent_id.as_ref());
        }
    }
    let context_totals = contexts
        .iter()
        .map(|c| ContextTotals {
            context_id: c.id.clone(),
            direct: direct.get(&Some(c.id.clone())).copied().unwrap_or_default(),
            total: inclusive
                .get(&Some(c.id.clone()))
                .copied()
                .unwrap_or_default(),
        })
        .collect();
    let recordings = db.prepare("SELECT id,context_id,started_at,ended_at FROM recordings WHERE started_at < ?2 AND (COALESCE(ended_at,?3) > ?1 OR (ended_at=started_at AND started_at >= ?1) OR (ended_at IS NULL AND ?3 = ?1)) ORDER BY started_at,id")?
        .query_map(params![q.week_start,q.week_end,now],recordings::row)?.collect::<std::result::Result<Vec<_>,_>>()?;
    let mut overlaps = HashSet::new();
    let mut latest: Option<(&str, i64)> = None;
    for recording in &recordings {
        let end = recording.ended_at.unwrap_or(now).min(q.week_end);
        let start = recording.started_at.max(q.week_start);
        if end <= start {
            continue;
        }
        if let Some((id, latest_end)) = latest {
            if start < latest_end {
                overlaps.insert(id.to_owned());
                overlaps.insert(recording.id.clone());
            }
        }
        if latest.is_none_or(|(_, latest_end)| end > latest_end) {
            latest = Some((&recording.id, end));
        }
    }
    let mut overlap_ids: Vec<_> = overlaps.into_iter().collect();
    overlap_ids.sort();
    Ok(Workspace {
        contexts,
        recordings,
        running,
        active_break,
        overlap_ids,
        totals: total,
        unassigned: direct.get(&None).copied().unwrap_or_default(),
        context_totals,
        calculated_at: now,
        query: q,
    })
}
