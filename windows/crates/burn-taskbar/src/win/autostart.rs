//! Launch at login through the per-user Run key (no admin rights, shows in Task Manager's
//! Startup apps, where the user can also turn it off).

use std::ffi::c_void;

use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, RRF_RT_REG_SZ, RegCloseKey, RegDeleteValueW, RegGetValueW,
    RegOpenKeyExW, RegSetValueExW,
};
use windows::core::{PCWSTR, w};

use super::util::wide;

const RUN: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const NAME: PCWSTR = w!("Burn");

/// The command Windows runs at login for this exe.
fn command() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    Some(format!("\"{}\"", exe.display()))
}

/// True when the Run key starts this copy of Burn.
pub fn is_enabled() -> bool {
    let mut buf = [0u16; 1024];
    let mut size = (buf.len() * 2) as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            RUN,
            NAME,
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut c_void),
            Some(&mut size),
        )
    };
    if status.is_err() {
        return false;
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    let value = String::from_utf16_lossy(&buf[..len]);
    command().is_some_and(|c| value.eq_ignore_ascii_case(&c))
}

pub fn set_enabled(on: bool) -> windows::core::Result<()> {
    unsafe {
        let mut key = HKEY::default();
        RegOpenKeyExW(HKEY_CURRENT_USER, RUN, None, KEY_SET_VALUE, &mut key).ok()?;
        let result = if on {
            let value = wide(&command().unwrap_or_default());
            let bytes = std::slice::from_raw_parts(value.as_ptr() as *const u8, value.len() * 2);
            RegSetValueExW(key, NAME, None, REG_SZ, Some(bytes)).ok()
        } else {
            match RegDeleteValueW(key, NAME).ok() {
                // Already off.
                Err(e) if e.code() == windows::Win32::Foundation::ERROR_FILE_NOT_FOUND.to_hresult() => Ok(()),
                other => other,
            }
        };
        let _ = RegCloseKey(key);
        result
    }
}
