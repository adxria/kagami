use crate::{contexts, durable_fs, recordings, Error, Result, Store};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    io::{BufWriter, Read, Write},
    path::Path,
};
use uuid::Uuid;

const FORMAT: &str = "kagami-data";
const VERSION: u32 = 1;
const DEFAULT_COLOR: &str = "#7FBCF5";

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportMode {
    Add,
    Replace,
}

#[derive(Serialize, Deserialize)]
struct Header {
    format: String,
    version: u32,
}

#[derive(Serialize, Deserialize)]
struct DataFile<P, C = Vec<DataContext>> {
    #[serde(flatten)]
    header: Header,
    #[serde(default)]
    contexts: C,
    recordings: Vec<DataRecording<P>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DataContext {
    path: Vec<String>,
    color: String,
    tracking_locked: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DataRecording<P> {
    #[serde(
        deserialize_with = "Deserialize::deserialize",
        bound(deserialize = "P: Deserialize<'de>")
    )]
    context_path: P,
    started_at: i64,
    ended_at: i64,
}

struct ImportContext {
    parent: Option<usize>,
    name: String,
    key: String,
    color: String,
    tracking_locked: bool,
}

pub struct ImportData {
    contexts: Vec<ImportContext>,
    recordings: Vec<DataRecording<Option<usize>>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataPreview {
    token: String,
    name: String,
    contexts: usize,
    recordings: usize,
}

#[derive(Default)]
pub struct DataImports {
    pending: Option<(String, ImportData)>,
}

impl DataImports {
    pub fn stage(&mut self, name: String, data: ImportData) -> DataPreview {
        let (contexts, recordings) = data.counts();
        let token = Uuid::new_v4().to_string();
        self.pending = Some((token.clone(), data));
        DataPreview {
            token,
            name,
            contexts,
            recordings,
        }
    }

    pub fn cancel(&mut self, token: &str) {
        if self.pending.as_ref().is_some_and(|(id, _)| id == token) {
            self.pending = None;
        }
    }

    pub fn apply(
        &mut self,
        token: &str,
        store: &mut Store,
        mode: ImportMode,
        recovery_dir: &Path,
        now: i64,
    ) -> Result<bool> {
        let (_, data) = self
            .pending
            .as_ref()
            .filter(|(id, _)| id == token)
            .ok_or_else(|| Error::invalid("Choose the data file again before importing."))?;
        let recovered = store.import_data(data, mode, recovery_dir, now)?;
        self.pending = None;
        Ok(recovered)
    }
}

impl ImportData {
    pub fn read(mut reader: impl Read) -> Result<Self> {
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .map_err(|error| Error::invalid(format!("Could not read the data file: {error}")))?;
        // Check the format before decoding the version-specific payload.
        let header: Header = serde_json::from_slice(&bytes).map_err(invalid_json)?;
        if header.format != FORMAT {
            return Err(Error::invalid("Choose a Kagami data JSON file."));
        }
        if header.version != VERSION {
            return Err(Error::invalid(format!(
                "Kagami data version {} is unsupported.",
                header.version
            )));
        }
        let file: DataFile<Option<Vec<String>>> =
            serde_json::from_slice(&bytes).map_err(invalid_json)?;
        let mut data = Self {
            contexts: Vec::new(),
            recordings: Vec::with_capacity(file.recordings.len()),
        };
        let mut paths = HashMap::new();
        let mut defined = HashSet::new();
        for context in file.contexts {
            let index = data.register_path(&mut paths, context.path)?;
            if !defined.insert(index) {
                return Err(Error::invalid(
                    "The data file defines the same context path twice.",
                ));
            }
            data.contexts[index].color = contexts::color(&context.color)?;
            data.contexts[index].tracking_locked = context.tracking_locked;
        }
        for recording in file.recordings {
            if recording.started_at < 0
                || recording.started_at > recording.ended_at
                || recording.ended_at > recordings::MAX_TIMESTAMP
            {
                return Err(Error::invalid(
                    "The data file contains an invalid recording interval.",
                ));
            }
            let context = recording
                .context_path
                .map(|path| data.register_path(&mut paths, path))
                .transpose()?;
            data.recordings.push(DataRecording {
                context_path: context,
                started_at: recording.started_at,
                ended_at: recording.ended_at,
            });
        }
        Ok(data)
    }

    pub fn counts(&self) -> (usize, usize) {
        (self.contexts.len(), self.recordings.len())
    }

    fn register_path(
        &mut self,
        paths: &mut HashMap<(Option<usize>, String), usize>,
        path: Vec<String>,
    ) -> Result<usize> {
        let mut parent = None;
        for segment in path {
            let (name, key) = contexts::name_parts(&segment)?;
            let index = *paths.entry((parent, key.clone())).or_insert_with(|| {
                let index = self.contexts.len();
                self.contexts.push(ImportContext {
                    parent,
                    name,
                    key,
                    color: DEFAULT_COLOR.into(),
                    tracking_locked: false,
                });
                index
            });
            parent = Some(index);
        }
        parent.ok_or_else(|| Error::invalid("A context path needs at least one name."))
    }
}

fn invalid_json(error: serde_json::Error) -> Error {
    Error::invalid(format!("Could not read the Kagami data JSON: {error}"))
}

fn save_error(error: impl std::fmt::Display) -> Error {
    Error {
        code: "data_file",
        message: format!("Could not save the data file: {error}"),
    }
}

impl Store {
    pub fn export_data(&mut self, path: &Path, now: i64) -> Result<()> {
        recordings::valid_now(now)?;
        let tx = self.connection.transaction()?;
        save_snapshot(&tx, path, now)?;
        tx.commit()?;
        Ok(())
    }

