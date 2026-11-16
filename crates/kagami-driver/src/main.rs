//! Test-only JSON-lines transport. Never linked into the desktop application.
use kagami_core::{Change, Query, Store};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
fn main() {
    let directory = tempfile::tempdir().expect("temporary database directory");
    let path = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| directory.path().join("test.sqlite3"));
    let mut store = Store::open(&path).expect("test store");
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
