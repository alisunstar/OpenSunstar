//! Process gate for the Windows GUI entrypoint.
//!
//! The Tauri single-instance plugin normally handles duplicate launches, but on
//! some Windows install/shortcut/protocol paths several GUI processes can enter
//! the startup pipeline and show windows before the listener has settled. Holding
//! this mutex for the full GUI lifetime gives the desktop app one native main
//! window per user session. The separate `os` CLI binary does not call this path.

#[cfg(target_os = "windows")]
pub(super) struct StartupMutexGuard {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(target_os = "windows")]
impl Drop for StartupMutexGuard {
    fn drop(&mut self) {
        if self.handle.is_null() {
            return;
        }

        unsafe {
            let _ = windows_sys::Win32::System::Threading::ReleaseMutex(self.handle);
            let _ = windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}

#[cfg(target_os = "windows")]
pub(super) fn acquire() -> Option<StartupMutexGuard> {
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::CreateMutexW;

    const STARTUP_MUTEX_NAME: &str = "Local\\OpenSunstar.StartupGate";
    let name: Vec<u16> = STARTUP_MUTEX_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let handle = unsafe { CreateMutexW(std::ptr::null(), 1, name.as_ptr()) };
    if handle.is_null() {
        log::warn!("Windows startup gate 初始化失败，继续启动");
        return Some(StartupMutexGuard { handle });
    }

    let already_running = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    if already_running {
        focus_existing_window();
        unsafe {
            let _ = windows_sys::Win32::Foundation::CloseHandle(handle);
        }
        log::info!("检测到另一个 OpenSunstar 进程正在启动，本次重复启动已结束");
        return None;
    }

    Some(StartupMutexGuard { handle })
}

#[cfg(target_os = "windows")]
fn focus_existing_window() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowW, IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
    };

    let title: Vec<u16> = "OpenSunstar"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let hwnd = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    if hwnd.is_null() {
        return;
    }

    unsafe {
        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        } else {
            ShowWindow(hwnd, SW_SHOW);
        }
        SetForegroundWindow(hwnd);
    }
}

#[cfg(not(target_os = "windows"))]
pub(super) struct StartupMutexGuard;

#[cfg(not(target_os = "windows"))]
pub(super) fn acquire() -> Option<StartupMutexGuard> {
    Some(StartupMutexGuard)
}
