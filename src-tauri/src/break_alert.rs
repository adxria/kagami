use crate::{now, Database};
use std::{thread, time::Duration};
use tauri::{AppHandle, Emitter};
use windows_sys::Win32::{
    System::Diagnostics::Debug::MessageBeep, UI::WindowsAndMessaging::MB_ICONASTERISK,
};

pub(super) fn schedule(app: AppHandle, database: Database, id: String) -> std::io::Result<()> {
    thread::Builder::new()
        .name("break-alert".into())
        .spawn(move || {
            loop {
                let remaining = {
                    let Ok(mut store) = database.lock() else {
                        return;
                    };
                    let Some(active) = store.active_break().filter(|active| active.id == id) else {
                        return;
                    };
                    let Some(ends_at) = active.ends_at else {
                        return;
                    };
                    let now = now();
                    let remaining = ends_at - now;
                    if active.finished || remaining <= 0 {
                        store.finish_break_if_due(now);
                        // Windows queues the sound asynchronously. Keep the lock until it is queued
                        // so ending the break and sounding it have a consistent order.
                        unsafe { MessageBeep(MB_ICONASTERISK) };
                        break;
                    }
                    remaining
                };
                // Recheck wall time and cancellation promptly after sleep or a clock change.
                thread::sleep(Duration::from_millis(remaining.min(1_000) as u64));
            }
            let _ = app.emit("workspace-changed", ());
        })
        .map(|_| ())
}
