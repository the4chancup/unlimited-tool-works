//! What a Shift+click or a middle click on the taskbar button starts on
//! Windows. A portable program has no shortcut for Explorer to start it by, so
//! the window says on itself how it is started again: an application ID made
//! from the exe's path, and the command, name and icon the taskbar uses for
//! it. The values are pure functions of the exe's path, tested on every
//! platform; setting them is Windows-only (`set_relaunch`).

#[cfg(any(windows, test))]
use std::path::Path;

/// `exe`'s application ID: `UnlimitedToolWorks.` and the first 16 hex digits
/// of the SHA-256 of the exe's path in lower case, so one copy of the program
/// is one taskbar group and the ID does not change with the version.
#[cfg(any(windows, test))]
fn app_id(exe: &Path) -> String {
    use sha2::{Digest, Sha256};

    let text = exe.to_string_lossy().to_lowercase();
    let hash = Sha256::digest(text.as_bytes());
    let hex: String = hash
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("UnlimitedToolWorks.{hex}")
}

/// `exe` as the taskbar's relaunch command: the path between double quotes,
/// with no arguments, so a window started with a command comes back as the
/// plain window.
#[cfg(any(windows, test))]
fn relaunch_command(exe: &Path) -> String {
    format!("\"{}\"", exe.display())
}

/// `exe` as the taskbar's relaunch icon: the path, a comma and `0`, the exe's
/// first icon, not quoted.
#[cfg(any(windows, test))]
fn icon_resource(exe: &Path) -> String {
    format!("{},0", exe.display())
}

/// Tells the taskbar how to start the program again: a Shift+click or a
/// middle click on the window's taskbar button opens another window only when
/// Explorer knows a shortcut for the program, which a portable bundle never
/// has, so the window carries its own relaunch values. The ID goes first
/// because Windows reads the other three only on a window with an explicit
/// one, and a window has none until this sets it. A failure is logged and the
/// window works the same, the button's clicks staying as Windows has them.
#[cfg(windows)]
pub(super) fn set_relaunch(window: &impl raw_window_handle::HasWindowHandle) {
    use raw_window_handle::RawWindowHandle;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Storage::EnhancedStorage::{
        PKEY_AppUserModel_ID, PKEY_AppUserModel_RelaunchCommand,
        PKEY_AppUserModel_RelaunchDisplayNameResource, PKEY_AppUserModel_RelaunchIconResource,
    };
    use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
    use windows::Win32::System::Variant::VT_LPWSTR;
    use windows::Win32::UI::Shell::PropertiesSystem::{
        IPropertyStore, SHGetPropertyStoreForWindow,
    };
    use windows::core::PWSTR;

    use super::APP_NAME;

    let handle = match window.window_handle().map(|handle| handle.as_raw()) {
        Ok(RawWindowHandle::Win32(handle)) => handle,
        other => {
            log::warn!(
                "the taskbar's new-instance clicks stay as Windows has them: no Win32 window handle ({other:?})"
            );
            return;
        }
    };
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(error) => {
            log::warn!(
                "the taskbar's new-instance clicks stay as Windows has them: the program's path is unknown ({error})"
            );
            return;
        }
    };
    // SAFETY: `handle.hwnd` is the live window eframe just created, valid
    // while the creation context that lent it is borrowed here; the call
    // hands out the window's own property store as a COM object.
    let store = match unsafe {
        SHGetPropertyStoreForWindow::<IPropertyStore>(HWND(
            handle.hwnd.get() as *mut std::ffi::c_void
        ))
    } {
        Ok(store) => store,
        Err(error) => {
            log::warn!("the taskbar's new-instance clicks stay as Windows has them: {error}");
            return;
        }
    };
    for (key, text) in [
        (PKEY_AppUserModel_ID, app_id(&exe)),
        (PKEY_AppUserModel_RelaunchCommand, relaunch_command(&exe)),
        (
            PKEY_AppUserModel_RelaunchDisplayNameResource,
            APP_NAME.to_owned(),
        ),
        (PKEY_AppUserModel_RelaunchIconResource, icon_resource(&exe)),
    ] {
        let mut wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let mut value = PROPVARIANT::default();
        // SAFETY: a PROPVARIANT's `Anonymous` union member is the one to fill
        // for a plain value, and `wide` outlives the SetValue call below,
        // which copies the string into the store, so nothing is left dangling.
        // The value only borrows `wide`: this struct has no destructor, and
        // it must never be handed to `PropVariantClear`, which would free
        // the pointer as memory COM allocated, which a `Vec`'s is not.
        unsafe {
            (*value.Anonymous.Anonymous).vt = VT_LPWSTR;
            (*value.Anonymous.Anonymous).Anonymous.pwszVal = PWSTR::from_raw(wide.as_mut_ptr());
        }
        // SAFETY: `store` is the window's live property store, `key` a static
        // PROPERTYKEY and `value` a VT_LPWSTR PROPVARIANT pointing at `wide`,
        // which is alive for the call.
        if let Err(error) = unsafe { store.SetValue(&key, &value) } {
            log::warn!("the taskbar's new-instance clicks stay as Windows has them: {error}");
            return;
        }
    }
    // SAFETY: `store` is the window's live property store.
    if let Err(error) = unsafe { store.Commit() } {
        log::warn!("the taskbar's new-instance clicks stay as Windows has them: {error}");
    }
}

// set_relaunch has no test: it needs a real window.
#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    // ST-WIN-05
    #[test]
    fn the_relaunch_values_follow_the_exe_path() {
        let exe = Path::new(r"C:\Tools\UTW\UTW.exe");
        assert_eq!(relaunch_command(exe), r#""C:\Tools\UTW\UTW.exe""#);
        assert_eq!(icon_resource(exe), r"C:\Tools\UTW\UTW.exe,0");
        assert_eq!(app_id(exe), "UnlimitedToolWorks.be6639c454fb3ede");
        let id = app_id(exe);
        assert!(id.starts_with("UnlimitedToolWorks."));
        let suffix = &id["UnlimitedToolWorks.".len()..];
        assert_eq!(suffix.len(), 16);
        assert!(
            suffix
                .chars()
                .all(|digit| digit.is_ascii_digit() || ('a'..='f').contains(&digit))
        );
        assert_eq!(app_id(Path::new(r"c:\tools\utw\utw.exe")), id);
        assert_ne!(app_id(Path::new(r"D:\Other\UTW.exe")), id);
    }
}
