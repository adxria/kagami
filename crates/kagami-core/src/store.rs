use crate::{
    contexts, recordings, reporting, ActiveBreak, Change, Error, Query, Receipt, Result, Workspace,
};
use rusqlite::Connection;
use std::{path::Path, time::Duration};

pub struct Store {
    pub(crate) connection: Connection,
    active_break: Option<ActiveBreak>,
}
impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::initialize(Connection::open(path)?)
    }
    pub fn memory() -> Result<Self> {
        Self::initialize(Connection::open_in_memory()?)
    }
    fn initialize(mut connection: Connection) -> Result<Self> {
        // Inspect first. Never change journal settings or schema on a newer database.
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if !(0..=2).contains(&version) {
            return Err(Error {
                code: "version",
                message: format!(
                    "Database version {version} is unsupported. Use a compatible Kagami version."
                ),
            });
        }
        connection.busy_timeout(Duration::from_secs(5))?;
        if version < 2 {
            // Rebuild contexts without cascading through recordings that reference it.
            connection.pragma_update(None, "foreign_keys", false)?;
            let tx =
                connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let version: i64 = tx.pragma_query_value(None, "user_version", |row| row.get(0))?;
            if version == 0 {
                tx.execute_batch(include_str!("schema.sql"))?;
            }
            if version < 2 {
                tx.execute_batch(include_str!("context-names-v2.sql"))?;
                if tx.prepare("PRAGMA foreign_key_check")?.exists([])? {
                    return Err(Error::invalid(
                        "The database has invalid context references.",
                    ));
                }
            }
            tx.commit()?;
        }
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        Ok(Self {
            connection,
            active_break: None,
        })
    }
    pub fn apply_change(&mut self, change: Change, now: i64) -> Result<Receipt> {
        recordings::valid_now(now)?;
        let tx = self.connection.transaction()?;
        // Apply any replacement or clear only after the recording transaction commits.
        let mut break_update = None;
        let id = match change {
            Change::CreateContext {
                name,
                parent_id,
                color,
            } => contexts::create(&tx, &name, parent_id.as_deref(), &color)?,
            Change::RenameContext { id, name } => {
                contexts::rename(&tx, &id, &name)?;
                id
            }
            Change::ColorContext { id, color } => {
                contexts::set_color(&tx, &id, &color)?;
                id
            }
            Change::MoveContext {
                id,
                parent_id,
                position,
            } => {
                contexts::move_context(&tx, &id, parent_id.as_deref(), position)?;
                id
            }
            Change::LockContext { id, locked } => {
                contexts::set_locked(&tx, &id, locked)?;
                id
            }
            Change::DeleteContext { id } => {
                contexts::delete(&tx, &id)?;
                id
            }
            Change::StartTimer { context_id } => {
                break_update = Some(None);
                recordings::start(&tx, context_id.as_deref(), now)?
            }
            Change::StopTimer { id } => {
                recordings::stop(&tx, &id, now)?;
                id
            }
            Change::PrepareBreak { id } => {
                let recording = recordings::stop(&tx, &id, now)?;
                break_update = Some(Some(ActiveBreak {
                    id: id.clone(),
                    context_id: recording.context_id,
                    focused_ms: now - recording.started_at,
                    ends_at: None,
                    finished: false,
                }));
                id
            }
            Change::StartBreak { id, duration_ms } => {
                let mut active = self
                    .active_break
                    .as_ref()
                    .filter(|active| active.id == id && active.ends_at.is_none())
                    .cloned()
                    .ok_or_else(Error::stale)?;
                let ends_at = now
                    .checked_add(duration_ms)
                    .filter(|ends_at| *ends_at > now && *ends_at <= recordings::MAX_TIMESTAMP)
                    .ok_or_else(|| Error::invalid("Enter a positive, valid break duration."))?;
                active.ends_at = Some(ends_at);
                break_update = Some(Some(active));
                id
            }
            Change::EndBreak { id } => {
                if self
                    .active_break
                    .as_ref()
                    .is_none_or(|active| active.id != id)
                {
                    return Err(Error::stale());
                }
                break_update = Some(None);
                id
            }
            Change::AssignTimer { id, context_id } => {
                recordings::assign(&tx, &id, context_id.as_deref())?;
                id
            }
            Change::CreateRecording {
                context_id,
                started_at,
                ended_at,
            } => recordings::create(&tx, context_id.as_deref(), started_at, ended_at, now)?,
            Change::EditRecording {
                id,
                context_id,
                started_at,
                ended_at,
                expected_running,
            } => {
                recordings::edit(
                    &tx,
                    &id,
                    context_id.as_deref(),
                    started_at,
                    ended_at,
                    expected_running,
                    now,
                )?;
                id
            }
            Change::DeleteRecording {
                id,
                expected_running,
            } => {
                recordings::delete(&tx, &id, expected_running)?;
                id
            }
        };
        tx.commit()?;
        if let Some(next) = break_update {
            self.active_break = next;
        }
        Ok(Receipt { id })
    }
    pub fn read_workspace(&mut self, query: Query, now: i64) -> Result<Workspace> {
        recordings::valid_now(now)?;
        self.finish_break_if_due(now);
        let tx = self.connection.transaction()?;
        let workspace = reporting::read(&tx, query, now, self.active_break.clone())?;
        tx.commit()?;
        Ok(workspace)
    }
    pub fn running(&self) -> Result<Option<crate::Recording>> {
        recordings::running(&self.connection)
    }
    pub fn active_break(&self) -> Option<&ActiveBreak> {
        self.active_break.as_ref()
    }
    pub fn finish_break_if_due(&mut self, now: i64) {
        if let Some(active) = &mut self.active_break {
            active.finished |= active.ends_at.is_some_and(|ends_at| now >= ends_at);
        }
    }
    pub(crate) fn clear_break(&mut self) {
        self.active_break = None;
    }
}
