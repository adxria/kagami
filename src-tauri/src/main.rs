#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod data_files;

use kagami_core::{Change, Error, Query, Receipt, Store, Workspace};
use std::{
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

type Database = Arc<Mutex<Store>>;
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
fn adapter_error(message: impl Into<String>) -> Error {
    Error {
        code: "desktop",
        message: message.into(),
    }
}
async fn database_call<T: Send + 'static>(
    db: Database,
    operation: impl FnOnce(&mut Store) -> kagami_core::Result<T> + Send + 'static,
) -> kagami_core::Result<T> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut store = db
            .lock()
            .map_err(|_| adapter_error("Local database access was interrupted. Restart Kagami."))?;
        operation(&mut store)
    })
    .await
    .map_err(|e| adapter_error(e.to_string()))?
}
#[tauri::command]
async fn read_workspace(
    db: tauri::State<'_, Database>,
    query: Query,
) -> kagami_core::Result<Workspace> {
    database_call(db.inner().clone(), move |store| {
        store.read_workspace(query, now())
    })
    .await
}
#[tauri::command]
async fn apply_change(
    app: tauri::AppHandle,
    db: tauri::State<'_, Database>,
    change: Change,
) -> kagami_core::Result<Receipt> {
    let receipt = database_call(db.inner().clone(), move |store| {
        store.apply_change(change, now())
    })
    .await?;
    let _ = app.emit("workspace-changed", ());
    Ok(receipt)
}
fn show(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
fn main() {
    let mut context = tauri::generate_context!();
    if cfg!(debug_assertions) {
        context.config_mut().identifier = "app.kagami.development".into();
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show(app)))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let identity = &app.config().identifier;
            let directory = app.path().local_data_dir()?.join(identity);
            let database_dir = directory.join("database");
            std::fs::create_dir_all(&database_dir)?;
            let store = Store::open(database_dir.join("kagami.sqlite3"))?;
            app.manage(Arc::new(Mutex::new(store)) as Database);
            app.manage(Arc::new(data_files::DataFiles::new(
                directory.join("recovery"),
            )));
            // Build at runtime so WebView2's profile never shares the database path
            // or writes next to the executable in Program Files.
            let window = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title(if app.config().identifier.ends_with("test") {
                "kagami — test"
            } else {
                "kagami"
            })
            .inner_size(1280., 860.)
            .min_inner_size(960., 640.)
            .theme(Some(tauri::Theme::Dark))
            .background_color(tauri::window::Color(26, 26, 29, 255))
            .data_directory(directory.join("webview"))
            // Let HTML drag-and-drop receive the sidebar's mouse drags in WebView2.
            .disable_drag_drop_handler()
            .build()?;
            let _ = window;
            let open = MenuItem::with_id(app, "open", "Open Kagami", true, None::<&str>)?;
            let stop = MenuItem::with_id(app, "stop", "Stop timer", true, None::<&str>)?;
            let quit =
                MenuItem::with_id(app, "quit", "Quit (keep timer running)", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &stop, &quit])?;
            TrayIconBuilder::with_id("main")
                .icon(
                    app.default_window_icon()
                        .expect("bundled application icon")
                        .clone(),
                )
                .tooltip(app.config().product_name.as_deref().unwrap_or("kagami"))
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show(app),
                    "quit" => app.exit(0),
                    "stop" => {
                        let app = app.clone();
                        let db = app.state::<Database>().inner().clone();
                        tauri::async_runtime::spawn(async move {
                            let result = database_call(db, |store| {
                                if let Some(timer) = store.running()? {
                                    store
                                        .apply_change(Change::StopTimer { id: timer.id }, now())?;
                                }
                                Ok(())
                            })
                            .await;
                            match result {
                                Ok(()) => {
                                    let _ = app.emit("workspace-changed", ());
                                }
                                Err(error) => {
                                    show(&app);
                                    let _ = app.emit("desktop-error", error);
                                }
                            }
                        });
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        show(tray.app_handle());
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.hide().is_ok() {
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            read_workspace,
            apply_change,
            data_files::export_data,
            data_files::choose_import,
            data_files::cancel_import,
            data_files::apply_import,
            data_files::has_recovery_files,
        ])
        .run(context)
        .expect("Could not start Kagami");
}
