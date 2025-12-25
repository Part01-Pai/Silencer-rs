# Silencer-rs

本项目由 https://github.com/AsterNighT/silencer 使用 Rust 重构而来（原名 FocusMute），在重构过程中修复若干已知 bug，重点修复了快速切换窗口时的漏音问题（事件防抖 + 周期同步）。

## 项目简介

Silencer-rs 会根据当前前台窗口自动静音或恢复后台应用的声音，目标是在切换窗口或游戏场景时避免不必要的声音干扰。

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

## 赞助

如果此项目能帮助到您，我万分荣幸，或者您愿意请我喝杯奶茶 Oᴗoಣ

![赞助](photo/zanzhu.png)
