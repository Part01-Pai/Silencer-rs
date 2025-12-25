# Silencer-rs

Silencer-rs is a Rust refactor of https://github.com/AsterNighT/silencer (formerly FocusMute). During the refactor we fixed several known bugs, notably the fast window-switch audio leak (event debouncing + periodic sync).

## Overview

Silencer-rs automatically mutes or unmutes background applications based on the foreground window, preventing unwanted sounds when switching windows or playing games.

## Features

- Blacklist mode: mute listed processes when not foreground.
- Whitelist mode: only whitelist processes are allowed to stay audible in background.
- Auto-mute: real-time foreground window detection and audio session updates.
- Debounce + periodic sync: combines event debouncing (e.g. 50ms) with periodic checks (e.g. 200ms) to reduce missed or incorrect mutes.
- Multi-instance support: distinguish processes by name and PID, auto-number identical names.
- Modern UI: built with `egui` and `eframe`.

## Environment

- Rust (stable)
- Windows (uses WASAPI and Win32 APIs)
- Visual Studio Build Tools (if building with MSVC)

## Build

```powershell
cargo build --release
```

## Run

```powershell
cargo run --release
```

## Implementation

- UI: `egui` + `eframe`.
- Windows API: `windows` (windows-rs) for Win32/COM.
- Audio: interacts with WASAPI (`IAudioSessionManager2`, `ISimpleAudioVolume`).
- Events: uses `SetWinEventHook` for `EVENT_SYSTEM_FOREGROUND`.

## Fixes & Improvements (highlights)

- Event debouncing & merging: process only the latest event state by clearing the queue.
- Periodic forced sync: periodic checks (e.g. 200ms) to correct missed states.
- Stronger foreground detection: handle transient system windows by keeping previous state or retrying.
- Borderless/fullscreen fix: match by PID instead of process name to handle proxy/multi-window cases.
- Better process name retrieval: fall back to `QueryFullProcessImageNameW`, and show `process (PID)` if unavailable.
- Multi-instance numbering: auto-number same-name processes.

## Sponsor

If you want to support the project, donations are welcome  thank you!

![Sponsor](photo/zanzhu.png)
