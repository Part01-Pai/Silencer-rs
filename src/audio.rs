use std::collections::{HashSet, HashMap};
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use crate::utils;

pub struct AudioSessionInfo {
    pub name: String,
    pub pid: u32,
    pub window_title: String,
    pub display_name: String, // 新增：用于显示的名称，包含 (1), (2) 等
}

pub struct AudioManager {
    device_enumerator: IMMDeviceEnumerator,
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
            Ok(Self { device_enumerator })
        }
    }

    pub fn get_window_title(_pid: u32) -> String {
        unsafe {
            let mut title = String::new();
            
            unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
                unsafe {
                    let target_pid = lparam.0 as u32;
                    let mut process_id = 0;
                    GetWindowThreadProcessId(hwnd, Some(&mut process_id));
                    if process_id == target_pid && IsWindowVisible(hwnd).as_bool() {
                        let len = GetWindowTextLengthW(hwnd);
                        if len > 0 {
                            *(lparam.0 as *mut usize as *mut HWND) = hwnd;
                            return false.into();
                        }
                    }
                    true.into()
                }
            }

            let mut found_hwnd = HWND(std::ptr::null_mut());
            let _ = EnumWindows(Some(enum_windows_proc), LPARAM(&mut found_hwnd as *mut _ as isize));
            
            if !found_hwnd.is_invalid() {
                let mut buffer = [0u16; 1024];
                let len = GetWindowTextW(found_hwnd, &mut buffer);
                if len > 0 {
                    title = String::from_utf16_lossy(&buffer[..len as usize]);
                }
            }
            title
        }
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

        // 核心改进：处理重名，确保所有重复实例都有 (n) 标识
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
                    // 核心修复：优先使用 PID 匹配判断前台状态，解决无边框窗口问题
                    let is_foreground = pid == foreground_pid;
                    
                    let is_in_list = list.iter().any(|i| i.to_lowercase() == process_name_lower) 
                                  || list.contains(&process_with_pid);

                    if is_whitelist {
                        !is_in_list && !is_foreground
                    } else {
                        is_in_list && !is_foreground
                    }
                };

                simple_volume.SetMute(should_mute, std::ptr::null())?;
            }
        }
        Ok(())
    }
}
