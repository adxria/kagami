//! Test-only JSON-lines transport. Never linked into the desktop application.
use kagami_core::{Change, DataImports, ImportData, ImportMode, Query, Store};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
fn main() {
    let directory = tempfile::tempdir().expect("temporary database directory");
    let path = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| directory.path().join("test.sqlite3"));
    let mut store = Store::open(&path).expect("test store");
    let recovery = directory.path().join("recovery");
    let mut imports = DataImports::default();
    let mut picker = None;
    let mut fail_read = false;
    let mut fail_change = false;
    for line in io::stdin().lock().lines() {
        let result = (|| -> Result<Value, Box<dyn std::error::Error>> {
            let request: Value = serde_json::from_str(&line?)?;
            let now = request["now"].as_i64().ok_or("missing controlled time")?;
            match request["method"].as_str().ok_or("missing method")? {
                "read" => {
                    if std::mem::take(&mut fail_read) {
                        return Err("Injected refresh failure".into());
                    }
                    let query: Query = serde_json::from_value(request["query"].clone())?;
                    Ok(serde_json::to_value(store.read_workspace(query, now)?)?)
                }
                "change" => {
                    if std::mem::take(&mut fail_change) {
                        return Err("Injected write failure".into());
                    }
                    let change: Change = serde_json::from_value(request["change"].clone())?;
                    Ok(serde_json::to_value(store.apply_change(change, now)?)?)
                }
                "next_import" => {
                    let name = request["name"].as_str().ok_or("missing file name")?;
                    let contents = request["data"].to_string();
                    picker = Some((name.to_owned(), contents.into_bytes()));
                    Ok(Value::Null)
                }
                "choose_import" => {
                    if request["recovery"] == true {
                        if let Some(entry) = std::fs::read_dir(&recovery)?.next().transpose()? {
                            let data = ImportData::read(std::fs::File::open(entry.path())?)?;
                            let name = entry.file_name().to_string_lossy().into_owned();
                            return Ok(serde_json::to_value(imports.stage(name, data))?);
                        }
                    } else if let Some((name, contents)) = picker.take() {
                        let data = ImportData::read(contents.as_slice())?;
                        return Ok(serde_json::to_value(imports.stage(name, data))?);
                    }
                    Ok(Value::Null)
                }
                "cancel_import" => {
                    let token = request["token"].as_str().ok_or("missing import token")?;
                    imports.cancel(token);
                    Ok(Value::Null)
                }
                "apply_import" => {
                    let token = request["token"].as_str().ok_or("missing import token")?;
                    let mode: ImportMode = serde_json::from_value(request["mode"].clone())?;
                    Ok(json!(
                        imports.apply(token, &mut store, mode, &recovery, now)?
                    ))
                }
                "export_data" => {
                    store.export_data(&directory.path().join("kagami-data.json"), now)?;
                    Ok(json!(true))
                }
                "has_recovery_files" => Ok(json!(recovery.is_dir())),
                "fail_read" => {
                    fail_read = true;
                    Ok(Value::Null)
                }
                "fail_change" => {
                    fail_change = true;
                    Ok(Value::Null)
                }
                "reopen" => {
                    store = Store::open(&path)?;
                    Ok(Value::Null)
                }
                _ => Err("unknown method".into()),
            }
        })();
        let response = match result {
            Ok(value) => json!({"ok":value}),
            Err(error) => json!({"error":{"message":error.to_string()}}),
        };
        println!("{response}");
        io::stdout().flush().expect("driver output");
    }
}