    pub fn import_data(
        &mut self,
        data: &ImportData,
        mode: ImportMode,
        recovery_dir: &Path,
        now: i64,
    ) -> Result<bool> {
        recordings::valid_now(now)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut recovery_saved = false;
        if matches!(mode, ImportMode::Replace) {
            if recordings::running(&tx)?.is_some() {
                return Err(Error::invalid(
                    "Stop the current timer before replacing the workspace.",
                ));
            }
            let nonempty: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM contexts) OR EXISTS(SELECT 1 FROM recordings)",
                [],
                |row| row.get(0),
            )?;
            if nonempty {
                let date = chrono::DateTime::from_timestamp_millis(now)
                    .expect("validated timestamp")
                    .format("%Y-%m-%d_%H%M%S");
                let filename = format!("kagami-recovery-{date}-{}.json", Uuid::new_v4());
                durable_fs::create_dir_all(recovery_dir).map_err(save_error)?;
                save_snapshot(&tx, &recovery_dir.join(filename), now)?;
                recovery_saved = true;
                tx.execute("DELETE FROM recordings", [])?;
                contexts::delete_tree(&tx, None)?;
            }
        }
        insert_data(&tx, data)?;
        tx.commit()?;
        if matches!(mode, ImportMode::Replace) {
            self.clear_break();
        }
        Ok(recovery_saved)
    }
}

fn save_snapshot(db: &Connection, path: &Path, now: i64) -> Result<()> {
    let contexts = contexts::all(db)?;
    let mut children: HashMap<_, Vec<_>> = HashMap::new();
    for context in &contexts {
        children
            .entry(context.parent_id.as_deref())
            .or_default()
            .push(context);
    }
    let mut stack: Vec<_> = children
        .get(&None)
        .into_iter()
        .flatten()
        .rev()
        .map(|context| (*context, None))
        .collect();
    let mut paths = HashMap::new();
    let mut definitions = Vec::<DataContext>::with_capacity(contexts.len());
    while let Some((context, parent)) = stack.pop() {
        let mut path = parent.map_or_else(Vec::new, |index: usize| definitions[index].path.clone());
        path.push(context.name.clone());
        let index = definitions.len();
        paths.insert(context.id.as_str(), index);
        definitions.push(DataContext {
            path,
            color: context.color.clone(),
            tracking_locked: context.tracking_locked,
        });
        if let Some(children) = children.get(&Some(context.id.as_str())) {
            stack.extend(children.iter().rev().map(|child| (*child, Some(index))));
        }
    }
    if definitions.len() != contexts.len() {
        return Err(Error::invalid(
            "The workspace contains an invalid context tree.",
        ));
    }
    let mut statement = db.prepare(
        "SELECT id,context_id,started_at,ended_at FROM recordings ORDER BY started_at,id",
    )?;
    let mut entries = Vec::new();
    for recording in statement.query_map([], recordings::row)? {
        let recording = recording?;
        let context_path = recording
            .context_id
            .as_deref()
            .map(|id| {
                paths
                    .get(id)
                    .map(|index| definitions[*index].path.as_slice())
                    .ok_or_else(|| Error::invalid("A recording references a missing context."))
            })
            .transpose()?;
        entries.push(DataRecording {
            context_path,
            started_at: recording.started_at,
            ended_at: recording.ended_at.unwrap_or(now.max(recording.started_at)),
        });
    }
    write_file(
        path,
        &DataFile {
            header: Header {
                format: FORMAT.into(),
                version: VERSION,
            },
            contexts: definitions.as_slice(),
            recordings: entries,
        },
    )
}

fn write_file(path: &Path, data: &impl Serialize) -> Result<()> {
    let mut file =
        tempfile::NamedTempFile::new_in(durable_fs::parent_directory(path)).map_err(save_error)?;
    {
        let mut writer = BufWriter::new(file.as_file_mut());
        serde_json::to_writer(&mut writer, data).map_err(save_error)?;
        writer.write_all(b"\n").map_err(save_error)?;
        writer.flush().map_err(save_error)?;
    }
    durable_fs::publish_file(file, path).map_err(save_error)
}

fn insert_data(db: &Connection, data: &ImportData) -> Result<()> {
    let mut siblings = HashMap::new();
    let mut next_position = HashMap::new();
    for context in contexts::all(db)? {
        let next = next_position.entry(context.parent_id.clone()).or_insert(0);
        *next = (*next).max(context.position + 1);
        siblings.insert((context.parent_id, context.name.to_lowercase()), context.id);
    }
    let mut ids = Vec::<String>::with_capacity(data.contexts.len());
    let mut insert_context = db.prepare(
        "INSERT INTO contexts(id,name,name_key,parent_id,color,tracking_locked,position)
         VALUES(?1,?2,?3,?4,?5,?6,?7)",
    )?;
    for context in &data.contexts {
        let parent = context.parent.map(|index| ids[index].clone());
        if let Some(id) = siblings.get(&(parent.clone(), context.key.clone())) {
            ids.push(id.clone());
            continue;
        }
        let id = Uuid::new_v4().to_string();
        let position = next_position.entry(parent.clone()).or_insert(0);
        insert_context.execute(params![
            id,
            context.name,
            context.key,
            parent,
            context.color,
            context.tracking_locked,
            *position,
        ])?;
        *position += 1;
        ids.push(id);
    }
    let mut insert_recording = db
        .prepare("INSERT INTO recordings(id,context_id,started_at,ended_at) VALUES(?1,?2,?3,?4)")?;
    for recording in &data.recordings {
        let id = Uuid::new_v4().to_string();
        let context = recording.context_path.map(|index| ids[index].as_str());
        insert_recording.execute(params![
            id,
            context,
            recording.started_at,
            recording.ended_at
        ])?;
    }
    Ok(())
}
