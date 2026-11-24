//! The Windows half of the crate: UAC's `TokenElevation` query for `is_elevated` and
//! `ShellExecuteExW` with the `runas` verb for `relaunch_elevated`.

use std::ffi::OsString;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED, GetLastError, HANDLE};
use windows::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::Win32::UI::Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::PCWSTR;

use crate::ElevationError;
use crate::command_line;

/// A NUL-terminated UTF-16 string for the Win32 wide APIs, for text that is already a
/// `&str` (the `runas` verb). Paths and arguments use `OsStr::encode_wide` instead so
/// non-Unicode text reaches the OS unchanged.
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// The `lpParameters` buffer: `args` joined by [`command_line::join_arguments`],
/// NUL-terminated as the wide API expects.
fn wide_parameters(args: &[OsString]) -> Vec<u16> {
    let arguments: Vec<Vec<u16>> = args
        .iter()
        .map(|arg| arg.as_os_str().encode_wide().collect())
        .collect();
    let mut parameters = command_line::join_arguments(&arguments);
    parameters.push(0);
    parameters
}

pub(crate) fn is_elevated() -> bool {
    // SAFETY: `GetCurrentProcess` returns the current process's pseudo-handle, which is always
    // valid for `OpenProcessToken`. `token` is only used after `OpenProcessToken` filled it.
    // `elevation` is a properly sized local for the `TokenElevation` information class. The
    // token handle is closed on every path; `CloseHandle`'s result carries nothing the caller
    // can act on, so it is consumed.
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut length = 0u32;
        let result = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut TOKEN_ELEVATION as *mut _),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut length,
        );
        drop(CloseHandle(token));
        result.is_ok() && elevation.TokenIsElevated != 0
    }
}

pub(crate) fn relaunch_elevated(program: &Path, args: &[OsString]) -> Result<(), ElevationError> {
    let verb = wide("runas");
    // `encode_wide` is lossless: a program path or argument that is not valid Unicode
    // (an unpaired surrogate) reaches the OS unchanged.
    let file: Vec<u16> = program
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let parameters = wide_parameters(args);
    // SAFETY: `info` is a zeroed `SHELLEXECUTEINFOW` with `cbSize` set as the API requires;
    // `verb`, `file` and `parameters` point at NUL-terminated wide buffers bound to locals
    // that outlive the call. `hProcess` is a handle we own (SEE_MASK_NOCLOSEPROCESS) and is
    // closed before returning.
    unsafe {
        let mut info = SHELLEXECUTEINFOW {
            cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: PCWSTR(verb.as_ptr()),
            lpFile: PCWSTR(file.as_ptr()),
            lpParameters: PCWSTR(parameters.as_ptr()),
            nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };
        if ShellExecuteExW(&mut info).is_ok() {
            drop(CloseHandle(info.hProcess));
            Ok(())
        } else {
            let error = GetLastError();
            Err(if error == ERROR_CANCELLED {
                ElevationError::Declined
            } else {
                ElevationError::Failed(error.to_hresult().message())
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_is_nul_terminated_utf16() {
        assert_eq!(wide("ab"), [0x61, 0x62, 0]);
        assert_eq!(wide(""), [0]);
    }

    #[test]
    fn wide_parameters_joins_the_arguments_under_one_nul() {
        let args: Vec<OsString> = ["abc", "a b"].into_iter().map(OsString::from).collect();
        let buffer = wide_parameters(&args);
        // `ShellExecuteExW` reads the buffer until NUL: the joined line, then
        // exactly one terminator and no NUL inside it.
        assert_eq!(buffer.last(), Some(&0));
        assert!(buffer[..buffer.len() - 1].iter().all(|unit| *unit != 0));
        let joined = String::from_utf16(&buffer[..buffer.len() - 1]).unwrap();
        assert_eq!(joined, "abc \"a b\"");
        // No arguments: the terminator alone.
        assert_eq!(wide_parameters(&[]), [0]);
    }
}
