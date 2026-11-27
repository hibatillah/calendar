//! Named mutexes that make each process a single instance and let the tray process see whether the
//! UI process is open. The kernel releases a mutex when its process exits, even after a crash.

use std::fmt;
use windows::{
    Win32::{
        Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE},
        System::Threading::{CreateMutexW, OpenMutexW, SYNCHRONIZATION_SYNCHRONIZE},
    },
    core::HSTRING,
};

pub const UI_INSTANCE: &str = r"Local\CalendarDesktopUi";
pub const TRAY_INSTANCE: &str = r"Local\CalendarTray";

/// Another process already holds the name.
#[derive(Debug)]
pub struct AlreadyRunning;

impl fmt::Display for AlreadyRunning {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("already running")
    }
}

/// Holds the name until dropped or until the process ends.
pub struct Instance(HANDLE);

impl Drop for Instance {
    fn drop(&mut self) {
        // SAFETY: the handle came from `CreateMutexW` and is closed once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// Takes the name, or reports that another process holds it. An unusable mutex counts as free, so
/// a failure here never stops the app from starting.
pub fn acquire(name: &str) -> Result<Option<Instance>, AlreadyRunning> {
    // SAFETY: the name is a valid wide string for the duration of the call.
    let handle = match unsafe { CreateMutexW(None, false, &HSTRING::from(name)) } {
        Ok(handle) => handle,
        Err(_) => return Ok(None),
    };
    // SAFETY: read immediately after the call, on the same thread.
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        // SAFETY: closing the duplicate handle we just received.
        let _ = unsafe { CloseHandle(handle) };
        return Err(AlreadyRunning);
    }

    Ok(Some(Instance(handle)))
}

/// Whether some process currently holds the name.
pub fn is_running(name: &str) -> bool {
    // SAFETY: the name is a valid wide string for the duration of the call.
    match unsafe { OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, false, &HSTRING::from(name)) } {
        Ok(handle) => {
            // SAFETY: closing the handle we just opened.
            let _ = unsafe { CloseHandle(handle) };
            true
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_held_by_one_instance_until_it_is_dropped() {
        let name = format!(r"Local\CalendarTest{}", std::process::id());
        assert!(!is_running(&name));

        let first = acquire(&name).unwrap();
        assert!(is_running(&name));
        assert!(acquire(&name).is_err());

        drop(first);
        assert!(!is_running(&name));
        assert!(acquire(&name).is_ok());
    }
}
