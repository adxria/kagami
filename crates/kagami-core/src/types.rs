use serde::{Deserialize, Serialize};
use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    pub code: &'static str,
    pub message: String,
}
impl Error {
    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self {
            code: "invalid",
            message: message.into(),
        }
    }
    pub(crate) fn stale() -> Self {
        Self {
            code: "stale",
            message: "This item changed or was removed. Refresh and try again.".into(),
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Error {}
impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        Self {
            code: "storage",
            message: format!("Could not save or read local data: {error}"),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub color: String,
    pub tracking_locked: bool,
    pub position: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recording {
    pub id: String,
    pub context_id: Option<String>,
    pub started_at: i64,
    pub ended_at: Option<i64>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Query {
    pub week_start: i64,
    pub week_end: i64,
    pub today_start: i64,
    pub today_end: i64,
}
#[derive(Debug, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Change {
    CreateContext {
        name: String,
        parent_id: Option<String>,
        color: String,
    },
    RenameContext {
        id: String,
        name: String,
    },
    ColorContext {
        id: String,
        color: String,
    },
    MoveContext {
        id: String,
        parent_id: Option<String>,
        position: usize,
    },
    LockContext {
        id: String,
        locked: bool,
    },
    DeleteContext {
        id: String,
    },
    StartTimer {
        context_id: Option<String>,
    },
    StopTimer {
        id: String,
    },
    AssignTimer {
        id: String,
        context_id: Option<String>,
    },
    CreateRecording {
        context_id: Option<String>,
        started_at: i64,
        ended_at: i64,
    },
    EditRecording {
        id: String,
        context_id: Option<String>,
        started_at: i64,
        ended_at: Option<i64>,
        expected_running: bool,
    },
    DeleteRecording {
        id: String,
        expected_running: bool,
    },
}
#[derive(Debug, Serialize)]
pub struct Receipt {
    pub id: String,
}
#[derive(Debug, Default, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub today_ms: i64,
    pub week_ms: i64,
    pub lifetime_ms: i64,
}
impl std::ops::AddAssign for Totals {
    fn add_assign(&mut self, other: Self) {
        self.today_ms += other.today_ms;
        self.week_ms += other.week_ms;
        self.lifetime_ms += other.lifetime_ms;
    }
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextTotals {
    pub context_id: String,
    pub direct: Totals,
    pub total: Totals,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub contexts: Vec<Context>,
    pub recordings: Vec<Recording>,
    pub running: Option<Recording>,
    pub overlap_ids: Vec<String>,
    pub totals: Totals,
    pub unassigned: Totals,
    pub context_totals: Vec<ContextTotals>,
    pub calculated_at: i64,
    pub query: Query,
}
