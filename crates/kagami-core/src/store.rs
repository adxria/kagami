use crate::{contexts, recordings, reporting, Change, Error, Query, Receipt, Result, Workspace};
use rusqlite::Connection;
use std::{path::Path, time::Duration};

pub struct Store {
    pub(crate) connection: Connection,
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
        if version != 0 && version != 1 {
            return Err(Error {
                code: "version",
                message: format!(
                    "Database version {version} is unsupported. Use a compatible Kagami version."
                ),
            });
        }
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        if version == 0 {
            let tx = connection.transaction()?;
            tx.execute_batch(include_str!("schema.sql"))?;
            tx.commit()?;
        }
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        Ok(Self { connection })
    }
    pub fn apply_change(&mut self, change: Change, now: i64) -> Result<Receipt> {
        recordings::valid_now(now)?;
        let tx = self.connection.transaction()?;
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
                recordings::start(&tx, context_id.as_deref(), now)?
            }
            Change::StopTimer { id } => {
                recordings::stop(&tx, &id, now)?;
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
        Ok(Receipt { id })
    }
    pub fn read_workspace(&mut self, query: Query, now: i64) -> Result<Workspace> {
        recordings::valid_now(now)?;
        let tx = self.connection.transaction()?;
        let workspace = reporting::read(&tx, query, now)?;
        tx.commit()?;
        Ok(workspace)
    }
    pub fn running(&self) -> Result<Option<crate::Recording>> {
        recordings::running(&self.connection)
    }
}
