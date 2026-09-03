# Clipture

Clipture is a Windows-first, Linux-ready desktop capture app built with **Tauri v2**, **Rust**, and [`libobs-rs`](https://github.com/libobs-rs/libobs-rs). It is designed around one job: keep capture ready without making the user operate OBS.

This repository is a ground-up replacement for the older Electron/`obs-studio-node` implementation. The current code intentionally does not preserve the old architecture, server-auth dependency, custom OBS updater, rSPC layer, or legacy UI.

## What it does

- Keeps an OBS replay buffer ready in the background.
- `F9` queues a clip with configurable pre-roll and post-roll (30s + 10s by default).
- Supports overlapping clip requests without silently merging moments.
- Records full sessions manually or automatically for detected games.
- Automatically prefers game capture and falls back to manual game/window/monitor selection.
- Captures application/game audio and an optional microphone.
- Shows the live OBS scene directly inside the Home capture panel on Windows using a native OBS display child window (no frame copies through JavaScript).
- Stores ordinary media files under `Clipture/<Game>/<YYYY-MM-DD>/` with small `.clipture.json` sidecars.
- Provides a local Clips / Recordings library and quick, non-destructive trimming.
- Never removes clips automatically. Optional storage-budget cleanup only targets unprotected full recordings.
- Runs from the system tray, supports global hotkeys, notifications, and optional system startup.

The UI is deliberately small: **Home / Library / Settings**. OBS-specific controls stay behind Advanced settings instead of leaking into the normal workflow.

## Architecture

```text
React / TypeScript
  └─ typed desktop facade (Tauri invoke + events)
       └─ thin Tauri commands
            ├─ CaptureController
            │    └─ dedicated OBS worker thread
            │         ├─ libobs-simple: replay / recording / common capture
            │         └─ libobs-wrapper: context / scene / native preview / capability discovery
            ├─ LibraryStore
            │    └─ normal media files + .clipture.json sidecars
            └─ SettingsStore
```

`CaptureController` is intentionally the deep module boundary. The frontend asks for actions such as “save clip”, “record”, or “capture this source”; it never owns native OBS objects or their lifetime/thread-affinity rules.

## libobs-rs policy

Clipture follows the upstream `main` branch directly:

```toml
libobs-simple = { git = "https://github.com/libobs-rs/libobs-rs", branch = "main" }
libobs-wrapper = { git = "https://github.com/libobs-rs/libobs-rs", branch = "main" }
libobs-bootstrapper = { git = "https://github.com/libobs-rs/libobs-rs", branch = "main" }
```

On Windows, `libobs-bootstrapper` provisions a compatible official OBS runtime before the first OBS call and the build script enables delay-loading of `obs.dll`. The old dummy-DLL/updater path is gone. The Home preview uses `libobs-wrapper::display` to render the OBS main texture into a child HWND whose bounds are synchronized to the React preview slot; the display is explicitly removed before its parent Tauri window is destroyed.

On Linux, use a system/source-built libobs installation. The code keeps platform-specific capture behind the engine adapter so Windows remains the primary target without making the rest of the application Windows-specific.

## Development

Requirements:

- Rust stable
- Node.js + pnpm
- Windows: WebView2 (normally present) and the normal MSVC toolchain
- Linux: Tauri GTK/WebKit development packages plus libobs development files

Install and validate the frontend:

```bash
pnpm install
pnpm typecheck
pnpm build
```

Validate Rust:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Run the desktop app:

```bash
pnpm tauri dev
```

## Product direction

The current overhaul intentionally focuses on local capture. The old **Overlay** and **Discover** concepts are not implemented now, but the event-driven capture core and local library keep clean seams for adding them later without coupling them to the recorder.
