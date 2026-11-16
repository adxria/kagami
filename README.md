# Kagami

A local-first time tracker for Windows 11.

Kagami tracks time spent on activities and organizes them into contexts. Recordings can be reviewed and edited on a weekly timeline, with time totals available for individual activities and contexts.

## Usage

Select a context and start the timer, or track time without a context under Unassigned. Stopping the timer saves the recording. To switch activities, stop the current timer and start another. Changing the context while the timer is running reassigns the entire recording.

To add a past recording, use **Add entry** or double-click an empty area of the timeline. Hold Alt and scroll to zoom the timeline.

Closing the window sends Kagami to the system tray. The timer continues counting even when the application is closed or the computer is asleep. It must be stopped manually.

## Data and backups

Kagami stores recordings locally in an SQLite database:

```text
%LOCALAPPDATA%\app.kagami.desktop\database\kagami.sqlite3
```

To back up your data, quit Kagami and copy the entire `database` folder, including any WAL files.

Updates and normal uninstallation leave your data intact. Each Windows account has its own database.

## Development

Requires Node.js 24+, npm, stable Rust, and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

```sh
npm ci
npm run desktop
```

`npm run dev` runs the interface without persistence. Debug builds use a separate data profile.

### Checks

```sh
npm run build
npm test
npm run test:core
npm run format:check
npm run lint:rust
```

UI tests run against the Rust core using temporary databases. Install Chromium and its dependencies, then run the tests:

```sh
npx playwright install --with-deps chromium
npm run test:ui
```

## Windows builds

Windows builds require the MSVC Rust toolchain.

```sh
npm run build:windows    # Executable
npm run bundle:windows   # NSIS installer
```

Build outputs are located under `target/x86_64-pc-windows-msvc/release/`:

- `kagami.exe` — application executable
- `bundle/nsis/*-setup.exe` — NSIS installer

To build with an isolated data profile:

```sh
npm run build:windows:test
```

This produces a separate application using `app.kagami.test`, located at:

```text
target/test-profile/x86_64-pc-windows-msvc/release/kagami.exe
```

The normal output directory can be changed with `CARGO_TARGET_DIR`. The test build uses its own target directory.

### Cross-compiling from Linux or WSL

The same build commands work on Linux and WSL with LLVM/Clang, LLD, `cargo-xwin`, and the `x86_64-pc-windows-msvc` Rust target installed. NSIS is also required to build installers. All tools must be available on PATH.
