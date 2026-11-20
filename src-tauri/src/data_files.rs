use crate::{adapter_error, database_call, now, Database};
use kagami_core::{DataImports, DataPreview, ImportData, ImportMode, Result, Store};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};
use tauri::{AppHandle, Emitter, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

pub struct DataFiles {
    recovery_dir: PathBuf,
    imports: Mutex<DataImports>,
}

impl DataFiles {
    pub fn new(recovery_dir: PathBuf) -> Self {
        Self {
            recovery_dir,
            imports: Mutex::default(),
        }
    }

    fn imports(&self) -> Result<MutexGuard<'_, DataImports>> {
        self.imports
            .lock()
            .map_err(|_| adapter_error("Data file access was interrupted. Restart Kagami."))
    }
}

#[tauri::command]
pub async fn export_data(
    app: AppHandle,
    window: WebviewWindow,
    db: State<'_, Database>,
) -> Result<bool> {
    let choice = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_parent(&window)
            .add_filter("Kagami data", &["json"])
            .set_file_name("kagami-data.json")
            .blocking_save_file()
    })
    .await
    .map_err(|error| adapter_error(error.to_string()))?;
    let Some(choice) = choice else {
        return Ok(false);
    };
    let path = choice
        .into_path()
        .map_err(|error| adapter_error(error.to_string()))?;
    if !is_json(&path) {
        return Err(adapter_error("Choose a file name ending in .json."));
    }
    database_call(db.inner().clone(), move |store| {
        store.export_data(&path, now())
    })
    .await?;
    Ok(true)
}

#[tauri::command]
pub async fn choose_import(
    app: AppHandle,
    window: WebviewWindow,
    data: State<'_, Arc<DataFiles>>,
    recovery: bool,
) -> Result<Option<DataPreview>> {
    let data = data.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = app
            .dialog()
            .file()
            .set_parent(&window)
            .add_filter("Kagami data", &["json"]);
        if recovery {
            dialog = dialog.set_directory(&data.recovery_dir);
        }
        let Some(choice) = dialog.blocking_pick_file() else {
            return Ok(None);
        };
        let path = choice
            .into_path()
            .map_err(|error| adapter_error(error.to_string()))?;
        let file = File::open(&path)
            .map_err(|error| adapter_error(format!("Could not open the data file: {error}")))?;
        let import = ImportData::read(file)?;
        let name = path
            .file_name()
            .ok_or_else(|| adapter_error("The data file has no name."))?;
        Ok(Some(
            data.imports()?
                .stage(name.to_string_lossy().into_owned(), import),
        ))
    })
    .await
    .map_err(|error| adapter_error(error.to_string()))?
}

#[tauri::command]
pub async fn cancel_import(data: State<'_, Arc<DataFiles>>, token: String) -> Result<()> {
    data.imports()?.cancel(&token);
    Ok(())
}

#[tauri::command]
pub async fn apply_import(
    app: AppHandle,
    db: State<'_, Database>,
    data: State<'_, Arc<DataFiles>>,
    token: String,
    mode: ImportMode,
) -> Result<bool> {
    let data = data.inner().clone();
    let recovered = database_call(db.inner().clone(), move |store: &mut Store| {
        data.imports()?
            .apply(&token, store, mode, &data.recovery_dir, now())
    })
    .await?;
    let _ = app.emit("workspace-changed", ());
    Ok(recovered)
}

#[tauri::command]
pub async fn has_recovery_files(data: State<'_, Arc<DataFiles>>) -> Result<bool> {
    let directory = data.recovery_dir.clone();
    tauri::async_runtime::spawn_blocking(move || match fs::read_dir(directory) {
        Ok(entries) => Ok(entries
            .filter_map(|entry| entry.ok())
            .any(|entry| is_json(&entry.path()))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(adapter_error(format!(
            "Could not open the recovery folder: {error}"
        ))),
    })
    .await
    .map_err(|error| adapter_error(error.to_string()))?
}

fn is_json(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
}
