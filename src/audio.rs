use std::collections::{HashSet, HashMap};
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Media::Audio::*;
use std::sync::Mutex;
use windows::Win32::System::Com::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use crate::utils;

pub struct AudioSessionInfo {
    pub name: String,
    pub pid: u32,
    pub window_title: String,
    pub display_name: String, // 用于显示的名称，包含 (1), (2) 等
}

pub struct AudioManager {
    device_enumerator: IMMDeviceEnumerator,
    // 保存：当我们修改某个会话的静音状态时，记录其原始状态以便在退出时恢复
    saved_states: Mutex<HashMap<u32, bool>>,
}

impl AudioManager {
    pub fn new() -> Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
            let device_enumerator: IMMDeviceEnumerator = CoCreateInstance(
                &MMDeviceEnumerator,
                None,
                CLSCTX_ALL,
            )?;
            Ok(Self { device_enumerator, saved_states: Mutex::new(HashMap::new()) })
        }
    }

    // 简化：返回空标题（避免复杂的窗口枚举回调实现），主要用于展示
    pub fn get_window_title(_pid: u32) -> String {
        String::new()
    }

    pub fn get_active_sessions(&self) -> Result<Vec<AudioSessionInfo>> {
        let mut sessions = Vec::new();
        unsafe {
            let device = self.device_enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
            let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
            let enumerator = manager.GetSessionEnumerator()?;
            let count = enumerator.GetCount()?;

            for i in 0..count {
                let session = enumerator.GetSession(i)?;
                let session2: IAudioSessionControl2 = session.cast()?;
                let pid = session2.GetProcessId()?;

                if pid == 0 { continue; }

                let name = utils::get_process_name_by_pid(pid);
                let window_title = Self::get_window_title(pid);

                sessions.push(AudioSessionInfo {
                    name,
                    pid,
                    window_title,
                    display_name: String::new(),
                });
            }
        }

        // 处理重名，确保重复实例有 (n) 标识
        let mut total_counts: HashMap<String, usize> = HashMap::new();
        for s in &sessions {
            *total_counts.entry(s.name.clone()).or_insert(0) += 1;
        }

        let mut current_counts: HashMap<String, usize> = HashMap::new();
        for session in &mut sessions {
            let total = total_counts.get(&session.name).cloned().unwrap_or(0);
            if total > 1 {
                let current = current_counts.entry(session.name.clone()).or_insert(0);
                *current += 1;
                session.display_name = format!("{} ({})", session.name, current);
            } else {
                session.display_name = session.name.clone();
            }
        }

        Ok(sessions)
    }

    pub fn update_mute_status(&self, list: &HashSet<String>, is_whitelist: bool, enabled: bool, foreground_pid: u32) -> Result<()> {
        unsafe {
            let device = self.device_enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
            let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
            let enumerator = manager.GetSessionEnumerator()?;
            let count = enumerator.GetCount()?;

            for i in 0..count {
                let session = enumerator.GetSession(i)?;
                let session2: IAudioSessionControl2 = session.cast()?;
                let pid = session2.GetProcessId()?;

                if pid == 0 { continue; }

                let process_name = utils::get_process_name_by_pid(pid);
                let process_name_lower = process_name.to_lowercase();
                let process_with_pid = format!("{} [{}]", process_name, pid);
                let simple_volume: ISimpleAudioVolume = session.cast()?;

                let should_mute = if !enabled {
                    false
                } else {
                    // 优先使用 PID 匹配判断前台状态
                    let is_foreground = pid == foreground_pid;

                    let is_in_list = list.iter().any(|i| i.to_lowercase() == process_name_lower)
                                  || list.contains(&process_with_pid);

                    if is_whitelist {
                        !is_in_list && !is_foreground
                    } else {
                        is_in_list && !is_foreground
                    }
                };

                // 在首次对某个 PID 修改静音状态前，记录其原始状态
                if let Ok(current) = simple_volume.GetMute() {
                    let mut saved = self.saved_states.lock().unwrap();
                    if !saved.contains_key(&pid) {
                        saved.insert(pid, current.as_bool());
                    }
                }

                simple_volume.SetMute(should_mute, std::ptr::null())?;
            }
        }
        Ok(())
    }

    /// 在程序退出或需要恢复时，将所有被记录修改过的会话恢复到原始静音状态
    pub fn restore_saved_states(&self) -> Result<()> {
        let mut errors: Option<windows::core::Error> = None;
        let saved = std::mem::take(&mut *self.saved_states.lock().unwrap());

        unsafe {
            let device = self.device_enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
            let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
            let enumerator = manager.GetSessionEnumerator()?;
            let count = enumerator.GetCount()?;

            for i in 0..count {
                let session = enumerator.GetSession(i)?;
                let session2: IAudioSessionControl2 = session.cast()?;
                let pid = session2.GetProcessId()?;
                if pid == 0 { continue; }

                if saved.contains_key(&pid) {
                    let simple_volume: ISimpleAudioVolume = session.cast()?;
                    // 强制取消静音（确保程序退出后不再保持静音）
                    if let Err(e) = simple_volume.SetMute(false, std::ptr::null()) {
                        errors = Some(e);
                    }
                }
            }
        }

        if let Some(e) = errors {
            Err(e)
        } else {
            Ok(())
        }
    }
}
