use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::Threading::*;
use windows::Win32::System::ProcessStatus::*;

pub fn get_foreground_pid() -> u32 {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return 0;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        pid
    }
}

pub fn get_process_name_by_pid(pid: u32) -> String {
    if pid == 0 {
        return "System".to_string();
    }
    
    // 尝试多种权限打开进程，优先使用受限权限以提高成功率
    let access_flags = [
        PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
    ];

    for &flags in &access_flags {
        unsafe {
            if let Ok(handle) = OpenProcess(flags, false, pid) {
                let mut buffer = [0u16; 1024];
                let mut size = buffer.len() as u32;
                
                // 优先使用 QueryFullProcessImageNameW，它更现代且支持受限权限
                if QueryFullProcessImageNameW(handle, PROCESS_NAME_FORMAT(0), PWSTR(buffer.as_mut_ptr()), &mut size).is_ok() {
                    let path = String::from_utf16_lossy(&buffer[..size as usize]);
                    let _ = CloseHandle(handle);
                    if let Some(name) = path.split('\\').last() {
                        if !name.is_empty() {
                            return name.to_string();
                        }
                    }
                }
                
                // 备选方案：GetModuleBaseNameW
                let len = GetModuleBaseNameW(handle, None, &mut buffer);
                let _ = CloseHandle(handle);
                if len > 0 {
                    let name = String::from_utf16_lossy(&buffer[..len as usize]);
                    if !name.is_empty() {
                        return name.to_string();
                    }
                }
            }
        }
    }
    
    format!("进程 ({})", pid)
}
