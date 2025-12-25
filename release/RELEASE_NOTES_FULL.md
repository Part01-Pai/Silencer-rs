# Silencer-rs

本项目由 https://github.com/AsterNighT/silencer 使用 Rust 重构而来（原名 FocusMute），在重构过程中修复若干已知 bug，重点修复了快速切换窗口时的漏音问题（事件防抖 + 周期同步）。

## 项目简介

Silencer-rs 会根据当前前台窗口自动静音或恢复后台应用的声音，目标是在切换窗口或游戏场景时避免不必要的声音干扰。示例场景：在三角洲官方启动器与 WeGame 双开游戏并最小化切换时，后台游戏无法自动静音的问题。

## 功能

- 黑名单模式：列入黑名单的进程在不处于前台时会被静音。
- 白名单模式：仅允许白名单中的进程在后台不被静音，其它进程在后台时会被静音。
- 自动静音：实时监听前台窗口变化并更新音频会话状态。
- 防抖与周期同步：结合事件防抖（例如 50ms）与周期性检查（例如 200ms）以减少漏静音或误静音。
- 多实例区分：支持按进程名与 PID 区分不同实例，并自动编号同名进程。
- 现代化 UI：基于 `egui` 与 `eframe` 的卡片式界面。

## 环境

- Rust（stable）
- Windows（使用 WASAPI 与 Win32 API）
- 若使用 MSVC 工具链请安装 Visual Studio Build Tools

## 编译

```powershell
cargo build --release
```

## 运行

```powershell
cargo run --release
```

## 实现

- UI：使用 [egui](https://github.com/emilk/egui) 与 `eframe`。
- Windows API：使用 `windows`（windows-rs）调用 Win32/COM 接口。
- 音频：通过 WASAPI (`IAudioSessionManager2`, `ISimpleAudioVolume`) 控制会话静音。
- 事件监听：使用 `SetWinEventHook` 监听 `EVENT_SYSTEM_FOREGROUND`。

## 修复与改进（要点）

- 事件防抖与合并：不再为每个事件分别处理，而是清空事件队列，仅处理最新一次切换状态。
- 周期性强制同步：增加高频周期检查（如 200ms），即使钩子遗漏，也能快速纠正状态。
- 前台判定加固：遇到系统过渡窗口（如 TaskSwitcher）时保持上一次状态或重试，避免瞬间误判。
- 无边框/全屏窗口修复：由基于进程名匹配改为基于 PID 对比，解决代理进程或多窗口导致的不一致问题。
- 进程名获取改进：若 `GetModuleBaseNameW` 失败，尝试 `QueryFullProcessImageNameW`；兜底显示为 进程 (PID) 以便识别。
- 多实例自动编号：当发现多个同名进程时自动标注 `进程名 (1)`、`进程名 (2)` 等。

## 请你喝杯奶茶叭

如果此项目能帮助到您，我万分荣幸，或者您愿意请我喝杯奶茶 Oᴗoಣ

微信奶茶🍦：

![微信奶茶](photo/naicha_weixin.png)

支付宝奶茶🍰：

![支付宝奶茶](photo/naicha_zhifubao.png)


---

# Silencer-rs

Silencer-rs is a Rust refactor of https://github.com/AsterNighT/silencer (formerly FocusMute). During the refactor we fixed several known bugs, notably the fast window-switch audio leak (event debouncing + periodic sync).

## Overview

Silencer-rs automatically mutes or unmutes background applications based on the foreground window, preventing unwanted sounds when switching windows or playing games. Example scenario: when running Delta Force with its official launcher alongside a WeGame second instance and switching while minimized, the background game may fail to auto-mute.

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

## Buy me a milk tea

If this project helped you, I'd be very grateful — or you can buy me a milk tea Oᴗoಣ

WeChat milk tea 🍦:

![WeChat milk tea](photo/naicha_weixin.png)

Alipay milk tea 🍰:

![Alipay milk tea](photo/naicha_zhifubao.png)

