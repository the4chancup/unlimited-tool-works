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
    let window = HWND(handle.hwnd.get() as *mut std::ffi::c_void);
    if let Err(error) = set_relaunch_values(window, &exe) {
        log::warn!("the taskbar's new-instance clicks stay as Windows has them: {error}");
    }
}

/// Writes `exe`'s four relaunch values to `window`'s property store.
/// `window` must be a live top-level window of this thread.
#[cfg(windows)]
fn set_relaunch_values(
    window: windows::Win32::Foundation::HWND,
    exe: &Path,
) -> windows::core::Result<()> {
    use std::mem::ManuallyDrop;

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

    // SAFETY: the caller hands a live window of this thread; the call hands
    // out the window's own property store as a COM object.
    let store: IPropertyStore = unsafe { SHGetPropertyStoreForWindow(window) }?;
    for (key, text) in [
        (PKEY_AppUserModel_ID, app_id(exe)),
        (PKEY_AppUserModel_RelaunchCommand, relaunch_command(exe)),
        (
            PKEY_AppUserModel_RelaunchDisplayNameResource,
            APP_NAME.to_owned(),
        ),
        (PKEY_AppUserModel_RelaunchIconResource, icon_resource(exe)),
    ] {
        let mut wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        // Never dropped: the `windows` crate gives a PROPVARIANT a destructor
        // (`PropVariantClear`) that frees the string it holds as memory COM
        // allocated. This one only borrows `wide`, so the destructor would
        // free the `Vec`'s buffer and the `Vec` free it again: that double
        // free ended the program at every start with a corrupted heap.
        let mut value = ManuallyDrop::new(PROPVARIANT::default());
        // SAFETY: a PROPVARIANT's `Anonymous` union member is the one to fill
        // for a plain value; the pointer is to `wide`, NUL-ended and alive
        // until the SetValue call below has copied the string into the store.
        unsafe {
            (*value.Anonymous.Anonymous).vt = VT_LPWSTR;
            (*value.Anonymous.Anonymous).Anonymous.pwszVal = PWSTR::from_raw(wide.as_mut_ptr());
        }
        // SAFETY: `store` is the window's live property store, `key` a static
        // PROPERTYKEY and `value` a VT_LPWSTR PROPVARIANT pointing at `wide`,
        // which is alive for the call.
        unsafe { store.SetValue(&key, &*value) }?;
    }
    // Kept though no test fails without it: a test reads back the store it wrote,
    // uncommitted values included, and only the taskbar itself would tell.
    // SAFETY: `store` is the window's live property store.
    unsafe { store.Commit() }
}

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
        let id = app_id(exe);
        assert_eq!(id, "UnlimitedToolWorks.be6639c454fb3ede");
        assert_eq!(app_id(Path::new(r"c:\tools\utw\utw.exe")), id);
        assert_ne!(app_id(Path::new(r"D:\Other\UTW.exe")), id);
    }

    /// A real window to hold the values, and what its property store gives
    /// back: the two tests below set the values on one and read them here.
    #[cfg(windows)]
    mod window {
        use windows::Win32::Foundation::{HWND, PROPERTYKEY};
        use windows::Win32::UI::Shell::PropertiesSystem::{
            IPropertyStore, SHGetPropertyStoreForWindow,
        };
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, WINDOW_EX_STYLE, WS_OVERLAPPED,
        };
        use windows::core::{BSTR, w};

        /// A top-level window that is never shown.
        pub(super) fn hidden() -> HWND {
            // SAFETY: a window of the system's own STATIC class, with no
            // parent, menu or creation data.
            unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("STATIC"),
                    w!("relaunch values"),
                    WS_OVERLAPPED,
                    0,
                    0,
                    0,
                    0,
                    None,
                    None,
                    None,
                    None,
                )
            }
            .expect("a window to hold the values")
        }

        /// The string `window`'s property store holds under `key`.
        pub(super) fn held(window: HWND, key: PROPERTYKEY) -> String {
            // SAFETY: `window` is this thread's live window.
            let store: IPropertyStore = unsafe { SHGetPropertyStoreForWindow(window) }
                .expect("the window's property store");
            // SAFETY: `store` is live and `key` a static PROPERTYKEY; the
            // value read is an owned copy, freed when dropped.
            let value = unsafe { store.GetValue(&key) }.expect("the value is read");
            BSTR::try_from(&value)
                .expect("the value is a string")
                .to_string()
        }

        pub(super) fn destroy(window: HWND) {
            // SAFETY: `window` is this thread's own window, used by nothing else.
            unsafe { DestroyWindow(window) }.expect("the window is destroyed");
        }
    }

    /// The four values read back off a real window's property store, which
    /// also runs `set_relaunch_values` to its end on a live window: a value
    /// freed twice there ends the process, and so this test's.
    #[cfg(windows)]
    // ST-WIN-05
    #[test]
    fn a_window_holds_the_relaunch_values_it_was_given() {
        use windows::Win32::Storage::EnhancedStorage::{
            PKEY_AppUserModel_ID, PKEY_AppUserModel_RelaunchCommand,
            PKEY_AppUserModel_RelaunchDisplayNameResource, PKEY_AppUserModel_RelaunchIconResource,
        };

        let exe = Path::new(r"C:\Tools\UTW\UTW.exe");
        let window = window::hidden();

        set_relaunch_values(window, exe).expect("the values are set");

        assert_eq!(
            window::held(window, PKEY_AppUserModel_ID),
            "UnlimitedToolWorks.be6639c454fb3ede"
        );
        assert_eq!(
            window::held(window, PKEY_AppUserModel_RelaunchCommand),
            r#""C:\Tools\UTW\UTW.exe""#
        );
        assert_eq!(
            window::held(window, PKEY_AppUserModel_RelaunchDisplayNameResource),
            crate::shell::APP_NAME
        );
        assert_eq!(
            window::held(window, PKEY_AppUserModel_RelaunchIconResource),
            r"C:\Tools\UTW\UTW.exe,0"
        );
        window::destroy(window);
    }

    /// `set_relaunch` gives a window the values of the program that runs,
    /// here the test's own: the handle is taken from the window it is handed.
    #[cfg(windows)]
    // ST-WIN-05
    #[test]
    fn a_window_is_given_the_running_programs_values() {
        use raw_window_handle::{
            HandleError, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
        };
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Storage::EnhancedStorage::{
            PKEY_AppUserModel_ID, PKEY_AppUserModel_RelaunchCommand,
        };

        struct Lent(HWND);
        impl HasWindowHandle for Lent {
            fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
                let handle = std::num::NonZeroIsize::new(self.0.0 as isize)
                    .expect("a created window's handle is not null");
                let raw = RawWindowHandle::Win32(Win32WindowHandle::new(handle));
                // SAFETY: the window is alive for as long as the test holds `self`.
                Ok(unsafe { WindowHandle::borrow_raw(raw) })
            }
        }

        let window = window::hidden();
        set_relaunch(&Lent(window));

        let exe = std::env::current_exe().expect("the test program's path");
        assert_eq!(window::held(window, PKEY_AppUserModel_ID), app_id(&exe));
        assert_eq!(
            window::held(window, PKEY_AppUserModel_RelaunchCommand),
            relaunch_command(&exe)
        );
        window::destroy(window);
    }
}
